use std::path::PathBuf;

use crate::accounts::cli_account::CliProvider;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WslInvocation {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
}

pub fn build_wsl_list_invocation(system_root: &std::path::Path) -> Option<WslInvocation> {
    let executable = system_root
        .join("System32")
        .join("wsl.exe")
        .canonicalize()
        .ok()?;
    std::fs::metadata(&executable)
        .is_ok_and(|metadata| metadata.is_file())
        .then_some(WslInvocation {
            executable,
            arguments: vec!["--list".to_owned(), "--quiet".to_owned()],
        })
}

pub fn decode_distribution_list(bytes: &[u8]) -> Vec<String> {
    let text = if looks_like_utf16_le(bytes) {
        let (pairs, _) = bytes.as_chunks::<2>();
        let words = pairs
            .iter()
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        String::from_utf16_lossy(&words)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };

    text.lines()
        .map(|line| line.trim_matches(['\0', '\u{feff}', ' ', '\t', '\r']))
        .map(|line| line.strip_prefix('*').unwrap_or(line).trim())
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

pub fn build_wsl_invocation(
    wsl_executable: PathBuf,
    distribution: &str,
    provider: CliProvider,
    provider_arguments: &[String],
) -> WslInvocation {
    let mut arguments = vec![
        "--distribution".to_owned(),
        distribution.to_owned(),
        "--exec".to_owned(),
        provider.command_name().to_owned(),
    ];
    arguments.extend_from_slice(provider_arguments);
    WslInvocation {
        executable: wsl_executable,
        arguments,
    }
}

pub fn wsl_profile_path(distribution: &str, home: &str) -> Option<PathBuf> {
    let distribution = distribution.trim();
    if distribution.is_empty()
        || distribution.len() > 128
        || distribution == "."
        || distribution == ".."
        || distribution.contains(['\\', '/', ':', '\0', '\n', '\r'])
    {
        return None;
    }
    let home = home.trim();
    if !home.starts_with('/') || home.contains(['\\', ':', '\0', '\n', '\r']) {
        return None;
    }
    let components = home
        .split('/')
        .filter(|component| !component.is_empty())
        .collect::<Vec<_>>();
    if components.is_empty()
        || components
            .iter()
            .any(|component| *component == "." || *component == "..")
    {
        return None;
    }
    Some(PathBuf::from(format!(
        r"\\wsl.localhost\{}\{}",
        distribution,
        components.join("\\")
    )))
}

fn looks_like_utf16_le(bytes: &[u8]) -> bool {
    let (pairs, remainder) = bytes.as_chunks::<2>();
    bytes.starts_with(&[0xff, 0xfe])
        || (bytes.len() >= 4
            && remainder.is_empty()
            && pairs.iter().filter(|chunk| chunk[1] == 0).count() * 2 >= bytes.len() / 2)
}
