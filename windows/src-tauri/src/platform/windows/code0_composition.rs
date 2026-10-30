use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use super::code0_profile_lease::Code0LeaseError;
use super::code0_web_session::{Code0SnapshotStoring, Code0StoreError};
pub(crate) use super::code0_webview::{Code0LeaseSlot, cold_code0_lease_slot};
use crate::domain::UsageSnapshot;
use crate::persistence::Code0SnapshotStore;

/// Derive the one profile root from native LOCALAPPDATA without filesystem I/O.
/// Lexical acceptance is not authority: the existing store/lease guards still
/// validate every actual existing component and its native handles on use.
pub(crate) fn code0_profile_root(
    local_app_data: Option<&OsStr>,
) -> Result<PathBuf, Code0LeaseError> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use std::path::{Component, Prefix};

        let base = Path::new(local_app_data.ok_or(Code0LeaseError::Unavailable)?);
        let mut components = base.components();
        if !base.is_absolute()
            || !matches!(components.next(), Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
            || !matches!(components.next(), Some(Component::RootDir))
        {
            return Err(Code0LeaseError::Unavailable);
        }
        let units = base.as_os_str().encode_wide().collect::<Vec<_>>();
        if units.contains(&0)
            || units
                .split(|unit| matches!(*unit, 47 | 92))
                .any(|part| part == [46] || part == [46, 46])
            || !components.all(|component| match component {
                Component::Normal(name) => {
                    let name = name.encode_wide().collect::<Vec<_>>();
                    !name.iter().any(|unit| {
                        *unit < 32 || matches!(*unit, 34 | 42 | 47 | 58 | 60 | 62 | 63 | 92 | 124)
                    }) && !matches!(name.last(), Some(32 | 46))
                }
                _ => false,
            })
        {
            return Err(Code0LeaseError::Unavailable);
        }
        Ok(base.join("AI Token Meter Code0"))
    }
    #[cfg(not(windows))]
    {
        let _ = local_app_data;
        Err(Code0LeaseError::Unavailable)
    }
}

/// Select the current shared authority for every operation, including load.
/// This facade performs no acquisition and preserves the existing transaction,
/// security, recovery and no-follow behavior of Code0SnapshotStore unchanged.
pub(crate) struct Code0SlotSnapshotStore {
    root: PathBuf,
    slot: Code0LeaseSlot,
}

impl Code0SlotSnapshotStore {
    pub(crate) fn new(root: impl AsRef<Path>, slot: Code0LeaseSlot) -> Self {
        Self {
            root: root.as_ref().to_owned(),
            slot,
        }
    }

    fn delegate(&self) -> Result<Code0SnapshotStore, Code0StoreError> {
        let lease = {
            let authority = self
                .slot
                .try_borrow()
                .map_err(|_| Code0StoreError::Unavailable)?;
            authority.clone()
        };
        // No slot borrow survives delegation or the delegate/authority Drop.
        Ok(Code0SnapshotStore::new(&self.root, lease))
    }
}

impl Code0SnapshotStoring for Code0SlotSnapshotStore {
    fn load(&self) -> Result<Option<UsageSnapshot>, Code0StoreError> {
        self.delegate()?.load()
    }

    fn save_atomic(&mut self, snapshot: &UsageSnapshot) -> Result<(), Code0StoreError> {
        self.delegate()?.save_atomic(snapshot)
    }

    fn remove(&mut self) -> Result<(), Code0StoreError> {
        self.delegate()?.remove()
    }
}

