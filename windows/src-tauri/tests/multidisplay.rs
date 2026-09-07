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
