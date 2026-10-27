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

// These three fixture entry points accept an owned TempDir, never an arbitrary
// source path. Callers acquired their still-held lease at this exact Code0 root.
mod native_snapshot_fixtures {
    use std::ffi::OsStr;
    use std::fs;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use std::path::{Component, Path};
    use std::ptr::null_mut;

    use super::Code0ProfileLease;
    use tempfile::TempDir;
    use windows_sys::Wdk::Storage::FileSystem::{
        FILE_LINK_INFORMATION, FILE_RENAME_INFORMATION, FileLinkInformation, FileRenameInformation,
        NtSetInformationFile,
    };
    use windows_sys::Win32::Foundation::{
        ERROR_FILE_NOT_FOUND, GetLastError, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, DELETE, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_ATTRIBUTE_TAG_INFO, FILE_FLAG_OPEN_REPARSE_POINT, FILE_ID_INFO, FILE_READ_ATTRIBUTES,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_STANDARD_INFO, FileAttributeTagInfo, FileIdInfo,
        FileStandardInfo, GetFileInformationByHandleEx, OPEN_EXISTING, READ_CONTROL,
    };
    use windows_sys::Win32::System::IO::{IO_STATUS_BLOCK, IO_STATUS_BLOCK_0};

    pub(super) enum FixtureError {
        Predicate,
        Win32(u32),
        Native { status: i32, io_status: Option<i32> },
    }

