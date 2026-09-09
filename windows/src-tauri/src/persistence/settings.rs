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
    #[serde(default, deserialize_with = "deserialize_locale")]
    pub locale: Locale,
    #[serde(default, deserialize_with = "deserialize_displays")]
    pub displays: Option<crate::platform::windows::monitor::DisplayPreferences>,
    #[serde(
        default = "legacy_strip_preferences",
        deserialize_with = "deserialize_strip_preferences"
    )]
    pub strip_preferences: crate::platform::windows::strip_preferences::StripPreferences,
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
    #[serde(default = "default_display_font")]
    pub display_font: String,
    #[serde(default)]
    pub claude_cli: ProviderCliSettings,
    #[serde(default)]
    pub codex_cli: ProviderCliSettings,
}

fn legacy_strip_preferences() -> crate::platform::windows::strip_preferences::StripPreferences {
    let mut value = crate::platform::windows::strip_preferences::StripPreferences::default();
    value.hidden_providers.push("gemini".into());
    value
}

fn deserialize_strip_preferences<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<crate::platform::windows::strip_preferences::StripPreferences, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    let mut preferences: crate::platform::windows::strip_preferences::StripPreferences =
        serde_json::from_value(value).unwrap_or_else(|_| legacy_strip_preferences());
    preferences.normalize();
    Ok(preferences)
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            locale: Locale::default(),
            displays: Some(Default::default()),
            strip_preferences: Default::default(),
            edge: MeterEdge::Right,
            meter_vertical_per_mille: default_meter_vertical_per_mille(),
            meter_monitor_id: None,
            refresh_interval_seconds: 300,
            deepseek_balance_baseline_cents: default_deepseek_balance_baseline_cents(),
            notifications_enabled: false,
            launch_at_login: false,
            detail_auto_hide_seconds: 8,
            display_font: default_display_font(),
            claude_cli: ProviderCliSettings::default(),
            codex_cli: ProviderCliSettings::default(),
        }
    }
}

impl AppSettings {
    pub fn normalize_display_preferences(&mut self) {
        use crate::platform::windows::monitor::{
            DisplayMode, DisplayPlacement, DisplayPreferences,
        };
        if self.displays.as_ref().is_some_and(|d| d.version == 1) {
            return;
        }
        let mut displays = DisplayPreferences::default();
        if let Some(id) = &self.meter_monitor_id {
            displays.mode = DisplayMode::Selected;
            displays.selected_id = Some(id.clone());
            displays.placements.insert(
                id.clone(),
                DisplayPlacement {
                    edge: self.edge,
                    vertical_per_mille: self.meter_vertical_per_mille.min(1000),
                },
            );
        }
        self.displays = Some(displays);
    }
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
            crate::domain::ProviderId::DeepSeek | crate::domain::ProviderId::Gemini => None,
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
            crate::domain::ProviderId::Gemini => {
                return Err("Gemini CLI integration is currently unavailable");
            }
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
        if text.trim().is_empty() || text.len() > 4096 || text.contains(['\0', '\n', '\r']) {
            return Err("invalid CLI runtime setting");
        }
    }
    Ok(())
}

pub const SUPPORTED_DISPLAY_FONTS: [&str; 11] = [
    "Microsoft YaHei",
    "SimHei",
    "KaiTi",
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

fn default_display_font() -> String {
    "Microsoft YaHei".to_owned()
}

fn deserialize_locale<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Locale, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or_default())
}

fn deserialize_displays<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<crate::platform::windows::monitor::DisplayPreferences>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).ok())
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub enum Locale {
    #[serde(rename = "zh-CN")]
    SimplifiedChinese,
    #[default]
    #[serde(rename = "en", other)]
    English,
}
