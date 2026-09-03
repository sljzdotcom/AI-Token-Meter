use crate::platform::windows::executable_locator::ExecutableCandidate;
use crate::platform::windows::process::{
    CommandBuildError, CommandInvocation, command_for_candidate,
};

use super::cli_account::CliProvider;

pub fn codex_login_command(
    candidate: &ExecutableCandidate,
) -> Result<CommandInvocation, CommandBuildError> {
    command_for_candidate(candidate, CliProvider::Codex, &["login"])
}
