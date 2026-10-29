//! Test-only UI-local runtime seam with an intentional settled-pump RED.
//! Product actions remain unavailable; no native owner, timer or bootstrap is installed.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use super::code0_dispatcher::{
    Code0Action, Code0DispatchError, Code0Dispatcher, Code0Proxy, Code0ReplySink, Code0Revision,
    Code0Schedule, Code0SessionDto,
};
use super::code0_web_session::{
    Code0CleanupRequest, Code0SessionClock, Code0SessionFailure, Code0SnapshotStoring,
    Code0WebSession, Code0WindowDriving, Code0WindowEvent, Code0WindowRequest,
};

type UiJob = Box<dyn FnOnce() + Send + 'static>;
type Emitter = Box<dyn FnMut(&Code0SessionDto)>;

/// The eventual AppHandle adapter posts only a binding-mark job, never a local owner.
trait MainThreadExecutor: Send + Sync {
    fn post(&self, job: UiJob) -> Result<(), Code0DispatchError>;
}
struct Schedule {
    binding: u64,
    executor: Arc<dyn MainThreadExecutor>,
}
impl Code0Schedule for Schedule {
    fn schedule(&self) -> Result<(), Code0DispatchError> {
        let binding = self.binding;
        self.executor
            .post(Box::new(move || {
                if let Some(record) = lookup(binding) {
                    record.mark_pending();
                }
            }))
            .map_err(|_| Code0DispatchError::Unavailable)
    }
}

/// Real backend methods remain the only callback producers. These merely queue.
#[derive(Clone)]
struct CallbackSink {
    binding: u64,
    proxy: Code0Proxy,
}
impl CallbackSink {
    fn window(
        &self,
        request: Code0WindowRequest,
        event: Code0WindowEvent,
    ) -> Result<(), Code0DispatchError> {
        self.proxy.window(self.binding, request, event)
    }
    fn cleanup(
        &self,
        request: Code0CleanupRequest,
        result: Result<(), Code0SessionFailure>,
    ) -> Result<(), Code0DispatchError> {
        self.proxy.cleanup(self.binding, request, result)
    }
}
trait UiBackend: Code0WindowDriving {
    fn on_window_event(&mut self, label: &str, event: &tauri::WindowEvent);
    fn poll_cleanup(&mut self);
}
struct DriverFacade<B> {
    binding: u64,
    proxy: Code0Proxy,
    backend: Rc<RefCell<B>>,
}
impl<B: UiBackend> Code0WindowDriving for DriverFacade<B> {
    fn acquire_profile_lease(&mut self) -> Result<(), Code0SessionFailure> {
        self.backend.borrow_mut().acquire_profile_lease()
    }
    fn release_profile_lease(&mut self) {
        self.backend.borrow_mut().release_profile_lease();
    }
    fn create_unloaded(&mut self, request: Code0WindowRequest) -> Result<(), Code0SessionFailure> {
        self.proxy
            .expect_window_release(self.binding, request.clone())
            .map_err(|_| Code0SessionFailure::PolicyRejected)?;
        self.backend.borrow_mut().create_unloaded(request)
    }
    fn navigate(&mut self, instance_id: &str) -> Result<(), Code0SessionFailure> {
        self.backend.borrow_mut().navigate(instance_id)
    }
    fn close(&mut self, instance_id: &str) {
        self.backend.borrow_mut().close(instance_id);
    }
    fn clear_website_data(
        &mut self,
        request: Code0CleanupRequest,
    ) -> Result<(), Code0SessionFailure> {
        self.proxy
            .expect_cleanup(self.binding, request.clone())
            .map_err(|_| Code0SessionFailure::CleanupFailed)?;
        self.backend.borrow_mut().clear_website_data(request)
    }
}

