use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use reqwest::Url;
use tauri::webview::{PageLoadEvent, WebviewWindowBuilder};
use tauri::{Emitter, Manager, WebviewUrl};
use time::OffsetDateTime;

use crate::RuntimeState;
use crate::collectors::deepseek_history::{
    DeepSeekHistoryChunk, DeepSeekHistoryError, apply_history,
};
use crate::domain::ProviderId;
use crate::platform::windows::deepseek_history_window::{
    DeepSeekHistoryChunkOutcome, DeepSeekHistoryGeneration, DeepSeekHistoryReadyResolution,
    DeepSeekHistoryTerminalClaim, DeepSeekHistoryWindowAction, DeepSeekHistoryWindowActionExecutor,
    DeepSeekHistoryWindowCoordinator, DeepSeekHistoryWindowExecution, execute_window_actions,
};
use crate::platform::windows::window_controller::show_detail_window;

const OFFICIAL_CONSOLE_HOST: &str = "platform.deepseek.com";
const OFFICIAL_CONSOLE_ORIGIN: &str = "https://platform.deepseek.com";
const CALLBACK_SCHEME: &str = "aimeter-deepseek";
const CALLBACK_HOST: &str = "history";
const HISTORY_WINDOW_LABEL: &str = "deepseek-history";
const HISTORY_URL: &str = "https://platform.deepseek.com/usage";
const HISTORY_STATUS_EVENT: &str = "deepseek-history-status";
const HISTORY_LOAD_TIMEOUT: Duration = Duration::from_secs(30);
const BRIDGE_SOURCE: &str = include_str!("../../../../src/deepseek-web/bridge.ts");

type HistoryRuntime = Arc<Mutex<DeepSeekHistoryWindowRuntime>>;

#[derive(Default)]
pub(crate) struct DeepSeekHistoryWindowRuntime {
    coordinator: DeepSeekHistoryWindowCoordinator,
    timeout: Option<(
        DeepSeekHistoryGeneration,
        tauri::async_runtime::JoinHandle<()>,
    )>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeepSeekBridgeCallback {
    Ready { nonce: String, origin: String },
    Chunk(DeepSeekHistoryChunk),
}

pub fn isolated_profile_directory(local_app_data: &Path) -> PathBuf {
    local_app_data
        .join("AI Token Meter")
        .join("WebView2")
        .join("DeepSeek")
}

pub fn is_allowed_deepseek_navigation(url: &Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some(OFFICIAL_CONSOLE_HOST)
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
}

pub fn parse_bridge_message(url: &Url) -> Result<DeepSeekBridgeCallback, DeepSeekHistoryError> {
    validate_callback_location(url)?;
    let values = callback_values(url)?;
    if values.contains_key("ready") {
        if values
            .keys()
            .any(|key| !matches!(key.as_str(), "nonce" | "origin" | "ready"))
            || required(&values, "ready")? != "1"
        {
            return Err(DeepSeekHistoryError::InvalidPayload);
        }
        let nonce = required_nonce(&values)?;
        let origin = required(&values, "origin")?.to_owned();
        if origin != OFFICIAL_CONSOLE_ORIGIN {
            return Err(DeepSeekHistoryError::UnsafeOrigin);
        }
        return Ok(DeepSeekBridgeCallback::Ready { nonce, origin });
    }
    parse_chunk_values(values).map(DeepSeekBridgeCallback::Chunk)
}

pub fn parse_bridge_callback(url: &Url) -> Result<DeepSeekHistoryChunk, DeepSeekHistoryError> {
    validate_callback_location(url)?;
    parse_chunk_values(callback_values(url)?)
}

fn validate_callback_location(url: &Url) -> Result<(), DeepSeekHistoryError> {
    if url.scheme() != CALLBACK_SCHEME
        || url.host_str() != Some(CALLBACK_HOST)
        || !(url.path().is_empty() || url.path() == "/")
        || url.fragment().is_some()
    {
        return Err(DeepSeekHistoryError::InvalidPayload);
    }
    Ok(())
}

fn callback_values(url: &Url) -> Result<BTreeMap<String, String>, DeepSeekHistoryError> {
    let mut values = BTreeMap::new();
    for (key, value) in url.query_pairs() {
        if values
            .insert(key.into_owned(), value.into_owned())
            .is_some()
        {
            return Err(DeepSeekHistoryError::InvalidPayload);
        }
    }
    Ok(values)
}

fn parse_chunk_values(
    values: BTreeMap<String, String>,
) -> Result<DeepSeekHistoryChunk, DeepSeekHistoryError> {
    if values.keys().any(|key| {
        !matches!(
            key.as_str(),
            "nonce" | "origin" | "sequence" | "total" | "payload"
        )
    }) {
        return Err(DeepSeekHistoryError::InvalidPayload);
    }
    Ok(DeepSeekHistoryChunk {
        nonce: required_nonce(&values)?,
        origin: required(&values, "origin")?.to_owned(),
        sequence: required(&values, "sequence")?
            .parse()
            .map_err(|_| DeepSeekHistoryError::InvalidPayload)?,
        total: required(&values, "total")?
            .parse()
            .map_err(|_| DeepSeekHistoryError::InvalidPayload)?,
        payload_fragment: required(&values, "payload")?.to_owned(),
    })
}

fn required_nonce(values: &BTreeMap<String, String>) -> Result<String, DeepSeekHistoryError> {
    let nonce = required(values, "nonce")?.to_owned();
    if nonce.len() != 32 || !nonce.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(DeepSeekHistoryError::InvalidPayload);
    }
    Ok(nonce)
}

