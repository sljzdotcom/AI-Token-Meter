use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Listener, Manager};

use crate::domain::{MetricKind, MetricUnit, ProviderId, UsageSnapshot, UsageStatus};

use super::window_controller::{show_settings_window, toggle_meter_window};

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let claude_summary = MenuItemBuilder::with_id("claude-summary", "Claude Code · Unavailable")
        .enabled(false)
        .build(app)?;
    let codex_summary = MenuItemBuilder::with_id("codex-summary", "OpenAI Codex · Unavailable")
        .enabled(false)
        .build(app)?;
    let deepseek_summary = MenuItemBuilder::with_id("deepseek-summary", "DeepSeek · Unavailable")
        .enabled(false)
        .build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&claude_summary, &codex_summary, &deepseek_summary])
        .separator()
        .text("refresh", "Refresh")
        .text("settings", "Settings")
        .text("toggle-meter", "Show / Hide Meter")
        .text("about", "About AI Token Meter")
        .separator()
        .text("quit", "Quit AI Token Meter")
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

    app.listen("snapshot-updated", move |event| {
        let Ok(snapshot) = serde_json::from_str::<UsageSnapshot>(event.payload()) else {
            return;
        };
        let text = format_summary(&snapshot);
        let _ = match snapshot.provider_id {
            ProviderId::Claude => claude_summary.set_text(text),
            ProviderId::Codex => codex_summary.set_text(text),
            ProviderId::DeepSeek => deepseek_summary.set_text(text),
        };
    });
    Ok(())
}

pub fn format_summary(snapshot: &UsageSnapshot) -> String {
    let value = if snapshot.provider_id == ProviderId::DeepSeek {
        snapshot.primary_metric.as_ref().and_then(|metric| {
            (metric.kind == MetricKind::Balance && metric.unit == MetricUnit::Cny)
                .then(|| format!("¥{:.2} available", metric.current))
        })
    } else {
        snapshot
            .used_ratio
            .map(|ratio| format!("{:.0}% used", ratio.get() * 100.0))
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
                value.unwrap_or_else(|| "Unavailable".to_owned())
            );
        }
    };
    format!("{} · {status}", snapshot.display_name)
}
