use std::process::Command;
use std::time::Duration;

use windows_sys::Win32::System::Threading::CREATE_NEW_CONSOLE;

use crate::accounts::service_status::{
    ServiceAccountStatus, deepseek_status, parse_claude_auth_status, sanitized_cli_version,
};
use crate::collectors::codex_app_server::collect_account_status_from_invocation;
use crate::domain::ProviderId;
use crate::persistence::{AppSettings, ProviderCliSettings};
use crate::platform::windows::claude_workspace::ClaudeUsageWorkspace;
use crate::platform::windows::credential_manager::WindowsCredentialManager;
use crate::platform::windows::process::{
    BoundedProcessRunner, CancellationToken, ProcessRequest, command_for_candidate,
    configure_restricted_command,
};
use crate::security::{CredentialAccount, CredentialStore};

use super::claude::claude_login_command;
use super::cli_account::CliProvider;
use super::cli_discovery::CliDiscovery;
use super::codex::codex_login_command;
use super::runtime_discovery::discover_runtime_cli;

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

    let candidate = match discover_account_cli(provider, configuration) {
        CliDiscovery::Found(candidate) => candidate,
        CliDiscovery::Missing => return Err("CLI is not installed"),
        CliDiscovery::Unavailable | CliDiscovery::Cancelled => {
            return Err("CLI is temporarily unavailable");
        }
    };
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

pub fn launch_claude_usage_initialization(
    configuration: &ProviderCliSettings,
) -> Result<(), &'static str> {
    use std::os::windows::process::CommandExt;

    let candidate = match discover_account_cli(CliProvider::Claude, configuration) {
        CliDiscovery::Found(candidate) => candidate,
        CliDiscovery::Missing => return Err("CLI is not installed"),
        CliDiscovery::Unavailable | CliDiscovery::Cancelled => {
            return Err("CLI is temporarily unavailable");
        }
    };
    let local_app_data = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let workspace = ClaudeUsageWorkspace::for_candidate(&candidate, &local_app_data);
    if let Some(directory) = workspace.working_directory() {
        std::fs::create_dir_all(directory)
            .map_err(|_| "The private workspace could not be created")?;
    } else if let Some(invocation) = workspace.prepare_command() {
        let mut request = ProcessRequest::new(invocation.executable, invocation.arguments);
        request.timeout = Duration::from_secs(3);
        request.max_output_bytes = 4 * 1024;
        let output = BoundedProcessRunner
            .run(request)
            .map_err(|_| "The private workspace could not be created")?;
        if output.exit_code != Some(0) {
            return Err("The private workspace could not be created");
        }
    }
    let invocation = workspace
        .setup_command(&candidate)
        .map_err(|_| "The initialization command could not be prepared")?;
    let mut command = Command::new(&invocation.executable);
    configure_restricted_command(&mut command, &invocation.executable);
    command
        .args(&invocation.arguments)
        .creation_flags(CREATE_NEW_CONSOLE);
    if let Some(directory) = workspace.working_directory() {
        command.current_dir(directory);
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|_| "The Claude Code setup window could not be opened")
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
        CliDiscovery::Cancelled => {
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
    discover_runtime_cli(
        provider,
        configuration,
        std::sync::Arc::new(CancellationToken::new()),
    )
}

fn read_claude_status(
    candidate: &crate::platform::windows::executable_locator::ExecutableCandidate,
    checked_at: &str,
) -> Option<ServiceAccountStatus> {
    let local_app_data = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let workspace = ClaudeUsageWorkspace::for_candidate(candidate, &local_app_data);
    prepare_claude_workspace(&workspace)?;
    let invocation = workspace
        .command(candidate, &["auth", "status", "--json"])
        .ok()?;
    let mut request = ProcessRequest::new(invocation.executable, invocation.arguments);
    request.working_directory = workspace.working_directory().map(std::path::Path::to_owned);
    request.timeout = Duration::from_secs(5);
    request.max_output_bytes = 32 * 1024;
    let output = BoundedProcessRunner.run(request).ok()?;
    parse_claude_auth_status(&format!("{}\n{}", output.stdout, output.stderr), checked_at).ok()
}

fn prepare_claude_workspace(workspace: &ClaudeUsageWorkspace) -> Option<()> {
    if let Some(directory) = workspace.working_directory() {
        return std::fs::create_dir_all(directory).ok();
    }
    let invocation = workspace.prepare_command()?;
    let mut request = ProcessRequest::new(invocation.executable, invocation.arguments);
    request.timeout = Duration::from_secs(3);
    request.max_output_bytes = 4 * 1024;
    BoundedProcessRunner
        .run(request)
        .ok()
        .filter(|output| output.exit_code == Some(0))
        .map(|_| ())
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
