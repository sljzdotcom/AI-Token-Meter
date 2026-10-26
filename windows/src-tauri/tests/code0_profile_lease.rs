#![cfg(windows)]

use std::fs;
use std::io;
use std::os::windows::io::AsRawHandle;
use std::process::{Child, Command, ExitStatus};
use std::thread::sleep;
use std::time::{Duration, Instant};

use ai_token_meter_windows::platform::windows::code0_profile_lease::{
    Code0LeaseError, Code0ProfileLease,
};
use tempfile::tempdir;
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
};

const CHILD_ROOT: &str = "AI_METER_REQ004_LEASE_ROOT";
const CHILD_READY: &str = "AI_METER_REQ004_LEASE_READY";
const CHILD_RELEASE: &str = "AI_METER_REQ004_LEASE_RELEASE";

struct ControlledLeaseChild {
    child: Child,
    release_marker: std::path::PathBuf,
    reaped: bool,
}

impl ControlledLeaseChild {
    fn release_and_wait(&mut self, timeout: Duration) -> io::Result<ExitStatus> {
        fs::write(&self.release_marker, b"release")?;
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = self.child.try_wait()? {
                self.reaped = true;
                return Ok(status);
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "controlled lease child did not exit before deadline",
                ));
            }
            sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for ControlledLeaseChild {
    fn drop(&mut self) {
        if self.reaped {
            return;
        }
        if let Err(error) = fs::write(&self.release_marker, b"release") {
            eprintln!("controlled lease child release request failed: {error}");
        }
        let deadline = Instant::now() + Duration::from_secs(14);
        while Instant::now() < deadline {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    self.reaped = true;
                    if !status.success() {
                        eprintln!("controlled lease child exited unsuccessfully: {status}");
                    }
                    return;
                }
                Err(error) => {
                    eprintln!("controlled lease child reap status unconfirmed: {error}");
                    return;
                }
                Ok(None) => sleep(Duration::from_millis(10)),
            }
        }
        eprintln!("controlled lease child exit/reap remains unconfirmed after bounded wait");
    }
}

fn file_identity(path: &std::path::Path) -> io::Result<(u32, u64)> {
    let file = fs::File::open(path)?;
    let mut information = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    let succeeded =
        unsafe { GetFileInformationByHandle(file.as_raw_handle(), information.as_mut_ptr()) };
    if succeeded == 0 {
        return Err(io::Error::last_os_error());
    }
    let information = unsafe { information.assume_init() };
    let file_index =
        (u64::from(information.nFileIndexHigh) << 32) | u64::from(information.nFileIndexLow);
    if file_index == 0 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Windows did not provide a stable nonzero lock file index",
        ));
    }
    Ok((information.dwVolumeSerialNumber, file_index))
}

#[test]
fn second_process_is_busy_until_held_lease_is_released_and_lock_remains() {
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        child_holds_lease_until_released(root.into());
        return;
    }

    let temporary = tempdir().expect("isolated temporary directory");
    let root = temporary.path().join("Code0");
    let ready = temporary.path().join("ready");
    let release = temporary.path().join("release");
    let child = Command::new(std::env::current_exe().expect("test executable"))
        .arg("--exact")
        .arg("second_process_is_busy_until_held_lease_is_released_and_lock_remains")
        .arg("--nocapture")
        .env(CHILD_ROOT, &root)
        .env(CHILD_READY, &ready)
        .env(CHILD_RELEASE, &release)
        .spawn()
        .expect("start only the controlled lease child");
    let mut child = ControlledLeaseChild {
        child,
        release_marker: release,
        reaped: false,
    };
    let lock = root.join("profile.lock");
    let observations = (|| {
        wait_for_child_marker(&mut child, &ready, Duration::from_secs(8))?;
        let before = read_identity_marker(&ready)?;
        let busy = matches!(
            Code0ProfileLease::acquire(&root),
            Err(Code0LeaseError::Busy)
        );
        Ok::<_, String>((before, busy))
    })();

    let status = child
        .release_and_wait(Duration::from_secs(8))
        .expect("controlled child exits after release");
    assert!(status.success(), "controlled child succeeds: {status}");

    let (before, busy) = observations.expect("controlled child acquired the profile lease");
    assert!(
        busy,
        "the competing process must observe an exclusive lease"
    );
    assert!(
        lock.exists(),
        "normal release must preserve the stable lock file"
    );
    assert_eq!(
        file_identity(&lock).expect("lock identity after release"),
        before
    );
    assert!(Code0ProfileLease::acquire(&root).is_ok());
}

