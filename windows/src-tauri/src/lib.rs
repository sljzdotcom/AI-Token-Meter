use serde::Deserialize;

pub mod accounts;
pub mod collectors;
pub mod domain;
pub mod persistence;
pub mod platform;
pub mod security;

const PRODUCT_NAME: &str = "AI Token Meter";
const SHARED_VERSION: &str = include_str!("../../../VERSION");
const PROVIDER_CONTRACT: &str = include_str!("../../../contracts/presentation/providers.json");

#[derive(Debug, PartialEq, Eq)]
pub struct AppMetadata {
    pub product_name: &'static str,
    pub version: String,
    pub providers: [String; 3],
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PresentationContract {
    providers: Vec<ProviderContract>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderContract {
    display_name: String,
}

pub fn app_metadata() -> Result<AppMetadata, serde_json::Error> {
    let contract: PresentationContract = serde_json::from_str(PROVIDER_CONTRACT)?;
    let providers = contract
        .providers
        .into_iter()
        .map(|provider| provider.display_name)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| {
            serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "the shared contract must contain exactly three providers",
            ))
        })?;

    Ok(AppMetadata {
        product_name: PRODUCT_NAME,
        version: SHARED_VERSION.trim().to_owned(),
        providers,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run AI Token Meter")
}
