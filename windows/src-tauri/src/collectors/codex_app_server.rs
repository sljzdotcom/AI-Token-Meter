use serde::Deserialize;
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::domain::{
    MetricKind, MetricUnit, ProviderId, Ratio, ResetCredit, ResetCreditKind, UsageMetric,
    UsageSnapshot, UsageStatus,
};

use super::CollectionError;

const STALE_AFTER_SECONDS: u64 = 300;

pub fn parse_account_response(text: &str, expected_id: u64) -> Result<(), CollectionError> {
    let response = response_for_id(text, expected_id)?;
    let result = response
        .get("result")
        .ok_or(CollectionError::InvalidResponse)?;
    let account = result
        .get("account")
        .ok_or(CollectionError::InvalidResponse)?;
    let requires_openai_auth = result
        .get("requiresOpenaiAuth")
        .and_then(Value::as_bool)
        .ok_or(CollectionError::InvalidResponse)?;
    if account.is_null() && requires_openai_auth {
        Err(CollectionError::AuthenticationRequired)
    } else if account.is_object() {
        Ok(())
    } else {
        Err(CollectionError::InvalidResponse)
    }
}

pub fn parse_rate_limits_response(
    text: &str,
    expected_id: u64,
    fetched_at: &str,
) -> Result<UsageSnapshot, CollectionError> {
    let response = response_for_id(text, expected_id)?;
    let result: RateLimitsResult = serde_json::from_value(
        response
            .get("result")
            .cloned()
            .ok_or(CollectionError::InvalidResponse)?,
    )
    .map_err(|_| CollectionError::InvalidResponse)?;
    let primary = result.rate_limits.primary.map(metric).transpose()?;
    let secondary = result.rate_limits.secondary.map(metric).transpose()?;
    let used_ratio = primary
        .as_ref()
        .map(|value| Ratio::new(value.current / 100.0))
        .transpose()
        .map_err(|_| CollectionError::UnrecognizedOutput)?;
    if primary.is_none() && secondary.is_none() {
        return Err(CollectionError::UnrecognizedOutput);
    }
    let reset_credits = result
        .rate_limit_reset_credits
        .map(|summary| {
            summary
                .credits
                .unwrap_or_default()
                .into_iter()
                .filter(|credit| credit.status == "available")
                .filter_map(|credit| {
                    let expires_at = credit.expires_at.and_then(rfc3339_from_epoch)?;
                    Some(ResetCredit {
                        kind: ResetCreditKind::FullUsageReset,
                        count: 1,
                        expires_at,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(UsageSnapshot {
        schema_version: 1,
        provider_id: ProviderId::Codex,
        display_name: "OpenAI Codex".to_owned(),
        status: UsageStatus::Fresh,
        used_ratio,
        primary_metric: primary,
        secondary_metric: secondary,
        fetched_at: fetched_at.to_owned(),
        stale_after_seconds: STALE_AFTER_SECONDS,
        source_version: Some("codex-app-server".to_owned()),
        status_message: None,
        reset_credits,
        local_activity: None,
        daily_history: Vec::new(),
    })
}

fn response_for_id(text: &str, expected_id: u64) -> Result<Value, CollectionError> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|value| value.get("id").and_then(Value::as_u64) == Some(expected_id))
        .ok_or(CollectionError::InvalidResponse)
}

fn metric(window: RateLimitWindow) -> Result<UsageMetric, CollectionError> {
    if !(0.0..=100.0).contains(&window.used_percent) || !window.used_percent.is_finite() {
        return Err(CollectionError::UnrecognizedOutput);
    }
    Ok(UsageMetric {
        label: match window.window_duration_mins {
            Some(300) => "5h limit".to_owned(),
            Some(10_080) => "Weekly limit".to_owned(),
            Some(minutes) => format!("{minutes}m limit"),
            None => "Usage limit".to_owned(),
        },
        current: window.used_percent,
        limit: Some(100.0),
        unit: MetricUnit::Percent,
        kind: MetricKind::OfficialLimit,
        reset_at: window.resets_at.and_then(rfc3339_from_epoch),
        reset_description: None,
    })
}

fn rfc3339_from_epoch(value: i64) -> Option<String> {
    OffsetDateTime::from_unix_timestamp(value)
        .ok()?
        .format(&Rfc3339)
        .ok()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimitsResult {
    rate_limits: RateLimitSnapshot,
    #[serde(default)]
    rate_limit_reset_credits: Option<ResetCreditSummary>,
}

#[derive(Deserialize)]
struct RateLimitSnapshot {
    #[serde(default)]
    primary: Option<RateLimitWindow>,
    #[serde(default)]
    secondary: Option<RateLimitWindow>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimitWindow {
    used_percent: f64,
    #[serde(default)]
    window_duration_mins: Option<i64>,
    #[serde(default)]
    resets_at: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResetCreditSummary {
    #[allow(dead_code)]
    available_count: u64,
    #[serde(default)]
    credits: Option<Vec<ResetCreditEntry>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResetCreditEntry {
    status: String,
    #[serde(default)]
    expires_at: Option<i64>,
}
