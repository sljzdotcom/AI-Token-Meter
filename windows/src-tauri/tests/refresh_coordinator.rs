use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use ai_token_meter_windows::collectors::CollectionError;
use ai_token_meter_windows::collectors::refresh::{
    DEFAULT_REFRESH_INTERVAL, ProviderRefreshRequest, RefreshCoordinator, RefreshPriority,
    RefreshResult,
};
use ai_token_meter_windows::domain::{ProviderId, UsageSnapshot};

#[test]
fn three_providers_refresh_concurrently_and_failures_are_isolated() {
    let coordinator = RefreshCoordinator::new();
    let gate = Arc::new(Barrier::new(3));
    let started = Instant::now();
    let requests = [ProviderId::Claude, ProviderId::Codex, ProviderId::DeepSeek]
        .into_iter()
        .map(|provider| {
            let gate = gate.clone();
            ProviderRefreshRequest::new(provider, move |_| {
                gate.wait();
                thread::sleep(Duration::from_millis(80));
                if provider == ProviderId::Codex {
                    Err(CollectionError::Transport)
                } else {
                    Ok(fixture(provider))
                }
            })
        })
        .collect();

    let results = coordinator.refresh_all(requests, RefreshPriority::Scheduled);

    assert!(started.elapsed() < Duration::from_millis(220));
    assert!(matches!(results[0].1, RefreshResult::Snapshot(_)));
    assert_eq!(
        results[1].1,
        RefreshResult::Failed(CollectionError::Transport)
    );
    assert!(matches!(results[2].1, RefreshResult::Snapshot(_)));
    assert_eq!(DEFAULT_REFRESH_INTERVAL, Duration::from_secs(300));
}

#[test]
fn scheduled_duplicates_are_deduplicated() {
    let coordinator = Arc::new(RefreshCoordinator::new());
    let (started_sender, started_receiver) = mpsc::channel();
    let release = Arc::new(AtomicBool::new(false));
    let first = {
        let coordinator = coordinator.clone();
        let release = release.clone();
        thread::spawn(move || {
            coordinator.refresh(
                ProviderRefreshRequest::new(ProviderId::Claude, move |_| {
                    started_sender.send(()).expect("started");
                    while !release.load(Ordering::Acquire) {
                        thread::yield_now();
                    }
                    Ok(fixture(ProviderId::Claude))
                }),
                RefreshPriority::Scheduled,
            )
        })
    };
    started_receiver.recv().expect("first refresh started");

    let duplicate = coordinator.refresh(
        ProviderRefreshRequest::new(ProviderId::Claude, |_| Ok(fixture(ProviderId::Claude))),
        RefreshPriority::Scheduled,
    );
    assert_eq!(duplicate, RefreshResult::AlreadyRefreshing);

    release.store(true, Ordering::Release);
    assert!(matches!(
        first.join().expect("first join"),
        RefreshResult::Snapshot(_)
    ));
}

#[test]
fn a_deduplicated_request_never_runs_its_start_lifecycle() {
    let coordinator = Arc::new(RefreshCoordinator::new());
    let (started_sender, started_receiver) = mpsc::channel();
    let release = Arc::new(AtomicBool::new(false));
    let first = {
        let coordinator = coordinator.clone();
        let release = release.clone();
        thread::spawn(move || {
            coordinator.refresh(
                ProviderRefreshRequest::new(ProviderId::Claude, move |_| {
                    started_sender.send(()).expect("started");
                    while !release.load(Ordering::Acquire) {
                        thread::yield_now();
                    }
                    Ok(fixture(ProviderId::Claude))
                }),
                RefreshPriority::Scheduled,
            )
        })
    };
    started_receiver.recv().expect("first refresh started");
    let lifecycle_ran = Arc::new(AtomicBool::new(false));
    let lifecycle_for_request = Arc::clone(&lifecycle_ran);

    let duplicate = coordinator.refresh(
        ProviderRefreshRequest::new(ProviderId::Claude, |_| Ok(fixture(ProviderId::Claude)))
            .on_start(move || lifecycle_for_request.store(true, Ordering::Release)),
        RefreshPriority::Scheduled,
    );

    assert_eq!(duplicate, RefreshResult::AlreadyRefreshing);
    assert!(!lifecycle_ran.load(Ordering::Acquire));
    release.store(true, Ordering::Release);
    assert!(matches!(
        first.join().expect("first join"),
        RefreshResult::Snapshot(_)
    ));
}

