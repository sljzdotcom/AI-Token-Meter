use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
pub struct RefreshCoordinator {
    in_flight: Mutex<HashMap<ProviderId, Arc<CancellationToken>>>,
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
            let mut in_flight = self
                .in_flight
                .lock()
                .unwrap_or_else(|lock| lock.into_inner());
            if let Some(existing) = in_flight.get(&request.provider) {
                if priority == RefreshPriority::Scheduled {
                    return RefreshResult::AlreadyRefreshing;
                }
                existing.cancel();
            }
            in_flight.insert(request.provider, token.clone());
        }

        let outcome = (request.collector)(token.clone());
        let mut in_flight = self
            .in_flight
            .lock()
            .unwrap_or_else(|lock| lock.into_inner());
        if in_flight
            .get(&request.provider)
            .is_some_and(|current| Arc::ptr_eq(current, &token))
        {
            in_flight.remove(&request.provider);
        }
        drop(in_flight);

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
        let in_flight = self
            .in_flight
            .lock()
            .unwrap_or_else(|lock| lock.into_inner());
        for token in in_flight.values() {
            token.cancel();
        }
        in_flight.len()
    }
}
