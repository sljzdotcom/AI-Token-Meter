use std::path::Path;
use std::time::Duration;

use crate::accounts::cli_account::CliProvider;
use crate::domain::UsageSnapshot;
use crate::platform::windows::executable_locator::ExecutableCandidate;
use crate::platform::windows::process::command_for_candidate;

use super::CollectionError;
use super::codex_app_server::collect_rate_limits_from_invocation;

pub fn collect_usage_from_candidate(
    candidate: &ExecutableCandidate,
    working_directory: Option<&Path>,
    fetched_at: &str,
) -> Result<UsageSnapshot, CollectionError> {
    let invocation =
        command_for_candidate(candidate, CliProvider::Codex, &["app-server", "--stdio"])
            .map_err(|_| CollectionError::Transport)?;
    collect_rate_limits_from_invocation(
        &invocation,
        working_directory,
        fetched_at,
        Duration::from_secs(10),
    )
}
