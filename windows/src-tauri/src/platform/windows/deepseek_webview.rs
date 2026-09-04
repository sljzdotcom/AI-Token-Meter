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
    AcceptOutcome, DeepSeekHistoryAssembler, DeepSeekHistoryChunk, DeepSeekHistoryError,
    apply_history,
};
use crate::domain::ProviderId;
use crate::platform::windows::deepseek_history_window::{
    DeepSeekHistoryWindowAction, DeepSeekHistoryWindowEvent, DeepSeekHistoryWindowMachine,
};
use crate::platform::windows::window_controller::show_detail_window;

const OFFICIAL_CONSOLE_HOST: &str = "platform.deepseek.com";
const CALLBACK_SCHEME: &str = "aimeter-deepseek";
const CALLBACK_HOST: &str = "history";
const HISTORY_WINDOW_LABEL: &str = "deepseek-history";
const HISTORY_URL: &str = "https://platform.deepseek.com/usage";
const HISTORY_STATUS_EVENT: &str = "deepseek-history-status";
const HISTORY_LOAD_TIMEOUT: Duration = Duration::from_secs(30);
const BRIDGE_SOURCE: &str = include_str!("../../../../src/deepseek-web/bridge.ts");

type HistorySession = Arc<Mutex<Option<DeepSeekHistoryAssembler>>>;
type HistoryWindow = Arc<Mutex<DeepSeekHistoryWindowMachine>>;

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

pub fn parse_bridge_callback(url: &Url) -> Result<DeepSeekHistoryChunk, DeepSeekHistoryError> {
    if url.scheme() != CALLBACK_SCHEME
        || url.host_str() != Some(CALLBACK_HOST)
        || !(url.path().is_empty() || url.path() == "/")
        || url.fragment().is_some()
    {
        return Err(DeepSeekHistoryError::InvalidPayload);
    }

    let mut values = BTreeMap::new();
    for (key, value) in url.query_pairs() {
        if !matches!(
            key.as_ref(),
            "nonce" | "origin" | "sequence" | "total" | "payload"
        ) || values
            .insert(key.into_owned(), value.into_owned())
            .is_some()
        {
            return Err(DeepSeekHistoryError::InvalidPayload);
        }
    }

    let nonce = required(&values, "nonce")?.to_owned();
    if nonce.len() != 32 || !nonce.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(DeepSeekHistoryError::InvalidPayload);
    }
    Ok(DeepSeekHistoryChunk {
        nonce,
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

pub fn open_history_window(
    app: &tauri::AppHandle,
    session: HistorySession,
    lifecycle: HistoryWindow,
) -> Result<(), String> {
    let actions = transition(&lifecycle, DeepSeekHistoryWindowEvent::OpenRequested)?;
    let should_create = actions.contains(&DeepSeekHistoryWindowAction::CreateHidden);
    if let Err(error) = execute_actions(app, &actions) {
        recover_failure(app, &session, &lifecycle);
        return Err(error);
    }
    if !should_create {
        return Ok(());
    }

    let nonce = match secure_nonce() {
        Ok(nonce) => nonce,
        Err(error) => {
            recover_failure(app, &session, &lifecycle);
            return Err(error);
        }
    };
    if let Err(error) = start_session(&session, &nonce) {
        recover_failure(app, &session, &lifecycle);
        return Err(error);
    }
    let local_app_data = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| "Windows LocalAppData is unavailable".to_owned());
    let local_app_data = match local_app_data {
        Ok(local_app_data) => local_app_data,
        Err(error) => {
            recover_failure(app, &session, &lifecycle);
            return Err(error);
        }
    };
    let profile = isolated_profile_directory(&local_app_data);
    if std::fs::create_dir_all(&profile).is_err() {
        recover_failure(app, &session, &lifecycle);
        return Err("The isolated DeepSeek sign-in profile could not be created".to_owned());
    }
    let history_url = match Url::parse(HISTORY_URL) {
        Ok(url) => url,
        Err(_) => {
            recover_failure(app, &session, &lifecycle);
            return Err("The fixed DeepSeek history URL is invalid".to_owned());
        }
    };
    let script = initialization_script(&nonce);
    let app_for_navigation = app.clone();
    let session_for_navigation = Arc::clone(&session);
    let lifecycle_for_navigation = Arc::clone(&lifecycle);
    let app_for_page_load = app.clone();
    let session_for_page_load = Arc::clone(&session);
    let lifecycle_for_page_load = Arc::clone(&lifecycle);

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
                    let outcome = parse_bridge_callback(url).and_then(|chunk| {
                        session_for_navigation
                            .lock()
                            .map_err(|_| DeepSeekHistoryError::InvalidPayload)?
                            .as_mut()
                            .ok_or(DeepSeekHistoryError::InvalidPayload)?
                            .accept(chunk, OffsetDateTime::now_utc())
                    });
                    match outcome {
                        Ok(AcceptOutcome::Waiting) => {}
                        Ok(AcceptOutcome::Complete(history)) => {
                            if apply_completed_history(&app_for_navigation, &history).is_ok() {
                                handle_event(
                                    &app_for_navigation,
                                    &session_for_navigation,
                                    &lifecycle_for_navigation,
                                    DeepSeekHistoryWindowEvent::Completed,
                                );
                            } else {
                                recover_failure(
                                    &app_for_navigation,
                                    &session_for_navigation,
                                    &lifecycle_for_navigation,
                                );
                            }
                        }
                        Err(_) => recover_failure(
                            &app_for_navigation,
                            &session_for_navigation,
                            &lifecycle_for_navigation,
                        ),
                    }
                    return false;
                }
                let allowed = is_allowed_deepseek_navigation(url);
                if !allowed {
                    recover_failure(
                        &app_for_navigation,
                        &session_for_navigation,
                        &lifecycle_for_navigation,
                    );
                }
                allowed
            })
            .on_page_load(move |_, payload| {
                if payload.event() == PageLoadEvent::Finished {
                    let result = transition(
                        &lifecycle_for_page_load,
                        DeepSeekHistoryWindowEvent::PageLoadFinished,
                    )
                    .and_then(|actions| execute_actions(&app_for_page_load, &actions));
                    if result.is_err() {
                        recover_failure(
                            &app_for_page_load,
                            &session_for_page_load,
                            &lifecycle_for_page_load,
                        );
                    }
                }
            })
            .build();
    let window = match build_result {
        Ok(window) => window,
        Err(_) => {
            recover_failure(app, &session, &lifecycle);
            return Err("The isolated DeepSeek sign-in window could not be opened".to_owned());
        }
    };

    let app_for_close = app.clone();
    let session_for_close = Arc::clone(&session);
    let lifecycle_for_close = Arc::clone(&lifecycle);
    window.on_window_event(move |event| {
        if matches!(
            event,
            tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed
        ) {
            handle_event(
                &app_for_close,
                &session_for_close,
                &lifecycle_for_close,
                DeepSeekHistoryWindowEvent::WindowClosed,
            );
        }
    });

    let app_for_timeout = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(HISTORY_LOAD_TIMEOUT).await;
        handle_event(
            &app_for_timeout,
            &session,
            &lifecycle,
            DeepSeekHistoryWindowEvent::LoadTimedOut,
        );
    });
    Ok(())
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