fn child_holds_lease_until_released(root: std::path::PathBuf) {
    let ready = std::path::PathBuf::from(std::env::var_os(CHILD_READY).expect("ready path"));
    let release = std::path::PathBuf::from(std::env::var_os(CHILD_RELEASE).expect("release path"));
    let lease = Code0ProfileLease::acquire(&root).expect("child owns the profile lease");
    let identity = lease
        .lock_identity_for_test()
        .expect("read identity from the already-held lock handle");
    fs::write(
        ready,
        format!("{}:{}", identity.volume_serial_number, identity.file_index),
    )
    .expect("signal ready with held-handle identity");

    let deadline = Instant::now() + Duration::from_secs(12);
    while !release.exists() && Instant::now() < deadline {
        sleep(Duration::from_millis(10));
    }
    assert!(release.exists(), "parent must release the controlled child");
    lease.release();
}

fn read_identity_marker(path: &std::path::Path) -> Result<(u32, u64), String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let (volume, index) = content
        .split_once(':')
        .ok_or_else(|| "child identity marker is malformed".to_owned())?;
    let volume = volume
        .parse()
        .map_err(|error: std::num::ParseIntError| error.to_string())?;
    let index = index
        .parse()
        .map_err(|error: std::num::ParseIntError| error.to_string())?;
    Ok((volume, index))
}

fn wait_for_child_marker(
    child: &mut ControlledLeaseChild,
    marker: &std::path::Path,
    timeout: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while !marker.exists() && Instant::now() < deadline {
        match child.child.try_wait() {
            Ok(Some(status)) => {
                return Err(format!(
                    "controlled child exited before lease ready: {status}"
                ));
            }
            Ok(None) => {}
            Err(error) => return Err(format!("poll controlled child: {error}")),
        }
        sleep(Duration::from_millis(10));
    }
    if marker.exists() {
        Ok(())
    } else {
        Err("controlled child did not acquire within the bounded wait".to_owned())
    }
}

#[test]
fn cloned_authority_release_revokes_every_store_capability() {
    let temporary = tempdir().expect("isolated temporary directory");
    let root = temporary.path().join("Code0");
    let lease = Code0ProfileLease::acquire(&root).expect("acquire profile lease");
    let clone = lease.clone();

    assert!(lease.is_held());
    assert!(clone.is_bound_to(&root));
    lease.release();
    assert!(!clone.is_held());
    assert!(!clone.is_bound_to(&root));
}

#[test]
fn mismatched_root_is_not_authorized() {
    let temporary = tempdir().expect("isolated temporary directory");
    let root = temporary.path().join("Code0");
    let other_root = temporary.path().join("other-Code0");
    let lease = Code0ProfileLease::acquire(&root).expect("acquire profile lease");

    assert!(lease.is_bound_to(&root));
    assert!(!lease.is_bound_to(&other_root));
}

#[test]
fn held_lock_cannot_be_replaced_or_its_authority_is_invalidated() {
    let temporary = tempdir().expect("isolated temporary directory");
    let root = temporary.path().join("Code0");
    let lease = Code0ProfileLease::acquire(&root).expect("acquire profile lease");
    let lock = root.join("profile.lock");
    let renamed_lock = root.join("profile.lock.replaced");
    let lock_rename = fs::rename(&lock, &renamed_lock);
    if lock_rename.is_ok() {
        assert!(
            !lease.is_bound_to(&root),
            "moving the held lock must invalidate the old authority"
        );
    } else {
        assert!(lease.is_bound_to(&root));
    }
}

#[test]
fn held_profile_root_cannot_be_replaced_or_its_authority_is_invalidated() {
    let temporary = tempdir().expect("isolated temporary directory");
    let root = temporary.path().join("Code0");
    let lease = Code0ProfileLease::acquire(&root).expect("acquire profile lease");
    let moved_root = temporary.path().join("Code0.replaced");
    let root_rename = fs::rename(&root, &moved_root);
    if root_rename.is_ok() {
        assert!(
            !lease.is_bound_to(&root),
            "moving the held profile root must invalidate the old authority"
        );
    } else {
        assert!(lease.is_bound_to(&root));
    }
}
