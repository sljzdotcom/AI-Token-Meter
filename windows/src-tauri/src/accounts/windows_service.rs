use std::process::Command;
use std::time::Duration;

use windows_sys::Win32::System::Threading::CREATE_NEW_CONSOLE;

use crate::accounts::service_status::{
    ServiceAccountStatus, deepseek_status, parse_claude_auth_status, sanitized_cli_version,
};
use crate::collectors::application::locate;
use crate::collectors::codex_app_server::collect_account_status_from_invocation;
use crate::domain::ProviderId;
use crate::persistence::{AppSettings, ProviderCliSettings};
use crate::platform::windows::credential_manager::WindowsCredentialManager;
use crate::platform::windows::process::{
    BoundedProcessRunner, ProcessRequest, command_for_candidate, configure_restricted_command,
};
use crate::security::{CredentialAccount, CredentialStore};

use super::claude::claude_login_command;
use super::cli_account::CliProvider;
use super::cli_discovery::{CliDiscovery, CliProbe, discover_cli};
use super::codex::codex_login_command;

pub async fn read_all(checked_at: &str, settings: AppSettings) -> Vec<ServiceAccountStatus> {
    let claude_checked_at = checked_at.to_owned();
    let codex_checked_at = checked_at.to_owned();
    let claude_settings = settings.claude_cli;
    let codex_settings = settings.codex_cli;
    let claude = tauri::async_runtime::spawn_blocking(move || {
        read_cli_status(CliProvider::Claude, &claude_settings, &claude_checked_at)
    });
    let codex = tauri::async_runtime::spawn_blocking(move || {
        read_cli_status(CliProvider::Codex, &codex_settings, &codex_checked_at)
    });
    let deepseek = read_deepseek_status(checked_at).await;
    vec![
        claude
            .await
            .unwrap_or_else(|_| ServiceAccountStatus::unavailable(ProviderId::Claude, checked_at)),
        codex
            .await
            .unwrap_or_else(|_| ServiceAccountStatus::unavailable(ProviderId::Codex, checked_at)),
        deepseek,
    ]
}

pub async fn read_one(
    provider: ProviderId,
    checked_at: &str,
    settings: AppSettings,
) -> ServiceAccountStatus {
    match provider {
        ProviderId::Claude => {
            let checked_at = checked_at.to_owned();
            let configuration = settings.claude_cli;
            tauri::async_runtime::spawn_blocking(move || {
                read_cli_status(CliProvider::Claude, &configuration, &checked_at)
            })
            .await
            .unwrap_or_else(|_| ServiceAccountStatus::unavailable(provider, &now_rfc3339()))
        }
        ProviderId::Codex => {
            let checked_at = checked_at.to_owned();
            let configuration = settings.codex_cli;
            tauri::async_runtime::spawn_blocking(move || {
                read_cli_status(CliProvider::Codex, &configuration, &checked_at)
            })
            .await
            .unwrap_or_else(|_| ServiceAccountStatus::unavailable(provider, &now_rfc3339()))
        }
        ProviderId::DeepSeek => read_deepseek_status(checked_at).await,
    }
}

pub fn launch_login(
    provider: CliProvider,
    configuration: &ProviderCliSettings,
) -> Result<(), &'static str> {
    use std::os::windows::process::CommandExt;

    let candidate = locate(provider, configuration).ok_or("CLI is not installed")?;
    let invocation = match provider {
        CliProvider::Claude => claude_login_command(&candidate),
        CliProvider::Codex => codex_login_command(&candidate),
    }
    .map_err(|_| "The sign-in command could not be prepared")?;
    let mut command = Command::new(&invocation.executable);
    configure_restricted_command(&mut command, &invocation.executable);
    command
        .args(&invocation.arguments)
        .creation_flags(CREATE_NEW_CONSOLE);
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        command.current_dir(profile);
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|_| "The sign-in window could not be opened")
}

