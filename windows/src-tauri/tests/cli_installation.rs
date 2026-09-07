#[cfg(windows)]
use ai_token_meter_windows::accounts::cli_account::CliProvider;
use ai_token_meter_windows::accounts::installation::installer_command;
#[cfg(windows)]
use ai_token_meter_windows::accounts::installation::powershell_script;
use ai_token_meter_windows::accounts::installation::{InstallationDecision, installation_decision};
use ai_token_meter_windows::persistence::{CliRuntimeMode, ProviderCliSettings};
use ai_token_meter_windows::platform::windows::executable_locator::{
    CandidateOrigin, ExecutableCandidate, RuntimeSource,
};

#[test]
fn installer_environment_supports_windows_os_and_system_tools_without_inherited_path() {
    let root = std::path::Path::new("fixtureWindows");
    let command = installer_command(root, "exit 0").unwrap();
    let environment: std::collections::HashMap<_, _> = command
        .get_envs()
        .filter_map(|(key, value)| {
            value.map(|value| {
                (
                    key.to_string_lossy().to_string(),
                    value.to_string_lossy().to_string(),
                )
            })
        })
        .collect();
    assert_eq!(
        environment.get("OS").map(String::as_str),
        Some("Windows_NT")
    );
    assert!(environment["PATH"].contains("System32"));
    assert_eq!(
        std::env::split_paths(&environment["PATH"]).collect::<Vec<_>>(),
        vec![
            root.join("System32"),
            root.to_owned(),
            root.join("System32").join("WindowsPowerShell").join("v1.0")
        ]
    );
    for key in [
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "NODE_OPTIONS",
        "CODEX_INSTALL_DIR",
    ] {
        assert!(!environment.contains_key(key));
    }
}

#[test]
fn explicit_wsl_or_custom_path_never_installs_native() {
    for settings in [
        ProviderCliSettings {
            mode: CliRuntimeMode::Wsl,
            ..Default::default()
        },
        ProviderCliSettings {
            custom_path: Some("C:\\missing.exe".into()),
            ..Default::default()
        },
    ] {
        assert_eq!(
            installation_decision(&settings, None),
            InstallationDecision::ManualRequired
        );
    }
    assert_eq!(
        installation_decision(&Default::default(), None),
        InstallationDecision::Launch
    );
}

#[test]
fn discovered_native_and_wsl_candidates_are_preserved() {
    for source in [
        RuntimeSource::NativeWindows,
        RuntimeSource::Wsl {
            distribution: "Ubuntu".into(),
        },
    ] {
        let candidate = ExecutableCandidate {
            executable: "/existing/cli".into(),
            launcher: None,
            source,
            origin: CandidateOrigin::Conventional,
        };
        assert_eq!(
            installation_decision(&Default::default(), Some(&candidate)),
            InstallationDecision::AlreadyInstalled
        );
    }
}

#[cfg(windows)]
#[test]
fn generated_script_runs_only_a_complete_download_and_preserves_exit() {
    use std::fs;
    for provider in [CliProvider::Claude, CliProvider::Codex] {
        for fail in [true, false] {
            let dir = tempfile::tempdir().unwrap();
            let marker = dir.path().join("executed");
            let url_file = dir.path().join("url");
            let harness = format!(
                r#"if ($env:OS -ne 'Windows_NT') {{ exit 81 }}
if (-not (Get-Command tar.exe -ErrorAction SilentlyContinue)) {{ exit 82 }}
function Invoke-WebRequest {{ param($Uri,$OutFile,[switch]$UseBasicParsing) [IO.File]::WriteAllText('{}', $Uri); [IO.File]::WriteAllText($OutFile, "[IO.File]::WriteAllText('{}','yes'); exit 7"); {} }}
{}"#,
                url_file.display(),
                marker.display(),
                if fail { "throw 'fixture failure'" } else { "" },
                powershell_script(provider)
            );
            let root = std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap());
            let status = installer_command(&root, &harness)
                .unwrap()
                .status()
                .unwrap();
            assert_eq!(marker.exists(), !fail);
            assert_eq!(status.code(), Some(if fail { 1 } else { 7 }));
            assert_eq!(
                fs::read_to_string(url_file).unwrap(),
                match provider {
                    CliProvider::Claude => "https://claude.ai/install.ps1",
                    CliProvider::Codex => "https://chatgpt.com/codex/install.ps1",
                }
            );
        }
    }
}
