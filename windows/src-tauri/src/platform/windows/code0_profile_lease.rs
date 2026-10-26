use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Result of attempting to own the single Code0 WebView2 profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code0LeaseError {
    Busy,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LeaseState {
    Held,
    Released,
    Invalidated,
}

struct LeaseAuthority {
    root: PathBuf,
    state: Mutex<LeaseState>,
}

/// Cloneable reference to one future OS-backed lease authority.
///
/// Clones are intended to be shared by the WebView2 driver and snapshot store;
/// they never acquire a second lease. Releasing or invalidating the authority
/// must revoke writes for every clone. The current RED scaffold intentionally
/// never constructs a held authority until the Windows file-handle behavior is
/// implemented and accepted.
#[derive(Clone)]
pub struct Code0ProfileLease {
    authority: Arc<LeaseAuthority>,
}

impl Code0ProfileLease {
    /// Acquire the fixed profile rooted under a caller supplied isolated path.
    /// The production implementation will use a no-follow exclusive Windows
    /// share handle and validate the directory/lock identities and ACL.
    pub fn acquire(_root: impl AsRef<Path>) -> Result<Self, Code0LeaseError> {
        Err(Code0LeaseError::Busy)
    }

    /// Explicit release invalidates every clone; this is not a path deletion.
    pub fn release(&self) {
        if let Ok(mut state) = self.authority.state.lock() {
            *state = LeaseState::Released;
        }
    }

    /// Permanently deny further work after an identity or handle invariant fails.
    pub fn invalidate(&self) {
        if let Ok(mut state) = self.authority.state.lock() {
            *state = LeaseState::Invalidated;
        }
    }

    /// Whether this authority still owns the native handle and expected root.
    pub fn is_held(&self) -> bool {
        self.authority
            .state
            .lock()
            .is_ok_and(|state| *state == LeaseState::Held)
    }

    /// Check that the store is bound to the same root identity as this lease.
    /// A path match is only one part of the eventual handle identity check.
    pub fn is_bound_to(&self, root: &Path) -> bool {
        self.authority.root == root && self.is_held()
    }
}
