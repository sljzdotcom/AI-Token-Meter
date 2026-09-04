pub const fn should_restore_topology(
    has_baseline: bool,
    topology_changed: bool,
    drag_active: bool,
) -> bool {
    has_baseline && topology_changed && !drag_active
}

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
            let Some(existing) = tracker.as_mut() else {
                tracker = Some(super::monitor::MonitorTopologyTracker::new(current));
                continue;
            };
            let topology_changed = existing.has_changed(&current);
            let state = app.state::<crate::RuntimeState>();
            if !should_restore_topology(true, topology_changed, state.meter_drag_is_active()) {
                if topology_changed && state.meter_drag_is_active() {
                    existing.commit(current);
                }
                continue;
            }
            let (edge, normalized_y, preferred_monitor_id) = state.meter_position();
            if let Ok(migrated_identifier) = super::window_controller::restore_meter_position(
                &meter,
                edge,
                normalized_y,
                preferred_monitor_id.as_deref(),
            ) {
                state
                    .migrate_meter_monitor_id(preferred_monitor_id.as_deref(), migrated_identifier);
                existing.commit(current);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_first_observation_only_establishes_a_baseline() {
        assert!(!super::should_restore_topology(false, true, false));
    }

    #[test]
    fn active_dragging_blocks_topology_repositioning() {
        assert!(!super::should_restore_topology(true, true, true));
    }

    #[test]
    fn a_real_change_after_the_baseline_is_restored() {
        assert!(super::should_restore_topology(true, true, false));
        assert!(!super::should_restore_topology(true, false, false));
    }
}
