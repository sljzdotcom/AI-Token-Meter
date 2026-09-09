//! Pinned Gemini startup policy. Reads settings metadata only, never OAuth content.
use super::CollectionError;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

pub struct GeminiEnvironment {
    pub directory: PathBuf,
    pub variables: Vec<(OsString, OsString)>,
    allow_name: String,
}
impl GeminiEnvironment {
    pub fn prepare(
        parent: &Path,
        inherited: Vec<(OsString, OsString)>,
        system_paths: &[PathBuf],
    ) -> Result<Self, CollectionError> {
        let env: BTreeMap<String, OsString> = inherited
            .into_iter()
            .map(|(key, value)| (key.to_string_lossy().to_ascii_uppercase(), value))
            .collect();
        for (key, value) in &env {
            if !value.is_empty()
                && (key.starts_with("GEMINI_")
                    || key.starts_with("GOOGLE_")
                    || [
                        "NODE_OPTIONS",
                        "NODE_PATH",
                        "NPM_CONFIG_NODE_OPTIONS",
                        "HTTP_PROXY",
                        "HTTPS_PROXY",
                        "ALL_PROXY",
                        "SSL_CERT_FILE",
                        "NODE_EXTRA_CA_CERTS",
                        "NODE_TLS_REJECT_UNAUTHORIZED",
                        "CLOUD_SHELL",
                        "SANDBOX",
                        "BUILD_SANDBOX",
                        "SEATBELT_PROFILE",
                        "SANDBOX_FLAGS",
                        "SANDBOX_MOUNTS",
                        "SANDBOX_ENV",
                    ]
                    .contains(&key.as_str()))
            {
                return Err(CollectionError::UnsupportedConfiguration);
            }
        }
        if system_paths
            .iter()
            .any(|p| !matches!(p.try_exists(), Ok(false)))
        {
            return Err(CollectionError::UnsupportedConfiguration);
        }
        let home = env
            .get("USERPROFILE")
            .or_else(|| env.get("HOME"))
            .map(PathBuf::from)
            .ok_or(CollectionError::UnsupportedConfiguration)?;
        if env.get("HOME").is_some_and(|h| Path::new(h) != home) {
            return Err(CollectionError::UnsupportedConfiguration);
        }
        let settings_path = home.join(".gemini/settings.json");
        match File::open(&settings_path) {
            Ok(mut file) => {
                if file
                    .metadata()
                    .map_err(|_| CollectionError::UnsupportedConfiguration)?
                    .len()
                    > 64 * 1024
                {
                    return Err(CollectionError::UnsupportedConfiguration);
                }
                let mut bytes = Vec::new();
                file.by_ref()
                    .take(64 * 1024 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|_| CollectionError::UnsupportedConfiguration)?;
                let settings: serde_json::Value = serde_json::from_slice(&bytes)
                    .map_err(|_| CollectionError::UnsupportedConfiguration)?;
                if !settings.is_object() {
                    return Err(CollectionError::UnsupportedConfiguration);
                }
                for pointer in ["/security", "/security/auth", "/tools", "/advanced"] {
                    if settings
                        .pointer(pointer)
                        .is_some_and(|value| !value.is_object())
                    {
                        return Err(CollectionError::UnsupportedConfiguration);
                    }
                }
                for pointer in ["/security/auth/selectedType", "/security/auth/enforcedType"] {
                    if let Some(value) = settings.pointer(pointer)
                        && value.as_str() != Some("oauth-personal")
                    {
                        return Err(CollectionError::UnsupportedConfiguration);
                    }
                }
                for pointer in [
                    "/advanced/ignoreLocalEnv",
                    "/tools/sandbox",
                    "/security/toolSandboxing",
                    "/security/auth/useExternal",
                ] {
                    if let Some(value) = settings.pointer(pointer)
                        && value != &serde_json::Value::Bool(false)
                        && !value.is_null()
                    {
                        return Err(CollectionError::UnsupportedConfiguration);
                    }
                }
                for pointer in ["/tools/discoveryCommand", "/tools/callCommand"] {
                    if settings
                        .pointer(pointer)
                        .is_some_and(|v| !v.is_null() && v.as_str() != Some(""))
                    {
                        return Err(CollectionError::UnsupportedConfiguration);
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(CollectionError::UnsupportedConfiguration),
        }
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|_| CollectionError::Transport)?;
        let allow_name = format!(
            "ai-meter-{}",
            random
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        let directory = parent.join(&allow_name);
        fs::create_dir(&directory).map_err(|_| CollectionError::Transport)?;
        let mut result = Self {
            directory,
            variables: Vec::new(),
            allow_name,
        };
        fs::write(result.directory.join(".env"), "").map_err(|_| CollectionError::Transport)?;
        let policy = serde_json::json!({"hooksConfig":{"enabled":false},"admin":{"mcp":{"enabled":false},"extensions":{"enabled":false},"skills":{"enabled":false}},"context":{"memoryBoundaryMarkers":[],"includeDirectories":[],"fileName":[]},"privacy":{"usageStatisticsEnabled":false},"telemetry":{"enabled":false},"ide":{"enabled":false},"general":{"enableAutoUpdate":false},"advanced":{"autoConfigureMemory":false}});
        fs::write(result.directory.join("system.json"), policy.to_string())
            .map_err(|_| CollectionError::Transport)?;
        fs::write(result.directory.join("system-defaults.json"), "{}")
            .map_err(|_| CollectionError::Transport)?;
        for key in [
            "HOME",
            "USERPROFILE",
            "APPDATA",
            "LOCALAPPDATA",
            "PATH",
            "SYSTEMROOT",
            "WINDIR",
            "COMSPEC",
            "PATHEXT",
            "TEMP",
            "TMP",
        ] {
            if let Some(value) = env.get(key) {
                result.variables.push((key.into(), value.clone()));
            }
        }
        for (key, value) in [
            ("NO_BROWSER", "true"),
            ("GEMINI_CLI_NO_RELAUNCH", "true"),
            ("GEMINI_CLI_TRUST_WORKSPACE", "false"),
            ("TERM", "xterm-256color"),
            ("LANG", "en_US.UTF-8"),
        ] {
            result.variables.push((key.into(), value.into()));
        }
        result.variables.push((
            "GEMINI_CLI_SYSTEM_SETTINGS_PATH".into(),
            result.directory.join("system.json").into_os_string(),
        ));
        result.variables.push((
            "GEMINI_CLI_SYSTEM_DEFAULTS_PATH".into(),
            result
                .directory
                .join("system-defaults.json")
                .into_os_string(),
        ));
        Ok(result)
    }
    pub fn arguments(&self, version: bool) -> Vec<String> {
        let mut args = vec![
            "-e".into(),
            "none".into(),
            "--allowed-mcp-server-names".into(),
            self.allow_name.clone(),
        ];
        if version {
            args.push("--version".into());
        }
        args
    }
}
impl Drop for GeminiEnvironment {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}
