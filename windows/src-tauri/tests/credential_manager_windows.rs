#![cfg(windows)]

use std::time::{SystemTime, UNIX_EPOCH};

use ai_token_meter_windows::platform::windows::credential_manager::WindowsCredentialManager;
use ai_token_meter_windows::security::{CredentialAccount, CredentialStore, SecretString};

#[tokio::test]
async fn generic_credential_round_trips_in_an_isolated_test_target() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let manager = WindowsCredentialManager::for_test_target(format!(
        "AI Token Meter Tests/{}/{}",
        std::process::id(),
        suffix
    ));
    let account = CredentialAccount::DeepSeekApiKey;

    manager
        .replace_verified(account, SecretString::new("windows-test-value"))
        .await
        .expect("write credential");
    let restored = manager
        .read(account)
        .await
        .expect("read credential")
        .expect("credential exists");

    assert_eq!(restored.expose(), "windows-test-value");
    manager.delete(account).await.expect("delete credential");
    assert!(
        manager
            .read(account)
            .await
            .expect("read after delete")
            .is_none()
    );
}