    impl std::fmt::Debug for FixtureError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Predicate => formatter.write_str("FixturePredicateRejected"),
                Self::Win32(code) => formatter
                    .debug_tuple("FixtureWin32Error")
                    .field(code)
                    .finish(),
                Self::Native { status, io_status } => formatter
                    .debug_struct("FixtureNativeError")
                    .field("status", status)
                    .field("io_status", io_status)
                    .finish(),
            }
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    struct Identity {
        volume: u64,
        id: [u8; 16],
    }

    #[derive(Clone, Copy)]
    enum Change {
        HardLink,
        Rename,
    }

    pub(super) enum Direction {
        Forward,
        Restore,
    }

    fn require(value: bool) -> Result<(), FixtureError> {
        value.then_some(()).ok_or(FixtureError::Predicate)
    }

    fn absent(path: &Path) -> Result<(), FixtureError> {
        match fs::symlink_metadata(path) {
            Err(error) if error.raw_os_error() == Some(ERROR_FILE_NOT_FOUND as i32) => Ok(()),
            _ => Err(FixtureError::Predicate),
        }
    }

    fn open(path: &Path, access: u32) -> Result<OwnedHandle, FixtureError> {
        let name = path.as_os_str().encode_wide().collect::<Vec<_>>();
        require(!name.contains(&0))?;
        let name = name.into_iter().chain(Some(0)).collect::<Vec<_>>();
        let handle = unsafe {
            CreateFileW(
                name.as_ptr(),
                access,
                FILE_SHARE_READ | FILE_SHARE_DELETE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_OPEN_REPARSE_POINT,
                null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            return Err(FixtureError::Win32(unsafe { GetLastError() }));
        }
        Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
    }

    fn identity(handle: HANDLE, expected_links: u32) -> Result<Identity, FixtureError> {
        let mut tag = FILE_ATTRIBUTE_TAG_INFO::default();
        if unsafe {
            GetFileInformationByHandleEx(
                handle,
                FileAttributeTagInfo,
                (&mut tag as *mut FILE_ATTRIBUTE_TAG_INFO).cast(),
                std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
            )
        } == 0
        {
            return Err(FixtureError::Win32(unsafe { GetLastError() }));
        }
        require(
            tag.FileAttributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) == 0,
        )?;
        let mut standard = FILE_STANDARD_INFO::default();
        if unsafe {
            GetFileInformationByHandleEx(
                handle,
                FileStandardInfo,
                (&mut standard as *mut FILE_STANDARD_INFO).cast(),
                std::mem::size_of::<FILE_STANDARD_INFO>() as u32,
            )
        } == 0
        {
            return Err(FixtureError::Win32(unsafe { GetLastError() }));
        }
        require(
            !standard.Directory
                && !standard.DeletePending
                && standard.NumberOfLinks == expected_links,
        )?;
        let mut id = FILE_ID_INFO::default();
        if unsafe {
            GetFileInformationByHandleEx(
                handle,
                FileIdInfo,
                (&mut id as *mut FILE_ID_INFO).cast(),
                std::mem::size_of::<FILE_ID_INFO>() as u32,
            )
        } == 0
        {
            return Err(FixtureError::Win32(unsafe { GetLastError() }));
        }
        require(id.VolumeSerialNumber != 0 && id.FileId.Identifier.iter().any(|byte| *byte != 0))?;
        Ok(Identity {
            volume: id.VolumeSerialNumber,
            id: id.FileId.Identifier,
        })
    }

    fn submit(handle: HANDLE, leaf: &'static str, change: Change) -> Result<(), FixtureError> {
        require(
            !leaf.is_empty() && !leaf.contains(['\0', '/', '\\']) && !matches!(leaf, "." | ".."),
        )?;
        let name = leaf.encode_utf16().collect::<Vec<_>>();
        let name_bytes = name
            .len()
            .checked_mul(std::mem::size_of::<u16>())
            .ok_or(FixtureError::Predicate)?;
        let (size, offset, alignment, class) = match change {
            Change::HardLink => (
                std::mem::size_of::<FILE_LINK_INFORMATION>(),
                std::mem::offset_of!(FILE_LINK_INFORMATION, FileName),
                std::mem::align_of::<FILE_LINK_INFORMATION>(),
                FileLinkInformation,
            ),
            Change::Rename => (
                std::mem::size_of::<FILE_RENAME_INFORMATION>(),
                std::mem::offset_of!(FILE_RENAME_INFORMATION, FileName),
                std::mem::align_of::<FILE_RENAME_INFORMATION>(),
                FileRenameInformation,
            ),
        };
        // New fixture buffers deliberately satisfy the stricter DDI sizeof +
        // name formulation. The product's accepted counted buffer is unchanged.
        let length = size
            .checked_add(name_bytes)
            .ok_or(FixtureError::Predicate)?;
        require(std::mem::align_of::<u128>() >= alignment.max(std::mem::align_of::<i32>()))?;
        require(
            offset
                .checked_add(name_bytes)
                .is_some_and(|end| end <= length),
        )?;
        let submitted = u32::try_from(length).map_err(|_| FixtureError::Predicate)?;
        let name_bytes = u32::try_from(name_bytes).map_err(|_| FixtureError::Predicate)?;
        let mut buffer = vec![0u128; length.div_ceil(std::mem::size_of::<u128>())];
        unsafe {
            match change {
                Change::HardLink => {
                    let info = buffer.as_mut_ptr().cast::<FILE_LINK_INFORMATION>();
                    (*info).Anonymous.ReplaceIfExists = false;
                    (*info).RootDirectory = null_mut();
                    (*info).FileNameLength = name_bytes;
                    std::ptr::copy_nonoverlapping(
                        name.as_ptr(),
                        std::ptr::addr_of_mut!((*info).FileName).cast::<u16>(),
                        name.len(),
                    );
                }
                Change::Rename => {
                    let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFORMATION>();
                    (*info).Anonymous.ReplaceIfExists = false;
                    (*info).RootDirectory = null_mut();
                    (*info).FileNameLength = name_bytes;
                    std::ptr::copy_nonoverlapping(
                        name.as_ptr(),
                        std::ptr::addr_of_mut!((*info).FileName).cast::<u16>(),
                        name.len(),
                    );
                }
            }
        }
        let mut io_status = IO_STATUS_BLOCK {
            Anonymous: IO_STATUS_BLOCK_0 { Status: i32::MIN },
            Information: 0,
        };
        // Nt/ZwSetInformationFile is a synchronous service. Source, IOSB,
        // stationary aligned buffer and caller's original lease all outlive it.
        let status = unsafe {
            NtSetInformationFile(
                handle,
                &mut io_status,
                buffer.as_ptr().cast(),
                submitted,
                class,
            )
        };
        let completion = unsafe { io_status.Anonymous.Status };
        if status != 0 || completion != 0 {
            return Err(FixtureError::Native {
                status,
                io_status: (completion != i32::MIN).then_some(completion),
            });
        }
        Ok(())
    }

    fn change_owned_link(
        temporary: &TempDir,
        lease: &Code0ProfileLease,
        source_leaf: &'static str,
        target_leaf: &'static str,
        change: Change,
    ) -> Result<(), FixtureError> {
        require(matches!(
            (change, source_leaf, target_leaf),
            (Change::HardLink, "snapshot.json", "unexpected-link")
                | (Change::Rename, "snapshot.json", "unrelated-target")
                | (Change::Rename, "unrelated-target", "snapshot.json")
                | (Change::Rename, "snapshot.json", "snapshot-candidate-7.tmp")
                | (Change::Rename, "snapshot-candidate-7.tmp", "snapshot.json")
        ))?;
        let root = temporary.path().join("Code0");
        require(
            root.is_absolute()
                && root.file_name() == Some(OsStr::new("Code0"))
                && !root
                    .components()
                    .any(|part| matches!(part, Component::CurDir | Component::ParentDir)),
        )?;
        require(lease.is_held())?;
        let source_path = root.join(source_leaf);
        let target_path = root.join(target_leaf);
        absent(&target_path)?;
        let access = FILE_READ_ATTRIBUTES
            | READ_CONTROL
            | match change {
                Change::HardLink => 0,
                Change::Rename => DELETE,
            };
        let source = open(&source_path, access)?;
        let before = identity(source.as_raw_handle(), 1)?;
        submit(source.as_raw_handle(), target_leaf, change)?;
        let links = match change {
            Change::HardLink => 2,
            Change::Rename => 1,
        };
        require(identity(source.as_raw_handle(), links)? == before)?;
        let target = open(&target_path, FILE_READ_ATTRIBUTES | READ_CONTROL)?;
        require(identity(target.as_raw_handle(), links)? == before)?;
        match change {
            Change::HardLink => {
                let original = open(&source_path, FILE_READ_ATTRIBUTES | READ_CONTROL)?;
                require(identity(original.as_raw_handle(), 2)? == before)?;
                drop(original);
            }
            Change::Rename => absent(&source_path)?,
        }
        drop(target);
        drop(source);
        require(lease.is_held())
    }

    pub(super) fn hard_link_snapshot(
        temporary: &TempDir,
        lease: &Code0ProfileLease,
    ) -> Result<(), FixtureError> {
        change_owned_link(
            temporary,
            lease,
            "snapshot.json",
            "unexpected-link",
            Change::HardLink,
        )
    }

    pub(super) fn move_for_unsafe_link(
        temporary: &TempDir,
        lease: &Code0ProfileLease,
        direction: Direction,
    ) -> Result<(), FixtureError> {
        let (source, target) = match direction {
            Direction::Forward => ("snapshot.json", "unrelated-target"),
            Direction::Restore => ("unrelated-target", "snapshot.json"),
        };
        change_owned_link(temporary, lease, source, target, Change::Rename)
    }

    pub(super) fn move_for_unknown_slot(
        temporary: &TempDir,
        lease: &Code0ProfileLease,
        direction: Direction,
    ) -> Result<(), FixtureError> {
        let (source, target) = match direction {
            Direction::Forward => ("snapshot.json", "snapshot-candidate-7.tmp"),
            Direction::Restore => ("snapshot-candidate-7.tmp", "snapshot.json"),
        };
        change_owned_link(temporary, lease, source, target, Change::Rename)
    }
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

