#[cfg(windows)]
use ai_token_meter_windows::accounts::cli_account::CliProvider;
#[cfg(windows)]
use ai_token_meter_windows::accounts::installation::powershell_script;
use ai_token_meter_windows::accounts::installation::{InstallationDecision, installation_decision};
use ai_token_meter_windows::persistence::{CliRuntimeMode, ProviderCliSettings};
use ai_token_meter_windows::platform::windows::executable_locator::{
    CandidateOrigin, ExecutableCandidate, RuntimeSource,
};

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
    use std::{fs, process::Command};
    for provider in [CliProvider::Claude, CliProvider::Codex] {
        for fail in [true, false] {
            let dir = tempfile::tempdir().unwrap();
            let marker = dir.path().join("executed");
            let url_file = dir.path().join("url");
            let harness = format!(
                r#"function Invoke-WebRequest {{ param($Uri,$OutFile,[switch]$UseBasicParsing) [IO.File]::WriteAllText('{}', $Uri); [IO.File]::WriteAllText($OutFile, "[IO.File]::WriteAllText('{}','yes'); exit 7"); {} }}
{}"#,
                url_file.display(),
                marker.display(),
                if fail { "throw 'fixture failure'" } else { "" },
                powershell_script(provider)
            );
            let path = dir.path().join("test.ps1");
            fs::write(&path, harness).unwrap();
            let status = Command::new("powershell.exe")
                .args(["-NoProfile", "-NonInteractive", "-File"])
                .arg(path)
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
