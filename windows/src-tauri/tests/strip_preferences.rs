use ai_token_meter_windows::platform::windows::strip_preferences::{FoldState, StripPreferences};
use ai_token_meter_windows::platform::windows::window_controller::{
    Edge, PhysicalSize, WindowPlacement,
};

#[test]
fn malformed_strip_does_not_reset_monitor_or_edge() {
    use ai_token_meter_windows::persistence::{AppSettings, MeterEdge};
    let original = AppSettings {
        edge: MeterEdge::Left,
        meter_vertical_per_mille: 100,
        meter_monitor_id: Some("saved-monitor".into()),
        ..AppSettings::default()
    };
    let mut json = serde_json::to_value(&original).unwrap();
    json["stripPreferences"] = serde_json::json!({"foldDelay": -1, "density": null});
    let restored: AppSettings = serde_json::from_value(json).unwrap();
    assert_eq!(restored.edge, MeterEdge::Left);
    assert_eq!(restored.meter_monitor_id, original.meter_monitor_id);
    assert_eq!(restored.meter_vertical_per_mille, 100);
}

#[test]
fn folding_keeps_expanded_center_and_edge() {
    let expanded = WindowPlacement::new(1222, 125, PhysicalSize::new(78, 286));
    let folded = expanded.folded(PhysicalSize::new(12, 96), Edge::Right);
    assert_eq!(folded.origin.y + 48, expanded.origin.y + 143);
    assert_eq!(folded.origin.x + 12, expanded.origin.x + 78);
}

#[test]
fn density_switch_keeps_noncentral_anchor_at_all_dpi_scales() {
    use ai_token_meter_windows::platform::windows::window_controller::{
        POSITION_REFERENCE_HEIGHT, PhysicalRect,
    };
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let work = PhysicalRect::new(0, 0, (1200.0 * scale) as u32, (800.0 * scale) as u32);
        let reference = (POSITION_REFERENCE_HEIGHT * scale) as u32;
        let small = WindowPlacement::anchored_meter(
            work,
            PhysicalSize::new((78.0 * scale) as u32, (286.0 * scale) as u32),
            reference,
            Edge::Left,
            0.1,
        );
        let large = WindowPlacement::anchored_meter(
            work,
            PhysicalSize::new((108.0 * scale) as u32, (356.0 * scale) as u32),
            reference,
            Edge::Left,
            0.1,
        );
        assert!(
            (small.origin.y + small.size.height as i32 / 2
                - large.origin.y
                - large.size.height as i32 / 2)
                .abs()
                <= 1
        );
    }
}

#[test]
fn upgrade_preserves_legacy_450_pixel_position() {
    use ai_token_meter_windows::platform::windows::window_controller::{
        POSITION_REFERENCE_HEIGHT, PhysicalRect,
    };
    let work = PhysicalRect::new(0, 0, 1200, 800);
    let old = WindowPlacement::meter(work, PhysicalSize::new(116, 450), Edge::Left, 0.1);
    let new = WindowPlacement::anchored_meter(
        work,
        PhysicalSize::new(78, 286),
        POSITION_REFERENCE_HEIGHT as u32,
        Edge::Left,
        0.1,
    );
    assert_eq!(old.origin.y + 225, new.origin.y + 143);
}

