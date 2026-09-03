use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshWake {
    Due,
    SettingsChanged,
    Stopped,
}

pub fn wait_for_refresh(receiver: &Receiver<()>, interval: Duration) -> RefreshWake {
    match receiver.recv_timeout(interval) {
        Ok(()) => RefreshWake::SettingsChanged,
        Err(RecvTimeoutError::Timeout) => RefreshWake::Due,
        Err(RecvTimeoutError::Disconnected) => RefreshWake::Stopped,
    }
}
