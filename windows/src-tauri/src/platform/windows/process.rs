use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use regex::Regex;

use crate::accounts::cli_account::CliProvider;

use super::executable_locator::{ExecutableCandidate, RuntimeSource};
use super::wsl::build_wsl_invocation;

const READ_CHUNK_SIZE: usize = 4096;
const READER_QUEUE_DEPTH: usize = 8;
const POLL_INTERVAL: Duration = Duration::from_millis(5);
const OUTPUT_DRAIN_GRACE: Duration = Duration::from_secs(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandInvocation {
    pub executable: PathBuf,
    pub arguments: Vec<OsString>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandBuildError {
    UnsupportedLauncher,
    UnsafeCommandWrapperPath,
    UnsafeCommandWrapperArgument,
}

pub fn command_for_candidate(
    candidate: &ExecutableCandidate,
    provider: CliProvider,
    provider_arguments: &[&str],
) -> Result<CommandInvocation, CommandBuildError> {
    if let RuntimeSource::Wsl { distribution } = &candidate.source {
        let arguments = provider_arguments
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>();
        let invocation = build_wsl_invocation(
            candidate.executable.clone(),
            distribution,
            provider,
            &arguments,
        );
        return Ok(CommandInvocation {
            executable: invocation.executable,
            arguments: invocation
                .arguments
                .into_iter()
                .map(OsString::from)
                .collect(),
        });
    }

    let Some(launcher) = candidate.launcher.as_ref() else {
        return Ok(CommandInvocation {
            executable: candidate.executable.clone(),
            arguments: provider_arguments.iter().map(OsString::from).collect(),
        });
    };
    let launcher_name = launcher
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default();
    if launcher_name.eq_ignore_ascii_case("node.exe") {
        let mut arguments = vec![interpreter_path(&candidate.executable).into_os_string()];
        arguments.extend(provider_arguments.iter().map(OsString::from));
        return Ok(CommandInvocation {
            executable: launcher.clone(),
            arguments,
        });
    }
    if launcher_name.eq_ignore_ascii_case("cmd.exe") {
        if contains_cmd_metacharacter(candidate.executable.as_os_str()) {
            return Err(CommandBuildError::UnsafeCommandWrapperPath);
        }
        if provider_arguments
            .iter()
            .any(|argument| contains_cmd_metacharacter(OsStr::new(argument)))
        {
            return Err(CommandBuildError::UnsafeCommandWrapperArgument);
        }
        let mut arguments = vec![
            OsString::from("/d"),
            OsString::from("/s"),
            OsString::from("/c"),
            interpreter_path(&candidate.executable).into_os_string(),
        ];
        arguments.extend(provider_arguments.iter().map(OsString::from));
        return Ok(CommandInvocation {
            executable: launcher.clone(),
            arguments,
        });
    }
    Err(CommandBuildError::UnsupportedLauncher)
}

fn contains_cmd_metacharacter(path: &OsStr) -> bool {
    path.to_string_lossy().chars().any(|character| {
        matches!(
            character,
            '&' | '|' | '<' | '>' | '^' | '%' | '!' | '\n' | '\r'
        )
    })
}

fn interpreter_path(path: &Path) -> PathBuf {
    let value = path.as_os_str().to_string_lossy();
    if let Some(network_path) = value.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{network_path}"));
    }
    value
        .strip_prefix(r"\\?\")
        .map_or_else(|| path.to_owned(), PathBuf::from)
}

#[derive(Clone)]
pub struct ProcessRequest {
    pub executable: PathBuf,
    pub arguments: Vec<OsString>,
    pub working_directory: Option<PathBuf>,
    pub environment: Vec<(OsString, OsString)>,
    pub timeout: Duration,
    pub max_output_bytes: usize,
    pub cancellation: Arc<CancellationToken>,
}

impl ProcessRequest {
    pub fn new(executable: PathBuf, arguments: Vec<OsString>) -> Self {
        Self {
            executable,
            arguments,
            working_directory: None,
            environment: Vec::new(),
            timeout: Duration::from_secs(10),
            max_output_bytes: 64 * 1024,
            cancellation: Arc::new(CancellationToken::new()),
        }
    }
}

#[derive(Debug, Default)]
pub struct CancellationToken {
    cancelled: AtomicBool,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ProcessOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl fmt::Debug for ProcessOutput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ProcessOutput([REDACTED])")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessErrorKind {
    InvalidExecutable,
    StartFailed,
    ProcessControlFailed,
    TimedOut,
    Cancelled,
    OutputLimitExceeded,
    OutputDrainFailed,
}

pub struct ProcessRunError {
    kind: ProcessErrorKind,
}

impl ProcessRunError {
    fn new(kind: ProcessErrorKind) -> Self {
        Self { kind }
    }

    pub fn kind(&self) -> ProcessErrorKind {
        self.kind
    }
}

impl fmt::Debug for ProcessRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ProcessRunError({:?})", self.kind)
    }
}

#[derive(Default)]
pub struct BoundedProcessRunner;

