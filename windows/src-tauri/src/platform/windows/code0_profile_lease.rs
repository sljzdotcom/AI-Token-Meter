use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Result of attempting to own the single Code0 WebView2 profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code0LeaseError {
    Busy,
    Unavailable,
}

/// Stable file identity queried from the lease's already-held lock handle.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Code0ProfileIdentity {
    pub volume_serial_number: u32,
    pub file_index: u64,
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileIdentity {
    volume: u64,
    id: [u8; 16],
}

#[cfg(windows)]
enum LeaseState {
    Held {
        ancestors: Code0PathGuard,
        root: std::os::windows::io::OwnedHandle,
        lock: std::os::windows::io::OwnedHandle,
    },
    Released,
    Invalidated {
        _ancestors: Code0PathGuard,
        _root: std::os::windows::io::OwnedHandle,
        _lock: std::os::windows::io::OwnedHandle,
    },
}

#[cfg(not(windows))]
enum LeaseState {
    Released,
    Invalidated,
}

struct LeaseAuthority {
    root_path: PathBuf,
    #[cfg(windows)]
    root_identity: FileIdentity,
    #[cfg(windows)]
    lock_identity: FileIdentity,
    state: Mutex<LeaseState>,
    sid_failure: std::sync::atomic::AtomicBool,
}

/// Cloneable handle to the one OS-backed Code0 profile authority.
///
/// Every clone shares the original root and lock handles. Invalidation and
/// release are one-way state transitions, and store operations hold the state
/// mutex through their file transaction so a release cannot race a write.
#[derive(Clone)]
pub struct Code0ProfileLease {
    authority: Arc<LeaseAuthority>,
}

#[cfg(windows)]
pub(crate) fn with_code0_security<T>(
    directory: bool,
    operation: impl FnOnce(*const windows_sys::Win32::Security::SECURITY_ATTRIBUTES) -> T,
) -> Result<T, Code0LeaseError> {
    let user_sid = windows_impl::current_user_sid()?;
    let descriptor = windows_impl::security_descriptor(&user_sid, directory)?;
    let attributes = windows_impl::security_attributes(&descriptor);
    Ok(operation(&attributes))
}

#[cfg(windows)]
pub(crate) fn validate_code0_handle(
    handle: windows_sys::Win32::Foundation::HANDLE,
    directory: bool,
) -> Result<(), Code0LeaseError> {
    windows_impl::validate_secure_object(handle, directory, &windows_impl::current_user_sid()?)
}

/// Retained no-follow guards for every existing component of a cold-read path.
/// FILE_LIST_DIRECTORY makes their restrictive sharing an actual access
/// reservation; they never create a directory. Short identity probes below
/// remain metadata-only and do not replace these retained guards.
#[cfg(windows)]
pub(crate) struct Code0PathGuard {
    components: Vec<(PathBuf, std::os::windows::io::OwnedHandle, FileIdentity)>,
    complete: bool,
}

#[cfg(windows)]
impl Code0PathGuard {
    pub(crate) fn root_handle(&self) -> Option<windows_sys::Win32::Foundation::HANDLE> {
        use std::os::windows::io::AsRawHandle;
        if self.complete {
            self.components
                .last()
                .map(|(_, handle, _)| handle.as_raw_handle())
        } else {
            None
        }
    }

    pub(crate) fn validate(&self) -> Result<(), Code0LeaseError> {
        use std::os::windows::io::AsRawHandle;
        for (path, handle, expected) in &self.components {
            if windows_impl::handle_identity(handle.as_raw_handle())? != *expected
                || windows_impl::path_identity(path, true)? != *expected
            {
                return Err(Code0LeaseError::Unavailable);
            }
        }
        Ok(())
    }
}

#[cfg(windows)]
pub(crate) fn guard_code0_read_path(root: &Path) -> Result<Code0PathGuard, Code0LeaseError> {
    windows_impl::guard_path(root, true)
}

impl Code0ProfileLease {
    /// Create/validate the fixed Code0 directory and acquire its exclusive
    /// share-mode lock. The stable lock file is never truncated or removed.
    pub fn acquire(root: impl AsRef<Path>) -> Result<Self, Code0LeaseError> {
        #[cfg(windows)]
        {
            windows_impl::acquire(root.as_ref())
        }
        #[cfg(not(windows))]
        {
            let _ = root;
            Err(Code0LeaseError::Unavailable)
        }
    }