// Encode before writing so libtest cannot discard successful evidence and an
// oversized summary cannot produce a plausible truncated record.
fn write_rollback_diagnostic(
    output: &mut impl std::io::Write,
    arguments: std::fmt::Arguments<'_>,
) -> std::io::Result<()> {
    struct Line {
        bytes: [u8; 8191],
        used: usize,
    }
    impl std::fmt::Write for Line {
        fn write_str(&mut self, value: &str) -> std::fmt::Result {
            let end = self.used.checked_add(value.len()).ok_or(std::fmt::Error)?;
            let target = self.bytes.get_mut(self.used..end).ok_or(std::fmt::Error)?;
            target.copy_from_slice(value.as_bytes());
            self.used = end;
            Ok(())
        }
    }
    let mut line = Line {
        bytes: [0; 8191],
        used: 0,
    };
    if std::fmt::write(&mut line, arguments).is_err() {
        output.write_all(b"Code0StorageRollbackEvidence incomplete=EncodingOverflow\n")?;
        output.flush()?;
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    output.write_all(&line.bytes[..line.used])?;
    output.flush()
}

fn emit_rollback_diagnostic(arguments: std::fmt::Arguments<'_>) -> std::io::Result<()> {
    write_rollback_diagnostic(&mut std::io::stderr().lock(), arguments)
}

fn save_result_category(result: &Result<(), Code0StoreError>) -> &'static str {
    match result {
        Ok(()) => "Ok",
        Err(Code0StoreError::Unavailable) => "Unavailable",
        Err(Code0StoreError::InvalidSnapshot) => "InvalidSnapshot",
    }
}

fn load_result_category(result: &Result<Option<UsageSnapshot>, Code0StoreError>) -> &'static str {
    match result {
        Ok(Some(_)) => "Some",
        Ok(None) => "None",
        Err(Code0StoreError::Unavailable) => "Unavailable",
        Err(Code0StoreError::InvalidSnapshot) => "InvalidSnapshot",
    }
}

#[test]
fn rollback_diagnostic_output_is_bounded_and_does_not_hide_write_or_flush_failure() {
    struct FailingOutput {
        allow: usize,
        flush_fails: bool,
    }
    impl std::io::Write for FailingOutput {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.allow == 0 {
                return Err(std::io::ErrorKind::BrokenPipe.into());
            }
            let count = self.allow.min(bytes.len());
            self.allow -= count;
            Ok(count)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            if self.flush_fails {
                Err(std::io::ErrorKind::BrokenPipe.into())
            } else {
                Ok(())
            }
        }
    }
    for (allow, flush_fails) in [(0, false), (3, false), (usize::MAX, true)] {
        assert!(
            write_rollback_diagnostic(
                &mut FailingOutput { allow, flush_fails },
                format_args!("fixed evidence\n"),
            )
            .is_err()
        );
    }
    let mut complete = Vec::new();
    write_rollback_diagnostic(&mut complete, format_args!("fixed evidence\n")).unwrap();
    assert_eq!(complete, b"fixed evidence\n");
    let oversized = "x".repeat(8192);
    let mut incomplete = Vec::new();
    assert!(write_rollback_diagnostic(&mut incomplete, format_args!("{oversized}")).is_err());
    assert_eq!(
        incomplete,
        b"Code0StorageRollbackEvidence incomplete=EncodingOverflow\n"
    );
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
    let diagnostic_observer = Code0SnapshotTestGate::new();
    let mut store = Code0SnapshotStore::new(&root, Some(lease))
        .with_diagnostic_observer(diagnostic_observer.clone());
    let snapshot = expected_snapshot();

    let save_result = store.save_atomic(&snapshot);
    eprintln!(
        "Code0StorageRoundTrip {:?}",
        diagnostic_observer.diagnostic_summary()
    );
    save_result.expect("persist before publishing snapshot");
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
fn load_rejects_non_exact_or_oversized_projection_fields() {
    let temporary = tempdir().expect("isolated temporary directory");
    let root = temporary.path().join("Code0");
    let lease = Code0ProfileLease::acquire(&root).expect("acquire profile lease");
    let mut store = Code0SnapshotStore::new(&root, Some(lease));
    store
        .save_atomic(&expected_snapshot())
        .expect("create owned projection path");
    let snapshot_path = root.join("snapshot.json");
    let valid = fs::read(&snapshot_path).expect("valid owned bytes");
    assert!(store.load().unwrap().is_some());
    assert!(
        Code0SnapshotStore::new(&root, None)
            .load()
            .unwrap()
            .is_some()
    );
    let invalid_projections = [
        br#"{"schemaVersion":1,"balance":1,"cumulativeUsage":2,"fetchedAt":"2026-10-27T04:05:06Z","sourceVersion":"website","extra":true}"#.to_vec(),
        br#"{"schemaVersion":1,"balance":1,"balance":2,"cumulativeUsage":2,"fetchedAt":"2026-10-27T04:05:06Z","sourceVersion":"website"}"#.to_vec(),
        br#"{"schemaVersion":1,"balance":true,"cumulativeUsage":2,"fetchedAt":"2026-10-27T04:05:06Z","sourceVersion":"website"}"#.to_vec(),
        br#"{"schemaVersion":1,"balance":-1,"cumulativeUsage":2,"fetchedAt":"2026-10-27T04:05:06Z","sourceVersion":"website"}"#.to_vec(),
        br#"{"schemaVersion":1,"balance":1,"cumulativeUsage":2,"fetchedAt":"tomorrow","sourceVersion":"website"}"#.to_vec(),
    ];
    for bytes in invalid_projections {
        fs::write(&snapshot_path, bytes).expect("write synthetic malformed projection");
        assert_eq!(
            store.load(),
            Err(Code0StoreError::Unavailable),
            "malformed projection must not be treated as absent or accepted"
        );
        assert_eq!(
            Code0SnapshotStore::new(&root, None).load(),
            Err(Code0StoreError::Unavailable)
        );
        fs::write(&snapshot_path, &valid).expect("restore valid owned bytes");
        assert!(store.load().unwrap().is_some());
        assert!(
            Code0SnapshotStore::new(&root, None)
                .load()
                .unwrap()
                .is_some()
        );
    }

    fs::write(&snapshot_path, vec![b'x'; 4097]).expect("write oversized projection");
    assert_eq!(
        store.load(),
        Err(Code0StoreError::Unavailable),
        "projection larger than the fixed bound is rejected"
    );
    assert_eq!(
        Code0SnapshotStore::new(&root, None).load(),
        Err(Code0StoreError::Unavailable)
    );
    fs::write(&snapshot_path, &valid).expect("restore valid owned bytes");
    assert!(store.load().unwrap().is_some());
    assert!(
        Code0SnapshotStore::new(&root, None)
            .load()
            .unwrap()
            .is_some()
    );
}

