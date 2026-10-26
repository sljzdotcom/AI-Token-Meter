use std::path::{Path, PathBuf};

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

/// Independent five-field Code0 projection, separate from the ordinary cache.
///
/// The first implementation stage is a fixed refusal scaffold. In particular,
/// failing to cold-load is never reported as `Ok(None)`, and writes/removal do
/// not use the ordinary cache or reacquire authority by path.
pub struct Code0SnapshotStore {
    root: PathBuf,
    lease: Option<Code0ProfileLease>,
    failure_points: Vec<Code0SnapshotFailurePoint>,
}

impl Code0SnapshotStore {
    pub fn new(root: impl AsRef<Path>, lease: Option<Code0ProfileLease>) -> Self {
        Self {
            root: root.as_ref().to_owned(),
            lease,
            failure_points: Vec::new(),
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
            failure_points: failure_points.into_iter().collect(),
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