fn required<'a>(
    values: &'a BTreeMap<String, String>,
    key: &str,
) -> Result<&'a str, DeepSeekHistoryError> {
    values
        .get(key)
        .filter(|value| !value.is_empty())
        .map(String::as_str)
        .ok_or(DeepSeekHistoryError::InvalidPayload)
}

pub(crate) fn open_history_window(
    app: &tauri::AppHandle,
    runtime: HistoryRuntime,
) -> Result<(), String> {
    let nonce = secure_nonce()?;
    let open = runtime
        .lock()
        .map_err(|_| "DeepSeek history state is temporarily unavailable".to_owned())?
        .coordinator
        .open(&nonce, OffsetDateTime::now_utc());
    let should_create = open
        .actions
        .contains(&DeepSeekHistoryWindowAction::CreateHidden);
    let execution = execute_actions(app, &open.actions);
    if !execution.succeeded() {
        recover_failure(app, &runtime, open.generation);
        return Err("The DeepSeek history window could not be prepared".to_owned());
    }
    if !should_create {
        return Ok(());
    }

    let local_app_data = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| "Windows LocalAppData is unavailable".to_owned());
    let local_app_data = match local_app_data {
        Ok(local_app_data) => local_app_data,
        Err(error) => {
            recover_failure(app, &runtime, open.generation);
            return Err(error);
        }
    };
    let profile = isolated_profile_directory(&local_app_data);
    if std::fs::create_dir_all(&profile).is_err() {
        recover_failure(app, &runtime, open.generation);
        return Err("The isolated DeepSeek sign-in profile could not be created".to_owned());
    }
    let history_url = match Url::parse(HISTORY_URL) {
        Ok(url) => url,
        Err(_) => {
            recover_failure(app, &runtime, open.generation);
            return Err("The fixed DeepSeek history URL is invalid".to_owned());
        }
    };
    let script = initialization_script(&nonce);
    let app_for_navigation = app.clone();
    let runtime_for_navigation = Arc::clone(&runtime);
    let runtime_for_page_load = Arc::clone(&runtime);
    let generation = open.generation;

    let build_result =
        WebviewWindowBuilder::new(app, HISTORY_WINDOW_LABEL, WebviewUrl::External(history_url))
            .title("DeepSeek Usage · AI Token Meter")
            .inner_size(1040.0, 760.0)
            .min_inner_size(760.0, 560.0)
            .center()
            .visible(false)
            .data_directory(profile)
            .initialization_script(script)
            .on_navigation(move |url| {
                if url.scheme() == CALLBACK_SCHEME {
                    handle_bridge_callback(
                        &app_for_navigation,
                        &runtime_for_navigation,
                        generation,
                        url,
                    );
                    return false;
                }
                let allowed = is_allowed_deepseek_navigation(url);
                if !allowed {
                    recover_failure(&app_for_navigation, &runtime_for_navigation, generation);
                }
                allowed
            })
            .on_page_load(move |_, payload| {
                if payload.event() == PageLoadEvent::Finished
                    && let Ok(mut runtime) = runtime_for_page_load.lock()
                {
                    let _ = runtime.coordinator.navigation_finished(generation);
                }
            })
            .build();
    let window = match build_result {
        Ok(window) => window,
        Err(_) => {
            recover_failure(app, &runtime, generation);
            return Err("The isolated DeepSeek sign-in window could not be opened".to_owned());
        }
    };

    let app_for_close = app.clone();
    let runtime_for_close = Arc::clone(&runtime);
    window.on_window_event(move |event| {
        if matches!(
            event,
            tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed
        ) {
            let terminal = runtime_for_close
                .lock()
                .ok()
                .and_then(|mut runtime| runtime.coordinator.claim_closed(generation));
            if let Some(terminal) = terminal {
                finish_terminal(&app_for_close, &runtime_for_close, terminal, true);
            }
        }
    });

    install_timeout(app, &runtime, generation);
    Ok(())
}

