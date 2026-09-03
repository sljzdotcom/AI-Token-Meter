use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::accounts::cli_account::CliProvider;

use super::environment::DiscoveryInputs;
use super::wsl::{build_wsl_list_invocation, decode_distribution_list};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeSource {
    NativeWindows,
    Wsl { distribution: String },
}

impl RuntimeSource {
    pub fn may_read_windows_profile(&self) -> bool {
        matches!(self, Self::NativeWindows)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateOrigin {
    Custom,
    ProcessPath,
    UserRegistryPath,
    SystemRegistryPath,
    Conventional,
    DesktopApplication,
    Wsl,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutableCandidate {
    pub executable: PathBuf,
    pub launcher: Option<PathBuf>,
    pub source: RuntimeSource,
    pub origin: CandidateOrigin,
}

pub struct ExecutableLocator {
    inputs: DiscoveryInputs,
}

const MAX_DIRECTORIES_PER_SOURCE: usize = 128;
const MAX_WSL_DISTRIBUTIONS: usize = 64;
const SHEBANG_READ_LIMIT: u64 = 256;

pub struct DiscoveryBudget {
    deadline: Instant,
    remaining_processes: usize,
}

impl DiscoveryBudget {
    pub fn new(total_timeout: Duration, maximum_processes: usize) -> Self {
        Self {
            deadline: Instant::now() + total_timeout,
            remaining_processes: maximum_processes,
        }
    }

    pub fn next_process_timeout(&mut self, per_process_limit: Duration) -> Option<Duration> {
        if self.remaining_processes == 0 {
            return None;
        }
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return None;
        }
        self.remaining_processes -= 1;
        Some(remaining.min(per_process_limit))
    }
}

impl ExecutableLocator {
    pub fn new(inputs: DiscoveryInputs) -> Self {
        Self { inputs }
    }

    pub fn locate<F>(
        &self,
        provider: CliProvider,
        mut health_check: F,
    ) -> Option<ExecutableCandidate>
    where
        F: FnMut(&ExecutableCandidate) -> bool,
    {
        self.locate_native(provider, &mut health_check)
    }

    pub fn validate_custom<F>(
        &self,
        provider: CliProvider,
        path: &Path,
        mut health_check: F,
    ) -> Option<ExecutableCandidate>
    where
        F: FnMut(&ExecutableCandidate) -> bool,
    {
        let candidate =
            self.validate_candidate(path.to_owned(), provider, CandidateOrigin::Custom)?;
        health_check(&candidate).then_some(candidate)
    }

    pub fn locate_with_wsl_output<F>(
        &self,
        provider: CliProvider,
        distribution_output: Option<&[u8]>,
        mut health_check: F,
    ) -> Option<ExecutableCandidate>
    where
        F: FnMut(&ExecutableCandidate) -> bool,
    {
        if let Some(candidate) = self.locate_native(provider, &mut health_check) {
            return Some(candidate);
        }

        self.locate_wsl_with_output(provider, distribution_output?, None, health_check)
    }

    pub fn locate_wsl_with_output<F>(
        &self,
        _provider: CliProvider,
        distribution_output: &[u8],
        preferred_distribution: Option<&str>,
        mut health_check: F,
    ) -> Option<ExecutableCandidate>
    where
        F: FnMut(&ExecutableCandidate) -> bool,
    {
        let executable = build_wsl_list_invocation(self.inputs.system_root.as_ref()?)?.executable;
        let mut distributions = decode_distribution_list(distribution_output);
        if let Some(preferred) = preferred_distribution
            && let Some(index) = distributions
                .iter()
                .position(|distribution| distribution == preferred)
        {
            let selected = distributions.remove(index);
            distributions.insert(0, selected);
        }
        if preferred_distribution.is_some() {
            distributions
                .retain(|distribution| Some(distribution.as_str()) == preferred_distribution);
        }
        for distribution in distributions.into_iter().take(MAX_WSL_DISTRIBUTIONS) {
            let candidate = ExecutableCandidate {
                executable: executable.clone(),
                launcher: None,
                source: RuntimeSource::Wsl { distribution },
                origin: CandidateOrigin::Wsl,
            };
            if health_check(&candidate) {
                return Some(candidate);
            }
        }
        None
    }

    fn locate_native<F>(
        &self,
        provider: CliProvider,
        health_check: &mut F,
    ) -> Option<ExecutableCandidate>
    where
        F: FnMut(&ExecutableCandidate) -> bool,
    {
        let mut seen = HashSet::new();
        for (path, origin) in self.paths_in_priority_order(provider) {
            let Some(candidate) = self.validate_candidate(path, provider, origin) else {
                continue;
            };
            if seen.insert(candidate.executable.clone()) && health_check(&candidate) {
                return Some(candidate);
            }
        }
        None
    }

    fn paths_in_priority_order(&self, provider: CliProvider) -> Vec<(PathBuf, CandidateOrigin)> {
        let mut paths = Vec::new();
        if let Some(path) = self.inputs.custom_path.clone() {
            paths.push((path, CandidateOrigin::Custom));
        }
        append_directory_candidates(
            &mut paths,
            &self.inputs.process_paths,
            provider,
            CandidateOrigin::ProcessPath,
        );
        append_directory_candidates(
            &mut paths,
            &self.inputs.user_registry_paths,
            provider,
            CandidateOrigin::UserRegistryPath,
        );
        append_directory_candidates(
            &mut paths,
            &self.inputs.system_registry_paths,
            provider,
            CandidateOrigin::SystemRegistryPath,
        );
        append_directory_candidates(
            &mut paths,
            &self.inputs.conventional_paths,
            provider,
            CandidateOrigin::Conventional,
        );
        append_directory_candidates(
            &mut paths,
            &self.inputs.desktop_application_paths,
            provider,
            CandidateOrigin::DesktopApplication,
        );
        paths
    }

    fn validate_candidate(
        &self,
        path: PathBuf,
        provider: CliProvider,
        origin: CandidateOrigin,
    ) -> Option<ExecutableCandidate> {
        let executable = path.canonicalize().ok()?;
        if !fs::metadata(&executable).ok()?.is_file() || !name_matches(&executable, provider) {
            return None;
        }

        let launcher = match executable.extension().and_then(|value| value.to_str()) {
            Some(extension) if extension.eq_ignore_ascii_case("exe") => None,
            Some(extension) if extension.eq_ignore_ascii_case("cmd") => {
                Some(self.command_interpreter()?)
            }
            None if has_env_node_shebang(&executable) => Some(find_node_launcher(&executable)?),
            _ => return None,
        };

        Some(ExecutableCandidate {
            executable,
            launcher,
            source: RuntimeSource::NativeWindows,
            origin,
        })
    }

    fn command_interpreter(&self) -> Option<PathBuf> {
        let path = self
            .inputs
            .system_root
            .as_ref()?
            .join("System32")
            .join("cmd.exe")
            .canonicalize()
            .ok()?;
        fs::metadata(&path).ok()?.is_file().then_some(path)
    }
}

fn append_directory_candidates(
    output: &mut Vec<(PathBuf, CandidateOrigin)>,
    directories: &[PathBuf],
    provider: CliProvider,
    origin: CandidateOrigin,
) {
    for directory in directories.iter().take(MAX_DIRECTORIES_PER_SOURCE) {
        for name in provider.executable_names() {
            output.push((directory.join(name), origin));
        }
    }
}

fn name_matches(path: &Path, provider: CliProvider) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    provider
        .executable_names()
        .iter()
        .any(|expected| name.eq_ignore_ascii_case(expected))
}

fn has_env_node_shebang(path: &Path) -> bool {
    let Ok(file) = File::open(path) else {
        return false;
    };
    let mut bytes = Vec::with_capacity(SHEBANG_READ_LIMIT as usize);
    if file
        .take(SHEBANG_READ_LIMIT)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return false;
    }
    let first_line = bytes
        .split(|byte| *byte == b'\n')
        .next()
        .unwrap_or_default();
    first_line == b"#!/usr/bin/env node" || first_line == b"#!/usr/bin/env node\r"
}

fn find_node_launcher(script: &Path) -> Option<PathBuf> {
    let mut directory = script.parent();
    for _ in 0..=4 {
        let current = directory?;
        for candidate in [
            current.join("node.exe"),
            current.join("bin").join("node.exe"),
        ] {
            let Ok(candidate) = candidate.canonicalize() else {
                continue;
            };
            if fs::metadata(&candidate).is_ok_and(|metadata| metadata.is_file()) {
                return Some(candidate);
            }
        }
        directory = current.parent();
    }
    None
}
