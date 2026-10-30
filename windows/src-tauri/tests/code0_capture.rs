use ai_token_meter_windows::collectors::code0_capture::{Code0CaptureError, Code0WebsiteCapture};
use ai_token_meter_windows::domain::{MetricKind, MetricUnit, ProviderId, UsageSnapshot};
use serde_json::{Value, json};

#[test]
fn filtered_quota_converts_to_site_credits_without_inventing_limit() {
    let capture = Code0WebsiteCapture::decode_filtered_json(
        br#"{"quota":1250000,"usedQuota":750000,"divisor":500000}"#,
    )
    .expect("valid filtered numeric quota must be accepted");
    assert_eq!(capture.balance, 2.5);
    assert_eq!(capture.cumulative_usage, 1.5);
    let snapshot = capture
        .make_snapshot("2026-10-26T12:00:00Z")
        .expect("site-credit snapshot");
    assert_eq!(snapshot.provider_id.as_str(), "code0");
    assert_eq!(snapshot.display_name, "Code0");
    assert_eq!(snapshot.fetched_at, "2026-10-26T12:00:00Z");
    assert!(snapshot.used_ratio.is_none());
    assert!(snapshot.daily_history.is_empty());
    assert!(snapshot.local_activity.is_none());
    let balance = snapshot.primary_metric.expect("balance");
    let used = snapshot.secondary_metric.expect("cumulative usage");
    assert_eq!(balance.label, "Balance");
    assert_eq!(balance.current, 2.5);
    assert_eq!(serde_json::to_value(balance.unit).unwrap(), "siteCredits");
    assert_eq!(serde_json::to_value(balance.kind).unwrap(), "balance");
    assert_eq!(used.label, "Cumulative usage");
    assert_eq!(used.current, 1.5);
    assert_eq!(serde_json::to_value(used.unit).unwrap(), "siteCredits");
    assert_eq!(serde_json::to_value(used.kind).unwrap(), "cumulativeUsage");
    for metric in [balance, used] {
        assert!(metric.limit.is_none());
        assert!(metric.reset_at.is_none());
        assert!(metric.reset_description.is_none());
    }
}

