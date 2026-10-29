use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::Duration;

use crate::collectors::code0_bridge::{Code0BridgeContext, Code0BridgeEvent, Code0BridgeValidator};
use crate::collectors::code0_capture::Code0WebsiteCapture;
use crate::domain::{MetricKind, MetricUnit, ProviderId, UsageSnapshot, UsageStatus};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

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

/// Timing facts for one Session-bound cleanup request, not a native completion
/// or ownership receipt. Mutating this owned copy cannot alter the Session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Code0CleanupBudgetSnapshot {
    pub request: Code0CleanupRequest,
    pub admitted_at: Duration,
    pub deadline: Duration,
    pub failed: bool,
}

/// UI-local read-only view of the Session's one cleanup operation. There is no
/// public constructor or write/arm method. Reads neither sample a clock nor grant
/// native authority; a future caller must also check its current monotonic time.
/// Weak ownership makes a retained reader unavailable when its Session is gone.
#[derive(Clone)]
pub struct Code0CleanupBudgetHandle {
    state: Weak<RefCell<Option<CleanupBudgetEntry>>>,
}

impl Code0CleanupBudgetHandle {
    pub fn read(&self, request: &Code0CleanupRequest) -> Option<Code0CleanupBudgetSnapshot> {
        let shared = self.state.upgrade()?;
        let state = shared.borrow();
        let entry = state.as_ref()?;
        if entry.request.as_ref() != Some(request) {
            return None;
        }
        Some(Code0CleanupBudgetSnapshot {
            request: request.clone(),
            admitted_at: entry.admitted_at,
            deadline: entry.deadline?,
            failed: entry.failed,
        })
    }
}

struct CleanupBudgetEntry {
    admitted_at: Duration,
    deadline: Option<Duration>,
    last_observed: Duration,
    request: Option<Code0CleanupRequest>,
    failed: bool,
}

/// Only the Session owns this write end. An unbound admission is deliberately
/// invisible through the native handle until a real full request is installed.
struct CleanupBudgetContext {
    state: Rc<RefCell<Option<CleanupBudgetEntry>>>,
}

