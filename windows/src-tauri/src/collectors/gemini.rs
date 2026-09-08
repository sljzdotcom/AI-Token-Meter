use super::{CollectionError, gemini_terminal::screen};
use crate::domain::{
    MetricKind, MetricUnit, ProviderId, Ratio, UsageMetric, UsageSnapshot, UsageStatus,
};
use regex::Regex;
use std::sync::LazyLock;

pub const VERSION: &str = "0.58.0";
#[derive(Debug, PartialEq, Eq)]
pub enum TerminalState {
    Ready,
    Waiting,
    Model,
}

pub fn terminal_state(raw: &str) -> Result<TerminalState, CollectionError> {
    let visible = screen(raw)?;
    visible_state(&visible)
}
pub(super) fn visible_state(visible: &str) -> Result<TerminalState, CollectionError> {
    let lower = visible.to_lowercase();
    if [
        "enter the authorization code",
        "authorize the application",
        "sign in with google",
        "login with google",
    ]
    .iter()
    .any(|p| lower.contains(p))
    {
        return Err(CollectionError::AuthenticationRequired);
    }
    if [
        "trust this folder",
        "trust this workspace",
        "trust the files",
        "do you trust",
    ]
    .iter()
    .any(|p| lower.contains(p))
    {
        return Err(CollectionError::SetupRequired);
    }
    if [
        "select a theme",
        "select theme",
        "choose an account",
        "select an account",
        "welcome to gemini",
        "how would you like to get started",
        "use gemini api key",
        "use vertex ai",
    ]
    .iter()
    .any(|p| lower.contains(p))
    {
        return Err(CollectionError::UnsupportedConfiguration);
    }
    if visible.contains("Select Model") {
        return Ok(TerminalState::Model);
    }
    if visible.lines().any(|line| {
        line.trim().starts_with('>') && line.contains("Type your message or @path/to/file")
    }) {
        return Ok(TerminalState::Ready);
    }
    Ok(TerminalState::Waiting)
}

pub fn parse_terminal_quota(raw: &str, fetched_at: &str) -> Result<UsageSnapshot, CollectionError> {
    parse_visible_quota(&screen(raw)?, fetched_at)
}
pub(super) fn parse_visible_quota(
    visible: &str,
    fetched_at: &str,
) -> Result<UsageSnapshot, CollectionError> {
    if visible_state(visible)? != TerminalState::Model {
        return Err(CollectionError::UnrecognizedOutput);
    }
    let start = visible
        .rfind("Select Model")
        .ok_or(CollectionError::UnrecognizedOutput)?;
    let dialog = &visible[start..];
    let end = dialog
        .find("(Press Esc to close)")
        .ok_or(CollectionError::UnrecognizedOutput)?;
    if !dialog[end..].contains('╯') {
        return Err(CollectionError::UnrecognizedOutput);
    }
    let body = &dialog[..end];
    let (_, usage) = body
        .split_once("Model usage")
        .ok_or(CollectionError::QuotaUnavailable)?;
    static ROW: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^(Pro|Flash Lite|Flash)\s+[▬━─█░▓▒▏▎▍▌▋▊▉▐\s]+\s+(\d{1,3})%\s*(Resets: [^│]+)?$",
        )
        .unwrap()
    });
    let mut metrics = Vec::new();
    for line in usage.lines() {
        let text = line.trim().trim_matches('│').trim();
        if text.is_empty() {
            continue;
        }
        let captures = ROW
            .captures(text)
            .ok_or(CollectionError::UnrecognizedOutput)?;
        let label = captures[1].to_owned();
        let current = captures[2]
            .parse::<u32>()
            .map_err(|_| CollectionError::UnrecognizedOutput)?;
        if current > 100 || metrics.iter().any(|m: &UsageMetric| m.label == label) {
            return Err(CollectionError::UnrecognizedOutput);
        }
        metrics.push(UsageMetric {
            label,
            current: f64::from(current),
            limit: Some(100.0),
            unit: MetricUnit::Percent,
            kind: MetricKind::OfficialLimit,
            reset_at: None,
            reset_description: captures.get(3).map(|m| m.as_str().trim().to_owned()),
        });
    }
    if metrics.is_empty() {
        return Err(CollectionError::QuotaUnavailable);
    }
    metrics.sort_by_key(|m| match m.label.as_str() {
        "Pro" => 0,
        "Flash" => 1,
        _ => 2,
    });
    let mut ranked = metrics.clone();
    ranked.sort_by(|a, b| b.current.total_cmp(&a.current));
    Ok(UsageSnapshot {
        schema_version: 1,
        provider_id: ProviderId::Gemini,
        display_name: "Gemini".into(),
        status: UsageStatus::Fresh,
        used_ratio: Some(
            Ratio::new(ranked[0].current / 100.0).map_err(|_| CollectionError::InvalidResponse)?,
        ),
        primary_metric: ranked.first().cloned(),
        secondary_metric: ranked.get(1).cloned(),
        gemini_quota_metrics: metrics,
        fetched_at: fetched_at.into(),
        stale_after_seconds: 300,
        source_version: Some(VERSION.into()),
        status_message: None,
        reset_credits: Vec::new(),
        local_activity: None,
        daily_history: Vec::new(),
        history_fetched_at: None,
    })
}

pub fn service_status(
    snapshot: &UsageSnapshot,
) -> crate::accounts::service_status::ServiceAccountStatus {
    use crate::accounts::service_status::{
        ServiceAccountConnectionState as State, ServiceAccountStatus,
    };
    let mut status = ServiceAccountStatus::unavailable(ProviderId::Gemini, &snapshot.fetched_at);
    status.connection_state = match snapshot.status {
        UsageStatus::Fresh if !snapshot.gemini_quota_metrics.is_empty() => State::Connected,
        UsageStatus::AuthenticationRequired => State::SignInRequired,
        UsageStatus::NotInstalled => State::NotInstalled,
        UsageStatus::Refreshing => State::Checking,
        UsageStatus::Cached
            if snapshot
                .status_message
                .as_deref()
                .is_some_and(|m| m.contains("sign in required")) =>
        {
            State::SignInRequired
        }
        _ => State::Unavailable,
    };
    status.account_detail = Some(snapshot.status_message.clone().unwrap_or_else(|| {
        if status.connection_state == State::Connected {
            "Official CLI quota available; account identity is not exposed.".into()
        } else {
            "Account status has not been verified by a current Gemini CLI quota result.".into()
        }
    }));
    status.cli_version = snapshot.source_version.clone();
    status
}