fn handle_bridge_callback(
    app: &tauri::AppHandle,
    runtime: &HistoryRuntime,
    generation: DeepSeekHistoryGeneration,
    url: &Url,
) {
    match parse_bridge_message(url) {
        Ok(DeepSeekBridgeCallback::Ready { nonce, .. }) => {
            handle_ready(app, runtime, generation, &nonce);
        }
        Ok(DeepSeekBridgeCallback::Chunk(chunk)) => {
            let outcome = runtime.lock().ok().and_then(|mut runtime| {
                runtime
                    .coordinator
                    .accept_chunk(generation, chunk, OffsetDateTime::now_utc())
                    .ok()
            });
            match outcome {
                Some(
                    DeepSeekHistoryChunkOutcome::Ignored | DeepSeekHistoryChunkOutcome::Waiting,
                ) => {}
                Some(DeepSeekHistoryChunkOutcome::Complete { history, terminal }) => {
                    cancel_timeout(runtime, generation);
                    let applied = apply_completed_history(app, &history).is_ok();
                    finish_terminal(app, runtime, terminal, applied);
                }
                None => recover_failure(app, runtime, generation),
            }
        }
        Err(_) => recover_failure(app, runtime, generation),
    }
}

fn handle_ready(
    app: &tauri::AppHandle,
    runtime: &HistoryRuntime,
    generation: DeepSeekHistoryGeneration,
    nonce: &str,
) {
    let ready = runtime
        .lock()
        .ok()
        .and_then(|mut runtime| runtime.coordinator.claim_ready(generation, nonce));
    let Some(ready) = ready else {
        return;
    };
    cancel_timeout(runtime, generation);
    let execution = execute_actions(app, ready.actions());
    let resolution = runtime
        .lock()
        .map(|mut runtime| runtime.coordinator.finish_ready(ready, &execution))
        .unwrap_or(DeepSeekHistoryReadyResolution::Ignored);
    match resolution {
        DeepSeekHistoryReadyResolution::Activated(actions) => {
            let _ = execute_actions(app, &actions);
        }
        DeepSeekHistoryReadyResolution::Recover(terminal) => {
            finish_terminal(app, runtime, terminal, true);
        }
        DeepSeekHistoryReadyResolution::Ignored => {}
    }
}

