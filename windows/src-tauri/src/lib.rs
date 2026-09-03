use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use serde::Deserialize;
use tauri::{Emitter, Manager, State};

use crate::domain::{ProviderId, UsageSnapshot};
use crate::persistence::{
    AppSettings, AtomicJsonStore, MeterEdge, ProviderCliSettings, UsageRuntime,
};
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
pub mod updater;

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
    update_state: Arc<Mutex<crate::updater::UpdateState>>,
    #[cfg_attr(not(windows), allow(dead_code))]
    notification_levels: Mutex<HashMap<String, u8>>,
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
            update_state: Arc::new(Mutex::new(crate::updater::UpdateState::new(
                SHARED_VERSION.trim(),
            ))),
            notification_levels: Mutex::new(HashMap::new()),
        }
    }
}

impl RuntimeState {
    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) fn app_settings_snapshot(&self) -> AppSettings {
        self.settings
            .lock()
            .map(|settings| settings.clone())
            .unwrap_or_default()
    }

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
    pub(crate) fn threshold_notice(&self, snapshot: &UsageSnapshot) -> Option<(u8, String)> {
        let enabled = self
            .settings
            .lock()
            .is_ok_and(|settings| settings.notifications_enabled);
        let mut levels = self
            .notification_levels
            .lock()
            .unwrap_or_else(|lock| lock.into_inner());
        evaluate_threshold(enabled, &mut levels, snapshot)
    }

    #[cfg(windows)]
    pub(crate) fn refresh_interval_seconds(&self) -> u64 {
        self.settings
            .lock()
            .map(|settings| settings.refresh_interval_seconds)
            .unwrap_or(300)
    }
}

#[cfg_attr(not(any(windows, test)), allow(dead_code))]
fn evaluate_threshold(
    enabled: bool,
    levels: &mut HashMap<String, u8>,
    snapshot: &UsageSnapshot,
) -> Option<(u8, String)> {
    if !enabled || snapshot.status != crate::domain::UsageStatus::Fresh {
        return None;
    }
    for metric in [
        snapshot.primary_metric.as_ref(),
        snapshot.secondary_metric.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        let Some(limit) = metric
            .limit
            .filter(|value| value.is_finite() && *value > 0.0)
        else {
            continue;
        };
        let ratio = (metric.current / limit).clamp(0.0, 1.0);
        let key = format!(
            "{:?}|{}|{}",
            snapshot.provider_id,
            metric.label,
            metric.reset_at.as_deref().unwrap_or("no-reset")
        );
        if ratio < 0.10 {
            levels.remove(&key);
            continue;
        }
        let reached = if ratio >= 0.90 {
            90
        } else if ratio >= 0.70 {
            70
        } else {
            continue;
        };
        let previous = levels.get(&key).copied().unwrap_or(0);
        if reached > previous {
            levels.insert(key, reached);
            return Some((reached, metric.label.clone()));
        }
    }
    None
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
fn set_refresh_interval_seconds(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
    seconds: u64,
) -> Result<(), String> {
    let updated = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Settings are temporarily unavailable".to_owned())?;
        settings
            .set_refresh_interval_seconds(seconds)
            .map_err(str::to_owned)?;
        persist_settings(state.settings_path.as_deref(), &settings)?;
        settings.clone()
    };
    app.emit("app-settings-changed", updated)
        .map_err(|_| "The refresh interval could not be updated".to_owned())
}

#[tauri::command]
fn set_deepseek_balance_baseline_cents(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
    cents: u64,
) -> Result<(), String> {
    let updated = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Settings are temporarily unavailable".to_owned())?;
        settings
            .set_deepseek_balance_baseline_cents(cents)
            .map_err(str::to_owned)?;
        persist_settings(state.settings_path.as_deref(), &settings)?;
        settings.clone()
    };
    app.emit("app-settings-changed", updated)
        .map_err(|_| "The DeepSeek balance baseline could not be updated".to_owned())?;
    #[cfg(windows)]
    crate::collectors::application::trigger(
        &app,
        crate::collectors::refresh::RefreshPriority::Manual,
    );
    Ok(())
}

#[tauri::command]
fn set_notifications_enabled(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
    enabled: bool,
) -> Result<(), String> {
    let updated = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Settings are temporarily unavailable".to_owned())?;
        settings.notifications_enabled = enabled;
        persist_settings(state.settings_path.as_deref(), &settings)?;
        settings.clone()
    };
    app.emit("app-settings-changed", updated)
        .map_err(|_| "Usage alerts could not be updated".to_owned())
}

