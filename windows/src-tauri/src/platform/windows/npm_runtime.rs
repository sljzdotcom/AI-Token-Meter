use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Deserialize;

const PACKAGE_JSON_LIMIT: u64 = 64 * 1024;

#[derive(Deserialize)]
struct PackageIdentity {
    name: String,
    bin: PackageBin,
}

#[derive(Deserialize)]
struct PackageBin {
    #[serde(default)]
    codex: String,
    #[serde(default)]
    gemini: String,
}

pub(super) fn resolve_provider_npm_runtime<'a>(
    wrapper: &Path,
    provider: crate::accounts::cli_account::CliProvider,
    node_directories: impl IntoIterator<Item = &'a PathBuf>,
) -> Option<(PathBuf, PathBuf)> {
    let npm_root = wrapper.parent()?;
    let (scope, package, entry_path) = match provider {
        crate::accounts::cli_account::CliProvider::Codex => ("@openai", "codex", "bin/codex.js"),
        crate::accounts::cli_account::CliProvider::Gemini => {
            ("@google", "gemini-cli", "bundle/gemini.js")
        }
        _ => return None,
    };
    let package_root = npm_root.join("node_modules").join(scope).join(package);
    let identity = read_package_identity(&package_root.join("package.json"))?;
    let bin = if provider == crate::accounts::cli_account::CliProvider::Gemini {
        &identity.bin.gemini
    } else {
        &identity.bin.codex
    };
    if identity.name != format!("{scope}/{package}") || bin != entry_path {
        return None;
    }
    let entry = package_root.join(entry_path).canonicalize().ok()?;
    if !fs::metadata(&entry).ok()?.is_file() {
        return None;
    }

    let node = std::iter::once(npm_root.to_owned())
        .chain(node_directories.into_iter().cloned())
        .take(super::executable_locator::MAX_NODE_DIRECTORIES)
        .find_map(|directory| canonical_file(&directory.join("node.exe")))?;
    Some((entry, node))
}

fn read_package_identity(path: &Path) -> Option<PackageIdentity> {
    let file = File::open(path).ok()?;
    if file.metadata().ok()?.len() > PACKAGE_JSON_LIMIT {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(PACKAGE_JSON_LIMIT + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > PACKAGE_JSON_LIMIT {
        return None;
    }
    serde_json::from_slice(&bytes).ok()
}

fn canonical_file(path: &Path) -> Option<PathBuf> {
    let path = path.canonicalize().ok()?;
    fs::metadata(&path).ok()?.is_file().then_some(path)
}
