use super::{CollectionError, gemini::VERSION, gemini_environment::GeminiEnvironment};
use crate::{
    accounts::cli_account::CliProvider,
    platform::windows::{
        environment::DiscoveryInputs,
        executable_locator::{DiscoveryBudget, ExecutableCandidate, ExecutableLocator},
        process::{
            BoundedProcessRunner, CancellationToken, ProcessErrorKind, ProcessRequest,
            command_for_candidate,
        },
    },
};
use std::{sync::Arc, time::Duration};

/// Native-only discovery: every probe is inside the same checked Gemini environment.
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
            if cancellation.is_cancelled() {
                return Err(CollectionError::Cancelled);
            }
            let timeout = budget
                .next_process_timeout(Duration::from_secs(4))
                .ok_or(CollectionError::TimedOut)?;
            let args = environment.arguments(true);
            let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
            let command = command_for_candidate(candidate, CliProvider::Gemini, &refs)
                .map_err(|_| CollectionError::UnsupportedConfiguration)?;
            let mut request = ProcessRequest::new(command.executable, command.arguments);
            request.working_directory = Some(environment.directory.clone());
            request.environment = environment.variables.clone();
            request.timeout = timeout;
            request.max_output_bytes = 16 * 1024;
            request.cancellation = Arc::clone(&cancellation);
            let output = BoundedProcessRunner
                .run(request)
                .map_err(|e| match e.kind() {
                    ProcessErrorKind::Cancelled => CollectionError::Cancelled,
                    ProcessErrorKind::TimedOut => CollectionError::TimedOut,
                    _ => CollectionError::Transport,
                })?;
            if output.exit_code != Some(0) {
                return Err(CollectionError::Transport);
            }
            if output.stdout.trim() != VERSION {
                return Err(CollectionError::UnsupportedVersion);
            }
            Ok(())
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

pub struct NativeTerminal {
    terminal: Option<crate::platform::windows::conpty::ConPty>,
    child: Option<crate::platform::windows::conpty::ConPtyChild>,
}
impl NativeTerminal {
    pub fn open(
        candidate: &ExecutableCandidate,
        environment: &GeminiEnvironment,
    ) -> Result<Self, CollectionError> {
        use crate::platform::windows::conpty::{ConPty, ConPtySize};
        let args = environment.arguments(false);
        let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        let command = command_for_candidate(candidate, CliProvider::Gemini, &refs)
            .map_err(|_| CollectionError::UnsupportedConfiguration)?;
        let mut terminal = ConPty::open(ConPtySize {
            columns: 140,
            rows: 50,
        })
        .map_err(|_| CollectionError::Transport)?;
        let child = terminal
            .spawn(
                &command,
                Some(&environment.directory),
                &environment.variables,
            )
            .map_err(|_| CollectionError::Transport)?;
        Ok(Self {
            terminal: Some(terminal),
            child: Some(child),
        })
    }
}
impl super::gemini_session::GeminiTerminal for NativeTerminal {
    fn read(&mut self) -> Result<Vec<u8>, CollectionError> {
        self.terminal
            .as_mut()
            .ok_or(CollectionError::Transport)?
            .read_available()
            .map_err(|_| CollectionError::Transport)
    }
    fn send(&mut self, bytes: &[u8]) -> Result<(), CollectionError> {
        self.terminal
            .as_mut()
            .ok_or(CollectionError::Transport)?
            .send_fixed_input(bytes)
            .map_err(|_| CollectionError::Transport)
    }
    fn stop(&mut self) -> Result<Vec<u8>, CollectionError> {
        use crate::platform::windows::conpty::ConPtyError;
        let mut tail = Vec::new();
        let result = (|| {
            let deadline = std::time::Instant::now() + Duration::from_millis(750);
            loop {
                let bytes = self.read()?;
                if tail.len() + bytes.len() > 256 * 1024 {
                    return Err(CollectionError::UnrecognizedOutput);
                }
                let empty = bytes.is_empty();
                tail.extend(bytes);
                let exited = self
                    .child
                    .as_ref()
                    .ok_or(CollectionError::Transport)?
                    .has_exited()
                    .map_err(|_| CollectionError::Transport)?;
                if exited && empty {
                    break;
                }
                if std::time::Instant::now() >= deadline {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            if let Some(child) = self.child.as_mut() {
                match child.wait(Duration::from_millis(100)) {
                    Ok(_) => {}
                    Err(ConPtyError::TimedOut) => {
                        child
                            .wait(Duration::from_secs(2))
                            .map_err(|_| CollectionError::Transport)?;
                    }
                    Err(_) => return Err(CollectionError::Transport),
                }
            }
            // Process exit can precede the final pipe read. Drain before releasing the PTY.
            loop {
                let bytes = self.read()?;
                if bytes.is_empty() {
                    break;
                }
                if tail.len() + bytes.len() > 256 * 1024 {
                    return Err(CollectionError::UnrecognizedOutput);
                }
                tail.extend(bytes);
            }
            Ok(tail)
        })();
        // Also observe termination if pipe validation failed before the normal wait.
        if result.is_err()
            && let Some(child) = self.child.as_mut()
        {
            let _ = child.wait(Duration::ZERO);
            let _ = child.wait(Duration::from_secs(2));
        }
        self.child.take();
        self.terminal.take();
        result
    }
}
impl Drop for NativeTerminal {
    fn drop(&mut self) {
        self.child.take();
        self.terminal.take();
    }
}

#[cfg(windows)]
pub fn collect(
    fetched_at: &str,
    cancellation: Arc<CancellationToken>,
) -> Result<crate::domain::UsageSnapshot, CollectionError> {
    let environment = GeminiEnvironment::prepare(
        &std::env::temp_dir(),
        std::env::vars_os().collect(),
        &[
            std::path::PathBuf::from(r"C:\ProgramData\gemini-cli\settings.json"),
            std::path::PathBuf::from(r"C:\ProgramData\gemini-cli\system-defaults.json"),
        ],
    )?;
    let candidate = discover(
        &environment,
        DiscoveryInputs::capture(None),
        Arc::clone(&cancellation),
    )?;
    let Some(candidate) = candidate else {
        let mut snapshot=crate::domain::UsageSnapshot::decode_compatible(&serde_json::json!({"schemaVersion":1,"providerId":"gemini","displayName":"Gemini","status":"notInstalled","fetchedAt":fetched_at,"staleAfterSeconds":300})).map_err(|_|CollectionError::InvalidResponse)?;
        snapshot.status_message=Some("Gemini CLI was not found in the supported native Windows locations. WSL collection is not supported. See the official CLI guide.".into());
        return Ok(snapshot);
    };
    if cancellation.is_cancelled() {
        return Err(CollectionError::Cancelled);
    }
    let mut terminal = NativeTerminal::open(&candidate, &environment)?;
    super::gemini_session::collect_session(
        &mut terminal,
        fetched_at,
        &cancellation,
        Default::default(),
    )
}