#[test]
fn load_rejects_a_hard_linked_snapshot_file() {
    let temporary = tempdir().expect("isolated temporary directory");
    let root = temporary.path().join("Code0");
    let lease = Code0ProfileLease::acquire(&root).expect("acquire profile lease");
    let mut store = Code0SnapshotStore::new(&root, Some(lease.clone()));
    store
        .save_atomic(&expected_snapshot())
        .expect("save projection before linking");
    assert!(store.load().unwrap().is_some());
    assert!(
        Code0SnapshotStore::new(&root, None)
            .load()
            .unwrap()
            .is_some()
    );
    native_snapshot_fixtures::hard_link_snapshot(&temporary, &lease)
        .expect("create isolated hard-link fixture");

    assert_eq!(
        store.load(),
        Err(Code0StoreError::Unavailable),
        "multi-link snapshot is not an owned projection"
    );
    assert_eq!(
        Code0SnapshotStore::new(&root, None).load(),
        Err(Code0StoreError::Unavailable)
    );
    fs::remove_file(root.join("unexpected-link")).expect("remove fixture extra link");
    assert!(store.load().unwrap().is_some());
    assert!(
        Code0SnapshotStore::new(&root, None)
            .load()
            .unwrap()
            .is_some()
    );
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
    let worker_observer = gate.clone();
    let worker = std::thread::spawn(move || {
        let mut failing = Code0SnapshotStore::with_failure_gate(
            worker_root,
            Some(lease),
            [Code0SnapshotFailurePoint::AfterCandidateReplace],
            worker_gate,
        )
        .with_diagnostic_observer(worker_observer);
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
    let diagnostic_output = emit_rollback_diagnostic(format_args!(
        "Code0StorageRollbackEvidence case=PostReplaceOldA iteration=0 stage=Joined save={} beforeClose=NotApplicable afterClose=NotApplicable {:?}\n",
        save_result_category(&save_result),
        gate.diagnostic_summary(),
    ));

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
    diagnostic_output.expect("complete bounded direct-stderr rollback evidence");
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
    let diagnostic_observer = Code0SnapshotTestGate::new();
    let worker_observer = diagnostic_observer.clone();
    let worker = std::thread::spawn(move || {
        let mut failing = Code0SnapshotStore::with_failure_gate(
            worker_root,
            Some(lease),
            [Code0SnapshotFailurePoint::AfterCandidateReplace],
            worker_gate,
        )
        .with_diagnostic_observer(worker_observer);
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
    eprintln!(
        "Code0StorageAbsent {:?}",
        diagnostic_observer.diagnostic_summary()
    );

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

#[test]
fn second_save_and_cold_read_after_release_accept_the_actual_committed_candidate() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Code0");
    let lease = Code0ProfileLease::acquire(&root).unwrap();
    let mut store = Code0SnapshotStore::new(&root, Some(lease.clone()));
    store.save_atomic(&expected_snapshot()).expect("first save");
    assert_eq!(store.load().unwrap().unwrap().fetched_at, FETCHED_AT);
    store
        .save_atomic(&candidate_snapshot())
        .expect("second replacement save");
    assert_eq!(
        store
            .load()
            .unwrap()
            .unwrap()
            .primary_metric
            .unwrap()
            .current,
        124.0
    );
    lease.release();
    assert_eq!(
        Code0SnapshotStore::new(&root, None)
            .load()
            .unwrap()
            .unwrap()
            .primary_metric
            .unwrap()
            .current,
        124.0
    );
}

#[test]
fn unsafe_reparse_ancestor_never_becomes_proven_absence() {
    let temporary = tempdir().unwrap();
    let actual = temporary.path().join("actual");
    let alias = temporary.path().join("alias");
    fs::create_dir(&actual).unwrap();
    std::os::windows::fs::symlink_dir(&actual, &alias)
        .expect("real reparse fixture creation is required; failure is uncovered, not a skip");
    let root = alias.join("Code0");
    assert_eq!(
        Code0SnapshotStore::new(&root, None).load(),
        Err(Code0StoreError::Unavailable)
    );
    assert!(
        !actual.join("Code0").exists(),
        "unsafe cold read creates no directory"
    );
    let lease = Code0ProfileLease::acquire(actual.join("Code0")).unwrap();
    assert_eq!(
        Code0SnapshotStore::new(&root, None).load(),
        Err(Code0StoreError::Unavailable)
    );
    let mut store = Code0SnapshotStore::new(actual.join("Code0"), Some(lease.clone()));
    store.save_atomic(&expected_snapshot()).unwrap();
    assert_eq!(
        Code0SnapshotStore::new(&root, None).load(),
        Err(Code0StoreError::Unavailable)
    );
    lease.release();
}

#[test]
fn malformed_owned_cache_and_pending_can_be_explicitly_removed_without_touching_sentinels() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Code0");
    let lease = Code0ProfileLease::acquire(&root).unwrap();
    let mut store = Code0SnapshotStore::new(&root, Some(lease.clone()));
    let sentinel = root.join("unrelated.txt");
    let cache = temporary.path().join("deepseek.json");
    let settings = temporary.path().join("settings.json");
    for path in [&sentinel, &cache, &settings] {
        fs::write(path, b"untouched").unwrap();
    }
    for malformed in [b"{".to_vec(), Vec::new(), vec![b'x'; 4097]] {
        store.save_atomic(&expected_snapshot()).unwrap();
        fs::write(root.join("snapshot.json"), malformed).unwrap();
        assert_eq!(
            Code0SnapshotStore::new(&root, None).load(),
            Err(Code0StoreError::Unavailable)
        );
        store
            .remove()
            .expect("secure fixed cache path can be removed without parsing JSON");
        assert!(!root.join("snapshot.json").exists());
    }
    store.save_atomic(&expected_snapshot()).unwrap();
    let mut failed = Code0SnapshotStore::with_failure_points(
        &root,
        Some(lease.clone()),
        [
            Code0SnapshotFailurePoint::AfterCandidateReplace,
            Code0SnapshotFailurePoint::RestorePreviousSnapshot,
        ],
    );
    assert_eq!(
        failed.save_atomic(&candidate_snapshot()),
        Err(Code0StoreError::Unavailable)
    );
    fs::write(root.join("snapshot.pending"), b"broken marker").unwrap();
    assert_eq!(
        Code0SnapshotStore::new(&root, None).load(),
        Err(Code0StoreError::Unavailable)
    );
    store
        .remove()
        .expect("secure malformed marker is disposable at explicit cleanup");
    for name in ["snapshot.json", "snapshot.pending", "snapshot.retired"] {
        assert!(!root.join(name).exists());
    }
    for path in [&sentinel, &cache, &settings] {
        assert_eq!(fs::read(path).unwrap(), b"untouched");
    }
    assert!(lease.is_held());
    store
        .save_atomic(&expected_snapshot())
        .expect("explicit Connect can proceed after cleanup");
}

fn open_compatible_marker_reader(path: &std::path::Path) -> std::fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };
    fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .open(path)
        .expect("retain compatible ordinary marker reader")
}

// Regression oracle: a reader retained beyond save's return must not make a
// returned Err later expose B when its old delete-pending marker disappears.
#[test]
fn marker_reader_retained_past_commit_or_error_never_reveals_a_failed_candidate() {
    for (iteration, failures) in [
        vec![],
        vec![Code0SnapshotFailurePoint::BeforePendingRetirement],
        vec![
            Code0SnapshotFailurePoint::BeforePendingRetirement,
            Code0SnapshotFailurePoint::RestorePreviousSnapshot,
        ],
    ]
    .into_iter()
    .enumerate()
    {
        let iteration = match iteration {
            0 => "Nominal",
            1 => "BeforeRetire",
            _ => "BeforeRetireAndRestoreFailure",
        };
        let expects_success = failures.is_empty();
        let expects_barrier =
            failures.contains(&Code0SnapshotFailurePoint::RestorePreviousSnapshot);
        let temporary = tempdir().unwrap();
        let root = temporary.path().join("Code0");
        let lease = Code0ProfileLease::acquire(&root).unwrap();
        let mut initial = Code0SnapshotStore::new(&root, Some(lease.clone()));
        initial.save_atomic(&expected_snapshot()).unwrap();
        let gate = Code0SnapshotTestGate::new();
        let worker_gate = gate.clone();
        let worker_root = root.clone();
        let worker_lease = lease.clone();
        let worker_observer = gate.clone();
        let worker = std::thread::spawn(move || {
            Code0SnapshotStore::with_failure_gate(
                worker_root,
                Some(worker_lease),
                failures,
                worker_gate,
            )
            .with_diagnostic_observer(worker_observer)
            .save_atomic(&candidate_snapshot())
        });
        let reached = gate.wait_until_after_replace(Duration::from_secs(5));
        let observation = reached.then(|| observe_blocked_candidate(&root));
        let reader = reached.then(|| open_compatible_marker_reader(&root.join("snapshot.pending")));
        gate.release();
        let result = worker.join().expect("bounded worker exit");
        let joined_output = emit_rollback_diagnostic(format_args!(
            "Code0StorageRollbackEvidence case=RetainedReader iteration={} stage=Joined save={} beforeClose=NotObserved afterClose=NotObserved {:?}\n",
            iteration,
            save_result_category(&result),
            gate.diagnostic_summary(),
        ));
        if expects_success {
            assert_eq!(
                result,
                Ok(()),
                "nominal retirement must report a committed save"
            );
        } else {
            assert_eq!(
                result,
                Err(Code0StoreError::Unavailable),
                "each configured failure must deny publication"
            );
        }
        assert!(reached);
        observation
            .unwrap()
            .expect("actual B blocked before commit");
        // Hold reader through the returned result and authority release.
        lease.release();
        let before_close = Code0SnapshotStore::new(&root, None).load();
        drop(reader);
        let after_close = Code0SnapshotStore::new(&root, None).load();
        let loaded_output = emit_rollback_diagnostic(format_args!(
            "Code0StorageRollbackEvidence case=RetainedReader iteration={} stage=Loaded save={} beforeClose={} afterClose={}\n",
            iteration,
            save_result_category(&result),
            load_result_category(&before_close),
            load_result_category(&after_close),
        ));
        for loaded in [before_close, after_close] {
            if expects_success {
                assert_eq!(
                    loaded.unwrap().unwrap().primary_metric.unwrap().current,
                    124.0
                );
            } else if expects_barrier {
                assert_eq!(loaded, Err(Code0StoreError::Unavailable));
            } else {
                let snapshot = loaded
                    .expect("retirement failure restores A")
                    .expect("A remains present");
                assert_eq!(snapshot.fetched_at, FETCHED_AT);
                assert_eq!(snapshot.primary_metric.unwrap().current, 123.5);
            }
        }
        joined_output.expect("complete bounded direct-stderr join evidence");
        loaded_output.expect("complete bounded direct-stderr load evidence");
    }
}

#[test]
fn candidate_and_restore_write_flush_failures_leave_no_owned_numeric_residue_after_cleanup() {
    for point in [
        Code0SnapshotFailurePoint::CandidateWrite,
        Code0SnapshotFailurePoint::CandidateFlush,
        Code0SnapshotFailurePoint::RestoreWrite,
        Code0SnapshotFailurePoint::RestoreFlush,
    ] {
        let temporary = tempdir().unwrap();
        let root = temporary.path().join("Code0");
        let lease = Code0ProfileLease::acquire(&root).unwrap();
        let mut initial = Code0SnapshotStore::new(&root, Some(lease.clone()));
        initial.save_atomic(&expected_snapshot()).unwrap();
        fs::write(root.join("unrelated.tmp"), b"unrelated").unwrap();
        let failures = if matches!(
            point,
            Code0SnapshotFailurePoint::RestoreWrite | Code0SnapshotFailurePoint::RestoreFlush
        ) {
            vec![Code0SnapshotFailurePoint::AfterCandidateReplace, point]
        } else {
            vec![point]
        };
        let mut failed =
            Code0SnapshotStore::with_failure_points(&root, Some(lease.clone()), failures);
        assert_eq!(
            failed.save_atomic(&candidate_snapshot()),
            Err(Code0StoreError::Unavailable)
        );
        let numeric_residue_before_cleanup = fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("snapshot-candidate-")
            })
            .count();
        assert_eq!(
            numeric_residue_before_cleanup, 0,
            "exact-handle failure cleanup removes newly created candidate"
        );
        initial
            .remove()
            .expect("explicit cleanup completes after failed write/flush");
        for slot in 0..16 {
            assert!(!root.join(format!("snapshot-candidate-{slot}.tmp")).exists());
        }
        assert_eq!(fs::read(root.join("unrelated.tmp")).unwrap(), b"unrelated");
        assert!(lease.is_held());
    }
}

