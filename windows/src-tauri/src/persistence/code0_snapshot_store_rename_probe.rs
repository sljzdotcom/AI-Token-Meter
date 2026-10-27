// Test-only native API evidence. A completed matrix does not assert that any
// rename arm succeeds, and does not establish transactional store correctness.
use super::*;
use crate::platform::windows::code0_profile_lease::Code0ProfileLease;
use serde::{Deserialize, Serialize};
use std::ffi::{OsStr, OsString};
use std::os::windows::fs::MetadataExt;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Instant;
use windows_sys::Wdk::Storage::FileSystem::{
    FILE_RENAME_INFORMATION, FileRenameInformation, NtSetInformationFile,
};
use windows_sys::Win32::Storage::FileSystem::{CreateDirectoryW, FILE_ATTRIBUTE_REPARSE_POINT};
use windows_sys::Win32::System::IO::{IO_STATUS_BLOCK, IO_STATUS_BLOCK_0};

const PREFIX: &str = "ai-meter-req004-native-rename-";
const SENTINEL_KEY: &str = "AI_METER_REQ004_NATIVE_RENAME_CHILD";
const SENTINEL: &str = "native-rename-evidence-v1";
const BASE_KEY: &str = "AI_METER_REQ004_NATIVE_RENAME_BASE";
const ARM_KEY: &str = "AI_METER_REQ004_NATIVE_RENAME_ARM";
const CHILD_TEST: &str = "persistence::code0_snapshot_store::windows_impl::native_rename_probe::native_rename_probe_child";
const MANIFEST: &[u8] = b"AI-METER-REQ004-NATIVE-RENAME-v1\nA\nB\nC\nD\nE\n";
const LEAF: &str = "snapshot.json";
const MAX_RECORD_BYTES: u64 = 8192;
const CHILD_WAIT: Duration = Duration::from_secs(15);
const WHOLE_WAIT: Duration = Duration::from_secs(70);
const WATCHDOG_WAIT: Duration = Duration::from_secs(10);
const UNWRITTEN_STATUS: i32 = i32::MIN;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Arm {
    A,
    B,
    C,
    D,
    E,
}

impl Arm {
    const ALL: [Self; 5] = [Self::A, Self::B, Self::C, Self::D, Self::E];