impl CleanupBudgetContext {
    fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(None)),
        }
    }

    fn arm(&self, admitted_at: Duration, earlier_close: Option<Duration>) -> bool {
        let deadline = admitted_at
            .checked_add(Duration::from_secs(10))
            .map(|end| earlier_close.map_or(end, |earlier| earlier.min(end)));
        let failed = deadline.is_none_or(|end| admitted_at >= end);
        *self.state.borrow_mut() = Some(CleanupBudgetEntry {
            admitted_at,
            deadline,
            last_observed: admitted_at,
            request: None,
            failed,
        });
        !failed
    }

    fn active(&self) -> bool {
        self.state.borrow().is_some()
    }

    fn deadline(&self) -> Option<Duration> {
        self.state
            .borrow()
            .as_ref()
            .and_then(|entry| entry.deadline)
    }

    fn expired(&self, now: Duration) -> bool {
        let mut state = self.state.borrow_mut();
        let Some(entry) = state.as_mut() else {
            return false;
        };
        if now < entry.last_observed || entry.deadline.is_none_or(|end| now >= end) {
            entry.failed = true;
        }
        if now >= entry.last_observed {
            entry.last_observed = now;
        }
        entry.failed
    }

    fn bind(&self, request: &Code0CleanupRequest) -> bool {
        let mut state = self.state.borrow_mut();
        let Some(entry) = state.as_mut() else {
            return false;
        };
        if entry.failed || entry.request.is_some() || entry.deadline.is_none() {
            return false;
        }
        entry.request = Some(request.clone());
        true
    }

    fn matches(&self, request: &Code0CleanupRequest) -> bool {
        self.state
            .borrow()
            .as_ref()
            .is_some_and(|entry| entry.request.as_ref() == Some(request))
    }

    fn fail(&self) -> bool {
        let mut state = self.state.borrow_mut();
        if let Some(entry) = state.as_mut() {
            entry.failed = true;
            true
        } else {
            false
        }
    }

    fn clear(&self) {
        *self.state.borrow_mut() = None;
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClosePlan {
    BeginSync,
    Activate,
    Pause,
    Disconnect,
    Stop,
    CleanupFailure,
}

/// One pure session authority. Native operations are commands with separately
/// delivered completion events; this type never constructs an OS window/store.
pub struct Code0WebSession<C, D, S> {
    clock: C,
    driver: D,
    store: S,
    on_snapshot_change: Box<dyn FnMut(Option<UsageSnapshot>)>,
    state: Code0SessionState,
    snapshot: Option<UsageSnapshot>,
    failure: Option<Code0SessionFailure>,
    automatically_syncing: bool,
    epoch: Option<String>,
    owned: Option<Code0WindowRequest>,
    cleanup: Option<Code0CleanupRequest>,
    cleanup_budget: CleanupBudgetContext,
    // Bounded collision exclusions, not persisted identities or account data.
    last_owned: Option<Code0WindowRequest>,
    last_cleanup: Option<Code0CleanupRequest>,
    validator: Option<Code0BridgeValidator>,
    policies_installed: bool,
    close_requested: bool,
    lease_held: bool,
    has_connection: bool,
    cache_may_exist: bool,
    stopped: bool,
    last_sync_start: Option<Duration>,
    attempt_deadline: Option<Duration>,
    close_deadline: Option<Duration>,
    close_plan: Option<ClosePlan>,
}

impl<C: Code0SessionClock, D: Code0WindowDriving, S: Code0SnapshotStoring>
    Code0WebSession<C, D, S>
{
    /// Construction only reads the independent numeric cache. Publication is a
    /// signal to the owning dispatcher; consumers queue actions rather than
    /// synchronously reentering this exclusively borrowed state machine.
    pub fn new(
        clock: C,
        driver: D,
        store: S,
        on_snapshot_change: Box<dyn FnMut(Option<UsageSnapshot>)>,
    ) -> Self {
        let mut session = Self {
            clock,
            driver,
            store,
            on_snapshot_change,
            state: Code0SessionState::Disconnected,
            snapshot: None,
            failure: None,
            automatically_syncing: false,
            epoch: None,
            owned: None,
            cleanup: None,
            cleanup_budget: CleanupBudgetContext::new(),
            last_owned: None,
            last_cleanup: None,
            validator: None,
            policies_installed: false,
            close_requested: false,
            lease_held: false,
            has_connection: false,
            cache_may_exist: false,
            stopped: false,
            last_sync_start: None,
            attempt_deadline: None,
            close_deadline: None,
            close_plan: None,
        };
        match session.store.load() {
            Ok(None) => {}
            Ok(Some(cached)) => {
                session.cache_may_exist = true;
                session.state = Code0SessionState::Paused;
                if let Some(cached) = cached_projection(&cached) {
                    session.snapshot = Some(cached);
                    session.has_connection = true;
                } else {
                    session.failure = Some(Code0SessionFailure::StorageFailed);
                }
            }
            Err(_) => {
                // A failed load is not evidence that prior account data is absent.
                session.cache_may_exist = true;
                session.state = Code0SessionState::Paused;
                session.failure = Some(Code0SessionFailure::StorageFailed);
            }
        }
        session
    }

    #[cfg(test)]
    pub(crate) fn terminal_stop_consumed(&self) -> bool {
        self.stopped
            && self.owned.is_none()
            && self.cleanup.is_none()
            && matches!(
                self.state,
                Code0SessionState::Paused | Code0SessionState::Disconnected
            )
    }

    pub fn state(&self) -> Code0SessionState {
        self.state
    }

    pub fn snapshot(&self) -> Option<&UsageSnapshot> {
        self.snapshot.as_ref()
    }

    pub fn failure(&self) -> Option<Code0SessionFailure> {
        self.failure
    }

    pub fn is_automatically_syncing(&self) -> bool {
        self.automatically_syncing
    }

    /// Clone the same cold, read-only UI-local slot for the future native facade.
    pub fn cleanup_budget_handle(&self) -> Code0CleanupBudgetHandle {
        Code0CleanupBudgetHandle {
            state: Rc::downgrade(&self.cleanup_budget.state),
        }
    }

    pub fn connect(&mut self) -> Code0SessionActionResult {
        if self.stopped {
            return Code0SessionActionResult::Denied;
        }
        if self.is_busy() {
            return Code0SessionActionResult::Busy;
        }
        let epoch = match self.next_id(&[]) {
            Ok(epoch) => epoch,
            Err(reason) => {
                self.pause(reason);
                return Code0SessionActionResult::Denied;
            }
        };
        self.epoch = Some(epoch);
        self.state = Code0SessionState::Connecting;
        self.failure = None;
        self.revoke_capture();
        if !self.acquire_lease() {
            return Code0SessionActionResult::Denied;
        }
        // Explicit Connect is an account boundary; an ordinary sync retry is not.
        if self.cache_may_exist {
            self.snapshot = None;
            if self.invalidate_cache().is_err() {
                self.fail_cleanup(Code0SessionFailure::StorageFailed);
                (self.on_snapshot_change)(None);
                return Code0SessionActionResult::Denied;
            }
            (self.on_snapshot_change)(None);
        }
        self.has_connection = true;
        self.last_sync_start = None;
        if self.begin_window(Code0WindowMode::Login) {
            Code0SessionActionResult::Started
        } else {
            Code0SessionActionResult::Denied
        }
    }

    pub fn login_completed(&mut self) -> Code0SessionActionResult {
        if self.stopped {
            return Code0SessionActionResult::Denied;
        }
        if self.state != Code0SessionState::Connecting
            || self.owned.as_ref().map(|r| r.mode) != Some(Code0WindowMode::Login)
            || !self.policies_installed
        {
            return if self.is_busy() {
                Code0SessionActionResult::Busy
            } else {
                Code0SessionActionResult::Denied
            };
        }
        if self.expire_attempt(self.clock.now()) {
            return Code0SessionActionResult::Denied;
        }
        self.begin_close(ClosePlan::BeginSync);
        Code0SessionActionResult::Started
    }

    pub fn sync_now(&mut self) -> Code0SessionActionResult {
        if self.stopped {
            return Code0SessionActionResult::Denied;
        }
        if self.is_busy() {
            return Code0SessionActionResult::Busy;
        }
        if !self.has_connection
            || !matches!(
                self.state,
                Code0SessionState::Active | Code0SessionState::Paused
            )
        {
            return Code0SessionActionResult::Denied;
        }
        if let Some(seconds_remaining) = self.remaining_interval() {
            return Code0SessionActionResult::Throttled { seconds_remaining };
        }
        if let Err(reason) = self.ensure_epoch() {
            self.pause(reason);
            return Code0SessionActionResult::Denied;
        }
        self.state = Code0SessionState::Syncing;
        if !self.acquire_lease() {
            return Code0SessionActionResult::Denied;
        }
        self.failure = None;
        if self.begin_window(Code0WindowMode::Sync) {
            Code0SessionActionResult::Started
        } else {
            Code0SessionActionResult::Denied
        }
    }

    pub fn disconnect(&mut self) -> Code0SessionActionResult {
        if self.stopped {
            return Code0SessionActionResult::Denied;
        }
        if self.cleanup.is_some()
            || self.state == Code0SessionState::CleanupPending
            || self.close_plan == Some(ClosePlan::Disconnect)
        {
            return Code0SessionActionResult::Busy;
        }
        if !(self.has_connection || self.lease_held || self.owned.is_some() || self.cache_may_exist)
        {
            return Code0SessionActionResult::Denied;
        }
        // One admission origin precedes lease/cache/native side effects. Keep a
        // physical close's earlier bound; no later phase may restart this clock.
        if !self
            .cleanup_budget
            .arm(self.clock.now(), self.close_deadline)
        {
            self.reject_disconnect_budget();
            return Code0SessionActionResult::Denied;
        }
        self.state = Code0SessionState::Closing;
        if !self.acquire_lease() {
            // No request/native cleanup was issued and no lease was acquired.
            self.cleanup_budget.clear();
            return Code0SessionActionResult::Denied;
        }
        if self.expire_cleanup_budget(self.clock.now()) {
            self.reject_disconnect_budget();
            return Code0SessionActionResult::Denied;
        }
        let epoch = match self.next_id(&[]) {
            Ok(epoch) => epoch,
            Err(reason) => {
                // Revoke even when a new correlation ID cannot be obtained.
                // Close only the existing owner; never retry entropy to admit
                // an unscoped website-data operation or fabricate an ID.
                self.epoch = None;
                self.revoke_capture();
                self.snapshot = None;
                self.failure = Some(reason);
                self.cleanup_budget.fail();
                self.begin_close(ClosePlan::CleanupFailure);
                (self.on_snapshot_change)(None);
                return Code0SessionActionResult::Denied;
            }
        };
        self.epoch = Some(epoch);
        self.revoke_capture();
        self.failure = None;
        self.snapshot = None;
        self.close_plan = Some(ClosePlan::Disconnect);
        (self.on_snapshot_change)(None);
        self.begin_close(ClosePlan::Disconnect);
        Code0SessionActionResult::Started
    }

    pub fn receive_window_event(&mut self, request: &Code0WindowRequest, event: Code0WindowEvent) {
        // Check every native-owned identity before looking at event contents.
        if self.owned.as_ref() != Some(request) {
            return;
        }
        let now = self.clock.now();
        if matches!(&event, Code0WindowEvent::Released) {
            self.expire_cleanup_budget(now);
            self.expire_close(now);
            if self.close_plan.is_none() {
                self.failure = Some(if self.attempt_deadline.is_some_and(|end| now >= end) {
                    timeout_reason(request.mode)
                } else {
                    Code0SessionFailure::NavigationFailed
                });
                self.close_plan = Some(ClosePlan::Pause);
            }
            self.finish_release();
            return;
        }
        // Physical close failures of this still-owned object remain relevant
        // after Disconnect/stop revoked numeric callbacks from its old epoch.
        if matches!(
            &event,
            Code0WindowEvent::Failed(Code0SessionFailure::CleanupFailed)
        ) && matches!(
            self.state,
            Code0SessionState::Closing | Code0SessionState::CleanupPending
        ) {
            self.demote_activation();
            self.fail_cleanup(Code0SessionFailure::CleanupFailed);
            return;
        }
        if self.stopped || self.epoch.as_deref() != Some(request.connection_epoch.as_str()) {
            return;
        }
        if self.expire_cleanup_budget(now) || self.expire_close(now) || self.expire_attempt(now) {
            return;
        }
        match event {
            Code0WindowEvent::PoliciesInstalled => {
                if self.policies_installed
                    || !matches!(
                        self.state,
                        Code0SessionState::Connecting | Code0SessionState::Syncing
                    )
                {
                    return;
                }
                self.policies_installed = true;
                if let Err(reason) = self.driver.navigate(&request.instance_id) {
                    self.pause(reason);
                }
            }
            Code0WindowEvent::Message {
                payload,
                source_url,
                is_main_frame,
            } => {
                if self.state != Code0SessionState::Syncing || !self.policies_installed {
                    return;
                }
                let Some(validator) = self.validator.as_mut() else {
                    return;
                };
                let event = validator.receive(
                    &payload,
                    &Code0BridgeContext {
                        connection_epoch: request.connection_epoch.clone(),
                        attempt_id: request.instance_id.clone(),
                        source_url,
                        is_main_frame,
                    },
                );
                match event {
                    Code0BridgeEvent::Control => {}
                    Code0BridgeEvent::Capture(capture) => self.accept_capture(capture),
                    Code0BridgeEvent::Invalidated(_) => self.authentication_changed(),
                    Code0BridgeEvent::Failure(_) | Code0BridgeEvent::Rejected(_) => {
                        self.pause(Code0SessionFailure::InvalidResponse);
                    }
                }
            }
            Code0WindowEvent::Failed(reason) => {
                if reason == Code0SessionFailure::AuthenticationChanged
                    && matches!(
                        self.state,
                        Code0SessionState::Connecting
                            | Code0SessionState::Syncing
                            | Code0SessionState::Closing
                    )
                {
                    self.authentication_changed();
                } else if self.state == Code0SessionState::Closing {
                    if matches!(
                        self.close_plan,
                        Some(ClosePlan::Activate | ClosePlan::BeginSync)
                    ) {
                        self.demote_activation();
                        self.failure = Some(reason);
                    }
                } else if matches!(
                    self.state,
                    Code0SessionState::Connecting | Code0SessionState::Syncing
                ) {
                    self.pause(reason);
                }
            }
            Code0WindowEvent::Released => {} // Handled before epoch revocation.
        }
    }

    pub fn receive_cleanup_result(
        &mut self,
        request: &Code0CleanupRequest,
        result: Result<(), Code0SessionFailure>,
    ) {
        if self.cleanup.as_ref() != Some(request) {
            return;
        }
        // A queued/captured result is checked at actual consumption. Matching
        // terminal delivery may retire correlation after failure, never heal it.
        let failed = self.expire_cleanup_budget(self.clock.now())
            || !self.cleanup_budget.matches(request)
            || self.state == Code0SessionState::CleanupPending;
        self.last_cleanup = self.cleanup.take();
        if failed || result.is_err() {
            self.fail_cleanup(self.failure.unwrap_or(Code0SessionFailure::CleanupFailed));
            return;
        }
        self.cleanup_budget.clear();
        self.close_plan = None;
        self.has_connection = false;
        self.last_sync_start = None;
        self.snapshot = None;
        self.failure = None;
        self.state = Code0SessionState::Disconnected;
        self.release_lease();
    }

    pub fn process_scheduled_work(&mut self) {
        let now = self.clock.now();
        if self.expire_cleanup_budget(now) || self.expire_close(now) || self.stopped {
            return;
        }
        if self.expire_attempt(now) {
            return;
        }
        if self.state == Code0SessionState::Active
            && self.automatically_syncing
            && self.owned.is_none()
            && self.cleanup.is_none()
            && self.remaining_interval().is_none()
        {
            self.failure = None;
            self.begin_window(Code0WindowMode::Sync);
        }
    }

    pub fn stop(&mut self) {
        if self.stopped {
            return;
        }
        self.stopped = true;
        self.epoch = None;
        self.revoke_capture();
        // Never turn an unfinished Disconnect/cache cleanup into ordinary Stop.
        if self.state == Code0SessionState::CleanupPending
            || matches!(
                self.close_plan,
                Some(ClosePlan::Disconnect | ClosePlan::CleanupFailure)
            )
        {
            return;
        }
        self.begin_close(ClosePlan::Stop);
    }

    fn is_busy(&self) -> bool {
        self.owned.is_some()
            || self.cleanup.is_some()
            || matches!(
                self.state,
                Code0SessionState::Connecting
                    | Code0SessionState::Syncing
                    | Code0SessionState::Closing
                    | Code0SessionState::CleanupPending
            )
    }

    fn remaining_interval(&self) -> Option<u64> {
        let start = self.last_sync_start?;
        let now = self.clock.now();
        if now < start {
            return Some(300); // A faulty backwards clock cannot grant an attempt.
        }
        let remaining = start
            .saturating_add(Duration::from_secs(300))
            .checked_sub(now)?;
        if remaining.is_zero() {
            None
        } else {
            Some(remaining.as_secs() + u64::from(remaining.subsec_nanos() != 0))
        }
    }

    fn acquire_lease(&mut self) -> bool {
        if self.lease_held {
            return true;
        }
        match self.driver.acquire_profile_lease() {
            Ok(()) => {
                self.lease_held = true;
                true
            }
            Err(reason) => {
                self.failure = Some(if reason == Code0SessionFailure::ProfileBusy {
                    reason
                } else {
                    Code0SessionFailure::StorageFailed
                });
                self.state = Code0SessionState::Paused;
                self.revoke_capture();
                false
            }
        }
    }

    fn ensure_epoch(&mut self) -> Result<(), Code0SessionFailure> {
        if self.epoch.is_none() {
            self.epoch = Some(self.next_id(&[])?);
        }
        Ok(())
    }

    fn next_id(&self, extra: &[&str]) -> Result<String, Code0SessionFailure> {
        let mut excluded = extra.to_vec();
        if let Some(epoch) = self.epoch.as_deref() {
            excluded.push(epoch);
        }
        for request in [&self.owned, &self.last_owned].into_iter().flatten() {
            excluded.extend([
                request.instance_id.as_str(),
                request.connection_epoch.as_str(),
                request.nonce.as_str(),
                request.document_id.as_str(),
            ]);
        }
        for request in [&self.cleanup, &self.last_cleanup].into_iter().flatten() {
            excluded.extend([
                request.connection_epoch.as_str(),
                request.cleanup_id.as_str(),
            ]);
        }
        random_uuid(&excluded)
    }

    fn begin_window(&mut self, mode: Code0WindowMode) -> bool {
        if self.stopped || !self.lease_held || self.owned.is_some() || self.cleanup.is_some() {
            return false;
        }
        let now = self.clock.now();
        let limit = if mode == Code0WindowMode::Login {
            600
        } else {
            45
        };
        let identifiers = (|| {
            let epoch = self
                .epoch
                .clone()
                .ok_or(Code0SessionFailure::ValidationRequired)?;
            let instance_id = self.next_id(&[])?;
            let nonce = self.next_id(&[&instance_id])?;
            let document_id = self.next_id(&[&instance_id, &nonce])?;
            let deadline = now
                .checked_add(Duration::from_secs(limit))
                .ok_or(Code0SessionFailure::ValidationRequired)?;
            Ok((
                Code0WindowRequest {
                    instance_id,
                    connection_epoch: epoch,
                    nonce,
                    document_id,
                    mode,
                },
                deadline,
            ))
        })();
        let (request, deadline) = match identifiers {
            Ok(value) => value,
            Err(reason) => {
                self.pause(reason);
                return false;
            }
        };
        self.policies_installed = false;
        self.close_requested = false;
        self.close_plan = None;
        self.close_deadline = None;
        self.attempt_deadline = Some(deadline);
        self.revoke_capture();
        self.state = if mode == Code0WindowMode::Login {
            Code0SessionState::Connecting
        } else {
            Code0SessionState::Syncing
        };
        if mode == Code0WindowMode::Sync {
            self.last_sync_start = Some(now);
            self.validator = Some(Code0BridgeValidator::new(
                &request.connection_epoch,
                &request.instance_id,
                &request.nonce,
                &request.document_id,
            ));
        }
        self.owned = Some(request.clone());
        if let Err(reason) = self.driver.create_unloaded(request) {
            // Ownership was installed before create. Even an error may leave
            // partial native objects, so do not replace them without Released.
            self.pause(reason);
        }
        true
    }

    fn expire_attempt(&mut self, now: Duration) -> bool {
        if matches!(
            self.state,
            Code0SessionState::Connecting | Code0SessionState::Syncing
        ) && self
            .attempt_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            let mode = self
                .owned
                .as_ref()
                .map_or(Code0WindowMode::Sync, |r| r.mode);
            self.pause(timeout_reason(mode));
            true
        } else {
            false
        }
    }

    fn reject_disconnect_budget(&mut self) {
        self.epoch = None;
        self.snapshot = None;
        self.fail_cleanup(Code0SessionFailure::CleanupFailed);
        self.request_close();
        (self.on_snapshot_change)(None);
    }

    fn expire_cleanup_budget(&mut self, now: Duration) -> bool {
        if self.cleanup_budget.expired(now) {
            self.fail_cleanup(self.failure.unwrap_or(Code0SessionFailure::CleanupFailed));
            true
        } else {
            false
        }
    }

    fn expire_close(&mut self, now: Duration) -> bool {
        if self.owned.is_some() && self.close_deadline.is_some_and(|deadline| now >= deadline) {
            self.close_deadline = None;
            self.demote_activation();
            self.fail_cleanup(Code0SessionFailure::CleanupFailed);
            true
        } else {
            false
        }
    }

    fn accept_capture(&mut self, capture: Code0WebsiteCapture) {
        let timestamp = self.clock.wall_now();
        let candidate = if valid_timestamp(&timestamp) {
            capture.make_snapshot(&timestamp).ok()
        } else {
            None
        };
        let Some(candidate) = candidate else {
            self.pause(Code0SessionFailure::StorageFailed);
            return;
        };
        self.cache_may_exist = true;
        if self.store.save_atomic(&candidate).is_err() {
            self.pause(Code0SessionFailure::StorageFailed);
            return;
        }
        self.snapshot = Some(candidate.clone());
        self.prepare_close(ClosePlan::Activate);
        (self.on_snapshot_change)(Some(candidate));
        self.request_close();
    }

    fn authentication_changed(&mut self) {
        self.epoch = None;
        self.revoke_capture();
        self.has_connection = false;
        self.snapshot = None;
        self.failure = Some(Code0SessionFailure::AuthenticationChanged);
        if self.invalidate_cache().is_err() {
            self.failure = Some(Code0SessionFailure::StorageFailed);
            self.begin_close(ClosePlan::CleanupFailure);
        } else {
            self.begin_close(ClosePlan::Pause);
        }
        (self.on_snapshot_change)(None);
    }

    fn pause(&mut self, reason: Code0SessionFailure) {
        self.failure = Some(reason);
        self.revoke_capture();
        self.begin_close(ClosePlan::Pause);
    }

    fn revoke_capture(&mut self) {
        self.validator = None;
        self.automatically_syncing = false;
    }

    fn prepare_close(&mut self, plan: ClosePlan) {
        self.revoke_capture();
        self.attempt_deadline = None;
        if self.cleanup_budget.active() && self.expire_cleanup_budget(self.clock.now()) {
            return;
        }
        self.close_plan = Some(plan);
        self.state = Code0SessionState::Closing;
        if self.owned.is_some() && self.close_deadline.is_none() {
            self.close_deadline = if plan == ClosePlan::Disconnect {
                self.cleanup_budget.deadline()
            } else {
                self.clock.now().checked_add(Duration::from_secs(10))
            };
            if self.close_deadline.is_none() {
                self.demote_activation();
                self.fail_cleanup(Code0SessionFailure::CleanupFailed);
            }
        }
    }

    fn begin_close(&mut self, plan: ClosePlan) {
        self.prepare_close(plan);
        if self.owned.is_some() {
            self.request_close();
        } else {
            self.finish_close(self.close_plan.unwrap_or(plan));
        }
    }

    fn request_close(&mut self) {
        if self.close_requested {
            return;
        }
        if let Some(request) = &self.owned {
            self.close_requested = true;
            self.driver.close(&request.instance_id);
        }
    }

    fn demote_activation(&mut self) {
        if matches!(
            self.close_plan,
            Some(ClosePlan::Activate | ClosePlan::BeginSync)
        ) {
            self.close_plan = Some(ClosePlan::Pause);
        }
    }

    fn finish_release(&mut self) {
        let mut plan = self.close_plan.unwrap_or(ClosePlan::Pause);
        if self.stopped && !matches!(plan, ClosePlan::Disconnect | ClosePlan::CleanupFailure) {
            plan = ClosePlan::Stop;
        }
        self.last_owned = self.owned.take();
        self.validator = None;
        self.policies_installed = false;
        self.close_requested = false;
        self.attempt_deadline = None;
        self.close_deadline = None;
        self.close_plan = None;
        self.finish_close(plan);
    }

    fn finish_close(&mut self, plan: ClosePlan) {
        match plan {
            ClosePlan::BeginSync => {
                if self.stopped {
                    self.finish_close(ClosePlan::Stop);
                } else {
                    self.begin_window(Code0WindowMode::Sync);
                }
            }
            ClosePlan::Activate => {
                if self.stopped {
                    self.finish_close(ClosePlan::Stop);
                } else {
                    self.state = Code0SessionState::Active;
                    self.failure = None;
                    self.automatically_syncing = true;
                }
            }
            ClosePlan::Pause => {
                self.state = Code0SessionState::Paused;
            }
            ClosePlan::CleanupFailure => {
                self.fail_cleanup(self.failure.unwrap_or(Code0SessionFailure::StorageFailed))
            }
            ClosePlan::Disconnect => self.begin_data_cleanup(),
            ClosePlan::Stop => {
                self.state = if self.snapshot.is_some() {
                    Code0SessionState::Paused
                } else {
                    Code0SessionState::Disconnected
                };
                self.release_lease();
            }
        }
    }

    fn begin_data_cleanup(&mut self) {
        if self.expire_cleanup_budget(self.clock.now()) {
            return;
        }
        if !self.cleanup_budget.active()
            || self.owned.is_some()
            || self.cleanup.is_some()
            || !self.lease_held
        {
            self.fail_cleanup(Code0SessionFailure::CleanupFailed);
            return;
        }
        self.state = Code0SessionState::Closing;
        self.close_plan = Some(ClosePlan::Disconnect);
        if self.invalidate_cache().is_err() {
            self.fail_cleanup(Code0SessionFailure::StorageFailed);
            return;
        }
        if self.expire_cleanup_budget(self.clock.now()) {
            return;
        }
        let request = (|| {
            self.ensure_epoch()?;
            Ok(Code0CleanupRequest {
                connection_epoch: self
                    .epoch
                    .clone()
                    .ok_or(Code0SessionFailure::ValidationRequired)?,
                cleanup_id: self.next_id(&[])?,
            })
        })();
        let request = match request {
            Ok(value) => value,
            Err(reason) => {
                self.fail_cleanup(reason);
                return;
            }
        };
        if self.expire_cleanup_budget(self.clock.now()) || !self.cleanup_budget.bind(&request) {
            self.fail_cleanup(self.failure.unwrap_or(Code0SessionFailure::CleanupFailed));
            return;
        }
        self.cleanup = Some(request.clone());
        if self.driver.clear_website_data(request).is_err() {
            // A start error is not proof that adopted native liabilities vanished.
            // Keep the actual full request until matching terminal delivery.
            self.fail_cleanup(Code0SessionFailure::CleanupFailed);
        }
        self.expire_cleanup_budget(self.clock.now());
    }

    fn invalidate_cache(&mut self) -> Result<(), Code0StoreError> {
        self.store.remove()?;
        self.cache_may_exist = false;
        Ok(())
    }

    fn fail_cleanup(&mut self, reason: Code0SessionFailure) {
        if self.cleanup_budget.fail() {
            // Even an eventual old-owner Released or late matching Ok cannot
            // restart cleanup or turn a failed admitted operation into success.
            self.close_plan = Some(ClosePlan::CleanupFailure);
        }
        self.failure = Some(reason);
        self.revoke_capture();
        self.state = Code0SessionState::CleanupPending;
    }

    fn release_lease(&mut self) {
        if self.lease_held {
            self.lease_held = false;
            self.driver.release_profile_lease();
        }
    }
}

