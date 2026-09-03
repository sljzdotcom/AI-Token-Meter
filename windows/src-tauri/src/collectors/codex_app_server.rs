use serde::Deserialize;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::accounts::service_status::{ServiceAccountStatus, parse_codex_account_status};
use crate::domain::{
    MetricKind, MetricUnit, ProviderId, Ratio, ResetCredit, ResetCreditKind, UsageMetric,
    UsageSnapshot, UsageStatus,
};

use super::CollectionError;
use crate::platform::windows::process::{
    CancellationToken, CommandInvocation, ProcessJob, configure_restricted_command,
};

const STALE_AFTER_SECONDS: u64 = 300;
const MAX_JSON_LINE_BYTES: usize = 1_048_576;
const MAX_RESPONSE_LINES: usize = 100;

pub fn collect_rate_limits_from_invocation(
    invocation: &CommandInvocation,
    working_directory: Option<&Path>,
    fetched_at: &str,
    timeout: Duration,
) -> Result<UsageSnapshot, CollectionError> {
    collect_rate_limits_from_invocation_with_cancellation(
        invocation,
        working_directory,
        fetched_at,
        timeout,
        Arc::new(CancellationToken::new()),
    )
}

pub fn collect_rate_limits_from_invocation_with_cancellation(
    invocation: &CommandInvocation,
    working_directory: Option<&Path>,
    fetched_at: &str,
    timeout: Duration,
    cancellation: Arc<CancellationToken>,
) -> Result<UsageSnapshot, CollectionError> {
    with_initialized_session(
        invocation,
        working_directory,
        timeout,
        cancellation,
        |input, receiver, cancellation| {
            write_request(
                input,
                serde_json::json!({ "id": 2, "method": "account/read", "params": { "refreshToken": false } }),
            )?;
            let account = receive_response_with_cancellation(receiver, 2, timeout, cancellation)?;
            parse_account_response(&account, 2)?;
            write_request(
                input,
                serde_json::json!({ "id": 3, "method": "account/rateLimits/read", "params": null }),
            )?;
            let limits = receive_response_with_cancellation(receiver, 3, timeout, cancellation)?;
            parse_rate_limits_response(&limits, 3, fetched_at)
        },
    )
}

pub fn collect_account_status_from_invocation(
    invocation: &CommandInvocation,
    working_directory: Option<&Path>,
    checked_at: &str,
    timeout: Duration,
) -> Result<ServiceAccountStatus, CollectionError> {
    with_initialized_session(
        invocation,
        working_directory,
        timeout,
        Arc::new(CancellationToken::new()),
        |input, receiver, cancellation| {
            write_request(
                input,
                serde_json::json!({ "id": 2, "method": "account/read", "params": { "refreshToken": false } }),
            )?;
            let account = receive_response_with_cancellation(receiver, 2, timeout, cancellation)?;
            parse_codex_account_status(&account, 2, checked_at)
                .map_err(|_| CollectionError::InvalidResponse)
        },
    )
}

fn with_initialized_session<T>(
    invocation: &CommandInvocation,
    working_directory: Option<&Path>,
    timeout: Duration,
    cancellation: Arc<CancellationToken>,
    operation: impl FnOnce(
        &mut std::process::ChildStdin,
        &Receiver<Result<Vec<u8>, ()>>,
        &CancellationToken,
    ) -> Result<T, CollectionError>,
) -> Result<T, CollectionError> {
    let mut command = Command::new(&invocation.executable);
    command
        .args(&invocation.arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    configure_restricted_command(&mut command, &invocation.executable);
    if let Some(directory) = working_directory {
        command.current_dir(directory);
    }
    let job = ProcessJob::create().map_err(|_| CollectionError::Transport)?;
    let mut child = command.spawn().map_err(|_| CollectionError::Transport)?;
    if job.assign(&child).is_err() {
        let _ = child.kill();
        let _ = child.wait();
        return Err(CollectionError::Transport);
    }
    let mut input = child.stdin.take().ok_or(CollectionError::Transport)?;
    let output = child.stdout.take().ok_or(CollectionError::Transport)?;
    let (sender, receiver) = mpsc::sync_channel(16);
    thread::spawn(move || {
        let mut reader = BufReader::new(output);
        loop {
            let mut line = Vec::new();
            match reader.read_until(b'\n', &mut line) {
                Ok(0) => break,
                Ok(_) if line.len() <= MAX_JSON_LINE_BYTES => {
                    if sender.send(Ok(line)).is_err() {
                        return;
                    }
                }
                Ok(_) | Err(_) => {
                    let _ = sender.send(Err(()));
                    return;
                }
            }
        }
    });

    let result = (|| {
        write_request(
            &mut input,
            serde_json::json!({
                "id": 1,
                "method": "initialize",
                "params": {
                    "clientInfo": { "name": "ai-token-meter", "version": env!("CARGO_PKG_VERSION") },
                    "capabilities": { "experimentalApi": true }
                }
            }),
        )?;
        let _ = receive_response_with_cancellation(&receiver, 1, timeout, &cancellation)?;
        write_request(&mut input, serde_json::json!({ "method": "initialized" }))?;
        operation(&mut input, &receiver, &cancellation)
    })();

    drop(input);
    job.terminate_descendants();
    let _ = child.kill();
    let _ = child.wait();
    result
}

fn write_request(input: &mut impl Write, value: Value) -> Result<(), CollectionError> {
    serde_json::to_writer(&mut *input, &value).map_err(|_| CollectionError::Transport)?;
    input
        .write_all(b"\n")
        .and_then(|_| input.flush())
        .map_err(|_| CollectionError::Transport)
}

fn receive_response_with_cancellation(
    receiver: &Receiver<Result<Vec<u8>, ()>>,
    expected_id: u64,
    timeout: Duration,
    cancellation: &CancellationToken,
) -> Result<String, CollectionError> {
    let deadline = Instant::now() + timeout;
    let mut response_lines = 0;
    while response_lines < MAX_RESPONSE_LINES {
        if cancellation.is_cancelled() {
            return Err(CollectionError::Cancelled);
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(CollectionError::TimedOut);
        }
        let bytes = match receiver.recv_timeout(remaining.min(Duration::from_millis(20))) {
            Ok(Ok(bytes)) => {
                response_lines += 1;
                bytes
            }
            Ok(Err(())) | Err(RecvTimeoutError::Disconnected) => {
                return Err(CollectionError::Transport);
            }
            Err(RecvTimeoutError::Timeout) => continue,
        };
        let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
            continue;
        };
        if value.get("id").and_then(Value::as_u64) == Some(expected_id) {
            return String::from_utf8(bytes).map_err(|_| CollectionError::InvalidResponse);
        }
    }
    Err(CollectionError::InvalidResponse)
}

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
        history_fetched_at: None,
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

#[cfg(test)]
mod cancellation_tests {
    use super::*;

    #[test]
    fn waiting_for_an_app_server_response_honours_cancellation() {
        let (_sender, receiver) = mpsc::sync_channel(1);
        let cancellation = CancellationToken::new();
        cancellation.cancel();

        assert_eq!(
            receive_response_with_cancellation(
                &receiver,
                1,
                Duration::from_secs(10),
                &cancellation,
            ),
            Err(CollectionError::Cancelled),
        );
    }
}
