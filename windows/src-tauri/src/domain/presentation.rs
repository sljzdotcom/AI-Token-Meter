use std::error::Error;

use serde::{Deserialize, Serialize};

use super::ProviderId;

const PROVIDER_CONTRACT: &str = include_str!("../../../../contracts/presentation/providers.json");

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderPresentation {
    pub id: ProviderId,
    pub display_name: String,
    pub logo_key: String,
    pub accent_color: String,
    pub progress_semantics: ProgressSemantics,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProgressSemantics {
    UsedQuota,
    ConsumedFromBalanceBaseline,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PresentationContract {
    schema_version: u64,
    providers: Vec<ProviderPresentation>,
}

pub fn embedded_provider_presentations()
-> Result<Vec<ProviderPresentation>, Box<dyn Error + Send + Sync>> {
    let contract: PresentationContract = serde_json::from_str(PROVIDER_CONTRACT)?;
    if contract.schema_version != 1 {
        return Err(format!(
            "unsupported presentation schema version {}",
            contract.schema_version
        )
        .into());
    }
    Ok(contract.providers)
}
