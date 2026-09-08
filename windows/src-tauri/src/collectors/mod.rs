pub mod activity_timeout;
#[cfg(windows)]
pub mod application;
pub mod backoff;
pub mod claude;
pub mod claude_activity;
pub mod codex;
pub mod codex_activity;
pub mod codex_app_server;
pub mod deepseek;
pub mod deepseek_history;
pub mod refresh;
pub mod refresh_schedule;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionError {
    RateLimited(u64),
    AuthenticationRequired,
    SetupRequired,
    InvalidResponse,
    UnrecognizedOutput,
    TimedOut,
    Transport,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityError {
    Unavailable,
    InvalidData,
    ReadFailure,
    Cancelled,
}

pub fn candidate_for_collection(
    discovery: crate::accounts::cli_discovery::CliDiscovery,
) -> Result<
    Option<crate::platform::windows::executable_locator::ExecutableCandidate>,
    CollectionError,
> {
    use crate::accounts::cli_discovery::CliDiscovery;
    match discovery {
        CliDiscovery::Found(candidate) => Ok(Some(candidate)),
        CliDiscovery::Missing => Ok(None),
        CliDiscovery::Unavailable => Err(CollectionError::Transport),
        CliDiscovery::Cancelled => Err(CollectionError::Cancelled),
    }
}
