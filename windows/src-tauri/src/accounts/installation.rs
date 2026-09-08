use super::cli_account::CliProvider;
use crate::persistence::{CliRuntimeMode, ProviderCliSettings};
use crate::platform::windows::executable_locator::ExecutableCandidate;

#[derive(Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallationDecision {
    Launch,
    AlreadyInstalled,
    ManualRequired,
    Unavailable,
}

pub fn installation_decision_for_discovery(
    settings: &ProviderCliSettings,
    discovery: &super::cli_discovery::CliDiscovery,
) -> InstallationDecision {
    use super::cli_discovery::CliDiscovery;
    match discovery {
        CliDiscovery::Found(candidate) => installation_decision(settings, Some(candidate)),
        CliDiscovery::Missing => installation_decision(settings, None),
        CliDiscovery::Unavailable | CliDiscovery::Cancelled => {
            match installation_decision(settings, None) {
                InstallationDecision::ManualRequired => InstallationDecision::ManualRequired,
                _ => InstallationDecision::Unavailable,
            }
        }
    }
}

/// An installer needs Windows system tools, but must not inherit credentials or CLI overrides.
pub fn installer_command(
    system_root: &std::path::Path,
    script: &str,
) -> Result<std::process::Command, &'static str> {
    use base64::Engine;
    let system32 = system_root.join("System32");
    let powershell = system32.join("WindowsPowerShell").join("v1.0");
    let mut command = std::process::Command::new(powershell.join("powershell.exe"));
    command.env_clear();
    for name in [
        "TEMP",
        "TMP",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "HOMEDRIVE",
        "HOMEPATH",
        "PROGRAMDATA",
        "COMSPEC",
        "PROCESSOR_ARCHITECTURE",
        "PROCESSOR_ARCHITEW6432",
    ] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    command
        .env("OS", "Windows_NT")
        .env("SystemRoot", system_root)
        .env("WINDIR", system_root);
    command.env(
        "PATH",
        std::env::join_paths([system32, system_root.to_owned(), powershell])
            .map_err(|_| "Windows system tools are unavailable")?,
    );
    let encoded = base64::engine::general_purpose::STANDARD.encode(
        script
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
    );
    command.args(["-NoProfile", "-EncodedCommand", &encoded]);
    Ok(command)
}

pub fn installation_decision(
    settings: &ProviderCliSettings,
    candidate: Option<&ExecutableCandidate>,
) -> InstallationDecision {
    if candidate.is_some() {
        return InstallationDecision::AlreadyInstalled;
    }
    if settings.mode == CliRuntimeMode::Wsl
        || settings
            .custom_path
            .as_ref()
            .is_some_and(|p| !p.trim().is_empty())
    {
        return InstallationDecision::ManualRequired;
    }
    InstallationDecision::Launch
}

pub fn powershell_script(provider: CliProvider) -> String {
    let url = match provider {
        CliProvider::Claude => "https://claude.ai/install.ps1",
        CliProvider::Codex => "https://chatgpt.com/codex/install.ps1",
    };
    format!(
        r#"$ErrorActionPreference = 'Stop'
$installerPath = [IO.Path]::Combine([IO.Path]::GetTempPath(), 'ai-meter-install-' + [Guid]::NewGuid().ToString('N') + '.ps1')
$result = 0
try {{
    Write-Host 'Downloading the official CLI installer...'
    Invoke-WebRequest -Uri '{url}' -OutFile $installerPath -UseBasicParsing
    $global:LASTEXITCODE = 0
    & ([ScriptBlock]::Create([IO.File]::ReadAllText($installerPath)))
    $result = $LASTEXITCODE
}} catch {{
    Write-Host 'The official installation did not complete. Check Status or retry.'
    $result = 1
}} finally {{
    if (Test-Path -LiteralPath $installerPath) {{ Remove-Item -LiteralPath $installerPath -Force }}
}}
exit $result
"#
    )
}