impl BoundedProcessRunner {
    pub fn run(&self, request: ProcessRequest) -> Result<ProcessOutput, ProcessRunError> {
        let executable = validate_executable(&request.executable)?;
        let mut command = Command::new(executable);
        command
            .args(&request.arguments)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in inherited_windows_environment() {
            command.env(key, value);
        }
        if let Some(parent) = request.executable.parent() {
            command.env("PATH", parent);
        }
        for (key, value) in &request.environment {
            command.env(key, value);
        }
        if let Some(directory) = &request.working_directory {
            command.current_dir(directory);
        }
        #[cfg(windows)]
        configure_windows_process(&mut command);

        let job = ProcessJob::create()?;
        let mut child = command
            .spawn()
            .map_err(|_| ProcessRunError::new(ProcessErrorKind::StartFailed))?;
        if let Err(error) = job.assign(&child) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ProcessRunError::new(ProcessErrorKind::StartFailed))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| ProcessRunError::new(ProcessErrorKind::StartFailed))?;
        let (sender, receiver) = mpsc::sync_channel(READER_QUEUE_DEPTH);
        spawn_reader(stdout, OutputStream::Stdout, sender.clone());
        spawn_reader(stderr, OutputStream::Stderr, sender);

        let started = Instant::now();
        let mut stdout_bytes = Vec::new();
        let mut stderr_bytes = Vec::new();
        let mut completed_readers = 0;
        let mut failure = None;
        let exit_status = loop {
            collect_available(
                &receiver,
                &mut stdout_bytes,
                &mut stderr_bytes,
                &mut completed_readers,
                request.max_output_bytes,
                &mut failure,
            );
            if failure.is_none() && request.cancellation.is_cancelled() {
                failure = Some(ProcessErrorKind::Cancelled);
            }
            if failure.is_none() && started.elapsed() >= request.timeout {
                failure = Some(ProcessErrorKind::TimedOut);
            }
            if failure.is_some() {
                terminate_process(&job, &mut child);
                break child.wait().ok();
            }
            match child.try_wait() {
                Ok(Some(status)) => {
                    job.terminate_descendants();
                    break Some(status);
                }
                Ok(None) => thread::sleep(POLL_INTERVAL),
                Err(_) => {
                    failure = Some(ProcessErrorKind::ProcessControlFailed);
                    terminate_process(&job, &mut child);
                    break child.wait().ok();
                }
            }
        };

        drain_readers(
            &receiver,
            &mut stdout_bytes,
            &mut stderr_bytes,
            &mut completed_readers,
            request.max_output_bytes,
            &mut failure,
        );
        if let Some(kind) = failure {
            return Err(ProcessRunError::new(kind));
        }
        Ok(ProcessOutput {
            exit_code: exit_status.and_then(|status| status.code()),
            stdout: normalize_output(&stdout_bytes),
            stderr: normalize_output(&stderr_bytes),
        })
    }
}

fn validate_executable(path: &Path) -> Result<PathBuf, ProcessRunError> {
    let canonical = path
        .canonicalize()
        .map_err(|_| ProcessRunError::new(ProcessErrorKind::InvalidExecutable))?;
    canonical
        .metadata()
        .is_ok_and(|metadata| metadata.is_file())
        .then_some(canonical)
        .ok_or_else(|| ProcessRunError::new(ProcessErrorKind::InvalidExecutable))
}

fn inherited_windows_environment() -> Vec<(OsString, OsString)> {
    #[cfg(windows)]
    {
        ["SystemRoot", "WINDIR", "TEMP", "TMP"]
            .into_iter()
            .filter_map(|key| std::env::var_os(key).map(|value| (OsString::from(key), value)))
            .collect()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

pub(crate) fn configure_restricted_command(command: &mut Command, executable: &Path) {
    command.env_clear();
    for (key, value) in restricted_environment_for(executable) {
        command.env(key, value);
    }
    #[cfg(windows)]
    configure_windows_process(command);
}

pub(crate) fn restricted_environment_for(executable: &Path) -> Vec<(OsString, OsString)> {
    let mut environment = inherited_windows_environment();
    if let Some(parent) = executable.parent() {
        environment.push((OsString::from("PATH"), parent.as_os_str().to_owned()));
    }
    environment
}

#[derive(Clone, Copy)]
enum OutputStream {
    Stdout,
    Stderr,
}

enum ReaderMessage {
    Bytes(OutputStream, Vec<u8>),
    Complete,
}

fn spawn_reader<R>(mut reader: R, stream: OutputStream, sender: SyncSender<ReaderMessage>)
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut buffer = [0_u8; READ_CHUNK_SIZE];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(count) => {
                    if sender
                        .send(ReaderMessage::Bytes(stream, buffer[..count].to_vec()))
                        .is_err()
                    {
                        return;
                    }
                }
            }
        }
        let _ = sender.send(ReaderMessage::Complete);
    });
}

