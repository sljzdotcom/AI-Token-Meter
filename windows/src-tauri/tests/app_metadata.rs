use ai_token_meter_windows::app_metadata;

#[test]
fn product_metadata_matches_the_shared_contract() {
    let metadata = app_metadata().expect("shared metadata should decode");

    assert_eq!(metadata.product_name, "AI Token Meter");
    assert_eq!(metadata.version, "0.8.0");
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
