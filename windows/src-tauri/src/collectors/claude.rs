use regex::Regex;
use std::path::Path;
use std::time::Duration;

use crate::accounts::cli_account::CliProvider;
use crate::domain::{
    MetricKind, MetricUnit, ProviderId, Ratio, UsageMetric, UsageSnapshot, UsageStatus,
};
use crate::platform::windows::conpty::{ConPty, ConPtyError, ConPtySize};
use crate::platform::windows::executable_locator::ExecutableCandidate;
use crate::platform::windows::process::normalize_output;
use crate::platform::windows::process::{
    BoundedProcessRunner, ProcessRequest, command_for_candidate, restricted_environment_for,
};

use super::CollectionError;

const STALE_AFTER_SECONDS: u64 = 300;

pub fn collect_usage_from_candidate(
    candidate: &ExecutableCandidate,
    working_directory: &Path,
    fetched_at: &str,
) -> Result<UsageSnapshot, CollectionError> {
    verify_authentication(candidate, working_directory)?;
    let command = command_for_candidate(
        candidate,
        CliProvider::Claude,
        &["--ax-screen-reader", "--safe-mode"],
    )
    .map_err(|_| CollectionError::Transport)?;
    let mut terminal = ConPty::open(ConPtySize {
        columns: 120,
        rows: 40,
    })
    .map_err(map_terminal_error)?;
    let environment = restricted_environment_for(&command.executable);
    let mut child = terminal
        .spawn(&command, Some(working_directory), &environment)
        .map_err(map_terminal_error)?;

    let initial = terminal
        .read_until(
            &[
                "Claude",
                "Permission Required",
                "Not logged in",
                "Please log in",
            ],
            Duration::from_secs(8),
            64 * 1024,
        )
        .map_err(map_terminal_error)?;
    detect_blocking_prompt(&initial)?;
    terminal
        .send_fixed_input(b"/usage\r")
        .map_err(map_terminal_error)?;
    let first = terminal
        .read_until(
            &[
                "Current week",
                "All models",
                "所有模型",
                "Permission Required",
                "Not logged in",
            ],
            Duration::from_secs(12),
            64 * 1024,
        )
        .map_err(map_terminal_error)?;
    let mut combined = initial;
    combined.push_str(&first);
    detect_blocking_prompt(&combined)?;
    if let Ok(snapshot) = parse_usage_output(&combined, fetched_at)
        && snapshot.secondary_metric.is_some()
    {
        return Ok(snapshot);
    }
    let tail = terminal
        .read_until(
            &["Resets", "重置", "Permission Required", "Not logged in"],
            Duration::from_secs(8),
            64 * 1024,
        )
        .map_err(map_terminal_error)?;
    combined.push_str(&tail);
    let parsed = parse_usage_output(&combined, fetched_at);
    let _ = child.wait(Duration::from_millis(250));
    parsed
}

fn verify_authentication(
    candidate: &ExecutableCandidate,
    working_directory: &Path,
) -> Result<(), CollectionError> {
    let invocation = command_for_candidate(candidate, CliProvider::Claude, &["auth", "status"])
        .map_err(|_| CollectionError::Transport)?;
    let mut request = ProcessRequest::new(invocation.executable, invocation.arguments);
    request.working_directory = Some(working_directory.to_owned());
    request.timeout = Duration::from_secs(5);
    let output = BoundedProcessRunner
        .run(request)
        .map_err(|_| CollectionError::Transport)?;
    detect_blocking_prompt(&format!("{}\n{}", output.stdout, output.stderr))?;
    if output.exit_code != Some(0) {
        return Err(CollectionError::Transport);
    }
    Ok(())
}

fn detect_blocking_prompt(output: &str) -> Result<(), CollectionError> {
    let lowercase = output.to_lowercase();
    if ["not logged in", "please log in", "login required"]
        .iter()
        .any(|marker| lowercase.contains(marker))
        || output.contains("需要登录")
    {
        return Err(CollectionError::AuthenticationRequired);
    }
    if lowercase.contains("permission required: accessing workspace") {
        return Err(CollectionError::SetupRequired);
    }
    Ok(())
}

fn map_terminal_error(error: ConPtyError) -> CollectionError {
    match error {
        ConPtyError::TimedOut => CollectionError::TimedOut,
        _ => CollectionError::Transport,
    }
}

