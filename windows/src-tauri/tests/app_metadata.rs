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
            "Code0".to_owned(),
        ]
    );
}

#[test]
fn windows_service_set_never_accepts_code0() {
    assert_eq!(
        [
            ProviderId::Claude,
            ProviderId::Codex,
            ProviderId::DeepSeek,
            ProviderId::Gemini,
        ]
        .map(ProviderId::as_str),
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