#[test]
fn interrupted_owned_candidate_cleanup_is_bounded_and_unremovable_candidate_retains_authority() {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Code0");
    let lease = Code0ProfileLease::acquire(&root).unwrap();
    let mut store = Code0SnapshotStore::new(&root, Some(lease.clone()));
    store.save_atomic(&expected_snapshot()).unwrap();
    let residue = root.join("snapshot-candidate-0.tmp");
    // Leave a real partially written candidate and its durable identity receipt.
    let mut failed = Code0SnapshotStore::with_failure_points(
        &root,
        Some(lease.clone()),
        [
            Code0SnapshotFailurePoint::CandidateWrite,
            Code0SnapshotFailurePoint::CandidateCleanup,
        ],
    );
    assert_eq!(
        failed.save_atomic(&candidate_snapshot()),
        Err(Code0StoreError::Unavailable)
    );
    assert!(residue.exists());
    fs::write(
        root.join("snapshot-unrelated.tmp"),
        b"leave this unrelated path",
    )
    .unwrap();
    let retained = fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(&residue)
        .unwrap();
    assert_eq!(store.remove(), Err(Code0StoreError::Unavailable));
    assert!(lease.is_held());
    assert!(matches!(
        Code0ProfileLease::acquire(&root),
        Err(ai_token_meter_windows::platform::windows::code0_profile_lease::Code0LeaseError::Busy)
    ));
    assert!(residue.exists());
    drop(retained);
    store
        .remove()
        .expect("exact owned residue removable after confirmed reader release");
    assert!(!residue.exists());
    assert_eq!(
        fs::read(root.join("snapshot-unrelated.tmp")).unwrap(),
        b"leave this unrelated path"
    );
}

