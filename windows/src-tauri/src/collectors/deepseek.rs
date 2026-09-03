use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use reqwest::{StatusCode, Url};
use serde::Deserialize;

use crate::domain::{
    MetricKind, MetricUnit, ProviderId, Ratio, UsageMetric, UsageSnapshot, UsageStatus,
};
use crate::platform::windows::process::CancellationToken;
use crate::security::{CredentialAccount, CredentialStore, SecretString};

use super::CollectionError;

const OFFICIAL_ENDPOINT: &str = "https://api.deepseek.com/user/balance";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_RESPONSE_LIMIT: usize = 64 * 1024;

#[derive(Clone)]
pub struct DeepSeekBalanceClient {
    client: reqwest::Client,
    endpoint: Url,
    response_limit: usize,
}

impl DeepSeekBalanceClient {
    pub fn new() -> Result<Self, DeepSeekClientError> {
        Self::for_endpoint(OFFICIAL_ENDPOINT, DEFAULT_TIMEOUT, DEFAULT_RESPONSE_LIMIT)
    }

    pub fn for_endpoint(
        endpoint: &str,
        timeout: Duration,
        response_limit: usize,
    ) -> Result<Self, DeepSeekClientError> {
        let endpoint = Url::parse(endpoint).map_err(|_| DeepSeekClientError::UnsafeEndpoint)?;
        if !is_allowed_endpoint(&endpoint) || response_limit == 0 {
            return Err(DeepSeekClientError::UnsafeEndpoint);
        }
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| DeepSeekClientError::Transport)?;
        Ok(Self {
            client,
            endpoint,
            response_limit,
        })
    }

    pub async fn fetch_balance(
        &self,
        secret: &SecretString,
    ) -> Result<DeepSeekBalance, DeepSeekClientError> {
        if secret.expose().trim().is_empty() {
            return Err(DeepSeekClientError::AuthenticationRequired);
        }
        let response = self
            .client
            .get(self.endpoint.clone())
            .header("Accept", "application/json")
            .bearer_auth(secret.expose().trim())
            .send()
            .await
            .map_err(map_reqwest_error)?;

        match response.status() {
            StatusCode::OK => {}
            StatusCode::UNAUTHORIZED => return Err(DeepSeekClientError::AuthenticationRequired),
            StatusCode::TOO_MANY_REQUESTS => return Err(DeepSeekClientError::RateLimited),
            _ => return Err(DeepSeekClientError::Transport),
        }
        if response
            .content_length()
            .is_some_and(|length| length > self.response_limit as u64)
        {
            return Err(DeepSeekClientError::ResponseTooLarge);
        }

        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(map_reqwest_error)?;
            if bytes.len().saturating_add(chunk.len()) > self.response_limit {
                return Err(DeepSeekClientError::ResponseTooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }

        let response: BalanceResponse =
            serde_json::from_slice(&bytes).map_err(|_| DeepSeekClientError::InvalidResponse)?;
        let info = response
            .balance_infos
            .into_iter()
            .find(|info| info.currency == "CNY")
            .ok_or(DeepSeekClientError::InvalidResponse)?;
        let total = info
            .total_balance
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite() && *value >= 0.0)
            .ok_or(DeepSeekClientError::InvalidResponse)?;
        Ok(DeepSeekBalance {
            is_available: response.is_available,
            total_cny: total,
        })
    }
}

impl Default for DeepSeekBalanceClient {
    fn default() -> Self {
        Self::new().expect("the fixed DeepSeek endpoint and HTTP client configuration are valid")
    }
}

pub struct DeepSeekCollector<S> {
    credentials: Arc<S>,
    client: DeepSeekBalanceClient,
    balance_baseline: f64,
}

impl<S> DeepSeekCollector<S>
where
    S: CredentialStore,
{
    pub fn new(credentials: Arc<S>, client: DeepSeekBalanceClient, balance_baseline: f64) -> Self {
        Self {
            credentials,
            client,
            balance_baseline: balance_baseline.max(1.0),
        }
    }

    pub async fn collect(&self, cached: Option<&UsageSnapshot>, fetched_at: &str) -> UsageSnapshot {
        let secret = match self
            .credentials
            .read(CredentialAccount::DeepSeekApiKey)
            .await
        {
            Ok(Some(secret)) if !secret.expose().trim().is_empty() => secret,
            Ok(_) => return status_snapshot(UsageStatus::SetupRequired, fetched_at, None),
            Err(_) => return cached_or_unavailable(cached, fetched_at, "credential unavailable"),
        };

        match self.client.fetch_balance(&secret).await {
            Ok(balance) => snapshot_from_balance(balance, self.balance_baseline, fetched_at),
            Err(DeepSeekClientError::AuthenticationRequired) => status_snapshot(
                UsageStatus::AuthenticationRequired,
                fetched_at,
                Some("DeepSeek API Key requires attention"),
            ),
            Err(DeepSeekClientError::TimedOut) => {
                cached_or_unavailable(cached, fetched_at, "refresh timed out")
            }
            Err(_) => cached_or_unavailable(cached, fetched_at, "refresh unavailable"),
        }
    }

    pub async fn collect_with_cancellation(
        &self,
        cached: Option<&UsageSnapshot>,
        fetched_at: &str,
        cancellation: Arc<CancellationToken>,
    ) -> Result<UsageSnapshot, CollectionError> {
        tokio::select! {
            biased;
            _ = wait_for_cancellation(&cancellation) => Err(CollectionError::Cancelled),
            snapshot = self.collect(cached, fetched_at) => Ok(snapshot),
        }
    }
}