#[tauri::command]
fn set_launch_at_login(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
    enabled: bool,
) -> Result<(), String> {
    #[cfg(windows)]
    configure_launch_at_login(enabled)?;
    let updated = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Settings are temporarily unavailable".to_owned())?;
        settings.launch_at_login = enabled;
        persist_settings(state.settings_path.as_deref(), &settings)?;
        settings.clone()
    };
    app.emit("app-settings-changed", updated)
        .map_err(|_| "Launch at login could not be updated".to_owned())
}

#[cfg(windows)]
fn configure_launch_at_login(enabled: bool) -> Result<(), String> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let current_executable = std::env::current_exe()
        .map_err(|_| "The application executable could not be located".to_owned())?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (run, _) = hkcu
        .create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run")
        .map_err(|_| "Windows startup settings are unavailable".to_owned())?;
    if enabled {
        run.set_value(
            PRODUCT_NAME,
            &format!("\"{}\"", current_executable.display()),
        )
        .map_err(|_| "Windows startup settings could not be updated".to_owned())
    } else {
        match run.delete_value(PRODUCT_NAME) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err("Windows startup settings could not be updated".to_owned()),
        }
    }
}

#[tauri::command]
fn set_provider_cli_settings(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
    provider_id: ProviderId,
    value: ProviderCliSettings,
) -> Result<AppSettings, String> {
    let updated = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Settings are temporarily unavailable".to_owned())?;
        settings
            .set_cli_settings(provider_id, value)
            .map_err(str::to_owned)?;
        persist_settings(state.settings_path.as_deref(), &settings)?;
        settings.clone()
    };
    app.emit("app-settings-changed", &updated)
        .map_err(|_| "The CLI runtime setting could not be updated".to_owned())?;
    #[cfg(windows)]
    crate::collectors::application::trigger(
        &app,
        crate::collectors::refresh::RefreshPriority::Manual,
    );
    Ok(updated)
}

