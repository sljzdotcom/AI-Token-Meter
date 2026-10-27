use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::domain::UsageSnapshot;
use crate::platform::windows::code0_profile_lease::Code0ProfileLease;
use crate::platform::windows::code0_web_session::{Code0SnapshotStoring, Code0StoreError};

/// Typed, instance-local failure injection used by bounded transaction tests.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code0SnapshotFailurePoint {
    AfterCandidateReplace,
    RestorePreviousSnapshot,
    BeforePendingRetirement,
    CandidateWrite,
    CandidateFlush,
    CandidateCleanup,
    RestoreWrite,
    RestoreFlush,
}

#[doc(hidden)]
#[derive(Default)]
struct Code0SnapshotGateState {
    reached_after_replace: bool,
    reached_after_path_validation: bool,
    released: bool,
}

#[doc(hidden)]
struct Code0SnapshotGateInner {
    state: Mutex<Code0SnapshotGateState>,
    changed: Condvar,
}

impl Default for Code0SnapshotGateInner {
    fn default() -> Self {
        Self {
            state: Mutex::new(Code0SnapshotGateState::default()),
            changed: Condvar::new(),
        }
    }
}

/// Instance-local bounded gate for a configured path-validation or candidate-
/// replacement boundary. It never changes filesystem validation or authority.
#[doc(hidden)]
#[derive(Clone, Default)]
pub struct Code0SnapshotTestGate {
    inner: Arc<Code0SnapshotGateInner>,
}

impl Code0SnapshotTestGate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn wait_until_after_replace(&self, timeout: Duration) -> bool {
        self.wait_until(timeout, |state| state.reached_after_replace)
    }

    pub fn wait_until_after_path_validation(&self, timeout: Duration) -> bool {
        self.wait_until(timeout, |state| state.reached_after_path_validation)
    }

    fn wait_until(&self, timeout: Duration, reached: fn(&Code0SnapshotGateState) -> bool) -> bool {
        let deadline = Instant::now() + timeout;
        let Ok(mut state) = self.inner.state.lock() else {
            return false;
        };
        while !reached(&state) {
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            let Ok((next, result)) = self.inner.changed.wait_timeout(state, deadline - now) else {
                return false;
            };
            state = next;
            if result.timed_out() && !reached(&state) {
                return false;
            }
        }
        true
    }

    pub fn release(&self) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.released = true;
            self.inner.changed.notify_all();
        }
    }

    fn pause_after_replace(&self, timeout: Duration) -> bool {
        self.pause(timeout, |state| state.reached_after_replace = true)
    }

    fn pause_after_path_validation(&self, timeout: Duration) -> bool {
        self.pause(timeout, |state| state.reached_after_path_validation = true)
    }

    fn pause(&self, timeout: Duration, mark_reached: fn(&mut Code0SnapshotGateState)) -> bool {
        let deadline = Instant::now() + timeout;
        let Ok(mut state) = self.inner.state.lock() else {
            return false;
        };
        mark_reached(&mut state);
        self.inner.changed.notify_all();
        while !state.released {
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            let Ok((next, result)) = self.inner.changed.wait_timeout(state, deadline - now) else {
                return false;
            };
            state = next;
            if result.timed_out() && !state.released {
                return false;
            }
        }
        true
    }
}

/// Independent strict five-field projection, protected by the Code0 profile
/// lease. It never shares the ordinary provider cache.
pub struct Code0SnapshotStore {
    root: PathBuf,
    lease: Option<Code0ProfileLease>,
    failure_points: Vec<Code0SnapshotFailurePoint>,
    after_replace_gate: Option<Code0SnapshotTestGate>,
    after_path_validation_gate: Option<Code0SnapshotTestGate>,
}

impl Code0SnapshotStore {
    pub fn new(root: impl AsRef<Path>, lease: Option<Code0ProfileLease>) -> Self {
        Self {
            root: root.as_ref().to_owned(),
            lease,
            failure_points: Vec::new(),
            after_replace_gate: None,
            after_path_validation_gate: None,
        }
    }

    /// Construct an isolated store with one-shot typed transaction failures.
    #[doc(hidden)]
    pub fn with_failure_points(
        root: impl AsRef<Path>,
        lease: Option<Code0ProfileLease>,
        failure_points: impl IntoIterator<Item = Code0SnapshotFailurePoint>,
    ) -> Self {
        Self {
            root: root.as_ref().to_owned(),
            lease,
            failure_points: failure_points.into_iter().collect(),
            after_replace_gate: None,
            after_path_validation_gate: None,
        }
    }

