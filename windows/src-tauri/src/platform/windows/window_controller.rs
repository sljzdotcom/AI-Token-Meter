use crate::domain::ProviderId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalSize {
    pub width: u32,
    pub height: u32,
}

impl PhysicalSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

pub fn fitted_detail_size(work_area: PhysicalRect, desired: PhysicalSize) -> PhysicalSize {
    const WORK_AREA_MARGIN: u32 = 48;
    PhysicalSize::new(
        desired
            .width
            .min(work_area.size.width.saturating_sub(WORK_AREA_MARGIN).max(1)),
        desired.height.min(
            work_area
                .size
                .height
                .saturating_sub(WORK_AREA_MARGIN)
                .max(1),
        ),
    )
}

pub fn fitted_meter_size(work_area: PhysicalRect, desired: PhysicalSize) -> PhysicalSize {
    const WORK_AREA_MARGIN: u32 = 16;
    let available_width = work_area.size.width.saturating_sub(WORK_AREA_MARGIN).max(1);
    let available_height = work_area
        .size
        .height
        .saturating_sub(WORK_AREA_MARGIN)
        .max(1);
    if desired.width <= available_width && desired.height <= available_height {
        return desired;
    }
    let scale = (f64::from(available_width) / f64::from(desired.width.max(1)))
        .min(f64::from(available_height) / f64::from(desired.height.max(1)))
        .min(1.0);
    PhysicalSize::new(
        (f64::from(desired.width) * scale).round().max(1.0) as u32,
        (f64::from(desired.height) * scale).round().max(1.0) as u32,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalRect {
    pub origin: PhysicalPoint,
    pub size: PhysicalSize,
}

impl PhysicalRect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            origin: PhysicalPoint { x, y },
            size: PhysicalSize { width, height },
        }
    }

    pub fn right(self) -> i32 {
        self.origin
            .x
            .saturating_add(unsigned_to_i32(self.size.width))
    }

    pub fn bottom(self) -> i32 {
        self.origin
            .y
            .saturating_add(unsigned_to_i32(self.size.height))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowPlacement {
    pub origin: PhysicalPoint,
    pub size: PhysicalSize,
}

impl WindowPlacement {
    pub const fn new(x: i32, y: i32, size: PhysicalSize) -> Self {
        Self {
            origin: PhysicalPoint { x, y },
            size,
        }
    }

    pub fn meter(
        work_area: PhysicalRect,
        meter_size: PhysicalSize,
        edge: Edge,
        normalized_y: f64,
    ) -> Self {
        let width = unsigned_to_i32(meter_size.width);
        let x = match edge {
            Edge::Left => work_area.origin.x,
            Edge::Right => work_area.right().saturating_sub(width),
        };
        let available = work_area.size.height.saturating_sub(meter_size.height);
        let fraction = if normalized_y.is_finite() {
            normalized_y.clamp(0.0, 1.0)
        } else {
            0.5
        };
        let offset = (f64::from(available) * fraction).round() as i32;
        let y = work_area.origin.y.saturating_add(offset);
        Self::new(x, y, meter_size)
    }

    pub fn normalized_y(work_area: PhysicalRect, meter_size: PhysicalSize, y: i32) -> f64 {
        let available = work_area.size.height.saturating_sub(meter_size.height);
        if available == 0 {
            return 0.0;
        }
        let offset = y
            .saturating_sub(work_area.origin.y)
            .clamp(0, unsigned_to_i32(available));
        f64::from(offset) / f64::from(available)
    }

    pub fn detail(
        work_area: PhysicalRect,
        meter: WindowPlacement,
        detail_size: PhysicalSize,
        edge: Edge,
    ) -> Self {
        const GAP: i32 = 14;
        let detail_width = unsigned_to_i32(detail_size.width);
        let x = match edge {
            Edge::Left => meter
                .origin
                .x
                .saturating_add(unsigned_to_i32(meter.size.width))
                .saturating_add(GAP),
            Edge::Right => meter
                .origin
                .x
                .saturating_sub(detail_width)
                .saturating_sub(GAP),
        };
        let centered_y = meter
            .origin
            .y
            .saturating_add(unsigned_to_i32(meter.size.height) / 2)
            .saturating_sub(unsigned_to_i32(detail_size.height) / 2);
        let max_x = work_area.right().saturating_sub(detail_width);
        let max_y = work_area
            .bottom()
            .saturating_sub(unsigned_to_i32(detail_size.height));
        Self::new(
            x.clamp(work_area.origin.x, max_x.max(work_area.origin.x)),
            centered_y.clamp(work_area.origin.y, max_y.max(work_area.origin.y)),
            detail_size,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DetailOwnershipToken {
    provider: ProviderId,
    revision: u64,
}

impl DetailOwnershipToken {
    pub const fn provider(self) -> ProviderId {
        self.provider
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum DetailPhase {
    #[default]
    Closed,
    Visible(DetailOwnershipToken),
    SuspendedByHistory(DetailOwnershipToken),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DetailState {
    revision: u64,
    phase: DetailPhase,
}

impl DetailState {
    pub fn open(&mut self, provider: ProviderId) -> (DetailOwnershipToken, DetailCommand) {
        self.revision = self.revision.wrapping_add(1).max(1);
        let ownership = DetailOwnershipToken {
            provider,
            revision: self.revision,
        };
        self.phase = DetailPhase::Visible(ownership);
        (ownership, DetailCommand::ShowFocusedTopmost)
    }

    pub fn open_with_rollback<E>(
        &mut self,
        provider: ProviderId,
        apply: impl FnOnce() -> Result<(), E>,
        rollback: impl FnOnce(),
    ) -> Result<DetailOwnershipToken, E> {
        let (ownership, _) = self.open(provider);
        match apply() {
            Ok(()) => Ok(ownership),
            Err(error) => {
                rollback();
                self.invalidate();
                Err(error)
            }
        }
    }

    pub fn close(&mut self) -> DetailCommand {
        if self.phase == DetailPhase::Closed {
            return DetailCommand::Noop;
        }
        self.invalidate();
        DetailCommand::HideAndClearTopmost
    }

    pub const fn is_visible(self) -> bool {
        matches!(self.phase, DetailPhase::Visible(_))
    }

    pub const fn current_provider(self) -> Option<ProviderId> {
        match self.phase {
            DetailPhase::Closed => None,
            DetailPhase::Visible(ownership) | DetailPhase::SuspendedByHistory(ownership) => {
                Some(ownership.provider)
            }
        }
    }

    pub fn suspend_for_history(&mut self, provider: ProviderId) -> Option<DetailOwnershipToken> {
        let DetailPhase::Visible(ownership) = self.phase else {
            return None;
        };
        if ownership.provider != provider {
            return None;
        }
        self.phase = DetailPhase::SuspendedByHistory(ownership);
        Some(ownership)
    }

    pub fn is_suspended_by(self, ownership: DetailOwnershipToken) -> bool {
        matches!(self.phase, DetailPhase::SuspendedByHistory(current) if current == ownership)
    }

    pub fn focus_lost(&mut self) -> DetailCommand {
        if matches!(self.phase, DetailPhase::Visible(_)) {
            self.close()
        } else {
            DetailCommand::Noop
        }
    }

    pub fn restore_if_owned<E>(
        &mut self,
        ownership: DetailOwnershipToken,
        restore: impl FnOnce(ProviderId) -> Result<(), E>,
        rollback: impl FnOnce(),
    ) -> Result<bool, E> {
        if !self.is_suspended_by(ownership) {
            return Ok(false);
        }
        match restore(ownership.provider) {
            Ok(()) => {
                self.phase = DetailPhase::Visible(ownership);
                Ok(true)
            }
            Err(error) => {
                rollback();
                self.invalidate();
                Err(error)
            }
        }
    }

    fn invalidate(&mut self) {
        self.revision = self.revision.wrapping_add(1).max(1);
        self.phase = DetailPhase::Closed;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailCommand {
    ShowFocusedTopmost,
    HideAndClearTopmost,
    Noop,
}

pub fn meter_shape_points(size: PhysicalSize, edge: Edge) -> Vec<PhysicalPoint> {
    let width = unsigned_to_i32(size.width);
    let scale = |x: i32, y: i32| PhysicalPoint {
        x: (f64::from(x) * f64::from(size.width) / 108.0).round() as i32,
        y: (f64::from(y) * f64::from(size.height) / 356.0).round() as i32,
    };
    let mut points = cubic_points(
        scale(108, 16),
        scale(98, 23),
        scale(88, 27),
        scale(66, 28),
        64,
    );
    points.extend(
        cubic_points(scale(66, 28), scale(29, 29), scale(0, 54), scale(0, 88), 64)
            .into_iter()
            .skip(1),
    );
    points.push(scale(0, 268));
    points.extend(
        cubic_points(
            scale(0, 268),
            scale(0, 302),
            scale(29, 327),
            scale(66, 328),
            64,
        )
        .into_iter()
        .skip(1),
    );
    points.extend(
        cubic_points(
            scale(66, 328),
            scale(88, 329),
            scale(98, 333),
            scale(108, 340),
            64,
        )
        .into_iter()
        .skip(1),
    );
    if edge == Edge::Left {
        points
            .into_iter()
            .map(|point| PhysicalPoint {
                x: width.saturating_sub(point.x),
                y: point.y,
            })
            .collect()
    } else {
        points
    }
}

fn cubic_points(
    start: PhysicalPoint,
    control_1: PhysicalPoint,
    control_2: PhysicalPoint,
    end: PhysicalPoint,
    segments: usize,
) -> Vec<PhysicalPoint> {
    (0..=segments)
        .map(|step| {
            let t = step as f64 / segments as f64;
            let inverse = 1.0 - t;
            let coordinate = |p0: i32, p1: i32, p2: i32, p3: i32| {
                (inverse.powi(3) * f64::from(p0)
                    + 3.0 * inverse.powi(2) * t * f64::from(p1)
                    + 3.0 * inverse * t.powi(2) * f64::from(p2)
                    + t.powi(3) * f64::from(p3))
                .round() as i32
            };
            PhysicalPoint {
                x: coordinate(start.x, control_1.x, control_2.x, end.x),
                y: coordinate(start.y, control_1.y, control_2.y, end.y),
            }
        })
        .collect()
}

fn unsigned_to_i32(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

pub const METER_WINDOW_LABEL: &str = "meter";
pub const DETAIL_WINDOW_LABEL: &str = "detail";
pub const SETTINGS_WINDOW_LABEL: &str = "settings";

pub fn configure_initial_windows(
    app: &tauri::AppHandle,
    edge: Edge,
    normalized_y: f64,
    preferred_monitor_id: Option<&str>,
) -> tauri::Result<Option<String>> {
    use tauri::Manager;

    let meter = app
        .get_webview_window(METER_WINDOW_LABEL)
        .ok_or_else(|| tauri::Error::WindowNotFound)?;
    meter.set_skip_taskbar(true)?;
    meter.set_focusable(false)?;
    meter.set_always_on_top(false)?;
    let migrated_identifier =
        position_meter_on_preferred(&meter, edge, normalized_y, preferred_monitor_id)?;

    if let Some(detail) = app.get_webview_window(DETAIL_WINDOW_LABEL) {
        detail.set_skip_taskbar(true)?;
        detail.set_always_on_top(false)?;
    }
    Ok(migrated_identifier)
}

pub fn place_meter(
    meter: &tauri::WebviewWindow,
    edge: Edge,
    normalized_y: f64,
) -> tauri::Result<()> {
    let monitor = meter.current_monitor()?.or(meter.primary_monitor()?);
    let Some(monitor) = monitor else {
        return Ok(());
    };
    let work = from_tauri_rect(monitor.work_area());
    let meter_size = fitted_meter_size(work, desired_meter_size(meter)?);
    meter.set_size(tauri::PhysicalSize::new(
        meter_size.width,
        meter_size.height,
    ))?;
    let placement = WindowPlacement::meter(work, meter_size, edge, normalized_y);
    meter.set_position(tauri::PhysicalPosition::new(
        placement.origin.x,
        placement.origin.y,
    ))?;
    apply_windows_meter_style(meter, edge)
}

#[cfg(windows)]
fn apply_windows_meter_style(meter: &tauri::WebviewWindow, edge: Edge) -> tauri::Result<()> {
    use std::ffi::c_void;
    use std::mem::size_of;
    use windows_sys::Win32::Graphics::Dwm::{
        DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DwmSetWindowAttribute,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GetWindowLongW, SetWindowLongW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };

    meter.set_shadow(false)?;
    let hwnd = meter.hwnd()?.0 as windows_sys::Win32::Foundation::HWND;
    let extended_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) };
    unsafe {
        SetWindowLongW(
            hwnd,
            GWL_EXSTYLE,
            extended_style | WS_EX_TOOLWINDOW as i32 | WS_EX_NOACTIVATE as i32,
        );
        let border_color = DWMWA_COLOR_NONE;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR as u32,
            (&border_color as *const u32).cast::<c_void>(),
            size_of::<u32>() as u32,
        );
    }
    let _ = edge;
    Ok(())
}

#[cfg(not(windows))]
fn apply_windows_meter_style(_meter: &tauri::WebviewWindow, _edge: Edge) -> tauri::Result<()> {
    Ok(())
}

pub fn show_detail_window(app: &tauri::AppHandle, edge: Edge) -> tauri::Result<()> {
    use tauri::Manager;

    let meter = app
        .get_webview_window(METER_WINDOW_LABEL)
        .ok_or_else(|| tauri::Error::WindowNotFound)?;
    let detail = app
        .get_webview_window(DETAIL_WINDOW_LABEL)
        .ok_or_else(|| tauri::Error::WindowNotFound)?;
    position_detail_next_to_meter(&meter, &detail, edge)?;
    detail.set_always_on_top(true)?;
    detail.show()?;
    detail.set_focus()?;
    Ok(())
}

pub fn snap_meter_after_drag(meter: &tauri::WebviewWindow) -> tauri::Result<(Edge, f64)> {
    let Some(monitor) = meter.current_monitor()? else {
        return Ok((Edge::Right, 0.5));
    };
    let work = from_tauri_rect(monitor.work_area());
    let origin = meter.outer_position()?;
    let size = meter.inner_size()?;
    let meter_size = PhysicalSize::new(size.width, size.height);
    let distance_to_left = origin.x.saturating_sub(work.origin.x).unsigned_abs();
    let distance_to_right = work
        .right()
        .saturating_sub(origin.x.saturating_add(unsigned_to_i32(size.width)))
        .unsigned_abs();
    let edge = if distance_to_left <= distance_to_right {
        Edge::Left
    } else {
        Edge::Right
    };
    let normalized_y = WindowPlacement::normalized_y(work, meter_size, origin.y);
    let placement = WindowPlacement::meter(work, meter_size, edge, normalized_y);
    meter.set_position(tauri::PhysicalPosition::new(
        placement.origin.x,
        placement.origin.y,
    ))?;
    apply_windows_meter_style(meter, edge)?;
    Ok((edge, normalized_y))
}

pub fn hide_detail_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::{Emitter, Manager};

    let detail = app
        .get_webview_window(DETAIL_WINDOW_LABEL)
        .ok_or_else(|| tauri::Error::WindowNotFound)?;
    detail.set_always_on_top(false)?;
    detail.hide()?;
    app.emit("detail-closed", ())?;
    Ok(())
}

pub fn show_settings_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::Manager;

    let settings = app
        .get_webview_window(SETTINGS_WINDOW_LABEL)
        .ok_or_else(|| tauri::Error::WindowNotFound)?;
    settings.show()?;
    settings.set_focus()
}

pub fn toggle_meter_window(
    app: &tauri::AppHandle,
    edge: Edge,
    normalized_y: f64,
    preferred_monitor_id: Option<&str>,
) -> tauri::Result<bool> {
    use tauri::Manager;

    let meter = app
        .get_webview_window(METER_WINDOW_LABEL)
        .ok_or_else(|| tauri::Error::WindowNotFound)?;
    if meter.is_visible()? {
        meter.hide()?;
        Ok(false)
    } else {
        position_meter_on_preferred(&meter, edge, normalized_y, preferred_monitor_id)?;
        meter.show()?;
        Ok(true)
    }
}

pub fn restore_meter_position(
    meter: &tauri::WebviewWindow,
    edge: Edge,
    normalized_y: f64,
    preferred_monitor_id: Option<&str>,
) -> tauri::Result<Option<String>> {
    position_meter_on_preferred(meter, edge, normalized_y, preferred_monitor_id)
}

fn position_meter_on_preferred(
    meter: &tauri::WebviewWindow,
    edge: Edge,
    normalized_y: f64,
    preferred_monitor_id: Option<&str>,
) -> tauri::Result<Option<String>> {
    let monitors = meter.available_monitors()?;
    let primary_id = meter
        .primary_monitor()?
        .as_ref()
        .map(legacy_monitor_identifier);
    let identities = monitors
        .iter()
        .map(|monitor| {
            let legacy_id = legacy_monitor_identifier(monitor);
            monitor_identity(monitor, primary_id.as_deref() == Some(legacy_id.as_str()))
        })
        .collect::<Vec<_>>();
    let resolution = super::monitor::resolve_monitor(&identities, preferred_monitor_id);
    let selected = resolution
        .as_ref()
        .and_then(|resolution| {
            identities
                .iter()
                .position(|candidate| candidate == resolution.selected)
        })
        .and_then(|index| monitors.get(index));
    let Some(monitor) = selected else {
        return Ok(None);
    };
    let work = from_tauri_rect(monitor.work_area());
    let meter_size = fitted_meter_size(work, desired_meter_size(meter)?);
    meter.set_size(tauri::PhysicalSize::new(
        meter_size.width,
        meter_size.height,
    ))?;
    let placement = WindowPlacement::meter(work, meter_size, edge, normalized_y);
    meter.set_position(tauri::PhysicalPosition::new(
        placement.origin.x,
        placement.origin.y,
    ))?;
    apply_windows_meter_style(meter, edge)?;
    Ok(resolution.and_then(|resolution| resolution.migrated_identifier))
}

pub fn current_monitor_identifier(meter: &tauri::WebviewWindow) -> tauri::Result<Option<String>> {
    Ok(meter
        .current_monitor()?
        .as_ref()
        .map(|monitor| monitor_identity(monitor, false).stable_id))
}

pub fn monitor_topology(
    meter: &tauri::WebviewWindow,
) -> tauri::Result<Vec<super::monitor::MonitorTopology>> {
    let monitors = meter.available_monitors()?;
    let primary_id = meter
        .primary_monitor()?
        .as_ref()
        .map(legacy_monitor_identifier);
    let mut topology = monitors
        .iter()
        .map(|monitor| {
            let legacy_id = legacy_monitor_identifier(monitor);
            let identity =
                monitor_identity(monitor, primary_id.as_deref() == Some(legacy_id.as_str()));
            let work = monitor.work_area();
            super::monitor::MonitorTopology::new(
                identity.stable_id,
                identity.is_primary,
                work.position.x,
                work.position.y,
                work.size.width,
                work.size.height,
            )
        })
        .collect::<Vec<_>>();
    topology.sort();
    Ok(topology)
}

fn monitor_identity(monitor: &tauri::Monitor, is_primary: bool) -> super::monitor::MonitorIdentity {
    let legacy_id = legacy_monitor_identifier(monitor);
    let stable_id = display_device_interface_name(&legacy_id)
        .as_deref()
        .and_then(super::monitor::stable_physical_identifier)
        .or_else(|| super::monitor::stable_runtime_identifier(&legacy_id))
        .unwrap_or_else(|| legacy_id.clone());
    super::monitor::MonitorIdentity::with_legacy_id(stable_id, legacy_id, is_primary)
}

fn legacy_monitor_identifier(monitor: &tauri::Monitor) -> String {
    monitor.name().cloned().unwrap_or_else(|| {
        let work = monitor.work_area();
        format!(
            "{}:{}:{}:{}",
            work.position.x, work.position.y, work.size.width, work.size.height
        )
    })
}

#[cfg(windows)]
fn display_device_interface_name(display_name: &str) -> Option<String> {
    use std::mem::{size_of, zeroed};
    use windows_sys::Win32::Graphics::Gdi::{DISPLAY_DEVICEW, EnumDisplayDevicesW};
    use windows_sys::Win32::UI::WindowsAndMessaging::EDD_GET_DEVICE_INTERFACE_NAME;

    let display_name = display_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut device: DISPLAY_DEVICEW = unsafe { zeroed() };
    device.cb = size_of::<DISPLAY_DEVICEW>() as u32;
    if unsafe {
        EnumDisplayDevicesW(
            display_name.as_ptr(),
            0,
            &mut device,
            EDD_GET_DEVICE_INTERFACE_NAME,
        )
    } == 0
    {
        return None;
    }
    let length = device
        .DeviceID
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(device.DeviceID.len());
    String::from_utf16(&device.DeviceID[..length])
        .ok()
        .filter(|value| !value.trim().is_empty())
}

#[cfg(not(windows))]
fn display_device_interface_name(_display_name: &str) -> Option<String> {
    None
}

fn position_detail_next_to_meter(
    meter: &tauri::WebviewWindow,
    detail: &tauri::WebviewWindow,
    edge: Edge,
) -> tauri::Result<()> {
    let Some(monitor) = meter.current_monitor()? else {
        return Ok(());
    };
    let work = from_tauri_rect(monitor.work_area());
    let meter_origin = meter.outer_position()?;
    let meter_size = meter.inner_size()?;
    let desired: tauri::PhysicalSize<u32> =
        tauri::LogicalSize::new(440.0, 760.0).to_physical(detail.scale_factor()?);
    let detail_size = fitted_detail_size(work, PhysicalSize::new(desired.width, desired.height));
    detail.set_size(tauri::PhysicalSize::new(
        detail_size.width,
        detail_size.height,
    ))?;
    let placement = WindowPlacement::detail(
        work,
        WindowPlacement::new(
            meter_origin.x,
            meter_origin.y,
            PhysicalSize::new(meter_size.width, meter_size.height),
        ),
        detail_size,
        edge,
    );
    detail.set_position(tauri::PhysicalPosition::new(
        placement.origin.x,
        placement.origin.y,
    ))
}

fn from_tauri_rect(rect: &tauri::PhysicalRect<i32, u32>) -> PhysicalRect {
    PhysicalRect::new(
        rect.position.x,
        rect.position.y,
        rect.size.width,
        rect.size.height,
    )
}

fn desired_meter_size(meter: &tauri::WebviewWindow) -> tauri::Result<PhysicalSize> {
    let desired: tauri::PhysicalSize<u32> =
        tauri::LogicalSize::new(116.0, 450.0).to_physical(meter.scale_factor()?);
    Ok(PhysicalSize::new(desired.width, desired.height))
}

#[cfg(test)]
mod tests {
    use super::{Edge, PhysicalPoint, PhysicalSize, meter_shape_points};

    #[test]
    fn meter_shape_uses_the_approved_macos_bezier_landmarks() {
        let points = meter_shape_points(PhysicalSize::new(108, 356), Edge::Right);

        for landmark in [
            PhysicalPoint { x: 108, y: 16 },
            PhysicalPoint { x: 66, y: 28 },
            PhysicalPoint { x: 0, y: 88 },
            PhysicalPoint { x: 0, y: 268 },
            PhysicalPoint { x: 66, y: 328 },
            PhysicalPoint { x: 108, y: 340 },
        ] {
            assert!(points.contains(&landmark), "missing landmark {landmark:?}");
        }
    }

    #[test]
    fn left_meter_shape_is_an_exact_horizontal_mirror() {
        let right = meter_shape_points(PhysicalSize::new(216, 712), Edge::Right);
        let left = meter_shape_points(PhysicalSize::new(216, 712), Edge::Left);

        assert_eq!(right.len(), left.len());
        for (right, left) in right.iter().zip(left) {
            assert_eq!(left.x, 216 - right.x);
            assert_eq!(left.y, right.y);
        }
    }
}
