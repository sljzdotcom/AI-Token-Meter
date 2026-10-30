//! UI-local coordinator for the single pure Code0 session authority.
//! No product command, UI registry, native adapter or scheduler is installed.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};

use super::code0_web_session::{
    Code0CleanupRequest, Code0SessionActionResult, Code0SessionClock, Code0SessionFailure,
    Code0SessionState, Code0SnapshotStoring, Code0WebSession, Code0WindowDriving, Code0WindowEvent,
    Code0WindowRequest,
};
use crate::domain::UsageSnapshot;

const GENERAL_CAPACITY: usize = 64;
const MAX_REVISION: u64 = 9_007_199_254_740_991;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Code0Action {
    Connect,
    LoginCompleted,
    SyncNow,
    Disconnect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code0DispatchError {
    Unavailable,
    QueueFull,
    RevisionExhausted,
    InvalidProjection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Code0Revision(u64);
impl Code0Revision {
    pub fn new(value: u64) -> Result<Self, Code0DispatchError> {
        if value > MAX_REVISION {
            Err(Code0DispatchError::RevisionExhausted)
        } else {
            Ok(Self(value))
        }
    }
    fn next(self) -> Result<Self, Code0DispatchError> {
        self.0
            .checked_add(1)
            .ok_or(Code0DispatchError::RevisionExhausted)
            .and_then(Self::new)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Code0MetricDto {
    label: &'static str,
    current: f64,
    limit: Option<()>,
    unit: &'static str,
    kind: &'static str,
    reset_at: Option<()>,
    reset_description: Option<()>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Code0SnapshotDto {
    schema_version: u8,
    provider_id: &'static str,
    display_name: &'static str,
    status: &'static str,
    used_ratio: Option<()>,
    primary_metric: Code0MetricDto,
    secondary_metric: Code0MetricDto,
    fetched_at: String,
    stale_after_seconds: u16,
    source_version: &'static str,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Code0SessionDto {
    pub revision: u64,
    state: &'static str,
    failure: Option<&'static str>,
    is_automatically_syncing: bool,
    retry_after_seconds: Option<u16>,
    snapshot: Option<Code0SnapshotDto>,
}
impl Code0SessionDto {
    fn empty(revision: Code0Revision) -> Self {
        Self {
            revision: revision.0,
            state: "disconnected",
            failure: None,
            is_automatically_syncing: false,
            retry_after_seconds: None,
            snapshot: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Code0Reply {
    View(Box<Code0SessionDto>),
    Action(Code0SessionActionResult),
    Stopped,
}
impl Serialize for Code0Reply {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::View(view) => view.serialize(serializer),
            Self::Stopped => serializer.serialize_unit(),
            Self::Action(action) => {
                let mut map = serializer.serialize_map(Some(
                    if matches!(action, Code0SessionActionResult::Throttled { .. }) {
                        2
                    } else {
                        1
                    },
                ))?;
                match action {
                    Code0SessionActionResult::Started => map.serialize_entry("kind", "started")?,
                    Code0SessionActionResult::Busy => map.serialize_entry("kind", "busy")?,
                    Code0SessionActionResult::Denied => map.serialize_entry("kind", "denied")?,
                    Code0SessionActionResult::Throttled { seconds_remaining } => {
                        if *seconds_remaining > 300 {
                            return Err(serde::ser::Error::custom("invalid throttle"));
                        }
                        map.serialize_entry("kind", "throttled")?;
                        map.serialize_entry("secondsRemaining", seconds_remaining)?;
                    }
                }
                map.end()
            }
        }
    }
}
pub type Code0ReplySink = Box<dyn FnOnce(Result<Code0Reply, Code0DispatchError>) + Send>;
type Code0Emitter = Box<dyn FnMut(&Code0SessionDto)>;

/// Signals a UI-local drain by binding/registry lookup. Implementations may run
/// inline; they must never capture/move the non-Send session in a Send closure.
pub trait Code0Schedule: Send + Sync {
    fn schedule(&self) -> Result<(), Code0DispatchError>;
}

enum Message {
    Read(Code0ReplySink),
    Action(Code0Action, Code0ReplySink),
    Window(u64, Code0WindowRequest, Code0WindowEvent),
    Cleanup(u64, Code0CleanupRequest, Result<(), Code0SessionFailure>),
    Tick(u64, u64),
    Stop(Code0ReplySink),
    StopSignal,
    Overflow,
}
impl Message {
    fn reject(self, error: Code0DispatchError) {
        match self {
            Self::Read(reply) | Self::Action(_, reply) | Self::Stop(reply) => reply(Err(error)),
            _ => {}
        }
    }
}

#[derive(Default)]
struct Inbox {
    general: VecDeque<Message>,
    stops: VecDeque<Code0ReplySink>,
    stop_pending: usize,
    stopping: bool,
    stop_signal: bool,
    overflow: bool,
    terminal_error: Option<Code0DispatchError>,
    binding: Option<(u64, u64)>,
    tick: Option<Message>,
    released: Option<Message>,
    // First matching Released fences at most the 64 general messages already
    // accepted. Later traffic and duplicate acknowledgements cannot extend it.
    release_prefix: usize,
    cleanup: Option<Message>,
    // Transport correlation/reservations only; these never grant authority.
    expected_window: Option<(u64, Code0WindowRequest)>,
    expected_cleanup: Option<(u64, Code0CleanupRequest)>,
    scheduled: bool,
}
impl Inbox {
    fn pop(&mut self) -> Option<Message> {
        if std::mem::take(&mut self.stop_signal) {
            return Some(Message::StopSignal);
        }
        if let Some(reply) = self.stops.pop_front() {
            return Some(Message::Stop(reply));
        }
        if std::mem::take(&mut self.overflow) {
            return Some(Message::Overflow);
        }
        if self.released.is_some() && self.release_prefix > 0 {
            self.release_prefix -= 1;
            if let Some(message) = self.general.pop_front() {
                return Some(message);
            }
            self.release_prefix = 0;
        }
        if let Some(message) = self.released.take() {
            self.release_prefix = 0;
            self.expected_window = None;
            return Some(message);
        }
        if let Some(message) = self.cleanup.take() {
            self.expected_cleanup = None;
            return Some(message);
        }
        self.tick.take().or_else(|| self.general.pop_front())
    }
}

#[derive(Clone)]
pub struct Code0Proxy {
    inbox: Arc<Mutex<Inbox>>,
    view: Arc<Mutex<Code0SessionDto>>,
    schedule: Arc<dyn Code0Schedule>,
}
impl Code0Proxy {
    pub fn new(schedule: Arc<dyn Code0Schedule>, revision: Code0Revision) -> Self {
        Self {
            inbox: Arc::new(Mutex::new(Inbox::default())),
            view: Arc::new(Mutex::new(Code0SessionDto::empty(revision))),
            schedule,
        }
    }
    pub fn committed(&self) -> Code0SessionDto {
        self.view.lock().unwrap().clone()
    }
    pub fn read(&self, reply: Code0ReplySink) {
        let _ = self.enqueue(Message::Read(reply));
    }
    pub fn action(&self, action: Code0Action, reply: Code0ReplySink) {
        let _ = self.enqueue(Message::Action(action, reply));
    }
    pub fn stop(&self, reply: Code0ReplySink) {
        let _ = self.enqueue(Message::Stop(reply));
    }
    #[cfg(test)]
    pub(crate) fn begin_terminal_stop(&self) {
        let _ = self.enqueue(Message::StopSignal);
    }

    pub fn window(
        &self,
        binding: u64,
        request: Code0WindowRequest,
        event: Code0WindowEvent,
    ) -> Result<(), Code0DispatchError> {
        self.enqueue(Message::Window(binding, request, event))
    }
    pub fn cleanup(
        &self,
        binding: u64,
        request: Code0CleanupRequest,
        result: Result<(), Code0SessionFailure>,
    ) -> Result<(), Code0DispatchError> {
        self.enqueue(Message::Cleanup(binding, request, result))
    }
    pub fn tick(&self, binding: u64, tick: u64) -> Result<(), Code0DispatchError> {
        self.enqueue(Message::Tick(binding, tick))
    }
    /// Called by the UI-local driver before an owned create/cleanup can callback.
    /// Only one window and cleanup reservation can exist for this single owner.
    pub fn expect_window_release(
        &self,
        binding: u64,
        request: Code0WindowRequest,
    ) -> Result<(), Code0DispatchError> {
        let mut inbox = self.inbox.lock().unwrap();
        if inbox.binding.map(|value| value.0) != Some(binding) {
            return Err(Code0DispatchError::Unavailable);
        }
        inbox.expected_window = Some((binding, request));
        inbox.released = None;
        inbox.release_prefix = 0;
        Ok(())
    }
    pub fn expect_cleanup(
        &self,
        binding: u64,
        request: Code0CleanupRequest,
    ) -> Result<(), Code0DispatchError> {
        let mut inbox = self.inbox.lock().unwrap();
        if inbox.binding.map(|value| value.0) != Some(binding) {
            return Err(Code0DispatchError::Unavailable);
        }
        inbox.expected_cleanup = Some((binding, request));
        inbox.cleanup = None;
        Ok(())
    }

    fn enqueue(&self, message: Message) -> Result<(), Code0DispatchError> {
        let mut rejected = None;
        let needs_schedule = {
            let mut inbox = self.inbox.lock().unwrap();
            // Correlation filtering precedes coalescing/reservation changes.
            // Only the real UI driver registers identities; the session still
            // checks every epoch/attempt/document before granting anything.
            match &message {
                Message::Tick(binding, tick) => {
                    if inbox.binding != Some((*binding, *tick)) {
                        return Ok(());
                    }
                }
                Message::Window(binding, request, _) => {
                    if inbox.expected_window.as_ref() != Some(&(*binding, request.clone())) {
                        return Ok(());
                    }
                }
                Message::Cleanup(binding, request, _) => {
                    if inbox.expected_cleanup.as_ref() != Some(&(*binding, request.clone())) {
                        return Ok(());
                    }
                }
                Message::Action(_, _) if inbox.stopping => {
                    let error = inbox.terminal_error;
                    drop(inbox);
                    if let Message::Action(_, reply) = message {
                        reply(match error {
                            Some(error) => Err(error),
                            None => Ok(Code0Reply::Action(Code0SessionActionResult::Denied)),
                        });
                    }
                    return Ok(());
                }
                _ => {}
            }
            match message {
                Message::Tick(binding, tick) => inbox.tick = Some(Message::Tick(binding, tick)),
                Message::Window(binding, request, Code0WindowEvent::Released) => {
                    if inbox.released.is_none() {
                        inbox.release_prefix = inbox.general.len();
                        inbox.released = Some(Message::Window(
                            binding,
                            request,
                            Code0WindowEvent::Released,
                        ));
                    }
                }
                Message::Cleanup(binding, request, result) => {
                    // Keep the first physical completion for this reservation.
                    if inbox.cleanup.is_none() {
                        inbox.cleanup = Some(Message::Cleanup(binding, request, result));
                    }
                }
                #[cfg(test)]
                Message::StopSignal => {
                    inbox.stopping = true;
                    inbox.stop_signal = true;
                }
                Message::Stop(reply) => {
                    inbox.stopping = true;
                    inbox.stop_signal = true;
                    if inbox.stop_pending >= GENERAL_CAPACITY {
                        rejected = Some(Message::Stop(reply));
                    } else {
                        inbox.stop_pending += 1;
                        inbox.stops.push_back(reply);
                    }
                }
                message => {
                    if inbox.general.len() >= GENERAL_CAPACITY {
                        inbox.overflow = true;
                        rejected = Some(message);
                    } else {
                        inbox.general.push_back(message);
                    }
                }
            }
            if inbox.scheduled {
                false
            } else {
                inbox.scheduled = true;
                true
            }
        };
        let full = rejected.is_some();
        if let Some(message) = rejected {
            message.reject(Code0DispatchError::QueueFull);
        }
        if needs_schedule && self.schedule.schedule().is_err() {
            let rejected = {
                let mut inbox = self.inbox.lock().unwrap();
                let prefix = inbox.release_prefix;
                let mut retained_prefix = 0;
                let mut retained = VecDeque::new();
                let mut messages = Vec::new();
                for (index, message) in inbox.general.drain(..).enumerate() {
                    if matches!(message, Message::Window(..)) {
                        // Native evidence already queued remains meaningful
                        // even if the UI scheduling signal failed. Reject RPCs
                        // without losing/reordering a pre-release revocation.
                        if index < prefix {
                            retained_prefix += 1;
                        }
                        retained.push_back(message);
                    } else {
                        messages.push(message);
                    }
                }
                inbox.general = retained;
                inbox.release_prefix = retained_prefix;
                let stops: Vec<_> = inbox.stops.drain(..).collect();
                messages.extend(stops.into_iter().map(Message::Stop));
                // Physical acknowledgements and fail-closed control signals
                // survive a scheduling failure; no false release is reported.
                inbox.scheduled = false;
                messages
            };
            for message in rejected {
                if matches!(message, Message::Stop(_)) {
                    self.inbox.lock().unwrap().stop_pending -= 1;
                }
                message.reject(Code0DispatchError::Unavailable);
            }
            return Err(Code0DispatchError::Unavailable);
        }
        if full {
            Err(Code0DispatchError::QueueFull)
        } else {
            Ok(())
        }
    }
}

#[cfg(any(test, all(windows, debug_assertions, feature = "code0-native-fixture")))]
struct DrainObservation<'a> {
    active: &'a Cell<bool>,
    prior: bool,
}

#[cfg(any(test, all(windows, debug_assertions, feature = "code0-native-fixture")))]
impl Drop for DrainObservation<'_> {
    fn drop(&mut self) {
        self.active.set(self.prior);
    }
}

/// The actual pure session remains the only authorization authority, retained
/// locally. The only extra deadline is presentation-only throttling. This type
/// has no Send bound and is not stored in Tauri managed state.
pub struct Code0Dispatcher<C, D, S> {
    session: RefCell<Code0WebSession<C, D, S>>,
    clock: C,
    binding: u64,
    tick_id: u64,
    proxy: Code0Proxy,
    emit: RefCell<Code0Emitter>,
    retry_deadline: Cell<Option<Duration>>,
    stop_waiters: RefCell<Vec<Code0ReplySink>>,
    #[cfg(any(test, all(windows, debug_assertions, feature = "code0-native-fixture")))]
    drain_active: Cell<bool>,
}
impl<C: Code0SessionClock, D: Code0WindowDriving, S: Code0SnapshotStoring>
    Code0Dispatcher<C, D, S>
{
    pub fn install(
        binding: u64,
        tick_id: u64,
        session: Code0WebSession<C, D, S>,
        clock: C,
        proxy: Code0Proxy,
        emit: Box<dyn FnMut(&Code0SessionDto)>,
    ) -> Result<Self, Code0DispatchError> {
        let old = proxy.committed();
        let mut view = projection(&session, old.revision)?;
        if view != old {
            view.revision = Code0Revision::new(old.revision)?.next()?.0;
        }
        {
            let mut inbox = proxy.inbox.lock().unwrap();
            if inbox.binding.is_some() {
                return Err(Code0DispatchError::Unavailable);
            }
            inbox.binding = Some((binding, tick_id));
        }
        *proxy.view.lock().unwrap() = view;
        Ok(Self {
            session: RefCell::new(session),
            clock,
            binding,
            tick_id,
            proxy,
            emit: RefCell::new(emit),
            retry_deadline: Cell::new(None),
            stop_waiters: RefCell::new(Vec::new()),
            #[cfg(any(test, all(windows, debug_assertions, feature = "code0-native-fixture")))]
            drain_active: Cell::new(false),
        })
    }
    #[cfg(any(test, all(windows, debug_assertions, feature = "code0-native-fixture")))]
    pub(crate) fn terminal_prerequisites_met(&self) -> bool {
        if self.drain_active.get() {
            return false;
        }
        {
            let Ok(session) = self.session.try_borrow() else {
                return false;
            };
            if !session.terminal_stop_consumed() {
                return false;
            }
        }
        {
            let Ok(waiters) = self.stop_waiters.try_borrow() else {
                return false;
            };
            if !waiters.is_empty() {
                return false;
            }
        }
        let Ok(inbox) = self.proxy.inbox.try_lock() else {
            return false;
        };
        inbox.general.is_empty()
            && inbox.stops.is_empty()
            && inbox.stop_pending == 0
            && inbox.tick.is_none()
            && inbox.released.is_none()
            && inbox.cleanup.is_none()
            && !inbox.overflow
            && !inbox.stop_signal
            && inbox.expected_window.is_none()
            && inbox.expected_cleanup.is_none()
            && inbox.release_prefix == 0
            && !inbox.scheduled
    }

    pub fn drain(&self) {
        #[cfg(any(test, all(windows, debug_assertions, feature = "code0-native-fixture")))]
        let _drain_observation = DrainObservation {
            active: &self.drain_active,
            prior: self.drain_active.replace(true),
        };
        loop {
            let message = {
                let mut inbox = self.proxy.inbox.lock().unwrap();
                match inbox.pop() {
                    Some(message) => message,
                    None => {
                        inbox.scheduled = false;
                        return;
                    }
                }
            };
            self.apply(message);
        }
    }

    fn apply(&self, message: Message) {
        if let Message::Read(reply) = message {
            reply(Ok(Code0Reply::View(Box::new(self.proxy.committed()))));
            return;
        }
        let old = self.proxy.committed();
        let (stopping, terminal, window) = {
            let inbox = self.proxy.inbox.lock().unwrap();
            (
                inbox.stopping,
                inbox.terminal_error,
                inbox.expected_window.clone(),
            )
        };
        let exhausted = old.revision == MAX_REVISION;
        if exhausted {
            self.fail_closed(Code0DispatchError::RevisionExhausted);
        }
        let mut action_reply = None;
        let candidate = {
            // No inbox/view lock spans this unique borrow, driver calls or
            // callbacks. Driver callbacks only enqueue while scheduled=true.
            let mut session = self.session.borrow_mut();
            if exhausted {
                session.stop();
            }
            match message {
                Message::Action(action, reply) => {
                    let value =
                        if exhausted || terminal.is_some() {
                            Err(terminal.unwrap_or(Code0DispatchError::RevisionExhausted))
                        } else if stopping {
                            Ok(Code0Reply::Action(Code0SessionActionResult::Denied))
                        } else {
                            let result = match action {
                                Code0Action::Connect => session.connect(),
                                Code0Action::LoginCompleted => session.login_completed(),
                                Code0Action::SyncNow => session.sync_now(),
                                Code0Action::Disconnect => session.disconnect(),
                            };
                            match result {
                                Code0SessionActionResult::Throttled { seconds_remaining } => {
                                    self.retry_deadline.set(self.clock.now().checked_add(
                                        Duration::from_secs(seconds_remaining.min(300)),
                                    ));
                                }
                                Code0SessionActionResult::Started => self.retry_deadline.set(None),
                                _ => {}
                            }
                            Ok(Code0Reply::Action(result))
                        };
                    action_reply = Some((reply, value));
                }
                Message::Window(binding, request, event) if binding == self.binding => {
                    session.receive_window_event(&request, event)
                }
                Message::Cleanup(binding, request, result) if binding == self.binding => {
                    session.receive_cleanup_result(&request, result)
                }
                Message::Tick(binding, tick) if binding == self.binding && tick == self.tick_id => {
                    session.process_scheduled_work()
                }
                Message::Stop(reply) => {
                    self.stop_waiters.borrow_mut().push(reply);
                    self.retry_deadline.set(None);
                    session.stop();
                }
                Message::StopSignal => {
                    self.retry_deadline.set(None);
                    session.stop();
                }
                Message::Overflow => {
                    if let Some((binding, request)) = window
                        && binding == self.binding
                    {
                        session.receive_window_event(
                            &request,
                            Code0WindowEvent::Failed(Code0SessionFailure::PolicyRejected),
                        );
                    }
                }
                _ => {}
            }
            projection(&session, old.revision).map(|mut view| {
                view.retry_after_seconds = self.retry_remaining();
                view
            })
        }; // The session RefMut is dropped BEFORE commit, emit, and reply.
        let publication = candidate.and_then(|mut view| {
            if view != old {
                view.revision = Code0Revision::new(old.revision)?.next()?.0;
                *self.proxy.view.lock().unwrap() = view.clone();
                (self.emit.borrow_mut())(&view);
            }
            Ok(())
        });
        if let Err(error) = publication {
            self.fail_closed(error);
            self.session.borrow_mut().stop();
            if let Some((_, value)) = action_reply.as_mut() {
                *value = Err(error);
            }
        }
        if let Some((reply, value)) = action_reply {
            reply(value);
        }
        // Only actual pure-session Stop completion permits exit: Closing and
        // CleanupPending cannot supply this proof. Late owned acknowledgements
        // still reach the session even when no revision can be published.
        let safe = matches!(
            self.session.borrow().state(),
            Code0SessionState::Paused | Code0SessionState::Disconnected
        );
        if safe {
            let replies = std::mem::take(&mut *self.stop_waiters.borrow_mut());
            for reply in replies {
                // Release one admission immediately before its callback. A
                // callback can synchronously request another Stop; the other
                // undelivered replies must still count toward the bound.
                self.proxy.inbox.lock().unwrap().stop_pending -= 1;
                reply(Ok(Code0Reply::Stopped));
            }
        }
    }

    fn retry_remaining(&self) -> Option<u16> {
        let end = self.retry_deadline.get()?;
        let remaining = end.saturating_sub(self.clock.now());
        if remaining.is_zero() {
            self.retry_deadline.set(None);
            None
        } else {
            Some((remaining.as_secs() + u64::from(remaining.subsec_nanos() != 0)).min(300) as u16)
        }
    }

    fn fail_closed(&self, error: Code0DispatchError) {
        let mut inbox = self.proxy.inbox.lock().unwrap();
        inbox.terminal_error = Some(error);
        inbox.stopping = true;
    }
}

fn projection<C: Code0SessionClock, D: Code0WindowDriving, S: Code0SnapshotStoring>(
    session: &Code0WebSession<C, D, S>,
    revision: u64,
) -> Result<Code0SessionDto, Code0DispatchError> {
    let state = match session.state() {
        Code0SessionState::Disconnected => "disconnected",
        Code0SessionState::Connecting => "connecting",
        Code0SessionState::Syncing => "syncing",
        Code0SessionState::Closing => "closing",
        Code0SessionState::Active => "active",
        Code0SessionState::Paused => "paused",
        Code0SessionState::CleanupPending => "cleanupPending",
    };
    let failure = session.failure().map(|failure| match failure {
        Code0SessionFailure::ValidationRequired => "validationRequired",
        Code0SessionFailure::ProfileBusy => "profileBusy",
        Code0SessionFailure::StorageFailed => "storageFailed",
        Code0SessionFailure::CleanupFailed => "cleanupFailed",
        Code0SessionFailure::PolicyRejected => "policyRejected",
        Code0SessionFailure::NavigationFailed => "navigationFailed",
        Code0SessionFailure::InvalidResponse => "invalidResponse",
        Code0SessionFailure::AuthenticationChanged => "authenticationChanged",
        Code0SessionFailure::LoginTimedOut => "loginTimedOut",
        Code0SessionFailure::SyncTimedOut => "syncTimedOut",
    });
    Ok(Code0SessionDto {
        revision,
        state,
        failure,
        is_automatically_syncing: session.is_automatically_syncing(),
        retry_after_seconds: None,
        snapshot: session.snapshot().map(snapshot_projection).transpose()?,
    })
}
fn snapshot_projection(snapshot: &UsageSnapshot) -> Result<Code0SnapshotDto, Code0DispatchError> {
    use crate::domain::{MetricKind, MetricUnit, ProviderId, UsageStatus};
    let primary = snapshot
        .primary_metric
        .as_ref()
        .ok_or(Code0DispatchError::InvalidProjection)?;
    let secondary = snapshot
        .secondary_metric
        .as_ref()
        .ok_or(Code0DispatchError::InvalidProjection)?;
    if snapshot.provider_id != ProviderId::Code0
        || snapshot.schema_version != 1
        || primary.kind != MetricKind::Balance
        || secondary.kind != MetricKind::CumulativeUsage
        || primary.unit != MetricUnit::SiteCredits
        || secondary.unit != MetricUnit::SiteCredits
        || !primary.current.is_finite()
        || primary.current < 0.0
        || !secondary.current.is_finite()
        || secondary.current < 0.0
    {
        return Err(Code0DispatchError::InvalidProjection);
    }
    let status = match snapshot.status {
        UsageStatus::Fresh => "fresh",
        UsageStatus::Cached => "cached",
        _ => return Err(Code0DispatchError::InvalidProjection),
    };
    let metric = |label, current, kind| Code0MetricDto {
        label,
        current,
        limit: None,
        unit: "siteCredits",
        kind,
        reset_at: None,
        reset_description: None,
    };
    Ok(Code0SnapshotDto {
        schema_version: 1,
        provider_id: "code0",
        display_name: "Code0",
        status,
        used_ratio: None,
        primary_metric: metric("Balance", primary.current, "balance"),
        secondary_metric: metric("Cumulative usage", secondary.current, "cumulativeUsage"),
        fetched_at: snapshot.fetched_at.clone(),
        stale_after_seconds: 300,
        source_version: "website",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::{Rc, Weak};
    use std::time::Duration;

    use super::super::code0_web_session::Code0StoreError;
    use crate::collectors::code0_capture::Code0WebsiteCapture;

    struct Clock;
    impl Code0SessionClock for Clock {
        fn now(&self) -> Duration {
            Duration::ZERO
        }
        fn wall_now(&self) -> String {
            "2026-10-27T01:00:00Z".into()
        }
    }
    struct Driver;
    impl Code0WindowDriving for Driver {
        fn acquire_profile_lease(&mut self) -> Result<(), Code0SessionFailure> {
            Ok(())
        }
        fn release_profile_lease(&mut self) {}
        fn create_unloaded(&mut self, _: Code0WindowRequest) -> Result<(), Code0SessionFailure> {
            Ok(())
        }
        fn navigate(&mut self, _: &str) -> Result<(), Code0SessionFailure> {
            Ok(())
        }
        fn close(&mut self, _: &str) {}
        fn clear_website_data(
            &mut self,
            _: Code0CleanupRequest,
        ) -> Result<(), Code0SessionFailure> {
            Ok(())
        }
    }
    struct Store;
    impl Code0SnapshotStoring for Store {
        fn load(&self) -> Result<Option<UsageSnapshot>, Code0StoreError> {
            Ok(Some(
                Code0WebsiteCapture::decode_filtered_json(
                    br#"{"quota":1000000,"usedQuota":500000,"divisor":500000}"#,
                )
                .unwrap()
                .make_snapshot("2026-10-27T00:00:00Z")
                .unwrap(),
            ))
        }
        fn save_atomic(&mut self, _: &UsageSnapshot) -> Result<(), Code0StoreError> {
            Ok(())
        }
        fn remove(&mut self) -> Result<(), Code0StoreError> {
            Ok(())
        }
    }
    struct UiSchedule;
    impl Code0Schedule for UiSchedule {
        fn schedule(&self) -> Result<(), Code0DispatchError> {
            Ok(())
        }
    }
    type Owner = Code0Dispatcher<Clock, Driver, Store>;
    thread_local! {
        static OWNER: RefCell<Option<Weak<Owner>>> = const { RefCell::new(None) };
    }
    // This accesses the real private owner cell directly. A queued read would
    // not detect a borrow retained across a callback while scheduled stays true.
    fn assert_owner_unborrowed(boundary: &str) {
        OWNER.with(|slot| {
            let owner = slot.borrow().as_ref().unwrap().upgrade().unwrap();
            assert!(
                owner.session.try_borrow_mut().is_ok(),
                "owner borrow retained across {boundary}"
            );
        });
    }
    fn install() -> (Rc<Owner>, Code0Proxy, Rc<Cell<bool>>) {
        let proxy = Code0Proxy::new(Arc::new(UiSchedule), Code0Revision::new(0).unwrap());
        let emitted = Rc::new(Cell::new(false));
        let seen = emitted.clone();
        let session = Code0WebSession::new(Clock, Driver, Store, Box::new(|_| {}));
        let owner = Rc::new(
            Code0Dispatcher::install(
                7,
                11,
                session,
                Clock,
                proxy.clone(),
                Box::new(move |_| {
                    assert_owner_unborrowed("emit");
                    seen.set(true);
                }),
            )
            .unwrap(),
        );
        OWNER.with(|slot| *slot.borrow_mut() = Some(Rc::downgrade(&owner)));
        (owner, proxy, emitted)
    }

    // Mutation: emit while the actual session's RefMut guard is still live.
    #[test]
    fn emit_callback_observes_unborrowed_owner() {
        let (owner, proxy, emitted) = install();
        proxy.action(Code0Action::Disconnect, Box::new(|_| {}));
        owner.drain();
        assert!(
            emitted.get(),
            "state-changing action must reach the emit oracle"
        );
    }

    // Mutation: reply while the actual session's RefMut guard is still live.
    #[test]
    fn action_reply_callback_observes_unborrowed_owner() {
        let (owner, proxy, _) = install();
        let replied = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let seen = replied.clone();
        proxy.action(
            Code0Action::Disconnect,
            Box::new(move |value| {
                assert_owner_unborrowed("reply");
                assert_eq!(
                    value,
                    Ok(Code0Reply::Action(Code0SessionActionResult::Started))
                );
                seen.store(true, std::sync::atomic::Ordering::SeqCst);
            }),
        );
        owner.drain();
        assert!(replied.load(std::sync::atomic::Ordering::SeqCst));
    }
}

#[cfg(test)]
mod terminal_readiness_red_tests {
    use super::*;
    use std::rc::{Rc, Weak};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use super::super::code0_web_session::Code0StoreError;
    use crate::collectors::code0_capture::Code0WebsiteCapture;

    const BINDING: u64 = 31;
    const TICK: u64 = 47;

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
    }

    struct Driver {
        proxy: Code0Proxy,
        facts: Rc<RefCell<Facts>>,
    }

    impl Code0WindowDriving for Driver {
        fn acquire_profile_lease(&mut self) -> Result<(), Code0SessionFailure> {
            Ok(())
        }

        fn release_profile_lease(&mut self) {}

        fn create_unloaded(
            &mut self,
            request: Code0WindowRequest,
        ) -> Result<(), Code0SessionFailure> {
            self.proxy
                .expect_window_release(BINDING, request.clone())
                .expect("driver window reservation");
            self.facts.borrow_mut().windows.push(request);
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
            self.proxy
                .expect_cleanup(BINDING, request.clone())
                .expect("driver cleanup reservation");
            self.facts.borrow_mut().cleanups.push(request);
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

    #[derive(Clone, Copy, Debug)]
    struct ScheduleObservation {
        unlocked: bool,
        stop_pending: usize,
        stops: usize,
        stopping: bool,
        stop_signal: bool,
    }

    #[derive(Default)]
    struct Schedule {
        calls: AtomicUsize,
        fail: AtomicBool,
        inbox: Mutex<Option<std::sync::Weak<Mutex<Inbox>>>>,
        observations: Mutex<Vec<ScheduleObservation>>,
    }

    impl Code0Schedule for Schedule {
        fn schedule(&self) -> Result<(), Code0DispatchError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let inbox = self
                .inbox
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .upgrade()
                .unwrap();
            let observed = match inbox.try_lock() {
                Ok(inbox) => ScheduleObservation {
                    unlocked: true,
                    stop_pending: inbox.stop_pending,
                    stops: inbox.stops.len(),
                    stopping: inbox.stopping,
                    stop_signal: inbox.stop_signal,
                },
                Err(_) => ScheduleObservation {
                    unlocked: false,
                    stop_pending: usize::MAX,
                    stops: usize::MAX,
                    stopping: false,
                    stop_signal: false,
                },
            };
            self.observations.lock().unwrap().push(observed);
            if self.fail.load(Ordering::SeqCst) {
                Err(Code0DispatchError::Unavailable)
            } else {
                Ok(())
            }
        }
    }

    type Owner = Code0Dispatcher<Clock, Driver, Store>;
    type Replies = Arc<Mutex<Vec<(usize, Result<Code0Reply, Code0DispatchError>)>>>;

    thread_local! {
        static OWNER: RefCell<Option<Weak<Owner>>> = const { RefCell::new(None) };
    }

    fn with_owner<R>(visit: impl FnOnce(&Owner) -> R) -> R {
        let owner = OWNER.with(|slot| slot.borrow().as_ref().unwrap().upgrade().unwrap());
        visit(&owner)
    }

    fn sink(id: usize, replies: &Replies) -> Code0ReplySink {
        let replies = replies.clone();
        Box::new(move |value| replies.lock().unwrap().push((id, value)))
    }

    fn no_reply() -> Code0ReplySink {
        Box::new(|_| {})
    }

    struct Fixture {
        owner: Rc<Owner>,
        proxy: Code0Proxy,
        schedule: Arc<Schedule>,
        facts: Rc<RefCell<Facts>>,
    }

    impl Fixture {
        fn new() -> Self {
            let schedule = Arc::new(Schedule::default());
            let proxy = Code0Proxy::new(schedule.clone(), Code0Revision::new(0).unwrap());
            *schedule.inbox.lock().unwrap() = Some(Arc::downgrade(&proxy.inbox));
            let facts = Rc::new(RefCell::new(Facts::default()));
            let cache = Code0WebsiteCapture {
                balance: 2.0,
                cumulative_usage: 1.0,
            }
            .make_snapshot("2026-10-29T00:00:00Z")
            .expect("terminal dispatcher numeric cache");
            let session = Code0WebSession::new(
                Clock,
                Driver {
                    proxy: proxy.clone(),
                    facts: facts.clone(),
                },
                Store(Some(cache)),
                Box::new(|_| {}),
            );
            let owner = Rc::new(
                Code0Dispatcher::install(
                    BINDING,
                    TICK,
                    session,
                    Clock,
                    proxy.clone(),
                    Box::new(|_| {}),
                )
                .unwrap(),
            );
            OWNER.with(|slot| *slot.borrow_mut() = Some(Rc::downgrade(&owner)));
            Self {
                owner,
                proxy,
                schedule,
                facts,
            }
        }

        fn apply_one(&self) {
            let message = self
                .proxy
                .inbox
                .lock()
                .unwrap()
                .pop()
                .expect("queued stage");
            self.owner.apply(message);
        }

        fn denied(&self) {
            let replies = Replies::default();
            self.proxy.action(Code0Action::Connect, sink(0, &replies));
            assert_eq!(
                *replies.lock().unwrap(),
                [(0, Ok(Code0Reply::Action(Code0SessionActionResult::Denied)))]
            );
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            OWNER.with(|slot| *slot.borrow_mut() = None);
        }
    }

    // Coupled blockers are real retained states, not independent conjunct kills.
    #[test]
    fn terminal_dispatcher_retained_work_blocks_until_settled() {
        let f = Fixture::new();
        assert!(
            !f.owner.terminal_prerequisites_met(),
            "cold DTO is not ready"
        );
        f.proxy.action(Code0Action::SyncNow, no_reply());
        f.owner.drain();
        let window = f.facts.borrow().windows[0].clone();
        f.proxy.action(Code0Action::Disconnect, no_reply());
        f.owner.drain();
        let stops = Replies::default();
        f.proxy.stop(sink(0, &stops));
        {
            let inbox = f.proxy.inbox.lock().unwrap();
            assert!(inbox.stop_signal && inbox.scheduled);
            assert_eq!(inbox.stops.len(), 1);
            assert_eq!(inbox.stop_pending, 1);
            assert!(inbox.expected_window.is_some());
        }
        assert!(
            !f.owner.terminal_prerequisites_met(),
            "Stop admission is pending"
        );
        f.apply_one();
        assert!(
            !f.owner.terminal_prerequisites_met(),
            "queued Stop reply remains"
        );
        f.apply_one();
        assert_eq!(f.owner.stop_waiters.borrow().len(), 1);
        assert!(
            !f.owner.terminal_prerequisites_met(),
            "owner Stop waiter remains"
        );
        f.proxy.tick(BINDING, TICK).unwrap();
        for _ in 0..65 {
            f.proxy.read(no_reply());
        }
        f.proxy
            .window(BINDING, window, Code0WindowEvent::Released)
            .unwrap();
        {
            let inbox = f.proxy.inbox.lock().unwrap();
            assert_eq!(inbox.general.len(), 64);
            assert!(inbox.tick.is_some() && inbox.released.is_some() && inbox.overflow);
            assert_eq!(inbox.release_prefix, 64);
            assert!(inbox.expected_window.is_some() && inbox.scheduled);
        }
        assert!(
            !f.owner.terminal_prerequisites_met(),
            "inbox evidence remains"
        );
        f.owner.drain();
        let cleanup = f.facts.borrow().cleanups[0].clone();
        {
            let inbox = f.proxy.inbox.lock().unwrap();
            assert!(!inbox.scheduled && inbox.general.is_empty());
            assert!(inbox.expected_window.is_none());
            assert!(inbox.expected_cleanup.is_some());
            assert_eq!(inbox.stop_pending, 1);
        }
        assert!(
            !f.owner.terminal_prerequisites_met(),
            "cleanup reservation remains"
        );
        f.proxy.cleanup(BINDING, cleanup, Ok(())).unwrap();
        assert!(f.proxy.inbox.lock().unwrap().cleanup.is_some());
        assert!(
            !f.owner.terminal_prerequisites_met(),
            "cleanup result still queued"
        );
        f.owner.drain();
        assert_eq!(*stops.lock().unwrap(), [(0, Ok(Code0Reply::Stopped))]);
        assert!(f.owner.stop_waiters.borrow().is_empty());
        assert!(
            f.owner.terminal_prerequisites_met(),
            "TR-D1 settled consumed Stop and empty dispatcher must be ready"
        );
    }

    // Detects Stopped or a nested empty drain being mistaken for outer return.
    #[test]
    fn terminal_stopped_reply_and_nested_drain_are_not_settled() {
        let f = Fixture::new();
        let events = Arc::new(Mutex::new(Vec::new()));
        let seen = events.clone();
        let proxy = f.proxy.clone();
        f.proxy.stop(Box::new(move |value| {
            assert_eq!(value, Ok(Code0Reply::Stopped));
            with_owner(|owner| {
                assert!(!owner.terminal_prerequisites_met(), "inside Stopped reply");
            });
            seen.lock().unwrap().push("stopped");
            let nested_seen = seen.clone();
            proxy.read(Box::new(move |value| {
                assert!(matches!(value, Ok(Code0Reply::View(_))));
                with_owner(|owner| {
                    assert!(
                        !owner.terminal_prerequisites_met(),
                        "inside nested Read reply"
                    );
                });
                nested_seen.lock().unwrap().push("nested-read");
            }));
            with_owner(|owner| owner.drain());
            assert_eq!(*seen.lock().unwrap(), ["stopped", "nested-read"]);
            with_owner(|owner| {
                assert!(
                    !owner.terminal_prerequisites_met(),
                    "outer reply still active"
                );
            });
            seen.lock().unwrap().push("outer-reply-return");
        }));
        f.owner.drain();
        assert_eq!(
            *events.lock().unwrap(),
            ["stopped", "nested-read", "outer-reply-return"]
        );
        assert!(
            f.owner.terminal_prerequisites_met(),
            "TR-D2 only returned outer drain may be ready"
        );
    }

    // Detects implementing terminal Stop by allocating public Stop reply sinks.
    #[test]
    fn terminal_no_waiter_stop_fences_and_coalesces_without_sinks() {
        let f = Fixture::new();
        for _ in 0..65 {
            f.proxy.begin_terminal_stop();
        }
        f.denied();
        assert_eq!(f.schedule.calls.load(Ordering::SeqCst), 1);
        let inbox = f.proxy.inbox.lock().unwrap();
        assert!(inbox.stopping && inbox.stop_signal && inbox.scheduled);
        assert!(inbox.general.is_empty());
        assert!(f.owner.stop_waiters.borrow().is_empty());
        assert_eq!(
            inbox.stop_pending, 0,
            "TR-D3 terminal Stop must allocate no reply admissions"
        );
        assert!(
            inbox.stops.is_empty(),
            "terminal Stop must allocate no reply sinks"
        );
    }

    // Detects loss of Stop fence/evidence on failed external scheduling.
    #[test]
    fn terminal_failed_schedule_retains_fence_without_waiter() {
        let f = Fixture::new();
        f.schedule.fail.store(true, Ordering::SeqCst);
        f.proxy.begin_terminal_stop();
        f.denied();
        {
            let inbox = f.proxy.inbox.lock().unwrap();
            assert!(inbox.stopping && inbox.stop_signal);
            assert!(!inbox.scheduled && inbox.stops.is_empty());
            assert_eq!(inbox.stop_pending, 0);
        }
        let observed = f.schedule.observations.lock().unwrap()[0];
        assert!(observed.unlocked && observed.stopping && observed.stop_signal);
        assert_eq!(f.schedule.calls.load(Ordering::SeqCst), 1);
        f.schedule.fail.store(false, Ordering::SeqCst);
        f.owner.drain();
        assert!(!f.proxy.inbox.lock().unwrap().stop_signal);
        assert_eq!(
            f.owner.session.borrow_mut().connect(),
            Code0SessionActionResult::Denied
        );
        assert_eq!(
            observed.stop_pending, 0,
            "TR-D4 terminal scheduling must observe no reply admission"
        );
        assert_eq!(
            observed.stops, 0,
            "terminal scheduling must observe no reply sink"
        );
    }

    // Control: the existing public Stop bound and reentrant admission stay exact.
    #[test]
    fn terminal_public_stop_capacity_and_reentry_control() {
        let f = Fixture::new();
        let replies = Replies::default();
        let seen = replies.clone();
        let proxy = f.proxy.clone();
        f.proxy.stop(Box::new(move |value| {
            seen.lock().unwrap().push((0, value));
            proxy.stop(sink(65, &seen));
            assert_eq!(proxy.inbox.lock().unwrap().stop_pending, 64);
        }));
        for id in 1..65 {
            f.proxy.stop(sink(id, &replies));
        }
        assert_eq!(
            *replies.lock().unwrap(),
            [(64, Err(Code0DispatchError::QueueFull))]
        );
        assert_eq!(f.proxy.inbox.lock().unwrap().stop_pending, 64);
        f.owner.drain();
        let mut actual = replies.lock().unwrap().clone();
        actual.sort_by_key(|(id, _)| *id);
        assert_eq!(actual.len(), 66);
        for (expected_id, (id, value)) in actual.into_iter().enumerate() {
            assert_eq!(id, expected_id, "public Stop reply must occur exactly once");
            assert_eq!(
                value,
                if id == 64 {
                    Err(Code0DispatchError::QueueFull)
                } else {
                    Ok(Code0Reply::Stopped)
                },
                "TR-D5 public Stop reply {id}"
            );
        }
        assert_eq!(f.proxy.inbox.lock().unwrap().stop_pending, 0);
        assert!(f.owner.stop_waiters.borrow().is_empty());
    }
}