fn install_timeout(
    app: &tauri::AppHandle,
    runtime: &HistoryRuntime,
    generation: DeepSeekHistoryGeneration,
) {
    let app_for_timeout = app.clone();
    let runtime_for_timeout = Arc::clone(runtime);
    let handle = tauri::async_runtime::spawn(async move {
        tokio::time::sleep(HISTORY_LOAD_TIMEOUT).await;
        let terminal = runtime_for_timeout.lock().ok().and_then(|mut runtime| {
            if runtime
                .timeout
                .as_ref()
                .is_some_and(|(scheduled, _)| *scheduled == generation)
            {
                runtime.timeout = None;
            }
            runtime.coordinator.claim_timeout(generation)
        });
        if let Some(terminal) = terminal {
            finish_terminal(&app_for_timeout, &runtime_for_timeout, terminal, true);
        }
    });
    let mut runtime_guard = match runtime.lock() {
        Ok(runtime) => runtime,
        Err(_) => {
            handle.abort();
            return;
        }
    };
    if runtime_guard.coordinator.is_opening(generation) {
        runtime_guard.timeout = Some((generation, handle));
    } else {
        handle.abort();
    }
}

fn cancel_timeout(runtime: &HistoryRuntime, generation: DeepSeekHistoryGeneration) {
    let handle = runtime.lock().ok().and_then(|mut runtime| {
        if runtime
            .timeout
            .as_ref()
            .is_some_and(|(scheduled, _)| *scheduled == generation)
        {
            runtime.timeout.take().map(|(_, handle)| handle)
        } else {
            None
        }
    });
    if let Some(handle) = handle {
        handle.abort();
    }
}

fn recover_failure(
    app: &tauri::AppHandle,
    runtime: &HistoryRuntime,
    generation: DeepSeekHistoryGeneration,
) {
    let terminal = runtime
        .lock()
        .ok()
        .and_then(|mut runtime| runtime.coordinator.claim_failed(generation));
    if let Some(terminal) = terminal {
        finish_terminal(app, runtime, terminal, true);
    }
}

fn finish_terminal(
    app: &tauri::AppHandle,
    runtime: &HistoryRuntime,
    terminal: DeepSeekHistoryTerminalClaim,
    operation_succeeded: bool,
) {
    cancel_timeout(runtime, terminal.generation());
    let mut execution = execute_actions(app, terminal.actions());
    if !operation_succeeded {
        execution.record_operation_failure();
    }
    let status_actions = runtime
        .lock()
        .map(|mut runtime| runtime.coordinator.finish_terminal(terminal, &execution))
        .unwrap_or_default();
    let _ = execute_actions(app, &status_actions);
}

fn apply_completed_history(
    app: &tauri::AppHandle,
    history: &crate::collectors::deepseek_history::DeepSeekHistory,
) -> Result<(), String> {
    let state = app.state::<RuntimeState>();
    let updated = apply_history(&state.usage.snapshot(ProviderId::DeepSeek), history)
        .map_err(|_| "DeepSeek history could not be applied".to_owned())?;
    state.usage.replace_external(updated.clone());
    app.emit("snapshot-updated", &updated)
        .map_err(|_| "DeepSeek history could not be published".to_owned())?;
    app.emit("active-detail-changed", &updated)
        .map_err(|_| "DeepSeek history could not be published".to_owned())?;
    Ok(())
}

struct TauriHistoryActionExecutor<'a> {
    app: &'a tauri::AppHandle,
}

