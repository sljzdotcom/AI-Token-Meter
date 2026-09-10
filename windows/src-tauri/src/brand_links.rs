use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BrandLink {
    Twitter,
    Github,
    Telegram,
}

impl BrandLink {
    pub fn url(self) -> &'static str {
        match self {
            Self::Twitter => "https://twitter.com/MillerPanYue",
            Self::Github => "https://github.com/sljzdotcom/AI-Token-Meter",
            Self::Telegram => "https://t.me/sljzdotcom",
        }
    }
}

#[tauri::command]
pub fn open_brand_link(target: BrandLink) -> Result<(), &'static str> {
    open_fixed_url(target.url())
}

#[cfg(windows)]
fn open_fixed_url(url: &'static str) -> Result<(), &'static str> {
    use crate::platform::windows::process::configure_restricted_command;
    use std::process::Command;

    let root = std::env::var_os("SystemRoot").ok_or("The author link could not be opened")?;
    let executable = std::path::PathBuf::from(root)
        .join("System32")
        .join("rundll32.exe");
    let mut command = Command::new(&executable);
    configure_restricted_command(&mut command, &executable);
    command
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()
        .map(|_| ())
        .map_err(|_| "The author link could not be opened")
}

#[cfg(not(windows))]
fn open_fixed_url(_url: &'static str) -> Result<(), &'static str> {
    Err("The author link could not be opened")
}

#[tauri::command]
pub fn open_gemini_installation_guide() -> Result<(), &'static str> {
    open_fixed_url(gemini_installation_guide_url())
        .map_err(|_| "The installation guide could not be opened")
}

pub fn gemini_installation_guide_url() -> &'static str {
    "https://antigravity.google/docs/cli/install/"
}