fn timeout_reason(mode: Code0WindowMode) -> Code0SessionFailure {
    if mode == Code0WindowMode::Login {
        Code0SessionFailure::LoginTimedOut
    } else {
        Code0SessionFailure::SyncTimedOut
    }
}

fn valid_timestamp(value: &str) -> bool {
    OffsetDateTime::parse(value, &Rfc3339).is_ok_and(|date| date.unix_timestamp() >= 0)
}

fn cached_projection(value: &UsageSnapshot) -> Option<UsageSnapshot> {
    let balance = value.primary_metric.as_ref()?;
    let used = value.secondary_metric.as_ref()?;
    if value.schema_version != 1
        || value.provider_id != ProviderId::Code0
        || value.source_version.as_deref() != Some("website")
        || !valid_timestamp(&value.fetched_at)
        || balance.label != "Balance"
        || balance.unit != MetricUnit::SiteCredits
        || balance.kind != MetricKind::Balance
        || used.label != "Cumulative usage"
        || used.unit != MetricUnit::SiteCredits
        || used.kind != MetricKind::CumulativeUsage
        || [balance, used].iter().any(|metric| {
            !metric.current.is_finite()
                || metric.current < 0.0
                || metric.limit.is_some()
                || metric.reset_at.is_some()
                || metric.reset_description.is_some()
        })
    {
        return None;
    }
    // Rebuild only the numeric projection, dropping any unrelated cached fields.
    let mut snapshot = Code0WebsiteCapture {
        balance: balance.current,
        cumulative_usage: used.current,
    }
    .make_snapshot(&value.fetched_at)
    .ok()?;
    snapshot.status = UsageStatus::Cached;
    Some(snapshot)
}