#[test]
fn manual_refresh_cancels_the_previous_provider_task() {
    let coordinator = Arc::new(RefreshCoordinator::new());
    let (started_sender, started_receiver) = mpsc::channel();
    let old = {
        let coordinator = coordinator.clone();
        thread::spawn(move || {
            coordinator.refresh(
                ProviderRefreshRequest::new(ProviderId::Codex, move |cancellation| {
                    started_sender.send(()).expect("started");
                    while !cancellation.is_cancelled() {
                        thread::yield_now();
                    }
                    Err(CollectionError::Cancelled)
                }),
                RefreshPriority::Scheduled,
            )
        })
    };
    started_receiver.recv().expect("old refresh started");

    let replacement = coordinator.refresh(
        ProviderRefreshRequest::new(ProviderId::Codex, |_| Ok(fixture(ProviderId::Codex))),
        RefreshPriority::Manual,
    );

    assert!(matches!(replacement, RefreshResult::Snapshot(_)));
    assert_eq!(old.join().expect("old join"), RefreshResult::Cancelled);
}

#[test]
fn a_collector_that_finishes_after_cancellation_cannot_publish_its_snapshot() {
    let coordinator = Arc::new(RefreshCoordinator::new());
    let (started_sender, started_receiver) = mpsc::channel();
    let release = Arc::new(AtomicBool::new(false));
    let old = {
        let coordinator = coordinator.clone();
        let release = release.clone();
        thread::spawn(move || {
            coordinator.refresh(
                ProviderRefreshRequest::new(ProviderId::Codex, move |_| {
                    started_sender.send(()).expect("started");
                    while !release.load(Ordering::Acquire) {
                        thread::yield_now();
                    }
                    Ok(fixture(ProviderId::Codex))
                }),
                RefreshPriority::Scheduled,
            )
        })
    };
    started_receiver.recv().expect("old refresh started");

    let replacement = coordinator.refresh(
        ProviderRefreshRequest::new(ProviderId::Codex, |_| Ok(fixture(ProviderId::Codex))),
        RefreshPriority::Manual,
    );
    assert!(matches!(replacement, RefreshResult::Snapshot(_)));
    release.store(true, Ordering::Release);
    assert_eq!(old.join().expect("old join"), RefreshResult::Cancelled);
}

#[test]
fn shutdown_cancels_every_in_flight_provider_before_update_installation() {
    let coordinator = Arc::new(RefreshCoordinator::new());
    let gate = Arc::new(Barrier::new(4));
    let handles = [ProviderId::Claude, ProviderId::Codex, ProviderId::DeepSeek]
        .into_iter()
        .map(|provider| {
            let coordinator = Arc::clone(&coordinator);
            let gate = Arc::clone(&gate);
            thread::spawn(move || {
                coordinator.refresh(
                    ProviderRefreshRequest::new(provider, move |cancellation| {
                        gate.wait();
                        while !cancellation.is_cancelled() {
                            thread::yield_now();
                        }
                        Err(CollectionError::Cancelled)
                    }),
                    RefreshPriority::Scheduled,
                )
            })
        })
        .collect::<Vec<_>>();
    gate.wait();

    assert_eq!(coordinator.cancel_all(), 3);
    for handle in handles {
        assert_eq!(
            handle.join().expect("collector join"),
            RefreshResult::Cancelled
        );
    }
    assert_eq!(coordinator.cancel_all(), 0);
}

fn fixture(provider: ProviderId) -> UsageSnapshot {
    let value = match provider {
        ProviderId::Claude => include_str!("../../../contracts/fixtures/claude-fresh.json"),
        ProviderId::Codex => include_str!("../../../contracts/fixtures/codex-reset-credit.json"),
        ProviderId::DeepSeek => include_str!("../../../contracts/fixtures/deepseek-balance.json"),
    };
    UsageSnapshot::decode_compatible(&serde_json::from_str(value).expect("fixture JSON"))
        .expect("usage fixture")
}