    fn label(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::E => "E",
        }
    }

    fn from_label(value: &OsStr) -> Option<Self> {
        Self::ALL.into_iter().find(|arm| value == arm.label())
    }

    fn native(self) -> bool {
        matches!(self, Self::C | Self::D)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Observation {
    Yes,
    No,
    Unknown,
}

impl Observation {
    fn from_bool(value: bool) -> Self {
        if value { Self::Yes } else { Self::No }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Phase {
    Win32Rename,
    NativeRename,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum Win32Error {
    NotApplicable,
    Code(u32),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum IoStatus {
    Unknown,
    Status(i32),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "api", deny_unknown_fields)]
enum ApiResult {
    Win32 { succeeded: bool, error: Win32Error },
    Native { status: i32, io_status: IoStatus },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: u32,
    arm: Arm,
    phase: Phase,
    result: ApiResult,
    source_identity_stable: Observation,
    root_identity_stable: Observation,
    authority_valid_after: Observation,
    original_parent_target_matches: Observation,
    unrelated_cwd_target_matches: Observation,
    complete: bool,
}

impl Record {
    fn new(arm: Arm, result: ApiResult) -> Self {
        Self {
            schema: 1,
            arm,
            phase: if arm.native() {
                Phase::NativeRename
            } else {
                Phase::Win32Rename
            },
            result,
            source_identity_stable: Observation::Unknown,
            root_identity_stable: Observation::Unknown,
            authority_valid_after: Observation::Unknown,
            original_parent_target_matches: Observation::Unknown,
            unrelated_cwd_target_matches: Observation::Unknown,
            complete: false,
        }
    }

    fn valid_complete(self, expected: Arm) -> bool {
        if self.schema != 1
            || self.arm != expected
            || !self.complete
            || self.source_identity_stable != Observation::Yes
            || self.root_identity_stable != Observation::Yes
            || self.authority_valid_after != Observation::Yes
        {
            return false;
        }
        match self.result {
            ApiResult::Win32 { succeeded, error } => {
                !expected.native()
                    && self.phase == Phase::Win32Rename
                    && matches!(
                        (succeeded, error),
                        (true, Win32Error::NotApplicable) | (false, Win32Error::Code(_))
                    )
            }
            ApiResult::Native { status, io_status } => {
                expected.native()
                    && self.phase == Phase::NativeRename
                    && status <= 0
                    && matches!(io_status, IoStatus::Unknown | IoStatus::Status(..=0))
            }
        }
    }
}

#[derive(Clone, Copy)]
enum ProbeFailure {
    Failed,
    Unreaped,
}

type ProbeResult<T> = Result<T, ProbeFailure>;

struct Layout {
    base: PathBuf,
    arm_dir: PathBuf,
    root: PathBuf,
    cwd: PathBuf,
    marker: PathBuf,
}

impl Layout {
    fn new(base: &Path, arm: Arm) -> Self {
        let arm_dir = base.join(arm.label());
        Self {
            base: base.to_owned(),
            root: arm_dir.join("Code0"),
            cwd: arm_dir.join("cwd"),
            marker: arm_dir.join("complete.json"),
            arm_dir,
        }
    }
}

fn failed<T>(_: T) -> ProbeFailure {
    ProbeFailure::Failed
}

fn require(condition: bool) -> ProbeResult<()> {
    if condition {
        Ok(())
    } else {
        Err(ProbeFailure::Failed)
    }
}

fn plain_directory(path: &Path) -> ProbeResult<()> {
    let metadata = fs::symlink_metadata(path).map_err(failed)?;
    require(metadata.is_dir() && metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0)
}

fn absent(path: &Path) -> ProbeResult<()> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err(ProbeFailure::Failed),
    }
}

// Canonical path equality plus every component's reparse check prevents the
// child-only base argument from becoming an arbitrary path or alias authority.
fn validated_base(base: &Path) -> ProbeResult<PathBuf> {
    require(base.is_absolute())?;
    require(!base.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir | std::path::Component::CurDir
        )
    }))?;
    let temp = fs::canonicalize(std::env::temp_dir()).map_err(failed)?;
    let canonical = fs::canonicalize(base).map_err(failed)?;
    require(canonical == base && canonical.parent() == Some(temp.as_path()))?;
    let name = canonical
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or(ProbeFailure::Failed)?;
    let suffix = name.strip_prefix(PREFIX).ok_or(ProbeFailure::Failed)?;
    require(
        (6..=32).contains(&suffix.len()) && suffix.bytes().all(|byte| byte.is_ascii_alphanumeric()),
    )?;
    for component in canonical.ancestors() {
        plain_directory(component)?;
    }
    Ok(canonical)
}

