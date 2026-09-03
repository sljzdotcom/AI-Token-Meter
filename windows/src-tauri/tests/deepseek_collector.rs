use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ai_token_meter_windows::accounts::deepseek::DeepSeekAccountService;
use ai_token_meter_windows::collectors::CollectionError;
use ai_token_meter_windows::collectors::deepseek::{
    DeepSeekBalanceClient, DeepSeekClientError, DeepSeekCollector,
};
use ai_token_meter_windows::domain::{ProviderId, UsageSnapshot, UsageStatus};
use ai_token_meter_windows::platform::windows::process::CancellationToken;
use ai_token_meter_windows::security::{
    CredentialAccount, CredentialStore, CredentialStoreError, SecretString,
};
use async_trait::async_trait;

#[tokio::test]
async fn missing_key_returns_setup_required_without_calling_the_network() {
    let store = Arc::new(FakeCredentialStore::new(None));
    let client = DeepSeekBalanceClient::for_endpoint(
        "http://127.0.0.1:9/user/balance",
        Duration::from_millis(20),
        16 * 1024,
    )
    .expect("test endpoint");
    let collector = DeepSeekCollector::new(store, client, 100.0);

    let snapshot = collector.collect(None, FETCHED_AT).await;

    assert_eq!(snapshot.provider_id, ProviderId::DeepSeek);
    assert_eq!(snapshot.status, UsageStatus::SetupRequired);
    assert_eq!(snapshot.used_ratio, None);
}

#[tokio::test]
async fn cancelled_balance_refresh_stops_before_reading_credentials_or_network() {
    let store = Arc::new(FakeCredentialStore::new(Some("stored-test-key")));
    let client = DeepSeekBalanceClient::for_endpoint(
        "http://127.0.0.1:9/user/balance",
        Duration::from_secs(10),
        16 * 1024,
    )
    .expect("test endpoint");
    let collector = DeepSeekCollector::new(store, client, 100.0);
    let cancellation = Arc::new(CancellationToken::new());
    cancellation.cancel();

    let result = collector
        .collect_with_cancellation(None, FETCHED_AT, cancellation)
        .await;

    assert_eq!(result, Err(CollectionError::Cancelled));
}