#[test]
fn zero_fractional_divisor_and_safe_integer_upper_bound_are_valid() {
    let zero =
        Code0WebsiteCapture::decode_filtered_json(br#"{"quota":0,"usedQuota":0,"divisor":500000}"#)
            .expect("zero is valid data");
    assert_eq!(zero.balance, 0.0);
    assert_eq!(zero.cumulative_usage, 0.0);
    let fractional =
        Code0WebsiteCapture::decode_filtered_json(br#"{"quota":1,"usedQuota":2,"divisor":0.5}"#)
            .expect("positive finite fractional divisor");
    assert_eq!(fractional.balance, 2.0);
    assert_eq!(fractional.cumulative_usage, 4.0);
    let upper_bound = Code0WebsiteCapture::decode_filtered_json(
        br#"{"quota":9007199254740991,"usedQuota":9007199254740991,"divisor":9007199254740991}"#,
    )
    .expect("inclusive safe integer bound");
    assert_eq!(upper_bound.balance, 1.0);
    assert_eq!(upper_bound.cumulative_usage, 1.0);
    let scientific = Code0WebsiteCapture::decode_filtered_json(
        br#"{"quota":1.25e6,"usedQuota":750000.0,"divisor":5e5}"#,
    )
    .expect("integer-valued scientific and decimal JSON numbers");
    assert_eq!(scientific.balance, 2.5);
    assert_eq!(scientific.cumulative_usage, 1.5);
}

#[test]
fn filtered_payload_rejects_missing_coerced_unsafe_and_extra_fields() {
    for payload in [
        r#"{"quota":1,"usedQuota":2}"#,
        r#"{"usedQuota":2,"divisor":1}"#,
        r#"{"quota":1,"divisor":1}"#,
        r#"{"quota":"1","usedQuota":2,"divisor":1}"#,
        r#"{"quota":1,"usedQuota":"2","divisor":1}"#,
        r#"{"quota":1,"usedQuota":2,"divisor":"1"}"#,
        r#"{"quota":true,"usedQuota":2,"divisor":1}"#,
        r#"{"quota":1,"usedQuota":false,"divisor":1}"#,
        r#"{"quota":1,"usedQuota":2,"divisor":true}"#,
        r#"{"quota":null,"usedQuota":2,"divisor":1}"#,
        r#"{"quota":-1,"usedQuota":2,"divisor":1}"#,
        r#"{"quota":1,"usedQuota":-2,"divisor":1}"#,
        r#"{"quota":1,"usedQuota":2,"divisor":0}"#,
        r#"{"quota":1,"usedQuota":2,"divisor":-1}"#,
        r#"{"quota":1.5,"usedQuota":2,"divisor":1}"#,
        r#"{"quota":1,"usedQuota":2.5,"divisor":1}"#,
        r#"{"quota":9007199254740992,"usedQuota":2,"divisor":1}"#,
        r#"{"quota":1,"usedQuota":9007199254740992,"divisor":1}"#,
        r#"{"quota":1,"usedQuota":2,"divisor":9007199254740992}"#,
        r#"{"quota":1e309,"usedQuota":2,"divisor":1}"#,
        r#"{"quota":1,"usedQuota":2,"divisor":NaN}"#,
        r#"{"quota":1,"usedQuota":2,"divisor":1e-320}"#,
        r#"{"quota":1,"usedQuota":2,"divisor":1,"identity":"synthetic-do-not-export"}"#,
        r#"{"quota":1,"usedQuota":2,"divisor":1,"data":{}}"#,
        r#"{"quota":1,"quota":3,"usedQuota":2,"divisor":1}"#,
        r#"{"quota":9007199254740991.1,"usedQuota":0,"divisor":1}"#,
        r#"{"quota":0,"usedQuota":9007199254740991.1,"divisor":1}"#,
        r#"{"quota":1,"usedQuota":2,"divisor":1} trailing"#,
        r#"{"quota":01,"usedQuota":2,"divisor":1}"#,
        r#"{"quota ":1,"usedQuota":2,"divisor":1}"#,
        r#"{"quota":1,"usedQuota":2,"divisor":9007199254740991.1}"#,
    ] {
        assert_eq!(
            Code0WebsiteCapture::decode_filtered_json(payload.as_bytes()),
            Err(Code0CaptureError::InvalidPayload),
            "invalid synthetic payload must be rejected: {payload}",
        );
    }
}

#[test]
fn modified_capture_values_and_invalid_native_timestamps_cannot_make_a_snapshot() {
    assert_eq!(
        Code0WebsiteCapture::decode_filtered_json(&[b' '; 2049]),
        Err(Code0CaptureError::InvalidPayload)
    );
    let original =
        Code0WebsiteCapture::decode_filtered_json(br#"{"quota":1,"usedQuota":2,"divisor":1}"#)
            .unwrap();
    assert_eq!(
        original.make_snapshot("not-a-date"),
        Err(Code0CaptureError::InvalidPayload)
    );
    for invalid in [f64::NAN, f64::INFINITY, -1.0] {
        let mut balance = original;
        balance.balance = invalid;
        assert_eq!(
            balance.make_snapshot("2026-10-26T12:00:00Z"),
            Err(Code0CaptureError::InvalidPayload)
        );
        let mut used = original;
        used.cumulative_usage = invalid;
        assert_eq!(
            used.make_snapshot("2026-10-26T12:00:00Z"),
            Err(Code0CaptureError::InvalidPayload)
        );
    }
}

#[test]
fn code0_identity_and_metric_semantics_decode_through_existing_json_entrypoints() {
    let provider: ProviderId = serde_json::from_value(json!("code0")).expect("Code0 identity");
    assert_eq!(provider.as_str(), "code0");
    let unit: MetricUnit = serde_json::from_value(json!("siteCredits")).expect("site-credit unit");
    assert_eq!(serde_json::to_value(unit).unwrap(), "siteCredits");
    let kind: MetricKind =
        serde_json::from_value(json!("cumulativeUsage")).expect("cumulative usage kind");
    assert_eq!(serde_json::to_value(kind).unwrap(), "cumulativeUsage");
    let legacy: ProviderId = serde_json::from_value(json!("deepseek")).expect("legacy identity");
    assert_eq!(legacy, ProviderId::DeepSeek);
}

#[test]
fn dedicated_seventh_fixture_decodes_without_currency_ratio_or_account_data() {
    let value: Value = serde_json::from_slice(include_bytes!(
        "../../../contracts/fixtures/code0-site-credits.json"
    ))
    .expect("dedicated synthetic fixture json");
    let snapshot = UsageSnapshot::decode_compatible(&value).expect("Code0 fixture must decode");
    assert_eq!(snapshot.provider_id.as_str(), "code0");
    assert_eq!(snapshot.display_name, "Code0");
    assert!(snapshot.used_ratio.is_none());
    assert_eq!(snapshot.primary_metric.unwrap().current, 2.5);
    assert_eq!(snapshot.secondary_metric.unwrap().current, 1.5);
    assert!(snapshot.daily_history.is_empty());
    assert!(snapshot.local_activity.is_none());
    let actual_keys = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    let expected_keys = [
        "schemaVersion",
        "providerId",
        "displayName",
        "status",
        "usedRatio",
        "primaryMetric",
        "secondaryMetric",
        "fetchedAt",
        "staleAfterSeconds",
        "sourceVersion",
    ]
    .into_iter()
    .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(actual_keys, expected_keys);
}
