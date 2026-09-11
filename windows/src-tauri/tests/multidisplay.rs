use ai_token_meter_windows::persistence::{AppSettings, MeterEdge};
use ai_token_meter_windows::platform::windows::monitor::MonitorIdentity;

#[test]
fn legacy_placement_migrates_once_to_selected_and_survives_reconnection() {
    let mut json = serde_json::to_value(AppSettings::default()).unwrap();
    json.as_object_mut().unwrap().remove("displays");
    json["meterMonitorId"] = "secondary".into();
    json["edge"] = "left".into();
    json["meterVerticalPerMille"] = 230.into();
    let mut settings: AppSettings = serde_json::from_value(json).unwrap();
    settings.normalize_display_preferences();
    let displays = settings.displays.as_ref().unwrap();
    let primary = vec![MonitorIdentity::new("primary", true)];
    assert_eq!(displays.targets(&primary), vec!["primary"]);
    assert_eq!(displays.selected_id.as_deref(), Some("secondary"));
    let reconnected = vec![MonitorIdentity::new("secondary", false), primary[0].clone()];
    assert_eq!(displays.targets(&reconnected), vec!["secondary"]);
    assert_eq!(displays.placement("secondary").edge, MeterEdge::Left);
    assert_eq!(displays.placement("secondary").vertical_per_mille, 230);
    assert!(displays.targets(&[]).is_empty());
}

#[test]
fn all_displays_keep_independent_positions_and_single_drag_selects_target() {
    use ai_token_meter_windows::platform::windows::monitor::{
        DisplayMode, DisplayPlacement, DisplayPreferences,
    };
    let mut displays = DisplayPreferences {
        mode: DisplayMode::All,
        ..Default::default()
    };
    let monitors = vec![
        MonitorIdentity::new("secondary", false),
        MonitorIdentity::new("primary", true),
    ];
    displays.record_drag(
        "secondary",
        DisplayPlacement {
            edge: MeterEdge::Left,
            vertical_per_mille: 210,
        },
    );
    displays.record_drag(
        "primary",
        DisplayPlacement {
            edge: MeterEdge::Right,
            vertical_per_mille: 780,
        },
    );
    assert_eq!(displays.targets(&monitors), vec!["primary", "secondary"]);
    assert_eq!(displays.placement("secondary").vertical_per_mille, 210);
    assert_eq!(displays.placement("primary").vertical_per_mille, 780);
    assert_eq!(displays.mode, DisplayMode::All);
    displays.mode = DisplayMode::Primary;
    displays.record_drag(
        "secondary",
        DisplayPlacement {
            edge: MeterEdge::Right,
            vertical_per_mille: 300,
        },
    );
    assert_eq!(displays.mode, DisplayMode::Selected);
    assert_eq!(displays.targets(&monitors), vec!["secondary"]);
}

#[test]
fn pointer_selects_negative_and_offset_screen_but_all_mode_keeps_owner() {
    use ai_token_meter_windows::platform::windows::monitor::{MonitorTopology, drag_target};
    let screens = vec![
        MonitorTopology::new("left", false, -1920, -300, 1920, 1080),
        MonitorTopology::new("primary", true, 0, 0, 2560, 1440),
    ];
    assert_eq!(drag_target(&screens, -1800, -200, None), Some("left"));
    assert_eq!(drag_target(&screens, 500, 400, None), Some("primary"));
    assert_eq!(drag_target(&screens, 500, 400, Some("left")), Some("left"));
}

#[test]
fn edge_edit_preserves_follow_primary_but_claims_a_borrowed_selected_screen() {
    use ai_token_meter_windows::platform::windows::monitor::{DisplayMode, DisplayPreferences};
    let mut prefs = DisplayPreferences::default();
    prefs.record_edge("primary", MeterEdge::Left);
    assert_eq!(prefs.mode, DisplayMode::Primary);
    prefs.mode = DisplayMode::Selected;
    prefs.selected_id = Some("offline".into());
    prefs.record_drag("offline", Default::default());
    prefs.record_edge("primary", MeterEdge::Right);
    assert_eq!(prefs.selected_id.as_deref(), Some("primary"));
    assert!(prefs.placements.contains_key("offline"));
    prefs.mode = DisplayMode::All;
    prefs.record_edge("external", MeterEdge::Left);
    assert_eq!(prefs.mode, DisplayMode::All);
}

