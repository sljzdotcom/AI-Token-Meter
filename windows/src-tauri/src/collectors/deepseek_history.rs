use std::collections::{BTreeMap, btree_map::Entry};
use std::error::Error;
use std::fmt::{Display, Formatter};

use reqwest::Url;
use serde::Deserialize;
use time::format_description::FormatItem;
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::{Date, Duration, OffsetDateTime};

use crate::domain::{DailyHistoryEntry, ProviderId, UsageSnapshot};

const OFFICIAL_CONSOLE_HOST: &str = "platform.deepseek.com";
const HISTORY_SCHEMA_VERSION: u64 = 1;
const MAX_CHUNKS: u16 = 64;
const MAX_CHUNK_BYTES: usize = 16 * 1024;
const MAX_PAYLOAD_BYTES: usize = 256 * 1024;
const ASSEMBLY_TIMEOUT: Duration = Duration::seconds(20);
const DATE_FORMAT: &[FormatItem<'static>] = format_description!("[year]-[month]-[day]");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeepSeekHistoryChunk {
    pub nonce: String,
    pub origin: String,
    pub sequence: u16,
    pub total: u16,
    pub payload_fragment: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeepSeekHistory {
    pub days: Vec<DailyHistoryEntry>,
    pub total_cost_cny: f64,
    pub total_requests: u64,
    pub total_tokens: u64,
    pub fetched_at: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AcceptOutcome {
    Waiting,
    Complete(DeepSeekHistory),
}

pub struct DeepSeekHistoryAssembler {
    expected_nonce: String,
    started_at: OffsetDateTime,
    expected_chunks: Option<u16>,
    chunks: BTreeMap<u16, String>,
    payload_bytes: usize,
}

impl DeepSeekHistoryAssembler {
    pub fn new(expected_nonce: impl Into<String>, started_at: OffsetDateTime) -> Self {
        Self {
            expected_nonce: expected_nonce.into(),
            started_at,
            expected_chunks: None,
            chunks: BTreeMap::new(),
            payload_bytes: 0,
        }
    }

    pub fn accept(
        &mut self,
        chunk: DeepSeekHistoryChunk,
        now: OffsetDateTime,
    ) -> Result<AcceptOutcome, DeepSeekHistoryError> {
        if now < self.started_at || now - self.started_at > ASSEMBLY_TIMEOUT {
            return Err(DeepSeekHistoryError::Expired);
        }
        if chunk.nonce != self.expected_nonce {
            return Err(DeepSeekHistoryError::NonceMismatch);
        }
        validate_origin(&chunk.origin)?;
        if chunk.total == 0
            || chunk.total > MAX_CHUNKS
            || chunk.sequence >= chunk.total
            || chunk.payload_fragment.len() > MAX_CHUNK_BYTES
        {
            return Err(DeepSeekHistoryError::ChunkLimitExceeded);
        }
        if self
            .expected_chunks
            .is_some_and(|expected| expected != chunk.total)
        {
            return Err(DeepSeekHistoryError::ChunkSequenceMismatch);
        }
        self.expected_chunks = Some(chunk.total);

        match self.chunks.entry(chunk.sequence) {
            Entry::Occupied(existing) if existing.get() != &chunk.payload_fragment => {
                return Err(DeepSeekHistoryError::ChunkSequenceMismatch);
            }
            Entry::Occupied(_) => return Ok(AcceptOutcome::Waiting),
            Entry::Vacant(entry) => {
                self.payload_bytes = self
                    .payload_bytes
                    .checked_add(chunk.payload_fragment.len())
                    .ok_or(DeepSeekHistoryError::ChunkLimitExceeded)?;
                if self.payload_bytes > MAX_PAYLOAD_BYTES {
                    return Err(DeepSeekHistoryError::ChunkLimitExceeded);
                }
                entry.insert(chunk.payload_fragment);
            }
        }

        if self.chunks.len() != usize::from(chunk.total) {
            return Ok(AcceptOutcome::Waiting);
        }

        let mut payload = String::with_capacity(self.payload_bytes);
        for sequence in 0..chunk.total {
            payload.push_str(
                self.chunks
                    .get(&sequence)
                    .ok_or(DeepSeekHistoryError::ChunkSequenceMismatch)?,
            );
        }
        reject_sensitive_payload(&payload)?;
        let history = parse_payload(&payload, now)?;
        Ok(AcceptOutcome::Complete(history))
    }
}

pub fn apply_history(
    balance: &UsageSnapshot,
    history: &DeepSeekHistory,
) -> Result<UsageSnapshot, DeepSeekHistoryError> {
    if balance.provider_id != ProviderId::DeepSeek {
        return Err(DeepSeekHistoryError::WrongProvider);
    }
    let mut updated = balance.clone();
    updated.daily_history.clone_from(&history.days);
    updated.history_fetched_at = Some(history.fetched_at.clone());
    Ok(updated)
}

fn validate_origin(value: &str) -> Result<(), DeepSeekHistoryError> {
    let url = Url::parse(value).map_err(|_| DeepSeekHistoryError::UnsafeOrigin)?;
    if url.scheme() != "https"
        || url.host_str() != Some(OFFICIAL_CONSOLE_HOST)
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(DeepSeekHistoryError::UnsafeOrigin);
    }
    Ok(())
}

fn reject_sensitive_payload(payload: &str) -> Result<(), DeepSeekHistoryError> {
    let lower = payload.to_ascii_lowercase();
    for forbidden in [
        "\"authorization\"",
        "\"cookie\"",
        "\"api_key\"",
        "\"apikey\"",
        "\"access_token\"",
        "\"phone\"",
        "\"email\"",
    ] {
        if lower.contains(forbidden) {
            return Err(DeepSeekHistoryError::SensitivePayload);
        }
    }
    Ok(())
}

fn parse_payload(
    payload: &str,
    fetched_at: OffsetDateTime,
) -> Result<DeepSeekHistory, DeepSeekHistoryError> {
    let payload: RawPayload =
        serde_json::from_str(payload).map_err(|_| DeepSeekHistoryError::InvalidPayload)?;
    if payload.schema_version != HISTORY_SCHEMA_VERSION {
        return Err(DeepSeekHistoryError::InvalidPayload);
    }

    let today = fetched_at.date();
    let first_day = today - Duration::days(29);
    let mut merged: BTreeMap<Date, DailyHistoryEntry> = BTreeMap::new();
    for day in payload.days {
        if !day.cost_cny.is_finite() || day.cost_cny < 0.0 {
            return Err(DeepSeekHistoryError::InvalidPayload);
        }
        let date = Date::parse(&day.date, DATE_FORMAT)
            .map_err(|_| DeepSeekHistoryError::InvalidPayload)?;
        if date < first_day || date > today {
            continue;
        }
        let entry = merged.entry(date).or_insert(DailyHistoryEntry {
            date: day.date,
            cost_cny: 0.0,
            requests: 0,
            tokens: 0,
        });
        entry.cost_cny += day.cost_cny;
        if !entry.cost_cny.is_finite() {
            return Err(DeepSeekHistoryError::InvalidPayload);
        }
        entry.requests = entry
            .requests
            .checked_add(day.requests)
            .ok_or(DeepSeekHistoryError::InvalidPayload)?;
        entry.tokens = entry
            .tokens
            .checked_add(day.tokens)
            .ok_or(DeepSeekHistoryError::InvalidPayload)?;
    }

    let days: Vec<_> = merged.into_values().collect();
    let mut total_cost_cny = 0.0;
    let mut total_requests = 0_u64;
    let mut total_tokens = 0_u64;
    for day in &days {
        total_cost_cny += day.cost_cny;
        total_requests = total_requests
            .checked_add(day.requests)
            .ok_or(DeepSeekHistoryError::InvalidPayload)?;
        total_tokens = total_tokens
            .checked_add(day.tokens)
            .ok_or(DeepSeekHistoryError::InvalidPayload)?;
    }

    Ok(DeepSeekHistory {
        days,
        total_cost_cny,
        total_requests,
        total_tokens,
        fetched_at: fetched_at
            .format(&Rfc3339)
            .map_err(|_| DeepSeekHistoryError::InvalidPayload)?,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawPayload {
    schema_version: u64,
    days: Vec<RawDay>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawDay {
    date: String,
    cost_cny: f64,
    requests: u64,
    tokens: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeepSeekHistoryError {
    UnsafeOrigin,
    NonceMismatch,
    Expired,
    ChunkLimitExceeded,
    ChunkSequenceMismatch,
    SensitivePayload,
    InvalidPayload,
    WrongProvider,
}

impl Display for DeepSeekHistoryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::UnsafeOrigin => "history payload came from an untrusted origin",
            Self::NonceMismatch => "history payload nonce did not match",
            Self::Expired => "history payload assembly expired",
            Self::ChunkLimitExceeded => "history payload exceeded its chunk limit",
            Self::ChunkSequenceMismatch => "history payload chunks were inconsistent",
            Self::SensitivePayload => "history payload contained a forbidden sensitive field",
            Self::InvalidPayload => "history payload format was invalid",
            Self::WrongProvider => "history can only be attached to DeepSeek",
        };
        formatter.write_str(message)
    }
}

impl Error for DeepSeekHistoryError {}
