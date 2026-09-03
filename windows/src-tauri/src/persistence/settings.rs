use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MeterEdge {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CliRuntimeMode {
    #[default]
    Auto,
    NativeWindows,
    Wsl,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCliSettings {
    pub mode: CliRuntimeMode,
    pub custom_path: Option<String>,
    pub wsl_distribution: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub edge: MeterEdge,
    #[serde(default = "default_meter_vertical_per_mille")]
    pub meter_vertical_per_mille: u16,
    #[serde(default)]
    pub meter_monitor_id: Option<String>,
    pub refresh_interval_seconds: u64,
    #[serde(default = "default_deepseek_balance_baseline_cents")]
    pub deepseek_balance_baseline_cents: u64,
    #[serde(default)]
    pub notifications_enabled: bool,
    #[serde(default)]
    pub launch_at_login: bool,
    pub detail_auto_hide_seconds: u64,
    pub display_font: String,
    #[serde(default)]
    pub claude_cli: ProviderCliSettings,
    #[serde(default)]
    pub codex_cli: ProviderCliSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            edge: MeterEdge::Right,
            meter_vertical_per_mille: default_meter_vertical_per_mille(),
            meter_monitor_id: None,
            refresh_interval_seconds: 300,
            deepseek_balance_baseline_cents: default_deepseek_balance_baseline_cents(),
            notifications_enabled: false,
            launch_at_login: false,
            detail_auto_hide_seconds: 8,
            display_font: "Antonio".to_owned(),
            claude_cli: ProviderCliSettings::default(),
            codex_cli: ProviderCliSettings::default(),
        }
    }
}

impl AppSettings {
    pub fn set_display_font(&mut self, font: &str) -> Result<(), &'static str> {
        if !SUPPORTED_DISPLAY_FONTS.contains(&font) {
            return Err("unsupported display font");
        }
        self.display_font = font.to_owned();
        Ok(())
    }

    pub fn set_detail_auto_hide_seconds(&mut self, seconds: u64) -> Result<(), &'static str> {
        if !(1..=300).contains(&seconds) {
            return Err("detail auto-hide must be between 1 and 300 seconds");
        }
        self.detail_auto_hide_seconds = seconds;
        Ok(())
    }

    pub fn set_refresh_interval_seconds(&mut self, seconds: u64) -> Result<(), &'static str> {
        if !(30..=86_400).contains(&seconds) {
            return Err("refresh interval must be between 30 seconds and 24 hours");
        }
        self.refresh_interval_seconds = seconds;
        Ok(())
    }

    pub fn set_deepseek_balance_baseline_cents(&mut self, cents: u64) -> Result<(), &'static str> {
        if !(100..=100_000_000).contains(&cents) {
            return Err("DeepSeek balance baseline must be between ¥1 and ¥1,000,000");
        }
        self.deepseek_balance_baseline_cents = cents;
        Ok(())
    }

    pub fn cli_settings(
        &self,
        provider: crate::domain::ProviderId,
    ) -> Option<&ProviderCliSettings> {
        match provider {
            crate::domain::ProviderId::Claude => Some(&self.claude_cli),
            crate::domain::ProviderId::Codex => Some(&self.codex_cli),
            crate::domain::ProviderId::DeepSeek => None,
        }
    }

    pub fn set_cli_settings(
        &mut self,
        provider: crate::domain::ProviderId,
        value: ProviderCliSettings,
    ) -> Result<(), &'static str> {
        validate_cli_settings(&value)?;
        match provider {
            crate::domain::ProviderId::Claude => self.claude_cli = value,
            crate::domain::ProviderId::Codex => self.codex_cli = value,
            crate::domain::ProviderId::DeepSeek => return Err("DeepSeek does not use a CLI"),
        }
        Ok(())
    }
}

const fn default_deepseek_balance_baseline_cents() -> u64 {
    10_000
}

fn validate_cli_settings(value: &ProviderCliSettings) -> Result<(), &'static str> {
    for text in [&value.custom_path, &value.wsl_distribution]
        .into_iter()
        .flatten()
    {
        if text.is_empty() || text.len() > 4096 || text.contains(['\0', '\n', '\r']) {
            return Err("invalid CLI runtime setting");
        }
    }
    Ok(())
}

pub const SUPPORTED_DISPLAY_FONTS: [&str; 8] = [
    "System Default",
    "Antonio",
    "DIN Condensed",
    "Alimama FangYuanTi VF",
    "Fira Code",
    "Leigo",
    "Menlo",
    "Alimama DaoLiTi",
];

const fn default_meter_vertical_per_mille() -> u16 {
    500
}