#[test]
fn unsafe_snapshot_link_is_rejected_by_load_and_explicit_cleanup() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Code0");
    let lease = Code0ProfileLease::acquire(&root).unwrap();
    let mut store = Code0SnapshotStore::new(&root, Some(lease.clone()));
    store.save_atomic(&expected_snapshot()).unwrap();
    let target = root.join("unrelated-target");
    native_snapshot_fixtures::move_for_unsafe_link(
        &temporary,
        &lease,
        native_snapshot_fixtures::Direction::Forward,
    )
    .expect("create exact same-parent unsafe-link fixture");
    let target_bytes = fs::read(&target).unwrap();
    std::os::windows::fs::symlink_file(&target, root.join("snapshot.json"))
        .expect("real file-reparse fixture creation is required; no silent skip");
    assert_eq!(store.load(), Err(Code0StoreError::Unavailable));
    assert_eq!(store.remove(), Err(Code0StoreError::Unavailable));
    assert_eq!(fs::read(&target).unwrap(), target_bytes);
    assert!(lease.is_held());
    fs::remove_file(root.join("snapshot.json")).unwrap();
    native_snapshot_fixtures::move_for_unsafe_link(
        &temporary,
        &lease,
        native_snapshot_fixtures::Direction::Restore,
    )
    .expect("restore exact same-parent snapshot identity");
    assert!(store.load().unwrap().is_some());
    store.remove().unwrap();
}