fn random_uuid(excluded: &[&str]) -> Result<String, Code0SessionFailure> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| Code0SessionFailure::ValidationRequired)?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let value = format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    );
    // Never retry with a constant/time/counter or knowingly reuse an owned ID.
    if excluded.contains(&value.as_str()) {
        Err(Code0SessionFailure::ValidationRequired)
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod cleanup_deadline_red_tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use super::*;

    #[derive(Clone)]
    struct Clock(Rc<Cell<Duration>>);

    impl Code0SessionClock for Clock {
        fn now(&self) -> Duration {
            self.0.get()
        }

        fn wall_now(&self) -> String {
            "2026-10-29T00:00:00Z".to_owned()
        }
    }

    #[derive(Default)]
    struct DriverFacts {
        windows: Vec<Code0WindowRequest>,
        cleanups: Vec<Code0CleanupRequest>,
        release_commands: usize,
    }

    struct Driver {
        clock: Clock,
        facts: Rc<RefCell<DriverFacts>>,
        lease_finishes_at: Option<Duration>,
    }

    impl Code0WindowDriving for Driver {
        fn acquire_profile_lease(&mut self) -> Result<(), Code0SessionFailure> {
            // Controlled dependency work consumes elapsed time. No Session
            // deadline is computed or emulated by this test driver.
            if let Some(now) = self.lease_finishes_at.take() {
                self.clock.0.set(now);
            }
            Ok(())
        }

        fn release_profile_lease(&mut self) {
            // A command observation, never physical native-release proof.
            self.facts.borrow_mut().release_commands += 1;
        }

        fn create_unloaded(
            &mut self,
            request: Code0WindowRequest,
        ) -> Result<(), Code0SessionFailure> {
            self.facts.borrow_mut().windows.push(request);
            Ok(())
        }

        fn navigate(&mut self, _instance_id: &str) -> Result<(), Code0SessionFailure> {
            Ok(())
        }

        fn close(&mut self, _instance_id: &str) {}

        fn clear_website_data(
            &mut self,
            request: Code0CleanupRequest,
        ) -> Result<(), Code0SessionFailure> {
            self.facts.borrow_mut().cleanups.push(request);
            Ok(())
        }
    }

    struct Store(Option<UsageSnapshot>);

    impl Code0SnapshotStoring for Store {
        fn load(&self) -> Result<Option<UsageSnapshot>, Code0StoreError> {
            Ok(self.0.clone())
        }

        fn save_atomic(&mut self, snapshot: &UsageSnapshot) -> Result<(), Code0StoreError> {
            self.0 = Some(snapshot.clone());
            Ok(())
        }

        fn remove(&mut self) -> Result<(), Code0StoreError> {
            self.0 = None;
            Ok(())
        }
    }

    type Subject = Code0WebSession<Clock, Driver, Store>;

    struct Fixture {
        session: Subject,
        clock: Clock,
        facts: Rc<RefCell<DriverFacts>>,
    }

    impl Fixture {
        fn cached(lease_finishes_at: Option<Duration>) -> Self {
            let clock = Clock(Rc::new(Cell::new(Duration::from_millis(1000))));
            let facts = Rc::new(RefCell::new(DriverFacts::default()));
            let cached = Code0WebsiteCapture {
                balance: 2.0,
                cumulative_usage: 1.0,
            }
            .make_snapshot("2026-10-28T00:00:00Z")
            .expect("valid numeric cache fixture");
            let session = Code0WebSession::new(
                clock.clone(),
                Driver {
                    clock: clock.clone(),
                    facts: facts.clone(),
                    lease_finishes_at,
                },
                Store(Some(cached)),
                Box::new(|_| {}),
            );
            assert_eq!(session.state(), Code0SessionState::Paused);
            Self {
                session,
                clock,
                facts,
            }
        }

        fn at_millis(&self, millis: u64) {
            self.clock.0.set(Duration::from_millis(millis));
        }

        fn cleanup(&self) -> Code0CleanupRequest {
            let facts = self.facts.borrow();
            assert_eq!(facts.cleanups.len(), 1, "one real Session cleanup command");
            facts.cleanups[0].clone()
        }

        fn window(&self) -> Code0WindowRequest {
            let facts = self.facts.borrow();
            assert_eq!(facts.windows.len(), 1, "one real Session window command");
            facts.windows[0].clone()
        }
    }

    fn different_uuid(value: &str) -> String {
        let mut other = value.to_owned();
        other.replace_range(..1, if value.starts_with('0') { "1" } else { "0" });
        other
    }

    // Detects anchoring the ownerless budget after acquire instead of admission,
    // or never expiring cleanup after the ordinary owner is absent.
    #[test]
    fn ownerless_disconnect_deadline_starts_before_lease_side_effect() {
        let mut f = Fixture::cached(Some(Duration::from_millis(10000)));
        assert_eq!(f.session.disconnect(), Code0SessionActionResult::Started);
        let _cleanup = f.cleanup();
        assert!(f.facts.borrow().windows.is_empty());
        f.at_millis(10999);
        f.session.process_scheduled_work();
        assert_eq!(f.session.state(), Code0SessionState::Closing);
        assert_eq!(f.session.failure(), None);
        f.at_millis(11000);
        f.session.process_scheduled_work();
        assert_eq!(
            f.session.state(),
            Code0SessionState::CleanupPending,
            "ownerless Disconnect must expire at admission+10s, including lease work"
        );
        assert_eq!(
            f.session.failure(),
            Some(Code0SessionFailure::CleanupFailed)
        );
        assert_eq!(f.facts.borrow().release_commands, 0);
    }

    // Detects granting a second ten seconds when old Released starts cleanup.
    #[test]
    fn old_owner_released_near_end_does_not_reset_disconnect_deadline() {
        let mut f = Fixture::cached(None);
        assert_eq!(f.session.sync_now(), Code0SessionActionResult::Started);
        let window = f.window();
        assert_eq!(f.session.disconnect(), Code0SessionActionResult::Started);
        assert!(f.facts.borrow().cleanups.is_empty());
        f.at_millis(10999);
        f.session
            .receive_window_event(&window, Code0WindowEvent::Released);
        let _cleanup = f.cleanup();
        assert_eq!(f.session.state(), Code0SessionState::Closing);
        f.at_millis(11000);
        f.session.process_scheduled_work();
        assert_eq!(
            f.session.state(),
            Code0SessionState::CleanupPending,
            "old owner release must not restart the admitted Disconnect budget"
        );
        assert_eq!(
            f.session.failure(),
            Some(Code0SessionFailure::CleanupFailed)
        );
        assert_eq!(f.facts.borrow().release_commands, 0);
    }

    // Models a queued completion, not a native callback/provenance witness.
    // Detects trusting its early availability over actual consumption time.
    #[test]
    fn completion_available_before_end_but_consumed_at_end_is_not_success() {
        let mut f = Fixture::cached(None);
        assert_eq!(f.session.disconnect(), Code0SessionActionResult::Started);
        f.at_millis(10999);
        let queued_completion = (f.cleanup(), Ok(()));
        f.at_millis(11000);
        f.session
            .receive_cleanup_result(&queued_completion.0, queued_completion.1);
        assert_eq!(
            f.session.state(),
            Code0SessionState::CleanupPending,
            "matching cleanup success consumed at the absolute end must fail"
        );
        assert_eq!(
            f.session.failure(),
            Some(Code0SessionFailure::CleanupFailed)
        );
        assert_eq!(f.facts.borrow().release_commands, 0);
    }

    // Positive boundary control: a blanket rejection would break this case.
    #[test]
    fn matching_cleanup_consumed_one_millisecond_before_end_succeeds() {
        let mut f = Fixture::cached(None);
        assert_eq!(f.session.disconnect(), Code0SessionActionResult::Started);
        let cleanup = f.cleanup();
        f.at_millis(10999);
        f.session.receive_cleanup_result(&cleanup, Ok(()));
        assert_eq!(f.session.state(), Code0SessionState::Disconnected);
        assert_eq!(f.session.failure(), None);
        assert!(f.session.snapshot().is_none());
        assert!(!f.session.is_automatically_syncing());
        assert_eq!(f.facts.borrow().release_commands, 1);
        assert!(f.facts.borrow().windows.is_empty());
    }

    // Positive identity control: wrong success/error cannot consume or poison
    // the actual request, which still succeeds before its original end.
    #[test]
    fn wrong_cleanup_identities_do_not_consume_the_matching_result() {
        for change_epoch in [true, false] {
            let mut f = Fixture::cached(None);
            assert_eq!(f.session.disconnect(), Code0SessionActionResult::Started);
            let cleanup = f.cleanup();
            let mut wrong = cleanup.clone();
            if change_epoch {
                wrong.connection_epoch = different_uuid(&wrong.connection_epoch);
            } else {
                wrong.cleanup_id = different_uuid(&wrong.cleanup_id);
            }
            f.at_millis(10000);
            f.session.receive_cleanup_result(&wrong, Ok(()));
            f.session
                .receive_cleanup_result(&wrong, Err(Code0SessionFailure::CleanupFailed));
            assert_eq!(f.session.state(), Code0SessionState::Closing);
            assert_eq!(f.session.failure(), None);
            assert_eq!(f.facts.borrow().release_commands, 0);
            f.at_millis(10999);
            f.session.receive_cleanup_result(&cleanup, Ok(()));
            assert_eq!(f.session.state(), Code0SessionState::Disconnected);
            assert_eq!(f.session.failure(), None);
            assert_eq!(f.facts.borrow().release_commands, 1);
        }
    }

    // Detects a wrong identity refreshing the deadline before matching success.
    #[test]
    fn wrong_cleanup_identities_cannot_extend_the_original_deadline() {
        for change_epoch in [true, false] {
            let mut f = Fixture::cached(None);
            assert_eq!(f.session.disconnect(), Code0SessionActionResult::Started);
            let cleanup = f.cleanup();
            let mut wrong = cleanup.clone();
            if change_epoch {
                wrong.connection_epoch = different_uuid(&wrong.connection_epoch);
            } else {
                wrong.cleanup_id = different_uuid(&wrong.cleanup_id);
            }
            f.at_millis(10999);
            f.session.receive_cleanup_result(&wrong, Ok(()));
            assert_eq!(f.session.state(), Code0SessionState::Closing);
            f.at_millis(11000);
            f.session.receive_cleanup_result(&cleanup, Ok(()));
            assert_eq!(
                f.session.state(),
                Code0SessionState::CleanupPending,
                "wrong cleanup identity cannot extend the absolute Disconnect end"
            );
            assert_eq!(
                f.session.failure(),
                Some(Code0SessionFailure::CleanupFailed)
            );
            assert_eq!(f.facts.borrow().release_commands, 0);
        }
    }

    // Positive Stop control: admitted teardown still completes, but Stop never
    // grants Connect/LoginCompleted/SyncNow/Disconnect or a replacement window.
    #[test]
    fn stop_allows_admitted_cleanup_before_end_without_reopening_actions() {
        let mut f = Fixture::cached(None);
        assert_eq!(f.session.sync_now(), Code0SessionActionResult::Started);
        let window = f.window();
        assert_eq!(f.session.disconnect(), Code0SessionActionResult::Started);
        f.at_millis(2000);
        f.session.stop();
        assert_eq!(f.session.connect(), Code0SessionActionResult::Denied);
        assert_eq!(
            f.session.login_completed(),
            Code0SessionActionResult::Denied
        );
        assert_eq!(f.session.sync_now(), Code0SessionActionResult::Denied);
        assert_eq!(f.session.disconnect(), Code0SessionActionResult::Denied);
        f.at_millis(10998);
        f.session
            .receive_window_event(&window, Code0WindowEvent::Released);
        let cleanup = f.cleanup();
        f.at_millis(10999);
        f.session.receive_cleanup_result(&cleanup, Ok(()));
        assert_eq!(f.session.state(), Code0SessionState::Disconnected);
        assert_eq!(f.session.failure(), None);
        assert_eq!(f.facts.borrow().release_commands, 1);
        assert_eq!(f.session.connect(), Code0SessionActionResult::Denied);
        assert_eq!(
            f.session.login_completed(),
            Code0SessionActionResult::Denied
        );
        assert_eq!(f.session.sync_now(), Code0SessionActionResult::Denied);
        assert_eq!(f.session.disconnect(), Code0SessionActionResult::Denied);
        f.at_millis(400000);
        f.session.process_scheduled_work();
        assert!(!f.session.is_automatically_syncing());
        assert_eq!(f.facts.borrow().windows.len(), 1);
        assert_eq!(f.facts.borrow().cleanups.len(), 1);
    }

    // Detects putting cleanup deadline processing behind the stopped early exit.
    #[test]
    fn stop_does_not_disable_the_admitted_cleanup_deadline() {
        let mut f = Fixture::cached(None);
        assert_eq!(f.session.disconnect(), Code0SessionActionResult::Started);
        let _cleanup = f.cleanup();
        f.at_millis(2000);
        f.session.stop();
        f.at_millis(11000);
        f.session.process_scheduled_work();
        assert_eq!(
            f.session.state(),
            Code0SessionState::CleanupPending,
            "Stop must still expire the original admitted cleanup operation"
        );
        assert_eq!(
            f.session.failure(),
            Some(Code0SessionFailure::CleanupFailed)
        );
        assert_eq!(f.session.connect(), Code0SessionActionResult::Denied);
        assert_eq!(f.session.sync_now(), Code0SessionActionResult::Denied);
        assert_eq!(f.facts.borrow().release_commands, 0);
    }
}

