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

#[test]
fn chinese_native_summary_and_notifications_preserve_provider_names() {
    use ai_token_meter_windows::persistence::Locale;
    use ai_token_meter_windows::platform::windows::tray::format_summary_localized;
    let claude = fixture("claude-fresh.json");
    assert_eq!(
        format_summary_localized(&claude, Locale::SimplifiedChinese),
        "Claude Code · 已用 23%"
    );
    let (title, body) = ai_token_meter_windows::localization::threshold_notice(
        Locale::SimplifiedChinese,
        "Claude Code",
        "Session",
        70,
    );
    assert_eq!(title, "Claude Code 用量已达 70%");
    assert!(body.contains("会话"));
    assert!(body.contains("70%"));
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