pub fn launch_installation(
    provider: CliProvider,
    configuration: &ProviderCliSettings,
) -> Result<super::installation::InstallationDecision, &'static str> {
    use super::installation::{
        InstallationDecision, installation_decision_for_discovery, installer_command,
        powershell_script,
    };
    use std::os::windows::process::CommandExt;
    let discovery = discover_account_cli(provider, configuration);
    let decision = installation_decision_for_discovery(configuration, &discovery);
    if decision != InstallationDecision::Launch {
        return Ok(decision);
    }
    let script = powershell_script(provider);
    let system_root = std::env::var_os("SystemRoot").ok_or("Windows PowerShell is unavailable")?;
    let mut command = installer_command(std::path::Path::new(&system_root), &script)?;
    command.creation_flags(CREATE_NEW_CONSOLE);
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        command.current_dir(profile);
    }
    command
        .spawn()
        .map_err(|_| "The installation window could not be opened")?;
    Ok(InstallationDecision::Launch)
}

pub fn open_installation_guide(provider: CliProvider) -> Result<(), &'static str> {
    let url = match provider {
        CliProvider::Claude => "https://code.claude.com/docs/en/setup",
        CliProvider::Codex => "https://learn.chatgpt.com/docs/codex/cli",
    };
    let root =
        std::env::var_os("SystemRoot").ok_or("The installation guide could not be opened")?;
    let executable = std::path::PathBuf::from(root)
        .join("System32")
        .join("rundll32.exe");
    let mut command = Command::new(&executable);
    configure_restricted_command(&mut command, &executable);
    command
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()
        .map(|_| ())
        .map_err(|_| "The installation guide could not be opened")
}

fn read_cli_status(
    provider: CliProvider,
    configuration: &ProviderCliSettings,
    checked_at: &str,
) -> ServiceAccountStatus {
    let provider_id = match provider {
        CliProvider::Claude => ProviderId::Claude,
        CliProvider::Codex => ProviderId::Codex,
    };
    let candidate = match discover_account_cli(provider, configuration) {
        CliDiscovery::Found(candidate) => candidate,
        CliDiscovery::Missing => {
            return ServiceAccountStatus::not_installed(provider_id, checked_at);
        }
        CliDiscovery::Unavailable => {
            return ServiceAccountStatus::unavailable(provider_id, checked_at);
        }
    };
    let version = cli_version(&candidate, provider);
    let status = match provider {
        CliProvider::Claude => read_claude_status(&candidate, checked_at),
        CliProvider::Codex => read_codex_status(&candidate, checked_at),
    }
    .unwrap_or_else(|| ServiceAccountStatus::unavailable(provider_id, checked_at));
    status.with_runtime(&candidate.source, version)
}

fn discover_account_cli(
    provider: CliProvider,
    configuration: &ProviderCliSettings,
) -> CliDiscovery {
    use crate::platform::windows::{
        environment::DiscoveryInputs, executable_locator::DiscoveryBudget,
        wsl::build_wsl_list_invocation,
    };
    let inputs = DiscoveryInputs::capture(None);
    let system_root = inputs.system_root.clone();
    let budget = std::cell::RefCell::new(DiscoveryBudget::new(Duration::from_secs(8), 12));
    discover_cli(
        provider,
        configuration,
        inputs,
        |candidate| probe_account_candidate(candidate, provider, &mut budget.borrow_mut()),
        || {
            let Some(root) = system_root.as_deref() else {
                return Err(());
            };
            let Some(invocation) = build_wsl_list_invocation(root) else {
                return if matches!(
                    root.join("System32").join("wsl.exe").try_exists(),
                    Ok(false)
                ) {
                    Ok(None)
                } else {
                    Err(())
                };
            };
            let timeout = budget
                .borrow_mut()
                .next_process_timeout(Duration::from_secs(3))
                .ok_or(())?;
            let mut request = ProcessRequest::new(
                invocation.executable,
                invocation.arguments.into_iter().map(Into::into).collect(),
            );
            request.timeout = timeout;
            request.max_output_bytes = 16 * 1024;
            let output = BoundedProcessRunner.run(request).map_err(|_| ())?;
            if output.exit_code == Some(0) {
                Ok(Some(output.stdout.into_bytes()))
            } else {
                Err(())
            }
        },
    )
}