#[cfg(test)]
mod cleanup_deadline_green_tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use super::*;

    #[derive(Clone)]
    struct Clock(Rc<Cell<Duration>>);

    impl Code0SessionClock for Clock {
        fn now(&self) -> Duration {
            self.0.get()
        }

        fn wall_now(&self) -> String {
            "2026-10-29T00:00:00Z".to_owned()
        }
    }

    #[derive(Default)]
    struct Facts {
        lease_calls: usize,
        release_calls: usize,
        removals: usize,
        windows: Vec<Code0WindowRequest>,
        cleanups: Vec<Code0CleanupRequest>,
        budgets_during_clear: Vec<Option<Code0CleanupBudgetSnapshot>>,
    }

    struct Driver {
        clock: Clock,
        facts: Rc<RefCell<Facts>>,
        budget_slot: Rc<RefCell<Option<Code0CleanupBudgetHandle>>>,
        lease_finishes_at: Option<Duration>,
        clear_error: bool,
    }

    impl Code0WindowDriving for Driver {
        fn acquire_profile_lease(&mut self) -> Result<(), Code0SessionFailure> {
            self.facts.borrow_mut().lease_calls += 1;
            if let Some(now) = self.lease_finishes_at.take() {
                self.clock.0.set(now);
            }
            Ok(())
        }

        fn release_profile_lease(&mut self) {
            self.facts.borrow_mut().release_calls += 1;
        }

        fn create_unloaded(
            &mut self,
            request: Code0WindowRequest,
        ) -> Result<(), Code0SessionFailure> {
            self.facts.borrow_mut().windows.push(request);
            Ok(())
        }

        fn navigate(&mut self, _instance_id: &str) -> Result<(), Code0SessionFailure> {
            Ok(())
        }

        fn close(&mut self, _instance_id: &str) {}

        fn clear_website_data(
            &mut self,
            request: Code0CleanupRequest,
        ) -> Result<(), Code0SessionFailure> {
            // This calls the real read-only handle inside the driver command.
            // A Session-held context borrow would panic rather than be hidden.
            let observed = self
                .budget_slot
                .borrow()
                .as_ref()
                .and_then(|handle| handle.read(&request));
            let mut facts = self.facts.borrow_mut();
            facts.budgets_during_clear.push(observed);
            facts.cleanups.push(request);
            if self.clear_error {
                Err(Code0SessionFailure::CleanupFailed)
            } else {
                Ok(())
            }
        }
    }

    struct Store {
        value: Option<UsageSnapshot>,
        facts: Rc<RefCell<Facts>>,
    }

    impl Code0SnapshotStoring for Store {
        fn load(&self) -> Result<Option<UsageSnapshot>, Code0StoreError> {
            Ok(self.value.clone())
        }

        fn save_atomic(&mut self, snapshot: &UsageSnapshot) -> Result<(), Code0StoreError> {
            self.value = Some(snapshot.clone());
            Ok(())
        }

        fn remove(&mut self) -> Result<(), Code0StoreError> {
            self.facts.borrow_mut().removals += 1;
            self.value = None;
            Ok(())
        }
    }

    type Subject = Code0WebSession<Clock, Driver, Store>;

    struct Fixture {
        session: Subject,
        clock: Clock,
        facts: Rc<RefCell<Facts>>,
        budget: Code0CleanupBudgetHandle,
    }

    impl Fixture {
        fn new(cached: bool, lease_finishes_at: Option<Duration>, clear_error: bool) -> Self {
            let clock = Clock(Rc::new(Cell::new(Duration::from_millis(1000))));
            let facts = Rc::new(RefCell::new(Facts::default()));
            let budget_slot = Rc::new(RefCell::new(None));
            let value = cached.then(|| {
                Code0WebsiteCapture {
                    balance: 2.0,
                    cumulative_usage: 1.0,
                }
                .make_snapshot("2026-10-28T00:00:00Z")
                .expect("numeric cache fixture")
            });
            let session = Code0WebSession::new(
                clock.clone(),
                Driver {
                    clock: clock.clone(),
                    facts: facts.clone(),
                    budget_slot: budget_slot.clone(),
                    lease_finishes_at,
                    clear_error,
                },
                Store {
                    value,
                    facts: facts.clone(),
                },
                Box::new(|_| {}),
            );
            let budget = session.cleanup_budget_handle();
            *budget_slot.borrow_mut() = Some(budget.clone());
            Self {
                session,
                clock,
                facts,
                budget,
            }
        }

        fn at(&self, millis: u64) {
            self.clock.0.set(Duration::from_millis(millis));
        }

        fn cleanup(&self) -> Code0CleanupRequest {
            let facts = self.facts.borrow();
            assert_eq!(facts.cleanups.len(), 1);
            facts.cleanups[0].clone()
        }
    }

    // Detects replacing an earlier physical-close bound with admission+10s.
    #[test]
    fn earlier_old_close_bound_survives_disconnect_and_cleanup() {
        let mut f = Fixture::new(true, None, false);
        assert_eq!(f.session.sync_now(), Code0SessionActionResult::Started);
        let window = f.facts.borrow().windows[0].clone();
        f.at(2000);
        f.session.receive_window_event(
            &window,
            Code0WindowEvent::Failed(Code0SessionFailure::NavigationFailed),
        );
        f.at(5000);
        assert_eq!(f.session.disconnect(), Code0SessionActionResult::Started);
        f.at(11999);
        f.session
            .receive_window_event(&window, Code0WindowEvent::Released);
        let cleanup = f.cleanup();
        let budget = f.budget.read(&cleanup).expect("same bound operation");
        assert_eq!(budget.admitted_at, Duration::from_millis(5000));
        assert_eq!(budget.deadline, Duration::from_millis(12000));
        f.at(12000);
        f.session.receive_cleanup_result(&cleanup, Ok(()));
        assert_eq!(f.session.state(), Code0SessionState::CleanupPending);
        assert_eq!(f.facts.borrow().release_calls, 0);
    }

    // Detects late Ok healing the state or releasing the held lease after expiry.
    #[test]
    fn expiry_then_late_matching_success_never_heals_or_releases() {
        let mut f = Fixture::new(true, None, false);
        assert_eq!(f.session.disconnect(), Code0SessionActionResult::Started);
        let cleanup = f.cleanup();
        f.at(11000);
        f.session.process_scheduled_work();
        assert_eq!(f.session.state(), Code0SessionState::CleanupPending);
        f.at(11001);
        f.session.receive_cleanup_result(&cleanup, Ok(()));
        f.session.receive_cleanup_result(&cleanup, Ok(()));
        assert_eq!(f.session.state(), Code0SessionState::CleanupPending);
        assert_eq!(
            f.session.failure(),
            Some(Code0SessionFailure::CleanupFailed)
        );
        assert!(
            f.budget
                .read(&cleanup)
                .expect("failed identity retained")
                .failed
        );
        assert_eq!(f.facts.borrow().release_calls, 0);
        assert_eq!(f.session.sync_now(), Code0SessionActionResult::Busy);
    }

    // Detects a backwards observed sample granting success before the deadline.
    #[test]
    fn backwards_cleanup_clock_cannot_accept_matching_success() {
        for backwards in [999, 1500] {
            let mut f = Fixture::new(true, None, false);
            assert_eq!(f.session.disconnect(), Code0SessionActionResult::Started);
            let cleanup = f.cleanup();
            f.at(2000);
            f.session.process_scheduled_work();
            f.at(backwards);
            f.session.receive_cleanup_result(&cleanup, Ok(()));
            assert_eq!(f.session.state(), Code0SessionState::CleanupPending);
            assert_eq!(
                f.session.failure(),
                Some(Code0SessionFailure::CleanupFailed)
            );
            assert_eq!(f.facts.borrow().release_calls, 0);
        }
    }

    // Detects saturating overflow or lease/cache/native work before checked arm.
    #[test]
    fn overflowing_admission_fails_before_lease_cache_or_native_work() {
        let mut f = Fixture::new(true, None, false);
        f.clock.0.set(Duration::MAX);
        assert_eq!(f.session.disconnect(), Code0SessionActionResult::Denied);
        assert_eq!(f.session.state(), Code0SessionState::CleanupPending);
        assert_eq!(
            f.session.failure(),
            Some(Code0SessionFailure::CleanupFailed)
        );
        let facts = f.facts.borrow();
        assert_eq!(facts.lease_calls, 0);
        assert_eq!(facts.removals, 0);
        assert!(facts.windows.is_empty() && facts.cleanups.is_empty());
        assert_eq!(facts.release_calls, 0);
    }

    // Detects read-side arming, wrong-request visibility or a borrow crossing clear.
    #[test]
    fn cold_reads_are_inert_and_driver_observes_the_same_bound_budget() {
        let mut cold = Fixture::new(false, None, false);
        let unknown = Code0CleanupRequest {
            connection_epoch: "unowned-epoch".to_owned(),
            cleanup_id: "unowned-cleanup".to_owned(),
        };
        assert!(cold.budget.read(&unknown).is_none());
        assert_eq!(cold.session.disconnect(), Code0SessionActionResult::Denied);
        assert!(cold.budget.read(&unknown).is_none());
        assert_eq!(cold.facts.borrow().lease_calls, 0);
        assert_eq!(cold.facts.borrow().removals, 0);
        let mut f = Fixture::new(true, None, false);
        assert_eq!(f.session.disconnect(), Code0SessionActionResult::Started);
        let cleanup = f.cleanup();
        let bound = f.budget.read(&cleanup).expect("Session bound before clear");
        assert_eq!(bound.request, cleanup);
        assert_eq!(bound.admitted_at, Duration::from_millis(1000));
        assert_eq!(bound.deadline, Duration::from_millis(11000));
        assert!(!bound.failed);
        assert_eq!(
            f.facts.borrow().budgets_during_clear,
            vec![Some(bound.clone())]
        );
        let mut wrong = cleanup.clone();
        wrong.cleanup_id.push('x');
        assert!(f.budget.read(&wrong).is_none());
        wrong = cleanup.clone();
        wrong.connection_epoch.push('x');
        assert!(f.budget.read(&wrong).is_none());
        assert_eq!(f.budget.read(&cleanup), Some(bound));
        f.at(10999);
        f.session.receive_cleanup_result(&cleanup, Ok(()));
        assert!(f.budget.read(&cleanup).is_none());
        assert_eq!(f.session.state(), Code0SessionState::Disconnected);
        assert_eq!(f.facts.borrow().release_calls, 1);
    }

    // Detects discarding correlation on start Err or treating late Ok as recovery.
    #[test]
    fn clear_start_error_keeps_failed_identity_without_late_healing() {
        let mut f = Fixture::new(true, None, true);
        assert_eq!(f.session.disconnect(), Code0SessionActionResult::Started);
        let cleanup = f.cleanup();
        assert_eq!(f.session.state(), Code0SessionState::CleanupPending);
        assert!(f.budget.read(&cleanup).expect("liability identity").failed);
        f.at(10999);
        f.session.receive_cleanup_result(&cleanup, Ok(()));
        assert_eq!(f.session.state(), Code0SessionState::CleanupPending);
        assert_eq!(
            f.session.failure(),
            Some(Code0SessionFailure::CleanupFailed)
        );
        assert_eq!(f.facts.borrow().release_calls, 0);
    }

    // Detects issuing clear after lease work has consumed the entire operation.
    #[test]
    fn lease_return_at_end_cannot_start_a_new_cleanup_budget() {
        let mut f = Fixture::new(true, Some(Duration::from_millis(11000)), false);
        assert_eq!(f.session.disconnect(), Code0SessionActionResult::Denied);
        assert_eq!(f.session.state(), Code0SessionState::CleanupPending);
        let facts = f.facts.borrow();
        assert_eq!(facts.lease_calls, 1);
        assert_eq!(facts.removals, 0);
        assert!(facts.cleanups.is_empty());
        assert_eq!(facts.release_calls, 0);
    }

    // Detects a retained reader keeping a dropped writer's operation usable.
    #[test]
    fn dropped_session_revokes_reader_without_synthesizing_native_release() {
        let mut f = Fixture::new(true, None, false);
        assert_eq!(f.session.disconnect(), Code0SessionActionResult::Started);
        let cleanup = f.cleanup();
        let retained = f.budget.clone();
        assert!(retained.read(&cleanup).is_some());
        drop(f.session);
        assert!(retained.read(&cleanup).is_none());
        assert_eq!(f.facts.borrow().release_calls, 0);
    }
}

