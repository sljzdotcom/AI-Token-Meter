use super::monitor::{DisplayMode, DisplayPlacement, MonitorIdentity};
use super::window_controller::{self, METER_WINDOW_LABEL};
use std::collections::BTreeMap;
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};
use tauri::{Emitter, Manager};

#[derive(Default)]
pub struct ReconcileQueue(Mutex<(bool, bool)>);

impl ReconcileQueue {
    // Only the first requester starts a worker. Requests during native calls never
    // wait for that worker: they ask it to read current state in another pass.
    pub fn request(&self) -> bool {
        let mut state = self.0.lock().unwrap();
        if state.0 {
            state.1 = true;
            false
        } else {
            state.0 = true;
            true
        }
    }

    pub fn is_active(&self) -> bool {
        self.0.lock().unwrap().0
    }

    /// Reserve interaction against a new reconciliation request. The callback
    /// only touches in-memory state and must not perform native UI operations.
    pub fn run_if_idle<T>(&self, action: impl FnOnce() -> T) -> Option<T> {
        let state = self.0.lock().unwrap();
        if state.0 { None } else { Some(action()) }
    }

    pub fn reserve_drag(
        &self,
        drag: &super::meter_drag::MeterDragGate,
    ) -> Result<u64, &'static str> {
        self.run_if_idle(|| drag.try_begin())
            .flatten()
            .ok_or("Displays are changing or a drag is already active")
    }

    pub fn run(&self, mut pass: impl FnMut()) {
        loop {
            pass(); // no queue or instance lock across native/main-thread calls
            let mut state = self.0.lock().unwrap();
            if state.1 {
                state.1 = false;
            } else {
                state.0 = false;
                return;
            }
        }
    }
}

static NEXT_METER_LABEL: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayInfo {
    pub id: String,
    pub name: String,
    pub is_primary: bool,
}

#[derive(Debug)]
pub struct WindowPlan {
    pub assignments: BTreeMap<String, String>,
    pub remove: Vec<String>,
    pub close_detail: bool,
}

impl WindowPlan {
    pub fn new(
        previous: &BTreeMap<String, String>,
        targets: &[String],
        detail_owner: &str,
    ) -> Self {
        let assignments: BTreeMap<_, _> = targets
            .iter()
            .enumerate()
            .map(|(index, id)| {
                let label = if index == 0 {
                    METER_WINDOW_LABEL.to_owned()
                } else {
                    previous
                        .iter()
                        .find(|(label, assigned)| {
                            label.as_str() != METER_WINDOW_LABEL && *assigned == id
                        })
                        .map(|(label, _)| label.clone())
                        .unwrap_or_else(|| {
                            format!("meter-{}", NEXT_METER_LABEL.fetch_add(1, Ordering::Relaxed))
                        })
                };
                (label, id.clone())
            })
            .collect();
        let remove = previous
            .keys()
            .filter(|label| !assignments.contains_key(*label) && *label != METER_WINDOW_LABEL)
            .cloned()
            .collect();
        let close_detail = !assignments.contains_key(detail_owner)
            || previous.get(detail_owner) != assignments.get(detail_owner);
        Self {
            assignments,
            remove,
            close_detail,
        }
    }
}

pub fn online(app: &tauri::AppHandle) -> tauri::Result<Vec<DisplayInfo>> {
    let meter = app
        .get_webview_window(METER_WINDOW_LABEL)
        .ok_or(tauri::Error::WindowNotFound)?;
    let primary = meter.primary_monitor()?;
    let mut bounds = std::collections::HashSet::new();
    Ok(meter
        .available_monitors()?
        .iter()
        .filter(|m| {
            bounds.insert((
                m.position().x,
                m.position().y,
                m.size().width,
                m.size().height,
            ))
        })
        .map(|m| {
            let is_primary = primary
                .as_ref()
                .is_some_and(|p| p.name() == m.name() && p.position() == m.position());
            DisplayInfo {
                id: window_controller::monitor_identity(m, is_primary).stable_id,
                name: m.name().cloned().unwrap_or_else(|| "Display".into()),
                is_primary,
            }
        })
        .collect())
}

pub fn meter_windows(app: &tauri::AppHandle) -> Vec<tauri::WebviewWindow> {
    let state = app.state::<crate::RuntimeState>();
    let labels: Vec<_> = state
        .meter_instances
        .lock()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    labels
        .iter()
        .filter_map(|label| app.get_webview_window(label))
        .collect()
}

