use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ai_token_meter_windows::collectors::code0_capture::Code0WebsiteCapture;
use ai_token_meter_windows::domain::{UsageSnapshot, UsageStatus};
use ai_token_meter_windows::platform::windows::code0_dispatcher::{
    Code0Action, Code0DispatchError as Error, Code0Dispatcher, Code0Proxy, Code0Reply,
    Code0ReplySink, Code0Revision, Code0Schedule, Code0SessionDto,
};
use ai_token_meter_windows::platform::windows::code0_web_session::{
    Code0CleanupRequest, Code0SessionActionResult as Action, Code0SessionClock,
    Code0SessionFailure as Failure, Code0SessionState as State, Code0SnapshotStoring,
    Code0StoreError, Code0WebSession, Code0WindowDriving, Code0WindowEvent as Event,
    Code0WindowMode as Mode, Code0WindowRequest,
};
use serde_json::{Value, json};

const BINDING: u64 = 7;
const TICK: u64 = 11;
const SAFE_MAX: u64 = 9_007_199_254_740_991;

// The fake executor deliberately permits synchronous scheduling, like the real
// UI scheduler. Only the thread-local runner knows the non-Send owner.
thread_local! {
    static RUNNER: RefCell<Option<Rc<dyn Fn()>>> = const { RefCell::new(None) };
    static DRAIN_DEPTH: Cell<usize> = const { Cell::new(0) };
    static MAX_DRAIN_DEPTH: Cell<usize> = const { Cell::new(0) };
    static ACTION_OBSERVATION: RefCell<Option<ActionObservation>> = const { RefCell::new(None) };
}

#[derive(Default)]
struct Executor {
    held: std::sync::atomic::AtomicBool,
    fail: std::sync::atomic::AtomicBool,
    schedules: std::sync::atomic::AtomicUsize,
}
impl Code0Schedule for Executor {
    fn schedule(&self) -> Result<(), Error> {
        use std::sync::atomic::Ordering::SeqCst;
        self.schedules.fetch_add(1, SeqCst);
        if self.fail.load(SeqCst) {
            return Err(Error::Unavailable);
        }
        if !self.held.load(SeqCst) {
            run_ui();
        }
        Ok(())
    }
}
fn run_ui() {
    let runner = RUNNER.with(|slot| slot.borrow().clone().unwrap());
    DRAIN_DEPTH.with(|depth| {
        depth.set(depth.get() + 1);
        MAX_DRAIN_DEPTH.with(|max| max.set(max.get().max(depth.get())));
    });
    runner();
    DRAIN_DEPTH.with(|depth| depth.set(depth.get() - 1));
}

#[derive(Clone)]
struct Clock(Rc<Cell<u64>>);
impl Code0SessionClock for Clock {
    fn now(&self) -> Duration {
        Duration::from_secs(self.0.get())
    }
    fn wall_now(&self) -> String {
        "2026-10-27T01:00:00Z".into()
    }
}
type Trace = Rc<RefCell<Vec<&'static str>>>;
struct ActionObservation {
    trace: Trace,
    emitted: Rc<RefCell<Vec<Code0SessionDto>>>,
}
struct Store {
    cache: Option<UsageSnapshot>,
    trace: Trace,
}
impl Code0SnapshotStoring for Store {
    fn load(&self) -> Result<Option<UsageSnapshot>, Code0StoreError> {
        self.trace.borrow_mut().push("load");
        Ok(self.cache.clone())
    }
    fn save_atomic(&mut self, value: &UsageSnapshot) -> Result<(), Code0StoreError> {
        self.trace.borrow_mut().push("save");
        self.cache = Some(value.clone());
        Ok(())
    }
    fn remove(&mut self) -> Result<(), Code0StoreError> {
        self.trace.borrow_mut().push("remove");
        self.cache = None;
        Ok(())
    }
}
#[derive(Default)]
struct Native {
    windows: Vec<Code0WindowRequest>,
    cleanup: Vec<Code0CleanupRequest>,
    held: bool,
    inline_policies: bool,
    proxy: Option<Code0Proxy>,
}
struct Driver {
    native: Rc<RefCell<Native>>,
    trace: Trace,
}
impl Code0WindowDriving for Driver {
    fn acquire_profile_lease(&mut self) -> Result<(), Failure> {
        self.trace.borrow_mut().push("acquire");
        self.native.borrow_mut().held = true;
        Ok(())
    }
    fn release_profile_lease(&mut self) {
        self.trace.borrow_mut().push("release");
        self.native.borrow_mut().held = false;
    }
    fn create_unloaded(&mut self, request: Code0WindowRequest) -> Result<(), Failure> {
        self.trace.borrow_mut().push("create");
        let (proxy, inline) = {
            let mut native = self.native.borrow_mut();
            native.windows.push(request.clone());
            (native.proxy.clone(), native.inline_policies)
        };
        if let Some(proxy) = proxy {
            proxy
                .expect_window_release(BINDING, request.clone())
                .unwrap();
            if inline {
                self.trace.borrow_mut().push("callback-enter");
                proxy
                    .window(BINDING, request, Event::PoliciesInstalled)
                    .unwrap();
                self.trace.borrow_mut().push("callback-return");
            }
        }
        Ok(())
    }
    fn navigate(&mut self, _: &str) -> Result<(), Failure> {
        self.trace.borrow_mut().push("navigate");
        Ok(())
    }
    fn close(&mut self, _: &str) {
        self.trace.borrow_mut().push("close-request");
    }
    fn clear_website_data(&mut self, request: Code0CleanupRequest) -> Result<(), Failure> {
        self.trace.borrow_mut().push("clear-request");
        let proxy = {
            let mut native = self.native.borrow_mut();
            native.cleanup.push(request.clone());
            native.proxy.clone()
        };
        if let Some(proxy) = proxy {
            proxy.expect_cleanup(BINDING, request).unwrap();
        }
        Ok(())
    }
}
type Session = Code0WebSession<Clock, Driver, Store>;
type Owner = Code0Dispatcher<Clock, Driver, Store>;
type Pending = Arc<Mutex<Option<Result<Code0Reply, Error>>>>;
fn pending() -> (Pending, Code0ReplySink) {
    let value = Arc::new(Mutex::new(None));
    let result = value.clone();
    (
        value,
        Box::new(move |reply| {
            let mut slot = result.lock().unwrap();
            assert!(slot.is_none(), "reply delivered more than once");
            *slot = Some(reply);
        }),
    )
}
fn take(value: &Pending) -> Result<Code0Reply, Error> {
    value
        .lock()
        .unwrap()
        .take()
        .expect("accepted request must reply")
}
fn action(proxy: &Code0Proxy, action: Code0Action) -> Action {
    let (value, reply) = pending();
    proxy.action(action, reply);
    match take(&value).unwrap() {
        Code0Reply::Action(result) => result,
        other => panic!("wrong action reply: {other:?}"),
    }
}
fn read(proxy: &Code0Proxy) -> Code0SessionDto {
    let (value, reply) = pending();
    proxy.read(reply);
    match take(&value).unwrap() {
        Code0Reply::View(view) => *view,
        other => panic!("wrong read reply: {other:?}"),
    }
}

