use tauri::image::Image;
use tauri::menu::{
    IconMenuItem, IconMenuItemBuilder, IsMenuItem, Menu, MenuBuilder, MenuItem, MenuItemBuilder,
};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Listener, Manager, Runtime};

use crate::domain::{MetricKind, MetricUnit, ProviderId, UsageSnapshot, UsageStatus};

use super::strip_preferences::StripPreferences;
use super::window_controller::{show_settings_window, toggle_meter_window};

pub struct BrandHeaderDefinition<'a> {
    pub id: &'static str,
    pub text: &'static str,
    pub enabled: bool,
    pub icon: Image<'a>,
}

pub fn brand_header_definition<'a>(icon: Image<'a>) -> tauri::Result<BrandHeaderDefinition<'a>> {
    let expected_len = (icon.width() as usize)
        .checked_mul(icon.height() as usize)
        .and_then(|pixels| pixels.checked_mul(4));
    if expected_len != Some(icon.rgba().len()) {
        return Err(tauri::Error::InvalidIcon(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "brand header icon dimensions do not match its RGBA data",
        )));
    }

    Ok(BrandHeaderDefinition {
        id: "brand-header",
        text: "AI Token Meter",
        enabled: false,
        icon,
    })
}

pub fn build_brand_header<R: Runtime>(
    app: &AppHandle<R>,
    icon: Image<'_>,
) -> tauri::Result<IconMenuItem<R>> {
    let definition = brand_header_definition(icon)?;
    IconMenuItemBuilder::with_id(definition.id, definition.text)
        .enabled(definition.enabled)
        .icon(definition.icon)
        .build(app)
}

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let settings = app.state::<crate::RuntimeState>().app_settings_snapshot();
    let summaries = [
        MenuItemBuilder::with_id(
            "claude-summary",
            unavailable_summary(ProviderId::Claude, settings.locale),
        )
        .enabled(false)
        .build(app)?,
        MenuItemBuilder::with_id(
            "codex-summary",
            unavailable_summary(ProviderId::Codex, settings.locale),
        )
        .enabled(false)
        .build(app)?,
        MenuItemBuilder::with_id(
            "deepseek-summary",
            unavailable_summary(ProviderId::DeepSeek, settings.locale),
        )
        .enabled(false)
        .build(app)?,
        MenuItemBuilder::with_id(
            "gemini-summary",
            unavailable_summary(ProviderId::Gemini, settings.locale),
        )
        .enabled(false)
        .build(app)?,
    ];
    update_summary_texts(
        &summaries,
        &app.state::<crate::RuntimeState>().usage.snapshots(),
        settings.locale,
    );
    let labels = [
        ("refresh", "Refresh"),
        ("settings", "Settings"),
        ("toggle-meter", "Show / Hide Meter"),
        ("show-meter-now", "Show Floating Strip Now"),
        ("about", "About AI Token Meter"),
        ("quit", "Quit AI Token Meter"),
    ];
    let actions = labels
        .iter()
        .map(|(id, key)| {
            MenuItemBuilder::with_id(*id, crate::localization::text(settings.locale, key))
                .build(app)
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("default window icon".to_owned()))?;
    let brand_header = build_brand_header(app, icon.clone())?;
    let summary_refs = summary_provider_ids(&settings.strip_preferences)
        .into_iter()
        .map(|provider| summary_item(&summaries, provider) as &dyn IsMenuItem<tauri::Wry>)
        .collect::<Vec<_>>();
    let menu = MenuBuilder::new(app)
        .item(&brand_header)
        .separator()
        .items(&summary_refs)
        .separator()
        .items(&[
            &actions[0],
            &actions[1],
            &actions[2],
            &actions[3],
            &actions[4],
        ])
        .separator()
        .item(&actions[5])
        .build()?;

    TrayIconBuilder::with_id("ai-token-meter")
        .icon(icon)
        .tooltip("AI Token Meter")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show-meter-now" => {
                let state = app.state::<crate::RuntimeState>();
                let mut value = state.app_settings_snapshot().strip_preferences;
                value.hidden_until = None;
                let owned_app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = owned_app.state::<crate::RuntimeState>();
                    let _ = crate::set_strip_preferences(owned_app.clone(), state, value).await;
                });
                state
                    .meter_enabled
                    .store(true, std::sync::atomic::Ordering::Release);
                let _ = super::strip_runtime::restore(app);
            }
            "refresh" => {
                let _ = app.emit("refresh-requested", ());
            }
            "settings" => {
                let _ = show_settings_window(app);
            }
            "toggle-meter" => {
                let state = app.state::<crate::RuntimeState>();
                let (edge, normalized_y, monitor_id) = state.meter_position();
                if let Ok(visible) =
                    toggle_meter_window(app, edge, normalized_y, monitor_id.as_deref())
                {
                    state
                        .meter_enabled
                        .store(visible, std::sync::atomic::Ordering::Release);
                }
            }
            "about" => {
                let _ = show_settings_window(app);
                let _ = app.emit("settings-tab-requested", "About");
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    let settings_app = app.clone();
    let settings_menu = menu.clone();
    let settings_summaries = summaries.clone();
    app.listen("app-settings-changed", move |event| {
        let Ok(settings) = serde_json::from_str::<crate::persistence::AppSettings>(event.payload())
        else {
            return;
        };
        for (item, (_, key)) in actions.iter().zip(labels) {
            let _ = item.set_text(crate::localization::text(settings.locale, key));
        }
        update_summary_texts(
            &settings_summaries,
            &settings_app
                .state::<crate::RuntimeState>()
                .usage
                .snapshots(),
            settings.locale,
        );
        let _ = reconcile_summary_items(
            &settings_menu,
            &settings_summaries,
            &settings.strip_preferences,
        );
    });
    let snapshot_app = app.clone();
    let snapshot_summaries = summaries.clone();
    app.listen("snapshot-updated", move |event| {
        let Ok(snapshot) = serde_json::from_str::<UsageSnapshot>(event.payload()) else {
            return;
        };
        let locale = snapshot_app
            .state::<crate::RuntimeState>()
            .app_settings_snapshot()
            .locale;
        let _ = summary_item(&snapshot_summaries, snapshot.provider_id)
            .set_text(format_summary_localized(&snapshot, locale));
    });
    Ok(())
}

