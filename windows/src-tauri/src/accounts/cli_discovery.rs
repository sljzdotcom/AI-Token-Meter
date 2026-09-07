use super::cli_account::CliProvider;
use crate::persistence::{CliRuntimeMode, ProviderCliSettings};
use crate::platform::windows::environment::DiscoveryInputs;
use crate::platform::windows::executable_locator::{ExecutableCandidate, ExecutableLocator};

#[derive(Debug)]
pub enum CliDiscovery {
    Found(ExecutableCandidate),
    Missing,
    Unavailable,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CliProbe {
    Healthy,
    Missing,
    Unavailable,
}

/// A native file rejected by health checks still exists; it must never authorize replacement.
pub fn discover_cli(
    provider: CliProvider,
    settings: &ProviderCliSettings,
    inputs: DiscoveryInputs,
    mut probe: impl FnMut(&ExecutableCandidate) -> CliProbe,
    mut wsl_list: impl FnMut() -> Result<Option<Vec<u8>>, ()>,
) -> CliDiscovery {
    let locator = ExecutableLocator::new(inputs);
    if settings.mode != CliRuntimeMode::Wsl {
        if let Some(path) = settings
            .custom_path
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty())
        {
            return locator
                .validate_custom(provider, std::path::Path::new(path), |c| {
                    probe(c) == CliProbe::Healthy
                })
                .map(CliDiscovery::Found)
                .unwrap_or(CliDiscovery::Unavailable);
        }
        if let Some(candidate) = locator.locate(provider, |c| probe(c) == CliProbe::Healthy) {
            return CliDiscovery::Found(candidate);
        }
        if locator.has_native_candidates_or_incomplete_search(provider) {
            return CliDiscovery::Unavailable;
        }
        if settings.mode == CliRuntimeMode::NativeWindows {
            return CliDiscovery::Missing;
        }
    }
    let output = match wsl_list() {
        Ok(Some(output)) => output,
        Ok(None) if settings.mode == CliRuntimeMode::Auto => return CliDiscovery::Missing,
        _ => return CliDiscovery::Unavailable,
    };
    let mut unavailable = false;
    let mut probed = false;
    let candidate = locator.locate_wsl_with_output(
        provider,
        &output,
        if settings.mode == CliRuntimeMode::Wsl {
            settings.wsl_distribution.as_deref()
        } else {
            None
        },
        |c| {
            probed = true;
            match probe(c) {
                CliProbe::Healthy => true,
                CliProbe::Missing => false,
                CliProbe::Unavailable => {
                    unavailable = true;
                    false
                }
            }
        },
    );
    if let Some(candidate) = candidate {
        CliDiscovery::Found(candidate)
    } else if unavailable || (settings.mode == CliRuntimeMode::Wsl && !probed) {
        CliDiscovery::Unavailable
    } else {
        CliDiscovery::Missing
    }
}
