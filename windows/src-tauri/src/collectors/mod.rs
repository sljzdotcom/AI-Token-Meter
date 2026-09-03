pub mod claude;
pub mod claude_activity;
pub mod codex;
pub mod codex_activity;
pub mod codex_app_server;
pub mod deepseek;
pub mod deepseek_history;
pub mod refresh;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionError {
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
}
