//! RED checkpoint: bounded transport and a narrow read projection only.
//! Actions/native completions/ticks/Stop deliberately do not drive the session.
//! No product command, UI registry, native adapter or scheduler is installed.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

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
    tick: Option<Message>,
    released: Option<Message>,
    cleanup: Option<Message>,
    // Transport correlation/reservations only; these never grant authority.
    expected_window: Option<(u64, Code0WindowRequest)>,
    expected_cleanup: Option<(u64, Code0CleanupRequest)>,
    scheduled: bool,
}
impl Inbox {
    fn pop(&mut self) -> Option<Message> {
        self.general
            .pop_front()
            .or_else(|| self.released.take())
            .or_else(|| self.cleanup.take())
            .or_else(|| self.tick.take())
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
        inbox.expected_window = Some((binding, request));
        inbox.released = None;
        Ok(())
    }
    pub fn expect_cleanup(
        &self,
        binding: u64,
        request: Code0CleanupRequest,
    ) -> Result<(), Code0DispatchError> {
        let mut inbox = self.inbox.lock().unwrap();
        inbox.expected_cleanup = Some((binding, request));
        inbox.cleanup = None;
        Ok(())
    }

    fn enqueue(&self, message: Message) -> Result<(), Code0DispatchError> {
        let needs_schedule = {
            let mut inbox = self.inbox.lock().unwrap();
            match &message {
                Message::Tick(_, _) => inbox.tick = Some(message),
                Message::Window(binding, request, Code0WindowEvent::Released) => {
                    if inbox.expected_window.as_ref() != Some(&(*binding, request.clone())) {
                        return Ok(());
                    }
                    inbox.released = Some(message);
                }
                Message::Cleanup(binding, request, _) => {
                    if inbox.expected_cleanup.as_ref() != Some(&(*binding, request.clone())) {
                        return Ok(());
                    }
                    inbox.cleanup = Some(message);
                }
                _ => {
                    if inbox.general.len() >= GENERAL_CAPACITY {
                        drop(inbox);
                        message.reject(Code0DispatchError::QueueFull);
                        return Err(Code0DispatchError::QueueFull);
                    }
                    inbox.general.push_back(message);
                }
            }
            if inbox.scheduled {
                false
            } else {
                inbox.scheduled = true;
                true
            }
        };
        if needs_schedule && self.schedule.schedule().is_err() {
            let rejected = {
                let mut inbox = self.inbox.lock().unwrap();
                let mut messages = Vec::new();
                while let Some(message) = inbox.pop() {
                    messages.push(message);
                }
                inbox.scheduled = false;
                messages
            };
            for message in rejected {
                message.reject(Code0DispatchError::Unavailable);
            }
            return Err(Code0DispatchError::Unavailable);
        }
        Ok(())
    }
}

/// The actual pure session remains the only authorization authority, retained
/// locally even in this intentionally deny-only RED implementation. This type
/// has no Send bound and is not stored in Tauri managed state.
pub struct Code0Dispatcher<C, D, S> {
    session: RefCell<Code0WebSession<C, D, S>>,
    _clock: C,
    binding: u64,
    tick_id: u64,
    proxy: Code0Proxy,
    _emit: Box<dyn FnMut(&Code0SessionDto)>,
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
        *proxy.view.lock().unwrap() = view;
        Ok(Self {
            session: RefCell::new(session),
            _clock: clock,
            binding,
            tick_id,
            proxy,
            _emit: emit,
        })
    }
    pub fn drain(&self) {
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
            // No queue/view lock spans the owner borrow, scheduling or reply.
            // RED: no action, callback, timer or Stop is applied to this session.
            let reply = {
                let _session = self.session.borrow_mut();
                match message {
                    Message::Read(reply) => Some((
                        reply,
                        Ok(Code0Reply::View(Box::new(self.proxy.committed()))),
                    )),
                    Message::Action(_action, reply) => Some((
                        reply,
                        Ok(Code0Reply::Action(Code0SessionActionResult::Denied)),
                    )),
                    Message::Stop(reply) => Some((reply, Err(Code0DispatchError::Unavailable))),
                    Message::Window(binding, _request, _event) if binding == self.binding => None,
                    Message::Cleanup(binding, _request, _result) if binding == self.binding => None,
                    Message::Tick(binding, tick)
                        if binding == self.binding && tick == self.tick_id =>
                    {
                        None
                    }
                    _ => None,
                }
            };
            if let Some((reply, value)) = reply {
                reply(value);
            }
        }
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
