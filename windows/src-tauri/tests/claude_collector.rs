use std::path::Path;

use ai_token_meter_windows::collectors::CollectionError;
use ai_token_meter_windows::collectors::claude::parse_usage_output;
use ai_token_meter_windows::domain::{MetricKind, MetricUnit, ProviderId, UsageStatus};

const FETCHED_AT: &str = "2026-09-03T00:00:00Z";

#[test]
fn parses_official_windows_usage_without_treating_the_promotion_as_a_limit() {
    let snapshot = parse_usage_output(&fixture(), FETCHED_AT).expect("Claude usage snapshot");

    assert_eq!(snapshot.provider_id, ProviderId::Claude);
    assert_eq!(snapshot.status, UsageStatus::Fresh);
    assert_eq!(snapshot.used_ratio.expect("used ratio").get(), 0.23);
    let primary = snapshot.primary_metric.expect("session metric");
    assert_eq!(primary.label, "Current session");
    assert_eq!(primary.current, 23.0);
    assert_eq!(primary.unit, MetricUnit::Percent);
    assert_eq!(primary.kind, MetricKind::OfficialLimit);
    assert_eq!(
        primary.reset_description.as_deref(),
        Some("Resets in 3 hr 42 min")
    );
    let secondary = snapshot.secondary_metric.expect("weekly metric");
    assert_eq!(secondary.label, "Current week (all models)");
    assert_eq!(secondary.current, 5.0);
    assert_eq!(
        secondary.reset_description.as_deref(),
        Some("Resets Sep 6 at 8:00am")
    );
}

#[test]
fn authentication_and_workspace_prompts_never_become_zero_usage() {
    assert_eq!(
        parse_usage_output("Not logged in. Please log in", FETCHED_AT),
        Err(CollectionError::AuthenticationRequired)
    );
    assert_eq!(
        parse_usage_output("Permission Required: Accessing workspace", FETCHED_AT),
        Err(CollectionError::SetupRequired)
    );
    assert_eq!(
        parse_usage_output("Claude Code is ready", FETCHED_AT),
        Err(CollectionError::UnrecognizedOutput)
    );
}

#[test]
fn remaining_percentages_are_converted_to_used_percentages() {
    let snapshot = parse_usage_output("All models\n20% remaining\nResets tomorrow", FETCHED_AT)
        .expect("remaining usage");

    assert_eq!(snapshot.used_ratio.expect("used ratio").get(), 0.8);
    assert_eq!(snapshot.primary_metric.expect("metric").current, 80.0);
}

fn fixture() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("contracts")
        .join("fixtures")
        .join("claude-usage-windows.txt");
    std::fs::read_to_string(path).expect("Claude Windows fixture")
}
