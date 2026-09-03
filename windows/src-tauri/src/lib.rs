use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use serde::Deserialize;
use tauri::{Emitter, Manager, State};

use crate::domain::{ProviderId, UsageSnapshot};
use crate::persistence::{AppSettings, AtomicJsonStore, MeterEdge, UsageRuntime};
use crate::platform::windows::window_controller::{
    Edge, METER_WINDOW_LABEL, configure_initial_windows, current_monitor_identifier,
    hide_detail_window, place_meter, show_detail_window, show_settings_window,
    snap_meter_after_drag,
};

pub mod accounts;
pub mod collectors;
pub mod domain;
pub mod persistence;
pub mod platform;
pub mod security;

const PRODUCT_NAME: &str = "AI Token Meter";
const SHARED_VERSION: &str = include_str!("../../../VERSION");
const PROVIDER_CONTRACT: &str = include_str!("../../../contracts/presentation/providers.json");

#[derive(Debug, PartialEq, Eq)]
pub struct AppMetadata {
    pub product_name: &'static str,
    pub version: String,
    pub providers: [String; 3],
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PresentationContract {
    providers: Vec<ProviderContract>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderContract {
    display_name: String,
}

pub fn app_metadata() -> Result<AppMetadata, serde_json::Error> {
    let contract: PresentationContract = serde_json::from_str(PROVIDER_CONTRACT)?;
    let providers = contract
        .providers
        .into_iter()
        .map(|provider| provider.display_name)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| {
            serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "the shared contract must contain exactly three providers",
            ))
        })?;

    Ok(AppMetadata {
        product_name: PRODUCT_NAME,
        version: SHARED_VERSION.trim().to_owned(),
        providers,
    })
}

pub struct RuntimeState {
    pub(crate) usage: Arc<UsageRuntime>,
    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) refresh_coordinator: Arc<crate::collectors::refresh::RefreshCoordinator>,
    settings: Mutex<AppSettings>,
    settings_path: Option<PathBuf>,
    pub(crate) meter_enabled: Arc<AtomicBool>,
    deepseek_history_session:
        Arc<Mutex<Option<crate::collectors::deepseek_history::DeepSeekHistoryAssembler>>>,
}

impl Default for RuntimeState {
    fn default() -> Self {
        let (settings, settings_path) = load_settings();
        Self {
            usage: Arc::new(load_usage_runtime()),
            refresh_coordinator: Arc::new(crate::collectors::refresh::RefreshCoordinator::new()),
            settings: Mutex::new(settings),
            settings_path,
            meter_enabled: Arc::new(AtomicBool::new(true)),
            deepseek_history_session: Arc::new(Mutex::new(None)),
        }
    }
}

impl RuntimeState {
    pub(crate) fn meter_position(&self) -> (Edge, f64, Option<String>) {
        self.settings
            .lock()
            .map(|settings| {
                (
                    edge_from_settings(settings.edge),
                    f64::from(settings.meter_vertical_per_mille.min(1000)) / 1000.0,
                    settings.meter_monitor_id.clone(),
                )
            })
            .unwrap_or((Edge::Right, 0.5, None))
    }

    #[cfg(windows)]
    pub(crate) fn refresh_interval_seconds(&self) -> u64 {
        self.settings
            .lock()
            .map(|settings| settings.refresh_interval_seconds)
            .unwrap_or(300)
    }
}

#[tauri::command]
fn usage_snapshots(state: State<'_, RuntimeState>) -> Vec<UsageSnapshot> {
    state.usage.snapshots()
}

#[tauri::command]
fn show_provider_detail(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
    provider_id: ProviderId,
) -> Result<UsageSnapshot, String> {
    let snapshot = state.usage.snapshot(provider_id);
    let edge = state
        .settings
        .lock()
        .map(|settings| edge_from_settings(settings.edge))
        .unwrap_or(Edge::Right);
    show_detail_window(&app, edge)
        .map_err(|_| "The detail window could not be shown".to_owned())?;
    app.emit("active-detail-changed", &snapshot)
        .map_err(|_| "The detail window could not be updated".to_owned())?;
    if snapshot.provider_id == ProviderId::DeepSeek && snapshot.daily_history.is_empty() {
        let session = Arc::clone(&state.deepseek_history_session);
        let _ = crate::platform::windows::deepseek_webview::open_history_window(&app, session);
    }
    Ok(snapshot)
}

#[tauri::command]
fn meter_drag_ended(app: tauri::AppHandle, state: State<'_, RuntimeState>) -> Result<(), String> {
    let meter = app
        .get_webview_window(METER_WINDOW_LABEL)
        .ok_or_else(|| "The meter window is unavailable".to_owned())?;
    let (edge, normalized_y) = snap_meter_after_drag(&meter)
        .map_err(|_| "The meter could not be snapped to the screen edge".to_owned())?;
    if let Ok(mut settings) = state.settings.lock() {
        settings.edge = edge_to_settings(edge);
        settings.meter_vertical_per_mille =
            (normalized_y * 1000.0).round().clamp(0.0, 1000.0) as u16;
        settings.meter_monitor_id = current_monitor_identifier(&meter).ok().flatten();
        persist_settings(state.settings_path.as_deref(), &settings)?;
    }
    app.emit("meter-edge-changed", edge_to_settings(edge))
        .map_err(|_| "The meter display could not be updated".to_owned())?;
    Ok(())
}

