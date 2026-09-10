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

#[cfg(windows)]
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

    let version_output = run(
        &candidate,
        &environment,
        environment.arguments(true),
        Duration::from_secs(8),
        VERSION_OUTPUT_LIMIT,
        Arc::clone(&cancellation),
    )?;
    let version = version_from_output(&version_output)?;
    let output = run(
        &candidate,
        &environment,
        environment.arguments(false),
        Duration::from_secs(30),
        USAGE_OUTPUT_LIMIT,
        cancellation,
    )?;
    if output.exit_code != Some(0) {
        return Err(if authentication_required(&output) {
            CollectionError::AuthenticationRequired
        } else {
            CollectionError::Transport
        });
    }
    super::gemini::parse_usage(&output.stdout, fetched_at, &version)
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
}