pub fn parse_usage_output(
    raw_output: &str,
    fetched_at: &str,
) -> Result<UsageSnapshot, CollectionError> {
    let text = normalize_output(raw_output.as_bytes());
    let lowercase = text.to_lowercase();
    if ["not logged in", "please log in", "login required"]
        .iter()
        .any(|marker| lowercase.contains(marker))
        || text.contains("需要登录")
    {
        return Err(CollectionError::AuthenticationRequired);
    }
    if lowercase.contains("permission required: accessing workspace") {
        return Err(CollectionError::SetupRequired);
    }

    let percent = percent_regex();
    let lines = text.lines().map(normalize_line).collect::<Vec<_>>();
    let mut label: Option<String> = None;
    let mut metrics = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        let Some(captures) = percent.captures(line) else {
            if !is_reset_line(line) && !is_promotion_line(line) && has_alphanumeric(line) {
                label = Some(line.clone());
            }
            continue;
        };
        if !is_usage_line(line) {
            continue;
        }
        let displayed = captures
            .get(1)
            .and_then(|value| value.as_str().parse::<f64>().ok())
            .ok_or(CollectionError::UnrecognizedOutput)?;
        if !(0.0..=100.0).contains(&displayed) {
            return Err(CollectionError::UnrecognizedOutput);
        }
        let lowered = line.to_lowercase();
        let is_remaining = ["remaining", "left"]
            .iter()
            .any(|marker| lowered.contains(marker))
            || line.contains("剩余")
            || line.contains("可用");
        let used = if is_remaining {
            100.0 - displayed
        } else {
            displayed
        };
        let metric_label = label
            .clone()
            .unwrap_or_else(|| "Claude Code usage".to_owned());
        let reset_description = lines
            .iter()
            .skip(index + 1)
            .take(2)
            .find(|candidate| !candidate.is_empty() && is_reset_line(candidate))
            .cloned();
        let metric = UsageMetric {
            label: metric_label,
            current: used,
            limit: Some(100.0),
            unit: MetricUnit::Percent,
            kind: MetricKind::OfficialLimit,
            reset_at: None,
            reset_description,
        };
        if let Some(existing) = metrics
            .iter()
            .position(|existing: &UsageMetric| existing.label == metric.label)
        {
            metrics[existing] = metric;
        } else {
            metrics.push(metric);
        }
    }

    let primary_metric = metrics
        .first()
        .cloned()
        .ok_or(CollectionError::UnrecognizedOutput)?;
    let used_ratio = Ratio::new(primary_metric.current / 100.0)
        .map_err(|_| CollectionError::UnrecognizedOutput)?;
    Ok(UsageSnapshot {
        schema_version: 1,
        provider_id: ProviderId::Claude,
        display_name: "Claude Code".to_owned(),
        status: UsageStatus::Fresh,
        used_ratio: Some(used_ratio),
        primary_metric: Some(primary_metric),
        secondary_metric: metrics.get(1).cloned(),
        fetched_at: fetched_at.to_owned(),
        stale_after_seconds: STALE_AFTER_SECONDS,
        source_version: Some("claude-cli".to_owned()),
        status_message: None,
        reset_credits: Vec::new(),
        local_activity: None,
        daily_history: Vec::new(),
    })
}

fn percent_regex() -> &'static Regex {
    static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"([0-9]+(?:\.[0-9]+)?)\s*%").expect("valid percent regex"))
}

fn normalize_line(line: &str) -> String {
    line.trim()
        .trim_matches(|character| matches!(character, '│' | '┃' | '╭' | '╮' | '╰' | '╯'))
        .trim()
        .to_owned()
}

fn is_usage_line(line: &str) -> bool {
    let lowered = line.to_lowercase();
    ["used", "remaining", "left"]
        .iter()
        .any(|marker| lowered.contains(marker))
        || ["已用", "使用", "剩余", "可用"]
            .iter()
            .any(|marker| line.contains(marker))
}

fn is_reset_line(line: &str) -> bool {
    line.to_lowercase().contains("reset") || line.contains("重置")
}

fn is_promotion_line(line: &str) -> bool {
    let lowered = line.to_lowercase();
    lowered.contains("promo") || lowered.contains("higher through")
}

fn has_alphanumeric(line: &str) -> bool {
    line.chars().any(char::is_alphanumeric)
}