struct QueuedWindow {
    label: String,
    event: tauri::WindowEvent,
}
trait LocalRecord {
    fn binding(&self) -> u64;
    fn mark_pending(&self);
    fn committed_view(&self) -> Code0SessionDto;
    fn queue_window(&self, event: QueuedWindow) -> Result<(), Code0DispatchError>;
    fn settled(&self) -> TurnResult;
}
struct RegistrySlot {
    installing: bool,
    record: Option<Rc<dyn LocalRecord>>,
}
thread_local! {
    static REGISTRY: RefCell<RegistrySlot> = const {
        RefCell::new(RegistrySlot { installing: false, record: None })
    };
}
static NEXT_BINDING: AtomicU64 = AtomicU64::new(1);
fn lookup(binding: u64) -> Option<Rc<dyn LocalRecord>> {
    REGISTRY.with(|slot| {
        slot.borrow()
            .record
            .as_ref()
            .filter(|r| r.binding() == binding)
            .map(Rc::clone)
    })
}
struct InstallGuard;
impl Drop for InstallGuard {
    fn drop(&mut self) {
        REGISTRY.with(|slot| slot.borrow_mut().installing = false);
    }
}
struct TurnGuard<'a>(&'a Cell<bool>);
impl Drop for TurnGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TurnResult {
    Missing,
    Deferred,
    Settled,
}
fn window_event(
    binding: u64,
    label: String,
    event: tauri::WindowEvent,
) -> Result<bool, Code0DispatchError> {
    let Some(record) = lookup(binding) else {
        return Ok(false);
    };
    record.queue_window(QueuedWindow { label, event })?;
    Ok(true)
}
fn passive_read(binding: u64) -> Result<Code0SessionDto, Code0DispatchError> {
    lookup(binding)
        .map(|record| record.committed_view())
        .ok_or(Code0DispatchError::Unavailable)
}
fn settled_turn(binding: u64) -> TurnResult {
    let Some(record) = lookup(binding) else {
        return TurnResult::Missing;
    };
    record.settled()
}
struct InstallInputs<C, S, F> {
    clock: C,
    store: S,
    backend: F,
    emit: Emitter,
}
struct Runtime<C, B, S> {
    binding: u64,
    proxy: Code0Proxy,
    dispatcher: Code0Dispatcher<C, DriverFacade<B>, S>,
    backend: Rc<RefCell<B>>,
    pending: Cell<bool>,
    in_turn: Cell<bool>,
    windows: RefCell<VecDeque<QueuedWindow>>,
}
impl<C, B, S> Runtime<C, B, S> {
    fn passive_view(&self) -> Code0SessionDto {
        self.proxy.committed()
    }
    /// The real product policy/chooser/clear/terminal routes are not ready.
    fn production_action(&self, _: Code0Action, reply: Code0ReplySink) {
        reply(Err(Code0DispatchError::Unavailable));
    }
}
impl<C, B, S> LocalRecord for Runtime<C, B, S>
where
    C: Code0SessionClock,
    B: UiBackend,
    S: Code0SnapshotStoring,
{
    fn binding(&self) -> u64 {
        self.binding
    }
    fn mark_pending(&self) {
        self.pending.set(true);
    }
    fn committed_view(&self) -> Code0SessionDto {
        self.passive_view()
    }
    fn queue_window(&self, event: QueuedWindow) -> Result<(), Code0DispatchError> {
        // This local deferral queue stays bounded through native reentry. Overflow
        // stops admissions through the existing proxy, without inventing an ACK.
        let accepted = {
            let mut events = self.windows.borrow_mut();
            if events.len() >= 64 {
                false
            } else {
                events.push_back(event);
                true
            }
        };
        self.pending.set(true);
        if !accepted {
            self.proxy.stop(Box::new(|_| {}));
            Err(Code0DispatchError::Unavailable)
        } else {
            Ok(())
        }
    }
    fn settled(&self) -> TurnResult {
        if self.in_turn.replace(true) {
            self.pending.set(true);
            return TurnResult::Deferred;
        }
        let _turn = TurnGuard(&self.in_turn);
        self.pending.set(false);
        // Only deliveries present at entry are forwarded in this turn. Nested
        // WindowEvents remain owned in the local queue for the next settled turn.
        let count = self.windows.borrow().len();
        for _ in 0..count {
            let event = self.windows.borrow_mut().pop_front();
            if let Some(event) = event {
                {
                    self.backend
                        .borrow_mut()
                        .on_window_event(&event.label, &event.event);
                }
                drop(event);
            }
        }
        {
            self.backend.borrow_mut().poll_cleanup();
        }
        // Intentional sole behavioral RED gap: queued proxy work is not drained
        // here yet. Root must observe real assertions before adding that pump.
        TurnResult::Settled
    }
}
fn install<C, B, S, F>(
    executor: Arc<dyn MainThreadExecutor>,
    inputs: InstallInputs<C, S, F>,
) -> Result<Rc<Runtime<C, B, S>>, Code0DispatchError>
where
    C: Code0SessionClock + Clone + 'static,
    B: UiBackend + 'static,
    S: Code0SnapshotStoring + 'static,
    F: FnOnce(CallbackSink) -> B,
{
    let reserved = REGISTRY.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.installing || slot.record.is_some() {
            false
        } else {
            slot.installing = true;
            true
        }
    });
    if !reserved {
        return Err(Code0DispatchError::Unavailable);
    }
    let _install = InstallGuard;
    let binding = NEXT_BINDING
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_add(1))
        .map_err(|_| Code0DispatchError::Unavailable)?;
    let schedule = Arc::new(Schedule { binding, executor });
    let proxy = Code0Proxy::new(schedule, Code0Revision::new(0)?);
    let backend = Rc::new(RefCell::new((inputs.backend)(CallbackSink {
        binding,
        proxy: proxy.clone(),
    })));
    let facade = DriverFacade {
        binding,
        proxy: proxy.clone(),
        backend: Rc::clone(&backend),
    };
    let session =
        Code0WebSession::new(inputs.clock.clone(), facade, inputs.store, Box::new(|_| {}));
    let dispatcher = Code0Dispatcher::install(
        binding,
        binding,
        session,
        inputs.clock,
        proxy.clone(),
        inputs.emit,
    )?;
    let runtime = Rc::new(Runtime {
        binding,
        proxy,
        dispatcher,
        backend,
        pending: Cell::new(false),
        in_turn: Cell::new(false),
        windows: RefCell::new(VecDeque::new()),
    });
    let record: Rc<dyn LocalRecord> = runtime.clone();
    REGISTRY.with(|slot| slot.borrow_mut().record = Some(record));
    Ok(runtime)
}