pub fn reconcile(app: &tauri::AppHandle) -> tauri::Result<()> {
    let state = app.state::<crate::RuntimeState>();
    if state.display_reconcile.request() {
        let app = app.clone();
        std::thread::spawn(move || {
            let state = app.state::<crate::RuntimeState>();
            state.display_reconcile.run(|| {
                let pending = state.pending_meter_drag.lock().unwrap().take();
                if let Some((label, session)) = pending {
                    if state.meter_drag.owns(session) {
                        let _ = crate::finish_meter_drag(&app, &label, session);
                    }
                    state.meter_drag.finish(session);
                    let _ = crate::publish_settings(&app);
                }
                match reconcile_once(&app) {
                    Ok(Some(applied_folded)) => {
                        let _ = app.emit("strip-folded", applied_folded);
                    }
                    Ok(None) => {}
                    Err(error) => {
                        eprintln!("Meter reconciliation failed: {error}");
                        state.strip_reset.store(true, Ordering::Release);
                    }
                }
            });
        });
    }
    Ok(())
}

fn reconcile_once(app: &tauri::AppHandle) -> tauri::Result<Option<bool>> {
    let state = app.state::<crate::RuntimeState>();
    if state.meter_drag_is_active() {
        return Ok(None);
    }
    let instances = state
        .meter_instances
        .lock()
        .map_err(|_| tauri::Error::WindowNotFound)?
        .clone();
    if state.meter_drag_is_active() {
        return Ok(None);
    }
    let applied_folded = state.strip_folded.load(Ordering::Acquire);
    let displays = online(app)?;
    let identities: Vec<_> = displays
        .iter()
        .map(|d| MonitorIdentity::new(&d.id, d.is_primary))
        .collect();
    let settings = state.app_settings_snapshot();
    let prefs = settings.displays.clone().unwrap_or_default();
    let targets = prefs.targets(&identities);
    if targets.is_empty() {
        return Ok(None);
    }
    let plan = {
        let mut detail = state
            .detail_state
            .lock()
            .map_err(|_| tauri::Error::WindowNotFound)?;
        let owner = state
            .detail_meter
            .lock()
            .map(|o| o.clone())
            .unwrap_or_default();
        let plan = WindowPlan::new(&instances, &targets, &owner);
        if plan.close_detail {
            detail.close();
        }
        plan
    };
    if plan.close_detail {
        let _ = window_controller::hide_detail_window(app);
    }
    for label in &plan.remove {
        if let Some(window) = app.get_webview_window(label) {
            window.destroy()?;
        }
        state.meter_instances.lock().unwrap().remove(label);
    }
    for (label, id) in &plan.assignments {
        let window = match app.get_webview_window(label) {
            Some(window) => window,
            None => {
                let builder = tauri::WebviewWindowBuilder::new(
                    app,
                    label,
                    tauri::WebviewUrl::App("index.html".into()),
                )
                .title("AI Token Meter")
                .inner_size(116.0, 450.0)
                .decorations(false)
                .shadow(false)
                .resizable(false)
                .skip_taskbar(true)
                .focused(false)
                .visible(false);
                #[cfg(windows)]
                let builder = builder.transparent(true);
                builder.build()?
            }
        };
        state
            .meter_instances
            .lock()
            .unwrap()
            .insert(label.clone(), id.clone());
        window.set_focusable(window_controller::meter_accepts_keyboard_focus())?;
        window.set_always_on_top(false)?;
        let placement = prefs.placement(id);
        window_controller::restore_meter_position(
            &window,
            crate::edge_from_settings(placement.edge),
            f64::from(placement.vertical_per_mille.min(1000)) / 1000.0,
            Some(id),
            applied_folded,
        )?;
        window.emit("meter-edge-changed", placement.edge)?;
        if state
            .meter_enabled
            .load(std::sync::atomic::Ordering::Acquire)
            && !settings
                .strip_preferences
                .hidden(time::OffsetDateTime::now_utc().unix_timestamp())
        {
            window.show()?;
        } else {
            window.hide()?;
        }
    }
    app.emit("displays-changed", displays)?;
    Ok(Some(applied_folded))
}

pub fn placement_for_window(app: &tauri::AppHandle, label: &str) -> DisplayPlacement {
    let state = app.state::<crate::RuntimeState>();
    let id = state
        .meter_instances
        .lock()
        .ok()
        .and_then(|m| m.get(label).cloned());
    let prefs = state.app_settings_snapshot().displays.unwrap_or_default();
    id.map(|id| prefs.placement(&id)).unwrap_or_default()
}

pub fn drag_owner(app: &tauri::AppHandle, label: &str) -> Option<String> {
    let state = app.state::<crate::RuntimeState>();
    if state
        .app_settings_snapshot()
        .displays
        .unwrap_or_default()
        .mode
        != DisplayMode::All
    {
        return None;
    }
    state
        .meter_instances
        .lock()
        .ok()
        .and_then(|m| m.get(label).cloned())
}
