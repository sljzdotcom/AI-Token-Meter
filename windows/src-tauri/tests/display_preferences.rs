use ai_token_meter_windows::persistence::AppSettings;

#[test]
fn fresh_settings_default_to_english_yahei_and_primary_display() {
    let value = serde_json::to_value(AppSettings::default()).unwrap();
    assert_eq!(value["displayFont"], "Microsoft YaHei");
    assert_eq!(value["locale"], "en");
    assert_eq!(value["displays"]["mode"], "primary");
}

#[test]
fn legacy_saved_font_survives_and_unknown_locale_falls_back() {
    let mut value = serde_json::to_value(AppSettings::default()).unwrap();
    value["displayFont"] = "Antonio".into();
    value["locale"] = "invalid-language".into();
    let settings: AppSettings = serde_json::from_value(value).unwrap();
    let value = serde_json::to_value(settings).unwrap();
    assert_eq!(value["displayFont"], "Antonio");
    assert_eq!(value["locale"], "en");
}

#[test]
fn chinese_fonts_are_accepted_without_replacing_existing_choices() {
    let mut settings = AppSettings::default();
    for font in ["Microsoft YaHei", "SimHei", "KaiTi", "Antonio", "Menlo"] {
        assert!(settings.set_display_font(font).is_ok(), "{font}");
        assert_eq!(settings.display_font, font);
    }
}

#[test]
fn malformed_new_preferences_do_not_discard_legacy_font_and_position() {
    let mut value = serde_json::to_value(AppSettings::default()).unwrap();
    value["locale"] = serde_json::json!({"unexpected": true});
    value["displays"] = serde_json::json!({"mode": "future-mode"});
    value["displayFont"] = "Menlo".into();
    value["meterMonitorId"] = "offline".into();
    let mut settings: AppSettings = serde_json::from_value(value).unwrap();
    settings.normalize_display_preferences();
    assert_eq!(settings.display_font, "Menlo");
    assert_eq!(
        settings.locale,
        ai_token_meter_windows::persistence::Locale::English
    );
    assert_eq!(
        settings.displays.unwrap().selected_id.as_deref(),
        Some("offline")
    );
}
