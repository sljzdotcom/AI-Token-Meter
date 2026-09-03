use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Listener, Manager};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::RuntimeState;
use crate::accounts::cli_account::CliProvider;
use crate::domain::{ProviderId, UsageSnapshot, UsageStatus};
use crate::platform::windows::credential_manager::WindowsCredentialManager;
use crate::platform::windows::environment::DiscoveryInputs;
use crate::platform::windows::executable_locator::{ExecutableCandidate, ExecutableLocator};
use crate::platform::windows::process::{
    BoundedProcessRunner, ProcessRequest, command_for_candidate,
};
use crate::platform::windows::wsl::build_wsl_list_invocation;

use super::CollectionError;
use super::claude::collect_usage_from_candidate as collect_claude;
use super::claude_activity::read_claude_activity;
use super::codex::collect_usage_from_candidate as collect_codex;
use super::codex_activity::read_codex_activity;
use super::deepseek::{DeepSeekBalanceClient, DeepSeekCollector};
use super::refresh::{ProviderRefreshRequest, RefreshPriority, RefreshResult};

pub fn start(app: &AppHandle) {
    let manual_app = app.clone();
    app.listen("refresh-requested", move |_| {
        trigger(&manual_app, RefreshPriority::Manual);
    });

    trigger(app, RefreshPriority::Scheduled);

    let scheduled_app = app.clone();
    std::thread::spawn(move || {
        loop {
            let seconds = scheduled_app
                .state::<RuntimeState>()
                .refresh_interval_seconds();
            std::thread::sleep(Duration::from_secs(seconds.max(30)));
            trigger(&scheduled_app, RefreshPriority::Scheduled);
        }
    });
}

pub fn trigger(app: &AppHandle, priority: RefreshPriority) {
    let state = app.state::<RuntimeState>();
    let runtime = Arc::clone(&state.usage);
    let coordinator = Arc::clone(&state.refresh_coordinator);
    let app = app.clone();

    std::thread::spawn(move || {
        let generations = Arc::new(Mutex::new(HashMap::new()));
        let requests = build_requests(&runtime)
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
                RefreshResult::AlreadyRefreshing | RefreshResult::Cancelled => false,
            };
            if applied {
                let _ = app.emit("snapshot-updated", runtime.snapshot(provider));
            }
        }
    });
}

fn build_requests(runtime: &Arc<crate::persistence::UsageRuntime>) -> Vec<ProviderRefreshRequest> {
    let working_directory = user_profile().unwrap_or_else(std::env::temp_dir);
    let claude = locate(CliProvider::Claude);
    let codex = locate(CliProvider::Codex);
    let credentials = Arc::new(WindowsCredentialManager::new());
    let deepseek_client = DeepSeekBalanceClient::new().ok();

    let claude_request = ProviderRefreshRequest::new(ProviderId::Claude, move |cancellation| {
        let Some(candidate) = claude.as_ref() else {
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
        let mut snapshot = collect_claude(candidate, &working_directory, &fetched_at)?;
        attach_claude_activity(&mut snapshot, &candidate.source);
        if cancellation.is_cancelled() {
            Err(CollectionError::Cancelled)
        } else {
            Ok(snapshot)
        }
    });

    let codex_request = ProviderRefreshRequest::new(ProviderId::Codex, move |cancellation| {
        let Some(candidate) = codex.as_ref() else {
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
        let mut snapshot = collect_codex(candidate, user_profile().as_deref(), &fetched_at)?;
        attach_codex_activity(&mut snapshot, &candidate.source);
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
        let collector = DeepSeekCollector::new(Arc::clone(&credentials), client, 100.0);
        let snapshot =
            tauri::async_runtime::block_on(collector.collect(Some(&cached), &now_rfc3339()));
        if cancellation.is_cancelled() {
            Err(CollectionError::Cancelled)
        } else {
            Ok(snapshot)
        }
    });

    vec![claude_request, codex_request, deepseek_request]
}

pub(crate) fn locate(provider: CliProvider) -> Option<ExecutableCandidate> {
    let inputs = DiscoveryInputs::capture(None);
    let wsl_output = inputs
        .system_root
        .as_deref()
        .and_then(build_wsl_list_invocation)
        .and_then(|invocation| {
            let mut request = ProcessRequest::new(
                invocation.executable,
                invocation.arguments.into_iter().map(Into::into).collect(),
            );
            request.timeout = Duration::from_secs(3);
            request.max_output_bytes = 16 * 1024;
            BoundedProcessRunner
                .run(request)
                .ok()
                .map(|output| output.stdout)
        });
    ExecutableLocator::new(inputs).locate_with_wsl_output(
        provider,
        wsl_output.as_deref().map(str::as_bytes),
        |candidate| candidate_is_healthy(candidate, provider),
    )
}

fn candidate_is_healthy(candidate: &ExecutableCandidate, provider: CliProvider) -> bool {
    let Ok(invocation) = command_for_candidate(candidate, provider, &["--version"]) else {
        return false;
    };
    let mut request = ProcessRequest::new(invocation.executable, invocation.arguments);
    request.timeout = Duration::from_secs(4);
    request.max_output_bytes = 16 * 1024;
    BoundedProcessRunner
        .run(request)
        .is_ok_and(|output| output.exit_code == Some(0))
}

fn attach_claude_activity(
    snapshot: &mut UsageSnapshot,
    source: &crate::platform::windows::executable_locator::RuntimeSource,
) {
    if !source.may_read_windows_profile() {
        snapshot.local_activity = None;
        return;
    }
    let Some(root) = user_profile().map(|path| path.join(".claude")) else {
        return;
    };
    let (start, end) = activity_window();
    snapshot.local_activity = read_claude_activity(&root.join("projects"), start, end).ok();
}

fn attach_codex_activity(
    snapshot: &mut UsageSnapshot,
    source: &crate::platform::windows::executable_locator::RuntimeSource,
) {
    if !source.may_read_windows_profile() {
        snapshot.local_activity = None;
        return;
    }
    let Some(root) = user_profile().map(|path| path.join(".codex")) else {
        return;
    };
    let (start, end) = activity_window();
    snapshot.local_activity = read_codex_activity(&root.join("state_5.sqlite"), start, end).ok();
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