#[cfg(test)]
mod tests {
    use super::super::code0_dispatcher::{Code0Action, Code0Reply};
    use super::super::code0_web_session::{Code0SessionActionResult as Action, Code0StoreError};
    use super::*;
    use crate::collectors::code0_capture::Code0WebsiteCapture;
    use crate::domain::{UsageSnapshot, UsageStatus};
    use std::sync::Mutex;
    use std::sync::atomic::AtomicBool;
    use std::thread::{self, ThreadId};
    use std::time::Duration;

    type ReplySlot = Arc<Mutex<Option<Result<Code0Reply, Code0DispatchError>>>>;
    fn slot() -> ReplySlot {
        Arc::new(Mutex::new(None))
    }
    fn reply_to(slot: &ReplySlot) -> Code0ReplySink {
        let slot = Arc::clone(slot);
        Box::new(move |reply| *slot.lock().unwrap() = Some(reply))
    }
    #[derive(Clone)]
    struct Clock;
    impl Code0SessionClock for Clock {
        fn now(&self) -> Duration {
            Duration::ZERO
        }
        fn wall_now(&self) -> String {
            "2026-10-29T00:00:00Z".into()
        }
    }
    #[derive(Default)]
    struct StoreState {
        loads: usize,
        removes: usize,
        fail_remove: bool,
    }
    struct Store(Rc<RefCell<StoreState>>);
    impl Code0SnapshotStoring for Store {
        fn load(&self) -> Result<Option<UsageSnapshot>, Code0StoreError> {
            self.0.borrow_mut().loads += 1;
            let mut value = Code0WebsiteCapture::decode_filtered_json(
                br#"{"quota":49400,"usedQuota":182700,"divisor":400}"#,
            )
            .unwrap()
            .make_snapshot("2026-10-29T00:00:00Z")
            .unwrap();
            value.status = UsageStatus::Cached;
            Ok(Some(value))
        }
        fn save_atomic(&mut self, _: &UsageSnapshot) -> Result<(), Code0StoreError> {
            Err(Code0StoreError::Unavailable)
        }
        fn remove(&mut self) -> Result<(), Code0StoreError> {
            let mut state = self.0.borrow_mut();
            state.removes += 1;
            if state.fail_remove {
                Err(Code0StoreError::Unavailable)
            } else {
                Ok(())
            }
        }
    }
    struct Executor {
        ui: ThreadId,
        fail: AtomicBool,
        hold: AtomicBool,
        jobs: Mutex<VecDeque<UiJob>>,
    }
    impl Executor {
        fn new() -> Self {
            Self {
                ui: thread::current().id(),
                fail: AtomicBool::new(false),
                hold: AtomicBool::new(false),
                jobs: Mutex::new(VecDeque::new()),
            }
        }
        fn flush(&self) {
            assert_eq!(thread::current().id(), self.ui);
            loop {
                let job = self.jobs.lock().unwrap().pop_front();
                let Some(job) = job else { break };
                job();
            }
        }
    }
    impl MainThreadExecutor for Executor {
        fn post(&self, job: UiJob) -> Result<(), Code0DispatchError> {
            if self.fail.load(Ordering::SeqCst) {
                return Err(Code0DispatchError::Unavailable);
            }
            if thread::current().id() == self.ui && !self.hold.load(Ordering::SeqCst) {
                job();
            } else {
                self.jobs.lock().unwrap().push_back(job);
            }
            Ok(())
        }
    }
    #[derive(Default)]
    struct BackendState {
        calls: Vec<&'static str>,
        requests: Vec<Code0WindowRequest>,
        window_events: usize,
        release_callbacks: usize,
        create_error: bool,
        release_in_create: bool,
        cleanup_inline: bool,
        poll_hook: Option<Rc<dyn Fn()>>,
    }
    struct Backend {
        callbacks: CallbackSink,
        state: Rc<RefCell<BackendState>>,
        current: Option<Code0WindowRequest>,
        release_at_poll: Option<Code0WindowRequest>,
    }
    impl Code0WindowDriving for Backend {
        fn acquire_profile_lease(&mut self) -> Result<(), Code0SessionFailure> {
            self.state.borrow_mut().calls.push("acquire");
            Ok(())
        }
        fn release_profile_lease(&mut self) {
            self.state.borrow_mut().calls.push("release-lease");
        }
        fn create_unloaded(
            &mut self,
            request: Code0WindowRequest,
        ) -> Result<(), Code0SessionFailure> {
            let (release, fail) = {
                let mut s = self.state.borrow_mut();
                s.calls.push("create");
                s.requests.push(request.clone());
                (s.release_in_create, s.create_error)
            };
            self.current = Some(request.clone());
            if release {
                self.current = None;
                self.state.borrow_mut().release_callbacks += 1;
                self.callbacks
                    .window(request, Code0WindowEvent::Released)
                    .unwrap();
            }
            if fail {
                Err(Code0SessionFailure::PolicyRejected)
            } else {
                Ok(())
            }
        }
        fn navigate(&mut self, _: &str) -> Result<(), Code0SessionFailure> {
            self.state.borrow_mut().calls.push("navigate");
            Err(Code0SessionFailure::PolicyRejected)
        }
        fn close(&mut self, _: &str) {
            self.state.borrow_mut().calls.push("close");
        }
        fn clear_website_data(
            &mut self,
            request: Code0CleanupRequest,
        ) -> Result<(), Code0SessionFailure> {
            let inline = {
                let mut s = self.state.borrow_mut();
                s.calls.push("clear");
                s.cleanup_inline
            };
            if inline {
                self.callbacks.cleanup(request, Ok(())).unwrap();
            }
            Ok(())
        }
    }
    impl UiBackend for Backend {
        fn on_window_event(&mut self, label: &str, event: &tauri::WindowEvent) {
            self.state.borrow_mut().window_events += 1;
            if self
                .current
                .as_ref()
                .is_some_and(|r| label.strip_prefix("owned-") == Some(r.instance_id.as_str()))
                && matches!(event, tauri::WindowEvent::Destroyed)
            {
                let request = self.current.take().unwrap();
                self.callbacks
                    .window(
                        request.clone(),
                        Code0WindowEvent::Failed(Code0SessionFailure::PolicyRejected),
                    )
                    .unwrap();
                self.release_at_poll = Some(request);
            }
        }
        fn poll_cleanup(&mut self) {
            self.state.borrow_mut().calls.push("poll");
            let hook = self.state.borrow_mut().poll_hook.take();
            if let Some(hook) = hook {
                hook();
            }
            if let Some(request) = self.release_at_poll.take() {
                self.state.borrow_mut().release_callbacks += 1;
                self.callbacks
                    .window(request, Code0WindowEvent::Released)
                    .unwrap();
            }
        }
    }
    type TestRuntime = Runtime<Clock, Backend, Store>;
    struct Fixture {
        runtime: Rc<TestRuntime>,
        executor: Arc<Executor>,
        backend: Rc<RefCell<BackendState>>,
        store: Rc<RefCell<StoreState>>,
    }
    impl Fixture {
        fn new() -> Self {
            let executor = Arc::new(Executor::new());
            let backend = Rc::new(RefCell::new(BackendState::default()));
            let store = Rc::new(RefCell::new(StoreState::default()));
            let state = Rc::clone(&backend);
            let runtime = install(
                executor.clone(),
                InstallInputs {
                    clock: Clock,
                    store: Store(Rc::clone(&store)),
                    backend: move |callbacks| Backend {
                        callbacks,
                        state,
                        current: None,
                        release_at_poll: None,
                    },
                    emit: Box::new(|_| {}),
                },
            )
            .unwrap();
            Self {
                runtime,
                executor,
                backend,
                store,
            }
        }
        // Component control only: intentionally bypasses the missing settled pump.
        fn component_drain(&self) {
            self.runtime.dispatcher.drain();
        }
        fn view(&self) -> serde_json::Value {
            serde_json::to_value(passive_read(self.runtime.binding).unwrap()).unwrap()
        }
        fn remove_record(&self) {
            let retired = REGISTRY.with(|slot| {
                let mut slot = slot.borrow_mut();
                if slot
                    .record
                    .as_ref()
                    .is_some_and(|r| r.binding() == self.runtime.binding)
                {
                    slot.record.take()
                } else {
                    None
                }
            });
            drop(retired);
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            self.remove_record();
        }
    }

