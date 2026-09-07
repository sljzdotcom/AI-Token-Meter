pub const fn should_restore_topology(
    has_baseline: bool,
    topology_changed: bool,
    _drag_active: bool,
) -> bool {
    has_baseline && topology_changed
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
                continue;
            }
            state.meter_drag.cancel();
            for meter in super::display_coordinator::meter_windows(&app) {
                if let Ok(hwnd) = meter.hwnd() {
                    // Stop the system move loop as well as invalidating the persistence session.
                    unsafe {
                        windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW(
                            hwnd.0 as _,
                            windows_sys::Win32::UI::WindowsAndMessaging::WM_CANCELMODE,
                            0,
                            0,
                        );
                    }
                }
            }
            if super::display_coordinator::reconcile(&app).is_ok() {
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
    fn topology_change_cancels_drag_and_repositions_without_losing_the_change() {
        assert!(super::should_restore_topology(true, true, true));
    }

    #[test]
    fn a_real_change_after_the_baseline_is_restored() {
        assert!(super::should_restore_topology(true, true, false));
        assert!(!super::should_restore_topology(true, false, false));
    }
}
