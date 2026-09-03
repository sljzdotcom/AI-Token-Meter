#[cfg(windows)]
pub fn start_monitoring(app: tauri::AppHandle) {
    use std::time::Duration;
    use tauri::Manager;

    std::thread::spawn(move || {
        let Some(meter) = app.get_webview_window(super::window_controller::METER_WINDOW_LABEL)
        else {
            return;
        };
        let mut tracker: Option<super::monitor::MonitorTopologyTracker> = None;

        loop {
            std::thread::sleep(Duration::from_millis(750));
            let Ok(current) = super::window_controller::monitor_topology(&meter) else {
                continue;
            };
            if tracker
                .as_ref()
                .is_some_and(|tracker| !tracker.has_changed(&current))
            {
                continue;
            }
            let state = app.state::<crate::RuntimeState>();
            let (edge, normalized_y, preferred_monitor_id) = state.meter_position();
            if let Ok(migrated_identifier) = super::window_controller::restore_meter_position(
                &meter,
                edge,
                normalized_y,
                preferred_monitor_id.as_deref(),
            ) {
                state
                    .migrate_meter_monitor_id(preferred_monitor_id.as_deref(), migrated_identifier);
                if let Some(tracker) = tracker.as_mut() {
                    tracker.commit(current);
                } else {
                    tracker = Some(super::monitor::MonitorTopologyTracker::new(current));
                }
            }
        }
    });
}