fn start_session(session: &HistorySession, nonce: &str) -> Result<(), String> {
    *session
        .lock()
        .map_err(|_| "DeepSeek history state is temporarily unavailable".to_owned())? = Some(
        DeepSeekHistoryAssembler::new(nonce, OffsetDateTime::now_utc()),
    );
    Ok(())
}

fn transition(
    lifecycle: &HistoryWindow,
    event: DeepSeekHistoryWindowEvent,
) -> Result<Vec<DeepSeekHistoryWindowAction>, String> {
    lifecycle
        .lock()
        .map_err(|_| "DeepSeek history state is temporarily unavailable".to_owned())
        .map(|mut lifecycle| lifecycle.transition(event))
}

fn handle_event(
    app: &tauri::AppHandle,
    session: &HistorySession,
    lifecycle: &HistoryWindow,
    event: DeepSeekHistoryWindowEvent,
) {
    let Ok(actions) = transition(lifecycle, event) else {
        clear_session(session);
        return;
    };
    if actions.iter().any(|action| {
        matches!(
            action,
            DeepSeekHistoryWindowAction::RestoreDetail
                | DeepSeekHistoryWindowAction::DestroyHistory
        )
    }) {
        clear_session(session);
    }
    let _ = execute_actions(app, &actions);
}

fn recover_failure(app: &tauri::AppHandle, session: &HistorySession, lifecycle: &HistoryWindow) {
    handle_event(app, session, lifecycle, DeepSeekHistoryWindowEvent::Failed);
}

fn clear_session(session: &HistorySession) {
    if let Ok(mut session) = session.lock() {
        *session = None;
    }
}

fn execute_actions(
    app: &tauri::AppHandle,
    actions: &[DeepSeekHistoryWindowAction],
) -> Result<(), String> {
    let mut first_error = None;
    for action in actions {
        let result = match action {
            DeepSeekHistoryWindowAction::CreateHidden => Ok(()),
            DeepSeekHistoryWindowAction::ShowFocused
            | DeepSeekHistoryWindowAction::FocusExisting => focus_history_window(app),
            DeepSeekHistoryWindowAction::RestoreDetail => restore_detail_window(app),
            DeepSeekHistoryWindowAction::DestroyHistory => {
                if let Some(window) = app.get_webview_window(HISTORY_WINDOW_LABEL) {
                    window
                        .destroy()
                        .map_err(|_| "The DeepSeek history window could not be closed".to_owned())
                } else {
                    Ok(())
                }
            }
            DeepSeekHistoryWindowAction::EmitStatus(status) => app
                .emit(HISTORY_STATUS_EVENT, status)
                .map_err(|_| "DeepSeek history status could not be published".to_owned()),
        };
        if let Err(error) = result
            && first_error.is_none()
        {
            first_error = Some(error);
        }
    }
    first_error.map_or(Ok(()), Err)
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
"#
    )
}
