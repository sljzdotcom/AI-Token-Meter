use std::path::PathBuf;

use ai_token_meter_windows::domain::{ProviderId, UsageSnapshot};
use ai_token_meter_windows::persistence::{
    AppSettings, AppStoragePaths, AtomicJsonStore, SnapshotCache,
};
use serde::ser::{Error as _, Serialize, Serializer};
use tempfile::tempdir;

#[test]
fn settings_round_trip_atomically() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("settings.json");
    let settings = AppSettings {
        edge: ai_token_meter_windows::persistence::MeterEdge::Left,
        refresh_interval_seconds: 420,
        detail_auto_hide_seconds: 12,
        display_font: "Antonio".to_owned(),
    };

    AtomicJsonStore::write(&path, &settings).expect("write settings");
    let decoded = AtomicJsonStore::read::<AppSettings>(&path)
        .expect("read settings")
        .expect("settings exist");

    assert_eq!(decoded, settings);
    assert!(!temporary_path(&path).exists());
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