fn bounded_read(path: &Path) -> ProbeResult<Vec<u8>> {
    let metadata = fs::symlink_metadata(path).map_err(failed)?;
    require(
        metadata.is_file()
            && metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0
            && metadata.len() <= MAX_RECORD_BYTES,
    )?;
    let file = File::open(path).map_err(failed)?;
    let mut bytes = Vec::new();
    file.take(MAX_RECORD_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(failed)?;
    require(bytes.len() as u64 <= MAX_RECORD_BYTES)?;
    Ok(bytes)
}

fn validate_layout(layout: &Layout) -> ProbeResult<()> {
    require(validated_base(&layout.base)? == layout.base)?;
    plain_directory(&layout.arm_dir)?;
    plain_directory(&layout.root)?;
    plain_directory(&layout.cwd)?;
    require(fs::canonicalize(&layout.arm_dir).map_err(failed)? == layout.arm_dir)?;
    require(fs::canonicalize(&layout.root).map_err(failed)? == layout.root)?;
    require(fs::canonicalize(&layout.cwd).map_err(failed)? == layout.cwd)?;
    require(bounded_read(&layout.base.join("manifest"))? == MANIFEST)?;
    absent(&layout.marker)?;
    absent(&layout.root.join("snapshot-candidate-0.tmp"))?;
    absent(&layout.root.join(LEAF))?;
    absent(&layout.cwd.join(LEAF))
}

fn secure_directory(path: &Path) -> ProbeResult<()> {
    let path = wide(path).map_err(failed)?;
    let succeeded = with_code0_security(true, |attributes| unsafe {
        CreateDirectoryW(path.as_ptr(), attributes)
    })
    .map_err(failed)?;
    require(succeeded != 0)
}

fn write_record(path: &Path, record: &Record) -> ProbeResult<()> {
    let bytes = serde_json::to_vec(record).map_err(failed)?;
    require(bytes.len() as u64 <= MAX_RECORD_BYTES)?;
    // The fixed file is created once. The parent only reads after successful
    // reap; no path rename/publication inside a guarded ancestor is needed.
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(failed)?;
    file.write_all(&bytes).map_err(failed)?;
    file.sync_all().map_err(failed)
}

#[test]
fn native_rename_probe_has_five_observed_arms() {
    // This test catches missing child selection, incomplete/pending evidence,
    // duplicate/missing arms and unbounded/unknown-schema marker acceptance.
    // API failure is an observed result, not a product compatibility pass.
    assert!(
        run_matrix().is_ok(),
        "Code0NativeRenameProbe harness did not complete"
    );
}

fn run_matrix() -> ProbeResult<()> {
    let deadline = Instant::now() + WHOLE_WAIT;
    let temp = fs::canonicalize(std::env::temp_dir()).map_err(failed)?;
    let owned = tempfile::Builder::new()
        .prefix(PREFIX)
        .tempdir_in(temp)
        .map_err(failed)?;
    let base = fs::canonicalize(owned.path()).map_err(failed)?;
    let result = run_owned_matrix(&base, deadline);
    if matches!(result, Err(ProbeFailure::Unreaped)) {
        // Never race deletion with a child whose termination is unconfirmed.
        // Do not log this owned path or target any process by PID.
        let _ = owned.keep();
        return result;
    }
    owned.close().map_err(failed)?;
    require(Instant::now() < deadline)?;
    result
}

fn run_owned_matrix(base: &Path, deadline: Instant) -> ProbeResult<()> {
    validated_base(base)?;
    fs::write(base.join("manifest"), MANIFEST).map_err(failed)?;
    for arm in Arm::ALL {
        require(Instant::now() < deadline)?;
        let layout = Layout::new(base, arm);
        fs::create_dir(&layout.arm_dir).map_err(failed)?;
        secure_directory(&layout.root)?;
        fs::create_dir(&layout.cwd).map_err(failed)?;
        validate_layout(&layout)?;
    }
    let executable = std::env::current_exe().map_err(failed)?;
    require(executable.is_absolute())?;
    let mut records = Vec::with_capacity(5);
    for arm in Arm::ALL {
        require(Instant::now() < deadline)?;
        let layout = Layout::new(base, arm);
        let mut command = Command::new(&executable);
        command
            .arg(CHILD_TEST)
            .args(["--exact", "--nocapture", "--test-threads=1"])
            .current_dir(&layout.cwd)
            .env_clear()
            .env(SENTINEL_KEY, SENTINEL)
            .env(BASE_KEY, base)
            .env(ARM_KEY, arm.label())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // SID/ACL lookup uses the inherited process token, not profile/account
        // environment. No PATH, credentials or app/default-root variables.
        for key in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        let mut child = command.spawn().map_err(failed)?;
        let child_deadline = (Instant::now() + CHILD_WAIT).min(deadline);
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Err(_) => return Err(ProbeFailure::Unreaped),
                Ok(None) if Instant::now() >= child_deadline => return Err(ProbeFailure::Unreaped),
                Ok(None) => thread::sleep(Duration::from_millis(10)),
            }
        };
        if !status.success() {
            if status.code() == Some(71) {
                // Exit71 also covers other unrecognized nonfinal statuses;
                // it does not identify an exact pending status or its cause.
                eprintln!(
                    "Code0NativeRenameProbe arm={} unfinal-child-status/missing-complete",
                    arm.label()
                );
            }
            return Err(ProbeFailure::Failed);
        }
        let bytes = bounded_read(&layout.marker)?;
        let record: Record = serde_json::from_slice(&bytes).map_err(failed)?;
        require(record.valid_complete(arm))?;
        require(!records.iter().any(|prior: &Record| prior.arm == record.arm))?;
        records.push(record);
    }
    require(records.len() == 5 && Instant::now() < deadline)?;
    // Emit only our parsed fixed schema after every arm completed. Never dump
    // child output, marker bytes, path-valued assertions or raw I/O errors.
    for record in records {
        let line = serde_json::to_string(&record).map_err(failed)?;
        require(line.len() as u64 <= MAX_RECORD_BYTES)?;
        eprintln!("Code0NativeRenameProbe {line}");
    }
    Ok(())
}

