use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Listener, Manager};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::RuntimeState;
use crate::accounts::cli_account::CliProvider;
use crate::accounts::cli_discovery::CliDiscovery;
use crate::accounts::runtime_discovery::discover_runtime_cli;
use crate::domain::{ProviderId, UsageSnapshot, UsageStatus};
use crate::persistence::{AppSettings, ProviderCliSettings};
use crate::platform::windows::credential_manager::WindowsCredentialManager;
use crate::platform::windows::executable_locator::{ExecutableCandidate, RuntimeSource};
use crate::platform::windows::process::{BoundedProcessRunner, CancellationToken, ProcessRequest};
use crate::platform::windows::wsl::wsl_profile_path;

use super::activity_timeout::collect_optional_with_timeout;
use super::claude::collect_usage_from_candidate_with_cancellation as collect_claude;
use super::claude_activity::read_claude_activity_with_cancellation;
use super::codex::collect_usage_from_candidate_with_cancellation as collect_codex;
use super::codex_activity::read_codex_activity_with_shared_cancellation;
use super::deepseek::{DeepSeekBalanceClient, DeepSeekCollector};
use super::refresh::{ProviderRefreshRequest, RefreshPriority, RefreshResult};
use super::refresh_schedule::{RefreshWake, wait_for_refresh};
use super::{CollectionError, candidate_for_collection};

pub fn start(app: &AppHandle) {
    let manual_app = app.clone();
    app.listen("refresh-requested", move |_| {
        trigger(&manual_app, RefreshPriority::Manual);
    });

    trigger(app, RefreshPriority::Scheduled);

    let (schedule_sender, schedule_receiver) = std::sync::mpsc::channel();
    app.listen("app-settings-changed", move |_| {
        let _ = schedule_sender.send(());
    });
    let scheduled_app = app.clone();
    std::thread::spawn(move || {
        loop {
            let seconds = scheduled_app
                .state::<RuntimeState>()
                .refresh_interval_seconds();
            match wait_for_refresh(&schedule_receiver, Duration::from_secs(seconds.max(30))) {
                RefreshWake::Due => trigger(&scheduled_app, RefreshPriority::Scheduled),
                RefreshWake::SettingsChanged => continue,
                RefreshWake::Stopped => break,
            }
        }
    });
}

pub fn trigger(app: &AppHandle, priority: RefreshPriority) {
    let state = app.state::<RuntimeState>();
    let runtime = Arc::clone(&state.usage);
    let coordinator = Arc::clone(&state.refresh_coordinator);
    let settings = state.app_settings_snapshot();
    let app = app.clone();

    std::thread::spawn(move || {
        let generations = Arc::new(Mutex::new(HashMap::new()));
        let requests = build_requests(&runtime, &settings)
            .into_iter()
            .map(|request| {
                let provider = request.provider();
                let runtime = Arc::clone(&runtime);
                let app = app.clone();
                let generations = Arc::clone(&generations);
                request.on_start(move || {
                    let generation = runtime.begin_refresh(provider);
                    generations
                        .lock()
                        .unwrap_or_else(|lock| lock.into_inner())
                        .insert(provider, generation);
                    let _ = app.emit("snapshot-updated", runtime.snapshot(provider));
                })
            })
            .collect();
        let results = coordinator.refresh_all(requests, priority);
        for (provider, result) in results {
            if let RefreshResult::Deferred(error) = result {
                if runtime.restore_deferred(provider, error, &now_rfc3339()) {
                    let _ = app.emit("snapshot-updated", runtime.snapshot(provider));
                }
                continue;
            }
            let Some(generation) = generations
                .lock()
                .unwrap_or_else(|lock| lock.into_inner())
                .get(&provider)
                .copied()
            else {
                continue;
            };
            let applied = match result {
                RefreshResult::Snapshot(snapshot) => {
                    runtime.complete_success(provider, generation, *snapshot)
                }
                RefreshResult::Failed(error) => {
                    runtime.complete_failure(provider, generation, error, &now_rfc3339())
                }
                RefreshResult::Cancelled => runtime.complete_failure(
                    provider,
                    generation,
                    CollectionError::Cancelled,
                    &now_rfc3339(),
                ),
                RefreshResult::AlreadyRefreshing | RefreshResult::Deferred(_) => false,
            };
            if applied {
                let snapshot = runtime.snapshot(provider);
                show_threshold_notification(&app, &snapshot);
                let _ = app.emit("snapshot-updated", snapshot);
            }
        }
    });
}