fn probe_account_candidate(
    candidate: &crate::platform::windows::executable_locator::ExecutableCandidate,
    provider: CliProvider,
    budget: &mut crate::platform::windows::executable_locator::DiscoveryBudget,
) -> CliProbe {
    use crate::platform::windows::executable_locator::RuntimeSource;
    if let RuntimeSource::Wsl { distribution } = &candidate.source {
        // Only this fixed provider-enum command enters the shell; the distribution is a separate argument.
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
        match BoundedProcessRunner.run(request) {
            Ok(output) if output.exit_code == Some(1) => return CliProbe::Missing,
            Ok(output) if output.exit_code == Some(0) => {}
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
    if BoundedProcessRunner
        .run(request)
        .is_ok_and(|output| output.exit_code == Some(0))
    {
        CliProbe::Healthy
    } else {
        CliProbe::Unavailable
    }
}

fn read_claude_status(
    candidate: &crate::platform::windows::executable_locator::ExecutableCandidate,
    checked_at: &str,
) -> Option<ServiceAccountStatus> {
    let invocation = command_for_candidate(
        candidate,
        CliProvider::Claude,
        &["auth", "status", "--json"],
    )
    .ok()?;
    let mut request = ProcessRequest::new(invocation.executable, invocation.arguments);
    request.timeout = Duration::from_secs(5);
    request.max_output_bytes = 32 * 1024;
    let output = BoundedProcessRunner.run(request).ok()?;
    parse_claude_auth_status(&format!("{}\n{}", output.stdout, output.stderr), checked_at).ok()
}

fn read_codex_status(
    candidate: &crate::platform::windows::executable_locator::ExecutableCandidate,
    checked_at: &str,
) -> Option<ServiceAccountStatus> {
    let invocation =
        command_for_candidate(candidate, CliProvider::Codex, &["app-server", "--stdio"]).ok()?;
    collect_account_status_from_invocation(
        &invocation,
        std::env::var_os("USERPROFILE")
            .map(std::path::PathBuf::from)
            .as_deref(),
        checked_at,
        Duration::from_secs(8),
    )
    .ok()
}

fn cli_version(
    candidate: &crate::platform::windows::executable_locator::ExecutableCandidate,
    provider: CliProvider,
) -> Option<String> {
    let invocation = command_for_candidate(candidate, provider, &["--version"]).ok()?;
    let mut request = ProcessRequest::new(invocation.executable, invocation.arguments);
    request.timeout = Duration::from_secs(4);
    request.max_output_bytes = 16 * 1024;
    let output = BoundedProcessRunner.run(request).ok()?;
    sanitized_cli_version(&format!("{}\n{}", output.stdout, output.stderr))
}

async fn read_deepseek_status(checked_at: &str) -> ServiceAccountStatus {
    let credentials = WindowsCredentialManager::new();
    match credentials.read(CredentialAccount::DeepSeekApiKey).await {
        Ok(secret) => deepseek_status(secret.as_ref().map(|value| value.expose()), checked_at),
        Err(_) => ServiceAccountStatus::unavailable(ProviderId::DeepSeek, checked_at),
    }
}

fn now_rfc3339() -> String {
    use time::format_description::well_known::Rfc3339;
    time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

#[cfg(test)]
mod discovery_tests {
    use super::*;
    use crate::platform::windows::environment::DiscoveryInputs;
    use crate::platform::windows::executable_locator::{DiscoveryBudget, ExecutableLocator};

    #[test]
    fn failed_and_timed_out_real_native_probe_cannot_authorize_installation() {
        for (contents, slow) in [
            ("@exit /b 1\r\n", false),
            ("@for /l %%i in (1,1,2147483647) do @rem wait\r\n", true),
        ] {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("codex.cmd");
            std::fs::write(&path, contents).unwrap();
            let candidate = ExecutableLocator::new(DiscoveryInputs::capture(None))
                .validate_custom(CliProvider::Codex, &path, |_| true)
                .unwrap();
            let mut budget = DiscoveryBudget::new(Duration::from_millis(100), 1);
            let started = std::time::Instant::now();
            assert!(matches!(
                probe_account_candidate(&candidate, CliProvider::Codex, &mut budget),
                CliProbe::Unavailable
            ));
            if slow {
                assert!(started.elapsed() >= Duration::from_millis(80));
            }
            assert_eq!(
                super::super::installation::installation_decision_for_discovery(
                    &ProviderCliSettings::default(),
                    &CliDiscovery::Unavailable
                ),
                super::super::installation::InstallationDecision::Unavailable
            );
        }
    }
}
