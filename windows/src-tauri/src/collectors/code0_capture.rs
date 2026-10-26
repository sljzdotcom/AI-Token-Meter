use std::fmt::{Display, Formatter};

use crate::domain::UsageSnapshot;

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
    /// Deny-only RED scaffold. Expected keys: quota, usedQuota, divisor.
    pub fn decode_filtered_json(_filtered_payload: &[u8]) -> Result<Self, Code0CaptureError> {
        Err(Code0CaptureError::InvalidPayload)
    }

    pub fn make_snapshot(&self, _fetched_at: &str) -> Result<UsageSnapshot, Code0CaptureError> {
        Err(Code0CaptureError::InvalidPayload)
    }
}
