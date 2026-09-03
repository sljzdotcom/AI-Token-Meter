use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

use async_trait::async_trait;
use zeroize::Zeroizing;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialAccount {
    DeepSeekApiKey,
}

#[derive(Clone)]
pub struct SecretString(Zeroizing<String>);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(Zeroizing::new(value.into()))
    }

    pub fn expose(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_trimmed(self) -> Self {
        Self(Zeroizing::new(self.0.trim().to_owned()))
    }
}

impl Debug for SecretString {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SecretString([REDACTED])")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialStoreError {
    Unavailable,
    InvalidData,
    Platform(u32),
}

impl Display for CredentialStoreError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("credential store unavailable"),
            Self::InvalidData => formatter.write_str("credential data is invalid"),
            Self::Platform(code) => write!(formatter, "credential platform error {code}"),
        }
    }
}

impl Error for CredentialStoreError {}

#[async_trait]
pub trait CredentialStore: Send + Sync {
    async fn read(
        &self,
        account: CredentialAccount,
    ) -> Result<Option<SecretString>, CredentialStoreError>;

    async fn replace_verified(
        &self,
        account: CredentialAccount,
        secret: SecretString,
    ) -> Result<(), CredentialStoreError>;

    async fn delete(&self, account: CredentialAccount) -> Result<(), CredentialStoreError>;
}