fn summary_item<R: Runtime>(summaries: &[MenuItem<R>; 4], provider: ProviderId) -> &MenuItem<R> {
    &summaries[match provider {
        ProviderId::Claude => 0,
        ProviderId::Codex => 1,
        ProviderId::DeepSeek => 2,
        ProviderId::Gemini => 3,
    }]
}

fn update_summary_texts<R: Runtime>(
    summaries: &[MenuItem<R>; 4],
    snapshots: &[UsageSnapshot],
    locale: crate::persistence::Locale,
) {
    for provider in [
        ProviderId::Claude,
        ProviderId::Codex,
        ProviderId::DeepSeek,
        ProviderId::Gemini,
    ] {
        let text = snapshots
            .iter()
            .find(|snapshot| snapshot.provider_id == provider)
            .map(|snapshot| format_summary_localized(snapshot, locale))
            .unwrap_or_else(|| unavailable_summary(provider, locale));
        let _ = summary_item(summaries, provider).set_text(text);
    }
}

fn reconcile_summary_items<R: Runtime>(
    menu: &Menu<R>,
    summaries: &[MenuItem<R>; 4],
    preferences: &StripPreferences,
) -> tauri::Result<()> {
    for item in summaries {
        if menu.get(item.id().as_ref()).is_some() {
            menu.remove(item)?;
        }
    }
    for (offset, provider) in summary_provider_ids(preferences).into_iter().enumerate() {
        menu.insert(summary_item(summaries, provider), 2 + offset)?;
    }
    Ok(())
}

