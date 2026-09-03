use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use crate::platform::windows::process::CancellationToken;

use super::ActivityError;

const CANCELLATION_POLL_INTERVAL: Duration = Duration::from_millis(10);

pub fn collect_optional_with_timeout<T, F>(
    parent_cancellation: Arc<CancellationToken>,
    timeout: Duration,
    reader: F,
) -> Result<Option<T>, ActivityError>
where
    T: Send + 'static,
    F: FnOnce(Arc<CancellationToken>) -> Result<T, ActivityError> + Send + 'static,
{
    if parent_cancellation.is_cancelled() {
        return Err(ActivityError::Cancelled);
    }
    let child_cancellation = Arc::new(CancellationToken::new());
    let worker_cancellation = Arc::clone(&child_cancellation);
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let _ = sender.send(reader(worker_cancellation));
    });

    let deadline = Instant::now() + timeout;
    loop {
        if parent_cancellation.is_cancelled() {
            child_cancellation.cancel();
            return Err(ActivityError::Cancelled);
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            child_cancellation.cancel();
            return Ok(None);
        }
        match receiver.recv_timeout(remaining.min(CANCELLATION_POLL_INTERVAL)) {
            Ok(Ok(value)) => return Ok(Some(value)),
            Ok(Err(ActivityError::Cancelled)) if parent_cancellation.is_cancelled() => {
                return Err(ActivityError::Cancelled);
            }
            Ok(Err(_)) | Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(None),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
}
