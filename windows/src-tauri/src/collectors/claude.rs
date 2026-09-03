use regex::Regex;

use crate::domain::{
    MetricKind, MetricUnit, ProviderId, Ratio, UsageMetric, UsageSnapshot, UsageStatus,
};
use crate::platform::windows::process::normalize_output;

use super::CollectionError;

const STALE_AFTER_SECONDS: u64 = 300;

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