#[tauri::command]
fn available_wsl_distributions() -> Vec<String> {
    #[cfg(windows)]
    {
        use std::time::Duration;

        let inputs = crate::platform::windows::environment::DiscoveryInputs::capture(None);
        let Some(invocation) = inputs
            .system_root
            .as_deref()
            .and_then(crate::platform::windows::wsl::build_wsl_list_invocation)
        else {
            return Vec::new();
        };
        let mut request = crate::platform::windows::process::ProcessRequest::new(
            invocation.executable,
            invocation.arguments.into_iter().map(Into::into).collect(),
        );
        request.timeout = Duration::from_secs(4);
        request.max_output_bytes = 64 * 1024;
        return crate::platform::windows::process::BoundedProcessRunner
            .run(request)
            .map(|output| {
                crate::platform::windows::wsl::decode_distribution_list(output.stdout.as_bytes())
            })
            .unwrap_or_default();
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
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

#[tauri::command]
fn update_state(state: State<'_, RuntimeState>) -> crate::updater::UpdateState {
    crate::updater::runtime::snapshot(&state.update_state)
}

#[tauri::command]
async fn check_for_updates(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
) -> Result<crate::updater::UpdateState, String> {
    crate::updater::runtime::check(app, Arc::clone(&state.update_state)).await
}

#[tauri::command]
async fn install_update(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
) -> Result<(), String> {
    crate::updater::runtime::install(
        app,
        Arc::clone(&state.update_state),
        Arc::clone(&state.refresh_coordinator),
    )
    .await
}

#[tauri::command]
async fn service_account_statuses(
    _state: State<'_, RuntimeState>,
) -> Result<Vec<crate::accounts::service_status::ServiceAccountStatus>, String> {
    let checked_at = current_timestamp();
    #[cfg(windows)]
    {
        return Ok(crate::accounts::windows_service::read_all(
            &checked_at,
            _state.app_settings_snapshot(),
        )
        .await);
    }
    #[cfg(not(windows))]
    {
        Ok(
            [ProviderId::Claude, ProviderId::Codex, ProviderId::DeepSeek]
                .into_iter()
                .map(|provider| {
                    crate::accounts::service_status::ServiceAccountStatus::unavailable(
                        provider,
                        &checked_at,
                    )
                })
                .collect(),
        )
    }
}

#[tauri::command]
async fn service_account_status(
    provider_id: ProviderId,
    _state: State<'_, RuntimeState>,
) -> Result<crate::accounts::service_status::ServiceAccountStatus, String> {
    let checked_at = current_timestamp();
    #[cfg(windows)]
    {
        return Ok(crate::accounts::windows_service::read_one(
            provider_id,
            &checked_at,
            _state.app_settings_snapshot(),
        )
        .await);
    }
    #[cfg(not(windows))]
    {
        Ok(
            crate::accounts::service_status::ServiceAccountStatus::unavailable(
                provider_id,
                &checked_at,
            ),
        )
    }
}

#[tauri::command]
fn begin_service_sign_in(
    provider_id: ProviderId,
    _state: State<'_, RuntimeState>,
) -> Result<(), String> {
    let provider = match provider_id {
        ProviderId::Claude => crate::accounts::cli_account::CliProvider::Claude,
        ProviderId::Codex => crate::accounts::cli_account::CliProvider::Codex,
        ProviderId::DeepSeek => return Err("DeepSeek uses an API Key".to_owned()),
    };
    #[cfg(windows)]
    {
        let settings = _state.app_settings_snapshot();
        let configuration = settings
            .cli_settings(provider_id)
            .ok_or_else(|| "This service does not use a CLI".to_owned())?;
        crate::accounts::windows_service::launch_login(provider, configuration)
            .map_err(str::to_owned)
    }
    #[cfg(not(windows))]
    {
        let _ = provider;
        Err("Service sign-in is available in the Windows app".to_owned())
    }
}

#[tauri::command]
async fn replace_deepseek_api_key(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
) -> Result<crate::accounts::service_status::ServiceAccountStatus, String> {
    #[cfg(windows)]
    {
        let parent = app
            .get_webview_window("settings")
            .and_then(|window| window.hwnd().ok())
            .map_or(std::ptr::null_mut(), |handle| handle.0 as _);
        let candidate = crate::platform::windows::credential_prompt::prompt_deepseek_api_key(
            parent,
        )
        .map_err(|error| match error {
            crate::platform::windows::credential_prompt::CredentialPromptError::Cancelled => {
                "Credential replacement was cancelled".to_owned()
            }
            crate::platform::windows::credential_prompt::CredentialPromptError::Empty => {
                "API Key is required".to_owned()
            }
            _ => "The protected credential prompt is unavailable".to_owned(),
        })?;
        let checked_at = current_timestamp();
        let credentials =
            Arc::new(crate::platform::windows::credential_manager::WindowsCredentialManager::new());
        let client = crate::collectors::deepseek::DeepSeekBalanceClient::new()
            .map_err(|_| "DeepSeek verification is currently unavailable".to_owned())?;
        let service = crate::accounts::deepseek::DeepSeekAccountService::new(
            credentials,
            client,
            state
                .app_settings_snapshot()
                .deepseek_balance_baseline_cents as f64
                / 100.0,
        );
        let snapshot = service
            .replace_key(candidate, &checked_at)
            .await
            .map_err(|error| error.to_string())?;
        let generation = state.usage.begin_refresh(ProviderId::DeepSeek);
        state
            .usage
            .complete_success(ProviderId::DeepSeek, generation, snapshot);
        let _ = app.emit(
            "snapshot-updated",
            state.usage.snapshot(ProviderId::DeepSeek),
        );
        Ok(crate::accounts::windows_service::read_one(
            ProviderId::DeepSeek,
            &checked_at,
            state.app_settings_snapshot(),
        )
        .await)
    }
    #[cfg(not(windows))]
    {
        let _ = (app, state);
        Err("DeepSeek credential management is available in the Windows app".to_owned())
    }
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
            set_refresh_interval_seconds,
            set_deepseek_balance_baseline_cents,
            set_notifications_enabled,
            set_launch_at_login,
            set_provider_cli_settings,
            available_wsl_distributions,
            open_deepseek_history,
            update_state,
            check_for_updates,
            install_update,
            service_account_statuses,
            service_account_status,
            begin_service_sign_in,
            replace_deepseek_api_key
        ])
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
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

#[cfg(test)]
mod threshold_tests {
    use super::*;

    #[test]
    fn alerts_fire_once_per_level_and_rearm_after_usage_resets() {
        let value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../contracts/fixtures/claude-fresh.json"
        ))
        .expect("fixture JSON");
        let mut snapshot = UsageSnapshot::decode_compatible(&value).expect("fixture snapshot");
        let metric = snapshot.primary_metric.as_mut().expect("primary metric");
        metric.current = 70.0;
        metric.limit = Some(100.0);
        let mut levels = HashMap::new();

        assert_eq!(
            evaluate_threshold(true, &mut levels, &snapshot).map(|notice| notice.0),
            Some(70)
        );
        assert_eq!(evaluate_threshold(true, &mut levels, &snapshot), None);
        snapshot.primary_metric.as_mut().expect("metric").current = 90.0;
        assert_eq!(
            evaluate_threshold(true, &mut levels, &snapshot).map(|notice| notice.0),
            Some(90)
        );
        snapshot.primary_metric.as_mut().expect("metric").current = 5.0;
        assert_eq!(evaluate_threshold(true, &mut levels, &snapshot), None);
        snapshot.primary_metric.as_mut().expect("metric").current = 70.0;
        assert_eq!(
            evaluate_threshold(true, &mut levels, &snapshot).map(|notice| notice.0),
            Some(70)
        );
        assert_eq!(evaluate_threshold(false, &mut levels, &snapshot), None);
    }
}
