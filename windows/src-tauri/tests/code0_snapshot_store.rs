#![cfg(windows)]

use std::collections::BTreeSet;
use std::fs;
use std::time::Duration;

use ai_token_meter_windows::collectors::code0_capture::Code0WebsiteCapture;
use ai_token_meter_windows::domain::{UsageSnapshot, UsageStatus};
use ai_token_meter_windows::persistence::{
    Code0SnapshotFailurePoint, Code0SnapshotStore, Code0SnapshotTestGate,
};
use ai_token_meter_windows::platform::windows::code0_profile_lease::Code0ProfileLease;
use ai_token_meter_windows::platform::windows::code0_web_session::{
    Code0SnapshotStoring, Code0StoreError,
};
use tempfile::tempdir;

const FETCHED_AT: &str = "2026-10-27T04:05:06Z";

fn expected_snapshot() -> UsageSnapshot {
    Code0WebsiteCapture::decode_filtered_json(
        br#"{"quota":49400,"usedQuota":182700,"divisor":400}"#,
    )
    .expect("valid synthetic Code0 capture")
    .make_snapshot(FETCHED_AT)
    .expect("native fetched time")
}

fn candidate_snapshot() -> UsageSnapshot {
    Code0WebsiteCapture::decode_filtered_json(
        br#"{"quota":49600,"usedQuota":183000,"divisor":400}"#,
    )
    .expect("valid synthetic candidate capture")
    .make_snapshot("2026-10-27T04:06:06Z")
    .expect("native candidate fetched time")
}

fn observe_blocked_candidate(root: &std::path::Path) -> Result<serde_json::Value, String> {
    if !root.join("snapshot.pending").exists() {
        return Err("pending barrier is absent while candidate gate is held".to_owned());
    }
    let bytes = fs::read(root.join("snapshot.json")).map_err(|error| error.to_string())?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if value["balance"] != 124.0 || value["cumulativeUsage"] != 457.5 {
        return Err("the actual installed projection is not candidate B".to_owned());
    }
    if !matches!(
        Code0SnapshotStore::new(root, None).load(),
        Err(Code0StoreError::Unavailable)
    ) {
        return Err("the pending barrier did not reject a cold load as unavailable".to_owned());
    }
    Ok(value)
}

#[test]
fn cold_load_returns_none_only_for_a_proven_absent_snapshot_without_creating_profile() {
    let temporary = tempdir().expect("isolated temporary directory");
    let root = temporary.path().join("Code0");
    let store = Code0SnapshotStore::new(&root, None);

    assert!(matches!(store.load(), Ok(None)));
    assert!(
        !root.exists(),
        "read-only cold load must not create the profile"
    );
}

#[test]
fn save_round_trip_persists_only_the_five_field_projection_and_loads_cached() {
    let temporary = tempdir().expect("isolated temporary directory");
    let root = temporary.path().join("Code0");
    let lease = Code0ProfileLease::acquire(&root).expect("acquire real profile lease");
    let mut store = Code0SnapshotStore::new(&root, Some(lease));
    let snapshot = expected_snapshot();

    store
        .save_atomic(&snapshot)
        .expect("persist before publishing snapshot");
    let path = root.join("snapshot.json");
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(path).expect("snapshot bytes")).expect("snapshot JSON");
    let keys = value
        .as_object()
        .expect("projection object")
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        keys,
        BTreeSet::from([
            "balance",
            "cumulativeUsage",
            "fetchedAt",
            "schemaVersion",
            "sourceVersion",
        ])
    );
    assert_eq!(value["schemaVersion"], 1);
    assert_eq!(value["balance"], 123.5);
    assert_eq!(value["cumulativeUsage"], 456.75);
    assert_eq!(value["sourceVersion"], "website");
    assert!(value.get("cookies").is_none());
    assert!(value.get("account").is_none());

    let loaded = store
        .load()
        .expect("load committed snapshot")
        .expect("snapshot exists");
    assert_eq!(loaded.status, UsageStatus::Cached);
    assert_eq!(loaded.provider_id.as_str(), "code0");
    assert_eq!(
        loaded.primary_metric.expect("balance metric").current,
        123.5
    );
    assert_eq!(
        loaded.secondary_metric.expect("usage metric").current,
        456.75
    );
}

