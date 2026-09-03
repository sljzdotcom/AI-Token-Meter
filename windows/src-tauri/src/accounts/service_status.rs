use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::ProviderId;
use crate::platform::windows::executable_locator::RuntimeSource;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ServiceAccountConnectionState {
    Connected,
    SignInRequired,
    NotInstalled,
    Checking,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccountStatus {
    pub provider_id: ProviderId,
    pub connection_state: ServiceAccountConnectionState,
    pub account_label: Option<String>,
    pub account_detail: Option<String>,
    pub runtime_source: Option<String>,
    pub cli_version: Option<String>,
    pub checked_at: Option<String>,
}

impl ServiceAccountStatus {
    pub fn unavailable(provider_id: ProviderId, checked_at: &str) -> Self {
        Self::new(
            provider_id,
            ServiceAccountConnectionState::Unavailable,
            checked_at,
        )
    }

    pub fn not_installed(provider_id: ProviderId, checked_at: &str) -> Self {
        Self::new(
            provider_id,
            ServiceAccountConnectionState::NotInstalled,
            checked_at,
        )
    }

    fn new(
        provider_id: ProviderId,
        connection_state: ServiceAccountConnectionState,
        checked_at: &str,
    ) -> Self {
        Self {
            provider_id,
            connection_state,
            account_label: None,
            account_detail: None,
            runtime_source: None,
            cli_version: None,
            checked_at: Some(checked_at.to_owned()),
        }
    }

    pub fn with_runtime(mut self, source: &RuntimeSource, version: Option<String>) -> Self {
        self.runtime_source = Some(runtime_source_label(source));
        self.cli_version = version.and_then(|value| normalized_visible_value(&value, 48));
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceAccountParseError {
    InvalidResponse,
}

pub fn parse_claude_auth_status(
    output: &str,
    checked_at: &str,
) -> Result<ServiceAccountStatus, ServiceAccountParseError> {
    let start = output
        .find('{')
        .ok_or(ServiceAccountParseError::InvalidResponse)?;
    let end = output
        .rfind('}')
        .ok_or(ServiceAccountParseError::InvalidResponse)?;
    let response: ClaudeAuthResponse = serde_json::from_str(&output[start..=end])
        .map_err(|_| ServiceAccountParseError::InvalidResponse)?;
    if !response.logged_in {
        return Ok(ServiceAccountStatus::new(
            ProviderId::Claude,
            ServiceAccountConnectionState::SignInRequired,
            checked_at,
        ));
    }

    let method = response
        .auth_method
        .as_deref()
        .and_then(display_auth_method);
    let subscription = response
        .subscription_type
        .as_deref()
        .and_then(display_title);
    let email = response
        .email
        .as_deref()
        .and_then(|value| normalized_visible_value(value, 160));
    let mut status = ServiceAccountStatus::new(
        ProviderId::Claude,
        ServiceAccountConnectionState::Connected,
        checked_at,
    );
    status.account_label = email
        .clone()
        .or_else(|| method.as_ref().map(|value| format!("{value} account")))
        .or_else(|| Some("Connected account".to_owned()));
    status.account_detail = if email.is_some() {
        joined_detail(method, subscription)
    } else {
        subscription
    };
    Ok(status)
}

pub fn parse_codex_account_status(
    output: &str,
    expected_id: u64,
    checked_at: &str,
) -> Result<ServiceAccountStatus, ServiceAccountParseError> {
    let response = output
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|value| value.get("id").and_then(Value::as_u64) == Some(expected_id))
        .ok_or(ServiceAccountParseError::InvalidResponse)?;
    let result = response
        .get("result")
        .and_then(Value::as_object)
        .ok_or(ServiceAccountParseError::InvalidResponse)?;
    let requires_auth = result
        .get("requiresOpenaiAuth")
        .and_then(Value::as_bool)
        .ok_or(ServiceAccountParseError::InvalidResponse)?;
    let account = result
        .get("account")
        .ok_or(ServiceAccountParseError::InvalidResponse)?;
    if account.is_null() {
        let mut status = ServiceAccountStatus::new(
            ProviderId::Codex,
            if requires_auth {
                ServiceAccountConnectionState::SignInRequired
            } else {
                ServiceAccountConnectionState::Connected
            },
            checked_at,
        );
        if !requires_auth {
            status.account_label = Some("Configured provider".to_owned());
        }
        return Ok(status);
    }
    let account = account
        .as_object()
        .ok_or(ServiceAccountParseError::InvalidResponse)?;
    let account_type = account
        .get("type")
        .and_then(Value::as_str)
        .ok_or(ServiceAccountParseError::InvalidResponse)?;
    let mut status = ServiceAccountStatus::new(
        ProviderId::Codex,
        ServiceAccountConnectionState::Connected,
        checked_at,
    );
    if account_type.eq_ignore_ascii_case("chatgpt") {
        status.account_label = account
            .get("email")
            .and_then(Value::as_str)
            .and_then(|value| normalized_visible_value(value, 160))
            .or_else(|| Some("ChatGPT account".to_owned()));
        status.account_detail = joined_detail(
            Some("ChatGPT".to_owned()),
            account
                .get("planType")
                .and_then(Value::as_str)
                .and_then(display_title),
        );
    } else if account_type.eq_ignore_ascii_case("apiKey") {
        status.account_label = Some("API Key account".to_owned());
    } else {
        status.account_label = display_title(account_type).map(|value| format!("{value} account"));
    }
    Ok(status)
}

pub fn deepseek_status(secret: Option<&str>, checked_at: &str) -> ServiceAccountStatus {
    let Some(secret) = secret.map(str::trim).filter(|value| !value.is_empty()) else {
        return ServiceAccountStatus::new(
            ProviderId::DeepSeek,
            ServiceAccountConnectionState::SignInRequired,
            checked_at,
        );
    };
    let suffix = secret.chars().rev().take(4).collect::<Vec<_>>();
    let suffix = suffix.into_iter().rev().collect::<String>();
    let mut status = ServiceAccountStatus::new(
        ProviderId::DeepSeek,
        ServiceAccountConnectionState::Connected,
        checked_at,
    );
    status.account_label = Some(format!("API Key ••••{suffix}"));
    status.account_detail = Some("Windows Credential Manager".to_owned());
    status
}

pub fn runtime_source_label(source: &RuntimeSource) -> String {
    match source {
        RuntimeSource::NativeWindows => "Native Windows".to_owned(),
        RuntimeSource::Wsl { distribution } => normalized_visible_value(distribution, 64)
            .map_or_else(|| "WSL".to_owned(), |value| format!("WSL · {value}")),
    }
}

pub fn sanitized_cli_version(output: &str) -> Option<String> {
    output.lines().next()?.split_whitespace().find_map(|token| {
        let token = token.trim_matches(|character: char| {
            !character.is_ascii_alphanumeric() && !matches!(character, '.' | '-')
        });
        let token = token.strip_prefix('v').unwrap_or(token);
        (token
            .chars()
            .next()
            .is_some_and(|value| value.is_ascii_digit())
            && token.chars().filter(|value| *value == '.').count() >= 2
            && token
                .chars()
                .all(|value| value.is_ascii_alphanumeric() || matches!(value, '.' | '-')))
        .then(|| token.chars().take(48).collect())
    })
}

fn display_auth_method(value: &str) -> Option<String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "none" => None,
        "oauth" | "oauth_token" => Some("OAuth".to_owned()),
        "claude.ai" | "claudeai" => Some("Claude Code".to_owned()),
        "api_key" | "apikey" | "api-key" => Some("API Key".to_owned()),
        _ => normalized_visible_value(value, 48),
    }
}

fn display_title(value: &str) -> Option<String> {
    let words = value
        .trim()
        .split(['_', '-', ' '])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut characters = word.chars();
            characters.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + characters.as_str()
            })
        })
        .collect::<Vec<_>>();
    normalized_visible_value(&words.join(" "), 64)
}

fn joined_detail(first: Option<String>, second: Option<String>) -> Option<String> {
    let detail = [first, second]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
    (!detail.is_empty()).then_some(detail)
}

fn normalized_visible_value(value: &str, max_characters: usize) -> Option<String> {
    let value = value
        .trim()
        .chars()
        .filter(|character| !character.is_control())
        .take(max_characters)
        .collect::<String>();
    (!value.is_empty()).then_some(value)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeAuthResponse {
    logged_in: bool,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    auth_method: Option<String>,
    #[serde(default)]
    subscription_type: Option<String>,
}
