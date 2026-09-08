use ai_token_meter_windows::collectors::CollectionError;
use ai_token_meter_windows::collectors::gemini::{
    TerminalState, parse_terminal_quota, terminal_state,
};
use ai_token_meter_windows::domain::{MetricKind, MetricUnit};
const OPEN: &str =
    include_str!("../../../contracts/gemini-cli/0.58.0/authenticated-model-open.ansi.txt");
const READY: &str = include_str!("../../../contracts/gemini-cli/0.58.0/ready.ansi.txt");
const DATE: &str = "2026-09-08T09:00:00Z";

#[test]
fn real_terminal_redraws_yield_visible_used_tiers_not_footer() {
    let snapshot = parse_terminal_quota(OPEN, DATE).unwrap();
    let value = serde_json::to_value(&snapshot).unwrap();
    assert_eq!(snapshot.used_ratio.unwrap().get(), 0.6);
    assert_eq!(snapshot.primary_metric.as_ref().unwrap().label, "Flash");
    assert_eq!(value["geminiQuotaMetrics"].as_array().unwrap().len(), 2);
    assert_eq!(value["geminiQuotaMetrics"][0]["current"], 25.0);
    assert_eq!(value["geminiQuotaMetrics"][1]["current"], 60.0);
    let metric = snapshot.primary_metric.unwrap();
    assert_eq!(metric.unit, MetricUnit::Percent);
    assert_eq!(metric.kind, MetricKind::OfficialLimit);
    assert_eq!(metric.limit, Some(100.0));
    assert_eq!(metric.reset_at, None);
    assert_eq!(
        metric.reset_description.as_deref(),
        Some("Resets: 5:47 PM (1h)")
    );
}

#[test]
fn ready_requires_real_input_and_authentication_never_becomes_ready() {
    assert_eq!(terminal_state(READY).unwrap(), TerminalState::Ready);
    for text in [
        include_str!("../../../contracts/gemini-cli/0.58.0/missing-auth.ansi.txt"),
        include_str!("../../../contracts/gemini-cli/0.58.0/invalid-auth.ansi.txt"),
    ] {
        assert_eq!(
            terminal_state(text),
            Err(CollectionError::AuthenticationRequired)
        );
    }
    assert_ne!(terminal_state("\x1b]0;Ready\x07"), Ok(TerminalState::Ready));
    assert!(terminal_state("Select a theme\n> Type your message or @path/to/file").is_err());
}

#[test]
fn latest_screen_cleared_or_failed_never_resurrects_old_quota() {
    for text in [
        include_str!("../../../contracts/gemini-cli/0.58.0/authenticated.ansi.txt"),
        include_str!("../../../contracts/gemini-cli/0.58.0/quota-failure-model-open.ansi.txt"),
        "",
        "43% used",
    ] {
        assert!(parse_terminal_quota(text, DATE).is_err());
    }
    assert!(
        parse_terminal_quota(
            &format!("{OPEN}\x1b[2J\x1b[HSelect Model\nPress Esc to close\n╰──╯"),
            DATE
        )
        .is_err()
    );
}

fn dialog(rows: &str) -> String {
    format!("╭──╮\n│ Select Model\n│ Model usage\n{rows}\n│ (Press Esc to close)\n╰──╯\n")
}
#[test]
fn all_three_tiers_and_ties_keep_stable_order_without_invented_reset() {
    let snapshot = parse_terminal_quota(
        &dialog("│ Flash Lite ▬ 25%\n│ Flash ▬ 60%\n│ Pro ▬ 60%"),
        DATE,
    )
    .unwrap();
    assert_eq!(snapshot.primary_metric.unwrap().label, "Pro");
    let value = serde_json::to_value(snapshot.secondary_metric).unwrap();
    assert_eq!(value["label"], "Flash");
}
#[test]
fn malformed_missing_conflicting_or_unknown_tiers_are_rejected() {
    for rows in [
        "│ Pro ▬ 101%",
        "│ Pro ▬ -1%",
        "│ Pro ▬ 2.5%",
        "│ Pro ▬ 25%\n│ Pro ▬ 70%",
        "│ Unknown ▬ 50%",
        "│ Pro ▬ 25%\n│ Flash ▬",
        "│ Pro ▬ 25% remaining",
    ] {
        assert!(parse_terminal_quota(&dialog(rows), DATE).is_err(), "{rows}");
    }
    assert!(parse_terminal_quota("Select Model\nModel usage\nPro ▬ 25%", DATE).is_err());
}

#[test]
fn real_untrusted_ready_and_quota_are_supported_without_trust_approval() {
    assert_eq!(
        terminal_state(include_str!(
            "../../../contracts/gemini-cli/0.58.0/untrusted-ready.ansi.txt"
        ))
        .unwrap(),
        TerminalState::Ready
    );
    let snapshot = parse_terminal_quota(
        include_str!(
            "../../../contracts/gemini-cli/0.58.0/authenticated-untrusted-model-open.ansi.txt"
        ),
        DATE,
    )
    .unwrap();
    assert_eq!(snapshot.used_ratio.unwrap().get(), 0.6);
    assert!(
        parse_terminal_quota(
            include_str!("../../../contracts/gemini-cli/0.58.0/authenticated-untrusted.ansi.txt"),
            DATE
        )
        .is_err()
    );
}

#[test]
fn real_model_frame_derives_the_shared_fresh_contract_and_bad_tiers_fail_decode() {
    use ai_token_meter_windows::domain::UsageSnapshot;
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../contracts/fixtures/gemini-fresh.json"
    ))
    .unwrap();
    let snapshot = UsageSnapshot::decode_compatible(&value).unwrap();
    assert_eq!(
        parse_terminal_quota(OPEN, &snapshot.fetched_at).unwrap(),
        snapshot
    );
    let mut invalid = value;
    invalid["geminiQuotaMetrics"][0]["current"] = serde_json::json!(110);
    assert!(UsageSnapshot::decode_compatible(&invalid).is_err());
}
