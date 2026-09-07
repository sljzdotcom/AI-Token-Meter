use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

#[derive(Debug, Default)]
pub struct MeterDragGate {
    active_session: Mutex<u64>,
    next_session: AtomicU64,
}

impl MeterDragGate {
    pub fn try_begin(&self) -> Option<u64> {
        let session = self
            .next_session
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_add(1)
            .max(1);
        let mut active = self.active_session.lock().unwrap();
        if *active != 0 {
            return None;
        }
        *active = session;
        Some(session)
    }

    pub fn is_active(&self) -> bool {
        *self.active_session.lock().unwrap() != 0
    }

    pub fn owns(&self, session: u64) -> bool {
        session != 0 && *self.active_session.lock().unwrap() == session
    }

    pub fn finish(&self, session: u64) -> bool {
        let mut active = self.active_session.lock().unwrap();
        if *active != session {
            return false;
        }
        *active = 0;
        true
    }

    pub fn cancel(&self) {
        *self.active_session.lock().unwrap() = 0;
    }

    /// The callback may persist settings, but must never dispatch native UI work.
    /// Cancellation and final placement persistence have one atomic boundary.
    pub fn commit_if_owned<T>(&self, session: u64, commit: impl FnOnce() -> T) -> Option<T> {
        let active = self.active_session.lock().unwrap();
        if session == 0 || *active != session {
            return None;
        }
        Some(commit())
    }
}

#[cfg(windows)]
pub fn wait_for_primary_button_release() -> bool {
    use std::time::{Duration, Instant};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};

    const PRESS_GRACE: Duration = Duration::from_millis(500);
    const DRAG_TIMEOUT: Duration = Duration::from_secs(15);
    const POLL_INTERVAL: Duration = Duration::from_millis(16);
    let started = Instant::now();
    let mut observed_pressed = false;
    loop {
        let pressed = unsafe { GetAsyncKeyState(i32::from(VK_LBUTTON)) } < 0;
        observed_pressed |= pressed;
        if observed_pressed && !pressed {
            return true;
        }
        if (!observed_pressed && started.elapsed() >= PRESS_GRACE)
            || started.elapsed() >= DRAG_TIMEOUT
        {
            return false;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

#[cfg(test)]
mod tests {
    use super::MeterDragGate;

    #[test]
    fn only_one_drag_session_can_be_active() {
        let gate = MeterDragGate::default();
        let first = gate.try_begin().expect("first drag should start");

        assert!(gate.is_active());
        assert_eq!(gate.try_begin(), None);
        assert!(gate.finish(first));
        assert!(!gate.is_active());
    }

    #[test]
    fn an_old_session_cannot_finish_a_new_drag() {
        let gate = MeterDragGate::default();
        let first = gate.try_begin().expect("first drag should start");
        assert!(gate.finish(first));
        let second = gate.try_begin().expect("second drag should start");

        assert!(!gate.finish(first));
        assert!(gate.is_active());
        assert!(gate.finish(second));
        assert!(!gate.is_active());
    }

    #[test]
    fn cancellation_always_releases_the_gate() {
        let gate = MeterDragGate::default();
        let _ = gate.try_begin().expect("drag should start");

        gate.cancel();

        assert!(!gate.is_active());
        assert!(gate.try_begin().is_some());
    }

    #[test]
    fn cancellation_after_native_target_calculation_rejects_placement_commit() {
        let gate = MeterDragGate::default();
        let session = gate.try_begin().unwrap();
        assert!(gate.owns(session));
        let calculated_target = "old-target";
        gate.cancel();
        let mut saved_target = "new-mode-target";
        assert_eq!(
            gate.commit_if_owned(session, || saved_target = calculated_target),
            None
        );
        assert_eq!(saved_target, "new-mode-target");
        let current = gate.try_begin().unwrap();
        assert_eq!(
            gate.commit_if_owned(current, || saved_target = "current-target"),
            Some(())
        );
        assert_eq!(saved_target, "current-target");
    }
}