    // Mutation: install/read/action admission acquires or creates instead of staying cold.
    #[test]
    fn cold_install_passive_read_and_production_action_are_inert() {
        assert_eq!(passive_read(0), Err(Code0DispatchError::Unavailable));
        let f = Fixture::new();
        assert_eq!(f.view()["state"], "paused");
        assert_eq!(f.view()["isAutomaticallySyncing"], false);
        assert!(!f.view()["snapshot"].is_null());
        let reply = slot();
        f.runtime
            .production_action(Code0Action::SyncNow, reply_to(&reply));
        assert_eq!(
            reply.lock().unwrap().clone(),
            Some(Err(Code0DispatchError::Unavailable))
        );
        assert_eq!(f.store.borrow().loads, 1);
        assert!(f.backend.borrow().calls.is_empty());
    }
    // RED: omitting the real settled dispatcher drain leaves the worker's accepted RPC unanswered.
    #[test]
    fn settled_worker_action_replies_on_the_ui_owner() {
        let f = Fixture::new();
        let reply = slot();
        let output = Arc::clone(&reply);
        let replied_on = Arc::new(Mutex::new(None));
        let callback_thread = Arc::clone(&replied_on);
        let proxy = f.runtime.proxy.clone();
        thread::spawn(move || {
            proxy.action(
                Code0Action::SyncNow,
                Box::new(move |value| {
                    *callback_thread.lock().unwrap() = Some(thread::current().id());
                    *output.lock().unwrap() = Some(value);
                }),
            )
        })
        .join()
        .unwrap();
        f.executor.flush();
        assert!(f.runtime.pending.get());
        settled_turn(f.runtime.binding);
        assert_eq!(
            reply.lock().unwrap().clone(),
            Some(Ok(Code0Reply::Action(Action::Started))),
            "settled worker action must reply through the real dispatcher"
        );
        assert_eq!(*replied_on.lock().unwrap(), Some(thread::current().id()));
        assert_eq!(f.view()["state"], "syncing");
    }
    // RED: a queued read must return the already committed view at a settled boundary.
    #[test]
    fn settled_read_replies_without_starting_native_work() {
        let f = Fixture::new();
        let reply = slot();
        f.runtime.proxy.read(reply_to(&reply));
        settled_turn(f.runtime.binding);
        let value = reply.lock().unwrap().clone();
        assert!(
            matches!(&value, Some(Ok(Code0Reply::View(_)))),
            "settled read must reach real dispatcher"
        );
        let Some(Ok(Code0Reply::View(view))) = value else {
            unreachable!()
        };
        assert_eq!(serde_json::to_value(view).unwrap()["state"], "paused");
        assert!(!f.backend.borrow().calls.contains(&"acquire"));
    }
    // RED: real controlled backend event callbacks enqueue, but only settled routing updates Session.
    #[test]
    fn window_event_and_poll_callbacks_reach_the_real_session() {
        let f = Fixture::new();
        f.runtime
            .proxy
            .action(Code0Action::SyncNow, Box::new(|_| {}));
        f.component_drain();
        assert_eq!(f.view()["state"], "syncing");
        let request = f.backend.borrow().requests[0].clone();
        window_event(
            f.runtime.binding,
            format!("owned-{}", request.instance_id),
            tauri::WindowEvent::Destroyed,
        )
        .unwrap();
        settled_turn(f.runtime.binding);
        assert_eq!(f.backend.borrow().window_events, 1);
        assert_eq!(f.backend.borrow().release_callbacks, 1);
        assert_eq!(
            f.view()["state"],
            "paused",
            "forwarded native callbacks must reach the real Session"
        );
    }
    // Mutation: reserve after create drops its inline Released, including a failed-create return.
    #[test]
    fn failed_create_keeps_its_inline_released_reservation() {
        let f = Fixture::new();
        {
            let mut s = f.backend.borrow_mut();
            s.release_in_create = true;
            s.create_error = true;
        }
        f.runtime
            .proxy
            .action(Code0Action::SyncNow, Box::new(|_| {}));
        f.component_drain();
        assert_eq!(f.backend.borrow().release_callbacks, 1);
        assert_eq!(f.view()["state"], "paused");
        assert_eq!(f.view()["failure"], "policyRejected");
    }
    // Mutation: missing pre-clear reservation loses the actual backend's inline cleanup disposition.
    #[test]
    fn inline_cleanup_is_reserved_before_the_backend_call() {
        let f = Fixture::new();
        f.backend.borrow_mut().cleanup_inline = true;
        f.runtime
            .proxy
            .action(Code0Action::Disconnect, Box::new(|_| {}));
        f.component_drain();
        assert_eq!(f.view()["state"], "disconnected");
        assert!(f.view()["snapshot"].is_null());
        assert_eq!(f.store.borrow().removes, 1);
        assert!(f.backend.borrow().calls.contains(&"clear"));
    }
    // Mutation: ignoring a failed reservation issues native work on an unbound proxy.
    #[test]
    fn reservation_failure_prevents_create_and_cleanup_calls() {
        let executor = Arc::new(Executor::new());
        let proxy = Code0Proxy::new(
            Arc::new(Schedule {
                binding: 777,
                executor,
            }),
            Code0Revision::new(0).unwrap(),
        );
        let state = Rc::new(RefCell::new(BackendState::default()));
        let backend = Rc::new(RefCell::new(Backend {
            callbacks: CallbackSink {
                binding: 777,
                proxy: proxy.clone(),
            },
            state: Rc::clone(&state),
            current: None,
            release_at_poll: None,
        }));
        let mut facade = DriverFacade {
            binding: 777,
            proxy,
            backend,
        };
        let request = Code0WindowRequest {
            instance_id: "i".into(),
            connection_epoch: "e".into(),
            nonce: "n".into(),
            document_id: "d".into(),
            mode: super::super::code0_web_session::Code0WindowMode::Sync,
        };
        assert_eq!(
            facade.create_unloaded(request),
            Err(Code0SessionFailure::PolicyRejected)
        );
        assert_eq!(
            facade.clear_website_data(Code0CleanupRequest {
                connection_epoch: "e".into(),
                cleanup_id: "c".into()
            }),
            Err(Code0SessionFailure::CleanupFailed)
        );
        assert!(state.borrow().calls.is_empty());
    }
    // Mutation: lookup by reused label/current owner lets an old Send closure mark the replacement.
    #[test]
    fn stale_binding_cannot_mark_or_forward_to_a_replacement() {
        let old = Fixture::new();
        old.executor.hold.store(true, Ordering::SeqCst);
        Schedule {
            binding: old.runtime.binding,
            executor: old.executor.clone(),
        }
        .schedule()
        .unwrap();
        assert!(!old.runtime.pending.get());
        old.remove_record();
        let replacement = Fixture::new();
        assert_ne!(old.runtime.binding, replacement.runtime.binding);
        old.executor.flush();
        assert!(!replacement.runtime.pending.get());
        assert!(
            !window_event(
                old.runtime.binding,
                "old".into(),
                tauri::WindowEvent::Destroyed
            )
            .unwrap()
        );
        assert_eq!(replacement.backend.borrow().window_events, 0);
        assert_eq!(replacement.view()["state"], "paused");
    }
    // Mutation: inline schedule/nested settled entry drains or reborrows the active backend.
    #[test]
    fn nested_schedule_and_window_turn_are_deferred_without_reborrowing() {
        let f = Fixture::new();
        let binding = f.runtime.binding;
        let proxy = f.runtime.proxy.clone();
        let nested = Rc::new(Cell::new(None));
        let observed = Rc::clone(&nested);
        f.backend.borrow_mut().poll_hook = Some(Rc::new(move || {
            proxy.read(Box::new(|_| {}));
            window_event(binding, "nested".into(), tauri::WindowEvent::Destroyed).unwrap();
            observed.set(Some(settled_turn(binding)));
        }));
        assert_eq!(settled_turn(binding), TurnResult::Settled);
        assert_eq!(nested.get(), Some(TurnResult::Deferred));
        assert!(f.runtime.pending.get());
        assert_eq!(f.backend.borrow().window_events, 0);
        settled_turn(binding);
        assert_eq!(f.backend.borrow().window_events, 1);
    }
    // Mutation: treating scheduling failure as admission or physical release starts native work.
    #[test]
    fn scheduling_failure_returns_unavailable_without_forged_release() {
        let f = Fixture::new();
        let before = f.runtime.passive_view();
        f.executor.fail.store(true, Ordering::SeqCst);
        let reply = slot();
        f.runtime
            .proxy
            .action(Code0Action::SyncNow, reply_to(&reply));
        assert_eq!(
            reply.lock().unwrap().clone(),
            Some(Err(Code0DispatchError::Unavailable))
        );
        assert_eq!(f.runtime.passive_view(), before);
        assert!(f.backend.borrow().calls.is_empty());
        assert_eq!(f.backend.borrow().release_callbacks, 0);
    }
    // Mutation: dropping null revocation or invoking emitter/reply before commit hides removed authority.
    #[test]
    fn committed_null_revocation_is_emitted_before_the_action_reply() {
        let executor = Arc::new(Executor::new());
        let backend = Rc::new(RefCell::new(BackendState::default()));
        let store = Rc::new(RefCell::new(StoreState {
            fail_remove: true,
            ..StoreState::default()
        }));
        let trace = Arc::new(Mutex::new(Vec::new()));
        let emitted = Arc::clone(&trace);
        let state = Rc::clone(&backend);
        let handle = Rc::new(RefCell::new(None::<std::rc::Weak<TestRuntime>>));
        let on_emit = Rc::clone(&handle);
        let runtime = install(
            executor.clone(),
            InstallInputs {
                clock: Clock,
                store: Store(Rc::clone(&store)),
                backend: move |callbacks| Backend {
                    callbacks,
                    state,
                    current: None,
                    release_at_poll: None,
                },
                emit: Box::new(move |view| {
                    let local = on_emit.borrow().as_ref().unwrap().upgrade().unwrap();
                    assert_eq!(local.proxy.committed(), *view);
                    assert!(local.backend.try_borrow_mut().is_ok());
                    assert!(REGISTRY.with(|r| r.try_borrow_mut().is_ok()));
                    assert!(serde_json::to_value(view).unwrap()["snapshot"].is_null());
                    emitted.lock().unwrap().push("emit");
                }),
            },
        )
        .unwrap();
        *handle.borrow_mut() = Some(Rc::downgrade(&runtime));
        let f = Fixture {
            runtime,
            executor,
            backend,
            store,
        };
        let reply_order = Arc::clone(&trace);
        // The dispatcher reply is Send; the UI-local emitter deliberately remains Rc-local.
        f.runtime.proxy.action(
            Code0Action::Disconnect,
            Box::new(move |value| {
                assert_eq!(value, Ok(Code0Reply::Action(Action::Started)));
                reply_order.lock().unwrap().push("reply");
            }),
        );
        f.component_drain();
        assert_eq!(*trace.lock().unwrap(), vec!["emit", "reply"]);
        assert!(f.view()["snapshot"].is_null());
        assert_eq!(f.view()["state"], "cleanupPending");
    }
}
