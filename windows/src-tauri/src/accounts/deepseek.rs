use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

use crate::collectors::deepseek::{
    DeepSeekBalanceClient, DeepSeekClientError, snapshot_from_balance,
};
use crate::domain::UsageSnapshot;
use crate::security::{CredentialAccount, CredentialStore, SecretString};

pub struct DeepSeekAccountService<S> {
    credentials: Arc<S>,
    client: DeepSeekBalanceClient,
    balance_baseline: f64,
}

impl<S> DeepSeekAccountService<S>
where
    S: CredentialStore,
{
    pub fn new(credentials: Arc<S>, client: DeepSeekBalanceClient, balance_baseline: f64) -> Self {
        Self {
            credentials,
            client,
            balance_baseline: balance_baseline.max(1.0),
        }
    }

    pub async fn replace_key(
        &self,
        candidate: &str,
        fetched_at: &str,
    ) -> Result<UsageSnapshot, DeepSeekReplacementError> {
        let candidate = candidate.trim();
        if candidate.is_empty() {
            return Err(DeepSeekReplacementError::EmptyCandidate);
        }
        let candidate = SecretString::new(candidate);
        let balance = self
            .client
            .fetch_balance(&candidate)
            .await
            .map_err(DeepSeekReplacementError::from_client)?;
        let account = CredentialAccount::DeepSeekApiKey;
        let previous = self
            .credentials
            .read(account)
            .await
            .map_err(|_| DeepSeekReplacementError::CredentialUnavailable)?;

        if self
            .credentials
            .replace_verified(account, candidate)
            .await
            .is_err()
        {
            if let Some(previous) = previous {
                let _ = self.credentials.replace_verified(account, previous).await;
            } else {
                let _ = self.credentials.delete(account).await;
            }
            return Err(DeepSeekReplacementError::CredentialUnavailable);
        }

        Ok(snapshot_from_balance(
            balance,
            self.balance_baseline,
            fetched_at,
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeepSeekReplacementError {
    EmptyCandidate,
    InvalidKey,
    VerificationUnavailable,
    CredentialUnavailable,
}

impl DeepSeekReplacementError {
    fn from_client(error: DeepSeekClientError) -> Self {
        match error {
            DeepSeekClientError::AuthenticationRequired => Self::InvalidKey,
            _ => Self::VerificationUnavailable,
        }
    }
}

impl Display for DeepSeekReplacementError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::EmptyCandidate => "API Key is required",
            Self::InvalidKey => "DeepSeek rejected this API Key",
            Self::VerificationUnavailable => "DeepSeek verification is currently unavailable",
            Self::CredentialUnavailable => "Windows Credential Manager is unavailable",
        })
    }
}

impl Error for DeepSeekReplacementError {}
