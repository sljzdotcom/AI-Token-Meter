pub mod claude;
pub mod codex;
pub mod codex_app_server;
pub mod deepseek;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionError {
    AuthenticationRequired,
    SetupRequired,
    InvalidResponse,
    UnrecognizedOutput,
    TimedOut,
    Transport,
}
