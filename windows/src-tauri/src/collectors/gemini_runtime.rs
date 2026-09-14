use super::{CollectionError, gemini::MINIMUM_VERSION, gemini_environment::GeminiEnvironment};
use crate::{
    accounts::cli_account::CliProvider,
    platform::windows::{
        environment::DiscoveryInputs,
        executable_locator::{DiscoveryBudget, ExecutableCandidate, ExecutableLocator},
        process::{
            BoundedProcessRunner, CancellationToken, ProcessErrorKind, ProcessOutput,
            ProcessRequest, command_for_candidate,
        },
    },
};
use std::{sync::Arc, time::Duration};

const VERSION_OUTPUT_LIMIT: usize = 16 * 1024;
#[cfg(windows)]
const USAGE_OUTPUT_LIMIT: usize = 64 * 1024;

pub fn discover(
    environment: &GeminiEnvironment,
    inputs: DiscoveryInputs,
    cancellation: Arc<CancellationToken>,
) -> Result<Option<ExecutableCandidate>, CollectionError> {
    if cancellation.is_cancelled() {
        return Err(CollectionError::Cancelled);
    }
    let locator = ExecutableLocator::new(inputs);
    let mut error = None;
    let mut budget = DiscoveryBudget::new(Duration::from_secs(8), 12);
    let found = locator.locate(CliProvider::Gemini, |candidate| {
        let outcome = (|| {
            let timeout = budget
                .next_process_timeout(Duration::from_secs(4))
                .ok_or(CollectionError::TimedOut)?;
            let output = run(
                candidate,
                environment,
                environment.arguments(true),
                timeout,
                VERSION_OUTPUT_LIMIT,
                Arc::clone(&cancellation),
            )?;
            version_from_output(&output)
        })();
        if let Err(failure) = outcome {
            error = Some(failure);
            false
        } else {
            true
        }
    });
    if cancellation.is_cancelled() {
        return Err(CollectionError::Cancelled);
    }
    if let Some(candidate) = found {
        return Ok(Some(candidate));
    }
    if let Some(error) = error {
        return Err(error);
    }
    if locator.has_native_candidates_or_incomplete_search(CliProvider::Gemini) {
        return Err(CollectionError::Transport);
    }
    Ok(None)
}

fn run(
    candidate: &ExecutableCandidate,
    environment: &GeminiEnvironment,
    arguments: Vec<String>,
    timeout: Duration,
    max_output_bytes: usize,
    cancellation: Arc<CancellationToken>,
) -> Result<ProcessOutput, CollectionError> {
    if cancellation.is_cancelled() {
        return Err(CollectionError::Cancelled);
    }
    let references = arguments.iter().map(String::as_str).collect::<Vec<_>>();
    let command = command_for_candidate(candidate, CliProvider::Gemini, &references)
        .map_err(|_| CollectionError::UnsupportedConfiguration)?;
    let mut request = ProcessRequest::new(command.executable, command.arguments);
    request.working_directory = Some(environment.directory.clone());
    request.environment = environment.variables.clone();
    request.timeout = timeout;
    request.max_output_bytes = max_output_bytes;
    request.cancellation = cancellation;
    BoundedProcessRunner
        .run(request)
        .map_err(|error| match error.kind() {
            ProcessErrorKind::Cancelled => CollectionError::Cancelled,
            ProcessErrorKind::TimedOut => CollectionError::TimedOut,
            ProcessErrorKind::OutputLimitExceeded => CollectionError::UnrecognizedOutput,
            _ => CollectionError::Transport,
        })
}

#[cfg(any(windows, test))]
fn optional_output_result(
    result: Result<ProcessOutput, CollectionError>,
) -> Result<Option<String>, CollectionError> {
    match result {
        Ok(output) if output.exit_code == Some(0) => Ok(Some(output.stdout)),
        Ok(_) => Ok(None),
        Err(CollectionError::Cancelled) => Err(CollectionError::Cancelled),
        Err(_) => Ok(None),
    }
}