    /// Configure a bounded transaction gate local to this store instance.
    #[doc(hidden)]
    pub fn with_failure_gate(
        root: impl AsRef<Path>,
        lease: Option<Code0ProfileLease>,
        failure_points: impl IntoIterator<Item = Code0SnapshotFailurePoint>,
        gate: Code0SnapshotTestGate,
    ) -> Self {
        Self {
            root: root.as_ref().to_owned(),
            lease,
            failure_points: failure_points.into_iter().collect(),
            after_replace_gate: Some(gate),
            after_path_validation_gate: None,
        }
    }

    /// Pause only this instance after real path validation and before its
    /// filesystem operation. The original guard/authority remains alive.
    #[doc(hidden)]
    pub fn with_path_validation_gate(
        root: impl AsRef<Path>,
        lease: Option<Code0ProfileLease>,
        gate: Code0SnapshotTestGate,
    ) -> Self {
        Self {
            root: root.as_ref().to_owned(),
            lease,
            failure_points: Vec::new(),
            after_replace_gate: None,
            after_path_validation_gate: Some(gate),
        }
    }

    #[cfg(windows)]
    fn path_validation_ready(gate: Option<&Code0SnapshotTestGate>) -> Result<(), Code0StoreError> {
        if gate.is_none_or(|gate| gate.pause_after_path_validation(Duration::from_secs(10))) {
            Ok(())
        } else {
            Err(Code0StoreError::Unavailable)
        }
    }
}

impl Code0SnapshotStoring for Code0SnapshotStore {
    fn load(&self) -> Result<Option<UsageSnapshot>, Code0StoreError> {
        #[cfg(windows)]
        {
            if let Some(lease) = &self.lease {
                return lease
                    .with_root_handle(&self.root, |root_handle, _| {
                        Self::path_validation_ready(self.after_path_validation_gate.as_ref())?;
                        windows_impl::read_projection_held(&self.root, root_handle)
                    })
                    .map_err(|_| Code0StoreError::Unavailable)?
                    .map(|projection| projection.map(|(snapshot, _)| snapshot));
            }
            windows_impl::read_projection(&self.root, self.after_path_validation_gate.as_ref())
                .map(|projection| projection.map(|(snapshot, _)| snapshot))
        }
        #[cfg(not(windows))]
        {
            Err(Code0StoreError::Unavailable)
        }
    }

    fn save_atomic(&mut self, snapshot: &UsageSnapshot) -> Result<(), Code0StoreError> {
        #[cfg(windows)]
        {
            let lease = self.lease.as_ref().ok_or(Code0StoreError::Unavailable)?;
            let projection = windows_impl::projection_from_snapshot(snapshot)?;
            let bytes =
                serde_json::to_vec(&projection).map_err(|_| Code0StoreError::InvalidSnapshot)?;
            lease
                .with_root_handle(&self.root, |root_handle, _lock_handle| {
                    Self::path_validation_ready(self.after_path_validation_gate.as_ref())?;
                    windows_impl::save_projection(
                        &self.root,
                        root_handle,
                        &projection,
                        &bytes,
                        &mut self.failure_points,
                        self.after_replace_gate.as_ref(),
                    )
                })
                .map_err(|_| Code0StoreError::Unavailable)?
        }
        #[cfg(not(windows))]
        {
            let _ = snapshot;
            Err(Code0StoreError::Unavailable)
        }
    }

