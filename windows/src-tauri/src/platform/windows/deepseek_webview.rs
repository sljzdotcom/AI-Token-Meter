use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use reqwest::Url;
use tauri::webview::WebviewWindowBuilder;
use tauri::{Emitter, Manager, WebviewUrl};
use time::OffsetDateTime;

use crate::RuntimeState;
use crate::collectors::deepseek_history::{
    AcceptOutcome, DeepSeekHistoryAssembler, DeepSeekHistoryChunk, DeepSeekHistoryError,
    apply_history,
};
use crate::domain::ProviderId;

const OFFICIAL_CONSOLE_HOST: &str = "platform.deepseek.com";
const CALLBACK_SCHEME: &str = "aimeter-deepseek";
const CALLBACK_HOST: &str = "history";
const HISTORY_WINDOW_LABEL: &str = "deepseek-history";
const HISTORY_URL: &str = "https://platform.deepseek.com/usage";
const BRIDGE_SOURCE: &str = include_str!("../../../../src/deepseek-web/bridge.ts");

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
    session: Arc<Mutex<Option<DeepSeekHistoryAssembler>>>,
) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window(HISTORY_WINDOW_LABEL) {
        existing
            .destroy()
            .map_err(|_| "The previous DeepSeek history window could not be closed".to_owned())?;
    }

    let nonce = secure_nonce()?;
    *session
        .lock()
        .map_err(|_| "DeepSeek history state is temporarily unavailable".to_owned())? = Some(
        DeepSeekHistoryAssembler::new(&nonce, OffsetDateTime::now_utc()),
    );
    let local_app_data = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| "Windows LocalAppData is unavailable".to_owned())?;
    let profile = isolated_profile_directory(&local_app_data);
    std::fs::create_dir_all(&profile)
        .map_err(|_| "The isolated DeepSeek sign-in profile could not be created".to_owned())?;
    let history_url = Url::parse(HISTORY_URL)
        .map_err(|_| "The fixed DeepSeek history URL is invalid".to_owned())?;
    let script = initialization_script(&nonce);
    let app_for_navigation = app.clone();
    let session_for_navigation = Arc::clone(&session);

    WebviewWindowBuilder::new(app, HISTORY_WINDOW_LABEL, WebviewUrl::External(history_url))
        .title("DeepSeek Usage · AI Token Meter")
        .inner_size(1040.0, 760.0)
        .min_inner_size(760.0, 560.0)
        .center()
        .data_directory(profile)
        .initialization_script(script)
        .on_navigation(move |url| {
            if url.scheme() == CALLBACK_SCHEME {
                if let Ok(chunk) = parse_bridge_callback(url) {
                    let outcome = session_for_navigation.lock().ok().and_then(|mut session| {
                        session
                            .as_mut()?
                            .accept(chunk, OffsetDateTime::now_utc())
                            .ok()
                    });
                    if let Some(AcceptOutcome::Complete(history)) = outcome {
                        apply_completed_history(&app_for_navigation, &history);
                        if let Some(window) =
                            app_for_navigation.get_webview_window(HISTORY_WINDOW_LABEL)
                        {
                            let _ = window.hide();
                        }
                    }
                }
                return false;
            }
            is_allowed_deepseek_navigation(url)
        })
        .build()
        .map_err(|_| "The isolated DeepSeek sign-in window could not be opened".to_owned())?;
    Ok(())
}

fn apply_completed_history(
    app: &tauri::AppHandle,
    history: &crate::collectors::deepseek_history::DeepSeekHistory,
) {
    let state = app.state::<RuntimeState>();
    let updated = state.snapshots.lock().ok().and_then(|mut snapshots| {
        let snapshot = snapshots
            .iter_mut()
            .find(|snapshot| snapshot.provider_id == ProviderId::DeepSeek)?;
        let updated = apply_history(snapshot, history).ok()?;
        *snapshot = updated.clone();
        Some(updated)
    });
    if let Some(updated) = updated {
        let _ = app.emit("snapshot-updated", &updated);
        let _ = app.emit("active-detail-changed", &updated);
    }
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
