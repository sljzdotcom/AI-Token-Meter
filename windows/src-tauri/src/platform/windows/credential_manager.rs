use std::ffi::c_void;
use std::ptr;

use async_trait::async_trait;
use windows_sys::Win32::Foundation::{ERROR_NOT_FOUND, GetLastError};
use windows_sys::Win32::Security::Credentials::{
    CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredDeleteW, CredFree, CredReadW,
    CredWriteW,
};
use zeroize::Zeroizing;

use crate::security::{CredentialAccount, CredentialStore, CredentialStoreError, SecretString};

const PRODUCTION_TARGET: &str = "AI Token Meter/DeepSeek API Key";

pub struct WindowsCredentialManager {
    target_name: String,
}

impl WindowsCredentialManager {
    pub fn new() -> Self {
        Self {
            target_name: PRODUCTION_TARGET.to_owned(),
        }
    }

    #[cfg(debug_assertions)]
    pub fn for_test_target(target_name: String) -> Self {
        Self { target_name }
    }

    fn target_utf16(&self) -> Vec<u16> {
        self.target_name.encode_utf16().chain(Some(0)).collect()
    }
}

impl Default for WindowsCredentialManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CredentialStore for WindowsCredentialManager {
    async fn read(
        &self,
        _account: CredentialAccount,
    ) -> Result<Option<SecretString>, CredentialStoreError> {
        let target = self.target_utf16();
        let mut raw = ptr::null_mut::<CREDENTIALW>();
        // SAFETY: target is a live NUL-terminated UTF-16 buffer and `raw` is a
        // valid out pointer. A successful allocation is released with CredFree.
        let found = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut raw) };
        if found == 0 {
            // SAFETY: GetLastError has no preconditions and is read immediately
            // after the failed credential operation on this thread.
            let code = unsafe { GetLastError() };
            return if code == ERROR_NOT_FOUND {
                Ok(None)
            } else {
                Err(CredentialStoreError::Platform(code))
            };
        }
        if raw.is_null() {
            return Err(CredentialStoreError::InvalidData);
        }

        // SAFETY: CredReadW returned a valid CREDENTIALW allocation. The blob
        // slice is bounded by CredentialBlobSize before the allocation is freed.
        let bytes = unsafe {
            let credential = &*raw;
            std::slice::from_raw_parts(
                credential.CredentialBlob,
                credential.CredentialBlobSize as usize,
            )
        };
        let value = String::from_utf8(bytes.to_vec())
            .map(SecretString::new)
            .map_err(|_| CredentialStoreError::InvalidData);
        // SAFETY: raw is the allocation returned by CredReadW and is freed once.
        unsafe { CredFree(raw.cast::<c_void>()) };
        value.map(Some)
    }

    async fn replace_verified(
        &self,
        _account: CredentialAccount,
        secret: SecretString,
    ) -> Result<(), CredentialStoreError> {
        let mut target = self.target_utf16();
        let mut blob = Zeroizing::new(secret.expose().as_bytes().to_vec());
        let credential = CREDENTIALW {
            Flags: 0,
            Type: CRED_TYPE_GENERIC,
            TargetName: target.as_mut_ptr(),
            Comment: ptr::null_mut(),
            LastWritten: Default::default(),
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: ptr::null_mut(),
            TargetAlias: ptr::null_mut(),
            UserName: ptr::null_mut(),
        };
        // SAFETY: all pointers in credential are either null or reference live
        // mutable buffers for the duration of this synchronous call.
        let written = unsafe { CredWriteW(&credential, 0) };
        if written == 0 {
            // SAFETY: read immediately after the failed credential operation.
            Err(CredentialStoreError::Platform(unsafe { GetLastError() }))
        } else {
            Ok(())
        }
    }

    async fn delete(&self, _account: CredentialAccount) -> Result<(), CredentialStoreError> {
        let target = self.target_utf16();
        // SAFETY: target is a live NUL-terminated UTF-16 buffer.
        let deleted = unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
        if deleted == 0 {
            // SAFETY: read immediately after the failed credential operation.
            let code = unsafe { GetLastError() };
            if code != ERROR_NOT_FOUND {
                return Err(CredentialStoreError::Platform(code));
            }
        }
        Ok(())
    }
}