#[test]
fn native_rename_probe_child() {
    if std::env::var_os(SENTINEL_KEY).as_deref() != Some(OsStr::new(SENTINEL)) {
        // The normal libtest invocation is a no-op, not a sixth native arm.
        return;
    }
    assert!(
        std::env::args_os()
            .skip(1)
            .eq([CHILD_TEST, "--exact", "--nocapture", "--test-threads=1"].map(OsString::from)),
        "Code0NativeRenameProbe child selection invalid"
    );
    let done = Arc::new(AtomicBool::new(false));
    let watchdog_done = Arc::clone(&done);
    let watchdog = thread::spawn(move || {
        let deadline = Instant::now() + WATCHDOG_WAIT;
        while !watchdog_done.load(Ordering::Acquire) {
            if Instant::now() >= deadline {
                // Only this exact helper's own CI process; no log can block it.
                std::process::exit(124);
            }
            thread::sleep(Duration::from_millis(10));
        }
    });
    if run_child().is_err() {
        std::process::exit(70);
    }
    done.store(true, Ordering::Release);
    assert!(
        watchdog.join().is_ok(),
        "Code0NativeRenameProbe watchdog failed"
    );
}

fn run_child() -> ProbeResult<()> {
    let arm = std::env::var_os(ARM_KEY)
        .as_deref()
        .and_then(Arm::from_label)
        .ok_or(ProbeFailure::Failed)?;
    let base = PathBuf::from(std::env::var_os(BASE_KEY).ok_or(ProbeFailure::Failed)?);
    let layout = Layout::new(&base, arm);
    validate_layout(&layout)?;
    require(
        fs::canonicalize(std::env::current_dir().map_err(failed)?).map_err(failed)? == layout.cwd,
    )?;
    let record = observe_arm(arm, &layout)?;
    // observe_arm's source, original lease and guards have all closed normally.
    write_record(&layout.marker, &record)
}

fn same_identity(
    result: Result<SnapshotFileIdentity, Code0StoreError>,
    expected: SnapshotFileIdentity,
) -> Observation {
    match result {
        Ok(identity) => Observation::from_bool(identity == expected),
        Err(_) => Observation::Unknown,
    }
}

fn target_identity(path: &Path, expected: SnapshotFileIdentity) -> Observation {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Observation::No,
        Err(_) => Observation::Unknown,
        Ok(_) => same_identity(file_identity_from_path(path), expected),
    }
}

fn observe_arm(arm: Arm, layout: &Layout) -> ProbeResult<Record> {
    let lease = Code0ProfileLease::acquire(&layout.root).map_err(failed)?;
    let (mut record, source) = lease
        .with_root_handle(&layout.root, |root, _lock| -> ProbeResult<(Record, File)> {
            let root_identity = file_identity(root).map_err(failed)?;
            validate_code0_handle(root, true).map_err(failed)?;
            let (source, _) = open_owned_file_captured(
                &layout.root.join("snapshot-candidate-0.tmp"),
                GENERIC_READ | GENERIC_WRITE | DELETE | READ_CONTROL,
                CREATE_NEW,
                false,
            );
            let source = source.map_err(failed)?;
            object_is_single_regular_file(source.as_raw_handle()).map_err(failed)?;
            let mut source = unsafe { File::from_raw_handle(source.into_raw_handle()) };
            source
                .write_all(b"synthetic-native-rename-evidence")
                .map_err(failed)?;
            source.sync_all().map_err(failed)?;
            let source_identity = file_identity(source.as_raw_handle()).map_err(failed)?;
            let result = match arm {
                Arm::A => original_win32(source.as_raw_handle(), root)?,
                Arm::B => alternate_win32(source.as_raw_handle(), null_mut(), false)?,
                Arm::E => alternate_win32(source.as_raw_handle(), root, true)?,
                Arm::C | Arm::D => native_rename(
                    source.as_raw_handle(),
                    if arm == Arm::C { root } else { null_mut() },
                )?,
            };
            let mut record = Record::new(arm, result);
            record.source_identity_stable =
                same_identity(file_identity(source.as_raw_handle()), source_identity);
            record.root_identity_stable = same_identity(file_identity(root), root_identity);
            record.original_parent_target_matches =
                target_identity(&layout.root.join(LEAF), source_identity);
            record.unrelated_cwd_target_matches =
                target_identity(&layout.cwd.join(LEAF), source_identity);
            Ok((record, source))
        })
        .map_err(failed)??;
    record.authority_valid_after = Observation::from_bool(lease.is_held());
    require(
        record.source_identity_stable == Observation::Yes
            && record.root_identity_stable == Observation::Yes
            && record.authority_valid_after == Observation::Yes,
    )?;
    drop(source);
    lease.release();
    drop(lease);
    record.complete = true;
    Ok(record)
}

