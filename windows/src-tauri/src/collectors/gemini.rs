use super::CollectionError;
use crate::domain::{
    MetricKind, MetricUnit, ProviderId, Ratio, UsageMetric, UsageSnapshot, UsageStatus,
};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

pub const MINIMUM_VERSION: &str = "1.1.28";

pub fn parse_usage(
    raw: &str,
    fetched_at: &str,
    source_version: &str,
) -> Result<UsageSnapshot, CollectionError> {
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum Key {
        GeminiFiveHour,
        GeminiWeekly,
        OtherFiveHour,
        OtherWeekly,
    }
    impl Key {
        fn from_columns(group: &str, window: &str) -> Option<Self> {
            match (group, window) {
                ("Gemini Models", "Five Hour Limit Remaining") => Some(Self::GeminiFiveHour),
                ("Gemini Models", "Weekly Limit Remaining") => Some(Self::GeminiWeekly),
                ("Claude and GPT models", "Five Hour Limit Remaining") => Some(Self::OtherFiveHour),
                ("Claude and GPT models", "Weekly Limit Remaining") => Some(Self::OtherWeekly),
                _ => None,
            }
        }
        fn label(self) -> &'static str {
            match self {
                Self::GeminiFiveHour => "Gemini · Five hour",
                Self::GeminiWeekly => "Gemini · Weekly",
                Self::OtherFiveHour => "Claude/GPT · Five hour",
                Self::OtherWeekly => "Claude/GPT · Weekly",
            }
        }
    }

    let rows = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if rows.len() != 4 {
        return Err(CollectionError::UnrecognizedOutput);
    }

    let mut metrics = Vec::with_capacity(4);
    for row in rows {
        let columns = row.split('\t').collect::<Vec<_>>();
        if columns.len() != 4 {
            return Err(CollectionError::UnrecognizedOutput);
        }
        let key =
            Key::from_columns(columns[0], columns[1]).ok_or(CollectionError::UnrecognizedOutput)?;
        if metrics.iter().any(|(existing, _)| *existing == key) {
            return Err(CollectionError::UnrecognizedOutput);
        }
        let remaining = columns[2]
            .strip_suffix('%')
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
            .ok_or(CollectionError::UnrecognizedOutput)?;
        OffsetDateTime::parse(columns[3], &Rfc3339)
            .map_err(|_| CollectionError::UnrecognizedOutput)?;
        metrics.push((
            key,
            UsageMetric {
                label: key.label().into(),
                current: 100.0 - remaining,
                limit: Some(100.0),
                unit: MetricUnit::Percent,
                kind: MetricKind::OfficialLimit,
                reset_at: Some(columns[3].into()),
                reset_description: None,
            },
        ));
    }
    metrics.sort_by_key(|(key, _)| *key);
    let metrics = metrics
        .into_iter()
        .map(|(_, metric)| metric)
        .collect::<Vec<_>>();
    let mut ranked = metrics.clone();
    ranked.sort_by(|a, b| b.current.total_cmp(&a.current));

    Ok(UsageSnapshot {
        schema_version: 1,
        provider_id: ProviderId::Gemini,
        display_name: "Google Antigravity".into(),
        status: UsageStatus::Fresh,
        used_ratio: Some(
            Ratio::new(ranked[0].current / 100.0).map_err(|_| CollectionError::InvalidResponse)?,
        ),
        primary_metric: ranked.first().cloned(),
        secondary_metric: ranked.get(1).cloned(),
        gemini_quota_metrics: metrics,
        fetched_at: fetched_at.into(),
        stale_after_seconds: 300,
        source_version: Some(source_version.into()),
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
                .is_some_and(|message| message.contains("sign in required")) =>
        {
            State::SignInRequired
        }
        _ => State::Unavailable,
    };
    status.account_detail = Some(snapshot.status_message.clone().unwrap_or_else(|| {
        if status.connection_state == State::Connected {
            "Official Antigravity quota available; account identity is not exposed.".into()
        } else {
            "Account status has not been verified by a current Antigravity CLI quota result.".into()
        }
    }));
    status.cli_version = snapshot.source_version.clone();
    status
}