fn show_threshold_notification(app: &AppHandle, snapshot: &UsageSnapshot) {
    use tauri_plugin_notification::NotificationExt;

    let state = app.state::<RuntimeState>();
    let Some((level, metric)) = state.threshold_notice(snapshot) else {
        return;
    };
    let (title, body) = crate::localization::threshold_notice(
        state.app_settings_snapshot().locale,
        &snapshot.display_name,
        &metric,
        level,
    );
    let _ = app.notification().builder().title(title).body(body).show();
}

fn build_requests(
    runtime: &Arc<crate::persistence::UsageRuntime>,
    settings: &AppSettings,
) -> Vec<ProviderRefreshRequest> {
    let working_directory = user_profile().unwrap_or_else(std::env::temp_dir);
    let claude_settings = settings.claude_cli.clone();
    let codex_settings = settings.codex_cli.clone();
    let credentials = Arc::new(WindowsCredentialManager::new());
    let deepseek_client = DeepSeekBalanceClient::new().ok();
    let deepseek_balance_baseline = settings.deepseek_balance_baseline_cents as f64 / 100.0;

    let claude_request = ProviderRefreshRequest::new(ProviderId::Claude, move |cancellation| {
        let Some(candidate) = candidate_for_collection(discover_runtime_cli(
            CliProvider::Claude,
            &claude_settings,
            Arc::clone(&cancellation),
        ))?
        else {
            return Ok(status_snapshot(
                ProviderId::Claude,
                UsageStatus::NotInstalled,
                &now_rfc3339(),
            ));
        };
        if cancellation.is_cancelled() {
            return Err(CollectionError::Cancelled);
        }
        let fetched_at = now_rfc3339();
        let mut snapshot = collect_claude(
            &candidate,
            &working_directory,
            &fetched_at,
            Arc::clone(&cancellation),
        )?;
        attach_claude_activity(&mut snapshot, &candidate, &cancellation)?;
        if cancellation.is_cancelled() {
            Err(CollectionError::Cancelled)
        } else {
            Ok(snapshot)
        }
    });

    let codex_request = ProviderRefreshRequest::new(ProviderId::Codex, move |cancellation| {
        let Some(candidate) = candidate_for_collection(discover_runtime_cli(
            CliProvider::Codex,
            &codex_settings,
            Arc::clone(&cancellation),
        ))?
        else {
            return Ok(status_snapshot(
                ProviderId::Codex,
                UsageStatus::NotInstalled,
                &now_rfc3339(),
            ));
        };
        if cancellation.is_cancelled() {
            return Err(CollectionError::Cancelled);
        }
        let fetched_at = now_rfc3339();
        let mut snapshot = collect_codex(
            &candidate,
            user_profile().as_deref(),
            &fetched_at,
            Arc::clone(&cancellation),
        )?;
        attach_codex_activity(&mut snapshot, &candidate, &cancellation)?;
        if cancellation.is_cancelled() {
            Err(CollectionError::Cancelled)
        } else {
            Ok(snapshot)
        }
    });

    let cached = runtime.snapshot(ProviderId::DeepSeek);
    let deepseek_request = ProviderRefreshRequest::new(ProviderId::DeepSeek, move |cancellation| {
        let Some(client) = deepseek_client.clone() else {
            return Err(CollectionError::Transport);
        };
        if cancellation.is_cancelled() {
            return Err(CollectionError::Cancelled);
        }
        let collector =
            DeepSeekCollector::new(Arc::clone(&credentials), client, deepseek_balance_baseline);
        tauri::async_runtime::block_on(collector.collect_with_cancellation(
            Some(&cached),
            &now_rfc3339(),
            cancellation,
        ))
    });

    vec![claude_request, codex_request, deepseek_request]
}

pub(crate) fn validate_custom_path(
    provider: CliProvider,
    path: &str,
) -> Result<String, &'static str> {
    let result = discover_runtime_cli(
        provider,
        &ProviderCliSettings {
            custom_path: Some(path.trim().to_owned()),
            ..Default::default()
        },
        Arc::new(CancellationToken::new()),
    );
    let CliDiscovery::Found(candidate) = result else {
        return Err("The custom CLI path is not a working provider executable");
    };
    Ok(candidate.executable.to_string_lossy().into_owned())
}