fn collect_available(
    receiver: &Receiver<ReaderMessage>,
    stdout: &mut Vec<u8>,
    stderr: &mut Vec<u8>,
    completed_readers: &mut usize,
    limit: usize,
    failure: &mut Option<ProcessErrorKind>,
) {
    loop {
        match receiver.try_recv() {
            Ok(message) => {
                collect_message(message, stdout, stderr, completed_readers, limit, failure)
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => return,
        }
    }
}

fn collect_message(
    message: ReaderMessage,
    stdout: &mut Vec<u8>,
    stderr: &mut Vec<u8>,
    completed_readers: &mut usize,
    limit: usize,
    failure: &mut Option<ProcessErrorKind>,
) {
    match message {
        ReaderMessage::Complete => *completed_readers += 1,
        ReaderMessage::Bytes(stream, bytes) => {
            let current = stdout.len().saturating_add(stderr.len());
            if bytes.len() > limit.saturating_sub(current) {
                *failure = Some(ProcessErrorKind::OutputLimitExceeded);
                return;
            }
            match stream {
                OutputStream::Stdout => stdout.extend_from_slice(&bytes),
                OutputStream::Stderr => stderr.extend_from_slice(&bytes),
            }
        }
    }
}

fn drain_readers(
    receiver: &Receiver<ReaderMessage>,
    stdout: &mut Vec<u8>,
    stderr: &mut Vec<u8>,
    completed_readers: &mut usize,
    limit: usize,
    failure: &mut Option<ProcessErrorKind>,
) {
    let deadline = Instant::now() + OUTPUT_DRAIN_GRACE;
    while *completed_readers < 2 && Instant::now() < deadline {
        match receiver.recv_timeout(POLL_INTERVAL) {
            Ok(message) => {
                collect_message(message, stdout, stderr, completed_readers, limit, failure)
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    if *completed_readers < 2 && failure.is_none() {
        *failure = Some(ProcessErrorKind::OutputDrainFailed);
    }
}

fn terminate_process(job: &ProcessJob, child: &mut std::process::Child) {
    job.terminate_descendants();
    let _ = child.kill();
}

pub fn normalize_output(bytes: &[u8]) -> String {
    let decoded = decode_text(bytes);
    static ANSI: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    ANSI.get_or_init(|| {
        Regex::new(r"\x1B(?:\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\))")
            .expect("constant ANSI pattern")
    })
    .replace_all(&decoded, "")
    .into_owned()
}

fn decode_text(bytes: &[u8]) -> String {
    let utf16 = bytes.starts_with(&[0xff, 0xfe])
        || (bytes.len() >= 4
            && bytes.len().is_multiple_of(2)
            && bytes
                .iter()
                .skip(1)
                .step_by(2)
                .filter(|byte| **byte == 0)
                .count()
                * 2
                >= bytes.len() / 2);
    if utf16 {
        let offset = usize::from(bytes.starts_with(&[0xff, 0xfe])) * 2;
        let (pairs, _) = bytes[offset..].as_chunks::<2>();
        let words = pairs
            .iter()
            .map(|pair| u16::from_le_bytes(*pair))
            .collect::<Vec<_>>();
        String::from_utf16_lossy(&words)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

#[cfg(windows)]
fn configure_windows_process(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

    command.creation_flags(CREATE_NO_WINDOW);
}

pub(crate) struct ProcessJob {
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HANDLE,
}

impl ProcessJob {
    pub(crate) fn create() -> Result<Self, ProcessRunError> {
        #[cfg(windows)]
        {
            use std::mem::{size_of, zeroed};
            use std::ptr;
            use windows_sys::Win32::System::JobObjects::{
                CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
                SetInformationJobObject,
            };

            let handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
            if handle.is_null() {
                return Err(ProcessRunError::new(ProcessErrorKind::ProcessControlFailed));
            }
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let configured = unsafe {
                SetInformationJobObject(
                    handle,
                    JobObjectExtendedLimitInformation,
                    (&raw const limits).cast(),
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            };
            if configured == 0 {
                unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) };
                return Err(ProcessRunError::new(ProcessErrorKind::ProcessControlFailed));
            }
            Ok(Self { handle })
        }
        #[cfg(not(windows))]
        {
            Ok(Self {})
        }
    }

    pub(crate) fn assign(&self, child: &std::process::Child) -> Result<(), ProcessRunError> {
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            use windows_sys::Win32::System::JobObjects::AssignProcessToJobObject;

            let assigned = unsafe {
                AssignProcessToJobObject(self.handle, child.as_raw_handle() as isize as _)
            };
            if assigned == 0 {
                return Err(ProcessRunError::new(ProcessErrorKind::ProcessControlFailed));
            }
        }
        #[cfg(not(windows))]
        let _ = child;
        Ok(())
    }

    pub(crate) fn terminate_descendants(&self) {
        #[cfg(windows)]
        unsafe {
            windows_sys::Win32::System::JobObjects::TerminateJobObject(self.handle, 1);
        }
    }
}

#[cfg(windows)]
impl Drop for ProcessJob {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}
