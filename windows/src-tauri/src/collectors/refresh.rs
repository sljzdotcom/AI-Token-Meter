use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::domain::{ProviderId, UsageSnapshot};
use crate::platform::windows::process::CancellationToken;

use super::CollectionError;

pub const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(300);

type Collector = dyn Fn(Arc<CancellationToken>) -> Result<UsageSnapshot, CollectionError>
    + Send
    + Sync
    + 'static;

pub struct ProviderRefreshRequest {
    provider: ProviderId,
    collector: Box<Collector>,
}

impl ProviderRefreshRequest {
    pub fn new<F>(provider: ProviderId, collector: F) -> Self
    where
        F: Fn(Arc<CancellationToken>) -> Result<UsageSnapshot, CollectionError>
            + Send
            + Sync
            + 'static,
    {
        Self {
            provider,
            collector: Box::new(collector),
        }
    }

    pub fn provider(&self) -> ProviderId {
        self.provider
    }

    pub fn on_start<F>(self, start: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        let provider = self.provider;
        let collector = self.collector;
        Self::new(provider, move |cancellation| {
            start();
            collector(cancellation)
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshPriority {
    Scheduled,
    Manual,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RefreshResult {
    Snapshot(Box<UsageSnapshot>),
    Failed(CollectionError),
    AlreadyRefreshing,
    Cancelled,
}

#[derive(Default)]
struct RefreshState {
    in_flight: HashMap<ProviderId, Arc<CancellationToken>>,
    active_count: usize,
    suspended: bool,
}

pub struct RefreshCoordinator {
    state: Mutex<RefreshState>,
    drained: Condvar,
}

impl Default for RefreshCoordinator {
    fn default() -> Self {
        Self {
            state: Mutex::new(RefreshState {
                in_flight: HashMap::new(),
                active_count: 0,
                suspended: false,
            }),
            drained: Condvar::new(),
        }
    }
}

impl RefreshCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn refresh(
        &self,
        request: ProviderRefreshRequest,
        priority: RefreshPriority,
    ) -> RefreshResult {
        let token = Arc::new(CancellationToken::new());
        {
            let mut state = self.state.lock().unwrap_or_else(|lock| lock.into_inner());
            if state.suspended {
                return RefreshResult::Cancelled;
            }
            if let Some(existing) = state.in_flight.get(&request.provider) {
                if priority == RefreshPriority::Scheduled {
                    return RefreshResult::AlreadyRefreshing;
                }
                existing.cancel();
            }
            state.in_flight.insert(request.provider, token.clone());
            state.active_count = state.active_count.saturating_add(1);
        }

        let outcome = (request.collector)(token.clone());
        let mut state = self.state.lock().unwrap_or_else(|lock| lock.into_inner());
        if state
            .in_flight
            .get(&request.provider)
            .is_some_and(|current| Arc::ptr_eq(current, &token))
        {
            state.in_flight.remove(&request.provider);
        }
        state.active_count = state.active_count.saturating_sub(1);
        if state.active_count == 0 {
            self.drained.notify_all();
        }
        drop(state);

        if token.is_cancelled() {
            return RefreshResult::Cancelled;
        }
        match outcome {
            Ok(snapshot) => RefreshResult::Snapshot(Box::new(snapshot)),
            Err(CollectionError::Cancelled) => RefreshResult::Cancelled,
            Err(error) => RefreshResult::Failed(error),
        }
    }

    pub fn refresh_all(
        &self,
        requests: Vec<ProviderRefreshRequest>,
        priority: RefreshPriority,
    ) -> Vec<(ProviderId, RefreshResult)> {
        std::thread::scope(|scope| {
            let handles = requests
                .into_iter()
                .map(|request| {
                    let provider = request.provider;
                    (
                        provider,
                        scope.spawn(move || self.refresh(request, priority)),
                    )
                })
                .collect::<Vec<_>>();
            handles
                .into_iter()
                .map(|(provider, handle)| {
                    let result = handle
                        .join()
                        .unwrap_or(RefreshResult::Failed(CollectionError::Transport));
                    (provider, result)
                })
                .collect()
        })
    }

    pub fn cancel_all(&self) -> usize {
        let state = self.state.lock().unwrap_or_else(|lock| lock.into_inner());
        for token in state.in_flight.values() {
            token.cancel();
        }
        state.in_flight.len()
    }

    pub fn suspend_and_wait(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut state = self.state.lock().unwrap_or_else(|lock| lock.into_inner());
        state.suspended = true;
        for token in state.in_flight.values() {
            token.cancel();
        }
        while state.active_count > 0 {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return false;
            }
            let (next, result) = self
                .drained
                .wait_timeout(state, remaining)
                .unwrap_or_else(|lock| lock.into_inner());
            state = next;
            if result.timed_out() && state.active_count > 0 {
                return false;
            }
        }
        true
    }

    pub fn resume(&self) {
        let mut state = self.state.lock().unwrap_or_else(|lock| lock.into_inner());
        state.suspended = false;
    }
}
