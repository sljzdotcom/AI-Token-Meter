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
        self.state = Code0SessionState::Closing;
        if !self.acquire_lease() {
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
        if self.expire_close(now) || self.expire_attempt(now) {
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
        self.last_cleanup = self.cleanup.take();
        if result.is_err() {
            self.fail_cleanup(Code0SessionFailure::CleanupFailed);
            return;
        }
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
        if self.expire_close(now) || self.stopped {
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
        self.close_plan = Some(plan);
        self.state = Code0SessionState::Closing;
        if self.owned.is_some() && self.close_deadline.is_none() {
            self.close_deadline = self.clock.now().checked_add(Duration::from_secs(10));
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
            self.finish_close(plan);
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
        if self.owned.is_some() || self.cleanup.is_some() || !self.lease_held {
            self.fail_cleanup(Code0SessionFailure::CleanupFailed);
            return;
        }
        self.state = Code0SessionState::Closing;
        self.close_plan = Some(ClosePlan::Disconnect);
        if self.invalidate_cache().is_err() {
            self.fail_cleanup(Code0SessionFailure::StorageFailed);
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
        self.cleanup = Some(request.clone());
        if self.driver.clear_website_data(request).is_err() {
            self.last_cleanup = self.cleanup.take();
            self.fail_cleanup(Code0SessionFailure::CleanupFailed);
        }
    }

    fn invalidate_cache(&mut self) -> Result<(), Code0StoreError> {
        self.store.remove()?;
        self.cache_may_exist = false;
        Ok(())
    }

    fn fail_cleanup(&mut self, reason: Code0SessionFailure) {
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