impl DeepSeekHistoryWindowActionExecutor for TauriHistoryActionExecutor<'_> {
    type Error = String;

    fn execute(&mut self, action: DeepSeekHistoryWindowAction) -> Result<(), Self::Error> {
        match action {
            DeepSeekHistoryWindowAction::CreateHidden => Ok(()),
            DeepSeekHistoryWindowAction::ShowFocused
            | DeepSeekHistoryWindowAction::FocusExisting => focus_history_window(self.app),
            DeepSeekHistoryWindowAction::RestoreDetail => restore_detail_window(self.app),
            DeepSeekHistoryWindowAction::DestroyHistory => {
                if let Some(window) = self.app.get_webview_window(HISTORY_WINDOW_LABEL) {
                    window
                        .destroy()
                        .map_err(|_| "The DeepSeek history window could not be closed".to_owned())
                } else {
                    Ok(())
                }
            }
            DeepSeekHistoryWindowAction::EmitStatus(status) => self
                .app
                .emit(HISTORY_STATUS_EVENT, status)
                .map_err(|_| "DeepSeek history status could not be published".to_owned()),
        }
    }
}

fn execute_actions(
    app: &tauri::AppHandle,
    actions: &[DeepSeekHistoryWindowAction],
) -> DeepSeekHistoryWindowExecution {
    execute_window_actions(actions, &mut TauriHistoryActionExecutor { app })
}

fn focus_history_window(app: &tauri::AppHandle) -> Result<(), String> {
    let detail = app
        .get_webview_window("detail")
        .ok_or_else(|| "The detail window is unavailable".to_owned())?;
    detail
        .set_always_on_top(false)
        .and_then(|_| detail.hide())
        .map_err(|_| "The detail window could not be suspended".to_owned())?;
    let history = app
        .get_webview_window(HISTORY_WINDOW_LABEL)
        .ok_or_else(|| "The DeepSeek history window is unavailable".to_owned())?;
    history
        .show()
        .and_then(|_| history.set_focus())
        .map_err(|_| "The DeepSeek history window could not be focused".to_owned())
}

fn restore_detail_window(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<RuntimeState>();
    let snapshot = state.usage.snapshot(ProviderId::DeepSeek);
    let (edge, _, _) = state.meter_position();
    show_detail_window(app, edge)
        .map_err(|_| "The detail window could not be restored".to_owned())?;
    app.emit("active-detail-changed", &snapshot)
        .map_err(|_| "The detail window could not be updated".to_owned())
}

fn secure_nonce() -> Result<String, String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|_| "A secure DeepSeek history session could not be created".to_owned())?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn initialization_script(nonce: &str) -> String {
    format!(
        r#"
const __aiMeterBridgeFactory = {BRIDGE_SOURCE};
__aiMeterBridgeFactory("{nonce}", (payload) => {{
  const fragments = [];
  for (let offset = 0; offset < payload.length; offset += 12000) {{
    fragments.push(payload.slice(offset, offset + 12000));
  }}
  const send = (sequence) => {{
    if (sequence >= fragments.length) return;
    const query = new URLSearchParams({{
      nonce: "{nonce}",
      origin: window.location.origin,
      sequence: String(sequence),
      total: String(fragments.length),
      payload: fragments[sequence],
    }});
    window.location.assign(`{CALLBACK_SCHEME}://{CALLBACK_HOST}?${{query}}`);
    window.setTimeout(() => send(sequence + 1), 30);
  }};
  send(0);
}});
const __aiMeterSignalReady = () => {{
  if (window.location.origin !== "{OFFICIAL_CONSOLE_ORIGIN}") return;
  const query = new URLSearchParams({{
    nonce: "{nonce}",
    origin: window.location.origin,
    ready: "1",
  }});
  window.location.assign(`{CALLBACK_SCHEME}://{CALLBACK_HOST}?${{query}}`);
}};
if (document.readyState === "loading") {{
  document.addEventListener("DOMContentLoaded", __aiMeterSignalReady, {{ once: true }});
}} else {{
  window.setTimeout(__aiMeterSignalReady, 0);
}}
"#
    )
}
