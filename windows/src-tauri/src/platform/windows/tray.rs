use tauri::image::Image;
use tauri::menu::{IconMenuItem, IconMenuItemBuilder, Menu, MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Listener, Manager, Runtime};

use crate::domain::{MetricKind, MetricUnit, ProviderId, UsageSnapshot, UsageStatus};
use crate::persistence::AppSettings;

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
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("default window icon".to_owned()))?;
    let settings = app.state::<crate::RuntimeState>().app_settings_snapshot();
    let menu = build_tray_menu(app, &settings)?;

    let tray = TrayIconBuilder::with_id("ai-token-meter")
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
    let settings_tray = tray.clone();
    app.listen("app-settings-changed", move |event| {
        let Ok(settings) = serde_json::from_str::<crate::persistence::AppSettings>(event.payload())
        else {
            return;
        };
        if let Ok(menu) = build_tray_menu(&settings_app, &settings) {
            let _ = settings_tray.set_menu(Some(menu));
        }
    });
    let snapshot_app = app.clone();
    let snapshot_tray = tray.clone();
    app.listen("snapshot-updated", move |event| {
        if serde_json::from_str::<UsageSnapshot>(event.payload()).is_err() {
            return;
        }
        let settings = snapshot_app
            .state::<crate::RuntimeState>()
            .app_settings_snapshot();
        if let Ok(menu) = build_tray_menu(&snapshot_app, &settings) {
            let _ = snapshot_tray.set_menu(Some(menu));
        }
    });
    Ok(())
}

fn build_tray_menu(app: &AppHandle, settings: &AppSettings) -> tauri::Result<Menu<tauri::Wry>> {
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("default window icon".to_owned()))?;
    let brand_header = build_brand_header(app, icon)?;
    let snapshots = app.state::<crate::RuntimeState>().usage.snapshots();
    let summary_items = summary_provider_ids(&settings.strip_preferences)
        .into_iter()
        .map(|provider| {
            let text = snapshots
                .iter()
                .find(|snapshot| snapshot.provider_id == provider)
                .map(|snapshot| format_summary_localized(snapshot, settings.locale))
                .unwrap_or_else(|| unavailable_summary(provider, settings.locale));
            MenuItemBuilder::with_id(format!("{}-summary", provider.as_str()), text)
                .enabled(false)
                .build(app)
        })
        .collect::<tauri::Result<Vec<_>>>()?;
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
    let summary_refs = summary_items
        .iter()
        .map(|item| item as &dyn tauri::menu::IsMenuItem<tauri::Wry>)
        .collect::<Vec<_>>();

    MenuBuilder::new(app)
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
        .build()
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