#[tokio::test]
async fn unauthorized_balance_maps_to_authentication_required() {
    let server = TestServer::respond(401, r#"{"error":"invalid key"}"#, Duration::ZERO);
    let store = Arc::new(FakeCredentialStore::new(Some("stored-test-key")));
    let collector = DeepSeekCollector::new(store, server.client(), 100.0);

    let snapshot = collector.collect(None, FETCHED_AT).await;

    assert_eq!(snapshot.status, UsageStatus::AuthenticationRequired);
    assert_eq!(snapshot.used_ratio, None);
}

#[tokio::test]
async fn timeout_returns_a_recognizable_cached_snapshot() {
    let server = TestServer::respond(200, BALANCE_BODY, Duration::from_millis(150));
    let store = Arc::new(FakeCredentialStore::new(Some("stored-test-key")));
    let collector = DeepSeekCollector::new(store, server.client_with_timeout(20), 100.0);
    let cached = deepseek_fixture();

    let snapshot = collector.collect(Some(&cached), FETCHED_AT).await;

    assert_eq!(snapshot.status, UsageStatus::Cached);
    assert_eq!(snapshot.primary_metric, cached.primary_metric);
    assert_eq!(snapshot.used_ratio, cached.used_ratio);
    assert_eq!(
        snapshot.status_message.as_deref(),
        Some("Cached · refresh timed out")
    );
}

#[tokio::test]
async fn verified_candidate_replaces_the_previous_key() {
    let server = TestServer::respond(200, BALANCE_BODY, Duration::ZERO);
    let store = Arc::new(FakeCredentialStore::new(Some("old-working-key")));
    let account = DeepSeekAccountService::new(Arc::clone(&store), server.client(), 100.0);

    let snapshot = account
        .replace_key(SecretString::new("  new-working-key  "), FETCHED_AT)
        .await
        .expect("verified replacement");

    assert_eq!(snapshot.status, UsageStatus::Fresh);
    assert_eq!(store.current(), Some("new-working-key".to_owned()));
    assert_eq!(store.replacement_count(), 1);
}

#[tokio::test]
async fn rejected_candidate_preserves_the_previous_key_and_never_leaks() {
    let server = TestServer::respond(401, r#"{"error":"invalid key"}"#, Duration::ZERO);
    let store = Arc::new(FakeCredentialStore::new(Some("old-working-key")));
    let account = DeepSeekAccountService::new(Arc::clone(&store), server.client(), 100.0);
    let candidate = "candidate-private-value";

    let error = account
        .replace_key(SecretString::new(candidate), FETCHED_AT)
        .await
        .expect_err("candidate should fail verification");

    assert_eq!(store.current(), Some("old-working-key".to_owned()));
    assert_eq!(store.replacement_count(), 0);
    assert!(!error.to_string().contains(candidate));
    assert!(!format!("{error:?}").contains(candidate));
}

#[tokio::test]
async fn credential_write_failure_restores_the_previous_key() {
    let server = TestServer::respond(200, BALANCE_BODY, Duration::ZERO);
    let store = Arc::new(FakeCredentialStore::failing_first_replace(
        "old-working-key",
    ));
    let account = DeepSeekAccountService::new(Arc::clone(&store), server.client(), 100.0);

    let error = account
        .replace_key(SecretString::new("new-working-key"), FETCHED_AT)
        .await
        .expect_err("credential write should fail");

    assert_eq!(store.current(), Some("old-working-key".to_owned()));
    assert_eq!(store.replacement_count(), 2);
    assert_eq!(
        error.to_string(),
        "Windows Credential Manager is unavailable"
    );
}

#[tokio::test]
async fn response_larger_than_the_configured_limit_is_rejected() {
    let server = TestServer::respond(200, BALANCE_BODY, Duration::ZERO);
    let client =
        DeepSeekBalanceClient::for_endpoint(&server.endpoint, Duration::from_millis(500), 8)
            .expect("test endpoint");

    let error = client
        .fetch_balance(&SecretString::new("stored-test-key"))
        .await
        .expect_err("oversized response");

    assert_eq!(error, DeepSeekClientError::ResponseTooLarge);
}

#[test]
fn arbitrary_network_origins_are_rejected() {
    let result = DeepSeekBalanceClient::for_endpoint(
        "https://example.com/user/balance",
        Duration::from_secs(1),
        1024,
    );

    assert!(matches!(result, Err(DeepSeekClientError::UnsafeEndpoint)));
}

const FETCHED_AT: &str = "2026-09-03T00:00:00Z";
const BALANCE_BODY: &str = r#"{
  "is_available": true,
  "balance_infos": [
    {
      "currency": "CNY",
      "total_balance": "77.99",
      "granted_balance": "0",
      "topped_up_balance": "77.99"
    }
  ]
}"#;

fn deepseek_fixture() -> UsageSnapshot {
    let value = serde_json::from_str(include_str!(
        "../../../contracts/fixtures/deepseek-balance.json"
    ))
    .expect("fixture JSON");
    UsageSnapshot::decode_compatible(&value).expect("fixture snapshot")
}

struct FakeCredentialStore {
    secret: Mutex<Option<String>>,
    replacements: Mutex<usize>,
    fail_next_replace: Mutex<bool>,
}

impl FakeCredentialStore {
    fn new(secret: Option<&str>) -> Self {
        Self {
            secret: Mutex::new(secret.map(str::to_owned)),
            replacements: Mutex::new(0),
            fail_next_replace: Mutex::new(false),
        }
    }

    fn failing_first_replace(secret: &str) -> Self {
        Self {
            secret: Mutex::new(Some(secret.to_owned())),
            replacements: Mutex::new(0),
            fail_next_replace: Mutex::new(true),
        }
    }

    fn current(&self) -> Option<String> {
        self.secret.lock().expect("secret lock").clone()
    }

    fn replacement_count(&self) -> usize {
        *self.replacements.lock().expect("replacement lock")
    }
}

#[async_trait]
impl CredentialStore for FakeCredentialStore {
    async fn read(
        &self,
        _account: CredentialAccount,
    ) -> Result<Option<SecretString>, CredentialStoreError> {
        Ok(self.current().map(SecretString::new))
    }

    async fn replace_verified(
        &self,
        _account: CredentialAccount,
        secret: SecretString,
    ) -> Result<(), CredentialStoreError> {
        *self.secret.lock().expect("secret lock") = Some(secret.expose().to_owned());
        *self.replacements.lock().expect("replacement lock") += 1;
        let should_fail =
            std::mem::take(&mut *self.fail_next_replace.lock().expect("failure lock"));
        if should_fail {
            Err(CredentialStoreError::Unavailable)
        } else {
            Ok(())
        }
    }

    async fn delete(&self, _account: CredentialAccount) -> Result<(), CredentialStoreError> {
        *self.secret.lock().expect("secret lock") = None;
        Ok(())
    }
}

struct TestServer {
    endpoint: String,
    thread: Option<thread::JoinHandle<()>>,
}

impl TestServer {
    fn respond(status: u16, body: &'static str, delay: Duration) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let address = listener.local_addr().expect("test server address");
        let thread = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request);
            thread::sleep(delay);
            let reason = if status == 200 { "OK" } else { "Unauthorized" };
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        });
        Self {
            endpoint: format!("http://{address}/user/balance"),
            thread: Some(thread),
        }
    }

    fn client(&self) -> DeepSeekBalanceClient {
        self.client_with_timeout(500)
    }

    fn client_with_timeout(&self, timeout_ms: u64) -> DeepSeekBalanceClient {
        DeepSeekBalanceClient::for_endpoint(
            &self.endpoint,
            Duration::from_millis(timeout_ms),
            16 * 1024,
        )
        .expect("test endpoint")
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            thread.join().expect("test server thread");
        }
    }
}