fn version_from_output(output: &ProcessOutput) -> Result<String, CollectionError> {
    if output.exit_code != Some(0) {
        return Err(CollectionError::Transport);
    }
    let version = output.stdout.trim();
    let parts = version
        .split('.')
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| CollectionError::UnsupportedVersion)?;
    let minimum = MINIMUM_VERSION
        .split('.')
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| CollectionError::UnsupportedVersion)?;
    if parts.len() != 3 || parts[0] != 1 || parts < minimum {
        return Err(CollectionError::UnsupportedVersion);
    }
    Ok(version.into())
}

#[cfg(any(windows, test))]
fn authentication_required(output: &ProcessOutput) -> bool {
    let message = format!("{}\n{}", output.stdout, output.stderr).to_ascii_lowercase();
    [
        "authentication required",
        "sign in required",
        "please sign in",
        "not authenticated",
        "login required",
    ]
    .iter()
    .any(|phrase| message.contains(phrase))
}

#[cfg(any(windows, test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CollectCommand {
    Version,
    Usage,
    CurrentModel,
    Models,
}

#[cfg(any(windows, test))]
fn collect_with_runner(
    fetched_at: &str,
    mut execute: impl FnMut(CollectCommand) -> Result<ProcessOutput, CollectionError>,
) -> Result<crate::domain::UsageSnapshot, CollectionError> {
    let version = version_from_output(&execute(CollectCommand::Version)?)?;
    let usage = execute(CollectCommand::Usage)?;
    if usage.exit_code != Some(0) {
        return Err(if authentication_required(&usage) {
            CollectionError::AuthenticationRequired
        } else {
            CollectionError::Transport
        });
    }
    let mut snapshot = super::gemini::parse_usage(&usage.stdout, fetched_at, &version)?;
    let current_model = optional_output_result(execute(CollectCommand::CurrentModel))?;
    let models = optional_output_result(execute(CollectCommand::Models))?;
    snapshot.antigravity_cli_info =
        super::gemini::cli_info(current_model.as_deref(), models.as_deref());
    Ok(snapshot)
}