enum Seed {
    Empty,
    Cached,
    Syncing,
    Active,
}
struct Fixture {
    // Keep the sole non-Send owner alive for the registered weak runner.
    _owner: Rc<Owner>,
    proxy: Code0Proxy,
    clock: Clock,
    native: Rc<RefCell<Native>>,
    trace: Trace,
    executor: Arc<Executor>,
    emitted: Rc<RefCell<Vec<Code0SessionDto>>>,
}
impl Fixture {
    fn new(seed: Seed) -> Self {
        Self::with_revision(seed, 0)
    }
    fn with_revision(seed: Seed, revision: u64) -> Self {
        let trace = Rc::new(RefCell::new(Vec::new()));
        let clock = Clock(Rc::new(Cell::new(0)));
        let native = Rc::new(RefCell::new(Native::default()));
        let cache = if matches!(seed, Seed::Empty) {
            None
        } else {
            Some(prior())
        };
        let signal = trace.clone();
        let mut session = Session::new(
            clock.clone(),
            Driver {
                native: native.clone(),
                trace: trace.clone(),
            },
            Store {
                cache,
                trace: trace.clone(),
            },
            Box::new(move |_| {
                signal.borrow_mut().push("dirty");
            }),
        );
        // These are real accepted session transitions used only to reach the
        // dispatcher relationship under test; the dispatcher is never faked.
        if matches!(seed, Seed::Syncing | Seed::Active) {
            assert_eq!(session.sync_now(), Action::Started);
            let request = native.borrow().windows.last().unwrap().clone();
            session.receive_window_event(&request, Event::PoliciesInstalled);
            if matches!(seed, Seed::Active) {
                for event in capture_events(&request) {
                    session.receive_window_event(&request, event);
                }
                session.receive_window_event(&request, Event::Released);
                assert_eq!(session.state(), State::Active);
            }
        }
        let executor = Arc::new(Executor::default());
        let proxy = Code0Proxy::new(executor.clone(), Code0Revision::new(revision).unwrap());
        let emitted = Rc::new(RefCell::new(Vec::new()));
        let events = emitted.clone();
        let observation = trace.clone();
        let projection = proxy.clone();
        let owner = Rc::new(
            Code0Dispatcher::install(
                BINDING,
                TICK,
                session,
                clock.clone(),
                proxy.clone(),
                Box::new(move |view| {
                    // Emit must see the already-committed matching state/snapshot and
                    // may synchronously requeue a read without borrowing the owner.
                    assert_eq!(&projection.committed(), view);
                    observation.borrow_mut().push("commit-observed");
                    let (reply, sink) = pending();
                    projection.read(sink);
                    assert!(
                        reply
                            .lock()
                            .unwrap()
                            .as_ref()
                            .is_none_or(|value| *value == Err(Error::QueueFull)),
                        "emit read must enqueue or reject saturated transport"
                    );
                    observation.borrow_mut().push("emit");
                    events.borrow_mut().push(view.clone());
                }),
            )
            .unwrap(),
        );
        let weak = Rc::downgrade(&owner);
        RUNNER.with(|slot| {
            *slot.borrow_mut() = Some(Rc::new(move || weak.upgrade().unwrap().drain()))
        });
        MAX_DRAIN_DEPTH.with(|depth| depth.set(0));
        native.borrow_mut().proxy = Some(proxy.clone());
        if let Some(request) = native.borrow().windows.last().cloned() {
            proxy.expect_window_release(BINDING, request).unwrap();
        }
        Self {
            _owner: owner,
            proxy,
            clock,
            native,
            trace,
            executor,
            emitted,
        }
    }
    fn window(&self) -> Code0WindowRequest {
        self.native.borrow().windows.last().unwrap().clone()
    }
    fn event(&self, request: &Code0WindowRequest, event: Event) {
        self.proxy.window(BINDING, request.clone(), event).unwrap();
    }
    fn tick(&self, seconds: u64) {
        self.clock.0.set(seconds);
        self.proxy.tick(BINDING, TICK).unwrap();
    }
    fn hold(&self) {
        self.executor
            .held
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
    fn flush(&self) {
        self.executor
            .held
            .store(false, std::sync::atomic::Ordering::SeqCst);
        run_ui();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        RUNNER.with(|slot| slot.borrow_mut().take());
        ACTION_OBSERVATION.with(|slot| slot.borrow_mut().take());
    }
}
fn prior() -> UsageSnapshot {
    let mut value = Code0WebsiteCapture::decode_filtered_json(
        br#"{"quota":1000000,"usedQuota":500000,"divisor":500000}"#,
    )
    .unwrap()
    .make_snapshot("2026-10-27T00:00:00Z")
    .unwrap();
    value.status = UsageStatus::Cached;
    // These must never cross the narrow DTO serialization boundary.
    value.status_message = Some("synthetic private text".into());
    value.history_fetched_at = Some("2026-10-26T00:00:00Z".into());
    value
}
fn capture_events(request: &Code0WindowRequest) -> Vec<Event> {
    [("ready", 1), ("selfStart", 2), ("capture", 3)].into_iter().map(|(kind, sequence)| {
        let mut body = json!({"version":1,"nonce":request.nonce,"document":request.document_id,"kind":kind,"sequence":sequence});
        if kind != "ready" {
            body["request"] = json!(1);
            body["revision"] = json!(0);
        }
        if kind == "capture" {
            body["quota"] = json!(2250000);
            body["usedQuota"] = json!(750000);
            body["divisor"] = json!(500000);
        }
        Event::Message { payload: serde_json::to_vec(&body).unwrap(), source_url:"https://console.code0.ai/console".into(), is_main_frame:true }
    }).collect()
}

// Mutation: Read starts a connection, leaks a full UsageSnapshot, or changes
// revision while returning the same view. Cold projection must stay passive.
#[test]
fn cold_reads_are_passive_stable_and_serialize_only_the_strict_projection() {
    let f = Fixture::new(Seed::Cached);
    let a = read(&f.proxy);
    assert_eq!(read(&f.proxy), a);
    assert!(f.native.borrow().windows.is_empty());
    assert_eq!(&*f.trace.borrow(), &["load"]);
    let json = serde_json::to_value(&a).unwrap();
    assert_eq!(json.as_object().unwrap().len(), 6);
    assert_eq!(json["state"], "paused");
    let snapshot = &json["snapshot"];
    assert_eq!(snapshot.as_object().unwrap().len(), 10);
    assert_eq!(snapshot["primaryMetric"].as_object().unwrap().len(), 7);
    assert_eq!(snapshot["primaryMetric"]["current"], 2.0);
    assert_eq!(snapshot["primaryMetric"]["unit"], "siteCredits");
    assert_eq!(snapshot["secondaryMetric"]["current"], 1.0);
    assert!(snapshot.get("statusMessage").is_none());
    assert!(snapshot.get("historyFetchedAt").is_none());
}

// Mutation: every explicit action is denied, login is retried while busy, or
// loginCompleted creates a sync window before actual owned release.
#[test]
fn four_actions_reply_from_the_same_session_and_busy_is_not_retried() {
    let f = Fixture::new(Seed::Empty);
    assert_eq!(action(&f.proxy, Code0Action::SyncNow), Action::Denied);
    assert_eq!(action(&f.proxy, Code0Action::Connect), Action::Started);
    let login = f.window();
    assert_eq!(login.mode, Mode::Login);
    assert_eq!(action(&f.proxy, Code0Action::Connect), Action::Busy);
    f.event(&login, Event::PoliciesInstalled);
    assert_eq!(
        action(&f.proxy, Code0Action::LoginCompleted),
        Action::Started
    );
    assert_eq!(f.native.borrow().windows.len(), 1);
    f.event(&login, Event::Released);
    assert_eq!(f.window().mode, Mode::Sync);
    assert_eq!(f.native.borrow().windows.len(), 2);
    assert_eq!(action(&f.proxy, Code0Action::Disconnect), Action::Started);
    f.event(&f.window(), Event::Released);
    let cleanup = f.native.borrow().cleanup.last().unwrap().clone();
    f.proxy.cleanup(BINDING, cleanup, Ok(())).unwrap();
    assert_eq!(
        serde_json::to_value(read(&f.proxy)).unwrap()["state"],
        "disconnected"
    );
    assert_eq!(
        f.native.borrow().windows.len(),
        2,
        "busy connect was never retried"
    );
}

// Mutation: an inline create callback schedules a recursive drain while the
// session is mutably borrowed, or the callback is discarded instead of queued.
#[test]
fn synchronous_create_callback_is_queued_with_maximum_owner_depth_one() {
    let f = Fixture::new(Seed::Empty);
    f.native.borrow_mut().inline_policies = true;
    assert_eq!(action(&f.proxy, Code0Action::Connect), Action::Started);
    assert_eq!(MAX_DRAIN_DEPTH.with(Cell::get), 1);
    assert_eq!(
        f.executor
            .schedules
            .load(std::sync::atomic::Ordering::SeqCst),
        1
    );
    let trace = f.trace.borrow();
    let returned = trace.iter().position(|e| *e == "callback-return").unwrap();
    let navigated = trace.iter().position(|e| *e == "navigate").unwrap();
    assert!(returned < navigated);
}

// Mutation: publication/emit/reply precedes the persisted state, or callbacks
// reenter the borrowed owner. Replies and emit observe one committed projection.
#[test]
fn save_then_commit_then_emit_then_reply_with_no_owner_borrow() {
    let f = Fixture::new(Seed::Syncing);
    let request = f.window();
    for event in capture_events(&request) {
        f.event(&request, event);
    }
    let committed = read(&f.proxy);
    let json = serde_json::to_value(&committed).unwrap();
    assert_eq!(json["state"], "closing");
    assert_eq!(json["snapshot"]["primaryMetric"]["current"], 4.5);
    assert_eq!(json["snapshot"]["fetchedAt"], "2026-10-27T01:00:00Z");
    let trace = f.trace.borrow();
    let save = trace.iter().position(|e| *e == "save").unwrap();
    let dirty = trace.iter().rposition(|e| *e == "dirty").unwrap();
    let emit = trace.iter().rposition(|e| *e == "emit").unwrap();
    assert!(save < dirty && dirty < emit);
    assert_eq!(f.emitted.borrow().last(), Some(&committed));
    drop(trace);
    let seen = f.proxy.clone();
    let (result, reply) = pending();
    f.proxy.action(
        Code0Action::SyncNow,
        Box::new(move |value| {
            assert_eq!(
                serde_json::to_value(seen.committed()).unwrap()["state"],
                "closing"
            );
            // An inline reply may enqueue work; recursive owner borrowing would panic.
            seen.read(Box::new(|_| {}));
            reply(value);
        }),
    );
    assert_eq!(take(&result), Ok(Code0Reply::Action(Action::Busy)));
    assert_eq!(MAX_DRAIN_DEPTH.with(Cell::get), 1);
}

// Mutation: a state-changing action replies Started before its new null view is
// committed, or before the matching revision has reached the emitter.
#[test]
fn disconnect_reply_observes_new_null_revision_after_matching_emit() {
    let f = Fixture::new(Seed::Cached);
    let before = read(&f.proxy);
    ACTION_OBSERVATION.with(|slot| {
        *slot.borrow_mut() = Some(ActionObservation {
            trace: f.trace.clone(),
            emitted: f.emitted.clone(),
        });
    });
    let projection = f.proxy.clone();
    let (result, reply) = pending();
    f.proxy.action(
        Code0Action::Disconnect,
        Box::new(move |value| {
            let committed = projection.committed();
            assert!(
                committed.revision > before.revision,
                "reply preceded changed-view commit"
            );
            let json = serde_json::to_value(&committed).unwrap();
            assert_eq!(json["state"], "closing");
            assert_eq!(json["snapshot"], Value::Null);
            ACTION_OBSERVATION.with(|slot| {
                let observation = slot.borrow();
                let observation = observation.as_ref().unwrap();
                assert_eq!(
                    observation.emitted.borrow().last(),
                    Some(&committed),
                    "reply preceded matching emit"
                );
                observation.trace.borrow_mut().push("reply");
            });
            assert_eq!(value, Ok(Code0Reply::Action(Action::Started)));
            reply(value);
        }),
    );
    assert_eq!(take(&result), Ok(Code0Reply::Action(Action::Started)));
    let order: Vec<_> = f
        .trace
        .borrow()
        .iter()
        .copied()
        .filter(|event| matches!(*event, "commit-observed" | "emit" | "reply"))
        .collect();
    assert_eq!(order, ["commit-observed", "emit", "reply"]);
}

// Mutation: a delayed ordinary/cache read replaces B or a committed null, or
// projection is republished with the same revision but different content.
#[test]
fn committed_b_and_null_outrank_late_read_without_ordinary_cache_fallback() {
    let f = Fixture::new(Seed::Syncing);
    let old_read = read(&f.proxy);
    let request = f.window();
    for event in capture_events(&request) {
        f.event(&request, event);
    }
    let b = f.proxy.committed();
    assert!(b.revision > old_read.revision);
    assert_eq!(
        serde_json::to_value(&b).unwrap()["snapshot"]["primaryMetric"]["current"],
        4.5
    );
    assert_eq!(action(&f.proxy, Code0Action::Disconnect), Action::Started);
    let revoked = read(&f.proxy);
    assert!(revoked.revision > b.revision);
    assert_eq!(
        serde_json::to_value(&revoked).unwrap()["snapshot"],
        Value::Null
    );
    // The proxy has no ordinary projection writer; retaining A cannot mutate it.
    drop(old_read);
    assert_eq!(read(&f.proxy), revoked);
    assert_eq!(f.emitted.borrow().last(), Some(&revoked));
}

// Mutation: the reserved release overtakes an earlier accepted current auth
// revocation, activating saved B and making the revocation stale to the session.
#[test]
fn queued_authentication_revocation_is_applied_before_matching_release() {
    let f = Fixture::new(Seed::Syncing);
    let current = f.window();
    for event in capture_events(&current) {
        f.event(&current, event);
    }
    let captured = read(&f.proxy);
    let captured_json = serde_json::to_value(&captured).unwrap();
    assert_eq!(captured_json["state"], "closing");
    assert_eq!(captured_json["snapshot"]["primaryMetric"]["current"], 4.5);
    let removals_before = f
        .trace
        .borrow()
        .iter()
        .filter(|event| **event == "remove")
        .count();
    f.hold();
    assert_eq!(
        f.proxy.window(
            BINDING,
            current.clone(),
            Event::Failed(Failure::AuthenticationChanged)
        ),
        Ok(())
    );
    assert_eq!(f.proxy.window(BINDING, current, Event::Released), Ok(()));
    f.flush();
    let revoked = read(&f.proxy);
    assert!(revoked.revision > captured.revision);
    assert_eq!(
        f.trace
            .borrow()
            .iter()
            .filter(|event| **event == "remove")
            .count(),
        removals_before + 1,
        "accepted authentication revocation must remove the cached B"
    );
    let revoked_json = serde_json::to_value(&revoked).unwrap();
    assert_eq!(revoked_json["snapshot"], Value::Null);
    assert_eq!(revoked_json["state"], "paused");
    assert_eq!(revoked_json["failure"], "authenticationChanged");
    assert_eq!(revoked_json["isAutomaticallySyncing"], false);
    assert_eq!(f.emitted.borrow().last(), Some(&revoked));
    let windows_before = f.native.borrow().windows.len();
    f.tick(300);
    assert_eq!(
        f.native.borrow().windows.len(),
        windows_before,
        "revoked session must not automatically capture at 300"
    );
    assert_eq!(read(&f.proxy), revoked);
}

// A general failure must demote activation through the session too. Duplicate
// releases and traffic arriving later must not expand the finite old prefix.
#[test]
fn pre_release_failure_precedes_ack_but_later_read_flood_does_not() {
    let f = Fixture::new(Seed::Syncing);
    let current = f.window();
    for event in capture_events(&current) {
        f.event(&current, event);
    }
    let later_reads = Arc::new(Mutex::new(Vec::new()));
    let observations = later_reads.clone();
    let proxy = f.proxy.clone();
    let duplicate = current.clone();
    f.hold();
    f.proxy.read(Box::new(move |value| {
        let initial = match value.unwrap() {
            Code0Reply::View(view) => view,
            other => panic!("wrong read reply {other:?}"),
        };
        assert_eq!(serde_json::to_value(initial).unwrap()["state"], "closing");
        // The failure is still queued, so these 63 fit the general bound.
        for _ in 0..63 {
            let observations = observations.clone();
            proxy.read(Box::new(move |value| {
                let view = match value.unwrap() {
                    Code0Reply::View(view) => view,
                    other => panic!("wrong read reply {other:?}"),
                };
                observations
                    .lock()
                    .unwrap()
                    .push(serde_json::to_value(view).unwrap()["state"].clone());
            }));
        }
        for _ in 0..1000 {
            proxy
                .window(BINDING, duplicate.clone(), Event::Released)
                .unwrap();
        }
    }));
    assert_eq!(
        f.proxy.window(
            BINDING,
            current.clone(),
            Event::Failed(Failure::NavigationFailed)
        ),
        Ok(())
    );
    assert_eq!(f.proxy.window(BINDING, current, Event::Released), Ok(()));
    f.flush();
    let states = later_reads.lock().unwrap();
    assert_eq!(states.len(), 63);
    assert!(
        states.iter().all(|state| *state == "paused"),
        "later reads must follow the protected ack, not extend its prefix"
    );
    let view = serde_json::to_value(read(&f.proxy)).unwrap();
    assert_eq!(view["state"], "paused");
    assert_eq!(view["failure"], "navigationFailed");
    assert_eq!(view["snapshot"]["primaryMetric"]["current"], 4.5);
    assert_eq!(view["isAutomaticallySyncing"], false);
    f.tick(300);
    assert_eq!(f.native.borrow().windows.len(), 1);
}

// Transport scheduling failure may reject RPCs, but it cannot silently discard
// queued native evidence and then activate B when physical release is retried.
#[test]
fn scheduling_failure_retains_native_revocation_before_later_release() {
    let f = Fixture::new(Seed::Syncing);
    let current = f.window();
    for event in capture_events(&current) {
        f.event(&current, event);
    }
    f.executor
        .fail
        .store(true, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(
        f.proxy.window(
            BINDING,
            current.clone(),
            Event::Failed(Failure::AuthenticationChanged)
        ),
        Err(Error::Unavailable)
    );
    f.executor
        .fail
        .store(false, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(f.proxy.window(BINDING, current, Event::Released), Ok(()));
    let view = serde_json::to_value(read(&f.proxy)).unwrap();
    assert_eq!(view["state"], "paused");
    assert_eq!(view["snapshot"], Value::Null);
    assert_eq!(view["failure"], "authenticationChanged");
    assert_eq!(view["isAutomaticallySyncing"], false);
    assert_eq!(
        f.trace
            .borrow()
            .iter()
            .filter(|event| **event == "remove")
            .count(),
        1
    );
    f.tick(300);
    assert_eq!(f.native.borrow().windows.len(), 1);
}

// Mutation: wrapping the JS safe integer revision authorizes an unpublished
// action, or an invalid initial revision reaches the frontend.
#[test]
fn checked_safe_integer_revision_refuses_action_before_native_work() {
    assert!(Code0Revision::new(SAFE_MAX + 1).is_err());
    let f = Fixture::with_revision(Seed::Empty, SAFE_MAX);
    let (result, reply) = pending();
    f.proxy.action(Code0Action::Connect, reply);
    assert_eq!(take(&result), Err(Error::RevisionExhausted));
    assert!(f.native.borrow().windows.is_empty());
    assert_eq!(f.proxy.committed().revision, SAFE_MAX);
}

// Mutation: ticks burst after sleep, 299 grants a sync, retry presentation calls
// sync_now, or an expired display countdown grants paused automatic permission.
#[test]
fn tick_299_300_and_sleep_are_coalesced_without_paused_authorization() {
    let f = Fixture::new(Seed::Active);
    f.tick(299);
    assert_eq!(f.native.borrow().windows.len(), 1);
    assert_eq!(
        action(&f.proxy, Code0Action::SyncNow),
        Action::Throttled {
            seconds_remaining: 1
        }
    );
    assert_eq!(
        serde_json::to_value(read(&f.proxy)).unwrap()["retryAfterSeconds"],
        1
    );
    f.hold();
    f.clock.0.set(300);
    for _ in 0..1000 {
        f.proxy.tick(BINDING, TICK).unwrap();
    }
    f.flush();
    assert_eq!(f.native.borrow().windows.len(), 2);
    let current = f.window();
    f.event(&current, Event::Failed(Failure::NavigationFailed));
    f.event(&current, Event::Released);
    assert_eq!(
        serde_json::to_value(read(&f.proxy)).unwrap()["state"],
        "paused"
    );
    assert_eq!(
        action(&f.proxy, Code0Action::SyncNow),
        Action::Throttled {
            seconds_remaining: 300
        }
    );
    f.hold();
    f.clock.0.set(3600);
    for _ in 0..1000 {
        f.proxy.tick(BINDING, TICK).unwrap();
    }
    f.flush();
    assert_eq!(f.native.borrow().windows.len(), 2);
    let dto = serde_json::to_value(read(&f.proxy)).unwrap();
    assert_eq!(dto["retryAfterSeconds"], Value::Null);
    assert_eq!(dto["isAutomaticallySyncing"], false);
    assert_eq!(action(&f.proxy, Code0Action::SyncNow), Action::Started);
}

// Mutation: wrong binding/request/cleanup/tick reaches the current session or
// consumes the finite reserved slot needed for its real physical release.
#[test]
fn old_binding_request_tick_and_cleanup_cannot_change_current_owner() {
    let f = Fixture::new(Seed::Syncing);
    let current = f.window();
    let before = read(&f.proxy);
    let mut stale = current.clone();
    stale.document_id = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".into();
    f.proxy.window(BINDING, stale, Event::Released).unwrap();
    f.proxy
        .window(BINDING - 1, current.clone(), Event::Released)
        .unwrap();
    f.clock.0.set(45);
    f.proxy.tick(BINDING - 1, TICK).unwrap();
    f.proxy.tick(BINDING, TICK - 1).unwrap();
    f.proxy
        .cleanup(
            BINDING - 1,
            Code0CleanupRequest {
                connection_epoch: current.connection_epoch.clone(),
                cleanup_id: "stale".into(),
            },
            Ok(()),
        )
        .unwrap();
    assert_eq!(read(&f.proxy), before);
    f.proxy.tick(BINDING, TICK).unwrap();
    assert_eq!(
        serde_json::to_value(read(&f.proxy)).unwrap()["state"],
        "closing"
    );
    f.event(&current, Event::Released);
    assert_eq!(
        serde_json::to_value(read(&f.proxy)).unwrap()["state"],
        "paused"
    );
    assert!(
        f.native.borrow().held,
        "ordinary pause retains the exclusive profile"
    );
}

// Mutation: last-writer tick coalescing lets a stale wake replace the current
// binding's pending tick, suppressing the actual current 300-second boundary.
#[test]
fn a_stale_wakeup_cannot_overwrite_the_current_coalesced_tick() {
    let f = Fixture::new(Seed::Active);
    f.hold();
    f.clock.0.set(300);
    f.proxy.tick(BINDING, TICK).unwrap();
    for _ in 0..1000 {
        f.proxy.tick(BINDING - 1, TICK).unwrap();
        f.proxy.tick(BINDING, TICK - 1).unwrap();
    }
    f.flush();
    assert_eq!(f.native.borrow().windows.len(), 2);
    assert_eq!(f.window().mode, Mode::Sync);
}

// Mutation: Stop treats close-request as physical release or drops the binding
// before the still-owned release can prove its lease is free.
#[test]
fn stop_waits_for_owned_released_before_replying_safe_to_exit() {
    let f = Fixture::new(Seed::Syncing);
    let current = f.window();
    let (result, reply) = pending();
    f.proxy.stop(reply);
    assert!(
        result.lock().unwrap().is_none(),
        "close request is not release proof"
    );
    assert!(f.native.borrow().held);
    assert_eq!(action(&f.proxy, Code0Action::Connect), Action::Denied);
    f.event(&current, Event::Released);
    assert_eq!(take(&result), Ok(Code0Reply::Stopped));
    assert!(!f.native.borrow().held);
    assert_eq!(f.native.borrow().windows.len(), 1);
}

// Mutation: Stop bypasses an unfinished Disconnect/cache cleanup or a stale
// cleanup result supplies false permission to exit.
#[test]
fn stop_during_disconnect_waits_for_matching_cleanup_proof() {
    let f = Fixture::new(Seed::Syncing);
    let current = f.window();
    assert_eq!(action(&f.proxy, Code0Action::Disconnect), Action::Started);
    let (result, reply) = pending();
    f.proxy.stop(reply);
    f.event(&current, Event::Released);
    assert!(result.lock().unwrap().is_none());
    assert!(f.native.borrow().held);
    let cleanup = f.native.borrow().cleanup.last().unwrap().clone();
    let mut stale = cleanup.clone();
    stale.cleanup_id = "old-cleanup".into();
    f.proxy.cleanup(BINDING, stale, Ok(())).unwrap();
    f.proxy
        .cleanup(BINDING - 1, cleanup.clone(), Ok(()))
        .unwrap();
    assert!(result.lock().unwrap().is_none());
    f.proxy.cleanup(BINDING, cleanup, Ok(())).unwrap();
    assert_eq!(take(&result), Ok(Code0Reply::Stopped));
    assert!(!f.native.borrow().held);
    assert_eq!(
        serde_json::to_value(read(&f.proxy)).unwrap()["snapshot"],
        Value::Null
    );
}

// Mutation: the 10-second close deadline synthesizes physical release and
// incorrectly replies safe while cleanupPending still retains ownership.
#[test]
fn stop_cleanup_pending_is_unsafe_until_late_actual_release() {
    let f = Fixture::new(Seed::Syncing);
    let current = f.window();
    let (result, reply) = pending();
    f.proxy.stop(reply);
    f.tick(10);
    assert_eq!(
        serde_json::to_value(read(&f.proxy)).unwrap()["state"],
        "cleanupPending"
    );
    assert!(result.lock().unwrap().is_none());
    assert!(f.native.borrow().held);
    f.event(&current, Event::Released);
    assert_eq!(take(&result), Ok(Code0Reply::Stopped));
    assert!(!f.native.borrow().held);
}

// Mutation: the bounded queue accepts a 65th general request or a full general
// queue loses the current physical Released / matching cleanup acknowledgement.
#[test]
fn saturated_general_queue_keeps_finite_release_and_cleanup_reservations() {
    let f = Fixture::new(Seed::Syncing);
    let current = f.window();
    assert_eq!(action(&f.proxy, Code0Action::Disconnect), Action::Started);
    f.hold();
    let mut reads = Vec::new();
    for _ in 0..64 {
        let (result, reply) = pending();
        f.proxy.read(reply);
        assert!(result.lock().unwrap().is_none());
        reads.push(result);
    }
    let (rejected, reply) = pending();
    f.proxy.read(reply);
    assert_eq!(take(&rejected), Err(Error::QueueFull));
    for _ in 0..1000 {
        f.event(&current, Event::Released);
    }
    f.flush();
    assert!(
        reads
            .iter()
            .all(|r| matches!(take(r), Ok(Code0Reply::View(_))))
    );
    assert_eq!(
        f.trace
            .borrow()
            .iter()
            .filter(|e| **e == "clear-request")
            .count(),
        1
    );
    let cleanup = f.native.borrow().cleanup.last().unwrap().clone();
    f.hold();
    for _ in 0..64 {
        f.proxy.read(Box::new(|_| {}));
    }
    for _ in 0..1000 {
        f.proxy.cleanup(BINDING, cleanup.clone(), Ok(())).unwrap();
    }
    f.flush();
    assert!(!f.native.borrow().held);
    assert_eq!(
        serde_json::to_value(read(&f.proxy)).unwrap()["state"],
        "disconnected"
    );
}

// Mutation: scheduler failure reports a successful action, retains a permanent
// scheduled flag, or falls back to executing the non-Send owner off the UI.
#[test]
fn scheduling_failure_replies_unavailable_and_never_runs_native_work() {
    let f = Fixture::new(Seed::Empty);
    f.executor
        .fail
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let (result, reply) = pending();
    f.proxy.action(Code0Action::Connect, reply);
    assert_eq!(take(&result), Err(Error::Unavailable));
    assert!(f.native.borrow().windows.is_empty());
    f.executor
        .fail
        .store(false, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(action(&f.proxy, Code0Action::Connect), Action::Started);
}

// Mutation: overflowing the general inbox rejects the RPC but leaves the
// current capture authorized, or loses its later physical release reservation.
#[test]
fn general_overflow_rejects_rpc_and_closes_the_current_capture() {
    let f = Fixture::new(Seed::Syncing);
    let current = f.window();
    f.hold();
    for _ in 0..64 {
        f.proxy.read(Box::new(|_| {}));
    }
    let (result, reply) = pending();
    f.proxy.read(reply);
    assert_eq!(take(&result), Err(Error::QueueFull));
    f.flush();
    let view = serde_json::to_value(read(&f.proxy)).unwrap();
    assert_eq!(view["state"], "closing");
    assert_eq!(view["failure"], "policyRejected");
    assert_eq!(
        f.trace
            .borrow()
            .iter()
            .filter(|event| **event == "close-request")
            .count(),
        1
    );
    for event in capture_events(&current) {
        f.event(&current, event);
    }
    assert!(!f.trace.borrow().contains(&"save"));
    f.event(&current, Event::Released);
    assert_eq!(
        serde_json::to_value(read(&f.proxy)).unwrap()["state"],
        "paused"
    );
}

// Mutation: pending Stop replies grow without limit or a repeated Stop moves
// the first owned close deadline forward, granting false/late exit permission.
#[test]
fn repeated_stop_is_bounded_and_does_not_restart_the_close_deadline() {
    let f = Fixture::new(Seed::Syncing);
    let current = f.window();
    let mut waiting = Vec::new();
    let (first, reply) = pending();
    let reentrant = Arc::new(Mutex::new(Vec::new()));
    let outcomes = reentrant.clone();
    let proxy = f.proxy.clone();
    f.proxy.stop(Box::new(move |value| {
        // While the other 63 old replies remain undelivered, only the single
        // just-completed admission may be reused, even from this callback.
        for _ in 0..64 {
            let outcomes = outcomes.clone();
            proxy.stop(Box::new(move |value| outcomes.lock().unwrap().push(value)));
        }
        reply(value);
    }));
    waiting.push(first);
    f.clock.0.set(9);
    for _ in 0..63 {
        let (result, reply) = pending();
        f.proxy.stop(reply);
        assert!(result.lock().unwrap().is_none());
        waiting.push(result);
    }
    let (overflow, reply) = pending();
    f.proxy.stop(reply);
    assert_eq!(take(&overflow), Err(Error::QueueFull));
    f.tick(10);
    assert_eq!(
        serde_json::to_value(read(&f.proxy)).unwrap()["state"],
        "cleanupPending"
    );
    assert!(
        waiting
            .iter()
            .all(|result| result.lock().unwrap().is_none())
    );
    assert_eq!(
        f.trace
            .borrow()
            .iter()
            .filter(|event| **event == "close-request")
            .count(),
        1
    );
    f.event(&current, Event::Released);
    for result in waiting {
        assert_eq!(take(&result), Ok(Code0Reply::Stopped));
    }
    let outcomes = reentrant.lock().unwrap();
    assert_eq!(outcomes.len(), 64);
    assert_eq!(
        outcomes
            .iter()
            .filter(|value| **value == Err(Error::QueueFull))
            .count(),
        63
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|value| **value == Ok(Code0Reply::Stopped))
            .count(),
        1
    );
    assert!(!f.native.borrow().held);
    assert_eq!(action(&f.proxy, Code0Action::Connect), Action::Denied);
}

// Mutation: exhausting the revision wraps or publishes changed same-revision
// content, discards cleanup evidence, or replies Stop-safe before cleanup.
#[test]
fn revision_exhaustion_freezes_view_but_still_waits_for_cleanup_stop_proof() {
    let f = Fixture::with_revision(Seed::Syncing, SAFE_MAX - 2);
    let current = f.window();
    assert_eq!(action(&f.proxy, Code0Action::Disconnect), Action::Started);
    let frozen = read(&f.proxy);
    assert_eq!(frozen.revision, SAFE_MAX);
    assert_eq!(
        serde_json::to_value(&frozen).unwrap()["snapshot"],
        Value::Null
    );
    let (stopped, reply) = pending();
    f.proxy.stop(reply);
    assert!(stopped.lock().unwrap().is_none());
    f.event(&current, Event::Released);
    assert!(stopped.lock().unwrap().is_none());
    assert!(f.native.borrow().held);
    let cleanup = f.native.borrow().cleanup.last().unwrap().clone();
    f.proxy.cleanup(BINDING, cleanup, Ok(())).unwrap();
    assert_eq!(take(&stopped), Ok(Code0Reply::Stopped));
    assert!(!f.native.borrow().held);
    assert_eq!(read(&f.proxy), frozen);
    let (denied, reply) = pending();
    f.proxy.action(Code0Action::Connect, reply);
    assert_eq!(take(&denied), Err(Error::RevisionExhausted));
    assert_eq!(f.native.borrow().windows.len(), 1);
}

// Mutation: an Active projection at the last safe revision is used as network
// authority, allowing the next tick to create a window that cannot be published.
#[test]
fn last_revision_tick_cannot_authorize_another_automatic_capture() {
    let f = Fixture::with_revision(Seed::Active, SAFE_MAX - 1);
    let frozen = read(&f.proxy);
    assert_eq!(frozen.revision, SAFE_MAX);
    f.tick(300);
    assert_eq!(f.native.borrow().windows.len(), 1);
    assert!(!f.native.borrow().held);
    assert_eq!(read(&f.proxy), frozen);
    let (result, reply) = pending();
    f.proxy.stop(reply);
    assert_eq!(take(&result), Ok(Code0Reply::Stopped));
}

// Compile-time transport contract, not proof of a real STA or COM callback.
#[test]
fn proxy_and_replies_are_send_sync_without_moving_the_owner() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Code0Proxy>();
    send_sync::<Code0SessionDto>();
    assert!(serde_json::from_value::<Code0Action>(json!("ordinaryRefresh")).is_err());
    assert_eq!(
        serde_json::to_value(Code0Action::LoginCompleted).unwrap(),
        "loginCompleted"
    );
    assert_eq!(
        serde_json::to_value(Code0Action::SyncNow).unwrap(),
        "syncNow"
    );
    assert_eq!(
        serde_json::to_value(Code0Reply::Action(Action::Started)).unwrap(),
        json!({"kind":"started"})
    );
    assert_eq!(
        serde_json::to_value(Code0Reply::Action(Action::Busy)).unwrap(),
        json!({"kind":"busy"})
    );
    assert_eq!(
        serde_json::to_value(Code0Reply::Action(Action::Denied)).unwrap(),
        json!({"kind":"denied"})
    );
    assert_eq!(
        serde_json::to_value(Code0Reply::Action(Action::Throttled {
            seconds_remaining: 300
        }))
        .unwrap(),
        json!({"kind":"throttled","secondsRemaining":300})
    );
    assert!(
        serde_json::to_value(Code0Reply::Action(Action::Throttled {
            seconds_remaining: 301
        }))
        .is_err()
    );
}
