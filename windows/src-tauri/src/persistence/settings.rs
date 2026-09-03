use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MeterEdge {
    Left,
    Right,
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
    pub detail_auto_hide_seconds: u64,
    pub display_font: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            edge: MeterEdge::Right,
            meter_vertical_per_mille: default_meter_vertical_per_mille(),
            meter_monitor_id: None,
            refresh_interval_seconds: 300,
            detail_auto_hide_seconds: 8,
            display_font: "Antonio".to_owned(),
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
