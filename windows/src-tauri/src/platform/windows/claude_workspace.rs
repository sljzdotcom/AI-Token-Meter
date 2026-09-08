use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::accounts::cli_account::CliProvider;

use super::executable_locator::{ExecutableCandidate, RuntimeSource};
use super::process::{CommandBuildError, CommandInvocation, command_for_candidate};

pub const WSL_WORKSPACE: &str = "$HOME/.local/share/ai-token-meter/ClaudeUsageWorkspace";
const PREPARE_SCRIPT: &str =
    "umask 077 && mkdir -p -- \"$HOME/.local/share/ai-token-meter/ClaudeUsageWorkspace\"";
pub const SETUP_SCRIPT: &str = "set -eu\numask 077\nworkspace=\"$HOME/.local/share/ai-token-meter/ClaudeUsageWorkspace\"\nmkdir -p -- \"$workspace\"\ncd -- \"$workspace\"\nexec \"$@\"";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClaudeUsageWorkspace {
    Native(PathBuf),
    Wsl {
        executable: PathBuf,
        distribution: String,
    },
}

impl ClaudeUsageWorkspace {
    pub fn for_candidate(candidate: &ExecutableCandidate, local_app_data: &Path) -> Self {
        match &candidate.source {
            RuntimeSource::NativeWindows => Self::Native(
                local_app_data
                    .join("AI Token Meter")
                    .join("ClaudeUsageWorkspace"),
            ),
            RuntimeSource::Wsl { distribution } => Self::Wsl {
                executable: candidate.executable.clone(),
                distribution: distribution.clone(),
            },
        }
    }

    pub fn for_collection(candidate: &ExecutableCandidate, native_directory: &Path) -> Self {
        match &candidate.source {
            RuntimeSource::NativeWindows => Self::Native(native_directory.to_owned()),
            RuntimeSource::Wsl { distribution } => Self::Wsl {
                executable: candidate.executable.clone(),
                distribution: distribution.clone(),
            },
        }
    }

    pub fn working_directory(&self) -> Option<&Path> {
        match self {
            Self::Native(path) => Some(path),
            Self::Wsl { .. } => None,
        }
    }

    pub fn prepare_command(&self) -> Option<CommandInvocation> {
        let Self::Wsl {
            executable,
            distribution,
        } = self
        else {
            return None;
        };
        Some(CommandInvocation {
            executable: executable.clone(),
            arguments: [
                "--distribution",
                distribution,
                "--exec",
                "/bin/sh",
                "-c",
                PREPARE_SCRIPT,
            ]
            .into_iter()
            .map(OsString::from)
            .collect(),
        })
    }

    pub fn command(
        &self,
        candidate: &ExecutableCandidate,
        provider_arguments: &[&str],
    ) -> Result<CommandInvocation, CommandBuildError> {
        match self {
            Self::Native(_) => {
                command_for_candidate(candidate, CliProvider::Claude, provider_arguments)
            }
            Self::Wsl {
                executable,
                distribution,
            } => {
                let mut arguments = [
                    "--distribution",
                    distribution,
                    "--exec",
                    "/bin/sh",
                    "-c",
                    SETUP_SCRIPT,
                    "ai-token-meter",
                    "claude",
                ]
                .into_iter()
                .map(OsString::from)
                .collect::<Vec<_>>();
                arguments.extend(provider_arguments.iter().map(OsString::from));
                Ok(CommandInvocation {
                    executable: executable.clone(),
                    arguments,
                })
            }
        }
    }

    pub fn setup_command(
        &self,
        candidate: &ExecutableCandidate,
    ) -> Result<CommandInvocation, CommandBuildError> {
        self.command(candidate, &["--ax-screen-reader", "--safe-mode"])
    }
}
