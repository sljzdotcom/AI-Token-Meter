pub mod claude_workspace;
pub mod conpty;
#[cfg(windows)]
pub mod credential_manager;
#[cfg(windows)]
pub mod credential_prompt;
pub mod deepseek_history_window;
pub mod deepseek_webview;
pub mod desktop_visibility;
pub mod display_coordinator;
pub mod display_topology;
pub mod environment;
pub mod executable_locator;
pub mod meter_drag;
pub mod monitor;
mod npm_runtime;
pub mod process;
pub mod strip_preferences;
pub mod strip_runtime;
pub mod tray;
pub mod window_controller;
pub mod wsl;
