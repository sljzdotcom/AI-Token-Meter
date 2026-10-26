use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::domain::UsageSnapshot;
use crate::platform::windows::code0_profile_lease::Code0ProfileLease;
use crate::platform::windows::code0_web_session::{Code0SnapshotStoring, Code0StoreError};

/// Typed, instance-local failure injection for recovery contract tests.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code0SnapshotFailurePoint {
    AfterCandidateReplace,
    RestorePreviousSnapshot,
}

#[doc(hidden)]
#[derive(Default)]
struct Code0SnapshotGateState {
    reached_after_replace: bool,
    released: bool,
}

#[doc(hidden)]
struct Code0SnapshotGateInner {
    state: Mutex<Code0SnapshotGateState>,
    changed: Condvar,
}

impl Default for Code0SnapshotGateInner {
    fn default() -> Self {
        Self {
            state: Mutex::new(Code0SnapshotGateState::default()),
            changed: Condvar::new(),
        }
    }
}

/// Instance-local test gate. The test must also observe the candidate and
/// pending marker on disk before releasing this synchronization point.
#[doc(hidden)]
#[derive(Clone, Default)]
pub struct Code0SnapshotTestGate {
    inner: Arc<Code0SnapshotGateInner>,
}

impl Code0SnapshotTestGate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn wait_until_after_replace(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let Ok(mut state) = self.inner.state.lock() else {
            return false;
        };
        while !state.reached_after_replace {
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            let Ok((next, result)) = self.inner.changed.wait_timeout(state, deadline - now) else {
                return false;
            };
            state = next;
            if result.timed_out() && !state.reached_after_replace {
                return false;
            }
        }
        true
    }

    pub fn release(&self) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.released = true;
            self.inner.changed.notify_all();
        }
    }

    #[allow(dead_code)] // The fixed refusal scaffold does not enter the transaction gate.
    fn pause_after_replace(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let Ok(mut state) = self.inner.state.lock() else {
            return false;
        };
        state.reached_after_replace = true;
        self.inner.changed.notify_all();
        while !state.released {
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            let Ok((next, result)) = self.inner.changed.wait_timeout(state, deadline - now) else {
                return false;
            };
            state = next;
            if result.timed_out() && !state.released {
                return false;
            }
        }
        true
    }
}

/// Independent five-field Code0 projection, separate from the ordinary cache.
///
/// The first implementation stage is a fixed refusal scaffold. In particular,
/// failing to cold-load is never reported as `Ok(None)`, and writes/removal do
/// not use the ordinary cache or reacquire authority by path.
pub struct Code0SnapshotStore {
    root: PathBuf,
    lease: Option<Code0ProfileLease>,
    _failure_points: Vec<Code0SnapshotFailurePoint>,
    _after_replace_gate: Option<Code0SnapshotTestGate>,
}

impl Code0SnapshotStore {
    pub fn new(root: impl AsRef<Path>, lease: Option<Code0ProfileLease>) -> Self {
        Self {
            root: root.as_ref().to_owned(),
            lease,
            _failure_points: Vec::new(),
            _after_replace_gate: None,
        }
    }

    /// Construct an isolated store with one-shot, typed transaction failures.
    /// This is local to the instance and never consults process-global state.
    #[doc(hidden)]
    pub fn with_failure_points(
        root: impl AsRef<Path>,
        lease: Option<Code0ProfileLease>,
        failure_points: impl IntoIterator<Item = Code0SnapshotFailurePoint>,
    ) -> Self {
        Self {
            root: root.as_ref().to_owned(),
            lease,
            _failure_points: failure_points.into_iter().collect(),
            _after_replace_gate: None,
        }
    }

    /// Configure a bounded gate that transaction code enters after installing
    /// the candidate and pending marker, before handling an injected failure.
    #[doc(hidden)]
    pub fn with_failure_gate(
        root: impl AsRef<Path>,
        lease: Option<Code0ProfileLease>,
        failure_points: impl IntoIterator<Item = Code0SnapshotFailurePoint>,
        gate: Code0SnapshotTestGate,
    ) -> Self {
        Self {
            root: root.as_ref().to_owned(),
            lease,
            _failure_points: failure_points.into_iter().collect(),
            _after_replace_gate: Some(gate),
        }
    }

    fn has_same_live_authority(&self) -> bool {
        self.lease
            .as_ref()
            .is_some_and(|lease| lease.is_bound_to(&self.root))
    }
}

impl Code0SnapshotStoring for Code0SnapshotStore {
    fn load(&self) -> Result<Option<UsageSnapshot>, Code0StoreError> {
        // RED scaffold: unreadable/unsupported state is not evidence of absence.
        Err(Code0StoreError::Unavailable)
    }

    fn save_atomic(&mut self, _snapshot: &UsageSnapshot) -> Result<(), Code0StoreError> {
        if !self.has_same_live_authority() {
            return Err(Code0StoreError::Unavailable);
        }
        Err(Code0StoreError::Unavailable)
    }

    fn remove(&mut self) -> Result<(), Code0StoreError> {
        if !self.has_same_live_authority() {
            return Err(Code0StoreError::Unavailable);
        }
        Err(Code0StoreError::Unavailable)
    }
}