#[test]
fn writes_and_removal_require_the_same_live_authority_and_leave_other_data_untouched() {
    let temporary = tempdir().expect("isolated temporary directory");
    let root = temporary.path().join("Code0");
    let mut unowned = Code0SnapshotStore::new(&root, None);
    assert_eq!(
        unowned.save_atomic(&expected_snapshot()),
        Err(Code0StoreError::Unavailable)
    );
    assert_eq!(unowned.remove(), Err(Code0StoreError::Unavailable));
    assert!(!root.exists());

    let lease = Code0ProfileLease::acquire(&root).expect("acquire real profile lease");
    let mut wrong_root =
        Code0SnapshotStore::new(temporary.path().join("other-Code0"), Some(lease.clone()));
    assert_eq!(
        wrong_root.save_atomic(&expected_snapshot()),
        Err(Code0StoreError::Unavailable)
    );
    assert_eq!(wrong_root.remove(), Err(Code0StoreError::Unavailable));
    let mut store = Code0SnapshotStore::new(&root, Some(lease.clone()));
    fs::create_dir_all(&root).expect("temporary isolated Code0 root");
    let ordinary_cache = temporary.path().join("cache");
    let settings = temporary.path().join("settings.json");
    fs::create_dir_all(&ordinary_cache).expect("ordinary cache fixture");
    fs::write(ordinary_cache.join("deepseek.json"), b"sentinel").expect("cache sentinel");
    fs::write(&settings, b"settings sentinel").expect("settings sentinel");

    store
        .save_atomic(&expected_snapshot())
        .expect("save under held authority");
    lease.release();
    assert_eq!(store.remove(), Err(Code0StoreError::Unavailable));
    assert_eq!(
        fs::read(ordinary_cache.join("deepseek.json")).unwrap(),
        b"sentinel"
    );
    assert_eq!(fs::read(&settings).unwrap(), b"settings sentinel");
}

#[test]
fn save_rejects_invalid_site_credit_values_without_replacing_a_snapshot() {
    let temporary = tempdir().expect("isolated temporary directory");
    let root = temporary.path().join("Code0");
    let lease = Code0ProfileLease::acquire(&root).expect("acquire profile lease");
    let mut store = Code0SnapshotStore::new(&root, Some(lease));
    let mut invalid = expected_snapshot();
    invalid
        .primary_metric
        .as_mut()
        .expect("balance metric")
        .current = -1.0;

    assert_eq!(
        store.save_atomic(&invalid),
        Err(Code0StoreError::InvalidSnapshot)
    );
    assert!(!root.join("snapshot.json").exists());
}

#[test]
fn post_replace_failure_restores_previous_snapshot_before_any_new_load_can_accept_candidate() {
    let temporary = tempdir().expect("isolated temporary directory");
    let root = temporary.path().join("Code0");
    let lease = Code0ProfileLease::acquire(&root).expect("acquire profile lease");
    let mut initial = Code0SnapshotStore::new(&root, Some(lease.clone()));
    initial
        .save_atomic(&expected_snapshot())
        .expect("save initial snapshot A");

    let previous_projection: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("snapshot.json")).expect("initial A projection"),
    )
    .expect("valid initial projection");
    let gate = Code0SnapshotTestGate::new();
    let worker_gate = gate.clone();
    let worker_root = root.clone();
    let worker = std::thread::spawn(move || {
        let mut failing = Code0SnapshotStore::with_failure_gate(
            worker_root,
            Some(lease),
            [Code0SnapshotFailurePoint::AfterCandidateReplace],
            worker_gate,
        );
        failing.save_atomic(&candidate_snapshot())
    });
    let reached = gate.wait_until_after_replace(Duration::from_secs(5));
    let observation = if reached {
        observe_blocked_candidate(&root)
    } else {
        Err("candidate replace gate was not reached".to_owned())
    };
    gate.release();
    let save_result = worker.join().expect("save worker exits after gate release");

    assert!(
        reached,
        "fault injection must occur after candidate replacement"
    );
    let candidate_projection = observation.expect("candidate B and pending barrier are visible");
    assert_ne!(
        candidate_projection["fetchedAt"], previous_projection["fetchedAt"],
        "the blocked file is the newer candidate, not old A"
    );
    assert_eq!(save_result, Err(Code0StoreError::Unavailable));
    assert!(!root.join("snapshot.pending").exists());

    let loaded = Code0SnapshotStore::new(&root, None)
        .load()
        .expect("recovered A is readable")
        .expect("A remains present");
    assert_eq!(loaded.status, UsageStatus::Cached);
    assert_eq!(loaded.fetched_at, FETCHED_AT, "A keeps its old native time");
    assert_eq!(loaded.primary_metric.expect("balance").current, 123.5);
    assert_eq!(
        loaded.secondary_metric.expect("cumulative usage").current,
        456.75
    );
}