    fn remove(&mut self) -> Result<(), Code0StoreError> {
        #[cfg(windows)]
        {
            let lease = self.lease.as_ref().ok_or(Code0StoreError::Unavailable)?;
            lease
                .with_root_handle(&self.root, |root_handle, _lock_handle| {
                    Self::path_validation_ready(self.after_path_validation_gate.as_ref())?;
                    windows_impl::remove_projection(&self.root, root_handle)
                })
                .map_err(|_| Code0StoreError::Unavailable)?
        }
        #[cfg(not(windows))]
        {
            Err(Code0StoreError::Unavailable)
        }
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::{Code0SnapshotFailurePoint, Code0SnapshotTestGate};
    use crate::collectors::code0_capture::Code0WebsiteCapture;
    use crate::domain::{MetricKind, MetricUnit, UsageSnapshot, UsageStatus};
    use crate::platform::windows::code0_profile_lease::{
        guard_code0_read_path, validate_code0_handle, with_code0_security,
    };
    use crate::platform::windows::code0_web_session::Code0StoreError;
    use serde::{Deserialize, Serialize};
    use std::fs::{self, File};
    use std::io::{Read, Write};
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::{AsRawHandle, FromRawHandle, IntoRawHandle, OwnedHandle};
    use std::path::{Path, PathBuf};
    use std::ptr::{null, null_mut};
    use std::time::Duration;
    use time::{OffsetDateTime, format_description::well_known::Rfc3339};
    use windows_sys::Win32::Foundation::{
        ERROR_FILE_NOT_FOUND, GENERIC_READ, GENERIC_WRITE, GetLastError, HANDLE,
        INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CREATE_NEW, CreateFileW, DELETE, FILE_ATTRIBUTE_NORMAL, FILE_DISPOSITION_INFO,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_ID_INFO, FILE_READ_ATTRIBUTES, FILE_RENAME_INFO,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_STANDARD_INFO,
        FileDispositionInfo, FileIdInfo, FileRenameInfo, FileStandardInfo, OPEN_EXISTING,
        READ_CONTROL, SetFileInformationByHandle,
    };

    const MAX_PROJECTION_BYTES: usize = 4096;
    const PENDING_BYTES: &[u8] = b"AI-METER-CODE0-PENDING-v1";
    // These 16 exact reserved names bound interrupted-write cleanup. No scans/globs.
    const CANDIDATE_SLOTS: usize = 16;
    type SnapshotFileIdentity = (u64, [u8; 16]);
    type CandidateReceipt = (usize, SnapshotFileIdentity);
    const RECEIPT_BYTES: usize = 25;
    const MAX_RECEIPTS: usize = 2;

    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    #[serde(deny_unknown_fields, rename_all = "camelCase")]
    pub(super) struct Projection {
        schema_version: u64,
        balance: f64,
        cumulative_usage: f64,
        fetched_at: String,
        source_version: String,
    }

    fn unavailable<T>() -> Result<T, Code0StoreError> {
        Err(Code0StoreError::Unavailable)
    }

    fn wide(path: &Path) -> Result<Vec<u16>, Code0StoreError> {
        if !path.is_absolute()
            || path.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::CurDir | std::path::Component::ParentDir
                )
            })
        {
            return unavailable();
        }
        Ok(path.as_os_str().encode_wide().chain(Some(0)).collect())
    }

    fn open_handle(
        path: &Path,
        access: u32,
        share: u32,
        disposition: u32,
        flags: u32,
        security: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
    ) -> Result<OwnedHandle, u32> {
        let path = path
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                access,
                share,
                security,
                disposition,
                flags,
                null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            return Err(unsafe { GetLastError() });
        }
        Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
    }

    fn open_owned_file(
        path: &Path,
        access: u32,
        disposition: u32,
    ) -> Result<OwnedHandle, Code0StoreError> {
        let path_wide = wide(path)?;
        let result = with_code0_security(false, |attributes| unsafe {
            CreateFileW(
                path_wide.as_ptr(),
                access,
                FILE_SHARE_READ | FILE_SHARE_DELETE,
                attributes,
                disposition,
                FILE_FLAG_OPEN_REPARSE_POINT | FILE_ATTRIBUTE_NORMAL,
                null_mut(),
            )
        })
        .map_err(|_| Code0StoreError::Unavailable)?;
        if result == INVALID_HANDLE_VALUE || result.is_null() {
            return unavailable();
        }
        Ok(unsafe { OwnedHandle::from_raw_handle(result) })
    }

    fn path_is_absent(path: &Path) -> Result<bool, Code0StoreError> {
        match fs::symlink_metadata(path) {
            Ok(_) => Ok(false),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
            Err(_) => unavailable(),
        }
    }

    fn no_pending(root: &Path) -> Result<(), Code0StoreError> {
        if path_is_absent(&root.join("snapshot.pending"))? {
            Ok(())
        } else {
            unavailable()
        }
    }

    fn object_is_single_regular_file(handle: HANDLE) -> Result<(), Code0StoreError> {
        validate_code0_handle(handle, false).map_err(|_| Code0StoreError::Unavailable)
    }

    fn read_handle_with_identity(
        handle: OwnedHandle,
    ) -> Result<(Vec<u8>, SnapshotFileIdentity), Code0StoreError> {
        object_is_single_regular_file(handle.as_raw_handle())?;
        let raw = handle.as_raw_handle();
        let identity = file_identity(raw)?;
        let mut standard = FILE_STANDARD_INFO::default();
        if unsafe {
            windows_sys::Win32::Storage::FileSystem::GetFileInformationByHandleEx(
                raw,
                FileStandardInfo,
                (&mut standard as *mut FILE_STANDARD_INFO).cast(),
                std::mem::size_of::<FILE_STANDARD_INFO>() as u32,
            )
        } == 0
            || standard.EndOfFile < 0
            || standard.EndOfFile as usize > MAX_PROJECTION_BYTES
        {
            return unavailable();
        }
        let raw = handle.into_raw_handle();
        let file = unsafe { File::from_raw_handle(raw) };
        let mut bytes = Vec::with_capacity(standard.EndOfFile as usize);
        file.take((MAX_PROJECTION_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| Code0StoreError::Unavailable)?;
        if bytes.len() > MAX_PROJECTION_BYTES {
            return unavailable();
        }
        Ok((bytes, identity))
    }

    fn parse_projection(bytes: &[u8]) -> Result<(Projection, UsageSnapshot), Code0StoreError> {
        if bytes.len() > MAX_PROJECTION_BYTES {
            return unavailable();
        }
        let projection: Projection =
            serde_json::from_slice(bytes).map_err(|_| Code0StoreError::Unavailable)?;
        if projection.schema_version != 1
            || projection.source_version != "website"
            || !valid_number(projection.balance)
            || !valid_number(projection.cumulative_usage)
            || !valid_native_timestamp(&projection.fetched_at)
        {
            return unavailable();
        }
        let mut snapshot = Code0WebsiteCapture {
            balance: projection.balance,
            cumulative_usage: projection.cumulative_usage,
        }
        .make_snapshot(&projection.fetched_at)
        .map_err(|_| Code0StoreError::Unavailable)?;
        snapshot.status = UsageStatus::Cached;
        Ok((projection, snapshot))
    }

    fn valid_native_timestamp(value: &str) -> bool {
        let Ok(parsed) = OffsetDateTime::parse(value, &Rfc3339) else {
            return false;
        };
        parsed.unix_timestamp() >= 0
            && parsed.offset() == time::UtcOffset::UTC
            && parsed
                .format(&Rfc3339)
                .is_ok_and(|formatted| formatted == value)
    }

    fn valid_number(value: f64) -> bool {
        value.is_finite() && value >= 0.0 && value <= 9_007_199_254_740_991.0
    }

    fn open_existing_file(path: &Path) -> Result<OwnedHandle, u32> {
        open_handle(
            path,
            GENERIC_READ | READ_CONTROL,
            FILE_SHARE_READ | FILE_SHARE_DELETE,
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT,
            null(),
        )
    }

    fn open_delete_file(path: &Path) -> Result<OwnedHandle, u32> {
        open_handle(
            path,
            GENERIC_READ | DELETE | READ_CONTROL,
            FILE_SHARE_READ | FILE_SHARE_DELETE,
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT,
            null(),
        )
    }

    pub(super) fn read_projection(
        root: &Path,
        gate: Option<&Code0SnapshotTestGate>,
    ) -> Result<Option<(UsageSnapshot, Vec<u8>)>, Code0StoreError> {
        let guard = guard_code0_read_path(root).map_err(|_| Code0StoreError::Unavailable)?;
        if !gate.is_none_or(|gate| gate.pause_after_path_validation(Duration::from_secs(10))) {
            return unavailable();
        }
        let Some(root_handle) = guard.root_handle() else {
            guard.validate().map_err(|_| Code0StoreError::Unavailable)?;
            return Ok(None);
        };
        let result = read_projection_held(root, root_handle)?;
        guard.validate().map_err(|_| Code0StoreError::Unavailable)?;
        Ok(result)
    }

    pub(super) fn read_projection_held(
        root: &Path,
        root_handle: HANDLE,
    ) -> Result<Option<(UsageSnapshot, Vec<u8>)>, Code0StoreError> {
        validate_code0_handle(root_handle, true).map_err(|_| Code0StoreError::Unavailable)?;
        no_pending(root)?;
        let snapshot_path = root.join("snapshot.json");
        let snapshot_handle = match open_existing_file(&snapshot_path) {
            Ok(handle) => handle,
            Err(ERROR_FILE_NOT_FOUND) => {
                no_pending(root)?;
                validate_code0_handle(root_handle, true)
                    .map_err(|_| Code0StoreError::Unavailable)?;
                return Ok(None);
            }
            Err(_) => return unavailable(),
        };
        let (bytes, opened_identity) = read_handle_with_identity(snapshot_handle)?;
        let (_, snapshot) = parse_projection(&bytes)?;
        no_pending(root)?;
        validate_code0_handle(root_handle, true).map_err(|_| Code0StoreError::Unavailable)?;
        let reopened =
            open_existing_file(&snapshot_path).map_err(|_| Code0StoreError::Unavailable)?;
        object_is_single_regular_file(reopened.as_raw_handle())?;
        if opened_identity != file_identity(reopened.as_raw_handle())? {
            return unavailable();
        }
        // Check the barrier after identity acceptance too, while the final read
        // handle still rejects writes. A concurrent replacement cannot make an
        // earlier opened B eligible after a failed transaction restores A.
        no_pending(root)?;
        Ok(Some((snapshot, bytes)))
    }

    fn file_identity(handle: HANDLE) -> Result<SnapshotFileIdentity, Code0StoreError> {
        let mut info = FILE_ID_INFO::default();
        if unsafe {
            windows_sys::Win32::Storage::FileSystem::GetFileInformationByHandleEx(
                handle,
                FileIdInfo,
                (&mut info as *mut FILE_ID_INFO).cast(),
                std::mem::size_of::<FILE_ID_INFO>() as u32,
            )
        } == 0
            || info.FileId.Identifier.iter().all(|byte| *byte == 0)
        {
            return unavailable();
        }
        Ok((info.VolumeSerialNumber, info.FileId.Identifier))
    }

    fn file_identity_from_path(path: &Path) -> Result<SnapshotFileIdentity, Code0StoreError> {
        // A metadata-only reopen must share the access of our still-owned
        // writer. Its existing strict handle continues to deny other writers.
        let handle = open_handle(
            path,
            FILE_READ_ATTRIBUTES | READ_CONTROL,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT,
            null(),
        )
        .map_err(|_| Code0StoreError::Unavailable)?;
        object_is_single_regular_file(handle.as_raw_handle())?;
        file_identity(handle.as_raw_handle())
    }

    pub(super) fn projection_from_snapshot(
        snapshot: &UsageSnapshot,
    ) -> Result<Projection, Code0StoreError> {
        let primary = snapshot
            .primary_metric
            .as_ref()
            .ok_or(Code0StoreError::InvalidSnapshot)?;
        let secondary = snapshot
            .secondary_metric
            .as_ref()
            .ok_or(Code0StoreError::InvalidSnapshot)?;
        if snapshot.schema_version != 1
            || snapshot.provider_id != crate::domain::ProviderId::Code0
            || snapshot.display_name != "Code0"
            || snapshot.status != UsageStatus::Fresh
            || snapshot.used_ratio.is_some()
            || snapshot.source_version.as_deref() != Some("website")
            || snapshot.stale_after_seconds != 300
            || snapshot.status_message.is_some()
            || !snapshot.gemini_quota_metrics.is_empty()
            || !snapshot.antigravity_shared_quota_metrics.is_empty()
            || snapshot.antigravity_cli_info.is_some()
            || !snapshot.reset_credits.is_empty()
            || snapshot.local_activity.is_some()
            || !snapshot.daily_history.is_empty()
            || snapshot.history_fetched_at.is_some()
            || primary.label != "Balance"
            || primary.kind != MetricKind::Balance
            || primary.unit != MetricUnit::SiteCredits
            || primary.limit.is_some()
            || primary.reset_at.is_some()
            || primary.reset_description.is_some()
            || secondary.label != "Cumulative usage"
            || secondary.kind != MetricKind::CumulativeUsage
            || secondary.unit != MetricUnit::SiteCredits
            || secondary.limit.is_some()
            || secondary.reset_at.is_some()
            || secondary.reset_description.is_some()
            || !valid_number(primary.current)
            || !valid_number(secondary.current)
            || !valid_native_timestamp(&snapshot.fetched_at)
        {
            return Err(Code0StoreError::InvalidSnapshot);
        }
        Ok(Projection {
            schema_version: 1,
            balance: primary.current,
            cumulative_usage: secondary.current,
            fetched_at: snapshot.fetched_at.clone(),
            source_version: "website".to_owned(),
        })
    }

    fn create_pending(root: &Path) -> Result<File, Code0StoreError> {
        let path = root.join("snapshot.pending");
        let handle = open_owned_file(
            &path,
            GENERIC_READ | GENERIC_WRITE | DELETE | READ_CONTROL,
            CREATE_NEW,
        )?;
        object_is_single_regular_file(handle.as_raw_handle())?;
        let mut file = unsafe { File::from_raw_handle(handle.into_raw_handle()) };
        // A partially written marker still blocks cold readers and may be
        // removed by explicit secure cleanup; never infer absence from failure.
        file.write_all(PENDING_BYTES)
            .map_err(|_| Code0StoreError::Unavailable)?;
        file.sync_all().map_err(|_| Code0StoreError::Unavailable)?;
        file_identity(file.as_raw_handle())?;
        Ok(file)
    }

    fn record_candidate(
        pending: &mut File,
        slot: usize,
        handle: HANDLE,
    ) -> Result<(), Code0StoreError> {
        let (volume, id) = file_identity(handle)?;
        let mut receipt = [0u8; RECEIPT_BYTES];
        receipt[0] = u8::try_from(slot).map_err(|_| Code0StoreError::Unavailable)?;
        receipt[1..9].copy_from_slice(&volume.to_le_bytes());
        receipt[9..].copy_from_slice(&id);
        pending
            .write_all(&receipt)
            .map_err(|_| Code0StoreError::Unavailable)?;
        // Numeric bytes may only be written after this ownership receipt is
        // durable. A crash before receipt completion leaves no numeric bytes.
        pending.sync_all().map_err(|_| Code0StoreError::Unavailable)
    }

    fn read_candidate_receipts(root: &Path) -> Result<Vec<CandidateReceipt>, Code0StoreError> {
        let mut receipts = Vec::new();
        for name in ["snapshot.pending", "snapshot.retired"] {
            let handle = match open_existing_file(&root.join(name)) {
                Ok(handle) => handle,
                Err(ERROR_FILE_NOT_FOUND) => continue,
                Err(_) => return unavailable(),
            };
            let (bytes, _) = read_handle_with_identity(handle)?;
            if !bytes.starts_with(PENDING_BYTES) {
                return unavailable();
            }
            let rest = &bytes[PENDING_BYTES.len()..];
            if rest.len() % RECEIPT_BYTES != 0 || rest.len() / RECEIPT_BYTES > MAX_RECEIPTS {
                return unavailable();
            }
            for record in rest.chunks_exact(RECEIPT_BYTES) {
                let slot = usize::from(record[0]);
                let volume = u64::from_le_bytes(
                    record[1..9]
                        .try_into()
                        .map_err(|_| Code0StoreError::Unavailable)?,
                );
                let id: [u8; 16] = record[9..]
                    .try_into()
                    .map_err(|_| Code0StoreError::Unavailable)?;
                if slot >= CANDIDATE_SLOTS || id.iter().all(|byte| *byte == 0) {
                    return unavailable();
                }
                receipts.push((slot, (volume, id)));
            }
        }
        Ok(receipts)
    }

    fn no_occupied_candidates(root: &Path) -> Result<(), Code0StoreError> {
        for slot in 0..CANDIDATE_SLOTS {
            if !path_is_absent(&candidate_path(root, slot))? {
                return unavailable();
            }
        }
        Ok(())
    }

    fn candidate_path(root: &Path, slot: usize) -> PathBuf {
        root.join(format!("snapshot-candidate-{slot}.tmp"))
    }

    fn create_candidate(
        root: &Path,
        bytes: &[u8],
        failure_points: &mut Vec<Code0SnapshotFailurePoint>,
        restoring: bool,
        pending: &mut File,
    ) -> Result<(PathBuf, File), Code0StoreError> {
        for slot in 0..CANDIDATE_SLOTS {
            let path = candidate_path(root, slot);
            let handle = match open_owned_file(
                &path,
                GENERIC_READ | GENERIC_WRITE | DELETE | READ_CONTROL,
                CREATE_NEW,
            ) {
                Ok(handle) => handle,
                Err(Code0StoreError::Unavailable) if !path_is_absent(&path)? => continue,
                Err(error) => return Err(error),
            };
            object_is_single_regular_file(handle.as_raw_handle())?;
            let mut file = unsafe { File::from_raw_handle(handle.into_raw_handle()) };
            let write_point = if restoring {
                Code0SnapshotFailurePoint::RestoreWrite
            } else {
                Code0SnapshotFailurePoint::CandidateWrite
            };
            let flush_point = if restoring {
                Code0SnapshotFailurePoint::RestoreFlush
            } else {
                Code0SnapshotFailurePoint::CandidateFlush
            };
            // Inject after an actual partial write/file creation, not before
            // ownership exists, so residue handling is exercised.
            let result = (|| {
                record_candidate(pending, slot, file.as_raw_handle())?;
                file.write_all(&bytes[..bytes.len().min(8)])
                    .map_err(|_| Code0StoreError::Unavailable)?;
                if take_failure(failure_points, write_point) {
                    return unavailable();
                }
                file.write_all(&bytes[bytes.len().min(8)..])
                    .map_err(|_| Code0StoreError::Unavailable)?;
                if take_failure(failure_points, flush_point) {
                    return unavailable();
                }
                file.sync_all().map_err(|_| Code0StoreError::Unavailable)
            })();
            if let Err(error) = result {
                let handle = unsafe { OwnedHandle::from_raw_handle(file.into_raw_handle()) };
                // A failed delete leaves one of the finite reserved paths;
                // remove_projection will retry only its exact secure handle.
                if !take_failure(failure_points, Code0SnapshotFailurePoint::CandidateCleanup) {
                    let _ = delete_handle_and_confirm(&path, handle);
                }
                return Err(error);
            }
            return Ok((path, file));
        }
        unavailable()
    }

    fn rename_handle(
        handle: HANDLE,
        root_handle: HANDLE,
        leaf_name: &str,
    ) -> Result<(), Code0StoreError> {
        let name = leaf_name.encode_utf16().collect::<Vec<_>>();
        let name_bytes = name
            .len()
            .checked_mul(std::mem::size_of::<u16>())
            .ok_or(Code0StoreError::Unavailable)?;
        let offset = std::mem::offset_of!(FILE_RENAME_INFO, FileName);
        let buffer_len = offset
            .checked_add(name_bytes)
            .ok_or(Code0StoreError::Unavailable)?;
        if name_bytes > u32::MAX as usize || buffer_len > u32::MAX as usize {
            return unavailable();
        }
        let mut aligned = vec![0u128; buffer_len.div_ceil(std::mem::size_of::<u128>())];
        let info = aligned.as_mut_ptr().cast::<FILE_RENAME_INFO>();
        unsafe {
            (*info).Anonymous.ReplaceIfExists = true;
            (*info).RootDirectory = root_handle;
            (*info).FileNameLength = name_bytes as u32;
            std::ptr::copy_nonoverlapping(
                name.as_ptr(),
                std::ptr::addr_of_mut!((*info).FileName).cast::<u16>(),
                name.len(),
            );
        }
        if unsafe {
            SetFileInformationByHandle(handle, FileRenameInfo, info.cast(), buffer_len as u32)
        } == 0
        {
            return unavailable();
        }
        Ok(())
    }

    fn mark_delete(handle: HANDLE) -> Result<(), Code0StoreError> {
        let mut disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
        if unsafe {
            SetFileInformationByHandle(
                handle,
                FileDispositionInfo,
                (&mut disposition as *mut FILE_DISPOSITION_INFO).cast(),
                std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
            )
        } == 0
        {
            return unavailable();
        }
        Ok(())
    }

    fn delete_handle_and_confirm(path: &Path, handle: OwnedHandle) -> Result<(), Code0StoreError> {
        mark_delete(handle.as_raw_handle())?;
        drop(handle);
        if path_is_absent(path)? {
            Ok(())
        } else {
            unavailable()
        }
    }

    fn remove_fixed_owned_file(root: &Path, leaf: &str) -> Result<(), Code0StoreError> {
        let path = root.join(leaf);
        let handle = match open_delete_file(&path) {
            Ok(handle) => handle,
            Err(ERROR_FILE_NOT_FOUND) => return Ok(()),
            Err(_) => return unavailable(),
        };
        object_is_single_regular_file(handle.as_raw_handle())?;
        delete_handle_and_confirm(&path, handle)
    }

    fn retire_pending(root_handle: HANDLE, pending: File) -> Result<(), Code0StoreError> {
        // Final visibility commit: a successful rename removes the pending
        // NAME immediately, even while another compatible handle retains the
        // marker object. Nothing fallible follows this transition. The retired
        // nonnumeric marker is securely removed before the next transaction or
        // explicit cleanup, never treated as an unconfirmed delete/absence.
        rename_handle(pending.as_raw_handle(), root_handle, "snapshot.retired")
    }

    fn take_failure(
        failure_points: &mut Vec<Code0SnapshotFailurePoint>,
        point: Code0SnapshotFailurePoint,
    ) -> bool {
        let Some(index) = failure_points.iter().position(|value| *value == point) else {
            return false;
        };
        failure_points.remove(index);
        true
    }

    fn restore_projection(
        root: &Path,
        root_handle: HANDLE,
        old: Option<&Projection>,
        candidate_file: &mut Option<File>,
        failure_points: &mut Vec<Code0SnapshotFailurePoint>,
        pending: &mut File,
    ) -> Result<(), Code0StoreError> {
        if take_failure(
            failure_points,
            Code0SnapshotFailurePoint::RestorePreviousSnapshot,
        ) {
            return unavailable();
        }
        match old {
            Some(old) => {
                let bytes = serde_json::to_vec(old).map_err(|_| Code0StoreError::Unavailable)?;
                let (temporary_path, file) =
                    create_candidate(root, &bytes, failure_points, true, pending)?;
                if let Err(error) =
                    rename_handle(file.as_raw_handle(), root_handle, "snapshot.json")
                {
                    let owned = unsafe { OwnedHandle::from_raw_handle(file.into_raw_handle()) };
                    let _ = delete_handle_and_confirm(&temporary_path, owned);
                    return Err(error);
                }
                file.sync_all().map_err(|_| Code0StoreError::Unavailable)?;
                if file_identity_from_path(&root.join("snapshot.json"))?
                    != file_identity(file.as_raw_handle())?
                {
                    return unavailable();
                }
                Ok(())
            }
            None => {
                let path = root.join("snapshot.json");
                let file = candidate_file.take().ok_or(Code0StoreError::Unavailable)?;
                let handle = unsafe { OwnedHandle::from_raw_handle(file.into_raw_handle()) };
                object_is_single_regular_file(handle.as_raw_handle())?;
                delete_handle_and_confirm(&path, handle)
            }
        }
    }

    pub(super) fn save_projection(
        root: &Path,
        root_handle: HANDLE,
        projection: &Projection,
        candidate_bytes: &[u8],
        failure_points: &mut Vec<Code0SnapshotFailurePoint>,
        gate: Option<&Code0SnapshotTestGate>,
    ) -> Result<(), Code0StoreError> {
        validate_code0_handle(root_handle, true).map_err(|_| Code0StoreError::Unavailable)?;
        no_pending(root)?;
        // Any inability to clear the previous nonnumeric retirement object is
        // resolved before installing B or creating this transaction's marker.
        no_occupied_candidates(root)?;
        remove_fixed_owned_file(root, "snapshot.retired")?;
        let old = read_projection_held(root, root_handle)?
            .map(|(_, bytes)| {
                serde_json::from_slice::<Projection>(&bytes)
                    .map_err(|_| Code0StoreError::Unavailable)
            })
            .transpose()?;
        let mut pending = create_pending(root)?;
        let (candidate_path, candidate_file) =
            match create_candidate(root, candidate_bytes, failure_points, false, &mut pending) {
                Ok(candidate) => candidate,
                Err(error) => {
                    let _ = retire_pending(root_handle, pending);
                    return Err(error);
                }
            };
        if let Err(error) =
            rename_handle(candidate_file.as_raw_handle(), root_handle, "snapshot.json")
        {
            let owned = unsafe { OwnedHandle::from_raw_handle(candidate_file.into_raw_handle()) };
            let _ = delete_handle_and_confirm(&candidate_path, owned);
            let _ = retire_pending(root_handle, pending);
            return Err(error);
        }

        let gate_released =
            gate.is_none_or(|gate| gate.pause_after_replace(Duration::from_secs(10)));
        let forced_failure = take_failure(
            failure_points,
            Code0SnapshotFailurePoint::AfterCandidateReplace,
        );
        let mut candidate_file = Some(candidate_file);
        let candidate = candidate_file.as_ref().expect("candidate remains owned");
        let candidate_visible = file_identity(candidate.as_raw_handle())
            .and_then(|identity| {
                file_identity_from_path(&root.join("snapshot.json"))
                    .map(|visible| visible == identity)
            })
            .unwrap_or(false);
        let precommit = gate_released
            && !forced_failure
            && candidate_visible
            && candidate.sync_all().is_ok()
            && validate_code0_handle(root_handle, true).is_ok()
            && serde_json::from_slice::<Projection>(candidate_bytes)
                .ok()
                .as_ref()
                == Some(projection)
            && !take_failure(
                failure_points,
                Code0SnapshotFailurePoint::BeforePendingRetirement,
            );
        // Keep the original marker handle available on failed retirement; it
        // has not been marked delete-pending and cannot disappear on reader close.
        if precommit
            && rename_handle(pending.as_raw_handle(), root_handle, "snapshot.retired").is_ok()
        {
            return Ok(()); // Exact final commit; no fallible post-close observation.
        }

        // Exactly one restoration attempt under the original durable barrier.
        // A failed restore or retirement leaves pending in place across reopen.
        let restored = restore_projection(
            root,
            root_handle,
            old.as_ref(),
            &mut candidate_file,
            failure_points,
            &mut pending,
        );
        drop(candidate_file);
        if restored.is_ok() {
            let _ = retire_pending(root_handle, pending);
        }
        unavailable()
    }

    pub(super) fn remove_projection(
        root: &Path,
        root_handle: HANDLE,
    ) -> Result<(), Code0StoreError> {
        validate_code0_handle(root_handle, true).map_err(|_| Code0StoreError::Unavailable)?;
        // Payload syntax is irrelevant to deletion of these exact disposable
        // owned paths. Type/reparse/link/owner/DACL and handle deletion remain strict.
        // Validate candidate provenance BEFORE deleting anything, so a secure
        // unknown same-name file is preserved and its receipts are not lost.
        let receipts = if no_occupied_candidates(root).is_err() {
            read_candidate_receipts(root)?
        } else {
            Vec::new()
        };
        let mut candidates = Vec::new();
        for slot in 0..CANDIDATE_SLOTS {
            let path = candidate_path(root, slot);
            let handle = match open_delete_file(&path) {
                Ok(handle) => handle,
                Err(ERROR_FILE_NOT_FOUND) => continue,
                Err(_) => return unavailable(),
            };
            object_is_single_regular_file(handle.as_raw_handle())?;
            let identity = file_identity(handle.as_raw_handle())?;
            if !receipts.contains(&(slot, identity)) {
                return unavailable();
            }
            candidates.push((path, handle));
        }
        remove_fixed_owned_file(root, "snapshot.json")?;
        for (path, handle) in candidates {
            delete_handle_and_confirm(&path, handle)?;
        }
        remove_fixed_owned_file(root, "snapshot.retired")?;
        // Remove the blocking marker last, only after all numeric residue is gone.
        remove_fixed_owned_file(root, "snapshot.pending")
    }
}