#[tauri::command]
fn set_meter_edge(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
    edge: MeterEdge,
) -> Result<(), String> {
    let meter = app
        .get_webview_window(METER_WINDOW_LABEL)
        .ok_or_else(|| "The meter window is unavailable".to_owned())?;
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "Settings are temporarily unavailable".to_owned())?;
    settings.edge = edge;
    let normalized_y = f64::from(settings.meter_vertical_per_mille.min(1000)) / 1000.0;
    place_meter(&meter, edge_from_settings(edge), normalized_y)
        .map_err(|_| "The meter could not be moved".to_owned())?;
    persist_settings(state.settings_path.as_deref(), &settings)?;
    app.emit("meter-edge-changed", edge)
        .map_err(|_| "The meter display could not be updated".to_owned())
}

#[tauri::command]
fn app_settings(state: State<'_, RuntimeState>) -> AppSettings {
    state
        .settings
        .lock()
        .map_or_else(|_| AppSettings::default(), |settings| settings.clone())
}

#[tauri::command]
fn set_display_font(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
    font: String,
) -> Result<(), String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "Settings are temporarily unavailable".to_owned())?;
    settings.set_display_font(&font).map_err(str::to_owned)?;
    persist_settings(state.settings_path.as_deref(), &settings)?;
    app.emit("display-font-changed", &settings.display_font)
        .map_err(|_| "The display font could not be updated".to_owned())
}

#[tauri::command]
fn set_detail_auto_hide_seconds(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
    seconds: u64,
) -> Result<(), String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "Settings are temporarily unavailable".to_owned())?;
    settings
        .set_detail_auto_hide_seconds(seconds)
        .map_err(str::to_owned)?;
    persist_settings(state.settings_path.as_deref(), &settings)?;
    app.emit("detail-auto-hide-changed", seconds)
        .map_err(|_| "The detail timer could not be updated".to_owned())
}

#[tauri::command]
fn close_provider_detail(app: tauri::AppHandle) -> Result<(), String> {
    hide_detail_window(&app).map_err(|_| "The detail window could not be closed".to_owned())
}

#[tauri::command]
fn open_settings(app: tauri::AppHandle) -> Result<(), String> {
    show_settings_window(&app).map_err(|_| "Settings could not be opened".to_owned())
}

#[tauri::command]
fn open_deepseek_history(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
) -> Result<(), String> {
    crate::platform::windows::deepseek_webview::open_history_window(
        &app,
        Arc::clone(&state.deepseek_history_session),
    )
}

fn edge_from_settings(edge: MeterEdge) -> Edge {
    match edge {
        MeterEdge::Left => Edge::Left,
        MeterEdge::Right => Edge::Right,
    }
}

fn edge_to_settings(edge: Edge) -> MeterEdge {
    match edge {
        Edge::Left => MeterEdge::Left,
        Edge::Right => MeterEdge::Right,
    }
}

fn persist_settings(path: Option<&std::path::Path>, settings: &AppSettings) -> Result<(), String> {
    match path {
        Some(path) => AtomicJsonStore::write(path, settings)
            .map_err(|_| "Settings could not be saved".to_owned()),
        None => Ok(()),
    }
}

fn load_settings() -> (AppSettings, Option<PathBuf>) {
    #[cfg(windows)]
    if let Ok(paths) = crate::persistence::AppStoragePaths::discover() {
        let settings = AtomicJsonStore::read::<AppSettings>(&paths.settings_file)
            .ok()
            .flatten()
            .unwrap_or_default();
        return (settings, Some(paths.settings_file));
    }
    (AppSettings::default(), None)
}

fn load_usage_runtime() -> UsageRuntime {
    #[cfg(windows)]
    if let Ok(paths) = crate::persistence::AppStoragePaths::discover() {
        return UsageRuntime::load(
            crate::persistence::SnapshotCache::new(paths.cache_directory),
            &current_timestamp(),
        );
    }
    UsageRuntime::unavailable(&current_timestamp())
}

fn current_timestamp() -> String {
    use time::format_description::well_known::Rfc3339;
    time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(RuntimeState::default())
        .invoke_handler(tauri::generate_handler![
            usage_snapshots,
            show_provider_detail,
            close_provider_detail,
            open_settings,
            meter_drag_ended,
            set_meter_edge,
            app_settings,
            set_display_font,
            set_detail_auto_hide_seconds,
            open_deepseek_history
        ])
        .setup(|app| {
            let (edge, normalized_y, monitor_id) = app
                .state::<RuntimeState>()
                .settings
                .lock()
                .map(|settings| {
                    (
                        edge_from_settings(settings.edge),
                        f64::from(settings.meter_vertical_per_mille.min(1000)) / 1000.0,
                        settings.meter_monitor_id.clone(),
                    )
                })
                .unwrap_or((Edge::Right, 0.5, None));
            configure_initial_windows(app.handle(), edge, normalized_y, monitor_id.as_deref())?;
            crate::platform::windows::tray::install(app.handle())?;
            #[cfg(windows)]
            {
                crate::collectors::application::start(app.handle());
                let enabled = app.state::<RuntimeState>().meter_enabled.clone();
                crate::platform::windows::desktop_visibility::start_monitoring(
                    app.handle().clone(),
                    enabled,
                );
            }
            if let Some(settings) = app.get_webview_window("settings") {
                let settings_for_close = settings.clone();
                settings.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = settings_for_close.hide();
                    }
                });
            }
            if let Some(detail) = app.get_webview_window("detail") {
                let app_for_focus = app.handle().clone();
                detail.on_window_event(move |event| {
                    if matches!(event, tauri::WindowEvent::Focused(false)) {
                        let _ = hide_detail_window(&app_for_focus);
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run AI Token Meter")
}