#[test]
fn a_secure_unknown_file_in_a_reserved_candidate_slot_is_preserved_and_blocks_cleanup() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Code0");
    let lease = Code0ProfileLease::acquire(&root).unwrap();
    let mut store = Code0SnapshotStore::new(&root, Some(lease.clone()));
    store.save_atomic(&expected_snapshot()).unwrap();
    let unknown = root.join("snapshot-candidate-7.tmp");
    // Exact restricted DACL and reserved name are insufficient without this
    // candidate's volume/file-ID receipt in an owned pending/retired marker.
    native_snapshot_fixtures::move_for_unknown_slot(
        &temporary,
        &lease,
        native_snapshot_fixtures::Direction::Forward,
    )
    .expect("create exact unknown same-parent reserved candidate");
    let before = fs::read(&unknown).unwrap();
    assert_eq!(store.remove(), Err(Code0StoreError::Unavailable));
    assert_eq!(fs::read(&unknown).unwrap(), before);
    assert_eq!(
        store.save_atomic(&candidate_snapshot()),
        Err(Code0StoreError::Unavailable)
    );
    assert_eq!(fs::read(&unknown).unwrap(), before);
    assert!(lease.is_held());
    native_snapshot_fixtures::move_for_unknown_slot(
        &temporary,
        &lease,
        native_snapshot_fixtures::Direction::Restore,
    )
    .expect("restore exact unknown-candidate source identity");
    store.remove().unwrap();
}

#[test]
fn invalidated_authority_denies_cache_work_while_retaining_owned_exclusion() {
    for sid_failure in [false, true] {
        let temporary = tempdir().unwrap();
        let root = temporary.path().join("Code0");
        let lease = Code0ProfileLease::acquire(&root).unwrap();
        let mut store = Code0SnapshotStore::new(&root, Some(lease.clone()));
        store.save_atomic(&expected_snapshot()).unwrap();
        if sid_failure {
            lease.fail_next_sid_lookup_for_test();
            assert!(!lease.is_held());
        } else {
            lease.invalidate();
        }
        assert_eq!(store.load(), Err(Code0StoreError::Unavailable));
        assert_eq!(
            store.save_atomic(&candidate_snapshot()),
            Err(Code0StoreError::Unavailable)
        );
        assert_eq!(store.remove(), Err(Code0StoreError::Unavailable));
        assert!(matches!(Code0ProfileLease::acquire(&root), Err(ai_token_meter_windows::platform::windows::code0_profile_lease::Code0LeaseError::Busy)));
        lease.release();
        assert_eq!(
            Code0SnapshotStore::new(&root, None)
                .load()
                .unwrap()
                .unwrap()
                .fetched_at,
            FETCHED_AT
        );
    }
}

fn directory_access_error(path: &std::path::Path, access: u32) -> Option<u32> {
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::{FromRawHandle, OwnedHandle};
    use windows_sys::Win32::Foundation::{GetLastError, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    let wide = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        Some(unsafe { GetLastError() })
    } else {
        drop(unsafe { OwnedHandle::from_raw_handle(handle) });
        None
    }
}

fn assert_directory_mutation_access_is_available(path: &std::path::Path) {
    use windows_sys::Win32::Storage::FileSystem::{DELETE, FILE_ADD_FILE};
    assert_eq!(
        directory_access_error(path, FILE_ADD_FILE),
        None,
        "fixture grants real directory write access without guards"
    );
    assert_eq!(
        directory_access_error(path, DELETE),
        None,
        "fixture grants real directory delete access without guards"
    );
}

