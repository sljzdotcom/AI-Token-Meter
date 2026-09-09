use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Deserialize;
use tauri::{Emitter, Manager, State};

use crate::domain::{ProviderId, UsageSnapshot};
use crate::persistence::{
    AppSettings, AtomicJsonStore, MeterEdge, ProviderCliSettings, UsageRuntime,
};
use crate::platform::windows::window_controller::{
    DetailCommand, DetailState, Edge, METER_WINDOW_LABEL, configure_initial_windows,
    current_monitor_identifier, hide_detail_window, show_detail_window, show_settings_window,
    snap_meter_after_drag,
};

pub mod accounts;
pub mod brand_links;
pub mod collectors;
pub mod domain;
pub mod localization;
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
    pub providers: Vec<String>,
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
        .collect();

    Ok(AppMetadata {
        product_name: PRODUCT_NAME,
        version: SHARED_VERSION.trim().to_owned(),
        providers,
    })
}

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Default)]
struct ExclusiveOperationGate {
    active: AtomicBool,
}

#[cfg_attr(not(windows), allow(dead_code))]
impl ExclusiveOperationGate {
    fn try_enter(&self) -> Result<ExclusiveOperationGuard<'_>, ()> {
        self.active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ExclusiveOperationGuard {
                active: &self.active,
            })
            .map_err(|_| ())
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
struct ExclusiveOperationGuard<'a> {
    active: &'a AtomicBool,
}

impl Drop for ExclusiveOperationGuard<'_> {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Release);
    }
}

pub struct RuntimeState {
    pub(crate) meter_instances: Mutex<std::collections::BTreeMap<String, String>>,
    pub(crate) display_reconcile: crate::platform::windows::display_coordinator::ReconcileQueue,
    pub(crate) pending_meter_drag: Mutex<Option<(String, u64)>>,
    pub(crate) detail_meter: Mutex<String>,
    pub(crate) usage: Arc<UsageRuntime>,
    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) refresh_coordinator: Arc<crate::collectors::refresh::RefreshCoordinator>,
    settings: Mutex<AppSettings>,
    settings_path: Option<PathBuf>,
    pub(crate) meter_enabled: Arc<AtomicBool>,
    meter_drag: Arc<crate::platform::windows::meter_drag::MeterDragGate>,
    pub(crate) strip_folded: AtomicBool,
    pub(crate) strip_pointer: AtomicBool,
    pub(crate) strip_focus: AtomicBool,
    pub(crate) strip_menu: AtomicBool,
    pub(crate) strip_reset: AtomicBool,
    #[cfg_attr(not(windows), allow(dead_code))]
    deepseek_key_replacement: ExclusiveOperationGate,
    deepseek_history:
        Arc<Mutex<crate::platform::windows::deepseek_webview::DeepSeekHistoryWindowRuntime>>,
    pub(crate) detail_state: Mutex<DetailState>,
    update_state: Arc<Mutex<crate::updater::UpdateState>>,
    #[cfg_attr(not(windows), allow(dead_code))]
    notification_levels: Mutex<HashMap<String, u8>>,
}

