use ai_token_meter_windows::accounts::service_status::{
    ServiceAccountConnectionState, deepseek_status, parse_claude_auth_status,
    parse_codex_account_status, runtime_source_label, sanitized_cli_version,
};
use ai_token_meter_windows::collectors::codex_app_server::collect_account_status_from_invocation;
use ai_token_meter_windows::domain::ProviderId;
use ai_token_meter_windows::platform::windows::executable_locator::RuntimeSource;
use std::time::Duration;

mod support;

#[test]
fn claude_status_preserves_the_account_identity_without_exposing_credentials() {
    let output = r#"notice\n{"loggedIn":true,"email":"member@example.com","authMethod":"claude.ai","subscriptionType":"max"}\n"#;
    let status =
        parse_claude_auth_status(output, "2026-09-03T00:00:00Z").expect("Claude account status");

    assert_eq!(status.provider_id, ProviderId::Claude);
    assert_eq!(
        status.connection_state,
        ServiceAccountConnectionState::Connected
    );
    assert_eq!(status.account_label.as_deref(), Some("member@example.com"));
    assert_eq!(status.account_detail.as_deref(), Some("Claude Code · Max"));
    assert_eq!(status.checked_at.as_deref(), Some("2026-09-03T00:00:00Z"));

    let signed_out = parse_claude_auth_status(
        r#"{"loggedIn":false,"authMethod":"none"}"#,
        "2026-09-03T00:00:00Z",
    )
    .expect("signed-out status");
    assert_eq!(
        signed_out.connection_state,
        ServiceAccountConnectionState::SignInRequired
    );
}

#[test]
fn codex_status_accepts_future_plan_names_and_distinguishes_non_openai_providers() {
    let signed_in = parse_codex_account_status(
        r#"{"id":2,"result":{"account":{"type":"chatgpt","email":"member@example.com","planType":"prolite"},"requiresOpenaiAuth":true}}"#,
        2,
        "2026-09-03T00:00:00Z",
    )
    .expect("Codex account status");
    assert_eq!(
        signed_in.connection_state,
        ServiceAccountConnectionState::Connected
    );
    assert_eq!(
        signed_in.account_label.as_deref(),
        Some("member@example.com")
    );
    assert_eq!(
        signed_in.account_detail.as_deref(),
        Some("ChatGPT · Prolite")
    );

    let signed_out = parse_codex_account_status(
        r#"{"id":2,"result":{"account":null,"requiresOpenaiAuth":true}}"#,
        2,
        "2026-09-03T00:00:00Z",
    )
    .expect("signed-out Codex account");
    assert_eq!(
        signed_out.connection_state,
        ServiceAccountConnectionState::SignInRequired
    );

    let external = parse_codex_account_status(
        r#"{"id":2,"result":{"account":null,"requiresOpenaiAuth":false}}"#,
        2,
        "2026-09-03T00:00:00Z",
    )
    .expect("external provider account");
    assert_eq!(
        external.connection_state,
        ServiceAccountConnectionState::Connected
    );
    assert_eq!(
        external.account_label.as_deref(),
        Some("Configured provider")
    );
}

#[test]
fn deepseek_and_runtime_labels_are_descriptive_but_never_reveal_paths_or_keys() {
    let status = deepseek_status(Some("private-key-ending-7xyz"), "2026-09-03T00:00:00Z");
    assert_eq!(
        status.connection_state,
        ServiceAccountConnectionState::Connected
    );
    assert_eq!(status.account_label.as_deref(), Some("API Key ••••7xyz"));
    assert_eq!(
        status.account_detail.as_deref(),
        Some("Windows Credential Manager")
    );
    assert!(!format!("{status:?}").contains("private-key"));

    assert_eq!(
        runtime_source_label(&RuntimeSource::NativeWindows),
        "Native Windows"
    );
    assert_eq!(
        runtime_source_label(&RuntimeSource::Wsl {
            distribution: "Ubuntu".to_owned(),
        }),
        "WSL · Ubuntu"
    );
    assert_eq!(
        sanitized_cli_version("codex-cli 0.148.0\r\nprivate/path"),
        Some("0.148.0".to_owned())
    );
    assert_eq!(sanitized_cli_version("not a version"), None);
}

#[test]
fn codex_account_status_uses_the_bounded_official_app_server_session() {
    let invocation = support::native_fixture_invocation(Vec::new());

    let status = collect_account_status_from_invocation(
        &invocation,
        None,
        "2026-09-03T00:00:00Z",
        Duration::from_secs(3),
    )
    .expect("bounded Codex account conversation");
    assert_eq!(
        status.connection_state,
        ServiceAccountConnectionState::Connected
    );
    assert_eq!(status.account_label.as_deref(), Some("private@example.com"));
    assert_eq!(status.account_detail.as_deref(), Some("ChatGPT · Pro"));
}