    /// Explicit release closes the shared OS handles and revokes all clones.
    pub fn release(&self) {
        if let Ok(mut state) = self.authority.state.lock() {
            *state = LeaseState::Released;
        }
    }

    /// Permanently revoke every clone after an identity or handle invariant fails.
    pub fn invalidate(&self) {
        if let Ok(mut state) = self.authority.state.lock() {
            Self::invalidate_state(&mut state);
        }
    }

    fn invalidate_state(state: &mut LeaseState) {
        #[cfg(windows)]
        {
            if matches!(*state, LeaseState::Held { .. }) {
                let previous = std::mem::replace(state, LeaseState::Released);
                if let LeaseState::Held {
                    ancestors,
                    root,
                    lock,
                } = previous
                {
                    *state = LeaseState::Invalidated {
                        _ancestors: ancestors,
                        _root: root,
                        _lock: lock,
                    };
                }
            }
        }
        #[cfg(not(windows))]
        {
            if !matches!(*state, LeaseState::Released) {
                *state = LeaseState::Invalidated;
            }
        }
    }

    /// Inject one unavailable SID lookup without disclosing or fabricating SID bytes.
    #[doc(hidden)]
    pub fn fail_next_sid_lookup_for_test(&self) {
        self.authority
            .sid_failure
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Whether this authority still holds its original root and lock identities.
    pub fn is_held(&self) -> bool {
        self.with_root_handle(&self.authority.root_path, |_, _| ())
            .is_ok()
    }

    /// Check both the supplied path binding and the held OS identities.
    pub fn is_bound_to(&self, root: &Path) -> bool {
        self.authority.root_path == root && self.is_held()
    }

    /// Narrow test observation from the original held lock handle.
    #[doc(hidden)]
    pub fn lock_identity_for_test(&self) -> Result<Code0ProfileIdentity, Code0LeaseError> {
        #[cfg(windows)]
        {
            self.with_root_handle(&self.authority.root_path, |root, lock| {
                let _ = root;
                windows_impl::legacy_identity(lock)
            })?
        }
        #[cfg(not(windows))]
        {
            Err(Code0LeaseError::Unavailable)
        }
    }

    /// Run an operation while holding the shared authority lock. The callback
    /// receives the original root handle; it must not reacquire authority by
    /// path. A failed identity check invalidates this authority permanently.
    #[cfg(windows)]
    pub(crate) fn with_root_handle<T>(
        &self,
        root_path: &Path,
        operation: impl FnOnce(
            windows_sys::Win32::Foundation::HANDLE,
            windows_sys::Win32::Foundation::HANDLE,
        ) -> T,
    ) -> Result<T, Code0LeaseError> {
        use std::os::windows::io::AsRawHandle;

        if root_path != self.authority.root_path {
            return Err(Code0LeaseError::Unavailable);
        }
        let mut state = self
            .authority
            .state
            .lock()
            .map_err(|_| Code0LeaseError::Unavailable)?;
        let user_sid = if self
            .authority
            .sid_failure
            .swap(false, std::sync::atomic::Ordering::Relaxed)
        {
            Err(Code0LeaseError::Unavailable)
        } else {
            windows_impl::current_user_sid()
        };
        let valid = match (&*state, user_sid.as_ref()) {
            (
                LeaseState::Held {
                    ancestors,
                    root,
                    lock,
                },
                Ok(user_sid),
            ) => {
                ancestors.validate().is_ok()
                    && windows_impl::handle_identity(root.as_raw_handle())
                        .is_ok_and(|identity| identity == self.authority.root_identity)
                    && windows_impl::handle_identity(lock.as_raw_handle())
                        .is_ok_and(|identity| identity == self.authority.lock_identity)
                    && windows_impl::path_identity(&self.authority.root_path, true)
                        .is_ok_and(|identity| identity == self.authority.root_identity)
                    && windows_impl::path_identity(
                        &self.authority.root_path.join("profile.lock"),
                        false,
                    )
                    .is_ok_and(|identity| identity == self.authority.lock_identity)
                    && windows_impl::validate_secure_object(root.as_raw_handle(), true, user_sid)
                        .is_ok()
                    && windows_impl::validate_secure_object(lock.as_raw_handle(), false, user_sid)
                        .is_ok()
            }
            _ => false,
        };
        if !valid {
            Self::invalidate_state(&mut state);
            return Err(Code0LeaseError::Unavailable);
        }
        let LeaseState::Held { root, lock, .. } = &*state else {
            return Err(Code0LeaseError::Unavailable);
        };
        Ok(operation(root.as_raw_handle(), lock.as_raw_handle()))
    }

    #[cfg(not(windows))]
    pub(crate) fn with_root_handle<T>(
        &self,
        _root_path: &Path,
        _operation: impl FnOnce(*mut std::ffi::c_void, *mut std::ffi::c_void) -> T,
    ) -> Result<T, Code0LeaseError> {
        Err(Code0LeaseError::Unavailable)
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::{
        Code0LeaseError, Code0PathGuard, Code0ProfileIdentity, FileIdentity, LeaseAuthority,
        LeaseState,
    };
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use std::path::{Component, Path};
    use std::ptr::{null, null_mut};
    use std::sync::{Arc, Mutex};

    use windows_sys::Win32::Foundation::{
        ERROR_ALREADY_EXISTS, ERROR_FILE_EXISTS, ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND,
        ERROR_SHARING_VIOLATION, GENERIC_READ, GENERIC_WRITE, GetLastError, INVALID_HANDLE_VALUE,
        LocalFree,
    };
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo, SE_FILE_OBJECT,
    };
    use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
    use windows_sys::Win32::Security::{
        ACCESS_ALLOWED_ACE, ACL_SIZE_INFORMATION, DACL_SECURITY_INFORMATION, EqualSid, GetAce,
        GetAclInformation, GetSecurityDescriptorControl, GetSecurityDescriptorDacl,
        OWNER_SECURITY_INFORMATION, SE_DACL_PROTECTED, TOKEN_QUERY, TOKEN_USER, WinLocalSystemSid,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, CreateDirectoryW, CreateFileW, FILE_ALL_ACCESS,
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_ATTRIBUTE_TAG_INFO, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_ID_INFO, FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE,
        FILE_STANDARD_INFO, FileAttributeTagInfo, FileIdInfo, FileStandardInfo,
        GetFileInformationByHandle, GetFileInformationByHandleEx, OPEN_ALWAYS, OPEN_EXISTING,
        READ_CONTROL,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    const DIR_INHERIT_FLAGS: u8 = 0x03;
    const ACE_ALLOW: u8 = 0;

    pub(super) struct LocalAlloc(*mut c_void);
    impl Drop for LocalAlloc {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe { LocalFree(self.0) };
            }
        }
    }

