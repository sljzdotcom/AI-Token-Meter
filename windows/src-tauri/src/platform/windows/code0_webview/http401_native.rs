// Selected owned HTTP401 native adapter. All new tests/native behavior UNRUN.
use super::super::http401_protocol::{Http401Protocol, ReceiveStatus, SendOutcome};
use super::*;
use serde::Serialize;
use std::time::{Duration, Instant};
use webview2_com::{NavigationCompletedEventHandler, WebResourceRequestedEventHandler};
use windows::Win32::System::Com::STREAM_SEEK_SET;
use windows::Win32::UI::Shell::SHCreateMemStream;
use windows::core::BOOL;
use windows_sys::Win32::Networking::WinSock as ws;

// All potentially blocking writes/diagnostics complete while the caller's
// original watchdog remains armed. The final check is memory-only after join.
pub fn finalize_http401_with_watchdog(
    write: impl FnOnce() -> bool,
    stop_and_join: impl FnOnce() -> bool,
    final_check: impl FnOnce() -> bool,
) -> bool {
    let written = write();
    let joined = stop_and_join();
    let timely = final_check();
    written && joined && timely
}
pub fn record_http401_session_failure(facts: &mut Facts) {
    if !bump(&mut facts.session.failures, 1) {
        facts.overflow = true;
    }
}
// Join independently observed fixture facts with a later native snapshot.
// Session overflow is a sticky fixture fact as well as a native fact.
pub fn merge_http401_native_facts(facts: &mut Facts, native: &Facts) {
    let session = facts.session.clone();
    let exited = facts.cleanup.browser_process_exited_once;
    let failure = facts.failure;
    let overflow = facts.overflow;
    *facts = native.clone();
    facts.session = session;
    facts.cleanup.browser_process_exited_once = exited;
    facts.overflow |= overflow;
    if failure.is_some() {
        facts.failure = failure;
    }
}
pub(super) fn dispatch_child<T>(
    selected: bool,
    restrictive: impl FnOnce() -> T,
    existing: impl FnOnce() -> T,
) -> T {
    if selected { restrictive() } else { existing() }
}
pub(super) fn restrict_child<T>(revoke: impl FnOnce(), cancel: impl FnOnce() -> T) -> T {
    revoke();
    cancel()
}
fn record_first_resource_metadata(
    resource: &mut Resource,
    first: bool,
    exact: Option<bool>,
    qualified: Option<bool>,
) {
    if first {
        resource.exact_document_get = exact;
        resource.qualification_matched = qualified;
    }
}

