use super::cli_account::CliProvider;
use crate::persistence::{CliRuntimeMode, ProviderCliSettings};
use crate::platform::windows::executable_locator::ExecutableCandidate;

#[derive(Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallationDecision {
    Launch,
    AlreadyInstalled,
    ManualRequired,
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
