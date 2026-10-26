use std::time::Duration;

use crate::domain::UsageSnapshot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code0SessionState {
    Disconnected,
    Connecting,
    Syncing,
    Closing,
    Active,
    Paused,
    CleanupPending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code0SessionFailure {
    ValidationRequired,
    ProfileBusy,
    StorageFailed,
    CleanupFailed,
    PolicyRejected,
    NavigationFailed,
    InvalidResponse,
    AuthenticationChanged,
    LoginTimedOut,
    SyncTimedOut,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code0SessionActionResult {
    Started,
    Busy,
    Throttled { seconds_remaining: u64 },
    Denied,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code0WindowMode {
    Login,
    Sync,
}

/// Native generates these identities for one owned window/document. They are
/// never deserialized from webpage input or accepted as account identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Code0WindowRequest {
    pub instance_id: String,
    pub connection_epoch: String,
    pub nonce: String,
    pub document_id: String,
    pub mode: Code0WindowMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Code0CleanupRequest {
    pub connection_epoch: String,
    pub cleanup_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Code0WindowEvent {
    /// Async environment/controller, refusal handlers and script registration
    /// have all succeeded. Merely accepting create_unloaded is not this proof.
    PoliciesInstalled,
    /// source_url and main-frame status come from the owned native callback.
    /// Only the filtered <=2KiB bridge envelope belongs in payload.
    Message {
        payload: Vec<u8>,
        source_url: String,
        is_main_frame: bool,
    },
    /// This owned window/controller is actually released, not merely hidden or
    /// asked to close. Revoked attempts must still be allowed to report release.
    Released,
    Failed(Code0SessionFailure),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code0StoreError {
    Unavailable,
    InvalidSnapshot,
}

/// Injected monotonic elapsed time (including suspension for the eventual native
/// clock). wall_now is a native-generated RFC3339 success time, never page time.
pub trait Code0SessionClock {
    fn now(&self) -> Duration;
    fn wall_now(&self) -> String;
}

/// Independent Code0 projection. load distinguishes absent from invalid/error;
/// writes/removal later require the same real lease held by the native driver.
pub trait Code0SnapshotStoring {
    fn load(&self) -> Result<Option<UsageSnapshot>, Code0StoreError>;
    fn save_atomic(&mut self, snapshot: &UsageSnapshot) -> Result<(), Code0StoreError>;
    fn remove(&mut self) -> Result<(), Code0StoreError>;
}

/// Commands request native work; they never synchronously reenter the session.
/// The owning dispatcher queues async acknowledgements through receive_*.
/// A failed create may retain partial native objects until Released is observed.
/// No Send bound: the future direct-COM owner remains on its UI/STA thread.
pub trait Code0WindowDriving {
    fn acquire_profile_lease(&mut self) -> Result<(), Code0SessionFailure>;
    fn release_profile_lease(&mut self);
    fn create_unloaded(&mut self, request: Code0WindowRequest) -> Result<(), Code0SessionFailure>;
    fn navigate(&mut self, instance_id: &str) -> Result<(), Code0SessionFailure>;
    fn close(&mut self, instance_id: &str);
    fn clear_website_data(
        &mut self,
        request: Code0CleanupRequest,
    ) -> Result<(), Code0SessionFailure>;
}

/// RED scaffold only. This object owns the eventual single state authority, but
/// currently grants no action, consumes no callbacks and performs no I/O.
/// Behavior admission requires root's compiled, isolated actual RED first.
pub struct Code0WebSession<C, D, S> {
    _clock: C,
    _driver: D,
    _store: S,
    _on_snapshot_change: Box<dyn FnMut(Option<UsageSnapshot>)>,
}

impl<C: Code0SessionClock, D: Code0WindowDriving, S: Code0SnapshotStoring>
    Code0WebSession<C, D, S>
{
    pub fn new(
        clock: C,
        driver: D,
        store: S,
        on_snapshot_change: Box<dyn FnMut(Option<UsageSnapshot>)>,
    ) -> Self {
        Self {
            _clock: clock,
            _driver: driver,
            _store: store,
            _on_snapshot_change: on_snapshot_change,
        }
    }

    pub fn state(&self) -> Code0SessionState {
        Code0SessionState::Disconnected
    }

    pub fn snapshot(&self) -> Option<&UsageSnapshot> {
        None
    }

    pub fn failure(&self) -> Option<Code0SessionFailure> {
        Some(Code0SessionFailure::ValidationRequired)
    }

    pub fn is_automatically_syncing(&self) -> bool {
        false
    }

    pub fn connect(&mut self) -> Code0SessionActionResult {
        Code0SessionActionResult::Denied
    }

    pub fn login_completed(&mut self) -> Code0SessionActionResult {
        Code0SessionActionResult::Denied
    }

    pub fn sync_now(&mut self) -> Code0SessionActionResult {
        Code0SessionActionResult::Denied
    }

    pub fn disconnect(&mut self) -> Code0SessionActionResult {
        Code0SessionActionResult::Denied
    }

    pub fn receive_window_event(
        &mut self,
        _request: &Code0WindowRequest,
        _event: Code0WindowEvent,
    ) {
    }

    pub fn receive_cleanup_result(
        &mut self,
        _request: &Code0CleanupRequest,
        _result: Result<(), Code0SessionFailure>,
    ) {
    }

    pub fn process_scheduled_work(&mut self) {}

    pub fn stop(&mut self) {}
}
