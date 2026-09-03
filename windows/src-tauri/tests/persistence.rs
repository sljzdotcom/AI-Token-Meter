use std::path::PathBuf;

use ai_token_meter_windows::domain::{ProviderId, UsageSnapshot};
use ai_token_meter_windows::persistence::{
    AppSettings, AppStoragePaths, AtomicJsonStore, CliRuntimeMode, ProviderCliSettings,
    SnapshotCache,
};
use serde::ser::{Error as _, Serialize, Serializer};
use tempfile::tempdir;

#[test]
fn settings_round_trip_atomically() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("settings.json");
    let settings = AppSettings {
        edge: ai_token_meter_windows::persistence::MeterEdge::Left,
        meter_vertical_per_mille: 375,
        meter_monitor_id: Some("DISPLAY-2".to_owned()),
        refresh_interval_seconds: 420,
        detail_auto_hide_seconds: 12,
        display_font: "Antonio".to_owned(),
        ..AppSettings::default()
    };

    AtomicJsonStore::write(&path, &settings).expect("write settings");
    let decoded = AtomicJsonStore::read::<AppSettings>(&path)
        .expect("read settings")
        .expect("settings exist");

    assert_eq!(decoded, settings);
    assert!(!temporary_path(&path).exists());
}

#[test]
fn settings_accept_only_supported_fonts_and_safe_auto_hide_intervals() {
    let mut settings = AppSettings::default();

    settings
        .set_display_font("Fira Code")
        .expect("supported font");
    assert_eq!(settings.display_font, "Fira Code");
    assert!(settings.set_display_font("url(evil)").is_err());
    assert_eq!(settings.display_font, "Fira Code");

    settings
        .set_detail_auto_hide_seconds(30)
        .expect("safe interval");
    assert_eq!(settings.detail_auto_hide_seconds, 30);
    assert!(settings.set_detail_auto_hide_seconds(0).is_err());
    assert!(settings.set_detail_auto_hide_seconds(301).is_err());
    assert_eq!(settings.detail_auto_hide_seconds, 30);

    settings
        .set_refresh_interval_seconds(120)
        .expect("safe refresh interval");
    assert_eq!(settings.refresh_interval_seconds, 120);
    assert!(settings.set_refresh_interval_seconds(10).is_err());

    settings
        .set_deepseek_balance_baseline_cents(25_050)
        .expect("safe balance baseline");
    assert_eq!(settings.deepseek_balance_baseline_cents, 25_050);
    assert!(settings.set_deepseek_balance_baseline_cents(0).is_err());

    settings.notifications_enabled = true;
    settings.launch_at_login = true;
    assert!(settings.notifications_enabled);
    assert!(settings.launch_at_login);
}

#[test]
fn cli_runtime_settings_are_provider_scoped_and_validate_untrusted_text() {
    let mut settings = AppSettings::default();
    let configuration = ProviderCliSettings {
        mode: CliRuntimeMode::Wsl,
        custom_path: None,
        wsl_distribution: Some("Ubuntu-24.04".to_owned()),
    };

    settings
        .set_cli_settings(ProviderId::Codex, configuration.clone())
        .expect("valid WSL selection");
    assert_eq!(
        settings.cli_settings(ProviderId::Codex),
        Some(&configuration)
    );
    assert_eq!(
        settings.cli_settings(ProviderId::Claude),
        Some(&ProviderCliSettings::default())
    );
    assert!(
        settings
            .set_cli_settings(ProviderId::DeepSeek, configuration)
            .is_err()
    );
    assert!(
        settings
            .set_cli_settings(
                ProviderId::Claude,
                ProviderCliSettings {
                    mode: CliRuntimeMode::NativeWindows,
                    custom_path: Some("C:\\tools\\claude.exe\nunsafe".to_owned()),
                    wsl_distribution: None,
                },
            )
            .is_err()
    );
}

#[test]
fn serialization_failure_preserves_the_previous_file() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("settings.json");
    AtomicJsonStore::write(&path, &AppSettings::default()).expect("initial write");
    let original = std::fs::read(&path).expect("initial bytes");

    let error = AtomicJsonStore::write(&path, &FailingValue).expect_err("serialization fails");

    assert!(error.to_string().contains("serialize"));
    assert_eq!(std::fs::read(&path).expect("preserved bytes"), original);
}

#[test]
fn snapshot_cache_round_trips_by_provider() {
    let directory = tempdir().expect("temporary directory");
    let cache = SnapshotCache::new(directory.path());
    let fixture = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../contracts/fixtures/claude-fresh.json"),
    )
    .expect("fixture");
    let value = serde_json::from_str(&fixture).expect("fixture json");
    let snapshot = UsageSnapshot::decode_compatible(&value).expect("snapshot");

    cache.save(&snapshot).expect("save snapshot");
    let decoded = cache
        .load(ProviderId::Claude)
        .expect("load snapshot")
        .expect("snapshot exists");

    assert_eq!(decoded, snapshot);
}

#[test]
fn damaged_json_is_reported_without_modifying_the_file() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("settings.json");
    std::fs::write(&path, b"{not-json").expect("damaged fixture");
    let original = std::fs::read(&path).expect("fixture bytes");

    let error = AtomicJsonStore::read::<AppSettings>(&path).expect_err("decode should fail");

    assert_eq!(error.category(), "decode");
    assert_eq!(std::fs::read(&path).expect("preserved bytes"), original);
}

#[test]
fn windows_storage_paths_separate_roaming_settings_from_local_cache() {
    let paths = AppStoragePaths::from_windows_roots(
        PathBuf::from(r"C:\Users\Example\AppData\Roaming"),
        PathBuf::from(r"C:\Users\Example\AppData\Local"),
    );

    assert_eq!(
        paths.settings_file,
        PathBuf::from(r"C:\Users\Example\AppData\Roaming")
            .join("AI Token Meter")
            .join("settings.json")
    );
    assert_eq!(
        paths.cache_directory,
        PathBuf::from(r"C:\Users\Example\AppData\Local")
            .join("AI Token Meter")
            .join("cache")
    );
}

struct FailingValue;

impl Serialize for FailingValue {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Err(S::Error::custom("intentional serialization failure"))
    }
}

fn temporary_path(path: &std::path::Path) -> PathBuf {
    path.with_extension("json.tmp")
}
