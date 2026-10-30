use ai_token_meter_windows::app_metadata;
use ai_token_meter_windows::domain::{ProviderId, UsageSnapshot};
use serde_json::json;

#[test]
fn product_metadata_matches_the_shared_contract() {
    let metadata = app_metadata().expect("shared metadata should decode");

    assert_eq!(metadata.product_name, "AI Token Meter");
    assert_eq!(metadata.version, "0.11.0");
    assert_eq!(
        metadata.providers.as_slice(),
        &[
            "Claude Code".to_owned(),
            "OpenAI Codex".to_owned(),
            "DeepSeek".to_owned(),
            "Google Antigravity".to_owned(),
        ]
    );
}

/// Exhaustive match without a wildcard arm: adding a fifth provider variant to
/// `ProviderId` breaks this function's compilation instead of leaving the
/// Windows service set silently wider than the assertion below.
fn windows_service_id(provider: ProviderId) -> &'static str {
    match provider {
        ProviderId::Claude => "claude",
        ProviderId::Codex => "codex",
        ProviderId::DeepSeek => "deepseek",
        ProviderId::Gemini => "gemini",
    }
}

#[test]
fn windows_service_set_never_accepts_code0() {
    assert_eq!(
        [
            windows_service_id(ProviderId::Claude),
            windows_service_id(ProviderId::Codex),
            windows_service_id(ProviderId::DeepSeek),
            windows_service_id(ProviderId::Gemini),
        ],
        ["claude", "codex", "deepseek", "gemini"]
    );

    let value = json!({
        "schemaVersion": 1,
        "providerId": "code0",
        "displayName": "Code0",
        "status": "fresh",
        "fetchedAt": "2026-10-30T00:00:00Z",
        "staleAfterSeconds": 300,
        "usedRatio": 0.1
    });
    let error = UsageSnapshot::decode_compatible(&value)
        .expect_err("the Windows service set has no Code0 provider");
    assert!(
        error.to_string().contains("code0"),
        "unexpected error: {error}"
    );
}
