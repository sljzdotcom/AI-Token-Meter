use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use ai_token_meter_windows::accounts::claude::claude_login_command;
use ai_token_meter_windows::accounts::cli_account::CliProvider;
use ai_token_meter_windows::accounts::codex::codex_login_command;
use ai_token_meter_windows::platform::windows::executable_locator::{
    CandidateOrigin, ExecutableCandidate, RuntimeSource,
};
use ai_token_meter_windows::platform::windows::process::{
    BoundedProcessRunner, CancellationToken, CommandBuildError, ProcessErrorKind, ProcessRequest,
    command_for_candidate,
};

#[test]
fn arguments_with_spaces_and_shell_metacharacters_remain_separate() {
    let output = runner()
        .run(request(
            "arguments",
            &["two words", "& whoami", "$(touch forbidden)", "你好"],
        ))
        .expect("fixture output");

    let arguments: Vec<String> = serde_json::from_str(&output.stdout).expect("argument JSON");
    assert_eq!(
        arguments,
        ["two words", "& whoami", "$(touch forbidden)", "你好"]
    );
}

#[test]
fn output_larger_than_the_limit_is_stopped_without_returning_raw_output() {
    let mut request = request("flood", &[]);
    request.max_output_bytes = 8 * 1024;

    let error = runner().run(request).expect_err("oversized output");

    assert_eq!(error.kind(), ProcessErrorKind::OutputLimitExceeded);
    assert_eq!(format!("{error:?}"), "ProcessRunError(OutputLimitExceeded)");
}

#[test]
fn timeout_terminates_the_process() {
    let mut request = request("sleep", &[]);
    request.timeout = Duration::from_millis(80);

    let error = runner().run(request).expect_err("timeout");

    assert_eq!(error.kind(), ProcessErrorKind::TimedOut);
}

#[test]
fn cancellation_terminates_the_process() {
    let mut request = request("sleep", &[]);
    request.timeout = Duration::from_secs(5);
    let cancellation = Arc::new(CancellationToken::new());
    request.cancellation = cancellation.clone();
    let handle = thread::spawn(move || runner().run(request));

    thread::sleep(Duration::from_millis(40));
    cancellation.cancel();
    let error = handle
        .join()
        .expect("runner thread")
        .expect_err("cancelled");

    assert_eq!(error.kind(), ProcessErrorKind::Cancelled);
}

#[test]
fn utf16_and_ansi_are_normalized_before_parsing() {
    let output = runner()
        .run(request("encoded", &[]))
        .expect("encoded output");

    assert_eq!(output.stdout, "安全输出\r\n");
}

#[test]
fn working_directory_and_minimal_explicit_environment_are_applied() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let mut request = request("environment", &[]);
    request.working_directory = Some(directory.path().to_owned());
    request.environment.push((
        OsString::from("AI_METER_ALLOWED"),
        OsString::from("fixture-value"),
    ));
    let expected_path = request
        .executable
        .parent()
        .expect("Node parent")
        .to_string_lossy()
        .into_owned();

    let output = runner().run(request).expect("environment output");
    let value: serde_json::Value = serde_json::from_str(&output.stdout).expect("environment JSON");

    let reported_cwd = PathBuf::from(value["cwd"].as_str().expect("working directory string"))
        .canonicalize()
        .expect("canonical reported working directory");
    assert_eq!(
        reported_cwd,
        directory
            .path()
            .canonicalize()
            .expect("canonical working directory")
    );
    assert_eq!(value["allowed"], "fixture-value");
    assert_eq!(value["path"], expected_path);
    assert_eq!(format!("{output:?}"), "ProcessOutput([REDACTED])");
}

#[test]
fn provider_login_actions_are_fixed_for_native_and_wsl_sources() {
    let native = ExecutableCandidate {
        executable: PathBuf::from(r"C:\Tools\claude.exe"),
        launcher: None,
        source: RuntimeSource::NativeWindows,
        origin: CandidateOrigin::Custom,
    };
    let wsl = ExecutableCandidate {
        executable: PathBuf::from(r"C:\Windows\System32\wsl.exe"),
        launcher: None,
        source: RuntimeSource::Wsl {
            distribution: "Ubuntu Test & whoami".to_owned(),
        },
        origin: CandidateOrigin::Wsl,
    };

    let claude = claude_login_command(&native).expect("Claude login command");
    assert_eq!(claude.executable, native.executable);
    assert_eq!(claude.arguments, ["auth", "login"]);

    let codex = codex_login_command(&wsl).expect("Codex WSL login command");
    assert_eq!(codex.executable, wsl.executable);
    assert_eq!(
        codex.arguments,
        [
            "--distribution",
            "Ubuntu Test & whoami",
            "--exec",
            CliProvider::Codex.command_name(),
            "login"
        ]
    );
}