#[cfg(test)]
mod terminal_readiness_red_tests {
    use super::*;

    #[derive(Clone)]
    struct Clock;

    impl Code0SessionClock for Clock {
        fn now(&self) -> Duration {
            Duration::from_secs(1)
        }

        fn wall_now(&self) -> String {
            "2026-10-30T00:00:00Z".to_owned()
        }
    }

    #[derive(Default)]
    struct Facts {
        windows: Vec<Code0WindowRequest>,
        cleanups: Vec<Code0CleanupRequest>,
        releases: usize,
    }

    struct Driver(Rc<RefCell<Facts>>);

    impl Code0WindowDriving for Driver {
        fn acquire_profile_lease(&mut self) -> Result<(), Code0SessionFailure> {
            Ok(())
        }

        fn release_profile_lease(&mut self) {
            self.0.borrow_mut().releases += 1;
        }

        fn create_unloaded(
            &mut self,
            request: Code0WindowRequest,
        ) -> Result<(), Code0SessionFailure> {
            self.0.borrow_mut().windows.push(request);
            Ok(())
        }

        fn navigate(&mut self, _: &str) -> Result<(), Code0SessionFailure> {
            Ok(())
        }

        fn close(&mut self, _: &str) {}

        fn clear_website_data(
            &mut self,
            request: Code0CleanupRequest,
        ) -> Result<(), Code0SessionFailure> {
            self.0.borrow_mut().cleanups.push(request);
            Ok(())
        }
    }