fn original_win32(source: HANDLE, root: HANDLE) -> ProbeResult<ApiResult> {
    let observer = Code0SnapshotTestGate::new();
    let result = rename_handle_observed(
        source,
        root,
        LEAF,
        Some(&observer),
        DiagnosticPhase::CandidateRename,
    );
    let summary = observer.inner.diagnostics.lock().map_err(failed)?;
    require(summary.used == 1)?;
    let record = summary.records[0].ok_or(ProbeFailure::Failed)?;
    require(record.phase == DiagnosticPhase::CandidateRename && !record.handled)?;
    match record.status {
        DiagnosticStatus::Success if result.is_ok() => Ok(ApiResult::Win32 {
            succeeded: true,
            error: Win32Error::NotApplicable,
        }),
        DiagnosticStatus::Win32Error(error) if result.is_err() => Ok(ApiResult::Win32 {
            succeeded: false,
            error: Win32Error::Code(error),
        }),
        _ => Err(ProbeFailure::Failed),
    }
}

struct RenameBuffer {
    storage: Box<[u128]>,
    submitted: u32,
    name_bytes: u32,
}

impl RenameBuffer {
    fn new(offset: usize, terminated: bool) -> ProbeResult<Self> {
        let name = LEAF.encode_utf16().collect::<Vec<_>>();
        let name_bytes = name
            .len()
            .checked_mul(std::mem::size_of::<u16>())
            .ok_or(ProbeFailure::Failed)?;
        let submitted = offset
            .checked_add(name_bytes)
            .and_then(|length| length.checked_add(if terminated { 2 } else { 0 }))
            .ok_or(ProbeFailure::Failed)?;
        let mut storage =
            vec![0u128; submitted.div_ceil(std::mem::size_of::<u128>())].into_boxed_slice();
        unsafe {
            std::ptr::copy_nonoverlapping(
                name.as_ptr(),
                storage.as_mut_ptr().cast::<u8>().add(offset).cast::<u16>(),
                name.len(),
            );
        }
        Ok(Self {
            storage,
            submitted: u32::try_from(submitted).map_err(failed)?,
            name_bytes: u32::try_from(name_bytes).map_err(failed)?,
        })
    }
}

fn alternate_win32(source: HANDLE, root: HANDLE, terminated: bool) -> ProbeResult<ApiResult> {
    let mut buffer =
        RenameBuffer::new(std::mem::offset_of!(FILE_RENAME_INFO, FileName), terminated)?;
    let info = buffer.storage.as_mut_ptr().cast::<FILE_RENAME_INFO>();
    unsafe {
        (*info).Anonymous.ReplaceIfExists = true;
        (*info).RootDirectory = root;
        (*info).FileNameLength = buffer.name_bytes;
    }
    let succeeded = unsafe {
        SetFileInformationByHandle(source, FileRenameInfo, info.cast(), buffer.submitted)
    };
    let error = if succeeded == 0 {
        Win32Error::Code(unsafe { GetLastError() })
    } else {
        Win32Error::NotApplicable
    };
    Ok(ApiResult::Win32 {
        succeeded: succeeded != 0,
        error,
    })
}

fn native_rename(source: HANDLE, root: HANDLE) -> ProbeResult<ApiResult> {
    let mut buffer = RenameBuffer::new(
        std::mem::offset_of!(FILE_RENAME_INFORMATION, FileName),
        false,
    )?;
    let info = buffer
        .storage
        .as_mut_ptr()
        .cast::<FILE_RENAME_INFORMATION>();
    let mut io_status = Box::new(IO_STATUS_BLOCK {
        Anonymous: IO_STATUS_BLOCK_0 {
            Status: UNWRITTEN_STATUS,
        },
        Information: 0,
    });
    unsafe {
        (*info).Anonymous.ReplaceIfExists = true;
        (*info).RootDirectory = root;
        (*info).FileNameLength = buffer.name_bytes;
    }
    let status = unsafe {
        NtSetInformationFile(
            source,
            &mut *io_status,
            info.cast(),
            buffer.submitted,
            FileRenameInformation,
        )
    };
    if status > 0 {
        // Includes STATUS_PENDING (259) and any unknown nonfinal return. Do not
        // log, write a marker, unwind, or issue another native call. The boxed
        // buffer/IOSB and caller's source/lease/guards live until process exit.
        std::process::exit(71);
    }
    let completion = unsafe { io_status.Anonymous.Status };
    if completion > 0 {
        // The same lifetime rule applies to an unexpected nonfinal IOSB.
        std::process::exit(71);
    }
    Ok(ApiResult::Native {
        status,
        io_status: if completion == UNWRITTEN_STATUS {
            IoStatus::Unknown
        } else {
            IoStatus::Status(completion)
        },
    })
}