#[test]
fn persisted_layout_is_normalized_and_cannot_hide_every_service() {
    let mut value: StripPreferences = serde_json::from_str(r#"{"orderedProviders":["codex","codex","unknown"],"hiddenProviders":["claude","codex","deepseek"]}"#).unwrap();
    value.normalize();
    assert_eq!(
        value.ordered_providers,
        vec!["codex", "claude", "deepseek", "gemini"]
    );
    assert_eq!(value.visible_providers(), vec!["codex"]);
    assert_eq!(value.logical_size(false), (78.0, 170.0));
    assert_eq!(value.logical_size(true), (12.0, 96.0));
    let restored: StripPreferences =
        serde_json::from_str(&serde_json::to_string(&value).unwrap()).unwrap();
    assert_eq!(value, restored);
}

#[test]
fn interaction_cancels_pending_fold_and_disabling_unfolds() {
    let mut state = FoldState::default();
    state.update(0.0, 5, false);
    state.update(4.0, 5, true);
    state.update(6.0, 5, false);
    assert!(!state.update(10.0, 5, false));
    assert!(state.update(11.0, 5, false));
    assert!(!state.update(12.0, 0, false));
}

#[test]
fn gemini_defaults_migration_and_reload_preserve_user_choices() {
    let fresh = StripPreferences::default();
    assert_eq!(
        fresh.visible_providers(),
        vec!["claude", "codex", "deepseek", "gemini"]
    );
    assert_eq!(fresh.logical_size(false), (78.0, 344.0));
    let mut legacy: StripPreferences = serde_json::from_str(r#"{"density":"comfortable","orderedProviders":["deepseek","claude","codex"],"hiddenProviders":["claude"],"hiddenUntil":123}"#).unwrap();
    legacy.normalize();
    assert_eq!(
        legacy.ordered_providers,
        vec!["deepseek", "claude", "codex", "gemini"]
    );
    assert_eq!(legacy.visible_providers(), vec!["deepseek", "codex"]);
    assert_eq!(legacy.logical_size(false), (108.0, 284.0));
    assert_eq!(legacy.hidden_until, Some(123));
    let stored = serde_json::to_value(&legacy).unwrap();
    assert_eq!(stored["schemaVersion"], 2);
    legacy.hidden_providers.retain(|id| id != "gemini");
    let mut restored: StripPreferences =
        serde_json::from_value(serde_json::to_value(&legacy).unwrap()).unwrap();
    restored.normalize();
    assert_eq!(restored, legacy);
    let mut already_present: StripPreferences = serde_json::from_str(r#"{"orderedProviders":["gemini","codex","claude","deepseek"],"hiddenProviders":["codex"]}"#).unwrap();
    already_present.normalize();
    assert_eq!(
        already_present.visible_providers(),
        vec!["gemini", "claude", "deepseek"]
    );
}

#[test]
fn every_four_provider_order_and_nonempty_subset_has_correct_native_size() {
    let ids = ["claude", "codex", "deepseek", "gemini"];
    let mut permutations = 0;
    for a in 0..4 {
        for b in 0..4 {
            for c in 0..4 {
                for d in 0..4 {
                    let order = [a, b, c, d];
                    if (0..4).any(|i| (i + 1..4).any(|j| order[i] == order[j])) {
                        continue;
                    }
                    permutations += 1;
                    for mask in 1u32..16 {
                        let visible: Vec<String> = order
                            .iter()
                            .filter(|&&i| mask & (1 << i) != 0)
                            .map(|&i| ids[i].into())
                            .collect();
                        let mut prefs = StripPreferences {
                            ordered_providers: order.iter().map(|&i| ids[i].into()).collect(),
                            hidden_providers: ids
                                .iter()
                                .enumerate()
                                .filter(|(i, _)| mask & (1 << i) == 0)
                                .map(|(_, id)| (*id).into())
                                .collect(),
                            ..Default::default()
                        };
                        prefs.normalize();
                        assert_eq!(prefs.visible_providers(), visible);
                        let count = visible.len();
                        assert_eq!(
                            prefs.logical_size(false),
                            (78.0, [170.0, 228.0, 286.0, 344.0][count - 1])
                        );
                        prefs.density = "comfortable".into();
                        assert_eq!(
                            prefs.logical_size(false),
                            (108.0, [212.0, 284.0, 356.0, 428.0][count - 1])
                        );
                        let mut restored: StripPreferences =
                            serde_json::from_value(serde_json::to_value(&prefs).unwrap()).unwrap();
                        restored.normalize();
                        assert_eq!(restored, prefs);
                    }
                }
            }
        }
    }
    assert_eq!(permutations, 24);
}

#[test]
fn existing_settings_without_strip_preferences_remain_three_visible_on_upgrade() {
    use ai_token_meter_windows::persistence::AppSettings;
    let mut json = serde_json::to_value(AppSettings::default()).unwrap();
    json.as_object_mut().unwrap().remove("stripPreferences");
    let restored: AppSettings = serde_json::from_value(json).unwrap();
    assert_eq!(
        restored.strip_preferences.visible_providers(),
        vec!["claude", "codex", "deepseek"]
    );
}
