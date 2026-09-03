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

fn read_cli_status(
    provider: CliProvider,
    configuration: &ProviderCliSettings,
    checked_at: &str,
) -> ServiceAccountStatus {
    let provider_id = match provider {
        CliProvider::Claude => ProviderId::Claude,
        CliProvider::Codex => ProviderId::Codex,
    };
    let Some(candidate) = locate(provider, configuration) else {
        return ServiceAccountStatus::not_installed(provider_id, checked_at);
    };
    let version = cli_version(&candidate, provider);
    let status = match provider {
        CliProvider::Claude => read_claude_status(&candidate, checked_at),
        CliProvider::Codex => read_codex_status(&candidate, checked_at),
    }
    .unwrap_or_else(|| ServiceAccountStatus::unavailable(provider_id, checked_at));
    status.with_runtime(&candidate.source, version)
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