// Mutation oracle: metadata-only ancestor handles let write/delete opens and
// retargeting through after validation; a real restrictive reservation blocks
// them while the worker is paused before its first filesystem operation.
#[test]
fn held_ancestor_reservation_blocks_mutation_between_validation_and_save() {
    use windows_sys::Win32::Foundation::ERROR_SHARING_VIOLATION;
    use windows_sys::Win32::Storage::FileSystem::{DELETE, FILE_ADD_FILE};
    let temporary = tempdir().unwrap();
    let product = temporary.path().join("product");
    let moved = temporary.path().join("product-retargeted");
    fs::create_dir(&product).unwrap();
    assert_directory_mutation_access_is_available(&product);
    fs::rename(&product, &moved).expect("fixture permits rename before guards");
    fs::rename(&moved, &product).unwrap();
    let root = product.join("Code0");
    let lease = Code0ProfileLease::acquire(&root).unwrap();
    let mut initial = Code0SnapshotStore::new(&root, Some(lease.clone()));
    initial.save_atomic(&expected_snapshot()).unwrap();
    let gate = Code0SnapshotTestGate::new();
    let worker_gate = gate.clone();
    let worker_root = root.clone();
    let worker_lease = lease.clone();
    let worker = std::thread::spawn(move || {
        Code0SnapshotStore::with_path_validation_gate(worker_root, Some(worker_lease), worker_gate)
            .save_atomic(&candidate_snapshot())
    });
    let reached = gate.wait_until_after_path_validation(Duration::from_secs(5));
    let write_error = reached.then(|| directory_access_error(&product, FILE_ADD_FILE));
    let delete_error = reached.then(|| directory_access_error(&product, DELETE));
    let rename_blocked = reached && fs::rename(&product, &moved).is_err();
    gate.release();
    let saved = worker
        .join()
        .expect("bounded save worker exits after release");
    lease.release();
    // Clean the fixture only after the owned worker and guards are released,
    // even if the protection oracle failed and the ancestor really moved.
    if moved.exists() {
        fs::rename(&moved, &product).unwrap();
    }
    assert!(reached, "must pause after actual authority validation");
    assert_eq!(write_error, Some(Some(ERROR_SHARING_VIOLATION)));
    assert_eq!(delete_error, Some(Some(ERROR_SHARING_VIOLATION)));
    assert!(
        rename_blocked,
        "ancestor retarget must be blocked throughout the validated save operation"
    );
    assert_eq!(saved, Ok(()));
    assert_directory_mutation_access_is_available(&product);
    let loaded = Code0SnapshotStore::new(&root, None)
        .load()
        .unwrap()
        .unwrap();
    assert_eq!(loaded.fetched_at, "2026-10-27T04:06:06Z");
    assert_eq!(loaded.primary_metric.unwrap().current, 124.0);
}

// The same interleaving must protect a cold root without a profile lease;
// otherwise the held-root save test could mask metadata-only cold guards.
#[test]
fn cold_root_reservation_blocks_mutation_between_validation_and_snapshot_read() {
    use windows_sys::Win32::Foundation::ERROR_SHARING_VIOLATION;
    use windows_sys::Win32::Storage::FileSystem::{DELETE, FILE_ADD_FILE};
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("Code0");
    let moved = temporary.path().join("Code0-retargeted");
    let lease = Code0ProfileLease::acquire(&root).unwrap();
    let mut initial = Code0SnapshotStore::new(&root, Some(lease.clone()));
    initial.save_atomic(&expected_snapshot()).unwrap();
    lease.release();
    assert_directory_mutation_access_is_available(&root);
    fs::rename(&root, &moved).expect("fixture permits root rename without a lease");
    fs::rename(&moved, &root).unwrap();
    let gate = Code0SnapshotTestGate::new();
    let worker_gate = gate.clone();
    let worker_root = root.clone();
    let worker = std::thread::spawn(move || {
        Code0SnapshotStore::with_path_validation_gate(worker_root, None, worker_gate).load()
    });
    let reached = gate.wait_until_after_path_validation(Duration::from_secs(5));
    let write_error = reached.then(|| directory_access_error(&root, FILE_ADD_FILE));
    let delete_error = reached.then(|| directory_access_error(&root, DELETE));
    let rename_blocked = reached && fs::rename(&root, &moved).is_err();
    gate.release();
    let result = worker
        .join()
        .expect("bounded cold-read worker exits after release");
    if moved.exists() {
        fs::rename(&moved, &root).unwrap();
    }
    assert!(
        reached,
        "must pause after real cold-path validation and before snapshot open"
    );
    assert_eq!(write_error, Some(Some(ERROR_SHARING_VIOLATION)));
    assert_eq!(delete_error, Some(Some(ERROR_SHARING_VIOLATION)));
    assert!(
        rename_blocked,
        "cold-root retarget must be blocked throughout the read"
    );
    let loaded = result
        .expect("safe cold read completes")
        .expect("snapshot remains present");
    assert_eq!(loaded.status, UsageStatus::Cached);
    assert_eq!(loaded.fetched_at, FETCHED_AT);
    assert_eq!(loaded.primary_metric.unwrap().current, 123.5);
    assert_directory_mutation_access_is_available(&root);
    assert!(!root.join("snapshot.pending").exists());
}
