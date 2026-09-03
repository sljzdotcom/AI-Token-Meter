use super::window_controller::PhysicalRect;

const FULLSCREEN_TOLERANCE: i32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForegroundWindow {
    pub bounds: PhysicalRect,
    pub monitor_handle: u64,
    pub is_application: bool,
}

impl ForegroundWindow {
    pub const fn application(bounds: PhysicalRect, monitor_handle: u64) -> Self {
        Self {
            bounds,
            monitor_handle,
            is_application: true,
        }
    }
}

pub fn should_hide_for_foreground(
    monitor_bounds: PhysicalRect,
    foreground: Option<ForegroundWindow>,
    meter_monitor_handle: u64,
) -> bool {
    let Some(foreground) = foreground else {
        return false;
    };
    foreground.is_application
        && foreground.monitor_handle == meter_monitor_handle
        && covers_monitor(foreground.bounds, monitor_bounds)
}

fn covers_monitor(window: PhysicalRect, monitor: PhysicalRect) -> bool {
    window.origin.x <= monitor.origin.x.saturating_add(FULLSCREEN_TOLERANCE)
        && window.origin.y <= monitor.origin.y.saturating_add(FULLSCREEN_TOLERANCE)
        && window.right() >= monitor.right().saturating_sub(FULLSCREEN_TOLERANCE)
        && window.bottom() >= monitor.bottom().saturating_sub(FULLSCREEN_TOLERANCE)
}

#[cfg(windows)]
pub fn start_monitoring(
    app: tauri::AppHandle,
    meter_enabled: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    use std::sync::atomic::Ordering;
    use std::time::Duration;
    use tauri::Manager;

    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(500));
            let Some(meter) = app.get_webview_window(super::window_controller::METER_WINDOW_LABEL)
            else {
                return;
            };
            let enabled = meter_enabled.load(Ordering::Acquire);
            let hide_for_fullscreen =
                enabled && foreground_covers_meter_monitor(&meter).unwrap_or(false);
            if !enabled || hide_for_fullscreen {
                let _ = meter.hide();
            } else {
                let _ = meter.show();
            }
        }
    });
}

#[cfg(windows)]
fn foreground_covers_meter_monitor(meter: &tauri::WebviewWindow) -> tauri::Result<bool> {
    use windows_sys::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromWindow};

    let meter_hwnd = meter.hwnd()?.0 as windows_sys::Win32::Foundation::HWND;
    let monitor = unsafe { MonitorFromWindow(meter_hwnd, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_null() {
        return Ok(false);
    }
    let Some(bounds) = monitor_bounds(monitor) else {
        return Ok(false);
    };
    let foreground = foreground_window();
    Ok(should_hide_for_foreground(
        bounds,
        foreground,
        monitor as usize as u64,
    ))
}

#[cfg(windows)]
fn foreground_window() -> Option<ForegroundWindow> {
    use windows_sys::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromWindow};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetDesktopWindow, GetForegroundWindow, GetShellWindow, GetWindowRect, IsWindowVisible,
    };

    let window = unsafe { GetForegroundWindow() };
    if window.is_null()
        || window == unsafe { GetShellWindow() }
        || window == unsafe { GetDesktopWindow() }
        || unsafe { IsWindowVisible(window) } == 0
    {
        return None;
    }
    let mut rect = unsafe { std::mem::zeroed() };
    if unsafe { GetWindowRect(window, &mut rect) } == 0 {
        return None;
    }
    let monitor = unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_null() {
        return None;
    }
    Some(ForegroundWindow::application(
        PhysicalRect::new(
            rect.left,
            rect.top,
            rect.right.saturating_sub(rect.left) as u32,
            rect.bottom.saturating_sub(rect.top) as u32,
        ),
        monitor as usize as u64,
    ))
}

#[cfg(windows)]
fn monitor_bounds(monitor: windows_sys::Win32::Graphics::Gdi::HMONITOR) -> Option<PhysicalRect> {
    use std::mem::{size_of, zeroed};
    use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITORINFO};

    let mut info: MONITORINFO = unsafe { zeroed() };
    info.cbSize = size_of::<MONITORINFO>() as u32;
    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return None;
    }
    Some(PhysicalRect::new(
        info.rcMonitor.left,
        info.rcMonitor.top,
        info.rcMonitor.right.saturating_sub(info.rcMonitor.left) as u32,
        info.rcMonitor.bottom.saturating_sub(info.rcMonitor.top) as u32,
    ))
}
