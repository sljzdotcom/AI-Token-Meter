use ai_token_meter_windows::{
    accounts::service_status::ServiceAccountConnectionState,
    collectors::{CollectionError, gemini::service_status},
    domain::{ProviderId, UsageSnapshot, UsageStatus},
    persistence::{SnapshotCache, UsageRuntime},
};
#[test]
fn successful_tiers_roundtrip_and_failed_refresh_preserves_timestamp_and_account_reason() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = UsageRuntime::load(SnapshotCache::new(dir.path()), "2026-09-08T08:00:00Z");
    let snapshot: UsageSnapshot = serde_json::from_str(include_str!(
        "../../../contracts/fixtures/gemini-fresh.json"
    ))
    .unwrap();
    let generation = runtime.begin_refresh(ProviderId::Gemini);
    assert!(runtime.complete_success(ProviderId::Gemini, generation, snapshot.clone()));
    assert_eq!(
        service_status(&runtime.snapshot(ProviderId::Gemini)).connection_state,
        ServiceAccountConnectionState::Connected
    );
    for error in [
        CollectionError::Transport,
        CollectionError::AuthenticationRequired,
        CollectionError::UnsupportedConfiguration,
    ] {
        let generation = runtime.begin_refresh(ProviderId::Gemini);
        assert!(runtime.complete_failure(
            ProviderId::Gemini,
            generation,
            error,
            "2026-09-08T10:00:00Z"
        ));
        let cached = runtime.snapshot(ProviderId::Gemini);
        assert_eq!(cached.status, UsageStatus::Cached);
        assert_eq!(cached.fetched_at, snapshot.fetched_at);
        assert_eq!(cached.gemini_quota_metrics, snapshot.gemini_quota_metrics);
        assert_eq!(
            service_status(&cached).connection_state,
            if error == CollectionError::AuthenticationRequired {
                ServiceAccountConnectionState::SignInRequired
            } else {
                ServiceAccountConnectionState::Unavailable
            }
        );
        assert!(service_status(&cached).account_label.is_none());
    }
    let loaded = UsageRuntime::load(SnapshotCache::new(dir.path()), "2026-09-08T11:00:00Z")
        .snapshot(ProviderId::Gemini);
    assert_eq!(loaded.gemini_quota_metrics, snapshot.gemini_quota_metrics);
    assert_eq!(loaded.fetched_at, snapshot.fetched_at);
}