pub const SELECTOR: &str = "refusal-http401-owned-loopback";
macro_rules! record {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Default, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct $name { $(pub $field: $ty),* }
    };
}
// Explicit wire spelling; never derive a field called finalCount.
#[derive(Clone, Debug, Default, Serialize)]
pub struct ReceiptCounts {
    pub target: u16,
    #[serde(rename = "final")]
    pub final_receipt: u16,
}
record!(Binding {request_matched:Option<bool>,generation_matched:Option<bool>,core_matched:Option<bool>,s2_install_matched:Option<bool>,guard_install_matched:Option<bool>,post_drop_current:Option<bool>});
record!(Install {core22_query_succeeded:Option<bool>,guard_adds:u16,guard_removes:u16,filter_adds:u16,filter_removes:u16,completed_adds:u16,completed_removes:u16,handlers_drained:Option<bool>});
record!(Navigation {attempts:u16,top_events:u16,grants:u16,hresult_succeeded:Option<bool>,qualification_matched:Option<bool>,navigation_id_readable:Option<bool>,cancel_false:Option<bool>});
record!(Resource {total:u16,passed:u16,refused:u16,exact_document_get:Option<bool>,qualification_matched:Option<bool>});
record!(Server {startup_succeeded:Option<bool>,version22:Option<bool>,exclusive_ack:Option<bool>,listener_linger_off:Option<bool>,listener_nonblocking:Option<bool>,bound_endpoint_matched:Option<bool>,listen_succeeded:Option<bool>,accepted_local_matched:Option<bool>,accepted_peer_matched:Option<bool>,stream_linger_off:Option<bool>,stream_nonblocking:Option<bool>,request_valid:Option<bool>,response_complete:Option<bool>,final_inventory_observed:Option<bool>,startup_attempts:u16,socket_creates:u16,binds:u16,listens:u16,accepts:u16,parsed_requests:u16,listener_close_attempts:u16,listener_close_successes:u16,stream_close_attempts:u16,stream_close_successes:u16,cleanup_attempts:u16,cleanup_successes:u16,request_bytes:u16,header_count:u16,sent_bytes:u16,setup_calls:u16,accepted_setup_calls:u16,io_attempts:u16,sockets_remaining:u16,startup_remaining:u16,server_threads:u16,server_joins:u16});
record!(Auth {callbacks:u16,cancel_attempts:u16,cancel_successes:u16,cancel_readbacks:u16,cancel_true:Option<bool>,uri_matched:Option<bool>,challenge_matched:Option<bool>,sender_matched:Option<bool>,post_drop_current:Option<bool>,within_attempt_deadline:Option<bool>});
record!(EarlyProfile {sender_matched:Option<bool>,navigation_id_matched:Option<bool>,args2_query_succeeded:Option<bool>,is_success:Option<bool>,http_status_code:Option<i32>,web_error_status:Option<&'static str>,full_wire_before_callback:Option<bool>,live_at_entry:Option<bool>,post_drop_current:Option<bool>});
impl EarlyProfile {
    fn positive(&self) -> bool {
        self.sender_matched == Some(true)
            && self.navigation_id_matched == Some(true)
            && self.args2_query_succeeded == Some(true)
            && self.is_success == Some(false)
            && self.http_status_code == Some(401)
            && self.web_error_status == Some("validAuthenticationCredentialsRequired")
            && self.full_wire_before_callback == Some(true)
            && self.live_at_entry == Some(true)
            && self.post_drop_current == Some(true)
    }
}
record!(Early {count:u16,profile:Option<EarlyProfile>});
record!(Session {failures:u16,reason:Option<&'static str>,request_matched:Option<bool>,action_before_failure:Option<bool>});
record!(Cleanup {authority_revoked:Option<bool>,actions_drained:Option<bool>,native_inventory_drained:Option<bool>,browser_process_exited_once:Option<bool>,first_close_adopted:Option<bool>,within_close_deadline:Option<bool>,temp_dir_close_count:u16,target_at_ms:Option<u64>,first_close_at_ms:Option<u64>,final_at_ms:Option<u64>,cleanup_elapsed_ms:Option<u64>});
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Facts {
    pub version: u8,
    pub selector: &'static str,
    pub phase: &'static str,
    pub unknown: bool,
    pub overflow: bool,
    pub receipts: ReceiptCounts,
    pub binding: Binding,
    pub install: Install,
    pub navigation: Navigation,
    pub resource: Resource,
    pub server: Server,
    pub auth: Auth,
    pub early401: Early,
    pub session: Session,
    pub cleanup: Cleanup,
    #[serde(skip)]
    pub failure: Option<&'static str>,
}
impl Default for Facts {
    fn default() -> Self {
        Self {
            version: 1,
            selector: SELECTOR,
            phase: "Preparing",
            unknown: false,
            overflow: false,
            receipts: ReceiptCounts::default(),
            binding: Binding::default(),
            install: Install::default(),
            navigation: Navigation::default(),
            resource: Resource::default(),
            server: Server::default(),
            auth: Auth::default(),
            early401: Early::default(),
            session: Session::default(),
            cleanup: Cleanup::default(),
            failure: None,
        }
    }
}
impl Facts {
    pub fn native_positive(&self) -> bool {
        let b = &self.binding;
        let i = &self.install;
        let n = &self.navigation;
        let r = &self.resource;
        let s = &self.server;
        let a = &self.auth;
        !self.unknown
            && !self.overflow
            && self.failure.is_none()
            && self.receipts.target == 1
            && self.receipts.final_receipt == 1
            && [
                b.request_matched,
                b.generation_matched,
                b.core_matched,
                b.s2_install_matched,
                b.guard_install_matched,
                b.post_drop_current,
                i.core22_query_succeeded,
                i.handlers_drained,
                n.hresult_succeeded,
                n.qualification_matched,
                n.navigation_id_readable,
                n.cancel_false,
                r.exact_document_get,
                r.qualification_matched,
                s.startup_succeeded,
                s.version22,
                s.exclusive_ack,
                s.listener_linger_off,
                s.listener_nonblocking,
                s.bound_endpoint_matched,
                s.listen_succeeded,
                s.accepted_local_matched,
                s.accepted_peer_matched,
                s.stream_linger_off,
                s.stream_nonblocking,
                s.request_valid,
                s.response_complete,
                s.final_inventory_observed,
                a.cancel_true,
                a.uri_matched,
                a.challenge_matched,
                a.sender_matched,
                a.post_drop_current,
                a.within_attempt_deadline,
            ]
            .into_iter()
            .all(|v| v == Some(true))
            && [
                i.guard_adds,
                i.guard_removes,
                i.filter_adds,
                i.filter_removes,
                i.completed_adds,
                i.completed_removes,
                n.attempts,
                n.top_events,
                n.grants,
                r.total,
                r.passed,
                a.callbacks,
                a.cancel_attempts,
                a.cancel_successes,
                a.cancel_readbacks,
                s.startup_attempts,
                s.socket_creates,
                s.binds,
                s.listens,
                s.accepts,
                s.parsed_requests,
                s.listener_close_attempts,
                s.listener_close_successes,
                s.stream_close_attempts,
                s.stream_close_successes,
                s.cleanup_attempts,
                s.cleanup_successes,
            ]
            .into_iter()
            .all(|v| v == 1)
            && r.refused == 0
            && s.request_bytes > 0
            && s.request_bytes <= 4096
            && s.header_count > 0
            && s.header_count <= 32
            && s.sent_bytes == 139
            && s.setup_calls == 9
            && s.accepted_setup_calls == 4
            && (3..=2048).contains(&s.io_attempts)
            && s.sockets_remaining == 0
            && s.startup_remaining == 0
            && s.server_threads == 0
            && s.server_joins == 0
            && match self.early401.count {
                0 => self.early401.profile.is_none(),
                1 => self
                    .early401
                    .profile
                    .as_ref()
                    .is_some_and(EarlyProfile::positive),
                _ => false,
            }
    }
    pub fn positive(&self) -> bool {
        self.native_positive()
            && self.phase == "Closed"
            && self.session.failures == 1
            && self.session.reason == Some("PolicyRejected")
            && self.session.request_matched == Some(true)
            && self.session.action_before_failure == Some(true)
            && self.cleanup.temp_dir_close_count == 1
            && [
                self.cleanup.authority_revoked,
                self.cleanup.actions_drained,
                self.cleanup.native_inventory_drained,
                self.cleanup.browser_process_exited_once,
                self.cleanup.first_close_adopted,
                self.cleanup.within_close_deadline,
            ]
            .into_iter()
            .all(|v| v == Some(true))
            && self.cleanup.target_at_ms.is_some_and(|v| v < 45000)
            && self.cleanup.first_close_at_ms.is_some()
            && self.cleanup.final_at_ms.is_some()
            && self.cleanup.cleanup_elapsed_ms.is_some()
    }
}
#[derive(Clone)]
pub struct Timing {
    started: Instant,
    end: Option<Instant>,
    first_close: Option<Instant>,
    target: Option<Instant>,
}
pub struct FinalClock {
    pub target: Option<u64>,
    pub close: Option<u64>,
    pub final_at: Option<u64>,
    pub elapsed: Option<u64>,
    pub within_close: bool,
}
impl Timing {
    fn new(started: Instant) -> Self {
        Self {
            started,
            end: started.checked_add(Duration::from_secs(45)),
            first_close: None,
            target: None,
        }
    }
    fn before(&self, now: Instant) -> bool {
        now >= self.started && self.end.is_some_and(|end| now < end)
    }
    fn close(&mut self, at: Instant) {
        if self.first_close.is_none() {
            self.first_close = Some(at);
        }
    }
    pub fn first_close(&self) -> Option<Instant> {
        self.first_close
    }
    pub fn matches_start(&self, started: Instant) -> bool {
        self.started == started
    }
    pub fn final_clock(&self, now: Instant) -> FinalClock {
        let ms = |at: Instant| {
            at.checked_duration_since(self.started)
                .map(|v| v.as_millis().min(u128::from(u64::MAX)) as u64)
        };
        FinalClock {
            target: self.target.and_then(ms),
            close: self.first_close.and_then(ms),
            final_at: ms(now),
            elapsed: self
                .first_close
                .and_then(|at| now.checked_duration_since(at))
                .map(|v| v.as_millis().min(u128::from(u64::MAX)) as u64),
            within_close: self.first_close.is_some_and(|at| {
                now >= at
                    && at
                        .checked_add(Duration::from_secs(10))
                        .is_some_and(|end| now < end)
            }),
        }
    }
}
pub struct Receipt {
    request: Code0WindowRequest,
    installation: Arc<AtomicBool>,
    guard: Arc<AtomicBool>,
    facts: Facts,
    timing: Timing,
}
impl Receipt {
    pub fn matches(
        &self,
        request: &Code0WindowRequest,
        install: &super::super::fixture::NativeRefusalInstallReceipt,
    ) -> bool {
        &self.request == request
            && install.matches(request)
            && Arc::ptr_eq(&self.installation, &install.installation)
    }
    pub fn same_guard(&self, previous: &Self) -> bool {
        self.request == previous.request && Arc::ptr_eq(&self.guard, &previous.guard)
    }
    pub fn facts(&self) -> &Facts {
        &self.facts
    }
    pub fn timing(&self) -> Timing {
        self.timing.clone()
    }
}
#[derive(Default)]
struct ServiceBudget {
    last: Option<Instant>,
    io: u16,
}
impl ServiceBudget {
    fn reserve(&mut self, now: Instant, io: bool) -> Result<bool, ()> {
        if self.last.is_some_and(|last| {
            now.checked_duration_since(last)
                .is_none_or(|v| v < Duration::from_millis(25))
        }) {
            return Ok(false);
        }
        if io && self.io >= 2048 {
            return Err(());
        }
        self.last = Some(now);
        if io {
            self.io += 1;
        }
        Ok(true)
    }
}
struct SocketLease {
    handle: ws::SOCKET,
    _id: u64,
    close_attempted: bool,
    closed: bool,
}
impl SocketLease {
    fn new(handle: ws::SOCKET, id: u64) -> Self {
        Self {
            handle,
            _id: id,
            close_attempted: false,
            closed: false,
        }
    }
    fn owned(&self) -> bool {
        !self.closed
    }
    fn reserve_close(&mut self) -> Option<ws::SOCKET> {
        if self.close_attempted || self.closed {
            None
        } else {
            self.close_attempted = true;
            Some(self.handle)
        }
    }
    fn close_returned(&mut self, success: bool) {
        if self.close_attempted && success {
            self.closed = true;
        }
    }
}
#[derive(Default)]
struct Allowance {
    reserved: bool,
    committed: bool,
    revoked: bool,
}
impl Allowance {
    fn reserve(&mut self, live: bool) -> bool {
        if !live || self.reserved || self.revoked {
            false
        } else {
            self.reserved = true;
            true
        }
    }
    fn commit(&mut self, live: bool) -> bool {
        if !live || self.revoked || !self.reserved || self.committed {
            self.revoked = true;
            false
        } else {
            self.committed = true;
            true
        }
    }
    fn revoke(&mut self) {
        self.revoked = true;
    }
}
fn restrictive_pair<S, G, A, B>(set: S, get: G) -> (A, B)
where
    S: FnOnce() -> A,
    G: FnOnce() -> B,
{
    (set(), get())
}
fn bump(value: &mut u16, cap: u16) -> bool {
    *value = value.saturating_add(1).min(cap + 1);
    *value <= cap
}

#[derive(Clone)]
pub(super) struct Context {
    pub(super) owner: Weak<RefCell<Owner>>,
    pub(super) callbacks: Arc<Callbacks>,
    pub(super) ui: ThreadId,
    request: Code0WindowRequest,
    generation: u64,
    installation: Arc<AtomicBool>,
    selected: Arc<AtomicBool>,
    core: Weak<ICoreWebView2>,
}
impl Context {
    pub(super) fn new(owner: &Rc<RefCell<Owner>>, request: &Code0WindowRequest) -> Self {
        let o = owner.borrow();
        let i = o.instance.as_ref().unwrap();
        Self {
            owner: Rc::downgrade(owner),
            callbacks: Arc::clone(&i.callbacks),
            ui: o.ui_thread,
            request: request.clone(),
            generation: i.refusal_registry.http_generation(),
            installation: Arc::clone(&i.refusal_registry.unknown),
            selected: Arc::clone(&i.http401_selected),
            core: Rc::downgrade(&i.refusal_registry.http_core().unwrap()),
        }
    }
    pub(super) fn selected(&self) -> bool {
        self.selected.load(Ordering::SeqCst)
    }
    fn same(&self, owner: &Rc<RefCell<Owner>>) -> bool {
        let o = owner.borrow();
        o.instance.as_ref().is_some_and(|i| {
            i.request == self.request
                && i.refusal_registry.http_generation() == self.generation
                && Arc::ptr_eq(&i.callbacks, &self.callbacks)
                && Arc::ptr_eq(&i.refusal_registry.unknown, &self.installation)
                && Arc::ptr_eq(&i.http401_selected, &self.selected)
                && i.http401.as_ref().is_some_and(|s| {
                    s.core
                        .as_ref()
                        .zip(self.core.upgrade().as_ref())
                        .is_some_and(|(a, b)| Rc::ptr_eq(a, b))
                })
        })
    }
    fn qualified(&self, owner: &Rc<RefCell<Owner>>, live: bool, now: Instant) -> bool {
        if !self.same(owner) {
            return false;
        }
        let o = owner.borrow();
        let i = o.instance.as_ref().unwrap();
        let s = i.http401.as_ref().unwrap();
        !o.unmatched
            && !i.failed
            && !i.released
            && !i.destroyed
            && !i.cleanup_blocked
            && s.facts.failure.is_none()
            && i.refusal_registry.complete()
            && s.resource_token.is_some()
            && s.completed_token.is_some()
            && s.filter
            && !self.callbacks.affinity_failed.load(Ordering::SeqCst)
            && !self.installation.load(Ordering::SeqCst)
            && !s.guard.load(Ordering::SeqCst)
            && s.timing.before(now)
            && s.timing.first_close.is_none_or(|at| {
                at.checked_add(Duration::from_secs(10))
                    .is_some_and(|end| now < end)
            })
            && (!live || (!i.closing && !i.failed && !s.closing && s.ready()))
    }
    fn sender(&self, sender: Option<&ICoreWebView2>) -> bool {
        let Some(sender) = sender else { return false };
        let Some(core) = self.core.upgrade() else {
            return false;
        };
        let expected = core.cast::<IUnknown>();
        let actual = sender.cast::<IUnknown>();
        let good = expected
            .as_ref()
            .ok()
            .zip(actual.as_ref().ok())
            .is_some_and(|(a, b)| a.as_raw() == b.as_raw());
        drop(actual);
        drop(expected);
        good
    }
    pub(super) fn lost(&self) {
        self.installation.store(true, Ordering::SeqCst);
    }
}
pub(super) struct State {
    ctx: Context,
    guard: Arc<AtomicBool>,
    facts: Facts,
    timing: Timing,
    core: Option<Rc<ICoreWebView2>>,
    core22: Option<Rc<ICoreWebView2_22>>,
    environment: Option<Rc<ICoreWebView2Environment>>,
    resource_token: Option<i64>,
    resource_add_attempted: bool,
    resource_remove_attempted: bool,
    completed_token: Option<i64>,
    completed_add_attempted: bool,
    completed_remove_attempted: bool,
    filter: bool,
    filter_attempted: bool,
    filter_remove_attempted: bool,
    listener: Option<SocketLease>,
    stream: Option<SocketLease>,
    startup: bool,
    startup_cleanup_attempted: bool,
    setup: u8,
    accepted_setup: u8,
    budget: ServiceBudget,
    in_action: bool,
    closing: bool,
    close_published: bool,
    parser: Option<Http401Protocol>,
    received: usize,
    port: Option<u16>,
    url: Option<String>,
    nav_id: Option<u64>,
    top: Allowance,
    resource: Allowance,
    navigate_reserved: bool,
    basic_reserved: bool,
}
impl State {
    fn ready(&self) -> bool {
        self.resource_add_attempted
            && self.completed_add_attempted
            && self.filter_attempted
            && self.facts.install.guard_adds == 1
            && self.facts.install.filter_adds == 1
            && self.facts.install.completed_adds == 1
            && self.core22.is_some()
            && self.environment.is_some()
            && self.setup == 9
    }
    fn revoke(&mut self, at: Instant) {
        self.timing.close(at);
        self.closing = true;
        self.top.revoke();
        self.resource.revoke();
        self.facts.phase = "Revoked";
        self.facts.cleanup.authority_revoked = Some(true);
        drop(self.parser.take());
    }
    fn fail(&mut self, reason: &'static str, at: Instant, unknown: bool) {
        if self.facts.failure.is_none() {
            self.facts.failure = Some(reason);
        }
        self.facts.unknown |= unknown;
        if unknown {
            self.guard.store(true, Ordering::SeqCst);
        }
        self.revoke(at);
    }
    fn inventory(&mut self) {
        self.facts.server.sockets_remaining = self.listener.as_ref().is_some_and(SocketLease::owned)
            as u16
            + self.stream.as_ref().is_some_and(SocketLease::owned) as u16;
        self.facts.server.startup_remaining = u16::from(self.startup);
    }
    fn quiescent(&self) -> bool {
        !self.in_action
            && self.parser.is_none()
            && !self.startup
            && self.listener.as_ref().is_none_or(|v| !v.owned())
            && self.stream.as_ref().is_none_or(|v| !v.owned())
            && self.resource_token.is_none()
            && self.completed_token.is_none()
            && !self.filter
            && self.core.is_none()
            && self.core22.is_none()
            && self.environment.is_none()
    }
}
pub(super) fn mark_close(i: &mut Instance) {
    if i.closing
        && let Some(s) = i.http401.as_mut()
    {
        if i.failed && s.facts.receipts.target == 0 {
            s.fail("http401Precondition", Instant::now(), false);
        } else {
            s.revoke(Instant::now());
        }
    }
}
fn fail(owner: &Rc<RefCell<Owner>>, reason: &'static str, unknown: bool) {
    let mut o = owner.borrow_mut();
    if let Some(i) = o.instance.as_mut()
        && let Some(s) = i.http401.as_mut()
    {
        s.fail(reason, Instant::now(), unknown);
        i.failed = true;
        i.closing = true;
        if unknown {
            i.cleanup_blocked = true;
        }
    }
}
pub(super) fn publish_close(owner: &Rc<RefCell<Owner>>) {
    let notification = {
        let mut o = owner.borrow_mut();
        let Some(i) = o.instance.as_mut() else { return };
        let Some(s) = i.http401.as_mut() else { return };
        if s.close_published || s.timing.first_close.is_none() {
            None
        } else {
            s.close_published = true;
            i.observer.clone().zip(s.timing.first_close)
        }
    };
    if let Some((observer, at)) = notification {
        observer(super::super::fixture::NativeSmokeObservation::Http401Closing { at });
    }
}
fn emit(owner: &Rc<RefCell<Owner>>) {
    let notification = {
        let o = owner.borrow();
        let Some(i) = o.instance.as_ref() else { return };
        let Some(s) = i.http401.as_ref() else { return };
        i.observer.clone().map(|observer| {
            (
                observer,
                Receipt {
                    request: i.request.clone(),
                    installation: Arc::clone(&s.ctx.installation),
                    guard: Arc::clone(&s.guard),
                    facts: s.facts.clone(),
                    timing: s.timing.clone(),
                },
            )
        })
    };
    if let Some((observer, receipt)) = notification {
        observer(super::super::fixture::NativeSmokeObservation::Http401(
            receipt,
        ));
    }
}

pub(in super::super) fn begin(
    owner: &Rc<RefCell<Owner>>,
    request: &Code0WindowRequest,
    started: Instant,
) -> Result<(), Code0SessionFailure> {
    let receipt = require_native_refusal_install(owner, request)?;
    if receipt.installed_mask() != 1023
        || receipt.load_rejected()
        || !Timing::new(started).before(Instant::now())
    {
        return Err(Code0SessionFailure::PolicyRejected);
    }
    let ctx = Context::new(owner, request);
    let _active = Active::enter(&ctx.callbacks);
    let environment = {
        let mut o = owner.borrow_mut();
        let i = o.instance.as_mut().unwrap();
        if i.http401.is_some()
            || i.policy.is_some()
            || i.policy_mailbox.is_some()
            || i.chooser.is_some()
            || i.capability.is_some()
            || !i.refusal_registry.http_available()
        {
            return Err(Code0SessionFailure::PolicyRejected);
        }
        i.http401_selected.store(true, Ordering::SeqCst);
        i.http401 = Some(State {
            ctx: ctx.clone(),
            guard: Arc::new(AtomicBool::new(false)),
            facts: Facts::default(),
            timing: Timing::new(started),
            core: ctx.core.upgrade(),
            core22: None,
            environment: None,
            resource_token: None,
            resource_add_attempted: false,
            resource_remove_attempted: false,
            completed_token: None,
            completed_add_attempted: false,
            completed_remove_attempted: false,
            filter: false,
            filter_attempted: false,
            filter_remove_attempted: false,
            listener: None,
            stream: None,
            startup: false,
            startup_cleanup_attempted: false,
            setup: 0,
            accepted_setup: 0,
            budget: ServiceBudget::default(),
            in_action: false,
            closing: false,
            close_published: false,
            parser: None,
            received: 0,
            port: None,
            url: None,
            nav_id: None,
            top: Allowance::default(),
            resource: Allowance::default(),
            navigate_reserved: false,
            basic_reserved: false,
        });
        i.environment.take().unwrap()
    };
    // Native AddRef outside Owner, with the original environment retained locally.
    let cached = Rc::new(environment.clone());
    {
        let mut o = owner.borrow_mut();
        let i = o.instance.as_mut().unwrap();
        i.environment = Some(environment);
        i.http401.as_mut().unwrap().environment = Some(cached);
    }
    if !setup_current(&ctx, owner) {
        fail(owner, "http401Binding", false);
        cleanup(owner);
        return Err(Code0SessionFailure::PolicyRejected);
    }
    let Some(core) = ctx.core.upgrade() else {
        fail(owner, "http401Binding", true);
        return Err(Code0SessionFailure::PolicyRejected);
    };
    let query = core.cast::<ICoreWebView2_22>();
    {
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        s.facts.install.core22_query_succeeded = Some(query.is_ok());
    }
    let core22 = match query {
        Ok(v) => Rc::new(v),
        Err(_) => {
            fail(owner, "http401Install", false);
            emit(owner);
            return Err(Code0SessionFailure::PolicyRejected);
        }
    };
    owner
        .borrow_mut()
        .instance
        .as_mut()
        .unwrap()
        .http401
        .as_mut()
        .unwrap()
        .core22 = Some(Rc::clone(&core22));
    if !setup_current(&ctx, owner) {
        return Err(Code0SessionFailure::PolicyRejected);
    }
    let resource_ctx = ctx.clone();
    let resource_environment = GuardEnvironment {
        environment: owner
            .borrow()
            .instance
            .as_ref()
            .unwrap()
            .http401
            .as_ref()
            .unwrap()
            .environment
            .as_ref()
            .map(Rc::clone),
        callbacks: Arc::clone(&ctx.callbacks),
        ui: ctx.ui,
        _lifetime: HandlerLifetime::new(&ctx.callbacks),
    };
    let handler = WebResourceRequestedEventHandler::create(Box::new(move |sender, args| {
        let _lifetime = &resource_environment;
        let active = Active::enter(&resource_ctx.callbacks);
        if !callback_on_ui(resource_ctx.ui, &resource_ctx.callbacks, 8601) {
            resource_ctx.lost();
            drop(args);
            drop(sender);
            drop(active);
            return Ok(());
        }
        let candidate = resource_callback(
            &resource_ctx,
            sender.as_ref(),
            args.as_ref(),
            resource_environment.environment.as_deref(),
        );
        drop(args);
        drop(sender);
        finish_resource(&resource_ctx, candidate);
        drop(active);
        Ok(())
    }));
    owner
        .borrow_mut()
        .instance
        .as_mut()
        .unwrap()
        .http401
        .as_mut()
        .unwrap()
        .resource_add_attempted = true;
    let mut token = 0;
    let result = unsafe { core.add_WebResourceRequested(&handler, &mut token) };
    {
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        if result.is_ok() {
            s.resource_token = Some(token);
            s.facts.install.guard_adds += 1;
        }
    }
    let ok = result.is_ok();
    drop(result);
    drop(handler);
    if !ok || !setup_current(&ctx, owner) {
        fail(owner, "http401Install", !ok);
        emit(owner);
        return Err(Code0SessionFailure::PolicyRejected);
    }
    owner
        .borrow_mut()
        .instance
        .as_mut()
        .unwrap()
        .http401
        .as_mut()
        .unwrap()
        .filter_attempted = true;
    let filtered = unsafe {
        core22.AddWebResourceRequestedFilterWithRequestSourceKinds(
            windows::core::w!("*"),
            COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
            COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
        )
    };
    {
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        if filtered.is_ok() {
            s.filter = true;
            s.facts.install.filter_adds += 1;
        }
    }
    let ok = filtered.is_ok();
    drop(filtered);
    if !ok || !setup_current(&ctx, owner) {
        fail(owner, "http401Install", !ok);
        emit(owner);
        return Err(Code0SessionFailure::PolicyRejected);
    }
    let completed_ctx = ctx.clone();
    let completed_lifetime = HandlerLifetime::new(&ctx.callbacks);
    let handler = NavigationCompletedEventHandler::create(Box::new(move |sender, args| {
        let _lifetime = &completed_lifetime;
        let active = Active::enter(&completed_ctx.callbacks);
        if !callback_on_ui(completed_ctx.ui, &completed_ctx.callbacks, 8602) {
            completed_ctx.lost();
            drop(args);
            drop(sender);
            drop(active);
            return Ok(());
        }
        let candidate = completed_callback(&completed_ctx, sender.as_ref(), args.as_ref());
        drop(args);
        drop(sender);
        finish_completed(&completed_ctx, candidate);
        drop(active);
        Ok(())
    }));
    owner
        .borrow_mut()
        .instance
        .as_mut()
        .unwrap()
        .http401
        .as_mut()
        .unwrap()
        .completed_add_attempted = true;
    let mut token = 0;
    let result = unsafe { core.add_NavigationCompleted(&handler, &mut token) };
    {
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        if result.is_ok() {
            s.completed_token = Some(token);
            s.facts.install.completed_adds += 1;
        }
    }
    let ok = result.is_ok();
    drop(result);
    drop(handler);
    drop(core22);
    drop(core);
    if !ok || !setup_current(&ctx, owner) {
        fail(owner, "http401Install", !ok);
        emit(owner);
        return Err(Code0SessionFailure::PolicyRejected);
    }
    emit(owner);
    Ok(())
}
fn setup_current(ctx: &Context, owner: &Rc<RefCell<Owner>>) -> bool {
    if !ctx.same(owner) {
        return false;
    }
    let o = owner.borrow();
    let i = o.instance.as_ref().unwrap();
    let s = i.http401.as_ref().unwrap();
    !o.unmatched
        && !i.failed
        && !i.closing
        && !i.destroyed
        && !i.released
        && !i.cleanup_blocked
        && !s.closing
        && s.facts.failure.is_none()
        && i.refusal_registry.complete()
        && !ctx.callbacks.affinity_failed.load(Ordering::SeqCst)
        && !ctx.installation.load(Ordering::SeqCst)
        && !s.guard.load(Ordering::SeqCst)
        && s.timing.before(Instant::now())
}

fn address_matches(address: &ws::SOCKADDR_IN, port: Option<u16>) -> bool {
    let octets = unsafe { address.sin_addr.S_un.S_un_b };
    address.sin_family == ws::AF_INET
        && [octets.s_b1, octets.s_b2, octets.s_b3, octets.s_b4] == [127, 0, 0, 1]
        && port.is_none_or(|p| u16::from_be(address.sin_port) == p)
}
fn loopback() -> ws::SOCKADDR_IN {
    ws::SOCKADDR_IN {
        sin_family: ws::AF_INET,
        sin_port: 0,
        sin_addr: ws::IN_ADDR {
            S_un: ws::IN_ADDR_0 {
                S_un_b: ws::IN_ADDR_0_0 {
                    s_b1: 127,
                    s_b2: 0,
                    s_b3: 0,
                    s_b4: 1,
                },
            },
        },
        sin_zero: [0; 8],
    }
}
fn socket_error(result: i32) -> Option<i32> {
    if result == ws::SOCKET_ERROR {
        Some(unsafe { ws::WSAGetLastError() })
    } else {
        None
    }
}
#[derive(Clone, Copy)]
enum Operation {
    Setup(u8, Option<ws::SOCKET>),
    Accepted(u8, ws::SOCKET),
    Accept(ws::SOCKET),
    Receive(ws::SOCKET, usize),
    Send(ws::SOCKET, &'static [u8]),
}
struct Outcome {
    ok: bool,
    error: Option<i32>,
    socket: Option<ws::SOCKET>,
    version: Option<u16>,
    address: Option<ws::SOCKADDR_IN>,
    linger_off: Option<bool>,
    bytes: [u8; 512],
    length: usize,
    eof: bool,
}
impl Drop for Outcome {
    fn drop(&mut self) {
        self.bytes.fill(0);
    }
}
fn perform(operation: Operation) -> Outcome {
    let mut out = Outcome {
        ok: false,
        error: None,
        socket: None,
        version: None,
        address: None,
        linger_off: None,
        bytes: [0; 512],
        length: 0,
        eof: false,
    };
    let mut linger = ws::LINGER {
        l_onoff: 0,
        l_linger: 0,
    };
    let mut address = loopback();
    let mut size = std::mem::size_of::<ws::SOCKADDR_IN>() as i32;
    match operation {
        Operation::Setup(0, _) => {
            let mut data = ws::WSADATA::default();
            let r = unsafe { ws::WSAStartup(0x0202, &mut data) };
            out.ok = r == 0;
            out.error = (r != 0).then_some(r);
            if r == 0 {
                out.version = Some(data.wVersion);
            }
        }
        Operation::Setup(1, _) => {
            let handle =
                unsafe { ws::socket(i32::from(ws::AF_INET), ws::SOCK_STREAM, ws::IPPROTO_TCP) };
            out.ok = handle != ws::INVALID_SOCKET;
            if out.ok {
                out.socket = Some(handle)
            } else {
                out.error = Some(unsafe { ws::WSAGetLastError() });
            }
        }
        Operation::Setup(2, Some(handle)) => {
            let value = 1_i32;
            let r = unsafe {
                ws::setsockopt(
                    handle,
                    ws::SOL_SOCKET,
                    ws::SO_EXCLUSIVEADDRUSE,
                    (&value as *const i32).cast(),
                    std::mem::size_of_val(&value) as i32,
                )
            };
            out.error = socket_error(r);
            out.ok = r == 0;
        }
        Operation::Setup(3, Some(handle)) | Operation::Accepted(0, handle) => {
            let r = unsafe {
                ws::setsockopt(
                    handle,
                    ws::SOL_SOCKET,
                    ws::SO_LINGER,
                    (&linger as *const ws::LINGER).cast(),
                    std::mem::size_of_val(&linger) as i32,
                )
            };
            out.error = socket_error(r);
            out.ok = r == 0;
        }
        Operation::Setup(4, Some(handle)) | Operation::Accepted(1, handle) => {
            linger.l_onoff = 1;
            let mut len = std::mem::size_of_val(&linger) as i32;
            let r = unsafe {
                ws::getsockopt(
                    handle,
                    ws::SOL_SOCKET,
                    ws::SO_LINGER,
                    (&mut linger as *mut ws::LINGER).cast(),
                    &mut len,
                )
            };
            out.error = socket_error(r);
            out.ok = r == 0 && len == std::mem::size_of_val(&linger) as i32;
            out.linger_off = (r == 0).then_some(out.ok && linger.l_onoff == 0);
        }
        Operation::Setup(5, Some(handle)) | Operation::Accepted(2, handle) => {
            let mut value = 1_u32;
            let r = unsafe { ws::ioctlsocket(handle, ws::FIONBIO, &mut value) };
            out.error = socket_error(r);
            out.ok = r == 0;
        }
        Operation::Setup(6, Some(handle)) => {
            let r = unsafe { ws::bind(handle, (&address as *const ws::SOCKADDR_IN).cast(), size) };
            out.error = socket_error(r);
            out.ok = r == 0;
        }
        Operation::Setup(7, Some(handle)) | Operation::Accepted(3, handle) => {
            address.sin_family = 0;
            let r = unsafe {
                ws::getsockname(
                    handle,
                    (&mut address as *mut ws::SOCKADDR_IN).cast(),
                    &mut size,
                )
            };
            out.error = socket_error(r);
            out.ok = r == 0 && size == std::mem::size_of::<ws::SOCKADDR_IN>() as i32;
            if out.ok {
                out.address = Some(address)
            }
        }
        Operation::Setup(8, Some(handle)) => {
            let r = unsafe { ws::listen(handle, 1) };
            out.error = socket_error(r);
            out.ok = r == 0;
        }
        Operation::Accept(handle) => {
            address.sin_family = 0;
            let stream = unsafe {
                ws::accept(
                    handle,
                    (&mut address as *mut ws::SOCKADDR_IN).cast(),
                    &mut size,
                )
            };
            out.ok = stream != ws::INVALID_SOCKET;
            if out.ok {
                out.socket = Some(stream);
                if size == std::mem::size_of::<ws::SOCKADDR_IN>() as i32 {
                    out.address = Some(address)
                }
            } else {
                out.error = Some(unsafe { ws::WSAGetLastError() });
            }
        }
        Operation::Receive(handle, len) => {
            let r = unsafe { ws::recv(handle, out.bytes.as_mut_ptr(), len as i32, 0) };
            if r == ws::SOCKET_ERROR {
                out.error = Some(unsafe { ws::WSAGetLastError() });
            } else {
                out.ok = true;
                out.length = r as usize;
                out.eof = r == 0;
            }
        }
        Operation::Send(handle, bytes) => {
            let r = unsafe { ws::send(handle, bytes.as_ptr(), bytes.len() as i32, 0) };
            if r == ws::SOCKET_ERROR {
                out.error = Some(unsafe { ws::WSAGetLastError() });
            } else {
                out.ok = true;
                out.length = r as usize;
                out.eof = r == 0;
            }
        }
        _ => {}
    }
    out
}
fn close_sockets(owner: &Rc<RefCell<Owner>>, handles: &[bool]) {
    let context = {
        let o = owner.borrow();
        let Some(i) = o.instance.as_ref() else { return };
        let Some(s) = i.http401.as_ref() else { return };
        if s.in_action {
            return;
        }
        s.ctx.clone()
    };
    let _active = Active::enter(&context.callbacks);
    for &listener in handles {
        let action = {
            let mut o = owner.borrow_mut();
            let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
            let lease = if listener {
                &mut s.listener
            } else {
                &mut s.stream
            };
            let handle = lease.as_mut().and_then(SocketLease::reserve_close);
            if handle.is_some() {
                if listener {
                    s.facts.server.listener_close_attempts += 1
                } else {
                    s.facts.server.stream_close_attempts += 1
                };
                s.in_action = true;
            }
            handle
        };
        if let Some(handle) = action {
            let r = unsafe { ws::closesocket(handle) };
            let _error = socket_error(r);
            let ok = r == 0;
            {
                let mut o = owner.borrow_mut();
                let i = o.instance.as_mut().unwrap();
                let s = i.http401.as_mut().unwrap();
                let lease = if listener {
                    s.listener.as_mut()
                } else {
                    s.stream.as_mut()
                };
                if let Some(lease) = lease {
                    lease.close_returned(ok);
                }
                if ok {
                    if listener {
                        s.facts.server.listener_close_successes += 1
                    } else {
                        s.facts.server.stream_close_successes += 1
                    }
                } else {
                    s.fail("http401Cleanup", Instant::now(), true);
                    i.failed = true;
                    i.closing = true;
                    i.cleanup_blocked = true;
                }
                s.in_action = false;
                s.inventory();
            }
        }
    }
    let balance = {
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        s.inventory();
        if s.startup
            && !s.startup_cleanup_attempted
            && s.facts.server.sockets_remaining == 0
            && !s.in_action
        {
            s.startup_cleanup_attempted = true;
            s.in_action = true;
            s.facts.server.cleanup_attempts += 1;
            true
        } else {
            false
        }
    };
    if balance {
        let r = unsafe { ws::WSACleanup() };
        let _error = socket_error(r);
        let mut o = owner.borrow_mut();
        let i = o.instance.as_mut().unwrap();
        let s = i.http401.as_mut().unwrap();
        if r == 0 {
            s.startup = false;
            s.facts.server.cleanup_successes += 1
        } else {
            s.fail("http401Cleanup", Instant::now(), true);
            i.failed = true;
            i.closing = true;
            i.cleanup_blocked = true;
        }
        s.in_action = false;
        s.inventory();
    }
    publish_close(owner);
}
pub(super) fn cleanup(owner: &Rc<RefCell<Owner>>) {
    let closing = owner
        .borrow()
        .instance
        .as_ref()
        .and_then(|i| i.http401.as_ref())
        .is_some_and(|s| s.closing);
    if closing {
        close_sockets(owner, &[true, false]);
    }
}
pub(super) fn external_close(i: &mut Instance) {
    if let Some(s) = i.http401.as_mut() {
        if s.facts.receipts.target == 0 && !(s.basic_reserved && s.closing) {
            s.fail("http401Cleanup", Instant::now(), false)
        } else {
            s.revoke(Instant::now());
        }
    }
}
pub(super) fn advance(owner: &Rc<RefCell<Owner>>) {
    let context = {
        let o = owner.borrow();
        let Some(i) = o.instance.as_ref() else { return };
        let Some(s) = i.http401.as_ref() else { return };
        s.ctx.clone()
    };
    if !setup_current(&context, owner) {
        let closing = owner
            .borrow()
            .instance
            .as_ref()
            .unwrap()
            .http401
            .as_ref()
            .unwrap()
            .closing;
        if !closing {
            let (uncertain, expired) = {
                let o = owner.borrow();
                let i = o.instance.as_ref().unwrap();
                let s = i.http401.as_ref().unwrap();
                (
                    context.installation.load(Ordering::SeqCst)
                        || s.guard.load(Ordering::SeqCst)
                        || context.callbacks.affinity_failed.load(Ordering::SeqCst)
                        || i.cleanup_blocked,
                    !s.timing.before(Instant::now()),
                )
            };
            fail(
                owner,
                if uncertain {
                    "http401Unknown"
                } else if expired {
                    "http401Deadline"
                } else {
                    "http401Binding"
                },
                uncertain,
            );
        } else {
            let reason = owner
                .borrow()
                .instance
                .as_ref()
                .unwrap()
                .http401
                .as_ref()
                .unwrap()
                .facts
                .failure;
            if let Some(reason) = reason {
                fail(owner, reason, false);
            }
        }
        cleanup(owner);
        publish_close(owner);
        emit(owner);
        return;
    }
    let active = Active::enter(&context.callbacks);
    let operation = {
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        if s.in_action {
            return;
        }
        let operation = if s.setup < 9 {
            Some(Operation::Setup(
                s.setup,
                s.listener.as_ref().map(|v| v.handle),
            ))
        } else if !s.navigate_reserved {
            None
        } else if !s.top.committed || !s.resource.committed {
            return;
        } else if s.stream.is_none() {
            s.listener.as_ref().map(|v| Operation::Accept(v.handle))
        } else if s.accepted_setup < 4 {
            s.stream
                .as_ref()
                .map(|v| Operation::Accepted(s.accepted_setup, v.handle))
        } else if s.facts.server.response_complete == Some(true) {
            return;
        } else if let Some(bytes) = s.parser.as_ref().and_then(Http401Protocol::response_suffix) {
            s.stream.as_ref().map(|v| Operation::Send(v.handle, bytes))
        } else if s.received < 4096 {
            s.stream
                .as_ref()
                .map(|v| Operation::Receive(v.handle, 512.min(4096 - s.received)))
        } else {
            None
        };
        if let Some(operation) = operation {
            let io = matches!(
                operation,
                Operation::Accept(_) | Operation::Receive(..) | Operation::Send(..)
            );
            match s.budget.reserve(Instant::now(), io) {
                Ok(false) => return,
                Err(()) => {
                    s.facts.overflow = true;
                    s.fail("http401Overflow", Instant::now(), false);
                    None
                }
                Ok(true) => {
                    s.in_action = true;
                    if io {
                        s.facts.server.io_attempts = s.budget.io;
                    } else {
                        match operation {
                            Operation::Setup(index, _) => {
                                s.facts.server.setup_calls += 1;
                                match index {
                                    0 => s.facts.server.startup_attempts += 1,
                                    1 => s.facts.server.socket_creates += 1,
                                    6 => s.facts.server.binds += 1,
                                    8 => s.facts.server.listens += 1,
                                    _ => {}
                                }
                            }
                            Operation::Accepted(..) => s.facts.server.accepted_setup_calls += 1,
                            _ => {}
                        }
                    }
                    Some(operation)
                }
            }
        } else {
            None
        }
    };
    if let Some(operation) = operation {
        let outcome = perform(operation);
        // Inventory liabilities first, including a successful result returned late.
        {
            let mut o = owner.borrow_mut();
            let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
            if let Operation::Setup(0, _) = operation {
                s.facts.server.startup_succeeded = Some(outcome.ok);
                if outcome.ok {
                    s.startup = true;
                }
                s.facts.server.version22 = outcome.version.map(|v| v == 0x0202);
            }
            match operation {
                Operation::Setup(1, _) => {
                    if let Some(handle) = outcome.socket {
                        s.listener = Some(SocketLease::new(handle, 1));
                    }
                }
                Operation::Accept(_) => {
                    if let Some(handle) = outcome.socket {
                        s.stream = Some(SocketLease::new(handle, 2));
                        s.facts.server.accepts += 1;
                    }
                }
                _ => {}
            }
            match operation {
                Operation::Setup(2, _) => s.facts.server.exclusive_ack = Some(outcome.ok),
                Operation::Setup(4, _) => s.facts.server.listener_linger_off = outcome.linger_off,
                Operation::Setup(5, _) => s.facts.server.listener_nonblocking = Some(outcome.ok),
                Operation::Setup(7, _) => {
                    s.facts.server.bound_endpoint_matched = outcome
                        .address
                        .as_ref()
                        .map(|a| address_matches(a, None) && a.sin_port != 0)
                }
                Operation::Setup(8, _) => s.facts.server.listen_succeeded = Some(outcome.ok),
                Operation::Accepted(1, _) => s.facts.server.stream_linger_off = outcome.linger_off,
                Operation::Accepted(2, _) => s.facts.server.stream_nonblocking = Some(outcome.ok),
                Operation::Accepted(3, _) => {
                    s.facts.server.accepted_local_matched =
                        outcome.address.as_ref().map(|a| address_matches(a, s.port))
                }
                Operation::Accept(_) => {
                    if outcome.socket.is_some() {
                        s.facts.server.accepted_peer_matched = Some(
                            outcome
                                .address
                                .as_ref()
                                .is_some_and(|a| address_matches(a, None)),
                        );
                    }
                }
                _ => {}
            }
            s.inventory();
        }
        let current = setup_current(&context, owner) && {
            let o = owner.borrow();
            let s = o.instance.as_ref().unwrap().http401.as_ref().unwrap();
            match operation {
                Operation::Setup(index, handle) => {
                    s.setup == index
                        && (index <= 1
                            || handle
                                == s.listener.as_ref().filter(|v| v._id == 1).map(|v| v.handle))
                }
                Operation::Accepted(index, handle) => {
                    s.accepted_setup == index
                        && s.stream
                            .as_ref()
                            .is_some_and(|v| v._id == 2 && v.handle == handle)
                }
                Operation::Accept(handle) => s
                    .listener
                    .as_ref()
                    .is_some_and(|v| v._id == 1 && v.handle == handle),
                Operation::Receive(handle, _) | Operation::Send(handle, _) => s
                    .stream
                    .as_ref()
                    .is_some_and(|v| v._id == 2 && v.handle == handle),
            }
        };
        let nonprogress = matches!(outcome.error, Some(ws::WSAEWOULDBLOCK | ws::WSAEINTR))
            && matches!(
                operation,
                Operation::Accept(_) | Operation::Receive(..) | Operation::Send(..)
            );
        let mut failed = None;
        {
            let mut o = owner.borrow_mut();
            let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
            // Actual successful provider lengths remain honest even if authority died.
            if matches!(operation, Operation::Receive(..)) && outcome.ok {
                s.received = s.received.saturating_add(outcome.length);
                s.facts.server.request_bytes = s.received.min(4097) as u16;
            }
            if matches!(operation, Operation::Send(..)) && outcome.ok {
                s.facts.server.sent_bytes = s
                    .facts
                    .server
                    .sent_bytes
                    .saturating_add(outcome.length as u16)
                    .min(140);
            }
            if !current {
                failed = Some("http401Binding");
            } else if !outcome.ok && !nonprogress {
                if matches!(operation, Operation::Send(..)) {
                    let _ = s.parser.as_mut().unwrap().record_send(SendOutcome::Failed);
                }
                failed = Some("http401Socket");
            } else if nonprogress {
                if matches!(operation, Operation::Send(..)) {
                    let outcome = if outcome.error == Some(ws::WSAEWOULDBLOCK) {
                        SendOutcome::WouldBlock
                    } else {
                        SendOutcome::Interrupted
                    };
                    if s.parser.as_mut().unwrap().record_send(outcome).is_err() {
                        failed = Some("http401Wire");
                    }
                }
            } else {
                match operation {
                    Operation::Setup(index, _) => {
                        let valid = match index {
                            0 => s.facts.server.version22 == Some(true),
                            2 => {
                                s.facts.server.exclusive_ack = Some(outcome.ok);
                                outcome.ok
                            }
                            4 => {
                                s.facts.server.listener_linger_off = outcome.linger_off;
                                outcome.linger_off == Some(true)
                            }
                            5 => {
                                s.facts.server.listener_nonblocking = Some(outcome.ok);
                                outcome.ok
                            }
                            7 => {
                                let valid = outcome
                                    .address
                                    .as_ref()
                                    .is_some_and(|a| address_matches(a, None) && a.sin_port != 0);
                                s.facts.server.bound_endpoint_matched = Some(valid);
                                if valid {
                                    let port =
                                        u16::from_be(outcome.address.as_ref().unwrap().sin_port);
                                    s.port = Some(port);
                                    s.url =
                                        Some(format!("http://127.0.0.1:{port}/code0-http-auth"));
                                    s.parser = Http401Protocol::new(port).ok();
                                }
                                valid && s.parser.is_some()
                            }
                            8 => {
                                s.facts.server.listen_succeeded = Some(outcome.ok);
                                outcome.ok
                            }
                            _ => outcome.ok,
                        };
                        if valid {
                            s.setup += 1;
                        } else {
                            failed = Some("http401Socket");
                        }
                    }
                    Operation::Accept(_) => {
                        let valid = outcome
                            .address
                            .as_ref()
                            .is_some_and(|a| address_matches(a, None));
                        s.facts.server.accepted_peer_matched = Some(valid);
                        if !valid {
                            failed = Some("http401Socket");
                        }
                    }
                    Operation::Accepted(index, _) => {
                        let valid = match index {
                            1 => {
                                s.facts.server.stream_linger_off = outcome.linger_off;
                                outcome.linger_off == Some(true)
                            }
                            2 => {
                                s.facts.server.stream_nonblocking = Some(outcome.ok);
                                outcome.ok
                            }
                            3 => {
                                let good = outcome
                                    .address
                                    .as_ref()
                                    .is_some_and(|a| address_matches(a, s.port));
                                s.facts.server.accepted_local_matched = Some(good);
                                good
                            }
                            _ => outcome.ok,
                        };
                        if valid {
                            s.accepted_setup += 1;
                        } else {
                            failed = Some("http401Socket");
                        }
                    }
                    Operation::Receive(..) => {
                        if outcome.eof {
                            let _ = s.parser.as_mut().unwrap().receive(&[]);
                            s.facts.server.request_valid = Some(false);
                            failed = Some("http401Request");
                        } else {
                            match s
                                .parser
                                .as_mut()
                                .unwrap()
                                .receive(&outcome.bytes[..outcome.length])
                            {
                                Ok(ReceiveStatus::Incomplete) => {}
                                Ok(ReceiveStatus::Accepted(receipt)) => {
                                    s.facts.server.parsed_requests += 1;
                                    s.facts.server.header_count = u16::from(receipt.header_count);
                                    s.facts.server.request_valid =
                                        Some(s.parser.as_ref().unwrap().receipt() == Some(receipt));
                                    s.facts.phase = "Writing";
                                }
                                Err(_) => {
                                    s.facts.server.request_valid = Some(false);
                                    failed = Some("http401Request");
                                }
                            }
                        }
                    }
                    Operation::Send(..) => {
                        let result = s
                            .parser
                            .as_mut()
                            .unwrap()
                            .record_send(SendOutcome::Sent(outcome.length));
                        match result {
                            Ok(progress) => {
                                if progress.complete {
                                    s.facts.server.response_complete = Some(true);
                                    s.facts.phase = "AwaitAuth";
                                }
                            }
                            Err(_) => {
                                s.facts.server.response_complete = Some(false);
                                failed = Some("http401Wire");
                            }
                        }
                    }
                }
            }
            s.in_action = false;
        }
        // Returned handles are closed on every path, never dropped as a receipt.
        if matches!(operation, Operation::Accept(_)) && outcome.socket.is_some() {
            close_sockets(owner, &[true]);
        }
        let complete = owner
            .borrow()
            .instance
            .as_ref()
            .unwrap()
            .http401
            .as_ref()
            .unwrap()
            .facts
            .server
            .response_complete
            == Some(true);
        if complete {
            close_sockets(owner, &[false]);
        }
        if let Some(reason) = failed {
            fail(owner, reason, false);
        }
        if !setup_current(&context, owner) {
            cleanup(owner);
        }
    } else {
        let ready = {
            let o = owner.borrow();
            let s = o.instance.as_ref().unwrap().http401.as_ref().unwrap();
            s.ready() && !s.navigate_reserved && !s.closing
        };
        if ready {
            navigate(owner, &context);
        } else if !setup_current(&context, owner) {
            cleanup(owner);
        }
    }
    drop(active);
    publish_close(owner);
    emit(owner);
}
fn navigate(owner: &Rc<RefCell<Owner>>, context: &Context) {
    let action = {
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        if s.navigate_reserved || !s.ready() {
            return;
        }
        s.navigate_reserved = true;
        s.facts.phase = "Armed";
        (s.core.as_ref().map(Rc::clone), s.url.clone())
    };
    let (Some(core), Some(url)) = action else {
        fail(owner, "http401Navigation", true);
        return;
    };
    let wide: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
    if !context.qualified(owner, true, Instant::now()) {
        drop(wide);
        drop(url);
        drop(core);
        fail(owner, "http401Binding", false);
        return;
    }
    // Reservation is not a native call observation. Count only at real entry.
    owner
        .borrow_mut()
        .instance
        .as_mut()
        .unwrap()
        .http401
        .as_mut()
        .unwrap()
        .facts
        .navigation
        .attempts += 1;
    let result = unsafe { core.Navigate(PCWSTR(wide.as_ptr())) };
    let good = result.is_ok();
    {
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        s.facts.navigation.hresult_succeeded = Some(good);
    }
    drop(result);
    drop(wide);
    drop(url);
    drop(core);
    if !good || !context.qualified(owner, true, Instant::now()) {
        fail(owner, "http401Navigation", false);
        cleanup(owner);
    }
}
pub(super) struct ChildCandidate {
    cancel_succeeded: bool,
}
pub(super) fn child_callback(
    ctx: &Context,
    args: Option<&ICoreWebView2NavigationStartingEventArgs>,
) -> ChildCandidate {
    let result = restrict_child(
        || {
            if let Some(owner) = ctx.owner.upgrade().filter(|owner| ctx.same(owner)) {
                fail(&owner, "http401Navigation", false);
                cleanup(&owner);
                publish_close(&owner);
            } else {
                ctx.lost();
            }
        },
        || {
            args.map_or_else(
                || Err(E_FAIL.into()),
                |args| unsafe { args.SetCancel(true) },
            )
        },
    );
    let cancel_succeeded = result.is_ok();
    drop(result);
    ChildCandidate { cancel_succeeded }
}
pub(super) fn finish_child(ctx: &Context, candidate: ChildCandidate) {
    let Some(owner) = ctx.owner.upgrade() else {
        ctx.lost();
        return;
    };
    if !ctx.same(&owner) {
        ctx.lost();
        return;
    }
    let now = Instant::now();
    let current = {
        let o = owner.borrow();
        let i = o.instance.as_ref().unwrap();
        let s = i.http401.as_ref().unwrap();
        !o.unmatched
            && !i.destroyed
            && !i.released
            && !ctx.callbacks.affinity_failed.load(Ordering::SeqCst)
            && !ctx.installation.load(Ordering::SeqCst)
            && !s.guard.load(Ordering::SeqCst)
            && s.timing.before(now)
            && s.timing.first_close.is_some_and(|at| {
                now >= at
                    && at
                        .checked_add(Duration::from_secs(10))
                        .is_some_and(|end| now < end)
            })
    };
    // This event is always terminal, even when its restricted native action and
    // post-Drop observation are current. It never touches the top allowance.
    if !candidate.cancel_succeeded {
        fail(&owner, "http401Navigation", true);
    } else if !current {
        fail(&owner, "http401Navigation", false);
    }
    cleanup(&owner);
    publish_close(&owner);
    emit(&owner);
}
pub(super) struct TopCandidate {
    eligible: bool,
    qualified: bool,
    id: Option<u64>,
    cancel_false: Option<bool>,
}
pub(super) fn top_callback(
    ctx: &Context,
    sender: Option<&ICoreWebView2>,
    args: Option<&ICoreWebView2NavigationStartingEventArgs>,
) -> TopCandidate {
    let Some(owner) = ctx.owner.upgrade() else {
        ctx.lost();
        if let Some(args) = args {
            let _ = unsafe { args.SetCancel(true) };
        }
        return TopCandidate {
            eligible: false,
            qualified: false,
            id: None,
            cancel_false: None,
        };
    };
    if !ctx.same(&owner) {
        ctx.lost();
        if let Some(args) = args {
            let _ = unsafe { args.SetCancel(true) };
        }
        return TopCandidate {
            eligible: false,
            qualified: false,
            id: None,
            cancel_false: None,
        };
    }
    let live = ctx.qualified(&owner, true, Instant::now());
    let (eligible, url) = {
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        let bounded = bump(&mut s.facts.navigation.top_events, 1);
        if !bounded {
            s.facts.overflow = true;
        }
        let eligible = bounded
            && s.navigate_reserved
            && s.facts.navigation.attempts == 1
            && s.top.reserve(live);
        (eligible, s.url.clone())
    };
    let Some(args) = args else {
        fail(&owner, "http401Navigation", true);
        return TopCandidate {
            eligible: false,
            qualified: false,
            id: None,
            cancel_false: None,
        };
    };
    let mut id = None;
    let mut cancel_false = None;
    let mut qualified = false;
    if eligible && ctx.sender(sender) && ctx.qualified(&owner, true, Instant::now()) {
        let uri = allocated_string(64, |raw| unsafe { args.Uri(raw) });
        let uri_good = uri
            .as_ref()
            .ok()
            .zip(url.as_ref())
            .is_some_and(|(a, b)| a == b);
        drop(uri);
        if uri_good && ctx.qualified(&owner, true, Instant::now()) {
            let mut navigation = 0;
            let r = unsafe { args.NavigationId(&mut navigation) };
            if r.is_ok() {
                id = Some(navigation)
            }
            let good = r.is_ok();
            drop(r);
            if good && ctx.qualified(&owner, true, Instant::now()) {
                let mut redirected = BOOL(0);
                let r = unsafe { args.IsRedirected(&mut redirected) };
                let good = r.is_ok() && !redirected.as_bool();
                drop(r);
                if good && ctx.qualified(&owner, true, Instant::now()) {
                    let mut cancel = BOOL(1);
                    let r = unsafe { args.Cancel(&mut cancel) };
                    cancel_false = r.is_ok().then_some(!cancel.as_bool());
                    drop(r);
                    qualified =
                        cancel_false == Some(true) && ctx.qualified(&owner, true, Instant::now());
                }
            }
        }
    }
    if !qualified {
        let r = unsafe { args.SetCancel(true) };
        let unknown = r.is_err();
        drop(r);
        fail(&owner, "http401Navigation", unknown);
        cleanup(&owner);
        publish_close(&owner);
    }
    TopCandidate {
        eligible,
        qualified,
        id,
        cancel_false,
    }
}
pub(super) fn finish_top(ctx: &Context, candidate: TopCandidate) {
    let Some(owner) = ctx.owner.upgrade() else {
        ctx.lost();
        return;
    };
    if !ctx.same(&owner) {
        ctx.lost();
        return;
    }
    let current =
        candidate.eligible && candidate.qualified && ctx.qualified(&owner, true, Instant::now());
    {
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        s.facts.navigation.qualification_matched = Some(current);
        s.facts.navigation.navigation_id_readable = Some(candidate.id.is_some());
        s.facts.navigation.cancel_false = candidate.cancel_false;
        if current && s.top.commit(true) {
            s.nav_id = candidate.id;
            s.facts.navigation.grants += 1;
        } else {
            s.top.revoke();
        }
    }
    if !current {
        fail(&owner, "http401Navigation", false);
        cleanup(&owner);
    }
    publish_close(&owner);
    emit(&owner);
}
// The stale safe-STA refusal needs an environment even after Owner retirement.
// Keep its native Release inside Active while the counted handler is still live.
struct GuardEnvironment {
    environment: Option<Rc<ICoreWebView2Environment>>,
    callbacks: Arc<Callbacks>,
    ui: ThreadId,
    _lifetime: HandlerLifetime,
}
impl Drop for GuardEnvironment {
    fn drop(&mut self) {
        let active = Active::enter(&self.callbacks);
        if thread::current().id() != self.ui {
            self.callbacks.affinity_failed.store(true, Ordering::SeqCst);
        }
        drop(self.environment.take());
        drop(active);
    }
}
struct ResourceCandidate {
    first: bool,
    observed: bool,
    eligible: bool,
    exact: Option<bool>,
    qualified: bool,
    refused: bool,
    denied: bool,
}
fn resource_callback(
    ctx: &Context,
    sender: Option<&ICoreWebView2>,
    args: Option<&ICoreWebView2WebResourceRequestedEventArgs>,
    fallback_environment: Option<&ICoreWebView2Environment>,
) -> ResourceCandidate {
    let Some(owner) = ctx.owner.upgrade() else {
        ctx.lost();
        if let Some((environment, args)) = fallback_environment.zip(args) {
            let _ = empty_403(environment, args);
        }
        return ResourceCandidate {
            first: false,
            observed: false,
            eligible: false,
            exact: None,
            qualified: false,
            refused: false,
            denied: false,
        };
    };
    if !ctx.same(&owner) {
        ctx.lost();
        if let Some((environment, args)) = fallback_environment.zip(args) {
            let _ = empty_403(environment, args);
        }
        return ResourceCandidate {
            first: false,
            observed: false,
            eligible: false,
            exact: None,
            qualified: false,
            refused: false,
            denied: false,
        };
    }
    let live = ctx.qualified(&owner, true, Instant::now());
    let (eligible, url, environment, first) = {
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        let bounded = bump(&mut s.facts.resource.total, 4);
        if !bounded {
            s.facts.overflow = true;
        }
        let eligible = bounded && s.top.committed && s.resource.reserve(live);
        (
            eligible,
            s.url.clone(),
            s.environment.as_ref().map(Rc::clone),
            s.facts.resource.total == 1,
        )
    };
    let Some(args) = args else {
        fail(&owner, "http401Resource", true);
        return ResourceCandidate {
            first,
            observed: false,
            eligible: false,
            exact: None,
            qualified: false,
            refused: false,
            denied: false,
        };
    };
    let mut exact = None;
    let mut qualified = false;
    if eligible && ctx.sender(sender) && ctx.qualified(&owner, true, Instant::now()) {
        let request = unsafe { args.Request() };
        if let Ok(request) = request {
            if ctx.qualified(&owner, true, Instant::now()) {
                let uri = allocated_string(64, |raw| unsafe { request.Uri(raw) });
                let uri_available = uri.is_ok();
                let uri_good = uri
                    .as_ref()
                    .ok()
                    .zip(url.as_ref())
                    .is_some_and(|(a, b)| a == b);
                drop(uri);
                if ctx.qualified(&owner, true, Instant::now()) {
                    let method = allocated_string(64, |raw| unsafe { request.Method(raw) });
                    let method_available = method.is_ok();
                    let method_good = method.as_ref().is_ok_and(|v| v == "GET");
                    drop(method);
                    if ctx.qualified(&owner, true, Instant::now()) {
                        let mut context = COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL;
                        let r = unsafe { args.ResourceContext(&mut context) };
                        exact = (uri_available && method_available && r.is_ok()).then_some(
                            context == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT
                                && uri_good
                                && method_good,
                        );
                        drop(r);
                    }
                }
            }
            drop(request);
        }
        qualified = exact == Some(true) && ctx.qualified(&owner, true, Instant::now());
    }
    let mut denied = false;
    if !qualified {
        {
            let mut o = owner.borrow_mut();
            let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
            let _ = bump(&mut s.facts.resource.refused, 4);
        }
        fail(&owner, "http401Resource", false);
        cleanup(&owner);
        publish_close(&owner);
        // Restrictive bounded response remains attempted on safely-STA stale input.
        if let Some(environment) = environment.as_ref() {
            denied = empty_403(environment, args);
        }
        if !denied {
            fail(&owner, "http401Resource", true);
        }
    }
    drop(environment);
    drop(url);
    ResourceCandidate {
        first,
        observed: true,
        eligible,
        exact,
        qualified,
        refused: !qualified,
        denied,
    }
}
fn empty_403(
    environment: &ICoreWebView2Environment,
    args: &ICoreWebView2WebResourceRequestedEventArgs,
) -> bool {
    let Some(stream) = (unsafe { SHCreateMemStream(None) }) else {
        return false;
    };
    let seek = unsafe { stream.Seek(0, STREAM_SEEK_SET, None) };
    let seek_good = seek.is_ok();
    drop(seek);
    if !seek_good {
        drop(stream);
        return false;
    }
    let response = unsafe {
        environment.CreateWebResourceResponse(
            &stream,
            403,
            windows::core::w!("Forbidden"),
            windows::core::w!(
                "Content-Length: 0\r\nConnection: close\r\nCache-Control: no-store\r\n"
            ),
        )
    };
    let mut good = false;
    if let Ok(response) = response {
        let set = unsafe { args.SetResponse(&response) };
        let set_good = set.is_ok();
        drop(set);
        let read = unsafe { args.Response() };
        if let Ok(read) = read {
            let expected = response.cast::<IUnknown>();
            let actual = read.cast::<IUnknown>();
            good = set_good
                && expected
                    .as_ref()
                    .ok()
                    .zip(actual.as_ref().ok())
                    .is_some_and(|(a, b)| a.as_raw() == b.as_raw());
            drop(actual);
            drop(expected);
            drop(read);
        }
        drop(response);
    }
    drop(stream);
    good
}
fn finish_resource(ctx: &Context, candidate: ResourceCandidate) {
    let Some(owner) = ctx.owner.upgrade() else {
        ctx.lost();
        return;
    };
    if !ctx.same(&owner) {
        ctx.lost();
        return;
    }
    let current =
        candidate.eligible && candidate.qualified && ctx.qualified(&owner, true, Instant::now());
    {
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        record_first_resource_metadata(
            &mut s.facts.resource,
            candidate.first,
            candidate.exact,
            candidate.observed.then_some(current),
        );
        if current && s.resource.commit(true) {
            s.facts.resource.passed += 1;
            s.facts.phase = "Passed";
        } else {
            s.resource.revoke();
        }
    }
    if !current {
        fail(
            &owner,
            "http401Resource",
            candidate.refused && !candidate.denied,
        );
        cleanup(&owner);
    }
    publish_close(&owner);
    emit(&owner);
}
pub(super) struct AuthCandidate {
    first: bool,
    eligible: bool,
    set: bool,
    read: Option<bool>,
    uri: Option<bool>,
    challenge: Option<bool>,
    sender: Option<bool>,
    before: bool,
}
pub(super) fn auth_callback(
    ctx: &Context,
    sender: Option<&ICoreWebView2>,
    args: Option<&ICoreWebView2BasicAuthenticationRequestedEventArgs>,
) -> AuthCandidate {
    let owner = ctx.owner.upgrade();
    let same = owner.as_ref().is_some_and(|o| ctx.same(o));
    let before = owner
        .as_ref()
        .filter(|_| same)
        .is_some_and(|o| ctx.qualified(o, true, Instant::now()));
    let mut url = None;
    let eligible = if let Some(owner) = owner.as_ref().filter(|_| same) {
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        let bounded = bump(&mut s.facts.auth.callbacks, 1);
        if !bounded {
            s.facts.overflow = true;
        }
        let allowed = before
            && bounded
            && !s.basic_reserved
            && s.top.committed
            && s.resource.committed
            && s.facts.server.response_complete == Some(true)
            && s.facts.server.sent_bytes == 139
            && s.facts.server.request_valid == Some(true);
        s.basic_reserved = true;
        url = s.url.clone();
        allowed
    } else {
        ctx.lost();
        false
    };
    let first = owner.as_ref().filter(|_| same).is_some_and(|o| {
        o.borrow()
            .instance
            .as_ref()
            .unwrap()
            .http401
            .as_ref()
            .unwrap()
            .facts
            .auth
            .callbacks
            == 1
    });
    let mut candidate = AuthCandidate {
        first,
        eligible,
        set: false,
        read: None,
        uri: None,
        challenge: None,
        sender: None,
        before,
    };
    let Some(args) = args else {
        if let Some(o) = owner.as_ref().filter(|_| same) {
            fail(o, "http401Auth", true);
        }
        return candidate;
    };
    // Exactly the two restrictive calls precede all metadata and identity QIs.
    let mut cancel = BOOL(0);
    let (set, get) = restrictive_pair(
        || {
            if let Some(owner) = owner.as_ref().filter(|_| same) {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
                if !bump(&mut s.facts.auth.cancel_attempts, 1) {
                    s.facts.overflow = true;
                }
            }
            // Revocation precedes the setter's possible synchronous reentry.
            // The original entry capture remains the only candidate authority.
            if let Some(owner) = owner.as_ref().filter(|_| same) {
                let mut o = owner.borrow_mut();
                let i = o.instance.as_mut().unwrap();
                i.http401.as_mut().unwrap().revoke(Instant::now());
                i.closing = true;
                drop(o);
                cleanup(owner);
                publish_close(owner);
            }
            let result = unsafe { args.SetCancel(true) };
            if let Some(owner) = owner.as_ref().filter(|_| same) {
                let mut o = owner.borrow_mut();
                let i = o.instance.as_mut().unwrap();
                let s = i.http401.as_mut().unwrap();
                if result.is_ok() && !bump(&mut s.facts.auth.cancel_successes, 1) {
                    s.facts.overflow = true;
                }
                s.revoke(Instant::now());
                i.closing = true;
                drop(o);
                cleanup(owner);
                publish_close(owner);
            }
            result
        },
        || {
            if let Some(owner) = owner.as_ref().filter(|_| same) {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
                if !bump(&mut s.facts.auth.cancel_readbacks, 1) {
                    s.facts.overflow = true;
                }
            }
            unsafe { args.Cancel(&mut cancel) }
        },
    );
    candidate.set = set.is_ok();
    candidate.read = get.is_ok().then_some(cancel.as_bool());
    drop(get);
    drop(set);
    if let Some(owner) = owner.as_ref().filter(|_| same)
        && eligible
        && candidate.set
        && candidate.read == Some(true)
        && ctx.qualified(owner, false, Instant::now())
    {
        let uri = allocated_string(64, |raw| unsafe { args.Uri(raw) });
        candidate.uri = uri.as_ref().ok().map(|v| url.as_ref() == Some(v));
        drop(uri);
        if ctx.qualified(owner, false, Instant::now()) {
            let challenge = allocated_string(64, |raw| unsafe { args.Challenge(raw) });
            candidate.challenge = challenge
                .as_ref()
                .ok()
                .map(|v| v == "Basic realm=\"code0-fixture\"");
            drop(challenge);
            if ctx.qualified(owner, false, Instant::now()) {
                candidate.sender = Some(ctx.sender(sender));
            }
        }
    }
    drop(url);
    candidate
}
pub(super) fn finish_auth(ctx: &Context, candidate: AuthCandidate) {
    let Some(owner) = ctx.owner.upgrade() else {
        ctx.lost();
        return;
    };
    if !ctx.same(&owner) {
        ctx.lost();
        return;
    }
    let now = Instant::now();
    let current = ctx.qualified(&owner, false, now);
    let good = candidate.eligible
        && candidate.before
        && candidate.set
        && candidate.read == Some(true)
        && candidate.uri == Some(true)
        && candidate.challenge == Some(true)
        && candidate.sender == Some(true)
        && current;
    {
        let mut o = owner.borrow_mut();
        let i = o.instance.as_mut().unwrap();
        let s = i.http401.as_mut().unwrap();
        if candidate.first {
            let a = &mut s.facts.auth;
            a.cancel_true = candidate.read;
            a.uri_matched = candidate.uri;
            a.challenge_matched = candidate.challenge;
            a.sender_matched = candidate.sender;
            a.post_drop_current = Some(current);
            a.within_attempt_deadline = Some(s.timing.before(now));
            s.facts.binding = Binding {
                request_matched: Some(i.request == ctx.request),
                generation_matched: Some(i.refusal_registry.http_generation() == ctx.generation),
                core_matched: candidate.sender,
                s2_install_matched: Some(
                    Arc::ptr_eq(&i.refusal_registry.unknown, &ctx.installation)
                        && i.refusal_registry.complete(),
                ),
                guard_install_matched: Some(
                    s.facts.install.guard_adds == 1
                        && s.facts.install.filter_adds == 1
                        && s.facts.install.completed_adds == 1
                        && !s.guard.load(Ordering::SeqCst),
                ),
                post_drop_current: Some(current),
            };
        }
        if good {
            s.facts.receipts.target = 1;
            s.timing.target = Some(now);
        } else {
            s.fail(
                "http401Auth",
                now,
                !candidate.set || candidate.read.is_none(),
            );
        }
        i.failed = true;
        i.closing = true;
    }
    cleanup(&owner);
    publish_close(&owner);
    emit(&owner);
}
struct CompletedCandidate {
    profile: Option<EarlyProfile>,
    expected_id: Option<u64>,
    first: bool,
}
fn completed_callback(
    ctx: &Context,
    sender: Option<&ICoreWebView2>,
    args: Option<&ICoreWebView2NavigationCompletedEventArgs>,
) -> CompletedCandidate {
    let Some(owner) = ctx.owner.upgrade() else {
        ctx.lost();
        return CompletedCandidate {
            profile: None,
            expected_id: None,
            first: false,
        };
    };
    if !ctx.same(&owner) {
        ctx.lost();
        return CompletedCandidate {
            profile: None,
            expected_id: None,
            first: false,
        };
    }
    let (disposition, wire, id) = {
        let o = owner.borrow();
        let s = o.instance.as_ref().unwrap().http401.as_ref().unwrap();
        (
            s.closing || s.facts.receipts.target > 0,
            s.facts.server.response_complete == Some(true) && s.facts.server.sent_bytes == 139,
            s.nav_id,
        )
    };
    if disposition {
        return CompletedCandidate {
            profile: None,
            expected_id: id,
            first: false,
        };
    }
    let live = ctx.qualified(&owner, true, Instant::now());
    let mut p = EarlyProfile {
        live_at_entry: Some(live),
        full_wire_before_callback: Some(wire),
        ..EarlyProfile::default()
    };
    let first = {
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        let first = bump(&mut s.facts.early401.count, 1);
        if !first {
            s.facts.overflow = true;
        } else {
            s.facts.early401.profile = Some(p.clone());
        }
        first
    };
    if live {
        p.sender_matched = Some(ctx.sender(sender));
    }
    if let Some(args) = args.filter(|_| live) {
        if ctx.qualified(&owner, true, Instant::now()) {
            let mut native = 0;
            let r = unsafe { args.NavigationId(&mut native) };
            p.navigation_id_matched = r.is_ok().then_some(id == Some(native));
            drop(r);
        }
        if ctx.qualified(&owner, true, Instant::now()) {
            let mut success = BOOL(0);
            let r = unsafe { args.IsSuccess(&mut success) };
            p.is_success = r.is_ok().then_some(success.as_bool());
            drop(r);
        }
        if ctx.qualified(&owner, true, Instant::now()) {
            let mut status = COREWEBVIEW2_WEB_ERROR_STATUS_UNKNOWN;
            let r = unsafe { args.WebErrorStatus(&mut status) };
            p.web_error_status = r.is_ok().then_some(
                if status == COREWEBVIEW2_WEB_ERROR_STATUS_VALID_AUTHENTICATION_CREDENTIALS_REQUIRED
                {
                    "validAuthenticationCredentialsRequired"
                } else {
                    "other"
                },
            );
            drop(r);
        }
        if ctx.qualified(&owner, true, Instant::now()) {
            let query = args.cast::<ICoreWebView2NavigationCompletedEventArgs2>();
            p.args2_query_succeeded = Some(query.is_ok());
            if let Ok(args2) = query {
                if ctx.qualified(&owner, true, Instant::now()) {
                    let mut code = 0;
                    let r = unsafe { args2.HttpStatusCode(&mut code) };
                    p.http_status_code = r.is_ok().then_some(code);
                    drop(r);
                }
                drop(args2);
            }
        }
    }
    CompletedCandidate {
        profile: Some(p),
        expected_id: id,
        first,
    }
}
fn finish_completed(ctx: &Context, mut candidate: CompletedCandidate) {
    let Some(mut profile) = candidate.profile.take() else {
        return;
    };
    let Some(owner) = ctx.owner.upgrade() else {
        ctx.lost();
        return;
    };
    if !ctx.same(&owner) {
        ctx.lost();
        return;
    }
    profile.post_drop_current = Some(ctx.qualified(&owner, true, Instant::now()));
    let good = profile.positive() && candidate.expected_id.is_some();
    let unique = {
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        let unique = candidate.first && s.facts.early401.count == 1;
        if candidate.first {
            s.facts.early401.profile = Some(profile);
        }
        unique
    };
    if !good || !unique {
        fail(&owner, "http401Completion", false);
        cleanup(&owner);
        publish_close(&owner);
    }
    emit(&owner);
}
pub(super) fn remove(owner: &Rc<RefCell<Owner>>) {
    let action = {
        let o = owner.borrow();
        let Some(i) = o.instance.as_ref() else { return };
        let Some(s) = i.http401.as_ref() else { return };
        if !s.closing {
            return;
        }
        (
            s.ctx.clone(),
            s.core.as_ref().map(Rc::clone),
            s.core22.as_ref().map(Rc::clone),
        )
    };
    let (context, core, core22) = action;
    let active = Active::enter(&context.callbacks);
    cleanup(owner);
    for resource in [true, false] {
        let token = {
            let mut o = owner.borrow_mut();
            let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
            let attempted = if resource {
                &mut s.resource_remove_attempted
            } else {
                &mut s.completed_remove_attempted
            };
            if *attempted {
                None
            } else {
                *attempted = true;
                if resource {
                    s.resource_token
                } else {
                    s.completed_token
                }
            }
        };
        if let Some(token) = token {
            let result = core.as_ref().map(|core| {
                if resource {
                    unsafe { core.remove_WebResourceRequested(token) }
                } else {
                    unsafe { core.remove_NavigationCompleted(token) }
                }
            });
            let good = result.as_ref().is_some_and(Result::is_ok);
            drop(result);
            if good {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
                if resource {
                    s.resource_token = None;
                    s.facts.install.guard_removes += 1;
                } else {
                    s.completed_token = None;
                    s.facts.install.completed_removes += 1;
                }
            } else {
                fail(owner, "http401Cleanup", true);
            }
        }
    }
    let filter = {
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        if s.filter && !s.filter_remove_attempted {
            s.filter_remove_attempted = true;
            true
        } else {
            false
        }
    };
    if filter {
        let result = core22.as_ref().map(|core| unsafe {
            core.RemoveWebResourceRequestedFilterWithRequestSourceKinds(
                windows::core::w!("*"),
                COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
            )
        });
        let good = result.as_ref().is_some_and(Result::is_ok);
        drop(result);
        if good {
            let mut o = owner.borrow_mut();
            let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
            s.filter = false;
            s.facts.install.filter_removes += 1;
        } else {
            fail(owner, "http401Cleanup", true);
        }
    }
    drop(core22);
    drop(core);
    drop(active);
    publish_close(owner);
    emit(owner);
}
pub(super) fn removed(i: &Instance) -> bool {
    i.http401.as_ref().is_none_or(|s| {
        s.resource_token.is_none()
            && s.completed_token.is_none()
            && !s.filter
            && !s.in_action
            && !s.startup
            && s.listener.as_ref().is_none_or(|v| !v.owned())
            && s.stream.as_ref().is_none_or(|v| !v.owned())
            && !s.guard.load(Ordering::SeqCst)
    })
}
pub(super) struct Controls {
    _core: Option<Rc<ICoreWebView2>>,
    _core22: Option<Rc<ICoreWebView2_22>>,
    _environment: Option<Rc<ICoreWebView2Environment>>,
}
pub(super) fn take_controls(i: &mut Instance) -> Controls {
    if let Some(s) = i.http401.as_mut() {
        Controls {
            _core: s.core.take(),
            _core22: s.core22.take(),
            _environment: s.environment.take(),
        }
    } else {
        Controls {
            _core: None,
            _core22: None,
            _environment: None,
        }
    }
}
pub(super) fn quiescent(i: &Instance) -> bool {
    i.http401.as_ref().is_none_or(State::quiescent)
}
pub(super) fn publish_final(owner: &Rc<RefCell<Owner>>) {
    {
        let mut o = owner.borrow_mut();
        let Some(i) = o.instance.as_mut() else { return };
        let released = i.released && inventory_empty(i);
        let handlers = i.callbacks.handlers.load(Ordering::SeqCst) == 0;
        let actions = i.callbacks.active.load(Ordering::SeqCst) == 0
            && i.callbacks.pending.load(Ordering::SeqCst) == 0;
        let Some(s) = i.http401.as_mut() else { return };
        if s.facts.receipts.final_receipt != 0 {
            return;
        }
        s.revoke(Instant::now());
        s.inventory();
        s.facts.receipts.final_receipt = 1;
        s.facts.install.handlers_drained = Some(handlers);
        s.facts.server.final_inventory_observed = Some(s.quiescent() && actions);
        s.facts.cleanup.actions_drained = Some(actions);
        s.facts.cleanup.native_inventory_drained = Some(released && s.quiescent() && handlers);
        if !released || !s.quiescent() || !handlers || !actions {
            s.fail("http401Cleanup", Instant::now(), true);
        }
    }
    emit(owner);
}
#[cfg(test)]
mod tests {
    use super::*;
    // Removing spacing or adding catch-up must permit the forbidden second call.
    #[test]
    fn service_spacing_has_no_catchup_and_nonprogress_spends_io_budget() {
        let t = Instant::now();
        let mut budget = ServiceBudget::default();
        assert_eq!(budget.reserve(t, true), Ok(true));
        assert_eq!(
            budget.reserve(t + Duration::from_millis(24), true),
            Ok(false)
        );
        assert_eq!(
            budget.reserve(t + Duration::from_millis(25), true),
            Ok(true)
        );
        assert_eq!(
            budget.reserve(t + Duration::from_millis(25), true),
            Ok(false)
        );
        assert_eq!(budget.io, 2);
        for n in 2..2048 {
            assert_eq!(
                budget.reserve(t + Duration::from_millis(n * 25), true),
                Ok(true)
            );
        }
        assert_eq!(budget.reserve(t + Duration::from_secs(60), true), Err(()));
        assert_eq!(budget.io, 2048);
    }
    // Treating descriptor zero as absent or Drop as close success loses liability.
    #[test]
    fn socket_zero_failed_close_and_replay_keep_exact_ownership() {
        let mut lease = SocketLease::new(0, 9);
        assert_eq!(lease.reserve_close(), Some(0));
        lease.close_returned(false);
        assert!(lease.owned());
        assert_eq!(lease.reserve_close(), None);
        let mut good = SocketLease::new(7, 10);
        assert_eq!(good.reserve_close(), Some(7));
        good.close_returned(true);
        assert!(!good.owned());
        assert_eq!(good.reserve_close(), None);
    }
    // Refill or a late post-Drop commit would authorize a duplicate request.
    #[test]
    fn allowance_is_spent_before_reentry_and_never_recovers() {
        let mut grant = Allowance::default();
        assert!(grant.reserve(true));
        assert!(!grant.reserve(true));
        grant.revoke();
        assert!(!grant.commit(true));
        assert!(!grant.reserve(true));
        let mut live = Allowance::default();
        assert!(live.reserve(true));
        assert!(live.commit(true));
        assert!(!live.commit(true));
        let mut revoked_at_commit = Allowance::default();
        assert!(revoked_at_commit.reserve(true));
        assert!(!revoked_at_commit.commit(false));
        assert!(!revoked_at_commit.commit(true));
        assert!(!revoked_at_commit.reserve(true));
    }
    // Setter failure must not prevent the real readback attempt or reorder metadata.
    #[test]
    fn restrictive_setter_and_readback_both_run_before_metadata_on_error() {
        let calls = RefCell::new(Vec::new());
        let (set, get) = restrictive_pair(
            || {
                calls.borrow_mut().push(1);
                Err::<(), _>(())
            },
            || {
                calls.borrow_mut().push(2);
                Ok::<_, ()>(true)
            },
        );
        calls.borrow_mut().push(3);
        assert!(set.is_err());
        assert_eq!(get, Ok(true));
        assert_eq!(*calls.borrow(), vec![1, 2, 3]);
    }
    // Receipt publication/late adoption cannot restart the first close10.
    #[test]
    fn original_close_and_final_temp_boundary_are_strict_and_immutable() {
        let t = Instant::now();
        let mut time = Timing::new(t);
        time.close(t + Duration::from_secs(3));
        time.close(t + Duration::from_secs(8));
        time.target = Some(t + Duration::from_secs(4));
        assert_eq!(time.first_close, Some(t + Duration::from_secs(3)));
        assert!(
            time.final_clock(t + Duration::from_millis(12_999))
                .within_close
        );
        assert!(!time.final_clock(t + Duration::from_secs(13)).within_close);
        assert!(
            !time
                .final_clock(t + Duration::from_millis(13_001))
                .within_close
        );
        assert!(!Timing::new(t).final_clock(t).within_close);
        assert!(Timing::new(t).before(t + Duration::from_millis(44_999)));
        assert!(!Timing::new(t).before(t + Duration::from_secs(45)));
        assert!(!Timing::new(t).before(t + Duration::from_millis(45_001)));
    }
    // Losing any exact early401 fact must reject rather than invent an auth event.
    #[test]
    fn provisional_401_requires_every_actual_fact_and_never_407() {
        let good = EarlyProfile {
            sender_matched: Some(true),
            navigation_id_matched: Some(true),
            args2_query_succeeded: Some(true),
            is_success: Some(false),
            http_status_code: Some(401),
            web_error_status: Some("validAuthenticationCredentialsRequired"),
            full_wire_before_callback: Some(true),
            live_at_entry: Some(true),
            post_drop_current: Some(true),
        };
        assert!(good.positive());
        for n in 0..9 {
            let mut p = good.clone();
            match n {
                0 => p.is_success = Some(true),
                1 => p.http_status_code = Some(407),
                2 => p.args2_query_succeeded = Some(false),
                3 => p.navigation_id_matched = Some(false),
                4 => p.full_wire_before_callback = Some(false),
                5 => p.post_drop_current = Some(false),
                6 => p.web_error_status = None,
                7 => p.sender_matched = Some(false),
                _ => p.live_at_entry = Some(false),
            };
            assert!(!p.positive());
        }
    }
    // Full send length is real provider accounting; one extra byte cannot complete.
    #[test]
    fn adapter_uses_real_parser_and_partial_suffix_without_nonprogress_eof() {
        let mut p = Http401Protocol::new(43127).unwrap();
        let request = b"GET /code0-http-auth HTTP/1.1\r\nHost: 127.0.0.1:43127\r\n\r\n";
        for b in &request[..request.len() - 1] {
            assert_eq!(p.receive(&[*b]), Ok(ReceiveStatus::Incomplete));
        }
        assert!(matches!(p.receive(b"\n"), Ok(ReceiveStatus::Accepted(_))));
        assert_eq!(p.record_send(SendOutcome::WouldBlock).unwrap().sent, 0);
        assert_eq!(p.record_send(SendOutcome::Interrupted).unwrap().sent, 0);
        assert_eq!(p.record_send(SendOutcome::Sent(138)).unwrap().sent, 138);
        assert!(p.record_send(SendOutcome::Sent(2)).is_err());
    }
}

#[cfg(test)]
mod fix1_tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[test]
    fn finalizer_protects_writer_and_failure_rewrite_before_join() {
        let armed = Cell::new(true);
        let trace = RefCell::new(Vec::new());
        let passed = finalize_http401_with_watchdog(
            || {
                // A pending writer has not yet allowed shutdown/join.
                assert!(armed.get());
                trace.borrow_mut().push("writer");
                assert!(armed.get());
                trace.borrow_mut().push("failed-rewrite");
                assert!(armed.get());
                false
            },
            || {
                trace.borrow_mut().push("join");
                armed.set(false);
                true
            },
            || {
                assert!(!armed.get());
                trace.borrow_mut().push("final-check");
                true
            },
        );
        assert!(!passed);
        assert_eq!(
            *trace.borrow(),
            vec!["writer", "failed-rewrite", "join", "final-check"]
        );
    }

    #[test]
    fn finalizer_uses_original_close_after_join_and_never_writes_again() {
        let start = Instant::now();
        let mut timing = Timing::new(start);
        timing.close(start);
        let now = Cell::new(start + Duration::from_millis(9_999));
        let writes = Cell::new(0);
        let passed = finalize_http401_with_watchdog(
            || {
                writes.set(writes.get() + 1);
                assert!(timing.final_clock(now.get()).within_close);
                true
            },
            || {
                now.set(start + Duration::from_secs(10));
                true
            },
            || timing.final_clock(now.get()).within_close,
        );
        assert!(!passed);
        assert_eq!(writes.get(), 1);
        assert_eq!(timing.first_close(), Some(start));
        assert!(finalize_http401_with_watchdog(|| true, || true, || true));
        assert!(!finalize_http401_with_watchdog(|| true, || false, || true));
        assert!(!finalize_http401_with_watchdog(|| false, || true, || true));
    }

    #[test]
    fn selected_child_reentry_cannot_commit_or_refill_the_actual_allowance() {
        let grant = RefCell::new(Allowance::default());
        assert!(grant.borrow_mut().reserve(true));
        let calls = Cell::new(0);
        restrict_child(
            || grant.borrow_mut().revoke(),
            || {
                calls.set(calls.get() + 1);
                assert!(!grant.borrow_mut().commit(true));
                assert!(!grant.borrow_mut().reserve(true));
            },
        );
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn defensive_session_counter_keeps_real_lower_bound_and_overflow() {
        let mut facts = Facts::default();
        record_http401_session_failure(&mut facts);
        assert_eq!(facts.session.failures, 1);
        assert!(!facts.overflow);
        record_http401_session_failure(&mut facts);
        assert_eq!(facts.session.failures, 2);
        assert!(facts.overflow);
        record_http401_session_failure(&mut facts);
        assert_eq!(facts.session.failures, 2);
        assert!(facts.overflow);
    }

    #[test]
    fn first_rejected_resource_retains_actual_false_and_unavailable_metadata() {
        let mut observed = Resource::default();
        record_first_resource_metadata(&mut observed, true, Some(false), Some(false));
        assert_eq!(observed.exact_document_get, Some(false));
        assert_eq!(observed.qualification_matched, Some(false));
        record_first_resource_metadata(&mut observed, false, Some(true), Some(true));
        assert_eq!(observed.exact_document_get, Some(false));
        assert_eq!(observed.qualification_matched, Some(false));
        let mut unavailable = Resource::default();
        record_first_resource_metadata(&mut unavailable, true, None, None);
        assert_eq!(unavailable.exact_document_get, None);
        assert_eq!(unavailable.qualification_matched, None);
    }

    #[test]
    fn session_overflow_survives_later_native_snapshots() {
        let mut local = Facts::default();
        record_http401_session_failure(&mut local);
        local.session.reason = Some("PolicyRejected");
        local.cleanup.browser_process_exited_once = Some(true);
        let native = Facts {
            receipts: ReceiptCounts {
                final_receipt: 1,
                ..ReceiptCounts::default()
            },
            ..Facts::default()
        };
        merge_http401_native_facts(&mut local, &native);
        assert_eq!(local.session.failures, 1);
        assert_eq!(local.session.reason, Some("PolicyRejected"));
        assert_eq!(local.cleanup.browser_process_exited_once, Some(true));
        assert_eq!(local.receipts.final_receipt, 1);
        assert!(!local.overflow);

        record_http401_session_failure(&mut local);
        local.failure = Some("http401Session");
        assert_eq!(local.session.failures, 2);
        assert!(local.overflow);
        for _ in 0..2 {
            merge_http401_native_facts(&mut local, &native);
            assert_eq!(local.session.failures, 2);
            assert_eq!(local.failure, Some("http401Session"));
            assert_eq!(local.cleanup.browser_process_exited_once, Some(true));
            assert!(local.overflow);
            assert!(!local.native_positive());
        }

        let native_overflow = Facts {
            overflow: true,
            failure: Some("http401Resource"),
            ..Facts::default()
        };
        let mut once = Facts::default();
        record_http401_session_failure(&mut once);
        merge_http401_native_facts(&mut once, &native_overflow);
        assert_eq!(once.session.failures, 1);
        assert!(once.overflow);
        assert_eq!(once.failure, Some("http401Resource"));
        merge_http401_native_facts(&mut once, &native);
        assert!(once.overflow);
        assert_eq!(once.failure, Some("http401Resource"));
    }
}
