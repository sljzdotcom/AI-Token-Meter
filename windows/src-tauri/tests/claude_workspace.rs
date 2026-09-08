use std::ffi::OsString;
use std::path::{Path, PathBuf};

use ai_token_meter_windows::platform::windows::claude_workspace::{
    ClaudeUsageWorkspace, SETUP_SCRIPT, WSL_WORKSPACE,
};
use ai_token_meter_windows::platform::windows::executable_locator::{
    CandidateOrigin, ExecutableCandidate, RuntimeSource,
};

#[test]
fn native_collection_and_setup_share_an_application_owned_workspace() {
    let root = Path::new(r"C:\Users\Example\AppData\Local");
    let workspace = ClaudeUsageWorkspace::for_candidate(&native_candidate(), root);

    assert_eq!(
        workspace.working_directory(),
        Some(
            root.join("AI Token Meter")
                .join("ClaudeUsageWorkspace")
                .as_path()
        )
    );
    let collection = workspace
        .command(&native_candidate(), &["--ax-screen-reader", "--safe-mode"])
        .expect("collection command");
    let setup = workspace
        .setup_command(&native_candidate())
        .expect("setup command");
    assert_eq!(collection.executable, PathBuf::from(r"C:\Tools\claude.exe"));
    assert_eq!(setup.executable, collection.executable);
    assert_eq!(setup.arguments, collection.arguments);
    assert!(
        !setup
            .arguments
            .iter()
            .any(|value| value == "yes" || value == "--trust")
    );
}

#[test]
fn wsl_collection_and_setup_use_the_same_fixed_distribution_workspace() {
    let candidate = wsl_candidate("Ubuntu-24.04");
    let workspace = ClaudeUsageWorkspace::for_candidate(&candidate, Path::new(r"C:\Local"));
    assert_eq!(workspace.working_directory(), None);

    let prepare = workspace.prepare_command().expect("WSL preparation");
    assert_eq!(
        prepare.arguments,
        vec![
            "--distribution",
            "Ubuntu-24.04",
            "--exec",
            "/bin/sh",
            "-c",
            "umask 077 && mkdir -p -- \"$HOME/.local/share/ai-token-meter/ClaudeUsageWorkspace\"",
        ]
        .into_iter()
        .map(OsString::from)
        .collect::<Vec<_>>()
    );
    assert!(!SETUP_SCRIPT.contains("Ubuntu-24.04"));
    assert!(SETUP_SCRIPT.contains(WSL_WORKSPACE));

    let collection = workspace
        .command(&candidate, &["auth", "status"])
        .expect("collection command");
    let setup = workspace.setup_command(&candidate).expect("setup command");
    assert_eq!(
        collection.arguments,
        vec![
            "--distribution",
            "Ubuntu-24.04",
            "--exec",
            "/bin/sh",
            "-c",
            SETUP_SCRIPT,
            "ai-token-meter",
            "claude",
            "auth",
            "status",
        ]
        .into_iter()
        .map(OsString::from)
        .collect::<Vec<_>>()
    );
    assert_eq!(
        setup.arguments,
        vec![
            "--distribution",
            "Ubuntu-24.04",
            "--exec",
            "/bin/sh",
            "-c",
            SETUP_SCRIPT,
            "ai-token-meter",
            "claude",
            "--ax-screen-reader",
            "--safe-mode",
        ]
        .into_iter()
        .map(OsString::from)
        .collect::<Vec<_>>()
    );
    assert!(
        !setup
            .arguments
            .iter()
            .any(|value| value == "yes" || value == "--trust")
    );
}

fn native_candidate() -> ExecutableCandidate {
    ExecutableCandidate {
        executable: PathBuf::from(r"C:\Tools\claude.exe"),
        launcher: None,
        source: RuntimeSource::NativeWindows,
        origin: CandidateOrigin::Conventional,
    }
}

fn wsl_candidate(distribution: &str) -> ExecutableCandidate {
    ExecutableCandidate {
        executable: PathBuf::from(r"C:\Windows\System32\wsl.exe"),
        launcher: None,
        source: RuntimeSource::Wsl {
            distribution: distribution.to_owned(),
        },
        origin: CandidateOrigin::Wsl,
    }
}