    struct Store(Option<UsageSnapshot>);

    impl Code0SnapshotStoring for Store {
        fn load(&self) -> Result<Option<UsageSnapshot>, Code0StoreError> {
            Ok(self.0.clone())
        }

        fn save_atomic(&mut self, value: &UsageSnapshot) -> Result<(), Code0StoreError> {
            self.0 = Some(value.clone());
            Ok(())
        }

        fn remove(&mut self) -> Result<(), Code0StoreError> {
            self.0 = None;
            Ok(())
        }
    }

    type Subject = Code0WebSession<Clock, Driver, Store>;

    fn subject(cached: bool) -> (Subject, Rc<RefCell<Facts>>) {
        let facts = Rc::new(RefCell::new(Facts::default()));
        let value = cached.then(|| {
            Code0WebsiteCapture {
                balance: 2.0,
                cumulative_usage: 1.0,
            }
            .make_snapshot("2026-10-29T00:00:00Z")
            .expect("terminal fixture numeric cache")
        });
        (
            Code0WebSession::new(Clock, Driver(facts.clone()), Store(value), Box::new(|_| {})),
            facts,
        )
    }

    // Detects treating a cold Paused/Disconnected projection as consumed Stop.
    #[test]
    fn terminal_cold_projection_requires_consumed_stop() {
        let mut settled = Vec::new();
        for cached in [false, true] {
            let (mut session, facts) = subject(cached);
            assert_eq!(
                session.state(),
                if cached {
                    Code0SessionState::Paused
                } else {
                    Code0SessionState::Disconnected
                }
            );
            assert!(
                !session.terminal_stop_consumed(),
                "cold DTO is not Stop proof"
            );
            session.stop();
            assert_eq!(session.connect(), Code0SessionActionResult::Denied);
            assert!(facts.borrow().windows.is_empty());
            settled.push(session.terminal_stop_consumed());
        }
        assert_eq!(
            settled,
            [true, true],
            "TR-S1 consumed cold Stop must be ready"
        );
    }

