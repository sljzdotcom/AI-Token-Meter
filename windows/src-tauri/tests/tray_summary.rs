use std::path::PathBuf;

use ai_token_meter_windows::domain::{UsageSnapshot, UsageStatus};
use ai_token_meter_windows::platform::windows::tray::format_summary;

#[test]
fn tray_summary_uses_provider_semantics_without_inventing_usage() {
    let claude = fixture("claude-fresh.json");
    let deepseek = fixture("deepseek-balance.json");
    let mut unavailable = claude.clone();
    unavailable.status = UsageStatus::Unavailable;
    unavailable.used_ratio = None;
    unavailable.primary_metric = None;

    assert_eq!(format_summary(&claude), "Claude Code · 23% used");
    assert_eq!(format_summary(&deepseek), "DeepSeek · ¥77.99 available");
    assert_eq!(format_summary(&unavailable), "Claude Code · Unavailable");
}

fn fixture(name: &str) -> UsageSnapshot {
    let value = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../contracts/fixtures")
            .join(name),
    )
    .expect("fixture");
    let json = serde_json::from_str(&value).expect("fixture json");
    UsageSnapshot::decode_compatible(&json).expect("snapshot")
}
