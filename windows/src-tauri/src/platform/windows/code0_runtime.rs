//! Test-only UI-local runtime seam with a guarded settled dispatcher pump.
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

pub(super) type UiJob = Box<dyn FnOnce() + Send + 'static>;
type Emitter = Box<dyn FnMut(&Code0SessionDto)>;

/// The eventual AppHandle adapter posts only a binding-mark job, never a local owner.
pub(super) trait MainThreadExecutor: Send + Sync {
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeTerminalState {
    Pending,
    Quiescent,
}
trait UiBackend: Code0WindowDriving {
    fn on_window_event(&mut self, label: &str, event: &tauri::WindowEvent);
    fn poll_cleanup(&mut self);
    fn terminal_state(&self) -> NativeTerminalState {
        NativeTerminalState::Pending
    }
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
    fn attach_tick(
        &self,
        ack: super::code0_tick::TickAcknowledgement,
    ) -> Result<(), Code0DispatchError>;
    fn mark_tick(&self);
    fn tick_attached(&self, ack: &super::code0_tick::TickAcknowledgement) -> bool;
    fn committed_view(&self) -> Code0SessionDto;
    fn queue_window(&self, event: QueuedWindow) -> Result<(), Code0DispatchError>;
    fn settled(&self) -> TurnResult;
    fn request_terminal(&self) -> Result<(), Code0DispatchError>;
    fn terminal_progress(&self);
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
pub(super) fn attach_tick(
    binding: u64,
    ack: super::code0_tick::TickAcknowledgement,
) -> Result<(), Code0DispatchError> {
    lookup(binding)
        .ok_or(Code0DispatchError::Unavailable)?
        .attach_tick(ack)
}
pub(super) fn tick_attached(binding: u64, ack: &super::code0_tick::TickAcknowledgement) -> bool {
    lookup(binding).is_some_and(|record| record.tick_attached(ack))
}
pub(super) fn mark_tick(binding: u64) {
    if let Some(record) = lookup(binding) {
        record.mark_tick();
    }
}
fn request_terminal(binding: u64) -> Result<(), Code0DispatchError> {
    lookup(binding)
        .ok_or(Code0DispatchError::Unavailable)?
        .request_terminal()
}
// Deliberately not Clone/Copy: a later GREEN owner must mint and consume once.
struct TerminalPermit {
    binding: u64,
    operation: u64,
}
struct TerminalJoinReceipt {
    binding: u64,
    operation: u64,
    result: std::thread::Result<super::code0_tick::TickExit>,
    thread: std::thread::ThreadId,
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
    let result = record.settled();
    if result == TurnResult::Settled {
        // The guarded poll/drain/reply turn has returned before terminal progress.
        record.terminal_progress();
    }
    result
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
    tick_due: Cell<bool>,
    fault_handled: Cell<bool>,
    tick_ack: RefCell<Option<super::code0_tick::TickAcknowledgement>>,
    in_turn: Cell<bool>,
    windows: RefCell<VecDeque<QueuedWindow>>,
    terminal_tick: RefCell<Option<super::code0_tick::TickProducer>>,
    terminal_operation: Cell<Option<u64>>,
    terminal_join: RefCell<Option<TerminalJoinReceipt>>,
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
impl<C, B, S> Runtime<C, B, S>
where
    C: Code0SessionClock,
    B: UiBackend,
    S: Code0SnapshotStoring,
{
    fn adopt_tick(
        &self,
        producer: super::code0_tick::TickProducer,
    ) -> Result<(), Code0DispatchError> {
        let mut slot = self.terminal_tick.borrow_mut();
        if slot.is_some() || !self.tick_attached(&producer.acknowledgement()) {
            return Err(Code0DispatchError::Unavailable);
        }
        *slot = Some(producer);
        Ok(())
    }
    fn terminal_conditions_met(&self) -> bool {
        !self.in_turn.get()
            && !self.pending.get()
            && !self.tick_due.get()
            && self.windows.borrow().is_empty()
            && self.terminal_operation.get().is_some()
            && self.backend.borrow().terminal_state() == NativeTerminalState::Quiescent
            && self.dispatcher.terminal_prerequisites_met()
    }
    fn take_terminal_permit(&self) -> Option<TerminalPermit> {
        // RED seam: retaining a producer/Stop/empty queues never proves a join.
        None
    }
    fn consume_terminal_permit(&self, permit: TerminalPermit) -> bool {
        let current = lookup(permit.binding);
        permit.binding == self.binding
            && current.is_some_and(|record| record.binding() == self.binding)
            && self.terminal_operation.get() == Some(permit.operation)
            && self.terminal_conditions_met()
            && self.terminal_join.borrow().as_ref().is_some_and(|receipt| {
                receipt.binding == permit.binding
                    && receipt.operation == permit.operation
                    && matches!(receipt.result, Ok(super::code0_tick::TickExit::Cancelled))
                    && receipt.thread != std::thread::current().id()
            })
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
    fn request_terminal(&self) -> Result<(), Code0DispatchError> {
        if self.terminal_operation.get().is_none() {
            // A registry binding is process-unique and this record has one lifetime
            // terminal operation. Repeated requests retain the same private key.
            self.terminal_operation.set(Some(self.binding));
            self.proxy.begin_terminal_stop();
        }
        Ok(())
    }
    fn terminal_progress(&self) {
        // Intentionally conservative RED: evaluate the real settled conditions,
        // retaining the owned producer until an actual join owner is implemented.
        let _ = self.terminal_conditions_met();
    }
    fn mark_pending(&self) {
        self.pending.set(true);
    }
    fn attach_tick(
        &self,
        ack: super::code0_tick::TickAcknowledgement,
    ) -> Result<(), Code0DispatchError> {
        let mut slot = self.tick_ack.borrow_mut();
        if slot.is_some() || !ack.attach(self.binding) {
            return Err(Code0DispatchError::Unavailable);
        }
        *slot = Some(ack);
        Ok(())
    }
    fn tick_attached(&self, ack: &super::code0_tick::TickAcknowledgement) -> bool {
        self.tick_ack
            .borrow()
            .as_ref()
            .is_some_and(|installed| installed.matches(ack))
    }
    fn mark_tick(&self) {
        let ack = self.tick_ack.borrow().clone();
        if let Some(ack) = ack
            && ack.delivered(self.binding)
        {
            self.tick_due.set(true);
            self.pending.set(true);
        }
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
        // Snapshot before external calls; a nested ACK remains due next turn.
        let tick_due = self.tick_due.replace(false);
        // Only deliveries present at entry are forwarded in this turn. Nested
        // WindowEvents remain owned in the local queue for the next settled turn.
        let count = self.windows.borrow().len();
        let faulted = self
            .tick_ack
            .borrow()
            .as_ref()
            .is_some_and(|ack| ack.faulted());
        if faulted && !self.fault_handled.replace(true) {
            // Fence admissions once, outside ACK/backend borrows. Stop does not
            // cancel polling or prove terminal quiescence or a worker join.
            self.proxy.stop(Box::new(|_| {}));
        }
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
        // Native/backend borrows end before real queued work is dispatched.
        // The turn guard defers synchronous nested scheduling/window delivery.
        if tick_due {
            // install binds this dispatcher to (binding, binding) as its tick ID.
            let _ = self.proxy.tick(self.binding, self.binding);
        }
        self.dispatcher.drain();
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
        tick_due: Cell::new(false),
        fault_handled: Cell::new(false),
        tick_ack: RefCell::new(None),
        in_turn: Cell::new(false),
        windows: RefCell::new(VecDeque::new()),
        terminal_tick: RefCell::new(None),
        terminal_operation: Cell::new(None),
        terminal_join: RefCell::new(None),
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
    // New tests use the existing real runtime registry/Fixture. No Session algorithm is copied.
    mod tick_tests {
        use super::super::super::code0_tick::{TickExit, TickProducer, TickStartFailure};
        use super::*;
        use std::sync::mpsc::{self, Receiver, SyncSender};
        use std::time::Instant;

        struct Posted {
            job: Option<UiJob>,
            at: Instant,
            thread: ThreadId,
        }
        struct TickExecutor {
            reject: bool,
            sends: SyncSender<Posted>,
            calls: std::sync::atomic::AtomicUsize,
        }
        impl TickExecutor {
            fn new(reject: bool) -> (Arc<Self>, Receiver<Posted>) {
                let (sends, receives) = mpsc::sync_channel(1);
                (
                    Arc::new(Self {
                        reject,
                        sends,
                        calls: std::sync::atomic::AtomicUsize::new(0),
                    }),
                    receives,
                )
            }
        }
        impl MainThreadExecutor for TickExecutor {
            fn post(&self, job: UiJob) -> Result<(), Code0DispatchError> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                let event = Posted {
                    job: (!self.reject).then_some(job),
                    at: Instant::now(),
                    thread: thread::current().id(),
                };
                let delivered = self.sends.try_send(event).is_ok();
                if self.reject || !delivered {
                    Err(Code0DispatchError::Unavailable)
                } else {
                    Ok(())
                }
            }
        }
        // Test cleanup only. Closing control on take_join also terminates a worker
        // if cancellation is accidentally broken; no worker ever waits for UI ACK.
        struct Running(TickProducer);
        impl Running {
            fn finish(&mut self) -> Option<Option<TickExit>> {
                self.0.cancel();
                self.0.take_join().map(|handle| handle.join().ok())
            }
        }
        impl Drop for Running {
            fn drop(&mut self) {
                let _ = self.finish();
            }
        }
        const OBSERVE: Duration = Duration::from_millis(1250);

        // Mutation: no periodic executor post leaves an idle installed record unmarked.
        #[test]
        fn tick_idle_worker_posts_to_the_real_matching_record() {
            let f = Fixture::new();
            let ui = thread::current().id();
            let (executor, events) = TickExecutor::new(false);
            let mut worker = Running(TickProducer::prepare(f.runtime.binding, executor));
            let attached = attach_tick(f.runtime.binding, worker.0.acknowledgement());
            let started = worker.0.start();
            let event = events.recv_timeout(OBSERVE).ok();
            let posted_on = event.as_ref().map(|v| v.thread);
            if let Some(mut event) = event
                && let Some(job) = event.job.take()
            {
                job();
            }
            let marked = f.runtime.pending.get();
            let joined = worker.finish();
            assert_eq!(attached, Ok(()));
            assert_eq!(started, Ok(()));
            assert_eq!(joined, Some(Some(TickExit::Cancelled)));
            assert!(
                posted_on.is_some_and(|id| id != ui),
                "idle worker emitted no off-UI pulse"
            );
            assert!(
                marked,
                "matching binding pulse did not mark the actual runtime record"
            );
            assert!(f.backend.borrow().calls.is_empty());
        }

        // Mutations: repeated unacknowledged posts grow work; missing actual UI ACK stalls rearm.
        #[test]
        fn tick_one_unacknowledged_job_coalesces_and_real_ui_ack_rearms() {
            let f = Fixture::new();
            let (executor, events) = TickExecutor::new(false);
            let mut worker = Running(TickProducer::prepare(f.runtime.binding, executor.clone()));
            let attached = attach_tick(f.runtime.binding, worker.0.acknowledgement());
            let started = worker.0.start();
            let first = events.recv_timeout(OBSERVE).ok();
            let mut held_calls = 0;
            let mut extra = None;
            let mut next = None;
            if let Some(mut first) = first {
                extra = events.recv_timeout(Duration::from_millis(650)).ok();
                held_calls = executor.calls.load(Ordering::SeqCst);
                if let Some(job) = first.job.take() {
                    job(); // Real registry lookup/ACK/mark, never a manual ACK substitute.
                }
                next = events.recv_timeout(OBSERVE).ok();
            }
            let rearmed = next.is_some();
            let joined = worker.finish();
            assert_eq!(attached, Ok(()));
            assert_eq!(started, Ok(()));
            assert_eq!(joined, Some(Some(TickExit::Cancelled)));
            assert_eq!(held_calls, 1, "one held pulse must be the only posted job");
            assert!(
                extra.is_none(),
                "unacknowledged pulse accumulated another job"
            );
            assert!(
                rearmed,
                "actual UI ACK did not permit a later interval pulse"
            );
        }

        // Mutation: post errors are ignored, or a tight retry loop bypasses the250ms wait.
        #[test]
        fn tick_failed_post_sets_fault_and_waits_before_recovery() {
            let f = Fixture::new();
            let (executor, events) = TickExecutor::new(true);
            let mut worker = Running(TickProducer::prepare(f.runtime.binding, executor));
            let ack = worker.0.acknowledgement();
            let attached = attach_tick(f.runtime.binding, ack.clone());
            let started = worker.0.start();
            let first = events.recv_timeout(OBSERVE).ok();
            let second = if first.is_some() {
                events.recv_timeout(OBSERVE).ok()
            } else {
                None
            };
            let spacing = first
                .as_ref()
                .zip(second.as_ref())
                .map(|(a, b)| b.at.duration_since(a.at));
            let fault = ack.faulted();
            let joined = worker.finish();
            assert_eq!(attached, Ok(()));
            assert_eq!(started, Ok(()));
            assert_eq!(joined, Some(Some(TickExit::Cancelled)));
            assert!(first.is_some(), "worker never attempted its first post");
            assert!(
                spacing.is_some_and(|v| v >= Duration::from_millis(250)),
                "failed post retried without a fresh interval or never recovered"
            );
            assert!(
                fault,
                "failed executor submission was not retained as a fault"
            );
        }

        // Control: cancellation has a real independent thread exit, not an ACK/UI dependency.
        #[test]
        fn tick_cancel_interrupts_wait_and_transfers_one_real_join_handle() {
            let f = Fixture::new();
            let (executor, _) = TickExecutor::new(false);
            let mut worker = Running(TickProducer::prepare(f.runtime.binding, executor));
            let attached = attach_tick(f.runtime.binding, worker.0.acknowledgement());
            let started = worker.0.start();
            let joined = worker.finish();
            let second_join = worker.0.take_join().is_none();
            assert_eq!(attached, Ok(()));
            assert_eq!(started, Ok(()));
            assert_eq!(joined, Some(Some(TickExit::Cancelled)));
            assert!(second_join);
        }

        // Control: channel loss is observed by the worker and cannot certify cancellation.
        #[test]
        fn tick_control_channel_loss_is_not_a_cancelled_receipt() {
            let f = Fixture::new();
            let (executor, _) = TickExecutor::new(false);
            let mut worker = Running(TickProducer::prepare(f.runtime.binding, executor));
            let attached = attach_tick(f.runtime.binding, worker.0.acknowledgement());
            let started = worker.0.start();
            let joined = worker.0.take_join().map(|handle| handle.join().ok());
            assert_eq!(attached, Ok(()));
            assert_eq!(started, Ok(()));
            assert_eq!(joined, Some(Some(TickExit::ControlLost)));
        }

        // Control: absent/wrong binding and duplicate attachment cannot replace a live ACK port.
        #[test]
        fn tick_attach_is_bound_once_and_retired_marks_leave_replacement_cold() {
            let f = Fixture::new();
            let (executor, _) = TickExecutor::new(false);
            let mut worker = Running(TickProducer::prepare(f.runtime.binding, executor.clone()));
            let wrong = TickProducer::prepare(f.runtime.binding + 1, executor);
            let wrong_attach = attach_tick(f.runtime.binding, wrong.acknowledgement());
            let attached = attach_tick(f.runtime.binding, worker.0.acknowledgement());
            let duplicate = attach_tick(f.runtime.binding, worker.0.acknowledgement());
            let old = f.runtime.binding;
            f.remove_record();
            let replacement = Fixture::new();
            let retired_start = worker.0.start();
            mark_tick(old);
            let joined = worker.finish();
            assert_eq!(retired_start, Err(TickStartFailure::NotAttached));
            assert_eq!(joined, None);
            assert_eq!(wrong_attach, Err(Code0DispatchError::Unavailable));
            assert_eq!(attached, Ok(()));
            assert_eq!(duplicate, Err(Code0DispatchError::Unavailable));
            assert!(!replacement.runtime.pending.get());
            assert!(replacement.backend.borrow().calls.is_empty());
        }

        // Control: no unattached or already-cancelled preparation can start a producer.
        #[test]
        fn tick_preparation_stays_cold_until_attached_and_start_is_one_way() {
            let f = Fixture::new();
            let (executor, _) = TickExecutor::new(false);
            let mut worker = Running(TickProducer::prepare(f.runtime.binding, executor.clone()));
            let cold = worker.0.start();
            let attached = attach_tick(f.runtime.binding, worker.0.acknowledgement());
            let first = worker.0.start();
            let duplicate = worker.0.start();
            let joined = worker.finish();
            let restart = worker.0.start();
            let mut cancelled = TickProducer::prepare(f.runtime.binding, executor);
            cancelled.cancel();
            let never_started = cancelled.start();
            assert_eq!(cold, Err(TickStartFailure::NotAttached));
            assert_eq!(attached, Ok(()));
            assert_eq!(first, Ok(()));
            assert_eq!(duplicate, Err(TickStartFailure::AlreadyStarted));
            assert_eq!(joined, Some(Some(TickExit::Cancelled)));
            assert_eq!(restart, Err(TickStartFailure::AlreadyStarted));
            assert_eq!(never_started, Err(TickStartFailure::Cancelled));
            assert!(cancelled.take_join().is_none());
        }
    }

    // Controlled integration only: all scheduling/session/ACK behavior remains real.
    mod clock_pulse_tests {
        use super::super::super::code0_clock::Code0WindowsClock;
        use super::super::super::code0_tick::{TickExit, TickProducer};
        use super::*;
        use std::sync::mpsc::{self, Receiver, SyncSender};
        use time::OffsetDateTime;
        use time::format_description::well_known::Rfc3339;

        const OBSERVE: Duration = Duration::from_millis(1250);
        const OLD_WALL: &str = "2026-10-29T00:00:00Z";
        const CAPTURE_WALL: &str = "2026-10-29T00:01:00Z";

        struct PulseEvent {
            job: Option<UiJob>,
            rejected: bool,
            thread: ThreadId,
        }
        struct PulseExecutor {
            ui: ThreadId,
            reject_next_worker: AtomicBool,
            events: SyncSender<PulseEvent>,
        }
        impl MainThreadExecutor for PulseExecutor {
            fn post(&self, job: UiJob) -> Result<(), Code0DispatchError> {
                if thread::current().id() == self.ui {
                    job();
                    return Ok(());
                }
                let rejected = self.reject_next_worker.swap(false, Ordering::SeqCst);
                let event = PulseEvent {
                    job: (!rejected).then_some(job),
                    rejected,
                    thread: thread::current().id(),
                };
                let queued = self.events.try_send(event).is_ok();
                if rejected || !queued {
                    Err(Code0DispatchError::Unavailable)
                } else {
                    Ok(())
                }
            }
        }
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        struct ObservedPulse {
            off_ui: bool,
            turn: TurnResult,
        }
        struct ObservedFault {
            held: Option<ObservedPulse>,
            rejected: bool,
            recovered: Option<ObservedPulse>,
            faulted: bool,
        }
        struct PulseWorker(TickProducer);
        impl PulseWorker {
            fn finish(&mut self) -> Option<Option<TickExit>> {
                self.0.cancel();
                self.0.take_join().map(|handle| handle.join().ok())
            }
        }
        impl Drop for PulseWorker {
            fn drop(&mut self) {
                let _ = self.finish();
            }
        }
        struct PulseStoreState {
            saved: Option<UsageSnapshot>,
            saves: usize,
        }
        struct PulseStore(Rc<RefCell<PulseStoreState>>);
        impl Code0SnapshotStoring for PulseStore {
            fn load(&self) -> Result<Option<UsageSnapshot>, Code0StoreError> {
                Ok(self.0.borrow().saved.clone())
            }
            fn save_atomic(&mut self, value: &UsageSnapshot) -> Result<(), Code0StoreError> {
                let mut state = self.0.borrow_mut();
                state.saved = Some(value.clone());
                state.saves += 1;
                Ok(())
            }
            fn remove(&mut self) -> Result<(), Code0StoreError> {
                self.0.borrow_mut().saved = None;
                Ok(())
            }
        }
        #[derive(Default)]
        struct PulseBackendState {
            current: Option<Code0WindowRequest>,
            requests: Vec<Code0WindowRequest>,
            closes: Vec<String>,
            releases: usize,
            polls: usize,
            deliveries: VecDeque<(Code0WindowRequest, Code0WindowEvent)>,
        }
        struct PulseBackend {
            callbacks: CallbackSink,
            state: Rc<RefCell<PulseBackendState>>,
        }
        impl Code0WindowDriving for PulseBackend {
            fn acquire_profile_lease(&mut self) -> Result<(), Code0SessionFailure> {
                Ok(())
            }
            fn release_profile_lease(&mut self) {}
            fn create_unloaded(
                &mut self,
                request: Code0WindowRequest,
            ) -> Result<(), Code0SessionFailure> {
                let mut state = self.state.borrow_mut();
                assert!(state.current.is_none(), "fixture already owns a window");
                state.current = Some(request.clone());
                state.requests.push(request);
                Ok(())
            }
            fn navigate(&mut self, instance: &str) -> Result<(), Code0SessionFailure> {
                assert_eq!(
                    self.state
                        .borrow()
                        .current
                        .as_ref()
                        .map(|request| request.instance_id.as_str()),
                    Some(instance)
                );
                Ok(())
            }
            fn close(&mut self, instance: &str) {
                let mut state = self.state.borrow_mut();
                assert_eq!(
                    state
                        .current
                        .as_ref()
                        .map(|request| request.instance_id.as_str()),
                    Some(instance)
                );
                state.closes.push(instance.to_owned());
            }
            fn clear_website_data(
                &mut self,
                _: Code0CleanupRequest,
            ) -> Result<(), Code0SessionFailure> {
                Err(Code0SessionFailure::CleanupFailed)
            }
        }
        impl UiBackend for PulseBackend {
            fn on_window_event(&mut self, _: &str, _: &tauri::WindowEvent) {}
            fn poll_cleanup(&mut self) {
                let deliveries = {
                    let mut state = self.state.borrow_mut();
                    state.polls += 1;
                    std::mem::take(&mut state.deliveries)
                };
                for (request, event) in deliveries {
                    if matches!(&event, Code0WindowEvent::Released) {
                        let mut state = self.state.borrow_mut();
                        assert_eq!(state.current.as_ref(), Some(&request));
                        assert!(
                            state.closes.contains(&request.instance_id),
                            "fixture Released preceded the real driver close"
                        );
                        state.current = None;
                        state.releases += 1;
                    }
                    self.callbacks.window(request, event).unwrap();
                }
            }
        }
        type PulseRuntime = Runtime<Code0WindowsClock, PulseBackend, PulseStore>;
        struct PulseFixture {
            runtime: Rc<PulseRuntime>,
            executor: Arc<PulseExecutor>,
            events: Receiver<PulseEvent>,
            elapsed: Rc<Cell<u64>>,
            wall: Rc<Cell<OffsetDateTime>>,
            backend: Rc<RefCell<PulseBackendState>>,
            store: Rc<RefCell<PulseStoreState>>,
            worker: PulseWorker,
        }
        fn wall(value: &str) -> OffsetDateTime {
            OffsetDateTime::parse(value, &Rfc3339).unwrap()
        }
        impl PulseFixture {
            fn new() -> Self {
                let elapsed = Rc::new(Cell::new(0));
                let wall = Rc::new(Cell::new(wall(CAPTURE_WALL)));
                let elapsed_source = elapsed.clone();
                let wall_source = wall.clone();
                let clock = Code0WindowsClock::from_sources(
                    Rc::new(move || elapsed_source.get()),
                    Rc::new(move || wall_source.get()),
                );
                let (send, events) = mpsc::sync_channel(1);
                let executor = Arc::new(PulseExecutor {
                    ui: thread::current().id(),
                    reject_next_worker: AtomicBool::new(false),
                    events: send,
                });
                let backend = Rc::new(RefCell::new(PulseBackendState::default()));
                let mut cached = Code0WebsiteCapture::decode_filtered_json(
                    br#"{"quota":49400,"usedQuota":182700,"divisor":400}"#,
                )
                .unwrap()
                .make_snapshot(OLD_WALL)
                .unwrap();
                cached.status = UsageStatus::Cached;
                let store = Rc::new(RefCell::new(PulseStoreState {
                    saved: Some(cached),
                    saves: 0,
                }));
                let backend_state = backend.clone();
                let runtime = install(
                    executor.clone(),
                    InstallInputs {
                        clock,
                        store: PulseStore(store.clone()),
                        backend: move |callbacks| PulseBackend {
                            callbacks,
                            state: backend_state,
                        },
                        emit: Box::new(|_| {}),
                    },
                )
                .unwrap();
                let worker = PulseWorker(TickProducer::prepare(runtime.binding, executor.clone()));
                let mut fixture = Self {
                    runtime,
                    executor,
                    events,
                    elapsed,
                    wall,
                    backend,
                    store,
                    worker,
                };
                assert_eq!(
                    attach_tick(fixture.runtime.binding, fixture.worker.0.acknowledgement()),
                    Ok(())
                );
                assert_eq!(fixture.worker.0.start(), Ok(()));
                fixture
            }
            fn view(&self) -> serde_json::Value {
                serde_json::to_value(passive_read(self.runtime.binding).unwrap()).unwrap()
            }
            fn turn(&self) {
                assert_eq!(settled_turn(self.runtime.binding), TurnResult::Settled);
            }
            fn action(
                &self,
                action: Code0Action,
            ) -> Option<Result<Code0Reply, Code0DispatchError>> {
                let reply = slot();
                self.runtime.proxy.action(action, reply_to(&reply));
                self.turn();
                let output = reply.lock().unwrap();
                output.clone()
            }
            fn current_request(&self) -> Code0WindowRequest {
                self.backend.borrow().current.as_ref().unwrap().clone()
            }
            fn queue(&self, request: &Code0WindowRequest, event: Code0WindowEvent) {
                self.backend
                    .borrow_mut()
                    .deliveries
                    .push_back((request.clone(), event));
            }
            fn activate(&self) {
                assert_eq!(
                    self.action(Code0Action::SyncNow),
                    Some(Ok(Code0Reply::Action(Action::Started)))
                );
                let request = self.current_request();
                self.queue(&request, Code0WindowEvent::PoliciesInstalled);
                self.turn();
                for (kind, sequence) in [("ready", 1), ("selfStart", 2), ("capture", 3)] {
                    let mut value = serde_json::json!({
                        "version":1,"nonce":request.nonce,"document":request.document_id,
                        "kind":kind,"sequence":sequence,
                    });
                    if kind != "ready" {
                        value["request"] = serde_json::json!(1);
                        value["revision"] = serde_json::json!(0);
                    }
                    if kind == "capture" {
                        value["quota"] = serde_json::json!(49400);
                        value["usedQuota"] = serde_json::json!(182700);
                        value["divisor"] = serde_json::json!(400);
                    }
                    self.queue(
                        &request,
                        Code0WindowEvent::Message {
                            payload: serde_json::to_vec(&value).unwrap(),
                            source_url: "https://console.code0.ai/console".into(),
                            is_main_frame: true,
                        },
                    );
                }
                self.turn();
                assert_eq!(self.view()["state"], "closing");
                assert_eq!(self.store.borrow().saves, 1);
                assert_eq!(
                    self.store.borrow().saved.as_ref().unwrap().fetched_at,
                    CAPTURE_WALL
                );
                assert_eq!(
                    self.backend.borrow().closes,
                    vec![request.instance_id.clone()]
                );
                self.queue(&request, Code0WindowEvent::Released);
                self.turn();
                assert_eq!(self.view()["state"], "active");
                assert_eq!(self.view()["isAutomaticallySyncing"], true);
                assert_eq!(self.backend.borrow().requests.len(), 1);
                assert_eq!(self.backend.borrow().releases, 1);
            }
            fn wait_event(&self) -> Option<PulseEvent> {
                self.events.recv_timeout(OBSERVE).ok()
            }
            fn deliver(&self, mut event: PulseEvent) -> Option<ObservedPulse> {
                if event.rejected {
                    return None;
                }
                let job = event.job.take()?;
                let off_ui = event.thread != thread::current().id();
                job(); // Real binding lookup/ACK; no posted or pending mutation by the fixture.
                Some(ObservedPulse {
                    off_ui,
                    turn: settled_turn(self.runtime.binding),
                })
            }
            fn pulse(&self) -> Option<ObservedPulse> {
                self.wait_event().and_then(|event| self.deliver(event))
            }
            fn fault_then_recover(&self) -> ObservedFault {
                let held = self.wait_event();
                // Arm while its genuine queued job still holds posted=true. The next
                // attempt cannot happen until that actual job performs the real ACK.
                self.executor
                    .reject_next_worker
                    .store(true, Ordering::SeqCst);
                let held = held.and_then(|event| self.deliver(event));
                let rejected = self
                    .wait_event()
                    .is_some_and(|event| event.rejected && event.job.is_none());
                // This later real post cannot occur before the worker handles Err.
                let recovered = self.pulse();
                let faulted = self.worker.0.acknowledgement().faulted();
                ObservedFault {
                    held,
                    rejected,
                    recovered,
                    faulted,
                }
            }
            fn finish(&mut self) -> Option<Option<TickExit>> {
                self.worker.finish()
            }
            fn remove_record(&self) {
                let retired = REGISTRY.with(|slot| {
                    let mut slot = slot.borrow_mut();
                    if slot
                        .record
                        .as_ref()
                        .is_some_and(|record| record.binding() == self.runtime.binding)
                    {
                        slot.record.take()
                    } else {
                        None
                    }
                });
                drop(retired);
            }
        }
        impl Drop for PulseFixture {
            fn drop(&mut self) {
                let _ = self.finish();
                self.remove_record();
            }
        }
        fn require_join(joined: Option<Option<TickExit>>) {
            assert_eq!(joined, Some(Some(TickExit::Cancelled)));
        }
        fn require_delivery(pulse: Option<ObservedPulse>) {
            assert_eq!(
                pulse,
                Some(ObservedPulse {
                    off_ui: true,
                    turn: TurnResult::Settled,
                }),
                "setup did not deliver a real off-UI worker job into the settled runtime"
            );
        }
        fn require_fault(fault: ObservedFault) {
            require_delivery(fault.held);
            assert!(fault.rejected, "setup did not observe a real rejected post");
            require_delivery(fault.recovered);
            assert!(fault.faulted, "setup lost the real sticky post fault");
        }

        // Mutation: a genuine worker ACK never queues the real dispatcher clock tick.
        #[test]
        fn pulse_reads_real_elapsed_299_then_admits_at_300() {
            let mut f = PulseFixture::new();
            f.activate();
            f.elapsed.set(299_000);
            let early_pulse = f.pulse();
            let early = f.view();
            let early_creates = f.backend.borrow().requests.len();
            f.elapsed.set(300_000);
            let due_pulse = f.pulse();
            let due = f.view();
            let due_creates = f.backend.borrow().requests.len();
            let joined = f.finish();
            require_join(joined);
            require_delivery(early_pulse);
            require_delivery(due_pulse);
            assert_eq!(early["state"], "active");
            assert_eq!(early_creates, 1);
            assert!(
                due["state"] == "syncing",
                "worker ACK at elapsed300 did not admit the real session attempt"
            );
            assert_eq!(due_creates, 2);
        }

        // Mutation: elapsed is frozen/synthetic, or a suspend gap creates catchup attempts.
        #[test]
        fn pulse_suspend_jump_admits_one_attempt_without_catchup() {
            let mut f = PulseFixture::new();
            f.activate();
            f.elapsed.set(3_600_000);
            let first = f.pulse();
            let second = f.pulse();
            let third = f.pulse();
            let creates = f.backend.borrow().requests.len();
            let view = f.view();
            let joined = f.finish();
            require_join(joined);
            require_delivery(first);
            require_delivery(second);
            require_delivery(third);
            assert!(
                creates == 2,
                "suspend-sized elapsed jump did not admit exactly one real due attempt"
            );
            assert_eq!(view["state"], "syncing");
        }

        // Control: forward/backward wall jumps cannot replace elapsed admission or save again.
        #[test]
        fn wall_only_jumps_leave_elapsed299_active_and_snapshot_unchanged() {
            let mut f = PulseFixture::new();
            f.activate();
            f.elapsed.set(299_000);
            f.wall.set(wall("2027-10-29T03:04:05Z"));
            let forward_pulse = f.pulse();
            let forward = f.view();
            f.wall.set(wall("2025-10-29T03:04:05Z"));
            let backward_pulse = f.pulse();
            let backward = f.view();
            let creates = f.backend.borrow().requests.len();
            let saves = f.store.borrow().saves;
            let saved_at = f.store.borrow().saved.as_ref().unwrap().fetched_at.clone();
            let joined = f.finish();
            require_join(joined);
            require_delivery(forward_pulse);
            require_delivery(backward_pulse);
            assert_eq!(forward["state"], "active");
            assert_eq!(backward["state"], "active");
            assert_eq!(creates, 1);
            assert_eq!(saves, 1);
            assert_eq!(saved_at, CAPTURE_WALL);
        }

        // Mutation: the real worker never advances the original, unreset45s attempt expiry.
        #[test]
        fn pulse_expires_original_attempt_at_45_not_early() {
            let mut f = PulseFixture::new();
            f.activate();
            f.elapsed.set(300_000);
            let started = f.action(Code0Action::SyncNow);
            let request = f.current_request();
            f.queue(&request, Code0WindowEvent::PoliciesInstalled);
            f.turn();
            f.elapsed.set(344_999);
            let early_pulse = f.pulse();
            let early = f.view();
            f.elapsed.set(345_000);
            let deadline_pulse = f.pulse();
            let deadline = f.view();
            let closes = f.backend.borrow().closes.len();
            let joined = f.finish();
            require_join(joined);
            require_delivery(early_pulse);
            require_delivery(deadline_pulse);
            assert_eq!(started, Some(Ok(Code0Reply::Action(Action::Started))));
            assert_eq!(early["state"], "syncing");
            assert!(early["failure"].is_null());
            assert!(
                deadline["state"] == "closing" && deadline["failure"] == "syncTimedOut",
                "worker ACK at original45 did not expire the real session attempt"
            );
            assert_eq!(closes, 2);
        }

        // Mutation: a sticky real post fault remains unobserved at the later settled UI turn.
        #[test]
        fn sticky_post_fault_fences_new_actions_after_recovered_ui_delivery() {
            let mut f = PulseFixture::new();
            f.activate();
            f.elapsed.set(300_000);
            let started = f.action(Code0Action::SyncNow);
            let fault = f.fault_then_recover();
            let reply = f.action(Code0Action::SyncNow);
            let creates = f.backend.borrow().requests.len();
            let joined = f.finish();
            require_join(joined);
            require_fault(fault);
            assert_eq!(started, Some(Ok(Code0Reply::Action(Action::Started))));
            assert!(
                matches!(
                    reply,
                    Some(Ok(Code0Reply::Action(Action::Denied)))
                        | Some(Err(Code0DispatchError::Unavailable))
                ),
                "sticky worker post fault left real proxy action admission open"
            );
            assert_eq!(creates, 2);
        }

        // Control: an ordinary settled turn must not manufacture a missing worker pulse.
        #[test]
        fn held_worker_job_does_not_admit_elapsed300_before_real_delivery() {
            let mut f = PulseFixture::new();
            f.activate();
            f.elapsed.set(300_000);
            let held = f.wait_event();
            let real_held_job = held.as_ref().is_some_and(|event| {
                !event.rejected && event.job.is_some() && event.thread != thread::current().id()
            });
            f.turn();
            f.turn();
            let view = f.view();
            let creates = f.backend.borrow().requests.len();
            let joined = f.finish();
            drop(held);
            require_join(joined);
            assert!(real_held_job, "setup did not hold an actual worker job");
            assert_eq!(view["state"], "active");
            assert_eq!(creates, 1);
        }

        // Control: finish the cold old worker, then deliver its actual queued binding-only job.
        #[test]
        fn retired_worker_job_cannot_pulse_the_real_replacement_clock() {
            let mut old = PulseFixture::new();
            let old_job = old.wait_event();
            let old_join = old.finish();
            old.remove_record();
            let mut replacement = PulseFixture::new();
            replacement.activate();
            replacement.elapsed.set(300_000);
            let stale_delivery = old_job.and_then(|event| replacement.deliver(event));
            let view = replacement.view();
            let creates = replacement.backend.borrow().requests.len();
            let replacement_join = replacement.finish();
            require_join(old_join);
            require_join(replacement_join);
            require_delivery(stale_delivery);
            assert_eq!(view["state"], "active");
            assert_eq!(creates, 1);
        }

        // Control: post failure must not disable normal polling of a retained matching release.
        #[test]
        fn sticky_fault_keeps_backend_poll_and_matching_released_delivery_live() {
            let mut f = PulseFixture::new();
            f.activate();
            f.elapsed.set(300_000);
            let started = f.action(Code0Action::SyncNow);
            let request = f.current_request();
            let fault = f.fault_then_recover();
            let before_poll = f.backend.borrow().polls;
            f.queue(
                &request,
                Code0WindowEvent::Failed(Code0SessionFailure::PolicyRejected),
            );
            f.turn();
            f.queue(&request, Code0WindowEvent::Released);
            f.turn();
            let read = slot();
            f.runtime.proxy.read(reply_to(&read));
            f.turn();
            let read_reply = read.lock().unwrap().clone();
            let view = f.view();
            let polls = f.backend.borrow().polls;
            let releases = f.backend.borrow().releases;
            let no_owner = f.backend.borrow().current.is_none();
            let creates = f.backend.borrow().requests.len();
            let joined = f.finish();
            require_join(joined);
            require_fault(fault);
            assert_eq!(started, Some(Ok(Code0Reply::Action(Action::Started))));
            assert!(polls >= before_poll + 2);
            assert_eq!(releases, 2);
            assert!(no_owner);
            assert_eq!(creates, 2);
            assert_eq!(view["state"], "paused");
            assert!(matches!(read_reply, Some(Ok(Code0Reply::View(_)))));
        }
    }
    // RED contract: the runtime, not fixture teardown, must own the terminal join.
    mod terminal_join_tests {
        use super::super::super::code0_tick::{TickExit, TickProducer};
        use super::*;
        use std::sync::mpsc::{self, Receiver, Sender};
        use std::time::Instant;

        const OBSERVE: Duration = Duration::from_millis(1250);
        const GATE_LIMIT: Duration = Duration::from_secs(5);

        struct TerminalExecutor {
            ui: ThreadId,
            jobs: Mutex<VecDeque<UiJob>>,
            pulses: Sender<bool>,
            completion_posts: Sender<ThreadId>,
            completion_jobs: Mutex<VecDeque<UiJob>>,
            gate: Mutex<Receiver<()>>,
            release: Sender<()>,
            hold_first: AtomicBool,
            panic_worker: AtomicBool,
            reject: AtomicBool,
            gate_expired: AtomicBool,
        }
        impl TerminalExecutor {
            fn new() -> (Arc<Self>, Receiver<bool>, Receiver<ThreadId>) {
                let (pulses, events) = mpsc::channel();
                let (completion_posts, completions) = mpsc::channel();
                let (release, gate) = mpsc::channel();
                (
                    Arc::new(Self {
                        ui: thread::current().id(),
                        jobs: Mutex::new(VecDeque::new()),
                        pulses,
                        completion_posts,
                        completion_jobs: Mutex::new(VecDeque::new()),
                        gate: Mutex::new(gate),
                        release,
                        hold_first: AtomicBool::new(true),
                        panic_worker: AtomicBool::new(false),
                        reject: AtomicBool::new(false),
                        gate_expired: AtomicBool::new(false),
                    }),
                    events,
                    completions,
                )
            }
            fn take_jobs(&self) -> VecDeque<UiJob> {
                std::mem::take(&mut *self.jobs.lock().unwrap())
            }
            fn take_completion_jobs(&self) -> VecDeque<UiJob> {
                std::mem::take(&mut *self.completion_jobs.lock().unwrap())
            }
            fn flush(&self) {
                for job in self.take_jobs() {
                    job();
                }
                for job in self.take_completion_jobs() {
                    job();
                }
            }
            fn open_gate(&self) {
                let _ = self.release.send(());
            }
        }
        impl MainThreadExecutor for TerminalExecutor {
            fn post(&self, job: UiJob) -> Result<(), Code0DispatchError> {
                let current = thread::current();
                let worker = current.name() == Some("code0-tick");
                let rejected = self.reject.load(Ordering::SeqCst);
                if !rejected {
                    if current.id() == self.ui {
                        job();
                    } else if worker {
                        self.jobs.lock().unwrap().push_back(job);
                    } else {
                        // The only other background posts in this fixture are
                        // the product join's actual completion delivery jobs.
                        self.completion_jobs.lock().unwrap().push_back(job);
                        let _ = self.completion_posts.send(current.id());
                    }
                }
                if worker {
                    let _ = self.pulses.send(rejected);
                    if self.hold_first.swap(false, Ordering::SeqCst)
                        && self.gate.lock().unwrap().recv_timeout(GATE_LIMIT).is_err()
                    {
                        self.gate_expired.store(true, Ordering::SeqCst);
                    }
                    assert!(
                        !self.panic_worker.load(Ordering::SeqCst),
                        "controlled actual tick executor panic"
                    );
                }
                if rejected {
                    Err(Code0DispatchError::Unavailable)
                } else {
                    Ok(())
                }
            }
        }
        #[derive(Clone)]
        struct TerminalClock(Rc<Cell<Duration>>);
        impl Code0SessionClock for TerminalClock {
            fn now(&self) -> Duration {
                self.0.get()
            }
            fn wall_now(&self) -> String {
                "2026-10-30T00:00:00Z".into()
            }
        }
        struct TerminalBackendState {
            native: NativeTerminalState,
            held: bool,
            current: Option<Code0WindowRequest>,
            windows: VecDeque<(Code0WindowRequest, Code0WindowEvent)>,
            cleanups: Vec<Code0CleanupRequest>,
            results: VecDeque<(Code0CleanupRequest, Result<(), Code0SessionFailure>)>,
            polls: usize,
            closes: usize,
            releases: usize,
            delivery_failures: usize,
        }
        struct TerminalBackend {
            callbacks: CallbackSink,
            state: Rc<RefCell<TerminalBackendState>>,
        }
        impl Code0WindowDriving for TerminalBackend {
            fn acquire_profile_lease(&mut self) -> Result<(), Code0SessionFailure> {
                self.state.borrow_mut().held = true;
                Ok(())
            }
            fn release_profile_lease(&mut self) {
                self.state.borrow_mut().held = false;
            }
            fn create_unloaded(
                &mut self,
                request: Code0WindowRequest,
            ) -> Result<(), Code0SessionFailure> {
                self.state.borrow_mut().current = Some(request);
                Ok(())
            }
            fn navigate(&mut self, _: &str) -> Result<(), Code0SessionFailure> {
                Ok(())
            }
            fn close(&mut self, _: &str) {
                self.state.borrow_mut().closes += 1;
            }
            fn clear_website_data(
                &mut self,
                request: Code0CleanupRequest,
            ) -> Result<(), Code0SessionFailure> {
                self.state.borrow_mut().cleanups.push(request);
                Ok(())
            }
        }
        impl UiBackend for TerminalBackend {
            fn on_window_event(&mut self, _: &str, _: &tauri::WindowEvent) {}
            fn poll_cleanup(&mut self) {
                let (windows, results) = {
                    let mut state = self.state.borrow_mut();
                    state.polls += 1;
                    (
                        std::mem::take(&mut state.windows),
                        std::mem::take(&mut state.results),
                    )
                };
                for (request, event) in windows {
                    if matches!(event, Code0WindowEvent::Released) {
                        let mut state = self.state.borrow_mut();
                        state.current = None;
                        state.releases += 1;
                    }
                    if self.callbacks.window(request, event).is_err() {
                        self.state.borrow_mut().delivery_failures += 1;
                    }
                }
                for (request, result) in results {
                    if self.callbacks.cleanup(request, result).is_err() {
                        self.state.borrow_mut().delivery_failures += 1;
                    }
                }
            }
            fn terminal_state(&self) -> NativeTerminalState {
                self.state.borrow().native
            }
        }
        type TerminalRuntime = Runtime<TerminalClock, TerminalBackend, Store>;
        struct CleanupEvidence {
            thread: ThreadId,
            worker: Option<TickExit>,
            panicked: bool,
        }
        struct TerminalFixture {
            runtime: Rc<TerminalRuntime>,
            executor: Arc<TerminalExecutor>,
            pulses: Receiver<bool>,
            completion_posts: Receiver<ThreadId>,
            backend: Rc<RefCell<TerminalBackendState>>,
            elapsed: Rc<Cell<Duration>>,
            escaped: Option<thread::JoinHandle<TickExit>>,
            setup: bool,
            requests_ok: Cell<bool>,
            turns_settled: Cell<bool>,
        }
        impl TerminalFixture {
            fn new() -> Self {
                let (executor, pulses, completion_posts) = TerminalExecutor::new();
                let backend = Rc::new(RefCell::new(TerminalBackendState {
                    native: NativeTerminalState::Pending,
                    held: false,
                    current: None,
                    windows: VecDeque::new(),
                    cleanups: Vec::new(),
                    results: VecDeque::new(),
                    polls: 0,
                    closes: 0,
                    releases: 0,
                    delivery_failures: 0,
                }));
                let elapsed = Rc::new(Cell::new(Duration::ZERO));
                let state = backend.clone();
                let runtime = install(
                    executor.clone(),
                    InstallInputs {
                        clock: TerminalClock(elapsed.clone()),
                        store: Store(Rc::new(RefCell::new(StoreState::default()))),
                        backend: move |callbacks| TerminalBackend { callbacks, state },
                        emit: Box::new(|_| {}),
                    },
                )
                .unwrap(); // No worker exists before installation succeeds.
                let mut producer = TickProducer::prepare(runtime.binding, executor.clone());
                let attached = attach_tick(runtime.binding, producer.acknowledgement()).is_ok();
                let started = producer.start().is_ok();
                let adopted = runtime.adopt_tick(producer).is_ok();
                Self {
                    runtime,
                    executor,
                    pulses,
                    completion_posts,
                    backend,
                    elapsed,
                    escaped: None,
                    setup: attached && started && adopted,
                    requests_ok: Cell::new(true),
                    turns_settled: Cell::new(true),
                }
            }
            fn turn(&self) -> TurnResult {
                let result = settled_turn(self.runtime.binding);
                self.turns_settled
                    .set(self.turns_settled.get() && result == TurnResult::Settled);
                result
            }
            fn request(&self) -> bool {
                let accepted = request_terminal(self.runtime.binding).is_ok();
                self.requests_ok.set(self.requests_ok.get() && accepted);
                accepted
            }
            fn claim(&self) -> bool {
                self.runtime
                    .take_terminal_permit()
                    .is_some_and(|permit| self.runtime.consume_terminal_permit(permit))
            }
            fn joined(&self) -> Option<(u64, u64, Option<TickExit>, ThreadId)> {
                self.runtime.terminal_join.borrow().as_ref().map(|receipt| {
                    (
                        receipt.binding,
                        receipt.operation,
                        receipt.result.as_ref().ok().copied(),
                        receipt.thread,
                    )
                })
            }
            fn pump(&self) {
                let deadline = Instant::now() + OBSERVE;
                while Instant::now() < deadline && self.joined().is_none() {
                    self.executor.flush();
                    self.turn();
                    thread::sleep(Duration::from_millis(2));
                }
                self.executor.flush();
                self.turn();
            }
            fn genuine_later_pulse(&self) -> bool {
                let polls = self.backend.borrow().polls;
                let pulse = self.pulses.recv_timeout(OBSERVE).ok();
                self.executor.flush();
                self.turn();
                pulse == Some(false) && self.backend.borrow().polls > polls
            }
            fn view(&self) -> serde_json::Value {
                serde_json::to_value(self.runtime.proxy.committed()).unwrap()
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
            // Fixture safety only. This never deposits a runtime receipt or permit.
            fn cleanup(&mut self) -> CleanupEvidence {
                self.executor.open_gate();
                let producer = self.runtime.terminal_tick.borrow_mut().take();
                let escaped = self.escaped.take();
                thread::scope(|scope| {
                    scope
                        .spawn(move || {
                            let mut producer = producer;
                            if let Some(producer) = producer.as_ref() {
                                producer.cancel();
                            }
                            let handle = escaped
                                .or_else(|| producer.as_mut().and_then(TickProducer::take_join));
                            let result = handle.map(|handle| handle.join());
                            CleanupEvidence {
                                thread: thread::current().id(),
                                worker: result
                                    .as_ref()
                                    .and_then(|result| result.as_ref().ok())
                                    .copied(),
                                panicked: result.is_some_and(|result| result.is_err()),
                            }
                        })
                        .join()
                        .unwrap()
                })
            }
        }
        impl Drop for TerminalFixture {
            fn drop(&mut self) {
                let _ = self.cleanup();
                self.remove_record();
            }
        }
        fn scoped<T>(
            body: impl FnOnce(&mut TerminalFixture) -> T,
        ) -> (T, CleanupEvidence, bool, bool) {
            let mut fixture = TerminalFixture::new();
            let observed = body(&mut fixture);
            let cleanup = fixture.cleanup();
            let setup = fixture.setup
                && fixture.requests_ok.get()
                && fixture.turns_settled.get()
                && fixture.backend.borrow().delivery_failures == 0;
            let gate_expired = fixture.executor.gate_expired.load(Ordering::SeqCst);
            fixture.remove_record();
            (observed, cleanup, setup, gate_expired)
        }
        fn check_cleanup(cleanup: &CleanupEvidence, setup: bool, gate_expired: bool) {
            assert!(setup, "terminal fixture failed real attach/start/adopt");
            assert!(
                !gate_expired,
                "terminal path blocked UI until the safety gate expired"
            );
            assert_ne!(
                cleanup.thread,
                thread::current().id(),
                "fixture actual worker join ran on UI"
            );
        }

        // Mutation: cancel/join blocks UI, grants before worker return, or never grants after real join.
        #[test]
        fn terminal_join_waits_for_real_worker_and_ui_keeps_progressing() {
            let (
                (first, requested, turn, elapsed, reply, early, joined, claims, key),
                cleanup,
                setup,
                expired,
            ) = scoped(|f| {
                let first = f.pulses.recv_timeout(OBSERVE).ok();
                f.executor.flush(); // Real tick binding job ACK while worker post remains held.
                f.backend.borrow_mut().native = NativeTerminalState::Quiescent;
                let start = Instant::now();
                let requested = f.request();
                let turn = f.turn();
                let read = slot();
                f.runtime.proxy.read(reply_to(&read));
                f.turn();
                let elapsed = start.elapsed();
                let reply = read.lock().unwrap().clone();
                let early = f.claim();
                let key = f.runtime.terminal_operation.get();
                f.executor.open_gate();
                f.pump();
                let joined = f.joined();
                let claims = [f.claim(), f.claim()];
                (
                    first, requested, turn, elapsed, reply, early, joined, claims, key,
                )
            });
            check_cleanup(&cleanup, setup, expired);
            assert_eq!(first, Some(false), "no genuine held worker submission");
            assert!(requested);
            assert_eq!(turn, TurnResult::Settled);
            assert!(
                elapsed < OBSERVE,
                "terminal call blocked the UI worker gate"
            );
            assert!(
                matches!(reply, Some(Ok(Code0Reply::View(_)))),
                "held join starved actual queued read"
            );
            assert!(!early, "held actual worker allowed early permission");
            assert_eq!(
                claims,
                [true, false],
                "TJ-R1 actual joined terminal operation must yield one consumed permit"
            );
            assert!(
                joined.is_some_and(|(_, operation, result, on)| Some(operation) == key
                    && result == Some(TickExit::Cancelled)
                    && on != thread::current().id()),
                "terminal permit lacked actual off-UI Cancelled join evidence"
            );
            assert!(
                cleanup.worker.is_none(),
                "fixture teardown, rather than runtime, still owned the join"
            );
        }

        // Mutation: Stop/physical zero cancels polling before matching reserved Released is consumed.
        #[test]
        fn terminal_retains_poll_until_matching_release() {
            let ((first, held, pulse, same_key, released, claims, joined), cleanup, setup, expired) =
                scoped(|f| {
                    let first = f.pulses.recv_timeout(OBSERVE).ok();
                    f.executor.flush();
                    f.runtime
                        .proxy
                        .action(Code0Action::SyncNow, Box::new(|_| {}));
                    f.turn();
                    let request = f.backend.borrow().current.clone();
                    let _ = f.request();
                    f.turn();
                    let key = f.runtime.terminal_operation.get();
                    let polls = f.backend.borrow().polls;
                    let held = f.view()["state"] == "closing" && !f.claim() && f.joined().is_none();
                    f.executor.open_gate();
                    let pulse = f.pulses.recv_timeout(OBSERVE).ok();
                    f.executor.flush();
                    f.turn();
                    let pulse = pulse == Some(false)
                        && f.backend.borrow().polls > polls
                        && f.joined().is_none();
                    for _ in 0..8 {
                        let _ = f.request();
                    }
                    let same_key = key.is_some() && f.runtime.terminal_operation.get() == key;
                    if let Some(request) = request {
                        let mut state = f.backend.borrow_mut();
                        state
                            .windows
                            .push_back((request, Code0WindowEvent::Released));
                        state.native = NativeTerminalState::Quiescent;
                    }
                    f.turn();
                    f.pump();
                    let released =
                        f.backend.borrow().releases == 1 && f.view()["state"] == "paused";
                    (
                        first,
                        held,
                        pulse,
                        same_key,
                        released,
                        [f.claim(), f.claim()],
                        f.joined(),
                    )
                });
            check_cleanup(&cleanup, setup, expired);
            assert_eq!(first, Some(false));
            assert!(
                held,
                "Stop retired an owned Session before matching Released"
            );
            assert!(pulse, "held owner stopped genuine worker polling");
            assert!(
                same_key,
                "duplicate terminal requests replaced the operation"
            );
            assert!(
                released,
                "matching reserved Released did not settle real Session"
            );
            assert_eq!(
                claims,
                [true, false],
                "TJ-R2 matching release must permit one runtime-owned joined terminal operation"
            );
            assert!(
                joined.is_some_and(|(_, _, exit, on)| exit == Some(TickExit::Cancelled)
                    && on != thread::current().id())
            );
            assert!(cleanup.worker.is_none());
        }

        // Controls: real failed/expired cleanup, retained work, native unknown and worker faults stay unsafe.
        #[test]
        fn terminal_failed_cleanup_and_retained_work_never_permit() {
            for expire in [false, true] {
                let ((first, pending, progress, denied), cleanup, setup, expired) = scoped(|f| {
                    let first = f.pulses.recv_timeout(OBSERVE).ok();
                    f.executor.flush();
                    f.runtime
                        .proxy
                        .action(Code0Action::Disconnect, Box::new(|_| {}));
                    f.turn();
                    let request = f.backend.borrow().cleanups.first().cloned();
                    if expire {
                        f.elapsed.set(Duration::from_secs(10));
                        let _ = f.runtime.proxy.tick(f.runtime.binding, f.runtime.binding);
                    } else if let Some(request) = request.clone() {
                        f.backend
                            .borrow_mut()
                            .results
                            .push_back((request, Err(Code0SessionFailure::CleanupFailed)));
                    }
                    let _ = f.request();
                    f.turn();
                    f.backend.borrow_mut().native = NativeTerminalState::Quiescent;
                    f.turn();
                    f.executor.open_gate();
                    let progress = f.genuine_later_pulse();
                    let failed = request.is_some()
                        && f.view()["state"] == "cleanupPending"
                        && f.view()["failure"] == "cleanupFailed";
                    (first, failed, progress, !f.claim() && f.joined().is_none())
                });
                check_cleanup(&cleanup, setup, expired);
                assert_eq!(first, Some(false));
                assert!(pending, "real failed/expired cleanup was healed");
                assert!(
                    progress,
                    "failed/expired cleanup cancelled the genuine worker or starved backend polls"
                );
                assert!(denied, "cleanup failure allowed terminal permission");
                assert_eq!(cleanup.worker, Some(TickExit::Cancelled));
            }
            let (
                (first, unknown, progress, retained, after_partial, stopped),
                cleanup,
                setup,
                expired,
            ) = scoped(|f| {
                let first = f.pulses.recv_timeout(OBSERVE).ok();
                f.executor.flush();
                let _ = f.request();
                f.turn();
                f.executor.open_gate();
                let progress = f.genuine_later_pulse();
                let unknown = !f.backend.borrow().held
                    && f.backend.borrow().native == NativeTerminalState::Pending
                    && !f.claim()
                    && f.joined().is_none();
                f.backend.borrow_mut().native = NativeTerminalState::Quiescent;
                let binding = f.runtime.binding;
                let callback = Arc::new(Mutex::new(None));
                let observed = callback.clone();
                f.runtime.proxy.stop(Box::new(move |reply| {
                    let record = lookup(binding);
                    *observed.lock().unwrap() =
                        Some(matches!(reply, Ok(Code0Reply::Stopped)) && record.is_some());
                    let _ = window_event(binding, "retained".into(), tauri::WindowEvent::Destroyed);
                }));
                let retained = !f.runtime.dispatcher.terminal_prerequisites_met() && !f.claim();
                f.turn();
                let after_partial = f.runtime.pending.get() && !f.claim() && f.joined().is_none();
                let stopped = *callback.lock().unwrap();
                (first, unknown, progress, retained, after_partial, stopped)
            });
            check_cleanup(&cleanup, setup, expired);
            assert_eq!(first, Some(false));
            assert!(
                unknown,
                "is_held=false substituted for explicit native quiescence"
            );
            assert!(
                progress,
                "unknown native state cancelled the genuine worker or starved backend polls"
            );
            assert!(
                retained && after_partial,
                "queued work or partial drain granted permission"
            );
            assert_eq!(
                stopped,
                Some(true),
                "Stopped callback retired registry prematurely"
            );
            assert_eq!(cleanup.worker, Some(TickExit::Cancelled));

            for fault in ["control-lost", "panic", "delivery"] {
                let ((first, denied, actual_fault, joined, join_panicked), cleanup, setup, expired) =
                    scoped(|f| {
                        let first = f.pulses.recv_timeout(OBSERVE).ok();
                        f.executor.flush();
                        match fault {
                            "control-lost" => {
                                f.escaped = f
                                    .runtime
                                    .terminal_tick
                                    .borrow_mut()
                                    .as_mut()
                                    .and_then(TickProducer::take_join);
                            }
                            "panic" => f.executor.panic_worker.store(true, Ordering::SeqCst),
                            _ => f.executor.reject.store(true, Ordering::SeqCst),
                        }
                        let _ = f.request();
                        f.turn();
                        f.executor.open_gate();
                        let actual_fault = if fault == "delivery" {
                            let rejected = f.pulses.recv_timeout(OBSERVE).ok() == Some(true);
                            let deadline = Instant::now() + OBSERVE;
                            while Instant::now() < deadline
                                && !f
                                    .runtime
                                    .tick_ack
                                    .borrow()
                                    .as_ref()
                                    .is_some_and(|ack| ack.faulted())
                            {
                                thread::sleep(Duration::from_millis(2));
                            }
                            rejected
                                && f.runtime
                                    .tick_ack
                                    .borrow()
                                    .as_ref()
                                    .is_some_and(|ack| ack.faulted())
                        } else {
                            true // Actual ControlLost/panic outcome is asserted after off-UI teardown below.
                        };
                        f.backend.borrow_mut().native = NativeTerminalState::Quiescent;
                        f.pump();
                        let joined = f.joined();
                        let join_panicked = f
                            .runtime
                            .terminal_join
                            .borrow()
                            .as_ref()
                            .is_some_and(|receipt| receipt.result.is_err());
                        (first, !f.claim(), actual_fault, joined, join_panicked)
                    });
                check_cleanup(&cleanup, setup, expired);
                assert_eq!(first, Some(false));
                assert!(
                    denied && actual_fault,
                    "real worker fault allowed a terminal permit: {fault}"
                );
                let actual_exit = joined.and_then(|(_, _, exit, _)| exit).or(cleanup.worker);
                match fault {
                    "control-lost" => assert_eq!(actual_exit, Some(TickExit::ControlLost)),
                    "panic" => assert!(
                        join_panicked || cleanup.panicked,
                        "worker did not actually panic"
                    ),
                    _ => assert_eq!(actual_exit, Some(TickExit::Cancelled)),
                }
            }
        }

        // Mutation: a receipt bypasses fresh queue/current-binding checks or old jobs affect replacement.
        #[test]
        fn terminal_join_receipt_rechecks_current_record() {
            let (
                (first, handoff, completion, during, partial, claims, joined, stale_safe),
                cleanup,
                setup,
                expired,
            ) = scoped(|f| {
                let first = f.pulses.recv_timeout(OBSERVE).ok();
                let mut old_jobs = f.executor.take_jobs(); // Genuine original tick binding job.
                f.backend.borrow_mut().native = NativeTerminalState::Quiescent;
                let _ = f.request();
                f.turn();
                // This observes actual producer ownership, never a joined flag.
                // The inert RED seam must fail this immediate behavioral endpoint;
                // a timeout after a real handoff is a separate setup failure.
                let handoff = f.runtime.terminal_tick.borrow().is_none();
                if !handoff {
                    return (
                        first,
                        handoff,
                        None,
                        false,
                        false,
                        [f.claim(), f.claim()],
                        f.joined(),
                        false,
                    );
                }
                f.executor.open_gate();
                // Do not run settled turns while waiting. The executor retains
                // the genuine product completion job before any UI execution.
                let completion = f.completion_posts.recv_timeout(OBSERVE).ok();
                let completion_jobs = f.executor.take_completion_jobs();
                let completion = completion
                    .filter(|on| *on != thread::current().id() && completion_jobs.len() == 1);
                old_jobs.extend(completion_jobs);
                let binding = f.runtime.binding;
                let read = slot();
                let output = read.clone();
                f.runtime.proxy.read(Box::new(move |reply| {
                    *output.lock().unwrap() = Some(reply);
                    let _ =
                        window_event(binding, "during-join".into(), tauri::WindowEvent::Destroyed);
                }));
                let during = !f.claim() && !f.runtime.dispatcher.terminal_prerequisites_met();
                // Fixed transport deposits real outcome DATA before posting its
                // binding mark. The mailbox is ready while this FnOnce mark is
                // withheld; no mark delivery is claimed. This direct settled poll
                // consumes actual receipt DATA after the read callback leaves
                // window work beyond this turn's original window snapshot.
                f.turn();
                let receipt = f.joined();
                let partial = receipt.is_some_and(|(seen_binding, operation, result, on)| {
                    seen_binding == binding
                        && Some(operation) == f.runtime.terminal_operation.get()
                        && result == Some(TickExit::Cancelled)
                        && on != thread::current().id()
                }) && f.runtime.pending.get()
                    && !f.runtime.windows.borrow().is_empty()
                    && !f.claim()
                    && matches!(*read.lock().unwrap(), Some(Ok(Code0Reply::View(_))));
                // Only now consume the real retained window work truthfully.
                f.turn();
                let joined = f.joined();
                let claims = [f.claim(), f.claim()];
                let stale_safe =
                    if claims == [true, false] && joined.is_some() && old_jobs.len() >= 2 {
                        // Only actual product completion can enter retirement.
                        f.remove_record();
                        let replacement = Fixture::new();
                        for job in old_jobs {
                            job();
                        }
                        let safe = !replacement.runtime.pending.get()
                            && replacement.backend.borrow().calls.is_empty()
                            && request_terminal(binding) == Err(Code0DispatchError::Unavailable)
                            && replacement.view()["state"] == "paused";
                        replacement.remove_record();
                        safe
                    } else {
                        false
                    };
                (
                    first, handoff, completion, during, partial, claims, joined, stale_safe,
                )
            });
            check_cleanup(&cleanup, setup, expired);
            assert_eq!(first, Some(false));
            assert!(
                handoff,
                "TJ-R4 terminal join must transfer the actual worker into a retained completion obligation"
            );
            assert!(
                completion.is_some(),
                "terminal fixture lacked a genuine completion post after actual worker handoff"
            );
            assert!(
                during && partial,
                "receipt bypassed actual retained work/partial settlement"
            );
            assert_eq!(
                claims,
                [true, false],
                "TJ-R4 current record must recheck receipt then yield one consumed permit after truthful settlement"
            );
            assert!(
                joined.is_some_and(|(_, _, result, on)| result == Some(TickExit::Cancelled)
                    && on != thread::current().id())
            );
            assert!(
                stale_safe,
                "actual retired operation jobs marked/polled/acted on replacement"
            );
            assert!(cleanup.worker.is_none());
        }
    }
}
