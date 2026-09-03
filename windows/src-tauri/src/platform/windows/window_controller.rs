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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DetailState {
    visible: bool,
}

impl DetailState {
    pub fn open(&mut self) -> DetailCommand {
        self.visible = true;
        DetailCommand::ShowFocusedTopmost
    }

    pub fn close(&mut self) -> DetailCommand {
        if !self.visible {
            return DetailCommand::Noop;
        }
        self.visible = false;
        DetailCommand::HideAndClearTopmost
    }

    pub const fn is_visible(self) -> bool {
        self.visible
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
    let height = unsigned_to_i32(size.height);
    let shoulder = (height.saturating_mul(13) / 100).clamp(36, 58);
    let mut points = cubic_points(
        PhysicalPoint { x: width, y: 0 },
        PhysicalPoint { x: width, y: 0 },
        PhysicalPoint { x: 0, y: 10 },
        PhysicalPoint { x: 0, y: shoulder },
        16,
    );
    points.push(PhysicalPoint {
        x: 0,
        y: height - shoulder,
    });
    points.extend(
        cubic_points(
            PhysicalPoint {
                x: 0,
                y: height - shoulder,
            },
            PhysicalPoint {
                x: 0,
                y: height - 10,
            },
            PhysicalPoint {
                x: width,
                y: height,
            },
            PhysicalPoint {
                x: width,
                y: height,
            },
            16,
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
) -> tauri::Result<()> {
    use tauri::Manager;

    let meter = app
        .get_webview_window(METER_WINDOW_LABEL)
        .ok_or_else(|| tauri::Error::WindowNotFound)?;
    meter.set_skip_taskbar(true)?;
    meter.set_focusable(false)?;
    meter.set_always_on_top(false)?;
    position_meter_on_preferred(&meter, edge, normalized_y, preferred_monitor_id)?;

    if let Some(detail) = app.get_webview_window(DETAIL_WINDOW_LABEL) {
        detail.set_skip_taskbar(true)?;
        detail.set_always_on_top(false)?;
    }
    Ok(())
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
    let size = meter.inner_size()?;
    let placement = WindowPlacement::meter(
        work,
        PhysicalSize::new(size.width, size.height),
        edge,
        normalized_y,
    );
    meter.set_position(tauri::PhysicalPosition::new(
        placement.origin.x,
        placement.origin.y,
    ))?;
    apply_windows_meter_style(meter, edge)
}

#[cfg(windows)]
fn apply_windows_meter_style(meter: &tauri::WebviewWindow, edge: Edge) -> tauri::Result<()> {
    use windows_sys::Win32::Foundation::{POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        CreatePolygonRgn, DeleteObject, SetWindowRgn, WINDING,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GetClientRect, GetWindowLongW, SetWindowLongW, WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW,
    };

    let hwnd = meter.hwnd()?.0 as windows_sys::Win32::Foundation::HWND;
    let extended_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) };
    unsafe {
        SetWindowLongW(
            hwnd,
            GWL_EXSTYLE,
            extended_style | WS_EX_TOOLWINDOW as i32 | WS_EX_NOACTIVATE as i32,
        );
    }
    let mut client: RECT = unsafe { std::mem::zeroed() };
    if unsafe { GetClientRect(hwnd, &mut client) } == 0 {
        return Err(tauri::Error::InvalidWindowHandle);
    }
    let points = meter_shape_points(
        PhysicalSize::new(
            client.right.saturating_sub(client.left) as u32,
            client.bottom.saturating_sub(client.top) as u32,
        ),
        edge,
    )
    .into_iter()
    .map(|point| POINT {
        x: point.x,
        y: point.y,
    })
    .collect::<Vec<_>>();
    let region = unsafe { CreatePolygonRgn(points.as_ptr(), points.len() as i32, WINDING) };
    if region.is_null() {
        return Err(tauri::Error::InvalidWindowHandle);
    }
    if unsafe { SetWindowRgn(hwnd, region, 1) } == 0 {
        unsafe { DeleteObject(region) };
        return Err(tauri::Error::InvalidWindowHandle);
    }
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
    use tauri::Manager;

    let detail = app
        .get_webview_window(DETAIL_WINDOW_LABEL)
        .ok_or_else(|| tauri::Error::WindowNotFound)?;
    detail.set_always_on_top(false)?;
    detail.hide()
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

fn position_meter_on_preferred(
    meter: &tauri::WebviewWindow,
    edge: Edge,
    normalized_y: f64,
    preferred_monitor_id: Option<&str>,
) -> tauri::Result<()> {
    let monitors = meter.available_monitors()?;
    let primary_id = meter.primary_monitor()?.as_ref().map(monitor_identifier);
    let identities = monitors
        .iter()
        .map(|monitor| {
            super::monitor::MonitorIdentity::new(
                monitor_identifier(monitor),
                primary_id.as_deref() == Some(monitor_identifier(monitor).as_str()),
            )
        })
        .collect::<Vec<_>>();
    let selected = super::monitor::choose_monitor(&identities, preferred_monitor_id)
        .and_then(|identity| {
            identities
                .iter()
                .position(|candidate| candidate == identity)
        })
        .and_then(|index| monitors.get(index));
    let Some(monitor) = selected else {
        return Ok(());
    };
    let work = from_tauri_rect(monitor.work_area());
    let size = meter.inner_size()?;
    let placement = WindowPlacement::meter(
        work,
        PhysicalSize::new(size.width, size.height),
        edge,
        normalized_y,
    );
    meter.set_position(tauri::PhysicalPosition::new(
        placement.origin.x,
        placement.origin.y,
    ))?;
    apply_windows_meter_style(meter, edge)
}

pub fn current_monitor_identifier(meter: &tauri::WebviewWindow) -> tauri::Result<Option<String>> {
    Ok(meter.current_monitor()?.as_ref().map(monitor_identifier))
}

fn monitor_identifier(monitor: &tauri::Monitor) -> String {
    monitor.name().cloned().unwrap_or_else(|| {
        let work = monitor.work_area();
        format!(
            "{}:{}:{}:{}",
            work.position.x, work.position.y, work.size.width, work.size.height
        )
    })
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
    let detail_size = detail.inner_size()?;
    let placement = WindowPlacement::detail(
        work,
        WindowPlacement::new(
            meter_origin.x,
            meter_origin.y,
            PhysicalSize::new(meter_size.width, meter_size.height),
        ),
        PhysicalSize::new(detail_size.width, detail_size.height),
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
