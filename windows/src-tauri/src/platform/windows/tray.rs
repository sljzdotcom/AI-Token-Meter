use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Listener, Manager};

use crate::domain::{MetricKind, MetricUnit, ProviderId, UsageSnapshot, UsageStatus};

use super::window_controller::{show_settings_window, toggle_meter_window};

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let locale = app
        .state::<crate::RuntimeState>()
        .app_settings_snapshot()
        .locale;
    let claude_summary = MenuItemBuilder::with_id("claude-summary", "Claude Code · Unavailable")
        .enabled(false)
        .build(app)?;
    let codex_summary = MenuItemBuilder::with_id("codex-summary", "OpenAI Codex · Unavailable")
        .enabled(false)
        .build(app)?;
    let deepseek_summary = MenuItemBuilder::with_id("deepseek-summary", "DeepSeek · Unavailable")
        .enabled(false)
        .build(app)?;
    let gemini_summary = MenuItemBuilder::with_id("gemini-summary", "Gemini · Unavailable")
        .enabled(false)
        .build(app)?;
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
            MenuItemBuilder::with_id(*id, crate::localization::text(locale, key)).build(app)
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    let menu = MenuBuilder::new(app)
        .items(&[
            &claude_summary,
            &codex_summary,
            &deepseek_summary,
            &gemini_summary,
        ])
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

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("default window icon".to_owned()))?;
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
                let _ = crate::set_strip_preferences(app.clone(), app.state(), value);
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

    let summaries = [
        claude_summary.clone(),
        codex_summary.clone(),
        deepseek_summary.clone(),
        gemini_summary.clone(),
    ];
    let update_app = app.clone();
    let update_summaries = move |locale| {
        for snapshot in update_app.state::<crate::RuntimeState>().usage.snapshots() {
            let index = match snapshot.provider_id {
                ProviderId::Claude => 0,
                ProviderId::Codex => 1,
                ProviderId::DeepSeek => 2,
                ProviderId::Gemini => 3,
            };
            let _ = summaries[index].set_text(format_summary_localized(&snapshot, locale));
        }
    };
    update_summaries(locale);
    app.listen("app-settings-changed", move |event| {
        let Ok(settings) = serde_json::from_str::<crate::persistence::AppSettings>(event.payload())
        else {
            return;
        };
        for (item, (_, key)) in actions.iter().zip(labels) {
            let _ = item.set_text(crate::localization::text(settings.locale, key));
        }
        update_summaries(settings.locale);
    });
    let update_app = app.clone();
    app.listen("snapshot-updated", move |event| {
        let Ok(snapshot) = serde_json::from_str::<UsageSnapshot>(event.payload()) else {
            return;
        };
        let text = format_summary_localized(
            &snapshot,
            update_app
                .state::<crate::RuntimeState>()
                .app_settings_snapshot()
                .locale,
        );
        let _ = match snapshot.provider_id {
            ProviderId::Claude => claude_summary.set_text(text),
            ProviderId::Codex => codex_summary.set_text(text),
            ProviderId::DeepSeek => deepseek_summary.set_text(text),
            ProviderId::Gemini => gemini_summary.set_text(text),
        };
    });
    Ok(())
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