#[test]
fn window_plan_releases_detail_owner_before_disconnected_window_is_removed() {
    use ai_token_meter_windows::platform::windows::display_coordinator::WindowPlan;
    let before = std::collections::BTreeMap::from([
        ("meter".into(), "primary".into()),
        ("meter-secondary".into(), "secondary".into()),
    ]);
    let plan = WindowPlan::new(&before, &["primary".into()], "meter-secondary");
    assert!(plan.close_detail);
    assert_eq!(plan.remove, vec!["meter-secondary"]);
    assert_eq!(plan.assignments.len(), 1);
    let moved = WindowPlan::new(&before, &["secondary".into()], "meter");
    assert!(moved.close_detail);
    assert_eq!(moved.assignments["meter"], "secondary");
    let stable = WindowPlan::new(&before, &["primary".into(), "secondary".into()], "meter");
    assert!(!stable.close_detail);
}

#[test]
fn dpi_change_requires_reconciliation_even_if_physical_bounds_are_unchanged() {
    use ai_token_meter_windows::platform::windows::monitor::{
        MonitorTopology, MonitorTopologyTracker,
    };
    let screen = MonitorTopology::new("primary", true, 0, 0, 1920, 1080);
    let tracker = MonitorTopologyTracker::new(vec![screen.clone()]);
    let mut changed = screen;
    changed.scale_per_mille = 1500;
    assert!(tracker.has_changed(&[changed]));
}

#[test]
fn choosing_primary_in_selected_dropdown_clears_old_target_without_erasing_placement() {
    use ai_token_meter_windows::platform::windows::monitor::{DisplayMode, DisplayPreferences};
    let mut prefs = DisplayPreferences::default();
    prefs.record_drag("secondary", Default::default());
    prefs.select_mode(DisplayMode::Selected, None).unwrap();
    assert_eq!(prefs.selected_id, None);
    assert_eq!(
        prefs.targets(&[
            MonitorIdentity::new("primary", true),
            MonitorIdentity::new("secondary", false)
        ]),
        vec!["primary"]
    );
    assert!(prefs.placements.contains_key("secondary"));
    let before = prefs.clone();
    assert!(
        prefs
            .select_mode(DisplayMode::All, Some(" ".into()))
            .is_err()
    );
    assert_eq!(prefs, before);
}

#[test]
fn reconcile_requests_during_native_work_are_nonblocking_and_coalesce_to_latest_state() {
    use ai_token_meter_windows::platform::windows::display_coordinator::ReconcileQueue;
    use std::sync::{Arc, Mutex};
    let queue = Arc::new(ReconcileQueue::default());
    let settings = Arc::new(Mutex::new("old"));
    let (started, wait_started) = std::sync::mpsc::channel();
    let (resume, wait_resume) = std::sync::mpsc::channel();
    assert!(queue.request());
    assert_eq!(queue.run_if_idle(|| "must not start drag"), None);
    let worker_queue = queue.clone();
    let worker_settings = settings.clone();
    let worker = std::thread::spawn(move || {
        let mut effects = Vec::new();
        worker_queue.run(|| {
            let snapshot = *worker_settings.lock().unwrap();
            if effects.is_empty() {
                started.send(()).unwrap();
                wait_resume.recv().unwrap(); // native API waits for UI
            }
            effects.push(snapshot);
        });
        effects
    });
    wait_started.recv().unwrap();
    // A UI callback can read/edit state and enqueue without waiting for the worker.
    let ui_queue = queue.clone();
    let (responded, response) = std::sync::mpsc::channel();
    let ui = std::thread::spawn(move || {
        *settings.try_lock().unwrap() = "new";
        responded
            .send((ui_queue.request(), ui_queue.request()))
            .unwrap();
    });
    let responsive = response.recv_timeout(std::time::Duration::from_secs(1));
    resume.send(()).unwrap();
    let effects = worker.join().unwrap();
    ui.join().unwrap();
    assert_eq!(responsive.unwrap(), (false, false));
    assert_eq!(effects, vec!["old", "new"]);
    assert!(!queue.is_active());
    assert_eq!(queue.run_if_idle(|| "drag started"), Some("drag started"));
    assert!(queue.request());
    queue.run(|| {});
}