fn attach_claude_activity(
    snapshot: &mut UsageSnapshot,
    candidate: &ExecutableCandidate,
    cancellation: &Arc<CancellationToken>,
) -> Result<(), CollectionError> {
    let (start, end) = activity_window();
    let candidate = candidate.clone();
    match collect_optional_with_timeout(
        Arc::clone(cancellation),
        Duration::from_secs(2),
        move |activity_cancellation| {
            let root = activity_profile(&candidate, Arc::clone(&activity_cancellation))?
                .ok_or(super::ActivityError::Unavailable)?;
            read_claude_activity_with_cancellation(
                &root.join(".claude").join("projects"),
                start,
                end,
                &activity_cancellation,
            )
        },
    ) {
        Ok(activity) => snapshot.local_activity = activity,
        Err(super::ActivityError::Cancelled) => return Err(CollectionError::Cancelled),
        Err(_) => snapshot.local_activity = None,
    }
    Ok(())
}

fn attach_codex_activity(
    snapshot: &mut UsageSnapshot,
    candidate: &ExecutableCandidate,
    cancellation: &Arc<CancellationToken>,
) -> Result<(), CollectionError> {
    let (start, end) = activity_window();
    let candidate = candidate.clone();
    match collect_optional_with_timeout(
        Arc::clone(cancellation),
        Duration::from_secs(2),
        move |activity_cancellation| {
            let root = activity_profile(&candidate, Arc::clone(&activity_cancellation))?
                .ok_or(super::ActivityError::Unavailable)?;
            read_codex_activity_with_shared_cancellation(
                &root.join(".codex").join("state_5.sqlite"),
                start,
                end,
                activity_cancellation,
            )
        },
    ) {
        Ok(activity) => snapshot.local_activity = activity,
        Err(super::ActivityError::Cancelled) => return Err(CollectionError::Cancelled),
        Err(_) => snapshot.local_activity = None,
    }
    Ok(())
}

fn activity_profile(
    candidate: &ExecutableCandidate,
    cancellation: Arc<CancellationToken>,
) -> Result<Option<PathBuf>, super::ActivityError> {
    match &candidate.source {
        RuntimeSource::NativeWindows => Ok(user_profile()),
        RuntimeSource::Wsl { distribution } => {
            let mut request = ProcessRequest::new(
                candidate.executable.clone(),
                ["--distribution", distribution, "--exec", "printenv", "HOME"]
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            );
            request.timeout = Duration::from_secs(1);
            request.max_output_bytes = 4 * 1024;
            request.cancellation = cancellation;
            let output = BoundedProcessRunner
                .run(request)
                .map_err(|error| match error.kind() {
                    crate::platform::windows::process::ProcessErrorKind::Cancelled => {
                        super::ActivityError::Cancelled
                    }
                    _ => super::ActivityError::Unavailable,
                })?;
            if output.exit_code != Some(0) {
                return Ok(None);
            }
            Ok(wsl_profile_path(distribution, &output.stdout))
        }
    }
}

fn activity_window() -> (i64, i64) {
    let end = OffsetDateTime::now_utc().unix_timestamp();
    (end.saturating_sub(30 * 86_400), end)
}

fn user_profile() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(PathBuf::from)
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

fn status_snapshot(provider: ProviderId, status: UsageStatus, fetched_at: &str) -> UsageSnapshot {
    UsageSnapshot {
        schema_version: 1,
        provider_id: provider,
        display_name: match provider {
            ProviderId::Claude => "Claude Code",
            ProviderId::Codex => "OpenAI Codex",
            ProviderId::DeepSeek => "DeepSeek",
        }
        .to_owned(),
        status,
        used_ratio: None,
        primary_metric: None,
        secondary_metric: None,
        fetched_at: fetched_at.to_owned(),
        stale_after_seconds: 300,
        source_version: None,
        status_message: None,
        reset_credits: Vec::new(),
        local_activity: None,
        daily_history: Vec::new(),
        history_fetched_at: None,
    }
}