impl Default for RuntimeState {
    fn default() -> Self {
        let (settings, settings_path) = load_settings();
        Self {
            meter_instances: Mutex::new(Default::default()),
            display_reconcile: Default::default(),
            pending_meter_drag: Mutex::new(None),
            detail_meter: Mutex::new(METER_WINDOW_LABEL.to_owned()),
            usage: Arc::new(load_usage_runtime()),
            refresh_coordinator: Arc::new(
                crate::collectors::refresh::RefreshCoordinator::with_backoff_path(
                    settings_path
                        .as_ref()
                        .map(|p| p.with_file_name("refresh-backoff.json")),
                ),
            ),
            settings: Mutex::new(settings),
            settings_path,
            meter_enabled: Arc::new(AtomicBool::new(true)),
            strip_folded: AtomicBool::new(false),
            strip_pointer: AtomicBool::new(false),
            strip_focus: AtomicBool::new(false),
            strip_menu: AtomicBool::new(false),
            strip_reset: AtomicBool::new(false),
            deepseek_key_replacement: ExclusiveOperationGate::default(),
            meter_drag: Arc::new(crate::platform::windows::meter_drag::MeterDragGate::default()),
            deepseek_history: Arc::new(Mutex::new(
                crate::platform::windows::deepseek_webview::DeepSeekHistoryWindowRuntime::default(),
            )),
            detail_state: Mutex::new(DetailState::default()),
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

    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) fn meter_drag_is_active(&self) -> bool {
        self.meter_drag.is_active()
    }

    pub(crate) fn migrate_meter_monitor_id(
        &self,
        previous_identifier: Option<&str>,
        migrated_identifier: Option<String>,
    ) {
        let Ok(mut settings) = self.settings.lock() else {
            return;
        };
        let _ = apply_meter_monitor_id_migration(
            &mut settings,
            previous_identifier,
            migrated_identifier,
            |candidate| persist_settings(self.settings_path.as_deref(), candidate),
        );
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

fn apply_meter_monitor_id_migration(
    settings: &mut AppSettings,
    previous_identifier: Option<&str>,
    migrated_identifier: Option<String>,
    persist: impl FnOnce(&AppSettings) -> Result<(), String>,
) -> bool {
    let (Some(previous_identifier), Some(migrated_identifier)) =
        (previous_identifier, migrated_identifier)
    else {
        return false;
    };
    if previous_identifier == migrated_identifier
        || settings.meter_monitor_id.as_deref() != Some(previous_identifier)
    {
        return false;
    }
    let mut candidate = settings.clone();
    candidate.meter_monitor_id = Some(migrated_identifier.clone());
    if let Some(displays) = candidate.displays.as_mut() {
        if displays.selected_id.as_deref() == Some(previous_identifier) {
            displays.selected_id = Some(migrated_identifier.clone());
        }
        if let Some(placement) = displays.placements.remove(previous_identifier) {
            displays
                .placements
                .entry(migrated_identifier)
                .or_insert(placement);
        }
    }
    if persist(&candidate).is_err() {
        return false;
    }
    *settings = candidate;
    true
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderDetailEffect {
    ShowDetailWindow,
    EmitActiveDetail,
}

pub fn provider_detail_effects(_: &UsageSnapshot) -> [ProviderDetailEffect; 2] {
    [
        ProviderDetailEffect::ShowDetailWindow,
        ProviderDetailEffect::EmitActiveDetail,
    ]
}

#[tauri::command]
fn show_provider_detail(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    state: State<'_, RuntimeState>,
    provider_id: ProviderId,
) -> Result<UsageSnapshot, String> {
    let mut detail_state = state
        .detail_state
        .lock()
        .map_err(|_| "The detail window state is temporarily unavailable".to_owned())?;
    if state.display_reconcile.is_active() {
        return Err("Displays are changing; try again".into());
    }
    let snapshot = state.usage.snapshot(provider_id);
    let effects = provider_detail_effects(&snapshot);
    let monitor_id = state
        .meter_instances
        .lock()
        .map_err(|_| "Meter unavailable")?
        .get(window.label())
        .cloned()
        .ok_or("Meter unavailable")?;
    let edge = edge_from_settings(
        state
            .app_settings_snapshot()
            .displays
            .unwrap_or_default()
            .placement(&monitor_id)
            .edge,
    );
    // Serialize owner assignment and presentation under the same detail revision lock.
    hide_detail_window(&app).map_err(|_| "The detail window could not be closed")?;
    *state.detail_meter.lock().map_err(|_| "Meter unavailable")? = window.label().to_owned();
    detail_state.open_with_rollback(
        provider_id,
        || {
            for effect in effects {
                match effect {
                    ProviderDetailEffect::ShowDetailWindow => show_detail_window(&app, edge)
                        .map_err(|_| "The detail window could not be shown".to_owned())?,
                    ProviderDetailEffect::EmitActiveDetail => app
                        .emit("active-detail-changed", &snapshot)
                        .map_err(|_| "The detail window could not be updated".to_owned())?,
                }
            }
            Ok::<(), String>(())
        },
        || {
            let _ = hide_detail_window(&app);
        },
    )?;
    Ok(snapshot)
}

#[cfg_attr(not(windows), allow(dead_code))]
fn finish_meter_drag(app: &tauri::AppHandle, label: &str, session: u64) -> Result<(), String> {
    let meter = app
        .get_webview_window(label)
        .ok_or_else(|| "The meter window is unavailable".to_owned())?;
    let (edge, normalized_y) = snap_meter_after_drag(&meter)
        .map_err(|_| "The meter could not be snapped to the screen edge".to_owned())?;
    let state = app.state::<RuntimeState>();
    let Some(id) = current_monitor_identifier(&meter).ok().flatten() else {
        return Ok(());
    };
    let committed = state.meter_drag.commit_if_owned(session, || {
        let mut settings = state.settings.lock().map_err(|_| "Settings unavailable")?;
        let mut candidate = settings.clone();
        candidate.normalize_display_preferences();
        candidate.displays.as_mut().unwrap().record_drag(
            &id,
            crate::platform::windows::monitor::DisplayPlacement {
                edge: edge_to_settings(edge),
                vertical_per_mille: (normalized_y * 1000.0).round().clamp(0.0, 1000.0) as u16,
            },
        );
        persist_settings(state.settings_path.as_deref(), &candidate)?;
        *settings = candidate;
        Ok::<(), String>(())
    });
    let Some(committed) = committed else {
        return Ok(());
    };
    committed?;
    meter
        .emit("meter-edge-changed", edge_to_settings(edge))
        .map_err(|_| "The meter display could not be updated".to_owned())?;
    Ok(())
}

#[tauri::command]
fn begin_meter_drag(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    state: State<'_, RuntimeState>,
) -> Result<(), String> {
    let session = state.display_reconcile.reserve_drag(&state.meter_drag)?;
    if !state
        .meter_instances
        .lock()
        .is_ok_and(|instances| instances.contains_key(window.label()))
    {
        state.meter_drag.finish(session);
        return Err("Meter unavailable".into());
    }
    let gate = Arc::clone(&state.meter_drag);

    #[cfg(windows)]
    std::thread::spawn(move || {
        let released = crate::platform::windows::meter_drag::wait_for_primary_button_release();
        if released {
            gate.commit_if_owned(session, || {
                *app.state::<RuntimeState>()
                    .pending_meter_drag
                    .lock()
                    .unwrap() = Some((window.label().to_owned(), session));
            });
        } else {
            let _ = gate.finish(session);
        }
        let _ = crate::platform::windows::display_coordinator::reconcile(&app);
    });

    #[cfg(not(windows))]
    {
        let _ = app;
        let _ = window;
        let _ = gate.finish(session);
    }
    Ok(())
}

#[tauri::command]
fn set_meter_edge(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
    edge: MeterEdge,
) -> Result<(), String> {
    let ids: Vec<_> = state
        .meter_instances
        .lock()
        .map_err(|_| "Meter unavailable")?
        .values()
        .cloned()
        .collect();
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "Settings are temporarily unavailable".to_owned())?;
    let mut candidate = settings.clone();
    candidate.normalize_display_preferences();
    let displays = candidate.displays.as_mut().unwrap();
    for id in ids {
        displays.record_edge(&id, edge);
    }
    candidate.edge = edge;
    persist_settings(state.settings_path.as_deref(), &candidate)?;
    *settings = candidate.clone();
    drop(settings);
    crate::platform::windows::display_coordinator::reconcile(&app)
        .map_err(|_| "The meter could not be moved")?;
    app.emit("app-settings-changed", candidate)
        .map_err(|_| "The meter display could not be updated".to_owned())
}

#[tauri::command]
fn available_displays(
    app: tauri::AppHandle,
) -> Result<Vec<crate::platform::windows::display_coordinator::DisplayInfo>, String> {
    crate::platform::windows::display_coordinator::online(&app)
        .map_err(|_| "Displays unavailable".into())
}

#[tauri::command]
fn set_display_mode(
    app: tauri::AppHandle,
    mode: crate::platform::windows::monitor::DisplayMode,
    selected_id: Option<String>,
) -> Result<(), String> {
    let state = app.state::<RuntimeState>();
    state.meter_drag.cancel();
    let candidate = {
        let mut settings = state.settings.lock().map_err(|_| "Settings unavailable")?;
        let mut candidate = settings.clone();
        candidate.normalize_display_preferences();
        let displays = candidate.displays.as_mut().unwrap();
        displays
            .select_mode(mode, selected_id)
            .map_err(str::to_owned)?;
        persist_settings(state.settings_path.as_deref(), &candidate)?;
        *settings = candidate.clone();
        candidate
    };
    crate::platform::windows::display_coordinator::reconcile(&app)
        .map_err(|_| "The meter could not be moved")?;
    app.emit("app-settings-changed", candidate)
        .map_err(|_| "Settings update failed".into())
}

// Background drag completion publishes on the same UI queue as synchronous
// settings commands, taking its snapshot at delivery rather than before native work.
pub(crate) fn publish_settings(app: &tauri::AppHandle) -> tauri::Result<()> {
    let app = app.clone();
    let handle = app.clone();
    handle.run_on_main_thread(move || {
        let current = app.state::<RuntimeState>().app_settings_snapshot();
        let _ = app.emit("app-settings-changed", current);
    })
}

#[tauri::command]
fn app_settings(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    state: State<'_, RuntimeState>,
) -> AppSettings {
    let mut settings = state
        .settings
        .lock()
        .map_or_else(|_| AppSettings::default(), |settings| settings.clone());
    if window.label().starts_with("meter") {
        settings.edge = crate::platform::windows::display_coordinator::placement_for_window(
            &app,
            window.label(),
        )
        .edge;
    }
    settings
}

#[tauri::command]
fn set_locale(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
    locale: crate::persistence::Locale,
) -> Result<(), String> {
    let candidate = {
        let mut settings = state.settings.lock().map_err(|_| "Settings unavailable")?;
        let mut candidate = settings.clone();
        candidate.locale = locale;
        persist_settings(state.settings_path.as_deref(), &candidate)?;
        *settings = candidate.clone();
        candidate
    };
    for (label, title) in [
        ("settings", "AI Token Meter Settings"),
        ("detail", "AI Token Meter Details"),
        ("deepseek-history", "DeepSeek Usage · AI Token Meter"),
    ] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.set_title(crate::localization::text(locale, title));
        }
    }
    app.emit("app-settings-changed", candidate)
        .map_err(|_| "Settings update failed".to_owned())
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
    #[cfg(windows)]
    let value = validated_provider_cli_settings(provider_id, value)?;
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

#[cfg(windows)]
fn validated_provider_cli_settings(
    provider_id: ProviderId,
    mut value: ProviderCliSettings,
) -> Result<ProviderCliSettings, String> {
    if value.mode != crate::persistence::CliRuntimeMode::Wsl
        && let Some(path) = value.custom_path.as_deref()
    {
        let provider = match provider_id {
            ProviderId::Claude => crate::accounts::cli_account::CliProvider::Claude,
            ProviderId::Codex => crate::accounts::cli_account::CliProvider::Codex,
            ProviderId::DeepSeek => return Err("DeepSeek does not use a CLI".to_owned()),
            ProviderId::Gemini => {
                return Err("Gemini CLI integration is currently unavailable".to_owned());
            }
        };
        value.custom_path = Some(
            crate::collectors::application::validate_custom_path(provider, path)
                .map_err(str::to_owned)?,
        );
    }
    Ok(value)
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
        crate::platform::windows::process::BoundedProcessRunner
            .run(request)
            .map(|output| {
                crate::platform::windows::wsl::decode_distribution_list(output.stdout.as_bytes())
            })
            .unwrap_or_default()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[tauri::command]
fn close_provider_detail(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
) -> Result<(), String> {
    let mut detail_state = state
        .detail_state
        .lock()
        .map_err(|_| "The detail window state is temporarily unavailable".to_owned())?;
    detail_state.close();
    drop(detail_state);
    hide_detail_window(&app).map_err(|_| "The detail window could not be closed".to_owned())
}

fn handle_detail_focus_lost(app: &tauri::AppHandle) {
    let state = app.state::<RuntimeState>();
    if state.strip_menu.load(std::sync::atomic::Ordering::Acquire) {
        return;
    }
    let Ok(mut detail_state) = state.detail_state.lock() else {
        return;
    };
    let should_hide = detail_state.focus_lost() == DetailCommand::HideAndClearTopmost;
    drop(detail_state);
    if should_hide {
        let _ = hide_detail_window(app);
    }
}

#[tauri::command]
fn open_settings(app: tauri::AppHandle, tab: Option<String>) -> Result<(), String> {
    let tab = validated_settings_tab(tab.as_deref())?;
    show_settings_window(&app).map_err(|_| "Settings could not be opened".to_owned())?;
    if let Some(tab) = tab {
        app.emit("settings-tab-requested", tab)
            .map_err(|_| "Settings tab could not be opened".to_owned())?;
    }
    Ok(())
}

fn validated_settings_tab(tab: Option<&str>) -> Result<Option<&'static str>, String> {
    match tab {
        None => Ok(None),
        Some("Appearance") => Ok(Some("Appearance")),
        Some("Monitoring") => Ok(Some("Monitoring")),
        Some("Services") => Ok(Some("Services")),
        Some("About") => Ok(Some("About")),
        Some(_) => Err("Unknown Settings tab".to_owned()),
    }
}

#[tauri::command]
async fn open_deepseek_history(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
) -> Result<crate::platform::windows::deepseek_history_window::DeepSeekHistoryStatusSnapshot, String>
{
    crate::platform::windows::deepseek_webview::open_history_window(
        &app,
        Arc::clone(&state.deepseek_history),
    )
    .await
}

#[tauri::command]
fn deepseek_history_status(
    state: State<'_, RuntimeState>,
) -> Result<crate::platform::windows::deepseek_history_window::DeepSeekHistoryStatusSnapshot, String>
{
    crate::platform::windows::deepseek_webview::history_status(&state.deepseek_history)
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
        let mut statuses =
            crate::accounts::windows_service::read_all(&checked_at, _state.app_settings_snapshot())
                .await;
        statuses.retain(|status| status.provider_id != ProviderId::Gemini);
        statuses.push(crate::collectors::gemini::service_status(
            &_state.usage.snapshot(ProviderId::Gemini),
        ));
        Ok(statuses)
    }
    #[cfg(not(windows))]
    {
        Ok([
            ProviderId::Claude,
            ProviderId::Codex,
            ProviderId::DeepSeek,
            ProviderId::Gemini,
        ]
        .into_iter()
        .map(|provider| {
            crate::accounts::service_status::ServiceAccountStatus::unavailable(
                provider,
                &checked_at,
            )
        })
        .collect())
    }
}

#[tauri::command]
async fn service_account_status(
    _app: tauri::AppHandle,
    provider_id: ProviderId,
    _retry_usage: bool,
    _state: State<'_, RuntimeState>,
) -> Result<crate::accounts::service_status::ServiceAccountStatus, String> {
    let checked_at = current_timestamp();
    #[cfg(windows)]
    {
        let status = if provider_id == ProviderId::Gemini {
            crate::collectors::gemini::service_status(&_state.usage.snapshot(ProviderId::Gemini))
        } else {
            crate::accounts::windows_service::read_one(
                provider_id,
                &checked_at,
                _state.app_settings_snapshot(),
            )
            .await
        };
        if _retry_usage {
            _state
                .refresh_coordinator
                .clear_manual_retry_backoff(provider_id);
            crate::collectors::application::trigger_provider(
                &_app,
                provider_id,
                crate::collectors::refresh::RefreshPriority::Manual,
            );
        }
        Ok(status)
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
fn begin_claude_usage_initialization(
    provider_id: ProviderId,
    _state: State<'_, RuntimeState>,
) -> Result<(), String> {
    if provider_id != ProviderId::Claude {
        return Err("Only Claude Code uses the private usage workspace".to_owned());
    }
    #[cfg(windows)]
    {
        let settings = _state.app_settings_snapshot();
        crate::accounts::windows_service::launch_claude_usage_initialization(&settings.claude_cli)
            .map_err(str::to_owned)
    }
    #[cfg(not(windows))]
    {
        Err("Claude Code usage initialization is available in the Windows app".to_owned())
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
        ProviderId::Gemini => {
            return Err("Gemini CLI integration is currently unavailable".to_owned());
        }
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
fn open_service_installation_guide(provider_id: ProviderId) -> Result<(), String> {
    let provider = match provider_id {
        ProviderId::Claude => crate::accounts::cli_account::CliProvider::Claude,
        ProviderId::Codex => crate::accounts::cli_account::CliProvider::Codex,
        ProviderId::DeepSeek => return Err("This service does not use a CLI".to_owned()),
        ProviderId::Gemini => {
            return Err("Gemini CLI integration is currently unavailable".to_owned());
        }
    };
    #[cfg(windows)]
    {
        crate::accounts::windows_service::open_installation_guide(provider).map_err(str::to_owned)
    }
    #[cfg(not(windows))]
    {
        let _ = provider;
        Err("Available in the Windows app".to_owned())
    }
}

#[tauri::command]
async fn begin_service_installation(
    provider_id: ProviderId,
    state: State<'_, RuntimeState>,
) -> Result<crate::accounts::installation::InstallationDecision, String> {
    let provider = match provider_id {
        ProviderId::Claude => crate::accounts::cli_account::CliProvider::Claude,
        ProviderId::Codex => crate::accounts::cli_account::CliProvider::Codex,
        ProviderId::DeepSeek => return Err("DeepSeek uses an API Key".to_owned()),
        ProviderId::Gemini => {
            return Err("Gemini CLI integration is currently unavailable".to_owned());
        }
    };
    let configuration = state
        .app_settings_snapshot()
        .cli_settings(provider_id)
        .cloned()
        .ok_or("This service does not use a CLI")?;
    #[cfg(windows)]
    {
        tauri::async_runtime::spawn_blocking(move || {
            crate::accounts::windows_service::launch_installation(provider, &configuration)
        })
        .await
        .map_err(|_| "The installation could not be started".to_owned())?
        .map_err(str::to_owned)
    }
    #[cfg(not(windows))]
    {
        let _ = (provider, configuration);
        Err("CLI installation is available in the Windows app".to_owned())
    }
}

#[tauri::command]
async fn replace_deepseek_api_key(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
) -> Result<crate::accounts::service_status::ServiceAccountStatus, String> {
    #[cfg(windows)]
    {
        let _replacement_guard = state
            .deepseek_key_replacement
            .try_enter()
            .map_err(|()| "DeepSeek API Key verification is already in progress".to_owned())?;
        let parent = app
            .get_webview_window("settings")
            .and_then(|window| window.hwnd().ok())
            .map_or(std::ptr::null_mut(), |handle| handle.0 as _);
        let candidate = crate::platform::windows::credential_prompt::prompt_deepseek_api_key(
            parent,
            state.app_settings_snapshot().locale,
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
            .refresh_coordinator
            .clear_authentication_backoff(ProviderId::DeepSeek);
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

#[cfg_attr(not(windows), allow(dead_code))]
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
        let mut settings = AtomicJsonStore::read::<AppSettings>(&paths.settings_file)
            .ok()
            .flatten()
            .unwrap_or_default();
        settings.strip_preferences.normalize();
        settings.normalize_display_preferences();
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

#[tauri::command]
fn set_strip_preferences(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
    mut value: crate::platform::windows::strip_preferences::StripPreferences,
) -> Result<(), String> {
    value.normalize();
    let updated = {
        let mut settings = state.settings.lock().map_err(|_| "Settings unavailable")?;
        let mut candidate = settings.clone();
        candidate.strip_preferences = value;
        persist_settings(state.settings_path.as_deref(), &candidate)?;
        *settings = candidate.clone();
        candidate
    };
    let selected = state
        .detail_state
        .lock()
        .ok()
        .and_then(|detail| detail.current_provider());
    if let Some(provider) = selected {
        let id = match provider {
            ProviderId::Claude => "claude",
            ProviderId::Codex => "codex",
            ProviderId::DeepSeek => "deepseek",
            ProviderId::Gemini => "gemini",
        };
        if !updated
            .strip_preferences
            .visible_providers()
            .iter()
            .any(|p| p == id)
        {
            close_provider_detail(app.clone(), app.state())?;
        }
    }
    state
        .strip_folded
        .store(false, std::sync::atomic::Ordering::Release);
    state
        .strip_reset
        .store(true, std::sync::atomic::Ordering::Release);
    app.emit("strip-folded", false)
        .map_err(|_| "Window update failed")?;
    crate::platform::windows::strip_runtime::restore(&app).map_err(|_| "Window resize failed")?;
    app.emit("app-settings-changed", updated)
        .map_err(|_| "Settings update failed".to_owned())
}

#[tauri::command]
fn strip_interaction(state: State<'_, RuntimeState>, kind: String, active: bool) {
    use std::sync::atomic::Ordering;
    match kind.as_str() {
        "pointer" => state.strip_pointer.store(active, Ordering::Release),
        "focus" => state.strip_focus.store(active, Ordering::Release),
        _ => {}
    }
}

#[tauri::command]
fn strip_folded(state: State<'_, RuntimeState>) -> bool {
    state
        .strip_folded
        .load(std::sync::atomic::Ordering::Acquire)
}

#[tauri::command]
async fn strip_context_menu(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    use tauri::menu::{ContextMenu, MenuBuilder, MenuItemBuilder};
    let state = app.state::<RuntimeState>();
    let refreshing = state
        .usage
        .snapshots()
        .iter()
        .any(|s| s.status == crate::domain::UsageStatus::Refreshing);
    let locale = state.app_settings_snapshot().locale;
    let refresh = MenuItemBuilder::with_id(
        "strip-refresh",
        crate::localization::text(locale, "Refresh now"),
    )
    .enabled(!refreshing)
    .build(&app)
    .map_err(|_| "Menu unavailable")?;
    let menu = MenuBuilder::new(&app)
        .item(&refresh)
        .text(
            "strip-hide",
            crate::localization::text(locale, "Hide for 1 hour"),
        )
        .separator()
        .text(
            "strip-settings",
            crate::localization::text(locale, "Settings…"),
        )
        .text(
            "strip-quit",
            crate::localization::text(locale, "Quit AI Token Meter"),
        )
        .build()
        .map_err(|_| "Menu unavailable")?;
    state.strip_menu.store(true, Ordering::Release);
    let _ = app.emit("strip-context-menu", true);
    let result = menu
        .popup(window.as_ref().window())
        .map_err(|_| "Menu could not open".to_owned());
    state.strip_menu.store(false, Ordering::Release);
    let _ = app.emit("strip-context-menu", false);
    result
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .on_menu_event(|app, event| match event.id().as_ref() {
            "strip-refresh" => {
                let _ = app.emit("refresh-requested", ());
            }
            "strip-hide" => {
                let state = app.state::<RuntimeState>();
                let mut value = state.app_settings_snapshot().strip_preferences;
                value.hidden_until = Some(time::OffsetDateTime::now_utc().unix_timestamp() + 3600);
                let _ = set_strip_preferences(app.clone(), state, value);
                let _ = hide_detail_window(app);
                for meter in crate::platform::windows::display_coordinator::meter_windows(app) {
                    let _ = meter.hide();
                }
            }
            "strip-settings" => {
                let _ = show_settings_window(app);
                let _ = app.emit("settings-tab-requested", "Appearance");
            }
            "strip-quit" => app.exit(0),
            _ => {}
        })
        .manage(RuntimeState::default())
        .invoke_handler(tauri::generate_handler![
            set_strip_preferences,
            available_displays,
            set_display_mode,
            set_locale,
            strip_interaction,
            strip_folded,
            strip_context_menu,
            usage_snapshots,
            show_provider_detail,
            close_provider_detail,
            open_settings,
            begin_meter_drag,
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
            deepseek_history_status,
            update_state,
            check_for_updates,
            install_update,
            service_account_statuses,
            service_account_status,
            begin_service_sign_in,
            begin_claude_usage_initialization,
            begin_service_installation,
            open_service_installation_guide,
            replace_deepseek_api_key,
            brand_links::open_brand_link,
            brand_links::open_gemini_installation_guide
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
            let migrated_monitor_id =
                configure_initial_windows(app.handle(), edge, normalized_y, monitor_id.as_deref())?;
            app.state::<RuntimeState>()
                .migrate_meter_monitor_id(monitor_id.as_deref(), migrated_monitor_id);
            crate::platform::windows::display_coordinator::reconcile(app.handle())?;
            let locale = app.state::<RuntimeState>().app_settings_snapshot().locale;
            for (label, title) in [
                ("settings", "AI Token Meter Settings"),
                ("detail", "AI Token Meter Details"),
            ] {
                if let Some(window) = app.get_webview_window(label) {
                    window.set_title(crate::localization::text(locale, title))?;
                }
            }
            crate::platform::windows::tray::install(app.handle())?;
            crate::platform::windows::strip_runtime::start(app.handle().clone());
            #[cfg(windows)]
            {
                crate::collectors::application::start(app.handle());
                let enabled = app.state::<RuntimeState>().meter_enabled.clone();
                crate::platform::windows::desktop_visibility::start_monitoring(
                    app.handle().clone(),
                    enabled,
                );
                crate::platform::windows::display_topology::start_monitoring(app.handle().clone());
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
                        handle_detail_focus_lost(&app_for_focus);
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
    fn deepseek_key_replacement_allows_only_one_operation_at_a_time() {
        let gate = ExclusiveOperationGate::default();
        let active = gate.try_enter().expect("first operation should start");
        assert!(gate.try_enter().is_err());
        drop(active);
        assert!(gate.try_enter().is_ok());
    }

    #[test]
    fn settings_tab_requests_accept_only_fixed_application_tabs() {
        assert_eq!(validated_settings_tab(None), Ok(None));
        for tab in ["Appearance", "Monitoring", "Services", "About"] {
            assert_eq!(validated_settings_tab(Some(tab)), Ok(Some(tab)));
        }
        assert_eq!(
            validated_settings_tab(Some("../../credentials")),
            Err("Unknown Settings tab".to_owned())
        );
    }

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

    #[test]
    fn failed_monitor_identifier_persistence_keeps_the_previous_setting() {
        let mut settings = AppSettings {
            meter_monitor_id: Some("\\\\.\\DISPLAY2".to_owned()),
            ..AppSettings::default()
        };

        let migrated = apply_meter_monitor_id_migration(
            &mut settings,
            Some("\\\\.\\DISPLAY2"),
            Some("device:stable-hash".to_owned()),
            |_| Err("disk full".to_owned()),
        );

        assert!(!migrated);
        assert_eq!(
            settings.meter_monitor_id.as_deref(),
            Some("\\\\.\\DISPLAY2")
        );
    }

    #[test]
    fn stale_monitor_identifier_migration_cannot_overwrite_a_newer_target() {
        let mut settings = AppSettings {
            meter_monitor_id: Some("device:new-target".to_owned()),
            ..AppSettings::default()
        };
        let mut persistence_called = false;

        let migrated = apply_meter_monitor_id_migration(
            &mut settings,
            Some("\\\\.\\DISPLAY2"),
            Some("device:old-target".to_owned()),
            |_| {
                persistence_called = true;
                Ok(())
            },
        );

        assert!(!migrated);
        assert!(!persistence_called);
        assert_eq!(
            settings.meter_monitor_id.as_deref(),
            Some("device:new-target")
        );
    }
}
