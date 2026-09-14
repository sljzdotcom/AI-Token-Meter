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
        assert_eq!(cached.antigravity_cli_info, snapshot.antigravity_cli_info);
        assert_eq!(
            cached
                .gemini_quota_metrics
                .iter()
                .map(|metric| metric.label.as_str())
                .collect::<Vec<_>>(),
            vec!["Gemini · Five hour", "Gemini · Weekly"]
        );
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
    assert_eq!(
        loaded
            .gemini_quota_metrics
            .iter()
            .map(|metric| metric.label.as_str())
            .collect::<Vec<_>>(),
        vec!["Gemini · Five hour", "Gemini · Weekly"]
    );
    assert_eq!(loaded.used_ratio.unwrap().get(), 0.6);
    assert_eq!(loaded.primary_metric.unwrap().label, "Gemini · Five hour");
    assert_eq!(loaded.fetched_at, snapshot.fetched_at);
    assert_eq!(loaded.antigravity_cli_info, snapshot.antigravity_cli_info);
}

#[test]
fn legacy_gemini_cache_keeps_quota_but_migrates_the_visible_provider_name() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path()).unwrap();
    std::fs::write(
        dir.path().join("gemini.json"),
        r#"{
          "schemaVersion":1,
          "providerId":"gemini",
          "displayName":"Gemini",
          "status":"fresh",
          "usedRatio":0.6,
          "primaryMetric":{"label":"Flash","current":60,"limit":100,"unit":"percent","kind":"officialLimit"},
          "geminiQuotaMetrics":[{"label":"Flash","current":60,"limit":100,"unit":"percent","kind":"officialLimit"}],
          "fetchedAt":"2026-09-08T08:47:00Z",
          "staleAfterSeconds":300,
          "sourceVersion":"0.58.0",
          "antigravityCLIInfo":{"currentModel":"Claude Sonnet","availableModelCount":1,"modelFamilies":["Claude Sonnet"]}
        }"#,
    )
    .unwrap();

    let snapshot = UsageRuntime::load(SnapshotCache::new(dir.path()), "2026-09-10T00:00:00Z")
        .snapshot(ProviderId::Gemini);
    assert_eq!(snapshot.display_name, "Google Antigravity");
    assert_eq!(snapshot.status, UsageStatus::Cached);
    assert!(snapshot.used_ratio.is_none());
    assert!(snapshot.primary_metric.is_none());
    assert!(snapshot.secondary_metric.is_none());
    assert!(snapshot.gemini_quota_metrics.is_empty());
    assert!(snapshot.antigravity_cli_info.is_none());
}

#[test]
fn legacy_four_window_cache_keeps_only_gemini_and_recomputes_the_summary() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path()).unwrap();
    std::fs::write(
        dir.path().join("gemini.json"),
        r#"{
          "schemaVersion":1,
          "providerId":"gemini",
          "displayName":"Google Antigravity",
          "status":"fresh",
          "usedRatio":0.8,
          "primaryMetric":{"label":"Claude/GPT · Five hour","current":80,"limit":100,"unit":"percent","kind":"officialLimit","resetAt":"2026-09-14T10:00:00Z"},
          "secondaryMetric":{"label":"Gemini · Five hour","current":60,"limit":100,"unit":"percent","kind":"officialLimit","resetAt":"2026-09-14T10:00:00Z"},
          "geminiQuotaMetrics":[
            {"label":"Gemini · Five hour","current":60,"limit":100,"unit":"percent","kind":"officialLimit","resetAt":"2026-09-14T10:00:00Z"},
            {"label":"Gemini · Weekly","current":25,"limit":100,"unit":"percent","kind":"officialLimit","resetAt":"2026-09-14T10:00:00Z"},
            {"label":"Claude/GPT · Five hour","current":80,"limit":100,"unit":"percent","kind":"officialLimit","resetAt":"2026-09-14T10:00:00Z"},
            {"label":"Claude/GPT · Weekly","current":20,"limit":100,"unit":"percent","kind":"officialLimit","resetAt":"2026-09-14T10:00:00Z"}
          ],
          "fetchedAt":"2026-09-14T08:47:00Z",
          "staleAfterSeconds":300,
          "sourceVersion":"1.1.28"
        }"#,
    )
    .unwrap();

    let snapshot = UsageRuntime::load(SnapshotCache::new(dir.path()), "2026-09-14T11:00:00Z")
        .snapshot(ProviderId::Gemini);
    assert_eq!(snapshot.status, UsageStatus::Cached);
    assert_eq!(snapshot.used_ratio.unwrap().get(), 0.6);
    assert_eq!(snapshot.primary_metric.unwrap().label, "Gemini · Five hour");
    assert_eq!(snapshot.secondary_metric.unwrap().label, "Gemini · Weekly");
    assert_eq!(
        snapshot
            .gemini_quota_metrics
            .iter()
            .map(|metric| metric.label.as_str())
            .collect::<Vec<_>>(),
        vec!["Gemini · Five hour", "Gemini · Weekly"]
    );
}

#[test]
fn cached_sensitive_or_non_gemini_model_information_is_discarded() {
    for unsafe_model in [
        "Claude Sonnet",
        "Gemini user@example.com",
        "Gemini /Users/example/.config",
        "Gemini sk-proj-secretvalue",
        "Gemini Claude/GPT",
        "Gemini 3.8 Flash 13800000000",
        "Gemini 3.8 .-",
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path()).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../contracts/fixtures/gemini-fresh.json"
        ))
        .unwrap();
        value["antigravityCLIInfo"]["currentModel"] = unsafe_model.into();
        value["antigravityCLIInfo"]["modelFamilies"] = serde_json::json!([unsafe_model]);
        std::fs::write(
            dir.path().join("gemini.json"),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();

        let snapshot = UsageRuntime::load(SnapshotCache::new(dir.path()), "2026-09-14T11:00:00Z")
            .snapshot(ProviderId::Gemini);
        assert!(
            snapshot.antigravity_cli_info.is_none(),
            "accepted {unsafe_model}"
        );
    }
}