#[test]
fn reconcile_receipt_waits_for_the_native_pass_that_owns_the_request() {
    use ai_token_meter_windows::platform::windows::display_coordinator::ReconcileQueue;
    use std::sync::Arc;
    use std::time::Duration;

    let queue = Arc::new(ReconcileQueue::default());
    let (start_worker, first_receipt) = queue.request_with_receipt();
    assert!(start_worker);
    let (started, wait_started) = std::sync::mpsc::channel();
    let (release, wait_release) = std::sync::mpsc::channel();
    let worker_queue = queue.clone();
    let worker = std::thread::spawn(move || {
        let mut pass = 0;
        worker_queue.run_with_outcome(|| {
            pass += 1;
            started.send(pass).unwrap();
            wait_release.recv().unwrap();
            Ok(())
        });
    });

    assert_eq!(wait_started.recv().unwrap(), 1);
    let (start_second_worker, second_receipt) = queue.request_with_receipt();
    assert!(!start_second_worker);
    assert!(
        first_receipt
            .recv_timeout(Duration::from_millis(20))
            .is_err()
    );
    assert!(
        second_receipt
            .recv_timeout(Duration::from_millis(20))
            .is_err()
    );

    release.send(()).unwrap();
    assert_eq!(
        first_receipt.recv_timeout(Duration::from_secs(1)).unwrap(),
        Ok(())
    );
    assert_eq!(
        wait_started.recv_timeout(Duration::from_secs(1)).unwrap(),
        2
    );
    assert!(
        second_receipt
            .recv_timeout(Duration::from_millis(20))
            .is_err()
    );
    release.send(()).unwrap();
    assert_eq!(
        second_receipt.recv_timeout(Duration::from_secs(1)).unwrap(),
        Ok(())
    );
    worker.join().unwrap();
}

#[test]
fn recreated_secondary_never_reuses_a_pending_destroy_window_label() {
    use ai_token_meter_windows::platform::windows::display_coordinator::WindowPlan;
    let empty = std::collections::BTreeMap::new();
    let targets = ["primary".into(), "secondary".into()];
    let first = WindowPlan::new(&empty, &targets, "meter");
    let removed = WindowPlan::new(&first.assignments, &["primary".into()], "meter");
    let recreated = WindowPlan::new(&removed.assignments, &targets, "meter");
    assert!(!recreated.assignments.contains_key(&removed.remove[0]));
    let stable = WindowPlan::new(&recreated.assignments, &targets, "meter");
    assert_eq!(stable.assignments, recreated.assignments);
}

#[test]
fn rejected_drag_reservation_is_an_error_so_frontend_does_not_start_native_drag() {
    use ai_token_meter_windows::platform::windows::{
        display_coordinator::ReconcileQueue, meter_drag::MeterDragGate,
    };
    let queue = ReconcileQueue::default();
    let drag = MeterDragGate::default();
    queue.request();
    assert!(queue.reserve_drag(&drag).is_err());
    assert!(!drag.is_active());
    queue.run(|| {});
    let session = queue.reserve_drag(&drag).unwrap();
    assert!(drag.owns(session));
    assert!(queue.reserve_drag(&drag).is_err());
}

#[test]
fn failed_background_restore_retries_even_when_fold_state_did_not_change() {
    use ai_token_meter_windows::platform::windows::strip_runtime::needs_restore;
    assert!(needs_restore(false, false, true));
    assert!(needs_restore(true, false, false));
    assert!(!needs_restore(false, false, false));
}