// The isolated Windows definitions below are intentionally UNRUN in this slice.
#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;

    use crate::collectors::code0_capture::Code0WebsiteCapture;
    use crate::domain::{UsageSnapshot, UsageStatus};
    use crate::platform::windows::code0_profile_lease::Code0ProfileLease;
    use crate::platform::windows::code0_web_session::{
        Code0CleanupRequest, Code0SessionClock, Code0SessionFailure, Code0SessionState,
        Code0WebSession, Code0WindowDriving, Code0WindowRequest,
    };
    use tempfile::tempdir;

    fn snapshot() -> UsageSnapshot {
        Code0WebsiteCapture::decode_filtered_json(
            br#"{"quota":49400,"usedQuota":182700,"divisor":400}"#,
        )
        .expect("synthetic numerical capture")
        .make_snapshot("2026-10-29T04:05:06Z")
        .expect("native success timestamp")
    }

    struct ColdClock;

    impl Code0SessionClock for ColdClock {
        fn now(&self) -> Duration {
            Duration::ZERO
        }

        fn wall_now(&self) -> String {
            "2026-10-29T04:05:06Z".into()
        }
    }

    struct NoNativeWork;

    impl Code0WindowDriving for NoNativeWork {
        fn acquire_profile_lease(&mut self) -> Result<(), Code0SessionFailure> {
            panic!("cold session must not acquire a lease")
        }

        fn release_profile_lease(&mut self) {
            panic!("cold session must not release a lease")
        }

        fn create_unloaded(&mut self, _: Code0WindowRequest) -> Result<(), Code0SessionFailure> {
            panic!("cold session must not create a window")
        }

        fn navigate(&mut self, _: &str) -> Result<(), Code0SessionFailure> {
            panic!("cold session must not navigate")
        }

        fn close(&mut self, _: &str) {
            panic!("cold session must not close a window")
        }

        fn clear_website_data(
            &mut self,
            _: Code0CleanupRequest,
        ) -> Result<(), Code0SessionFailure> {
            panic!("cold session must not clear website data")
        }
    }

    // Catches a fallback path or unsupported authority becoming an accepted root.
    #[test]
    fn derivation_accepts_only_absolute_local_disk_roots() {
        for (base, expected) in [
            (
                r"C:\Users\fixture\AppData\Local",
                r"C:\Users\fixture\AppData\Local\AI Token Meter Code0",
            ),
            (
                r"\\?\C:\Users\fixture\AppData\Local",
                r"\\?\C:\Users\fixture\AppData\Local\AI Token Meter Code0",
            ),
        ] {
            assert_eq!(
                code0_profile_root(Some(OsStr::new(base))),
                Ok(PathBuf::from(expected))
            );
        }
        assert_eq!(code0_profile_root(None), Err(Code0LeaseError::Unavailable));
        for base in [
            "",
            "relative",
            r"C:relative",
            r"\rooted",
            r"\\server\share\Local",
            r"\\?\UNC\server\share\Local",
            r"\\.\C:\Local",
            r"\\?\GLOBALROOT\Device\Local",
            r"C:\Local\..\other",
            r"C:\Local\.\other",
            r"C:\Local:stream",
            r"C:\Local\trailing.",
            r"C:\Local\trailing ",
            r"C:\Local\wild*card",
            r"\\?\C:\Local/other",
            "C:\\Local\0other",
        ] {
            assert_eq!(
                code0_profile_root(Some(OsStr::new(base))),
                Err(Code0LeaseError::Unavailable)
            );
        }
    }

    // Catches startup acquisition, directory creation, and write permission from None.
    #[test]
    fn cold_missing_root_is_disconnected_and_creates_nothing() {
        let temporary = tempdir().expect("isolated local base");
        let root = code0_profile_root(Some(temporary.path().as_os_str())).unwrap();
        assert!(!root.exists());
        let slot = cold_code0_lease_slot();
        let mut store = Code0SlotSnapshotStore::new(&root, slot.clone());
        assert_eq!(store.load(), Ok(None));
        assert_eq!(
            store.save_atomic(&snapshot()),
            Err(Code0StoreError::Unavailable)
        );
        assert_eq!(store.remove(), Err(Code0StoreError::Unavailable));
        let session = Code0WebSession::new(
            ColdClock,
            NoNativeWork,
            store,
            Box::new(|_| panic!("cold load must not publish a new capture")),
        );
        assert_eq!(session.state(), Code0SessionState::Disconnected);
        assert!(session.snapshot().is_none());
        assert!(!session.is_automatically_syncing());
        assert!(slot.borrow().is_none());
        assert!(!root.exists());
        assert!(!root.join("profile.lock").exists());
    }

    // Catches cached data reauthorizing synchronization or mutation without a lease.
    #[test]
    fn cold_cache_stays_paused_without_a_lease_or_new_lock() {
        let temporary = tempdir().unwrap();
        let root = code0_profile_root(Some(temporary.path().as_os_str())).unwrap();
        let lease = Code0ProfileLease::acquire(&root).unwrap();
        Code0SnapshotStore::new(&root, Some(lease.clone()))
            .save_atomic(&snapshot())
            .unwrap();
        lease.release();
        let before = std::fs::read(root.join("profile.lock")).unwrap();
        let slot = cold_code0_lease_slot();
        let store = Code0SlotSnapshotStore::new(&root, slot.clone());
        assert_eq!(store.load().unwrap().unwrap().status, UsageStatus::Cached);
        let session = Code0WebSession::new(
            ColdClock,
            NoNativeWork,
            store,
            Box::new(|_| panic!("cold cache is not a new capture")),
        );
        assert_eq!(session.state(), Code0SessionState::Paused);
        assert_eq!(session.snapshot().unwrap().status, UsageStatus::Cached);
        assert!(!session.is_automatically_syncing());
        assert!(slot.borrow().is_none());
        assert_eq!(std::fs::read(root.join("profile.lock")).unwrap(), before);
    }

    // Catches a permanently cold delegate or an independently acquired authority.
    #[test]
    fn held_slot_authority_enables_real_save_load_and_remove() {
        let temporary = tempdir().unwrap();
        let root = code0_profile_root(Some(temporary.path().as_os_str())).unwrap();
        let slot = cold_code0_lease_slot();
        let mut store = Code0SlotSnapshotStore::new(&root, slot.clone());
        let lease = Code0ProfileLease::acquire(&root).unwrap();
        *slot.borrow_mut() = Some(lease.clone());
        store.save_atomic(&snapshot()).unwrap();
        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.primary_metric.unwrap().current, 123.5);
        assert_eq!(loaded.secondary_metric.unwrap().current, 456.75);
        store.remove().unwrap();
        assert_eq!(store.load(), Ok(None));
        assert!(lease.is_bound_to(&root));
        lease.release();
        let removed = slot.borrow_mut().take();
        drop(removed);
    }

    // Catches treating Some or a held lease for a different root as write authority.
    #[test]
    fn mismatched_slot_authority_cannot_read_or_mutate_another_root() {
        let temporary = tempdir().unwrap();
        let root = code0_profile_root(Some(temporary.path().as_os_str())).unwrap();
        let other = temporary.path().join("other-profile");
        let lease = Code0ProfileLease::acquire(&root).unwrap();
        let slot = Rc::new(RefCell::new(Some(lease.clone())));
        let mut store = Code0SlotSnapshotStore::new(&other, slot);
        assert_eq!(store.load(), Err(Code0StoreError::Unavailable));
        assert_eq!(
            store.save_atomic(&snapshot()),
            Err(Code0StoreError::Unavailable)
        );
        assert_eq!(store.remove(), Err(Code0StoreError::Unavailable));
        lease.release();
        assert!(!other.exists());
    }

    // Catches falling back to a cold read or clearing/replacing an invalid authority.
    #[test]
    fn invalidated_slot_retains_the_original_lock_until_actual_release() {
        let temporary = tempdir().unwrap();
        let root = code0_profile_root(Some(temporary.path().as_os_str())).unwrap();
        let slot = cold_code0_lease_slot();
        let mut store = Code0SlotSnapshotStore::new(&root, slot.clone());
        let lease = Code0ProfileLease::acquire(&root).unwrap();
        *slot.borrow_mut() = Some(lease.clone());
        store.save_atomic(&snapshot()).unwrap();
        lease.invalidate();
        assert_eq!(store.load(), Err(Code0StoreError::Unavailable));
        assert_eq!(
            store.save_atomic(&snapshot()),
            Err(Code0StoreError::Unavailable)
        );
        assert_eq!(store.remove(), Err(Code0StoreError::Unavailable));
        assert!(slot.borrow().is_some());
        assert!(matches!(
            Code0ProfileLease::acquire(&root),
            Err(Code0LeaseError::Busy)
        ));
        lease.release();
        let removed = slot.borrow_mut().take();
        drop(removed);
        let fresh = Code0ProfileLease::acquire(&root).unwrap();
        fresh.release();
    }

    // Catches capturing one old authority in the facade across release/reacquisition.
    #[test]
    fn same_store_refreshes_authority_after_release_and_never_reuses_old_clones() {
        let temporary = tempdir().unwrap();
        let root = code0_profile_root(Some(temporary.path().as_os_str())).unwrap();
        let slot = cold_code0_lease_slot();
        let mut store = Code0SlotSnapshotStore::new(&root, slot.clone());
        let old = Code0ProfileLease::acquire(&root).unwrap();
        *slot.borrow_mut() = Some(old.clone());
        store.save_atomic(&snapshot()).unwrap();
        old.release();
        assert_eq!(store.load(), Err(Code0StoreError::Unavailable));
        assert_eq!(
            store.save_atomic(&snapshot()),
            Err(Code0StoreError::Unavailable)
        );
        let removed = slot.borrow_mut().take();
        drop(removed);
        assert_eq!(store.load().unwrap().unwrap().status, UsageStatus::Cached);
        assert_eq!(store.remove(), Err(Code0StoreError::Unavailable));
        let fresh = Code0ProfileLease::acquire(&root).unwrap();
        *slot.borrow_mut() = Some(fresh.clone());
        assert!(!old.is_held());
        let mut old_store = Code0SnapshotStore::new(&root, Some(old));
        assert_eq!(old_store.load(), Err(Code0StoreError::Unavailable));
        assert_eq!(
            old_store.save_atomic(&snapshot()),
            Err(Code0StoreError::Unavailable)
        );
        assert_eq!(old_store.remove(), Err(Code0StoreError::Unavailable));
        assert!(store.load().unwrap().is_some());
        store.remove().unwrap();
        store.save_atomic(&snapshot()).unwrap();
        assert_eq!(
            store.load().unwrap().unwrap().fetched_at,
            "2026-10-29T04:05:06Z"
        );
        assert!(fresh.is_bound_to(&root));
        fresh.release();
        let removed = slot.borrow_mut().take();
        drop(removed);
    }

    // Catches treating an unsafe existing prefix as absent on a cold load.
    #[test]
    fn cold_reparse_prefix_is_unavailable_and_creates_no_profile() {
        let temporary = tempdir().unwrap();
        let actual = temporary.path().join("actual");
        let alias = temporary.path().join("alias");
        std::fs::create_dir(&actual).unwrap();
        std::os::windows::fs::symlink_dir(&actual, &alias)
            .expect("real reparse fixture required; unsupported setup is not a skip");
        let root = code0_profile_root(Some(alias.as_os_str())).unwrap();
        let store = Code0SlotSnapshotStore::new(&root, cold_code0_lease_slot());
        assert_eq!(store.load(), Err(Code0StoreError::Unavailable));
        assert!(!actual.join("AI Token Meter Code0").exists());
    }

    // Catches invalid native paths or malformed cache being converted into absence.
    #[test]
    fn cold_invalid_path_and_malformed_cache_remain_errors() {
        for root in [
            PathBuf::from("relative-profile"),
            PathBuf::from(r"C:profile"),
        ] {
            let store = Code0SlotSnapshotStore::new(root, cold_code0_lease_slot());
            assert_eq!(store.load(), Err(Code0StoreError::Unavailable));
        }
        let temporary = tempdir().unwrap();
        let root = code0_profile_root(Some(temporary.path().as_os_str())).unwrap();
        let lease = Code0ProfileLease::acquire(&root).unwrap();
        lease.release();
        std::fs::write(root.join("snapshot.json"), b"malformed").unwrap();
        let store = Code0SlotSnapshotStore::new(&root, cold_code0_lease_slot());
        assert!(store.load().is_err());
    }

    // Catches bypassing real security validation or falling back after revocation.
    #[test]
    fn native_security_failure_revokes_and_retains_the_shared_authority() {
        let temporary = tempdir().unwrap();
        let root = code0_profile_root(Some(temporary.path().as_os_str())).unwrap();
        let lease = Code0ProfileLease::acquire(&root).unwrap();
        let slot = Rc::new(RefCell::new(Some(lease.clone())));
        let mut store = Code0SlotSnapshotStore::new(&root, slot.clone());
        store.save_atomic(&snapshot()).unwrap();
        lease.fail_next_sid_lookup_for_test();
        assert_eq!(store.load(), Err(Code0StoreError::Unavailable));
        assert!(!lease.is_held());
        assert!(slot.borrow().is_some());
        assert_eq!(
            store.save_atomic(&snapshot()),
            Err(Code0StoreError::Unavailable)
        );
        assert_eq!(store.remove(), Err(Code0StoreError::Unavailable));
        lease.release();
        let removed = slot.borrow_mut().take();
        drop(removed);
        assert!(store.load().unwrap().is_some());
    }

    // Catches RefCell panics on an unexpectedly busy shared slot.
    #[test]
    fn borrowed_slot_fails_unavailable_without_native_work() {
        let temporary = tempdir().unwrap();
        let root = code0_profile_root(Some(temporary.path().as_os_str())).unwrap();
        let slot = cold_code0_lease_slot();
        let mut store = Code0SlotSnapshotStore::new(&root, slot.clone());
        let _borrow = slot.borrow_mut();
        assert_eq!(store.load(), Err(Code0StoreError::Unavailable));
        assert_eq!(
            store.save_atomic(&snapshot()),
            Err(Code0StoreError::Unavailable)
        );
        assert_eq!(store.remove(), Err(Code0StoreError::Unavailable));
        assert!(!root.exists());
    }
}
