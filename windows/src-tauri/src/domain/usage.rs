use std::error::Error;
use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const CURRENT_SCHEMA_VERSION: u64 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Hash, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    Claude,
    Codex,
    DeepSeek,
}

impl ProviderId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::DeepSeek => "deepseek",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UsageStatus {
    Fresh,
    Cached,
    Refreshing,
    NotInstalled,
    AuthenticationRequired,
    SetupRequired,
    Unavailable,
    UnrecognizedOutput,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Ratio(f64);

impl Ratio {
    pub fn new(value: f64) -> Result<Self, UsageDecodeError> {
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(UsageDecodeError::new(
                "usedRatio must be a finite number between 0 and 1",
            ))
        }
    }

    pub fn get(self) -> f64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for Ratio {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::new(f64::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetric {
    pub label: String,
    pub current: f64,
    #[serde(default)]
    pub limit: Option<f64>,
    pub unit: MetricUnit,
    pub kind: MetricKind,
    #[serde(default)]
    pub reset_at: Option<String>,
    #[serde(default)]
    pub reset_description: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MetricUnit {
    Percent,
    Cny,
    Usd,
    Tokens,
    Requests,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MetricKind {
    OfficialLimit,
    Balance,
    LocalBudget,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetCredit {
    pub kind: ResetCreditKind,
    pub count: u64,
    pub expires_at: String,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ResetCreditKind {
    FullUsageReset,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalActivity {
    pub period_days: u64,
    pub sessions: u64,
    pub tokens: u64,
    pub active_days: u64,
    #[serde(default)]
    pub longest_session_seconds: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyHistoryEntry {
    pub date: String,
    pub cost_cny: f64,
    pub requests: u64,
    pub tokens: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub schema_version: u64,
    pub provider_id: ProviderId,
    pub display_name: String,
    pub status: UsageStatus,
    #[serde(default)]
    pub used_ratio: Option<Ratio>,
    #[serde(default)]
    pub primary_metric: Option<UsageMetric>,
    #[serde(default)]
    pub secondary_metric: Option<UsageMetric>,
    pub fetched_at: String,
    pub stale_after_seconds: u64,
    #[serde(default)]
    pub source_version: Option<String>,
    #[serde(default)]
    pub status_message: Option<String>,
    #[serde(default)]
    pub reset_credits: Vec<ResetCredit>,
    #[serde(default)]
    pub local_activity: Option<LocalActivity>,
    #[serde(default)]
    pub daily_history: Vec<DailyHistoryEntry>,
    #[serde(default)]
    pub history_fetched_at: Option<String>,
}

impl UsageSnapshot {
    pub fn decode_compatible(value: &Value) -> Result<Self, UsageDecodeError> {
        let schema_version = value
            .get("schemaVersion")
            .and_then(Value::as_u64)
            .ok_or_else(|| UsageDecodeError::new("schemaVersion is required"))?;

        if schema_version != CURRENT_SCHEMA_VERSION {
            return Self::safe_unknown_schema_fallback(value, schema_version);
        }

        let snapshot: Self = serde_json::from_value(value.clone())
            .map_err(|error| UsageDecodeError::new(error.to_string()))?;
        if snapshot.stale_after_seconds == 0 {
            return Err(UsageDecodeError::new(
                "staleAfterSeconds must be greater than zero",
            ));
        }
        Ok(snapshot)
    }

    pub fn is_stale_at_rfc3339(&self, now: &str) -> Result<bool, UsageDecodeError> {
        let fetched_at = OffsetDateTime::parse(&self.fetched_at, &Rfc3339)
            .map_err(|error| UsageDecodeError::new(format!("invalid fetchedAt: {error}")))?;
        let now = OffsetDateTime::parse(now, &Rfc3339)
            .map_err(|error| UsageDecodeError::new(format!("invalid comparison time: {error}")))?;
        let stale_after = i64::try_from(self.stale_after_seconds)
            .map_err(|_| UsageDecodeError::new("staleAfterSeconds is too large"))?;

        Ok(now >= fetched_at + time::Duration::seconds(stale_after))
    }

    fn safe_unknown_schema_fallback(
        value: &Value,
        schema_version: u64,
    ) -> Result<Self, UsageDecodeError> {
        let provider_id = serde_json::from_value(
            value
                .get("providerId")
                .cloned()
                .ok_or_else(|| UsageDecodeError::new("providerId is required"))?,
        )
        .map_err(|error| UsageDecodeError::new(error.to_string()))?;
        let display_name = value
            .get("displayName")
            .and_then(Value::as_str)
            .ok_or_else(|| UsageDecodeError::new("displayName is required"))?
            .to_owned();
        let fetched_at = value
            .get("fetchedAt")
            .and_then(Value::as_str)
            .ok_or_else(|| UsageDecodeError::new("fetchedAt is required"))?
            .to_owned();
        let stale_after_seconds = value
            .get("staleAfterSeconds")
            .and_then(Value::as_u64)
            .filter(|seconds| *seconds > 0)
            .ok_or_else(|| UsageDecodeError::new("staleAfterSeconds must be greater than zero"))?;

        Ok(Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            provider_id,
            display_name,
            status: UsageStatus::Unavailable,
            used_ratio: None,
            primary_metric: None,
            secondary_metric: None,
            fetched_at,
            stale_after_seconds,
            source_version: None,
            status_message: Some(format!(
                "Unsupported usage snapshot schema version {schema_version}"
            )),
            reset_credits: Vec::new(),
            local_activity: None,
            daily_history: Vec::new(),
            history_fetched_at: None,
        })
    }
}

#[derive(Debug)]
pub struct UsageDecodeError {
    message: String,
}

impl UsageDecodeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for UsageDecodeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for UsageDecodeError {}
