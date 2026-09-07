use super::strip_preferences::FoldState;
use super::window_controller::METER_WINDOW_LABEL;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};

pub fn restore(app: &tauri::AppHandle) -> tauri::Result<()> {
    super::display_coordinator::reconcile(app)
}

pub fn needs_restore(previous: bool, next: bool, retry_or_reset: bool) -> bool {
    previous != next || retry_or_reset
}

pub fn start(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let origin = Instant::now();
        let mut fold = FoldState::default();
        loop {
            std::thread::sleep(Duration::from_millis(250));
            let Some(meter) = app.get_webview_window(METER_WINDOW_LABEL) else {
                return;
            };
            let state = app.state::<crate::RuntimeState>();
            let prefs = state.app_settings_snapshot().strip_preferences;
            let reset = state.strip_reset.swap(false, Ordering::AcqRel);
            let locked = state.strip_pointer.load(Ordering::Acquire)
                || reset
                || state.strip_focus.load(Ordering::Acquire)
                || state.strip_menu.load(Ordering::Acquire)
                || state.meter_drag_is_active()
                || state
                    .detail_state
                    .lock()
                    .map(|d| d.current_provider().is_some())
                    .unwrap_or(true)
                || state
                    .usage
                    .snapshots()
                    .iter()
                    .any(|s| s.status == crate::domain::UsageStatus::Refreshing)
                || app
                    .get_webview_window("settings")
                    .is_some_and(|settings| settings.is_visible().unwrap_or(false))
                || screen_reader_active()
                || !meter.is_visible().unwrap_or(false);
            let next = fold.update(origin.elapsed().as_secs_f64(), prefs.fold_delay, locked);
            let previous = state.strip_folded.swap(next, Ordering::AcqRel);
            if needs_restore(previous, next, reset) {
                if restore(&app).is_ok() {
                    let _ = app.emit("strip-folded", next);
                } else {
                    state.strip_folded.store(previous, Ordering::Release);
                    state.strip_reset.store(true, Ordering::Release);
                }
            }
        }
    });
}

fn screen_reader_active() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SPI_GETSCREENREADER, SystemParametersInfoW,
        };
        let mut active: i32 = 0;
        // SPI_GETSCREENREADER writes one Win32 BOOL to the supplied buffer.
        unsafe {
            SystemParametersInfoW(SPI_GETSCREENREADER, 0, (&mut active as *mut i32).cast(), 0);
        }
        active != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}