    // Detects close-request/stale Released replacing actual ordinary release.
    #[test]
    fn terminal_ordinary_owner_requires_matching_released() {
        let (mut session, facts) = subject(false);
        assert_eq!(session.connect(), Code0SessionActionResult::Started);
        let request = facts.borrow().windows[0].clone();
        session.stop();
        assert_eq!(session.state(), Code0SessionState::Closing);
        assert!(
            !session.terminal_stop_consumed(),
            "ordinary owner still retained"
        );
        let mut stale = request.clone();
        stale.document_id.push_str("-stale");
        session.receive_window_event(&stale, Code0WindowEvent::Released);
        assert_eq!(session.owned.as_ref(), Some(&request));
        assert_eq!(facts.borrow().releases, 0);
        assert!(
            !session.terminal_stop_consumed(),
            "stale Released is not proof"
        );
        session.receive_window_event(&request, Code0WindowEvent::Released);
        assert!(session.owned.is_none());
        assert_eq!(facts.borrow().releases, 1);
        assert_eq!(session.connect(), Code0SessionActionResult::Denied);
        assert!(
            session.terminal_stop_consumed(),
            "TR-S2 matching ordinary Released must complete Stop"
        );
    }

    // Detects Stop overriding an admitted Disconnect before matching clear ACK.
    #[test]
    fn terminal_disconnect_requires_consumed_matching_cleanup() {
        let (mut session, facts) = subject(true);
        assert_eq!(session.disconnect(), Code0SessionActionResult::Started);
        let request = facts.borrow().cleanups[0].clone();
        session.stop();
        assert!(facts.borrow().windows.is_empty());
        assert_eq!(session.cleanup.as_ref(), Some(&request));
        assert!(
            !session.terminal_stop_consumed(),
            "admitted cleanup still pending"
        );
        let mut stale = request.clone();
        stale.cleanup_id.push_str("-stale");
        session.receive_cleanup_result(&stale, Ok(()));
        assert_eq!(session.cleanup.as_ref(), Some(&request));
        assert_eq!(facts.borrow().releases, 0);
        session.receive_cleanup_result(&request, Ok(()));
        assert_eq!(session.state(), Code0SessionState::Disconnected);
        assert!(session.cleanup.is_none());
        assert_eq!(facts.borrow().releases, 1);
        assert!(
            session.terminal_stop_consumed(),
            "TR-S3 matching cleanup consumption must complete Stop"
        );
    }

    // Control: sticky cleanup failure cannot be healed into terminal readiness.
    #[test]
    fn terminal_cleanup_failure_stays_pending_control() {
        let (mut session, facts) = subject(true);
        assert_eq!(session.disconnect(), Code0SessionActionResult::Started);
        let request = facts.borrow().cleanups[0].clone();
        session.stop();
        session.receive_cleanup_result(&request, Err(Code0SessionFailure::CleanupFailed));
        session.receive_cleanup_result(&request, Ok(()));
        session.stop();
        session.process_scheduled_work();
        assert_eq!(session.state(), Code0SessionState::CleanupPending);
        assert_eq!(session.failure(), Some(Code0SessionFailure::CleanupFailed));
        assert_eq!(facts.borrow().releases, 0);
        assert_eq!(session.connect(), Code0SessionActionResult::Denied);
        assert!(
            !session.terminal_stop_consumed(),
            "TR-S4 sticky failure is not ready"
        );
    }
}
