use std::collections::HashMap;
use std::sync::Mutex;

use crate::collectors::CollectionError;
use crate::domain::{ProviderId, UsageSnapshot, UsageStatus};

use super::SnapshotCache;

pub struct UsageRuntime {
    cache: Option<SnapshotCache>,
    snapshots: Mutex<Vec<UsageSnapshot>>,
    generations: Mutex<HashMap<ProviderId, u64>>,
}

impl UsageRuntime {
    pub fn load(cache: SnapshotCache, now: &str) -> Self {
        let snapshots = providers()
            .into_iter()
            .map(|provider| {
                cache
                    .load(provider)
                    .ok()
                    .flatten()
                    .filter(|snapshot| snapshot.provider_id == provider)
                    .map(|mut snapshot| {
                        snapshot.status = UsageStatus::Cached;
                        snapshot.status_message = Some("Cached · waiting for refresh".to_owned());
                        snapshot
                    })
                    .unwrap_or_else(|| {
                        status_snapshot(provider, UsageStatus::Unavailable, now, None)
                    })
            })
            .collect();
        Self {
            cache: Some(cache),
            snapshots: Mutex::new(snapshots),
            generations: Mutex::new(HashMap::new()),
        }
    }

    pub fn unavailable(now: &str) -> Self {
        Self {
            cache: None,
            snapshots: Mutex::new(
                providers()
                    .into_iter()
                    .map(|provider| status_snapshot(provider, UsageStatus::Unavailable, now, None))
                    .collect(),
            ),
            generations: Mutex::new(HashMap::new()),
        }
    }

    pub fn snapshots(&self) -> Vec<UsageSnapshot> {
        self.snapshots
            .lock()
            .unwrap_or_else(|lock| lock.into_inner())
            .clone()
    }

    pub fn snapshot(&self, provider: ProviderId) -> UsageSnapshot {
        self.snapshots()
            .into_iter()
            .find(|snapshot| snapshot.provider_id == provider)
            .unwrap_or_else(|| {
                status_snapshot(
                    provider,
                    UsageStatus::Unavailable,
                    "1970-01-01T00:00:00Z",
                    None,
                )
            })
    }

    pub fn begin_refresh(&self, provider: ProviderId) -> u64 {
        let generation = {
            let mut generations = self
                .generations
                .lock()
                .unwrap_or_else(|lock| lock.into_inner());
            let next = generations
                .get(&provider)
                .copied()
                .unwrap_or(0)
                .wrapping_add(1);
            generations.insert(provider, next);
            next
        };
        self.update(provider, |snapshot| {
            snapshot.status = UsageStatus::Refreshing;
            snapshot.status_message = None;
        });
        generation
    }

    pub fn complete_success(
        &self,
        provider: ProviderId,
        generation: u64,
        mut snapshot: UsageSnapshot,
    ) -> bool {
        if snapshot.provider_id != provider || !self.is_current(provider, generation) {
            return false;
        }
        let previous = self.snapshot(provider);
        if snapshot.local_activity.is_none() {
            snapshot.local_activity = previous.local_activity;
        }
        if snapshot.daily_history.is_empty() {
            snapshot.daily_history = previous.daily_history;
            snapshot.history_fetched_at = previous.history_fetched_at;
        }
        self.replace(provider, snapshot.clone());
        if snapshot.status == UsageStatus::Fresh
            && let Some(cache) = &self.cache
        {
            let _ = cache.save(&snapshot);
        }
        true
    }

    pub fn replace_external(&self, snapshot: UsageSnapshot) {
        let provider = snapshot.provider_id;
        self.replace(provider, snapshot.clone());
        if let Some(cache) = &self.cache {
            let _ = cache.save(&snapshot);
        }
    }

    pub fn complete_failure(
        &self,
        provider: ProviderId,
        generation: u64,
        error: CollectionError,
        now: &str,
    ) -> bool {
        if error == CollectionError::Cancelled || !self.is_current(provider, generation) {
            return false;
        }
        let previous = self.snapshot(provider);
        let replacement = if has_visible_data(&previous) {
            let mut cached = previous;
            cached.status = UsageStatus::Cached;
            cached.status_message = Some(format!("Cached · {}", error_message(error)));
            cached
        } else {
            status_snapshot(
                provider,
                status_for_error(error),
                now,
                Some(error_message(error)),
            )
        };
        self.replace(provider, replacement);
        true
    }

    fn is_current(&self, provider: ProviderId, generation: u64) -> bool {
        self.generations
            .lock()
            .unwrap_or_else(|lock| lock.into_inner())
            .get(&provider)
            .is_some_and(|current| *current == generation)
    }

    fn update(&self, provider: ProviderId, body: impl FnOnce(&mut UsageSnapshot)) {
        let mut snapshots = self
            .snapshots
            .lock()
            .unwrap_or_else(|lock| lock.into_inner());
        if let Some(snapshot) = snapshots
            .iter_mut()
            .find(|snapshot| snapshot.provider_id == provider)
        {
            body(snapshot);
        }
    }

    fn replace(&self, provider: ProviderId, replacement: UsageSnapshot) {
        let mut snapshots = self
            .snapshots
            .lock()
            .unwrap_or_else(|lock| lock.into_inner());
        if let Some(snapshot) = snapshots
            .iter_mut()
            .find(|snapshot| snapshot.provider_id == provider)
        {
            *snapshot = replacement;
        }
    }
}

fn providers() -> [ProviderId; 3] {
    [ProviderId::Claude, ProviderId::Codex, ProviderId::DeepSeek]
}

fn has_visible_data(snapshot: &UsageSnapshot) -> bool {
    snapshot.used_ratio.is_some()
        || snapshot.primary_metric.is_some()
        || snapshot.secondary_metric.is_some()
}

fn status_for_error(error: CollectionError) -> UsageStatus {
    match error {
        CollectionError::AuthenticationRequired => UsageStatus::AuthenticationRequired,
        CollectionError::SetupRequired => UsageStatus::SetupRequired,
        CollectionError::UnrecognizedOutput | CollectionError::InvalidResponse => {
            UsageStatus::UnrecognizedOutput
        }
        CollectionError::TimedOut | CollectionError::Transport | CollectionError::Cancelled => {
            UsageStatus::Unavailable
        }
    }
}

fn error_message(error: CollectionError) -> &'static str {
    match error {
        CollectionError::AuthenticationRequired => "sign in required",
        CollectionError::SetupRequired => "setup required",
        CollectionError::InvalidResponse => "invalid provider response",
        CollectionError::UnrecognizedOutput => "provider output changed",
        CollectionError::TimedOut => "refresh timed out",
        CollectionError::Transport => "refresh unavailable",
        CollectionError::Cancelled => "refresh cancelled",
    }
}

fn status_snapshot(
    provider: ProviderId,
    status: UsageStatus,
    fetched_at: &str,
    message: Option<&str>,
) -> UsageSnapshot {
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
        status_message: message.map(str::to_owned),
        reset_credits: Vec::new(),
        local_activity: None,
        daily_history: Vec::new(),
        history_fetched_at: None,
    }
}
