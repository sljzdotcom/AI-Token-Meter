use ai_token_meter_windows::collectors::CollectionError;
use ai_token_meter_windows::collectors::gemini::{
    parse_current_gemini_model, parse_gemini_catalog, parse_usage,
};
use ai_token_meter_windows::domain::{MetricKind, MetricUnit};

const USAGE: &str = include_str!("../../../contracts/antigravity-cli/1.1.28/usage.txt");
const DATE: &str = "2026-09-10T12:00:00Z";

#[test]
fn validates_all_official_windows_but_only_publishes_gemini_quota() {
    let snapshot = parse_usage(USAGE, DATE, "1.1.28").unwrap();
    assert_eq!(snapshot.display_name, "Google Antigravity");
    assert_eq!(snapshot.used_ratio.unwrap().get(), 0.6);
    assert_eq!(
        snapshot.primary_metric.as_ref().unwrap().label,
        "Gemini · Five hour"
    );
    assert_eq!(
        snapshot.secondary_metric.as_ref().unwrap().label,
        "Gemini · Weekly"
    );
    assert_eq!(
        snapshot
            .gemini_quota_metrics
            .iter()
            .map(|metric| metric.label.as_str())
            .collect::<Vec<_>>(),
        vec!["Gemini · Five hour", "Gemini · Weekly"]
    );
    assert_eq!(
        snapshot
            .gemini_quota_metrics
            .iter()
            .map(|metric| metric.current)
            .collect::<Vec<_>>(),
        vec![60.0, 25.0]
    );
    assert!(snapshot.gemini_quota_metrics.iter().all(|metric| {
        metric.limit == Some(100.0)
            && metric.unit == MetricUnit::Percent
            && metric.kind == MetricKind::OfficialLimit
            && metric.reset_at.is_some()
    }));
}

#[test]
fn parses_only_current_gemini_model() {
    assert_eq!(
        parse_current_gemini_model("gemini-3.8-flash-high\tGemini 3.8 Flash (High)\n"),
        Some("Gemini 3.8 Flash (High)".to_owned())
    );
    assert_eq!(
        parse_current_gemini_model("claude-sonnet-4-6\tClaude Sonnet 4.6 (Thinking)"),
        None
    );
    assert_eq!(
        parse_current_gemini_model(
            "gemini-3.8-flash-high\tGemini 3.8 Flash (High)\ngemini-3.7-flash-low\tGemini 3.7 Flash (Low)"
        ),
        None
    );
}

#[test]
fn filters_third_party_models_and_groups_gemini_families() {
    let catalog = parse_gemini_catalog(
        "Fetching available models...\n\
         gemini-3.8-flash-high\tGemini 3.8 Flash (High)\n\
         gemini-3.8-flash-low\tGemini 3.8 Flash (Low)\n\
         gemini-3.7-flash-high\tGemini 3.7 Flash (High)\n\
         claude-sonnet-4-6\tClaude Sonnet 4.6 (Thinking)\n",
    )
    .unwrap();
    assert_eq!(catalog.model_count, 3);
    assert_eq!(
        catalog.families,
        vec!["Gemini 3.8 Flash", "Gemini 3.7 Flash"]
    );
    assert!(parse_gemini_catalog("Fetching available models...").is_none());
    assert!(parse_gemini_catalog("gemini-3.8-flash-high").is_none());
}

#[test]
fn row_order_does_not_change_presentation_order() {
    let reversed = USAGE.lines().rev().collect::<Vec<_>>().join("\n");
    let snapshot = parse_usage(&reversed, DATE, "1.1.28").unwrap();
    assert_eq!(
        snapshot
            .gemini_quota_metrics
            .iter()
            .map(|metric| metric.current)
            .collect::<Vec<_>>(),
        vec![60.0, 25.0]
    );
}

#[test]
fn accepts_accounts_that_only_expose_the_two_gemini_windows() {
    let gemini_only = USAGE
        .lines()
        .filter(|line| line.starts_with("Gemini Models\t"))
        .collect::<Vec<_>>()
        .join("\n");
    let snapshot = parse_usage(&gemini_only, DATE, "1.1.28").unwrap();
    assert_eq!(snapshot.used_ratio.unwrap().get(), 0.6);
    assert_eq!(snapshot.gemini_quota_metrics.len(), 2);
}

#[test]
fn incomplete_ambiguous_and_legacy_output_is_rejected() {
    let cases = [
        String::new(),
        USAGE.lines().next().unwrap().into(),
        USAGE.replacen("75%", "101%", 1),
        USAGE.replacen("75%", "-1%", 1),
        USAGE.replacen("75%", "seventy%", 1),
        USAGE.replacen("2026-09-17T10:00:00Z", "tomorrow", 1),
        USAGE.replacen("Gemini Models", "Unknown models", 1),
        USAGE.replacen("Weekly Limit Remaining", "Daily Limit Remaining", 1),
        format!("{USAGE}\n{}", USAGE.lines().next().unwrap()),
        USAGE.lines().take(3).collect::<Vec<_>>().join("\n"),
        "Select Model\nModel usage\nPro 25%".into(),
    ];
    for value in cases {
        assert_eq!(
            parse_usage(&value, DATE, "1.1.28"),
            Err(CollectionError::UnrecognizedOutput)
        );
    }
}
