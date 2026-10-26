use ai_token_meter_windows::persistence::AppSettings;
use ai_token_meter_windows::platform::windows::strip_preferences::StripPreferences;
use serde_json::json;

#[test]
fn code0_is_catalogued_but_hidden_on_new_defaults() {
    let value = StripPreferences::default();
    assert_eq!(
        value.ordered_providers,
        ["claude", "codex", "deepseek", "gemini", "code0"]
    );
    assert_eq!(value.hidden_providers, ["code0"]);
    assert_eq!(
        value.visible_providers(),
        ["claude", "codex", "deepseek", "gemini"]
    );
    assert_eq!(value.logical_size(false), (78.0, 344.0));
}

#[test]
fn legacy_four_provider_choices_survive_hidden_code0_migration() {
    let value: StripPreferences = serde_json::from_value(json!({
        "schemaVersion": 4,
        "density": "comfortable",
        "automaticallyCollapses": false,
        "revealDelayMilliseconds": 200,
        "collapseDelayMilliseconds": 950,
        "orderedProviders": ["gemini", "codex", "claude", "deepseek"],
        "hiddenProviders": ["claude"],
        "hiddenUntil": 123,
    }))
    .unwrap();
    assert_eq!(
        value.ordered_providers,
        ["gemini", "codex", "claude", "deepseek", "code0"]
    );
    assert_eq!(value.visible_providers(), ["gemini", "codex", "deepseek"]);
    assert!(value.hidden_providers.contains(&"code0".to_owned()));
    assert_eq!(value.density, "comfortable");
    assert!(!value.automatically_collapses);
    assert_eq!(value.reveal_delay_milliseconds, 200);
    assert_eq!(value.collapse_delay_milliseconds, 950);
    assert_eq!(value.hidden_until, Some(123));
    assert_eq!(value.schema_version, 5);
    let restored: StripPreferences =
        serde_json::from_value(serde_json::to_value(&value).unwrap()).unwrap();
    assert_eq!(restored, value);
}

#[test]
fn new_schema_explicit_code0_visibility_and_order_survive_reload() {
    let value: StripPreferences = serde_json::from_value(json!({
        "schemaVersion": 5,
        "orderedProviders": ["code0", "gemini", "codex", "claude", "deepseek"],
        "hiddenProviders": ["codex"],
    }))
    .unwrap();
    assert_eq!(
        value.ordered_providers,
        ["code0", "gemini", "codex", "claude", "deepseek"]
    );
    assert_eq!(
        value.visible_providers(),
        ["code0", "gemini", "claude", "deepseek"]
    );
    let restored: StripPreferences =
        serde_json::from_value(serde_json::to_value(&value).unwrap()).unwrap();
    assert_eq!(restored, value);
}

#[test]
fn fifth_row_has_full_height_and_original_four_dimensions_stay_unchanged() {
    let ids = ["claude", "codex", "deepseek", "gemini", "code0"];
    for (density, heights) in [
        ("compact", [170.0, 228.0, 286.0, 344.0, 402.0]),
        ("comfortable", [212.0, 284.0, 356.0, 428.0, 500.0]),
    ] {
        for count in 1..=5 {
            let mut value = StripPreferences {
                density: density.to_owned(),
                ordered_providers: ids.iter().map(|id| (*id).to_owned()).collect(),
                hidden_providers: ids[count..].iter().map(|id| (*id).to_owned()).collect(),
                ..Default::default()
            };
            value.normalize();
            assert_eq!(value.visible_providers().len(), count);
            assert_eq!(value.logical_size(false).1, heights[count - 1]);
            assert_eq!(value.logical_size(true), (20.0, 96.0));
        }
    }
}

#[test]
fn older_settings_without_strip_keep_three_visible_and_code0_hidden() {
    let mut json = serde_json::to_value(AppSettings::default()).unwrap();
    json.as_object_mut().unwrap().remove("stripPreferences");
    let restored: AppSettings = serde_json::from_value(json).unwrap();
    assert_eq!(
        restored.strip_preferences.visible_providers(),
        ["claude", "codex", "deepseek"]
    );
    assert!(
        restored
            .strip_preferences
            .hidden_providers
            .contains(&"gemini".to_owned())
    );
    assert!(
        restored
            .strip_preferences
            .hidden_providers
            .contains(&"code0".to_owned())
    );
}
