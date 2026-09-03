use std::path::{Path, PathBuf};
use std::time::Duration;

use ai_token_meter_windows::collectors::CollectionError;
use ai_token_meter_windows::collectors::codex_app_server::{
    collect_rate_limits_from_invocation, parse_account_response, parse_rate_limits_response,
};
use ai_token_meter_windows::domain::{ProviderId, ResetCreditKind, UsageStatus};
use ai_token_meter_windows::platform::windows::process::CommandInvocation;

const FETCHED_AT: &str = "2026-09-03T00:00:00Z";

#[test]
fn parses_rate_limits_and_reset_credits_while_ignoring_notifications() {
    let snapshot =
        parse_rate_limits_response(&fixture(), 2, FETCHED_AT).expect("Codex rate-limit snapshot");

    assert_eq!(snapshot.provider_id, ProviderId::Codex);
    assert_eq!(snapshot.status, UsageStatus::Fresh);
    assert_eq!(snapshot.used_ratio.expect("used ratio").get(), 0.23);
    let primary = snapshot.primary_metric.expect("primary limit");
    assert_eq!(primary.label, "5h limit");
    assert_eq!(primary.current, 23.0);
    assert_eq!(primary.reset_at.as_deref(), Some("2026-09-03T08:00:00Z"));
    let secondary = snapshot.secondary_metric.expect("weekly limit");
    assert_eq!(secondary.label, "Weekly limit");
    assert_eq!(secondary.current, 5.0);
    assert_eq!(snapshot.reset_credits.len(), 1);
    assert_eq!(
        snapshot.reset_credits[0].kind,
        ResetCreditKind::FullUsageReset
    );
    assert_eq!(snapshot.reset_credits[0].count, 1);
    assert_eq!(snapshot.reset_credits[0].expires_at, "2026-09-21T00:25:00Z");
}

#[test]
fn account_response_distinguishes_sign_in_from_valid_accounts() {
    let signed_out = r#"{"id":3,"result":{"account":null,"requiresOpenaiAuth":true}}"#;
    assert_eq!(
        parse_account_response(signed_out, 3),
        Err(CollectionError::AuthenticationRequired)
    );

    let signed_in = r#"{"id":4,"result":{"account":{"type":"chatgpt","email":"private@example.com","planType":"pro"},"requiresOpenaiAuth":true}}"#;
    assert!(parse_account_response(signed_in, 4).is_ok());
}

#[test]
fn missing_or_malformed_expected_response_is_not_reported_as_zero() {
    assert_eq!(
        parse_rate_limits_response(r#"{"method":"unrelated"}"#, 2, FETCHED_AT),
        Err(CollectionError::InvalidResponse)
    );
    assert_eq!(
        parse_rate_limits_response(r#"{"id":2,"result":{"rateLimits":{}}}"#, 2, FETCHED_AT),
        Err(CollectionError::UnrecognizedOutput)
    );
}

#[test]
fn performs_the_bounded_app_server_handshake_without_a_shell() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("codex-app-server-fixture.js");
    let invocation = CommandInvocation {
        executable: find_node(),
        arguments: vec![fixture.into_os_string()],
    };

    let snapshot =
        collect_rate_limits_from_invocation(&invocation, None, FETCHED_AT, Duration::from_secs(3))
            .expect("bounded Codex app-server conversation");

    assert_eq!(snapshot.used_ratio.expect("used ratio").get(), 0.17);
    assert_eq!(snapshot.secondary_metric.expect("weekly").current, 4.0);
}

fn fixture() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("contracts")
        .join("fixtures")
        .join("codex-app-server-windows.jsonl");
    std::fs::read_to_string(path).expect("Codex app-server fixture")
}

fn find_node() -> PathBuf {
    let name = if cfg!(windows) { "node.exe" } else { "node" };
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|directory| directory.join(name))
        .find_map(|path| path.canonicalize().ok().filter(|path| path.is_file()))
        .expect("Node.js is required by the frontend toolchain")
}