#[test]
fn failed_restore_keeps_pending_barrier_and_cannot_publish_candidate_on_reopen() {
    let temporary = tempdir().expect("isolated temporary directory");
    let root = temporary.path().join("Code0");
    let lease = Code0ProfileLease::acquire(&root).expect("acquire profile lease");
    let mut initial = Code0SnapshotStore::new(&root, Some(lease.clone()));
    initial
        .save_atomic(&expected_snapshot())
        .expect("save initial snapshot A");

    let gate = Code0SnapshotTestGate::new();
    let worker_gate = gate.clone();
    let worker_root = root.clone();
    let worker = std::thread::spawn(move || {
        let mut failing = Code0SnapshotStore::with_failure_gate(
            worker_root,
            Some(lease),
            [
                Code0SnapshotFailurePoint::AfterCandidateReplace,
                Code0SnapshotFailurePoint::RestorePreviousSnapshot,
            ],
            worker_gate,
        );
        failing.save_atomic(&candidate_snapshot())
    });
    let reached = gate.wait_until_after_replace(Duration::from_secs(5));
    let observation = if reached {
        observe_blocked_candidate(&root)
    } else {
        Err("candidate replace gate was not reached".to_owned())
    };
    gate.release();
    let save_result = worker.join().expect("save worker exits after gate release");

    assert!(
        reached,
        "fault injection must occur after candidate replacement"
    );
    let candidate_projection = observation.expect("candidate B and pending barrier are visible");
    assert_eq!(candidate_projection["balance"], 124.0);
    assert_eq!(save_result, Err(Code0StoreError::Unavailable));
    assert!(
        root.join("snapshot.pending").exists(),
        "failed restoration leaves the visibility barrier"
    );
    assert_eq!(
        Code0SnapshotStore::new(&root, None).load(),
        Err(Code0StoreError::Unavailable),
        "pending state blocks a reopened store too"
    );
}

#[test]
fn post_replace_failure_from_absent_state_removes_candidate_before_next_load() {
    let temporary = tempdir().expect("isolated temporary directory");
    let root = temporary.path().join("Code0");
    let lease = Code0ProfileLease::acquire(&root).expect("acquire profile lease");
    let gate = Code0SnapshotTestGate::new();
    let worker_gate = gate.clone();
    let worker_root = root.clone();
    let worker = std::thread::spawn(move || {
        let mut failing = Code0SnapshotStore::with_failure_gate(
            worker_root,
            Some(lease),
            [Code0SnapshotFailurePoint::AfterCandidateReplace],
            worker_gate,
        );
        failing.save_atomic(&candidate_snapshot())
    });
    let reached = gate.wait_until_after_replace(Duration::from_secs(5));
    let observation = if reached {
        observe_blocked_candidate(&root)
    } else {
        Err("candidate replace gate was not reached".to_owned())
    };
    gate.release();
    let save_result = worker.join().expect("save worker exits after gate release");

    assert!(
        reached,
        "fault injection must occur after candidate replacement"
    );
    let candidate_projection = observation.expect("candidate B and pending barrier are visible");
    assert_ne!(candidate_projection["balance"], 0.0);
    assert_eq!(save_result, Err(Code0StoreError::Unavailable));
    assert!(!root.join("snapshot.pending").exists());
    assert!(!root.join("snapshot.json").exists());
    assert!(matches!(
        Code0SnapshotStore::new(&root, None).load(),
        Ok(None)
    ));
}
