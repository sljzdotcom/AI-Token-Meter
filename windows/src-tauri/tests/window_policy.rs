use std::cell::Cell;
use std::sync::{Arc, Mutex, mpsc};

use ai_token_meter_windows::domain::ProviderId;
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
    let (_, command) = state.open(ProviderId::Claude);
    assert_eq!(command, DetailCommand::ShowFocusedTopmost);
    assert!(state.is_visible());
    assert_eq!(state.close(), DetailCommand::HideAndClearTopmost);
    assert!(!state.is_visible());
    assert_eq!(state.close(), DetailCommand::Noop);
}

#[test]
fn stale_history_restore_cannot_override_a_new_provider_or_revision() {
    let mut state = DetailState::default();
    state.open(ProviderId::DeepSeek);
    let first = state
        .suspend_for_history(ProviderId::DeepSeek)
        .expect("visible DeepSeek detail can be suspended");

    state.open(ProviderId::Claude);
    assert!(
        !state
            .restore_if_owned(first, |_| Ok::<_, ()>(()), || {})
            .unwrap()
    );
    assert_eq!(state.current_provider(), Some(ProviderId::Claude));

    state.open(ProviderId::DeepSeek);
    assert!(
        !state
            .restore_if_owned(first, |_| Ok::<_, ()>(()), || {})
            .unwrap()
    );
    assert_eq!(state.current_provider(), Some(ProviderId::DeepSeek));
}

#[test]
fn stale_history_restore_cannot_reopen_an_explicitly_closed_detail() {
    let mut state = DetailState::default();
    state.open(ProviderId::DeepSeek);
    let first = state
        .suspend_for_history(ProviderId::DeepSeek)
        .expect("visible DeepSeek detail can be suspended");
    assert_eq!(state.close(), DetailCommand::HideAndClearTopmost);

    assert!(
        !state
            .restore_if_owned(first, |_| Ok::<_, ()>(()), || {})
            .unwrap()
    );
    assert_eq!(state.current_provider(), None);
}

#[test]
fn history_suspension_survives_the_internal_focus_loss() {
    let mut state = DetailState::default();
    state.open(ProviderId::DeepSeek);
    let ownership = state
        .suspend_for_history(ProviderId::DeepSeek)
        .expect("visible DeepSeek detail can be suspended");

    assert_eq!(state.focus_lost(), DetailCommand::Noop);
    assert!(state.is_suspended_by(ownership));

    assert!(
        state
            .restore_if_owned(ownership, |_| Ok::<_, ()>(()), || {})
            .unwrap()
    );
    assert_eq!(state.current_provider(), Some(ProviderId::DeepSeek));
}

#[test]
fn failed_history_restore_rolls_back_partial_window_effects_and_closes_ownership() {
    let mut state = DetailState::default();
    state.open(ProviderId::DeepSeek);
    let ownership = state
        .suspend_for_history(ProviderId::DeepSeek)
        .expect("visible DeepSeek detail can be suspended");
    let physically_visible = Cell::new(false);

    let restored = state.restore_if_owned(
        ownership,
        |_| {
            physically_visible.set(true);
            Err::<(), _>("late publish failure")
        },
        || physically_visible.set(false),
    );

    assert_eq!(restored, Err("late publish failure"));
    assert!(!physically_visible.get());
    assert_eq!(state.current_provider(), None);
    assert_eq!(state.focus_lost(), DetailCommand::Noop);
    assert!(
        !state
            .restore_if_owned(ownership, |_| Ok::<_, ()>(()), || {})
            .unwrap()
    );
}

#[test]
fn failed_provider_switch_rolls_back_partial_window_effects_and_closes_state() {
    let mut state = DetailState::default();
    state.open(ProviderId::Claude);
    let physically_visible = Cell::new(true);

    let opened = state.open_with_rollback(
        ProviderId::Codex,
        || {
            physically_visible.set(true);
            Err::<(), _>("focus failure")
        },
        || physically_visible.set(false),
    );

    assert_eq!(opened, Err("focus failure"));
    assert!(!physically_visible.get());
    assert_eq!(state.current_provider(), None);
}

#[test]
fn normal_visible_detail_focus_loss_closes_the_owned_revision() {
    let mut state = DetailState::default();
    state.open(ProviderId::Codex);

    assert_eq!(state.focus_lost(), DetailCommand::HideAndClearTopmost);
    assert_eq!(state.current_provider(), None);
}

#[test]
fn restore_side_effect_is_serialized_before_a_new_provider_selection() {
    let state = Arc::new(Mutex::new(DetailState::default()));
    let ownership = {
        let mut state = state.lock().unwrap();
        state.open(ProviderId::DeepSeek);
        state
            .suspend_for_history(ProviderId::DeepSeek)
            .expect("visible DeepSeek detail can be suspended")
    };
    let order = Arc::new(Mutex::new(Vec::new()));
    let (restore_checked_tx, restore_checked_rx) = mpsc::channel();
    let (finish_restore_tx, finish_restore_rx) = mpsc::channel();

    let restore_state = Arc::clone(&state);
    let restore_order = Arc::clone(&order);
    let restore = std::thread::spawn(move || {
        let mut state = restore_state.lock().unwrap();
        state
            .restore_if_owned(
                ownership,
                |_| {
                    restore_checked_tx.send(()).unwrap();
                    finish_restore_rx.recv().unwrap();
                    restore_order.lock().unwrap().push(ProviderId::DeepSeek);
                    Ok::<_, ()>(())
                },
                || {},
            )
            .unwrap();
    });
    restore_checked_rx.recv().unwrap();

    let select_state = Arc::clone(&state);
    let select_order = Arc::clone(&order);
    let (select_started_tx, select_started_rx) = mpsc::channel();
    let select = std::thread::spawn(move || {
        select_started_tx.send(()).unwrap();
        let mut state = select_state.lock().unwrap();
        state.open(ProviderId::Claude);
        select_order.lock().unwrap().push(ProviderId::Claude);
    });
    select_started_rx.recv().unwrap();
    finish_restore_tx.send(()).unwrap();
    restore.join().unwrap();
    select.join().unwrap();

    assert_eq!(
        *order.lock().unwrap(),
        [ProviderId::DeepSeek, ProviderId::Claude]
    );
    assert_eq!(
        state.lock().unwrap().current_provider(),
        Some(ProviderId::Claude)
    );
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