fn unavailable_summary(provider: ProviderId, locale: crate::persistence::Locale) -> String {
    let display_name = match provider {
        ProviderId::Claude => "Claude Code",
        ProviderId::Codex => "OpenAI Codex",
        ProviderId::DeepSeek => "DeepSeek",
        ProviderId::Gemini => "Google Antigravity",
    };
    format!(
        "{} · {}",
        display_name,
        crate::localization::text(locale, "Unavailable")
    )
}

fn summary_provider_ids(preferences: &StripPreferences) -> Vec<ProviderId> {
    preferences
        .visible_providers()
        .into_iter()
        .filter_map(|id| match id.as_str() {
            "claude" => Some(ProviderId::Claude),
            "codex" => Some(ProviderId::Codex),
            "deepseek" => Some(ProviderId::DeepSeek),
            "gemini" => Some(ProviderId::Gemini),
            _ => None,
        })
        .collect()
}

pub fn format_summary(snapshot: &UsageSnapshot) -> String {
    format_summary_localized(snapshot, crate::persistence::Locale::English)
}

pub fn format_summary_localized(
    snapshot: &UsageSnapshot,
    locale: crate::persistence::Locale,
) -> String {
    let chinese = locale == crate::persistence::Locale::SimplifiedChinese;
    let value = if snapshot.provider_id == ProviderId::DeepSeek {
        snapshot.primary_metric.as_ref().and_then(|metric| {
            (metric.kind == MetricKind::Balance && metric.unit == MetricUnit::Cny).then(|| {
                if chinese {
                    format!("可用 ¥{:.2}", metric.current)
                } else {
                    format!("¥{:.2} available", metric.current)
                }
            })
        })
    } else {
        snapshot.used_ratio.map(|ratio| {
            if chinese {
                format!("已用 {:.0}%", ratio.get() * 100.0)
            } else {
                format!("{:.0}% used", ratio.get() * 100.0)
            }
        })
    };
    let status = match snapshot.status {
        UsageStatus::NotInstalled => "Not installed",
        UsageStatus::AuthenticationRequired => "Sign in required",
        UsageStatus::SetupRequired => "Setup required",
        UsageStatus::UnrecognizedOutput => "Needs update",
        UsageStatus::Unavailable => "Unavailable",
        UsageStatus::Fresh | UsageStatus::Cached | UsageStatus::Refreshing => {
            return format!(
                "{} · {}",
                snapshot.display_name,
                value
                    .unwrap_or_else(|| crate::localization::text(locale, "Unavailable").to_owned())
            );
        }
    };
    format!(
        "{} · {}",
        snapshot.display_name,
        crate::localization::text(locale, status)
    )
}

#[cfg(test)]
mod tests {
    use crate::domain::ProviderId;
    use crate::platform::windows::strip_preferences::StripPreferences;

    use super::summary_provider_ids;

    #[test]
    fn tray_summaries_follow_visible_provider_order() {
        let mut preferences = StripPreferences {
            ordered_providers: vec![
                "gemini".into(),
                "deepseek".into(),
                "codex".into(),
                "claude".into(),
            ],
            hidden_providers: vec!["codex".into()],
            ..StripPreferences::default()
        };
        preferences.normalize();

        assert_eq!(
            summary_provider_ids(&preferences),
            vec![ProviderId::Gemini, ProviderId::DeepSeek, ProviderId::Claude]
        );
    }

    #[test]
    fn restoring_a_provider_restores_its_summary_without_changing_order() {
        let mut preferences = StripPreferences {
            ordered_providers: vec![
                "deepseek".into(),
                "claude".into(),
                "gemini".into(),
                "codex".into(),
            ],
            hidden_providers: vec!["gemini".into()],
            ..StripPreferences::default()
        };
        preferences.normalize();
        assert_eq!(
            summary_provider_ids(&preferences),
            vec![ProviderId::DeepSeek, ProviderId::Claude, ProviderId::Codex]
        );

        preferences.hidden_providers.clear();
        assert_eq!(
            summary_provider_ids(&preferences),
            vec![
                ProviderId::DeepSeek,
                ProviderId::Claude,
                ProviderId::Gemini,
                ProviderId::Codex,
            ]
        );
    }
}
