use std::path::PathBuf;

const PRODUCT_DIRECTORY: &str = "AI Token Meter";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppStoragePaths {
    pub settings_file: PathBuf,
    pub cache_directory: PathBuf,
}

impl AppStoragePaths {
    pub fn from_windows_roots(roaming_app_data: PathBuf, local_app_data: PathBuf) -> Self {
        Self {
            settings_file: roaming_app_data
                .join(PRODUCT_DIRECTORY)
                .join("settings.json"),
            cache_directory: local_app_data.join(PRODUCT_DIRECTORY).join("cache"),
        }
    }

    #[cfg(windows)]
    pub fn discover() -> Result<Self, super::PersistenceError> {
        let roaming = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .ok_or(super::PersistenceError::MissingEnvironment("APPDATA"))?;
        let local = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or(super::PersistenceError::MissingEnvironment("LOCALAPPDATA"))?;
        Ok(Self::from_windows_roots(roaming, local))
    }
}
