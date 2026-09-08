use std::cell::RefCell;
use std::sync::Arc;
use std::time::Duration;

use crate::persistence::ProviderCliSettings;
use crate::platform::windows::environment::DiscoveryInputs;
use crate::platform::windows::executable_locator::{
    DiscoveryBudget, ExecutableCandidate, RuntimeSource,
};
use crate::platform::windows::process::{
    BoundedProcessRunner, CancellationToken, ProcessErrorKind, ProcessRequest,
    command_for_candidate,
};
use crate::platform::windows::wsl::build_wsl_list_invocation;

use super::cli_account::CliProvider;
use super::cli_discovery::{CliDiscovery, CliProbe, CliWslList, discover_cli};

const TOTAL_TIMEOUT: Duration = Duration::from_secs(8);
const MAXIMUM_PROCESSES: usize = 12;

pub fn discover_runtime_cli(
    provider: CliProvider,
    settings: &ProviderCliSettings,
    cancellation: Arc<CancellationToken>,
) -> CliDiscovery {
    discover_runtime_cli_with_inputs(provider, settings, cancellation, discovery_inputs())
}

fn discover_runtime_cli_with_inputs(
    provider: CliProvider,
    settings: &ProviderCliSettings,
    cancellation: Arc<CancellationToken>,
    inputs: DiscoveryInputs,
) -> CliDiscovery {
    if cancellation.is_cancelled() {
        return CliDiscovery::Cancelled;
    }
    let system_root = inputs.system_root.clone();
    let budget = RefCell::new(DiscoveryBudget::new(TOTAL_TIMEOUT, MAXIMUM_PROCESSES));
    let result = discover_cli(
        provider,
        settings,
        inputs,
        |candidate| {
            probe_runtime_candidate(candidate, provider, &cancellation, &mut budget.borrow_mut())
        },
        || {
            if cancellation.is_cancelled() {
                return CliWslList::Cancelled;
            }
            let Some(root) = system_root.as_deref() else {
                return CliWslList::Unavailable;
            };
            let Some(invocation) = build_wsl_list_invocation(root) else {
                return if matches!(root.join("System32/wsl.exe").try_exists(), Ok(false)) {
                    CliWslList::Missing
                } else {
                    CliWslList::Unavailable
                };
            };
            let Some(timeout) = budget
                .borrow_mut()
                .next_process_timeout(Duration::from_secs(3))
            else {
                return CliWslList::Unavailable;
            };
            let mut request = ProcessRequest::new(
                invocation.executable,
                invocation.arguments.into_iter().map(Into::into).collect(),
            );
            request.timeout = timeout;
            request.max_output_bytes = 16 * 1024;
            request.cancellation = Arc::clone(&cancellation);
            match BoundedProcessRunner.run(request) {
                Ok(output) if output.exit_code == Some(0) => {
                    CliWslList::Output(output.stdout.into_bytes())
                }
                Err(error) if error.kind() == ProcessErrorKind::Cancelled => CliWslList::Cancelled,
                _ => CliWslList::Unavailable,
            }
        },
    );
    if cancellation.is_cancelled() {
        CliDiscovery::Cancelled
    } else {
        result
    }
}

#[cfg(windows)]
fn discovery_inputs() -> DiscoveryInputs {
    DiscoveryInputs::capture(None)
}

#[cfg(not(windows))]
fn discovery_inputs() -> DiscoveryInputs {
    DiscoveryInputs::default()
}

fn probe_runtime_candidate(
    candidate: &ExecutableCandidate,
    provider: CliProvider,
    cancellation: &Arc<CancellationToken>,
    budget: &mut DiscoveryBudget,
) -> CliProbe {
    if cancellation.is_cancelled() {
        return CliProbe::Cancelled;
    }
    if let RuntimeSource::Wsl { distribution } = &candidate.source {
        let script = match provider {
            CliProvider::Claude => "command -v claude >/dev/null 2>&1",
            CliProvider::Codex => "command -v codex >/dev/null 2>&1",
        };
        let Some(timeout) = budget.next_process_timeout(Duration::from_secs(3)) else {
            return CliProbe::Unavailable;
        };
        let mut request = ProcessRequest::new(
            candidate.executable.clone(),
            vec![
                "--distribution".into(),
                distribution.clone().into(),
                "--exec".into(),
                "/bin/sh".into(),
                "-c".into(),
                script.into(),
            ],
        );
        request.timeout = timeout;
        request.max_output_bytes = 1024;
        request.cancellation = Arc::clone(cancellation);
        match BoundedProcessRunner.run(request) {
            Ok(output) if output.exit_code == Some(1) => return CliProbe::Missing,
            Ok(output) if output.exit_code == Some(0) => {}
            Err(error) if error.kind() == ProcessErrorKind::Cancelled => {
                return CliProbe::Cancelled;
            }
            _ => return CliProbe::Unavailable,
        }
    }
    let Some(timeout) = budget.next_process_timeout(Duration::from_secs(4)) else {
        return CliProbe::Unavailable;
    };
    let Ok(invocation) = command_for_candidate(candidate, provider, &["--version"]) else {
        return CliProbe::Unavailable;
    };
    let mut request = ProcessRequest::new(invocation.executable, invocation.arguments);
    request.timeout = timeout;
    request.max_output_bytes = 16 * 1024;
    request.cancellation = Arc::clone(cancellation);
    match BoundedProcessRunner.run(request) {
        Ok(output) if output.exit_code == Some(0) => CliProbe::Healthy,
        Err(error) if error.kind() == ProcessErrorKind::Cancelled => CliProbe::Cancelled,
        _ => CliProbe::Unavailable,
    }
}

#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;
    use crate::persistence::CliRuntimeMode;

    #[test]
    fn existing_candidate_that_cannot_start_is_unavailable_not_missing() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("codex.exe"), "not executable").unwrap();
        let result = discover_runtime_cli_with_inputs(
            CliProvider::Codex,
            &ProviderCliSettings {
                mode: CliRuntimeMode::NativeWindows,
                ..Default::default()
            },
            Arc::new(CancellationToken::new()),
            DiscoveryInputs {
                conventional_paths: vec![root.path().to_owned()],
                ..Default::default()
            },
        );
        assert!(matches!(result, CliDiscovery::Unavailable));
    }

    #[test]
    fn no_candidate_in_a_complete_native_search_is_missing() {
        let result = discover_runtime_cli_with_inputs(
            CliProvider::Codex,
            &ProviderCliSettings {
                mode: CliRuntimeMode::NativeWindows,
                ..Default::default()
            },
            Arc::new(CancellationToken::new()),
            DiscoveryInputs::default(),
        );
        assert!(matches!(result, CliDiscovery::Missing));
    }
}
