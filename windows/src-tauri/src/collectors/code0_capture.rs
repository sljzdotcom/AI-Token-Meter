use std::fmt::{Display, Formatter};

use crate::domain::{MetricKind, MetricUnit, ProviderId, UsageMetric, UsageSnapshot, UsageStatus};
use serde::Deserialize;
use serde_json::value::RawValue;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code0CaptureError {
    InvalidPayload,
}

impl Display for Code0CaptureError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("invalid filtered Code0 quota payload")
    }
}

impl std::error::Error for Code0CaptureError {}

/// Accepts only filtered quota numbers. No account JSON is part of this contract.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Code0WebsiteCapture {
    pub balance: f64,
    pub cumulative_usage: f64,
}

impl Code0WebsiteCapture {
    pub fn decode_filtered_json(filtered_payload: &[u8]) -> Result<Self, Code0CaptureError> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields, rename_all = "camelCase")]
        struct Filtered {
            quota: Box<RawValue>,
            used_quota: Box<RawValue>,
            divisor: Box<RawValue>,
        }
        if filtered_payload.len() > 2048 {
            return Err(Code0CaptureError::InvalidPayload);
        }
        let filtered: Filtered = serde_json::from_slice(filtered_payload)
            .map_err(|_| Code0CaptureError::InvalidPayload)?;
        let quota = safe_quota(filtered.quota.get())?;
        let used = safe_quota(filtered.used_quota.get())?;
        let divisor = number(filtered.divisor.get())?;
        if !(0.0..=9_007_199_254_740_991.0).contains(&divisor)
            || divisor == 0.0
            || !within_maximum(filtered.divisor.get(), divisor)
        {
            return Err(Code0CaptureError::InvalidPayload);
        }
        let capture = Self {
            balance: quota / divisor,
            cumulative_usage: used / divisor,
        };
        capture.validate()?;
        Ok(Self {
            balance: if capture.balance == 0.0 {
                0.0
            } else {
                capture.balance
            },
            cumulative_usage: if capture.cumulative_usage == 0.0 {
                0.0
            } else {
                capture.cumulative_usage
            },
        })
    }

    pub fn make_snapshot(&self, fetched_at: &str) -> Result<UsageSnapshot, Code0CaptureError> {
        self.validate()?; // Public values may have been changed after a valid capture.
        OffsetDateTime::parse(fetched_at, &Rfc3339)
            .map_err(|_| Code0CaptureError::InvalidPayload)?;
        Ok(UsageSnapshot {
            schema_version: 1,
            provider_id: ProviderId::Code0,
            display_name: "Code0".to_owned(),
            status: UsageStatus::Fresh,
            used_ratio: None,
            primary_metric: Some(metric("Balance", self.balance, MetricKind::Balance)),
            secondary_metric: Some(metric(
                "Cumulative usage",
                self.cumulative_usage,
                MetricKind::CumulativeUsage,
            )),
            fetched_at: fetched_at.to_owned(),
            stale_after_seconds: 300,
            source_version: Some("website".to_owned()),
            status_message: None,
            gemini_quota_metrics: vec![],
            antigravity_shared_quota_metrics: vec![],
            antigravity_cli_info: None,
            reset_credits: vec![],
            local_activity: None,
            daily_history: vec![],
            history_fetched_at: None,
        })
    }

    fn validate(&self) -> Result<(), Code0CaptureError> {
        if [self.balance, self.cumulative_usage]
            .iter()
            .any(|n| !n.is_finite() || *n < 0.0)
        {
            return Err(Code0CaptureError::InvalidPayload);
        }
        Ok(())
    }
}

fn number(text: &str) -> Result<f64, Code0CaptureError> {
    text.parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .ok_or(Code0CaptureError::InvalidPayload)
}

fn safe_quota(text: &str) -> Result<f64, Code0CaptureError> {
    let value = number(text)?;
    if !(0.0..=9_007_199_254_740_991.0).contains(&value) {
        return Err(Code0CaptureError::InvalidPayload);
    }
    let (mantissa, exponent) = text
        .find(['e', 'E'])
        .map_or((text, "0"), |index| (&text[..index], &text[index + 1..]));
    let fraction_count = mantissa
        .split_once('.')
        .map_or(0, |(_, fraction)| fraction.len());
    let mut digits = mantissa
        .bytes()
        .filter(u8::is_ascii_digit)
        .map(char::from)
        .collect::<String>()
        .trim_start_matches('0')
        .to_owned();
    if digits.is_empty() {
        return Ok(0.0);
    }
    let exponent: i32 = exponent
        .parse()
        .map_err(|_| Code0CaptureError::InvalidPayload)?;
    if text.starts_with('-') || !(-4096..=4096).contains(&exponent) {
        return Err(Code0CaptureError::InvalidPayload);
    }
    let shift = exponent - fraction_count as i32;
    if shift < 0 {
        let count = (-shift) as usize;
        if count >= digits.len() || !digits[digits.len() - count..].bytes().all(|n| n == b'0') {
            return Err(Code0CaptureError::InvalidPayload);
        }
        digits.truncate(digits.len() - count);
    } else {
        if digits.len() + shift as usize > 16 {
            return Err(Code0CaptureError::InvalidPayload);
        }
        digits.push_str(&"0".repeat(shift as usize));
    }
    if digits.len() > 16 || (digits.len() == 16 && digits.as_str() > "9007199254740991") {
        return Err(Code0CaptureError::InvalidPayload);
    }
    Ok(value)
}

fn within_maximum(text: &str, value: f64) -> bool {
    if value != 9_007_199_254_740_991.0 {
        return true;
    }
    let (mantissa, exponent) = text
        .find(['e', 'E'])
        .map_or((text, "0"), |index| (&text[..index], &text[index + 1..]));
    let fraction_count = mantissa
        .split_once('.')
        .map_or(0, |(_, fraction)| fraction.len());
    let digits = mantissa
        .bytes()
        .filter(u8::is_ascii_digit)
        .map(char::from)
        .collect::<String>()
        .trim_start_matches('0')
        .to_owned();
    let Ok(exponent) = exponent.parse::<i32>() else {
        return false;
    };
    if !(-4096..=4096).contains(&exponent) {
        return false;
    }
    let point = digits.len() as i32 + exponent - fraction_count as i32;
    if point != 16 {
        return point < 16;
    }
    let integer = digits.chars().take(16).collect::<String>()
        + &"0".repeat(16usize.saturating_sub(digits.len()));
    if integer != "9007199254740991" {
        return integer.as_str() < "9007199254740991";
    }
    digits.bytes().skip(16).all(|byte| byte == b'0')
}

fn metric(label: &str, current: f64, kind: MetricKind) -> UsageMetric {
    UsageMetric {
        label: label.to_owned(),
        current,
        limit: None,
        unit: MetricUnit::SiteCredits,
        kind,
        reset_at: None,
        reset_description: None,
    }
}
