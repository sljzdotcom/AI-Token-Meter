use super::CollectionError;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

pub struct GeminiEnvironment {
    pub directory: PathBuf,
    pub variables: Vec<(OsString, OsString)>,
}

impl GeminiEnvironment {
    pub fn prepare(
        parent: &Path,
        inherited: Vec<(OsString, OsString)>,
        _legacy_system_paths: &[PathBuf],
    ) -> Result<Self, CollectionError> {
        let inherited: BTreeMap<String, OsString> = inherited
            .into_iter()
            .map(|(key, value)| (key.to_string_lossy().to_ascii_uppercase(), value))
            .collect();
        for (key, value) in &inherited {
            if !value.is_empty() && is_unsafe_override(key) {
                return Err(CollectionError::UnsupportedConfiguration);
            }
        }

        let home = inherited
            .get("USERPROFILE")
            .or_else(|| inherited.get("HOME"))
            .map(PathBuf::from)
            .ok_or(CollectionError::UnsupportedConfiguration)?;
        if inherited
            .get("HOME")
            .is_some_and(|candidate| Path::new(candidate) != home)
        {
            return Err(CollectionError::UnsupportedConfiguration);
        }

        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|_| CollectionError::Transport)?;
        let directory = parent.join(format!(
            "ai-meter-antigravity-{}",
            random
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ));
        fs::create_dir(&directory).map_err(|_| CollectionError::Transport)?;
        if fs::write(directory.join(".env"), "").is_err() {
            let _ = fs::remove_dir_all(&directory);
            return Err(CollectionError::Transport);
        }

        let mut variables = Vec::new();
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
            if let Some(value) = inherited.get(key) {
                variables.push((key.into(), value.clone()));
            }
        }
        variables.extend([
            ("NO_BROWSER".into(), "true".into()),
            ("TERM".into(), "dumb".into()),
            ("LANG".into(), "en_US.UTF-8".into()),
            ("TMPDIR".into(), directory.clone().into_os_string()),
        ]);

        Ok(Self {
            directory,
            variables,
        })
    }

    pub fn arguments(&self, version: bool) -> Vec<String> {
        let mut arguments = if version {
            vec!["--version".into()]
        } else {
            vec![
                "-p".into(),
                "/usage".into(),
                "--print-timeout".into(),
                "20s".into(),
            ]
        };
        arguments.extend([
            "--log-file".into(),
            self.directory
                .join("agy.log")
                .to_string_lossy()
                .into_owned(),
        ]);
        arguments
    }
}

fn is_unsafe_override(key: &str) -> bool {
    key.starts_with("AGY_")
        || key.starts_with("GEMINI_")
        || key.starts_with("GOOGLE_")
        || key.starts_with("GCLOUD_")
        || key.starts_with("CLOUDSDK_")
        || key.starts_with("NODE_")
        || [
            "NPM_CONFIG_NODE_OPTIONS",
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "SSL_CERT_FILE",
            "NODE_EXTRA_CA_CERTS",
            "NODE_TLS_REJECT_UNAUTHORIZED",
            "DYLD_INSERT_LIBRARIES",
            "DYLD_LIBRARY_PATH",
            "LD_PRELOAD",
            "BASH_ENV",
            "ENV",
        ]
        .contains(&key)
}

impl Drop for GeminiEnvironment {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}