#[test]
fn node_and_cmd_launchers_are_explicit_and_unsafe_cmd_paths_are_rejected() {
    let node_candidate = ExecutableCandidate {
        executable: PathBuf::from("codex"),
        launcher: Some(PathBuf::from("node.exe")),
        source: RuntimeSource::NativeWindows,
        origin: CandidateOrigin::Custom,
    };
    let node = command_for_candidate(&node_candidate, CliProvider::Codex, &["login"])
        .expect("Node invocation");
    assert_eq!(node.executable, PathBuf::from("node.exe"));
    assert_eq!(strings(&node.arguments), ["codex", "login"]);

    let unsafe_cmd = ExecutableCandidate {
        executable: PathBuf::from("Tools & whoami/claude.cmd"),
        launcher: Some(PathBuf::from("cmd.exe")),
        source: RuntimeSource::NativeWindows,
        origin: CandidateOrigin::Custom,
    };
    assert_eq!(
        command_for_candidate(&unsafe_cmd, CliProvider::Claude, &["auth", "login"]),
        Err(CommandBuildError::UnsafeCommandWrapperPath)
    );
}

#[test]
fn interpreter_arguments_remove_windows_extended_path_prefixes() {
    let node_candidate = ExecutableCandidate {
        executable: PathBuf::from(r"\\?\C:\Users\Example\AppData\Roaming\npm\codex"),
        launcher: Some(PathBuf::from("node.exe")),
        source: RuntimeSource::NativeWindows,
        origin: CandidateOrigin::Custom,
    };
    let node = command_for_candidate(&node_candidate, CliProvider::Codex, &["login"])
        .expect("Node invocation");
    assert_eq!(
        node.arguments[0],
        OsString::from(r"C:\Users\Example\AppData\Roaming\npm\codex")
    );

    let network_candidate = ExecutableCandidate {
        executable: PathBuf::from(r"\\?\UNC\server\tools\claude.cmd"),
        launcher: Some(PathBuf::from("cmd.exe")),
        source: RuntimeSource::NativeWindows,
        origin: CandidateOrigin::Custom,
    };
    let cmd = command_for_candidate(&network_candidate, CliProvider::Claude, &["auth", "status"])
        .expect("CMD invocation");
    assert_eq!(
        cmd.arguments[3],
        OsString::from(r"\\server\tools\claude.cmd")
    );
}

#[cfg(windows)]
#[test]
fn a_parent_exit_does_not_leave_a_descendant_running() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let sentinel = directory.path().join("descendant-survived");
    let mut request = request("spawn-child", &[sentinel.to_str().expect("sentinel path")]);
    request.timeout = Duration::from_secs(3);

    runner().run(request).expect("parent exits normally");
    thread::sleep(Duration::from_millis(900));

    assert!(
        !sentinel.exists(),
        "the Windows Job Object must terminate descendants"
    );
}

fn runner() -> BoundedProcessRunner {
    BoundedProcessRunner
}

fn request(mode: &str, arguments: &[&str]) -> ProcessRequest {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("process-fixture.js");
    let mut values = vec![fixture.into_os_string(), OsString::from(mode)];
    values.extend(arguments.iter().map(OsString::from));
    ProcessRequest::new(find_node(), values)
}

fn find_node() -> PathBuf {
    let names = if cfg!(windows) {
        ["node.exe", "node.exe"]
    } else {
        ["node", "node"]
    };
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .flat_map(|directory| names.map(|name| directory.join(name)))
        .find_map(|path| path.canonicalize().ok().filter(|path| path.is_file()))
        .expect("Node.js is required by the Windows frontend test toolchain")
}

fn strings(arguments: &[OsString]) -> Vec<String> {
    arguments
        .iter()
        .map(|value| value.to_string_lossy().into_owned())
        .collect()
}
