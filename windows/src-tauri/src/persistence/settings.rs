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
    pub refresh_interval_seconds: u64,
    pub detail_auto_hide_seconds: u64,
    pub display_font: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            edge: MeterEdge::Right,
            refresh_interval_seconds: 300,
            detail_auto_hide_seconds: 8,
            display_font: "System".to_owned(),
        }
    }
}
