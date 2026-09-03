use std::mem::size_of;

use windows_sys::Win32::Foundation::{ERROR_CANCELLED, HWND};
use windows_sys::Win32::Security::Credentials::{
    CREDUI_FLAGS_ALWAYS_SHOW_UI, CREDUI_FLAGS_DO_NOT_PERSIST, CREDUI_FLAGS_EXCLUDE_CERTIFICATES,
    CREDUI_FLAGS_GENERIC_CREDENTIALS, CREDUI_INFOW, CredUIPromptForCredentialsW,
};
use zeroize::Zeroizing;

use crate::security::SecretString;

const PASSWORD_CAPACITY: usize = 512;
const USERNAME_CAPACITY: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialPromptError {
    Cancelled,
    Empty,
    InvalidText,
    Platform(u32),
}

pub fn prompt_deepseek_api_key(parent: HWND) -> Result<SecretString, CredentialPromptError> {
    let caption = wide("AI Token Meter · DeepSeek");
    let message = wide(
        "Paste the DeepSeek API Key into the Password field. It stays inside this protected Windows dialog.",
    );
    let target = wide("AI Token Meter/DeepSeek API Key");
    let mut username = Zeroizing::new(vec![0_u16; USERNAME_CAPACITY]);
    let default_username = wide("DeepSeek API Key");
    username[..default_username.len()].copy_from_slice(&default_username);
    let mut password = Zeroizing::new(vec![0_u16; PASSWORD_CAPACITY]);
    let mut save = 0;
    let prompt = CREDUI_INFOW {
        cbSize: size_of::<CREDUI_INFOW>() as u32,
        hwndParent: parent,
        pszMessageText: message.as_ptr(),
        pszCaptionText: caption.as_ptr(),
        hbmBanner: std::ptr::null_mut(),
    };
    let result = unsafe {
        CredUIPromptForCredentialsW(
            &prompt,
            target.as_ptr(),
            std::ptr::null(),
            0,
            username.as_mut_ptr(),
            username.len() as u32,
            password.as_mut_ptr(),
            password.len() as u32,
            &mut save,
            CREDUI_FLAGS_ALWAYS_SHOW_UI
                | CREDUI_FLAGS_DO_NOT_PERSIST
                | CREDUI_FLAGS_EXCLUDE_CERTIFICATES
                | CREDUI_FLAGS_GENERIC_CREDENTIALS,
        )
    };
    if result == ERROR_CANCELLED {
        return Err(CredentialPromptError::Cancelled);
    }
    if result != 0 {
        return Err(CredentialPromptError::Platform(result));
    }
    let length = password
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(password.len());
    if length == 0 {
        return Err(CredentialPromptError::Empty);
    }
    let value =
        String::from_utf16(&password[..length]).map_err(|_| CredentialPromptError::InvalidText)?;
    Ok(SecretString::new(value))
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
