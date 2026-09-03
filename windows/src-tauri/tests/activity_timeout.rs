use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use ai_token_meter_windows::collectors::ActivityError;
use ai_token_meter_windows::collectors::activity_timeout::collect_optional_with_timeout;
use ai_token_meter_windows::platform::windows::process::CancellationToken;

#[test]
fn optional_activity_times_out_and_cancels_the_underlying_reader() {
    let parent = Arc::new(CancellationToken::new());
    let reader_stopped = Arc::new(AtomicBool::new(false));
    let stopped = Arc::clone(&reader_stopped);
    let started = Instant::now();

    let result =
        collect_optional_with_timeout(parent, Duration::from_millis(25), move |cancellation| {
            while !cancellation.is_cancelled() {
                std::thread::yield_now();
            }
            stopped.store(true, Ordering::Release);
            Err::<u64, _>(ActivityError::Cancelled)
        })
        .expect("timeout is optional");

    assert_eq!(result, None);
    assert!(started.elapsed() < Duration::from_secs(1));
    for _ in 0..100 {
        if reader_stopped.load(Ordering::Acquire) {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(reader_stopped.load(Ordering::Acquire));
}

#[test]
fn provider_cancellation_still_cancels_the_whole_refresh() {
    let parent = Arc::new(CancellationToken::new());
    parent.cancel();

    assert_eq!(
        collect_optional_with_timeout(parent, Duration::from_secs(1), |_| Ok::<_, ActivityError>(
            7
        )),
        Err(ActivityError::Cancelled)
    );
}
