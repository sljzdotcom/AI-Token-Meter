use tempfile::tempdir;

use ai_token_meter_windows::collectors::CollectionError;
use ai_token_meter_windows::domain::{LocalActivity, ProviderId, UsageSnapshot, UsageStatus};
use ai_token_meter_windows::persistence::{SnapshotCache, UsageRuntime};

const NOW: &str = "2026-09-03T12:00:00Z";

#[test]
fn startup_loads_each_valid_cache_as_cached_without_inventing_missing_data() {
    let directory = tempdir().expect("temporary directory");
    let cache = SnapshotCache::new(directory.path());
    cache
        .save(&fixture(ProviderId::Claude))
        .expect("save cache");

    let runtime = UsageRuntime::load(cache, NOW);
    let snapshots = runtime.snapshots();

    assert_eq!(snapshots.len(), 3);
    assert_eq!(
        snapshot(&snapshots, ProviderId::Claude).status,
        UsageStatus::Cached
    );
    assert_eq!(
        snapshot(&snapshots, ProviderId::Codex).status,
        UsageStatus::Unavailable
    );
    assert_eq!(snapshot(&snapshots, ProviderId::Codex).used_ratio, None);
}

#[test]
fn refresh_keeps_cached_values_visible_and_rejects_an_older_completion() {
    let directory = tempdir().expect("temporary directory");
    let cache = SnapshotCache::new(directory.path());
    cache.save(&fixture(ProviderId::Codex)).expect("save cache");
    let runtime = UsageRuntime::load(cache, NOW);

    let old_generation = runtime.begin_refresh(ProviderId::Codex);
    let new_generation = runtime.begin_refresh(ProviderId::Codex);
    assert_eq!(
        runtime.snapshot(ProviderId::Codex).status,
        UsageStatus::Refreshing
    );
    assert!(runtime.snapshot(ProviderId::Codex).primary_metric.is_some());

    let mut newer = fixture(ProviderId::Codex);
    newer.fetched_at = "2026-09-03T12:01:00Z".to_owned();
    assert!(runtime.complete_success(ProviderId::Codex, new_generation, newer));

    let mut older = fixture(ProviderId::Codex);
    older.fetched_at = "2026-09-03T11:59:00Z".to_owned();
    assert!(!runtime.complete_success(ProviderId::Codex, old_generation, older));
    assert_eq!(
        runtime.snapshot(ProviderId::Codex).fetched_at,
        "2026-09-03T12:01:00Z"
    );
}

#[test]
fn one_provider_failure_uses_only_its_own_cache_and_preserves_other_providers() {
    let directory = tempdir().expect("temporary directory");
    let cache = SnapshotCache::new(directory.path());
    cache
        .save(&fixture(ProviderId::Claude))
        .expect("save cache");
    cache
        .save(&fixture(ProviderId::DeepSeek))
        .expect("save cache");
    let runtime = UsageRuntime::load(cache, NOW);
    let deepseek_before = runtime.snapshot(ProviderId::DeepSeek);

    let generation = runtime.begin_refresh(ProviderId::Claude);
    assert!(runtime.complete_failure(
        ProviderId::Claude,
        generation,
        CollectionError::TimedOut,
        NOW,
    ));

    let claude = runtime.snapshot(ProviderId::Claude);
    assert_eq!(claude.status, UsageStatus::Cached);
    assert_eq!(
        claude.status_message.as_deref(),
        Some("Cached · refresh timed out")
    );
    assert_eq!(runtime.snapshot(ProviderId::DeepSeek), deepseek_before);
}

#[test]
fn authentication_failure_never_looks_like_zero_usage() {
    let directory = tempdir().expect("temporary directory");
    let runtime = UsageRuntime::load(SnapshotCache::new(directory.path()), NOW);
    let generation = runtime.begin_refresh(ProviderId::Claude);

    assert!(runtime.complete_failure(
        ProviderId::Claude,
        generation,
        CollectionError::AuthenticationRequired,
        NOW,
    ));

    let snapshot = runtime.snapshot(ProviderId::Claude);
    assert_eq!(snapshot.status, UsageStatus::AuthenticationRequired);
    assert_eq!(snapshot.used_ratio, None);
    assert_eq!(snapshot.primary_metric, None);
}

#[test]
fn balance_refresh_preserves_separately_collected_deepseek_history() {
    let directory = tempdir().expect("temporary directory");
    let cache = SnapshotCache::new(directory.path());
    let mut historical = fixture(ProviderId::DeepSeek);
    historical.daily_history = vec![ai_token_meter_windows::domain::DailyHistoryEntry {
        date: "2026-09-03".to_owned(),
        cost_cny: 1.25,
        requests: 4,
        tokens: 800,
    }];
    historical.history_fetched_at = Some(NOW.to_owned());
    cache.save(&historical).expect("save cache");
    let runtime = UsageRuntime::load(cache, NOW);

    let generation = runtime.begin_refresh(ProviderId::DeepSeek);
    assert!(runtime.complete_success(
        ProviderId::DeepSeek,
        generation,
        fixture(ProviderId::DeepSeek),
    ));

    let snapshot = runtime.snapshot(ProviderId::DeepSeek);
    assert_eq!(snapshot.daily_history, historical.daily_history);
    assert_eq!(snapshot.history_fetched_at, historical.history_fetched_at);
}

#[test]
fn successful_refresh_does_not_reuse_local_activity_omitted_by_current_runtime_source() {
    let directory = tempdir().expect("temporary directory");
    let cache = SnapshotCache::new(directory.path());
    let mut native_snapshot = fixture(ProviderId::Codex);
    native_snapshot.local_activity = Some(LocalActivity {
        period_days: 30,
        sessions: 12,
        tokens: 42_000,
        active_days: 6,
        longest_session_seconds: Some(900),
    });
    cache.save(&native_snapshot).expect("save native cache");
    let runtime = UsageRuntime::load(cache, NOW);

    let generation = runtime.begin_refresh(ProviderId::Codex);
    let current_source_snapshot = fixture(ProviderId::Codex);
    assert!(current_source_snapshot.local_activity.is_none());
    assert!(runtime.complete_success(ProviderId::Codex, generation, current_source_snapshot,));

    assert_eq!(runtime.snapshot(ProviderId::Codex).local_activity, None);
}

fn snapshot(snapshots: &[UsageSnapshot], provider: ProviderId) -> &UsageSnapshot {
    snapshots
        .iter()
        .find(|snapshot| snapshot.provider_id == provider)
        .expect("provider snapshot")
}

fn fixture(provider: ProviderId) -> UsageSnapshot {
    let value = match provider {
        ProviderId::Claude => include_str!("../../../contracts/fixtures/claude-fresh.json"),
        ProviderId::Codex => include_str!("../../../contracts/fixtures/codex-reset-credit.json"),
        ProviderId::DeepSeek => include_str!("../../../contracts/fixtures/deepseek-balance.json"),
    };
    UsageSnapshot::decode_compatible(&serde_json::from_str(value).expect("fixture JSON"))
        .expect("usage fixture")
}