    fn wide(path: &Path) -> Result<Vec<u16>, Code0LeaseError> {
        if !path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
        {
            return Err(Code0LeaseError::Unavailable);
        }
        Ok(path.as_os_str().encode_wide().chain(Some(0)).collect())
    }

    fn last_error() -> u32 {
        unsafe { GetLastError() }
    }

    fn code_for_error(error: u32) -> Code0LeaseError {
        if error == ERROR_SHARING_VIOLATION {
            Code0LeaseError::Busy
        } else {
            Code0LeaseError::Unavailable
        }
    }

    fn open(
        path: &Path,
        access: u32,
        share: u32,
        disposition: u32,
        flags: u32,
        security: *const SECURITY_ATTRIBUTES,
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
            return Err(last_error());
        }
        Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
    }

    pub(super) fn current_user_sid() -> Result<Vec<u8>, Code0LeaseError> {
        let mut token = null_mut();
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
            return Err(Code0LeaseError::Unavailable);
        }
        let token = unsafe { OwnedHandle::from_raw_handle(token) };
        let mut length = 0u32;
        unsafe {
            windows_sys::Win32::Security::GetTokenInformation(
                token.as_raw_handle(),
                windows_sys::Win32::Security::TokenUser,
                null_mut(),
                0,
                &mut length,
            );
        }
        if length == 0 {
            return Err(Code0LeaseError::Unavailable);
        }
        // TOKEN_USER contains pointers; preserve its native alignment.
        let mut buffer = vec![0usize; (length as usize).div_ceil(std::mem::size_of::<usize>())];
        if unsafe {
            windows_sys::Win32::Security::GetTokenInformation(
                token.as_raw_handle(),
                windows_sys::Win32::Security::TokenUser,
                buffer.as_mut_ptr().cast(),
                length,
                &mut length,
            )
        } == 0
        {
            return Err(Code0LeaseError::Unavailable);
        }
        let token_user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
        sid_bytes(token_user.User.Sid)
    }

    fn sid_bytes(sid: *mut c_void) -> Result<Vec<u8>, Code0LeaseError> {
        if sid.is_null() {
            return Err(Code0LeaseError::Unavailable);
        }
        let length = unsafe { windows_sys::Win32::Security::GetLengthSid(sid) };
        if length == 0 || length > 256 {
            return Err(Code0LeaseError::Unavailable);
        }
        let bytes = unsafe { std::slice::from_raw_parts(sid.cast::<u8>(), length as usize) };
        Ok(bytes.to_vec())
    }

    fn system_sid() -> Result<Vec<u8>, Code0LeaseError> {
        let mut size = 68u32;
        let mut buffer = vec![0u8; size as usize];
        if unsafe {
            windows_sys::Win32::Security::CreateWellKnownSid(
                WinLocalSystemSid,
                null_mut(),
                buffer.as_mut_ptr().cast(),
                &mut size,
            )
        } == 0
        {
            return Err(Code0LeaseError::Unavailable);
        }
        buffer.truncate(size as usize);
        Ok(buffer)
    }

    fn sid_text(sid: *const u8) -> Result<String, Code0LeaseError> {
        let mut text = null_mut();
        if unsafe {
            windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW(
                sid.cast_mut().cast(),
                &mut text,
            )
        } == 0
        {
            return Err(Code0LeaseError::Unavailable);
        }
        let _allocation = LocalAlloc(text.cast());
        let mut length = 0usize;
        unsafe {
            while *text.add(length) != 0 {
                length += 1;
            }
            String::from_utf16(std::slice::from_raw_parts(text, length))
                .map_err(|_| Code0LeaseError::Unavailable)
        }
    }

    pub(super) fn security_descriptor(
        user_sid: &[u8],
        directory: bool,
    ) -> Result<LocalAlloc, Code0LeaseError> {
        let user = sid_text(user_sid.as_ptr())?;
        let dacl = if directory {
            format!("O:{user}D:P(A;OICI;FA;;;{user})(A;OICI;FA;;;SY)")
        } else {
            format!("O:{user}D:P(A;;FA;;;{user})(A;;FA;;;SY)")
        };
        let wide = dacl.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        let mut descriptor = null_mut();
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                wide.as_ptr(),
                1,
                &mut descriptor,
                null_mut(),
            )
        } == 0
        {
            return Err(Code0LeaseError::Unavailable);
        }
        Ok(LocalAlloc(descriptor.cast()))
    }

    pub(super) fn security_attributes(descriptor: &LocalAlloc) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: 0,
        }
    }

    fn ensure_directory(root: &Path, user_sid: &[u8]) -> Result<(), Code0LeaseError> {
        let root_wide = wide(root)?;
        let descriptor = security_descriptor(user_sid, true)?;
        let attributes = security_attributes(&descriptor);
        if unsafe { CreateDirectoryW(root_wide.as_ptr(), &attributes) } != 0 {
            return Ok(());
        }
        match last_error() {
            ERROR_ALREADY_EXISTS | ERROR_FILE_EXISTS => Ok(()),
            error => Err(code_for_error(error)),
        }
    }

    pub(super) fn guard_path(
        root: &Path,
        allow_missing: bool,
    ) -> Result<Code0PathGuard, Code0LeaseError> {
        wide(root)?;
        // UNC/device paths are outside this local fixed-profile authority.
        if !matches!(root.components().next(), Some(Component::Prefix(prefix)) if matches!(prefix.kind(), std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)))
        {
            return Err(Code0LeaseError::Unavailable);
        }
        let mut paths = root.ancestors().collect::<Vec<_>>();
        paths.reverse();
        let mut guard = Code0PathGuard {
            components: Vec::new(),
            complete: true,
        };
        for path in paths {
            let handle = match open(
                path,
                FILE_LIST_DIRECTORY
                    | FILE_READ_ATTRIBUTES
                    | if path == root { READ_CONTROL } else { 0 },
                FILE_SHARE_READ,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                null(),
            ) {
                Ok(handle) => handle,
                Err(ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND) if allow_missing => {
                    guard.complete = false;
                    guard.validate()?;
                    return Ok(guard);
                }
                Err(error) => return Err(code_for_error(error)),
            };
            let info = object_info(handle.as_raw_handle())?;
            if !info.is_directory {
                return Err(Code0LeaseError::Unavailable);
            }
            guard
                .components
                .push((path.to_owned(), handle, info.identity));
        }
        guard.validate()?;
        Ok(guard)
    }

    struct ObjectInfo {
        identity: FileIdentity,
        is_directory: bool,
    }

    fn object_info(handle: *mut c_void) -> Result<ObjectInfo, Code0LeaseError> {
        let mut tag = FILE_ATTRIBUTE_TAG_INFO::default();
        if unsafe {
            GetFileInformationByHandleEx(
                handle,
                FileAttributeTagInfo,
                (&mut tag as *mut FILE_ATTRIBUTE_TAG_INFO).cast(),
                std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
            )
        } == 0
            || tag.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        {
            return Err(Code0LeaseError::Unavailable);
        }
        let mut standard = FILE_STANDARD_INFO::default();
        if unsafe {
            GetFileInformationByHandleEx(
                handle,
                FileStandardInfo,
                (&mut standard as *mut FILE_STANDARD_INFO).cast(),
                std::mem::size_of::<FILE_STANDARD_INFO>() as u32,
            )
        } == 0
            || standard.DeletePending
        {
            return Err(Code0LeaseError::Unavailable);
        }
        let mut id = FILE_ID_INFO::default();
        if unsafe {
            GetFileInformationByHandleEx(
                handle,
                FileIdInfo,
                (&mut id as *mut FILE_ID_INFO).cast(),
                std::mem::size_of::<FILE_ID_INFO>() as u32,
            )
        } == 0
            || id.FileId.Identifier.iter().all(|byte| *byte == 0)
        {
            return Err(Code0LeaseError::Unavailable);
        }
        let is_directory = tag.FileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0;
        if standard.Directory != is_directory || (!is_directory && standard.NumberOfLinks != 1) {
            return Err(Code0LeaseError::Unavailable);
        }
        Ok(ObjectInfo {
            identity: FileIdentity {
                volume: id.VolumeSerialNumber,
                id: id.FileId.Identifier,
            },
            is_directory,
        })
    }

    pub(super) fn validate_secure_object(
        handle: *mut c_void,
        directory: bool,
        user_sid: &[u8],
    ) -> Result<(), Code0LeaseError> {
        if object_info(handle)?.is_directory != directory {
            return Err(Code0LeaseError::Unavailable);
        }
        let mut owner = null_mut();
        let mut dacl = null_mut();
        let mut descriptor = null_mut();
        let status = unsafe {
            GetSecurityInfo(
                handle,
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &mut owner,
                null_mut(),
                &mut dacl,
                null_mut(),
                &mut descriptor,
            )
        };
        if status != 0 || descriptor.is_null() {
            return Err(Code0LeaseError::Unavailable);
        }
        let allocation = LocalAlloc(descriptor.cast());
        if owner.is_null()
            || user_sid.len() < 8
            || unsafe {
                windows_sys::Win32::Security::IsValidSid(user_sid.as_ptr().cast_mut().cast())
            } == 0
        {
            return Err(Code0LeaseError::Unavailable);
        }
        let expected_user = user_sid;
        if unsafe { EqualSid(owner, expected_user.as_ptr().cast_mut().cast()) } == 0 {
            return Err(Code0LeaseError::Unavailable);
        }
        let mut control = 0u16;
        let mut revision = 0u32;
        if unsafe { GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) } == 0
            || control & SE_DACL_PROTECTED == 0
        {
            return Err(Code0LeaseError::Unavailable);
        }
        let mut present = 0;
        let mut defaulted = 0;
        let mut actual_dacl = null_mut();
        if unsafe {
            GetSecurityDescriptorDacl(descriptor, &mut present, &mut actual_dacl, &mut defaulted)
        } == 0
            || present == 0
            || actual_dacl.is_null()
        {
            return Err(Code0LeaseError::Unavailable);
        }
        let mut size = ACL_SIZE_INFORMATION::default();
        if unsafe {
            GetAclInformation(
                actual_dacl,
                (&mut size as *mut ACL_SIZE_INFORMATION).cast(),
                std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
                windows_sys::Win32::Security::AclSizeInformation,
            )
        } == 0
            || size.AceCount != 2
        {
            return Err(Code0LeaseError::Unavailable);
        }
        let expected_system = system_sid()?;
        let mut seen_user = false;
        let mut seen_system = false;
        for index in 0..size.AceCount {
            let mut ace_pointer = null_mut();
            if unsafe { GetAce(actual_dacl, index, &mut ace_pointer) } == 0 || ace_pointer.is_null()
            {
                return Err(Code0LeaseError::Unavailable);
            }
            let ace = unsafe { &*ace_pointer.cast::<ACCESS_ALLOWED_ACE>() };
            let expected_flags = if directory { DIR_INHERIT_FLAGS } else { 0 };
            if ace.Header.AceType != ACE_ALLOW
                || ace.Header.AceFlags != expected_flags
                || ace.Mask != FILE_ALL_ACCESS
            {
                return Err(Code0LeaseError::Unavailable);
            }
            let ace_sid = std::ptr::addr_of!(ace.SidStart).cast_mut().cast();
            if unsafe { EqualSid(ace_sid, expected_user.as_ptr().cast_mut().cast()) } != 0 {
                if seen_user {
                    return Err(Code0LeaseError::Unavailable);
                }
                seen_user = true;
            } else if unsafe { EqualSid(ace_sid, expected_system.as_ptr().cast_mut().cast()) } != 0
            {
                if seen_system {
                    return Err(Code0LeaseError::Unavailable);
                }
                seen_system = true;
            } else {
                return Err(Code0LeaseError::Unavailable);
            }
        }
        drop(allocation);
        if seen_user && seen_system {
            Ok(())
        } else {
            Err(Code0LeaseError::Unavailable)
        }
    }

    pub(super) fn handle_identity(handle: *mut c_void) -> Result<FileIdentity, Code0LeaseError> {
        Ok(object_info(handle)?.identity)
    }

    pub(super) fn path_identity(
        path: &Path,
        directory: bool,
    ) -> Result<FileIdentity, Code0LeaseError> {
        let handle = open(
            path,
            FILE_READ_ATTRIBUTES | READ_CONTROL,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT
                | if directory {
                    FILE_FLAG_BACKUP_SEMANTICS
                } else {
                    FILE_ATTRIBUTE_NORMAL
                },
            null(),
        )
        .map_err(code_for_error)?;
        Ok(object_info(handle.as_raw_handle())?.identity)
    }

    pub(super) fn legacy_identity(
        handle: *mut c_void,
    ) -> Result<Code0ProfileIdentity, Code0LeaseError> {
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        if unsafe { GetFileInformationByHandle(handle, &mut info) } == 0 {
            return Err(Code0LeaseError::Unavailable);
        }
        let file_index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
        if file_index == 0 {
            return Err(Code0LeaseError::Unavailable);
        }
        Ok(Code0ProfileIdentity {
            volume_serial_number: info.dwVolumeSerialNumber,
            file_index,
        })
    }

    pub(super) fn acquire(root_path: &Path) -> Result<super::Code0ProfileLease, Code0LeaseError> {
        wide(root_path)?;
        let parent = root_path.parent().ok_or(Code0LeaseError::Unavailable)?;
        let ancestors = guard_path(parent, false)?;
        let user_sid = current_user_sid()?;
        ensure_directory(root_path, &user_sid)?;
        let root = open(
            root_path,
            FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | READ_CONTROL,
            FILE_SHARE_READ,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            null(),
        )
        .map_err(code_for_error)?;
        validate_secure_object(root.as_raw_handle(), true, &user_sid)?;
        let root_identity = handle_identity(root.as_raw_handle())?;

        let lock_path = root_path.join("profile.lock");
        let descriptor = security_descriptor(&user_sid, false)?;
        let attributes = security_attributes(&descriptor);
        let lock = open(
            &lock_path,
            GENERIC_READ | GENERIC_WRITE | READ_CONTROL,
            FILE_SHARE_READ,
            OPEN_ALWAYS,
            FILE_FLAG_OPEN_REPARSE_POINT | FILE_ATTRIBUTE_NORMAL,
            &attributes,
        )
        .map_err(code_for_error)?;
        validate_secure_object(lock.as_raw_handle(), false, &user_sid)?;
        let lock_identity = handle_identity(lock.as_raw_handle())?;

        let authority = LeaseAuthority {
            root_path: root_path.to_owned(),
            root_identity,
            lock_identity,
            state: Mutex::new(LeaseState::Held {
                ancestors,
                root,
                lock,
            }),
            sid_failure: std::sync::atomic::AtomicBool::new(false),
        };
        let lease = super::Code0ProfileLease {
            authority: Arc::new(authority),
        };
        if !lease.is_held() {
            lease.invalidate();
            return Err(Code0LeaseError::Unavailable);
        }
        Ok(lease)
    }
}
