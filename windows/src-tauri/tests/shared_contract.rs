use std::fs;
use std::path::PathBuf;

use ai_token_meter_windows::domain::{
    ProgressSemantics, ProviderId, UsageSnapshot, UsageStatus, embedded_provider_presentations,
};
use serde_json::{Value, json};

#[test]
fn every_shared_fixture_decodes_into_the_windows_domain() {
    let fixtures = fixtures_directory();
    let mut paths = fs::read_dir(fixtures)
        .expect("fixture directory")
        .map(|entry| entry.expect("fixture entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    paths.sort();

    assert_eq!(paths.len(), 4);
    for path in paths {
        let value: Value =
            serde_json::from_slice(&fs::read(&path).expect("fixture bytes")).expect("fixture json");
        let snapshot = UsageSnapshot::decode_compatible(&value).expect("compatible fixture");

        assert_eq!(snapshot.schema_version, 1);
        assert!(snapshot.used_ratio.is_none_or(|ratio| ratio.get() <= 1.0));
    }
}

#[test]
fn deepseek_fixture_preserves_consumed_balance_ratio() {
    let value: Value = serde_json::from_slice(
        &fs::read(fixtures_directory().join("deepseek-balance.json")).expect("fixture bytes"),
    )
    .expect("fixture json");

    let snapshot = UsageSnapshot::decode_compatible(&value).expect("compatible fixture");

    assert_eq!(snapshot.provider_id, ProviderId::DeepSeek);
    assert_eq!(snapshot.used_ratio.expect("ratio").get(), 0.2201);
    assert_eq!(snapshot.primary_metric.expect("balance").current, 77.99);
}

#[test]
fn unknown_major_schema_never_surfaces_a_number() {
    let value = json!({
        "schemaVersion": 99,
        "providerId": "codex",
        "displayName": "OpenAI Codex",
        "status": "fresh",
        "usedRatio": 0.91,
        "fetchedAt": "2026-09-03T00:00:00Z",
        "staleAfterSeconds": 300
    });

    let snapshot = UsageSnapshot::decode_compatible(&value).expect("safe fallback");

    assert_eq!(snapshot.provider_id, ProviderId::Codex);
    assert_eq!(snapshot.status, UsageStatus::Unavailable);
    assert_eq!(snapshot.used_ratio, None);
    assert_eq!(snapshot.primary_metric, None);
}

#[test]
fn an_out_of_range_ratio_is_rejected_instead_of_clamped_silently() {
    let value = json!({
        "schemaVersion": 1,
        "providerId": "claude",
        "displayName": "Claude Code",
        "status": "fresh",
        "usedRatio": 1.1,
        "fetchedAt": "2026-09-03T00:00:00Z",
        "staleAfterSeconds": 300
    });

    let error = UsageSnapshot::decode_compatible(&value).expect_err("ratio should be invalid");

    assert!(error.to_string().contains("usedRatio"));
}

#[test]
fn older_cache_without_optional_fields_and_unknown_fields_remains_compatible() {
    let value = json!({
        "schemaVersion": 1,
        "providerId": "claude",
        "displayName": "Claude Code",
        "status": "cached",
        "fetchedAt": "2026-09-03T00:00:00Z",
        "staleAfterSeconds": 300,
        "futureField": { "safeToIgnore": true }
    });

    let snapshot = UsageSnapshot::decode_compatible(&value).expect("compatible old cache");

    assert_eq!(snapshot.status, UsageStatus::Cached);
    assert_eq!(snapshot.used_ratio, None);
    assert!(snapshot.reset_credits.is_empty());
    assert!(snapshot.daily_history.is_empty());
}

#[test]
fn staleness_uses_fetched_time_plus_the_contract_duration() {
    let value: Value = serde_json::from_slice(
        &fs::read(fixtures_directory().join("claude-fresh.json")).expect("fixture bytes"),
    )
    .expect("fixture json");
    let snapshot = UsageSnapshot::decode_compatible(&value).expect("snapshot");

    assert!(
        !snapshot
            .is_stale_at_rfc3339("2026-09-03T00:04:59Z")
            .expect("comparison")
    );
    assert!(
        snapshot
            .is_stale_at_rfc3339("2026-09-03T00:05:00Z")
            .expect("comparison")
    );
}

#[test]
fn presentation_identity_comes_from_the_embedded_shared_contract() {
    let providers = embedded_provider_presentations().expect("presentation contract");

    assert_eq!(providers.len(), 3);
    assert_eq!(providers[0].display_name, "Claude Code");
    assert_eq!(providers[1].display_name, "OpenAI Codex");
    assert_eq!(providers[2].display_name, "DeepSeek");
    assert_eq!(
        providers[2].progress_semantics,
        ProgressSemantics::ConsumedFromBalanceBaseline
    );
}

fn fixtures_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../contracts/fixtures")
}
