use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiscoveryInputs {
    pub custom_path: Option<PathBuf>,
    pub process_paths: Vec<PathBuf>,
    pub user_registry_paths: Vec<PathBuf>,
    pub system_registry_paths: Vec<PathBuf>,
    pub conventional_paths: Vec<PathBuf>,
    pub desktop_application_paths: Vec<PathBuf>,
    pub system_root: Option<PathBuf>,
}

impl DiscoveryInputs {
    #[cfg(windows)]
    pub fn capture(custom_path: Option<PathBuf>) -> Self {
        let process_paths = std::env::var_os("PATH")
            .map(|value| std::env::split_paths(&value).collect())
            .unwrap_or_default();
        let user_registry_paths = registry_path(winreg::enums::HKEY_CURRENT_USER, "Environment");
        let system_registry_paths = registry_path(
            winreg::enums::HKEY_LOCAL_MACHINE,
            r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
        );
        let system_root = std::env::var_os("SystemRoot").map(PathBuf::from);
        let user_profile = std::env::var_os("USERPROFILE").map(PathBuf::from);
        let app_data = std::env::var_os("APPDATA").map(PathBuf::from);
        let local_app_data = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);

        let mut conventional_paths = Vec::new();
        if let Some(path) = app_data.as_ref() {
            conventional_paths.push(path.join("npm"));
        }
        if let Some(path) = user_profile.as_ref() {
            conventional_paths.push(path.join(".volta").join("bin"));
        }
        for variable in ["NVM_HOME", "NVM_SYMLINK", "FNM_MULTISHELL_PATH"] {
            if let Some(path) = std::env::var_os(variable) {
                conventional_paths.push(PathBuf::from(path));
            }
        }
        if let Some(path) = local_app_data.as_ref() {
            conventional_paths.push(path.join("fnm"));
            append_child_directories(&path.join("fnm_multishells"), &mut conventional_paths);
        }

        let mut desktop_application_paths = Vec::new();
        if let Some(path) = local_app_data {
            desktop_application_paths.push(path.join("Programs").join("Claude"));
            desktop_application_paths.push(path.join("Programs").join("OpenAI"));
            desktop_application_paths.push(path.join("Programs").join("ChatGPT"));
        }

        Self {
            custom_path,
            process_paths,
            user_registry_paths,
            system_registry_paths,
            conventional_paths,
            desktop_application_paths,
            system_root,
        }
    }
}

#[cfg(windows)]
fn registry_path(root: winreg::HKEY, subkey: &str) -> Vec<PathBuf> {
    use winreg::RegKey;

    let key = match RegKey::predef(root).open_subkey(subkey) {
        Ok(key) => key,
        Err(_) => return Vec::new(),
    };
    let value: String = match key.get_value("Path") {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let expanded = expand_known_environment_variables(&value);
    std::env::split_paths(&expanded).collect()
}

#[cfg(windows)]
fn expand_known_environment_variables(value: &str) -> std::ffi::OsString {
    let mut expanded = value.to_owned();
    for (key, environment_value) in std::env::vars_os() {
        let Some(key) = key.to_str() else { continue };
        let Some(environment_value) = environment_value.to_str() else {
            continue;
        };
        expanded = expanded.replace(&format!("%{key}%"), environment_value);
        expanded = expanded.replace(
            &format!("%{}%", key.to_ascii_uppercase()),
            environment_value,
        );
        expanded = expanded.replace(
            &format!("%{}%", key.to_ascii_lowercase()),
            environment_value,
        );
    }
    expanded.into()
}

#[cfg(windows)]
fn append_child_directories(root: &std::path::Path, output: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    output.extend(
        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir()),
    );
}