#[cfg(windows)]
pub fn collect(
    fetched_at: &str,
    cancellation: Arc<CancellationToken>,
) -> Result<crate::domain::UsageSnapshot, CollectionError> {
    let environment =
        GeminiEnvironment::prepare(&std::env::temp_dir(), std::env::vars_os().collect(), &[])?;
    let candidate = discover(
        &environment,
        DiscoveryInputs::capture(None),
        Arc::clone(&cancellation),
    )?;
    let Some(candidate) = candidate else {
        let mut snapshot = crate::domain::UsageSnapshot::decode_compatible(&serde_json::json!({
            "schemaVersion": 1,
            "providerId": "gemini",
            "displayName": "Google Antigravity",
            "status": "notInstalled",
            "fetchedAt": fetched_at,
            "staleAfterSeconds": 300
        }))
        .map_err(|_| CollectionError::InvalidResponse)?;
        snapshot.status_message = Some(
            "Antigravity CLI was not found in the supported native Windows locations. WSL collection is not supported. See the official CLI guide."
                .into(),
        );
        return Ok(snapshot);
    };

    collect_with_runner(fetched_at, |command| {
        let (arguments, timeout, output_limit) = match command {
            CollectCommand::Version => (
                environment.arguments(true),
                Duration::from_secs(8),
                VERSION_OUTPUT_LIMIT,
            ),
            CollectCommand::Usage => (
                environment.arguments(false),
                Duration::from_secs(30),
                USAGE_OUTPUT_LIMIT,
            ),
            CollectCommand::CurrentModel => (
                environment.arguments_for(&["-p", "/model", "--print-timeout", "10s"]),
                Duration::from_secs(15),
                USAGE_OUTPUT_LIMIT,
            ),
            CollectCommand::Models => (
                environment.arguments_for(&["models"]),
                Duration::from_secs(15),
                USAGE_OUTPUT_LIMIT,
            ),
        };
        run(
            &candidate,
            &environment,
            arguments,
            timeout,
            output_limit,
            Arc::clone(&cancellation),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(version: &str) -> ProcessOutput {
        ProcessOutput {
            exit_code: Some(0),
            stdout: version.into(),
            stderr: String::new(),
        }
    }

    #[test]
    fn supported_versions_stay_within_major_one_and_start_at_the_verified_baseline() {
        for version in ["1.1.28", "1.1.29", "1.2.0", "1.99.0"] {
            assert_eq!(version_from_output(&output(version)).unwrap(), version);
        }
        for version in [
            "1.1.27",
            "0.58.0",
            "2.0.0",
            "1.1",
            "1.1.28-beta",
            "1.1.28+build",
            "version 1.1.28",
        ] {
            assert!(matches!(
                version_from_output(&output(version)),
                Err(CollectionError::UnsupportedVersion)
            ));
        }
    }

    #[test]
    fn supplemental_failures_are_optional_but_cancellation_propagates() {
        let successful = ProcessOutput {
            exit_code: Some(0),
            stdout: "Gemini 3.8 Flash".into(),
            stderr: String::new(),
        };
        let failed = ProcessOutput {
            exit_code: Some(2),
            stdout: String::new(),
            stderr: "failed".into(),
        };
        assert_eq!(
            optional_output_result(Ok(successful)).unwrap().as_deref(),
            Some("Gemini 3.8 Flash")
        );
        assert_eq!(optional_output_result(Ok(failed)).unwrap(), None);
        for failure in [
            CollectionError::TimedOut,
            CollectionError::Transport,
            CollectionError::UnrecognizedOutput,
        ] {
            assert_eq!(optional_output_result(Err(failure)).unwrap(), None);
        }
        assert_eq!(
            optional_output_result(Err(CollectionError::Cancelled)),
            Err(CollectionError::Cancelled)
        );
    }

    fn successful_output(stdout: &str) -> ProcessOutput {
        ProcessOutput {
            exit_code: Some(0),
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }

    const USAGE: &str = "Gemini Models\tFive Hour Limit Remaining\t40%\t2026-09-14T10:00:00Z\nGemini Models\tWeekly Limit Remaining\t75%\t2026-09-21T10:00:00Z";

    #[test]
    fn production_collection_orchestration_short_circuits_after_quota_failure() {
        let mut calls = Vec::new();
        let result = collect_with_runner("2026-09-14T09:00:00Z", |command| {
            calls.push(command);
            Ok(match command {
                CollectCommand::Version => successful_output("1.2.2"),
                CollectCommand::Usage => ProcessOutput {
                    exit_code: Some(2),
                    stdout: "transport failure".into(),
                    stderr: String::new(),
                },
                _ => successful_output("unexpected"),
            })
        });
        assert_eq!(result, Err(CollectionError::Transport));
        assert_eq!(calls, vec![CollectCommand::Version, CollectCommand::Usage]);
    }

    #[test]
    fn production_collection_orchestration_keeps_supplements_independent() {
        let mut calls = Vec::new();
        let snapshot = collect_with_runner("2026-09-14T09:00:00Z", |command| {
            calls.push(command);
            match command {
                CollectCommand::Version => Ok(successful_output("1.2.2")),
                CollectCommand::Usage => Ok(successful_output(USAGE)),
                CollectCommand::CurrentModel => Err(CollectionError::TimedOut),
                CollectCommand::Models => Ok(successful_output(
                    "gemini-3.8-flash-high\tGemini 3.8 Flash (High)",
                )),
            }
        })
        .unwrap();
        assert_eq!(snapshot.status, crate::domain::UsageStatus::Fresh);
        let info = snapshot.antigravity_cli_info.unwrap();
        assert_eq!(info.current_model, None);
        assert_eq!(info.available_model_count, Some(1));
        assert_eq!(calls.len(), 4);
    }

    #[test]
    fn production_collection_orchestration_propagates_cancellation_before_second_supplement() {
        let mut calls = Vec::new();
        let result = collect_with_runner("2026-09-14T09:00:00Z", |command| {
            calls.push(command);
            match command {
                CollectCommand::Version => Ok(successful_output("1.2.2")),
                CollectCommand::Usage => Ok(successful_output(USAGE)),
                CollectCommand::CurrentModel => Err(CollectionError::Cancelled),
                CollectCommand::Models => Ok(successful_output("unexpected")),
            }
        });
        assert_eq!(result, Err(CollectionError::Cancelled));
        assert_eq!(
            calls,
            vec![
                CollectCommand::Version,
                CollectCommand::Usage,
                CollectCommand::CurrentModel
            ]
        );
    }
}