async fn wait_for_cancellation(cancellation: &CancellationToken) {
    while !cancellation.is_cancelled() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeepSeekBalance {
    pub is_available: bool,
    pub total_cny: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeepSeekClientError {
    AuthenticationRequired,
    RateLimited,
    TimedOut,
    InvalidResponse,
    ResponseTooLarge,
    Transport,
    UnsafeEndpoint,
}

impl Display for DeepSeekClientError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::AuthenticationRequired => "authentication required",
            Self::RateLimited => "request rate limited",
            Self::TimedOut => "request timed out",
            Self::InvalidResponse => "invalid balance response",
            Self::ResponseTooLarge => "balance response exceeded the size limit",
            Self::Transport => "balance transport unavailable",
            Self::UnsafeEndpoint => "balance endpoint is not allowed",
        })
    }
}

impl Error for DeepSeekClientError {}

pub(crate) fn snapshot_from_balance(
    balance: DeepSeekBalance,
    baseline: f64,
    fetched_at: &str,
) -> UsageSnapshot {
    let baseline = baseline.max(1.0);
    let consumed = (baseline - balance.total_cny).clamp(0.0, baseline);
    UsageSnapshot {
        schema_version: 1,
        provider_id: ProviderId::DeepSeek,
        display_name: "DeepSeek".to_owned(),
        status: if balance.is_available {
            UsageStatus::Fresh
        } else {
            UsageStatus::Unavailable
        },
        used_ratio: Ratio::new(consumed / baseline).ok(),
        primary_metric: Some(UsageMetric {
            label: "Available balance".to_owned(),
            current: balance.total_cny,
            limit: None,
            unit: MetricUnit::Cny,
            kind: MetricKind::Balance,
            reset_at: None,
            reset_description: None,
        }),
        secondary_metric: Some(UsageMetric {
            label: "Balance baseline".to_owned(),
            current: consumed,
            limit: Some(baseline),
            unit: MetricUnit::Cny,
            kind: MetricKind::LocalBudget,
            reset_at: None,
            reset_description: None,
        }),
        fetched_at: fetched_at.to_owned(),
        stale_after_seconds: 300,
        source_version: Some("deepseek-balance-api".to_owned()),
        status_message: None,
        reset_credits: Vec::new(),
        local_activity: None,
        daily_history: Vec::new(),
        history_fetched_at: None,
    }
}

fn status_snapshot(status: UsageStatus, fetched_at: &str, message: Option<&str>) -> UsageSnapshot {
    UsageSnapshot {
        schema_version: 1,
        provider_id: ProviderId::DeepSeek,
        display_name: "DeepSeek".to_owned(),
        status,
        used_ratio: None,
        primary_metric: None,
        secondary_metric: None,
        fetched_at: fetched_at.to_owned(),
        stale_after_seconds: 300,
        source_version: Some("deepseek-balance-api".to_owned()),
        status_message: message.map(str::to_owned),
        reset_credits: Vec::new(),
        local_activity: None,
        daily_history: Vec::new(),
        history_fetched_at: None,
    }
}

fn cached_or_unavailable(
    cached: Option<&UsageSnapshot>,
    fetched_at: &str,
    reason: &str,
) -> UsageSnapshot {
    if let Some(cached) = cached.filter(|snapshot| snapshot.provider_id == ProviderId::DeepSeek) {
        let mut snapshot = cached.clone();
        snapshot.status = UsageStatus::Cached;
        snapshot.status_message = Some(format!("Cached · {reason}"));
        snapshot
    } else {
        status_snapshot(UsageStatus::Unavailable, fetched_at, Some(reason))
    }
}

fn map_reqwest_error(error: reqwest::Error) -> DeepSeekClientError {
    if error.is_timeout() {
        DeepSeekClientError::TimedOut
    } else {
        DeepSeekClientError::Transport
    }
}

fn is_allowed_endpoint(endpoint: &Url) -> bool {
    if endpoint.as_str() == OFFICIAL_ENDPOINT {
        return true;
    }
    cfg!(debug_assertions)
        && endpoint.scheme() == "http"
        && endpoint
            .host_str()
            .and_then(|host| host.parse::<std::net::IpAddr>().ok())
            .is_some_and(|address| address.is_loopback())
        && endpoint.path() == "/user/balance"
        && endpoint.username().is_empty()
        && endpoint.password().is_none()
        && endpoint.query().is_none()
        && endpoint.fragment().is_none()
}

#[derive(Deserialize)]
struct BalanceResponse {
    is_available: bool,
    balance_infos: Vec<BalanceInfo>,
}

#[derive(Deserialize)]
struct BalanceInfo {
    currency: String,
    total_balance: String,
}
