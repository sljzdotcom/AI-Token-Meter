use ai_token_meter_windows::platform::windows::desktop_visibility::{
    ForegroundWindow, should_hide_for_foreground,
};
use ai_token_meter_windows::platform::windows::monitor::{MonitorIdentity, choose_monitor};
use ai_token_meter_windows::platform::windows::window_controller::{
    DetailCommand, DetailState, Edge, PhysicalRect, PhysicalSize, WindowPlacement,
    fitted_detail_size, fitted_meter_size, meter_shape_points,
};

#[test]
fn meter_clamps_to_either_work_area_edge_at_any_dpi() {
    let work = PhysicalRect::new(100, 60, 2300, 1320);
    let size = PhysicalSize::new(174, 585);

    assert_eq!(
        WindowPlacement::meter(work, size, Edge::Right, 0.5),
        WindowPlacement::new(2226, 428, size)
    );
    assert_eq!(
        WindowPlacement::meter(work, size, Edge::Left, -1.0),
        WindowPlacement::new(100, 60, size)
    );
    assert_eq!(
        WindowPlacement::meter(work, size, Edge::Right, 2.0),
        WindowPlacement::new(2226, 795, size)
    );
}

#[test]
fn dragging_is_persisted_as_a_resolution_independent_vertical_fraction() {
    let first = PhysicalRect::new(0, 0, 1920, 1040);
    let size = PhysicalSize::new(116, 390);
    let fraction = WindowPlacement::normalized_y(first, size, 325);
    assert!((fraction - 0.5).abs() < f64::EPSILON);

    let changed = PhysicalRect::new(-2560, 40, 2560, 1400);
    assert_eq!(
        WindowPlacement::meter(changed, size, Edge::Right, fraction)
            .origin
            .y,
        545
    );
}

#[test]
fn detail_opens_inward_and_remains_inside_the_work_area() {
    let work = PhysicalRect::new(0, 0, 1920, 1040);
    let meter_size = PhysicalSize::new(116, 450);
    let detail_size = PhysicalSize::new(440, 760);
    let right_meter = WindowPlacement::meter(work, meter_size, Edge::Right, 0.5);
    assert_eq!(
        WindowPlacement::detail(work, right_meter, detail_size, Edge::Right),
        WindowPlacement::new(1350, 140, detail_size)
    );

    let left_meter = WindowPlacement::meter(work, meter_size, Edge::Left, 0.0);
    assert_eq!(
        WindowPlacement::detail(work, left_meter, detail_size, Edge::Left),
        WindowPlacement::new(130, 0, detail_size)
    );
}

#[test]
fn detail_size_is_capped_to_small_high_dpi_work_areas() {
    let work = PhysicalRect::new(0, 0, 1280, 720);
    assert_eq!(
        fitted_detail_size(work, PhysicalSize::new(1320, 2280)),
        PhysicalSize::new(1232, 672)
    );
    assert_eq!(
        fitted_detail_size(work, PhysicalSize::new(660, 1140)),
        PhysicalSize::new(660, 672)
    );
}

#[test]
fn meter_size_scales_down_proportionally_for_small_high_dpi_work_areas() {
    let work = PhysicalRect::new(0, 0, 800, 600);
    assert_eq!(
        fitted_meter_size(work, PhysicalSize::new(232, 900)),
        PhysicalSize::new(151, 584)
    );
    assert_eq!(
        fitted_meter_size(work, PhysicalSize::new(116, 450)),
        PhysicalSize::new(116, 450)
    );
}

#[test]
fn a_fullscreen_foreground_on_the_same_monitor_hides_the_meter() {
    let monitor = PhysicalRect::new(0, 0, 1920, 1080);
    let fullscreen = ForegroundWindow::application(PhysicalRect::new(0, 0, 1920, 1080), 7);
    assert!(should_hide_for_foreground(monitor, Some(fullscreen), 7));

    let normal = ForegroundWindow::application(PhysicalRect::new(30, 20, 1200, 800), 7);
    assert!(!should_hide_for_foreground(monitor, Some(normal), 7));
    assert!(!should_hide_for_foreground(monitor, Some(fullscreen), 8));
    assert!(!should_hide_for_foreground(monitor, None, 7));
}

#[test]
fn detail_topmost_is_scoped_to_the_visible_session() {
    let mut state = DetailState::default();
    assert_eq!(state.open(), DetailCommand::ShowFocusedTopmost);
    assert!(state.is_visible());
    assert_eq!(state.close(), DetailCommand::HideAndClearTopmost);
    assert!(!state.is_visible());
    assert_eq!(state.close(), DetailCommand::Noop);
}

#[test]
fn a_disconnected_monitor_falls_back_to_primary_without_losing_side() {
    let monitors = [
        MonitorIdentity::new("internal", false),
        MonitorIdentity::new("desk", true),
    ];
    assert_eq!(
        choose_monitor(&monitors, Some("missing")),
        Some(&monitors[1])
    );
    assert_eq!(
        choose_monitor(&monitors, Some("internal")),
        Some(&monitors[0])
    );
}

#[test]
fn meter_shape_tapers_to_the_screen_edge_without_square_shoulders() {
    let points = meter_shape_points(PhysicalSize::new(116, 450), Edge::Right);
    assert_eq!(
        points.first().map(|point| (point.x, point.y)),
        Some((116, 20))
    );
    assert_eq!(
        points.last().map(|point| (point.x, point.y)),
        Some((116, 430))
    );
    assert!(points.iter().any(|point| point.x == 0 && point.y == 111));
    assert!(points.iter().any(|point| point.x == 0 && point.y == 339));
    assert!(points.iter().all(|point| (0..=116).contains(&point.x)));

    let left_points = meter_shape_points(PhysicalSize::new(116, 450), Edge::Left);
    assert_eq!(
        left_points.first().map(|point| (point.x, point.y)),
        Some((0, 20))
    );
    assert_eq!(
        left_points.last().map(|point| (point.x, point.y)),
        Some((0, 430))
    );
    assert!(
        left_points
            .iter()
            .any(|point| point.x == 116 && point.y == 111)
    );
    assert!(
        left_points
            .iter()
            .any(|point| point.x == 116 && point.y == 339)
    );
    assert_eq!(points.len(), left_points.len());
    assert!(
        points
            .iter()
            .zip(left_points.iter())
            .all(|(right, left)| { right.x.saturating_add(left.x) == 116 && right.y == left.y })
    );
}
