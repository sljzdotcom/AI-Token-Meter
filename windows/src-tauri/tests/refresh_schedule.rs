use std::sync::mpsc;
use std::time::{Duration, Instant};

use ai_token_meter_windows::collectors::refresh_schedule::{RefreshWake, wait_for_refresh};

#[test]
fn settings_change_restarts_the_refresh_countdown_immediately() {
    let (sender, receiver) = mpsc::channel();
    sender.send(()).expect("settings event");
    let started = Instant::now();

    assert_eq!(
        wait_for_refresh(&receiver, Duration::from_secs(60)),
        RefreshWake::SettingsChanged
    );
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn refresh_wait_distinguishes_due_and_shutdown() {
    let (_sender, receiver) = mpsc::channel();
    assert_eq!(
        wait_for_refresh(&receiver, Duration::from_millis(1)),
        RefreshWake::Due
    );
    let (sender, receiver) = mpsc::channel();
    drop(sender);
    assert_eq!(
        wait_for_refresh(&receiver, Duration::from_secs(60)),
        RefreshWake::Stopped
    );
}
