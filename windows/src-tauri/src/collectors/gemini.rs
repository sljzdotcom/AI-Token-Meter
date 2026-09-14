use super::CollectionError;
use crate::domain::{
    AntigravityCliInfo, MetricKind, MetricUnit, ProviderId, Ratio, UsageMetric, UsageSnapshot,
    UsageStatus, valid_gemini_display_name,
};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

pub const MINIMUM_VERSION: &str = "1.1.28";

const MAXIMUM_MODEL_ROWS: usize = 64;
const MAXIMUM_MODEL_VALUE_LENGTH: usize = 120;
const MAXIMUM_MODEL_FAMILIES: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeminiModelCatalog {
    pub model_count: u64,
    pub families: Vec<String>,
}

pub fn parse_current_gemini_model(raw: &str) -> Option<String> {
    let rows = parse_model_rows(raw, false)?;
    if rows.len() != 1 || !is_gemini_model(&rows[0]) {
        return None;
    }
    Some(rows[0].1.clone())
}

pub fn parse_gemini_catalog(raw: &str) -> Option<GeminiModelCatalog> {
    let rows = parse_model_rows(raw, true)?;
    if rows
        .iter()
        .any(|row| row.0.starts_with("gemini-") && !is_gemini_model(row))
    {
        return None;
    }
    let gemini = rows.into_iter().filter(is_gemini_model).collect::<Vec<_>>();
    if gemini.is_empty() {
        return None;
    }
    let mut families = Vec::new();
    for (_, name) in &gemini {
        let family = model_family(name);
        if !families.contains(&family) {
            if families.len() >= MAXIMUM_MODEL_FAMILIES {
                return None;
            }
            families.push(family);
        }
    }
    Some(GeminiModelCatalog {
        model_count: gemini.len().try_into().ok()?,
        families,
    })
}

pub fn cli_info(
    current_model_output: Option<&str>,
    models_output: Option<&str>,
) -> Option<AntigravityCliInfo> {
    let current_model = current_model_output.and_then(parse_current_gemini_model);
    let catalog = models_output.and_then(parse_gemini_catalog);
    if current_model.is_none() && catalog.is_none() {
        return None;
    }
    Some(AntigravityCliInfo {
        current_model,
        available_model_count: catalog.as_ref().map(|value| value.model_count),
        model_families: catalog.map(|value| value.families).unwrap_or_default(),
    })
}

fn parse_model_rows(raw: &str, allows_banner: bool) -> Option<Vec<(String, String)>> {
    let lines = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if lines.is_empty() || lines.len() > MAXIMUM_MODEL_ROWS + 1 {
        return None;
    }
    let mut rows = Vec::new();
    for line in lines {
        if allows_banner && line == "Fetching available models..." {
            continue;
        }
        let columns = line.split('\t').collect::<Vec<_>>();
        if columns.len() != 2 || !valid_model_value(columns[0]) || !valid_model_value(columns[1]) {
            return None;
        }
        rows.push((columns[0].to_owned(), columns[1].to_owned()));
    }
    if rows.is_empty() || rows.len() > MAXIMUM_MODEL_ROWS {
        None
    } else {
        Some(rows)
    }
}

fn valid_model_value(value: &str) -> bool {
    !value.is_empty()
        && value.chars().count() <= MAXIMUM_MODEL_VALUE_LENGTH
        && value.chars().all(|character| !character.is_control())
}

fn is_gemini_model((id, name): &(String, String)) -> bool {
    valid_gemini_model_id(id) && valid_gemini_display_name(name)
}

fn valid_gemini_model_id(value: &str) -> bool {
    value.starts_with("gemini-")
        && value.chars().count() <= MAXIMUM_MODEL_VALUE_LENGTH
        && value
            .chars()
            .last()
            .is_some_and(|character| character.is_ascii_alphanumeric())
        && !value.contains("--")
        && !value.contains("..")
        && value.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '-'
                || character == '.'
        })
}

fn model_family(name: &str) -> String {
    if name.ends_with(')')
        && let Some(opening) = name.rfind('(')
    {
        return name[..opening].trim_end().to_owned();
    }
    name.to_owned()
}

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
    if rows.len() != 2 && rows.len() != 4 {
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
    let has_other_five_hour = metrics.iter().any(|(key, _)| *key == Key::OtherFiveHour);
    let has_other_weekly = metrics.iter().any(|(key, _)| *key == Key::OtherWeekly);
    if has_other_five_hour != has_other_weekly {
        return Err(CollectionError::UnrecognizedOutput);
    }
    let metrics = metrics
        .into_iter()
        .filter(|(key, _)| matches!(key, Key::GeminiFiveHour | Key::GeminiWeekly))
        .map(|(_, metric)| metric)
        .collect::<Vec<_>>();
    if metrics.len() != 2 {
        return Err(CollectionError::UnrecognizedOutput);
    }
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
        antigravity_cli_info: None,
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
