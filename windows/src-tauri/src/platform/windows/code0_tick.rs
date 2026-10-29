//! Conservative test-only tick transport baseline. Periodic posting is intentionally absent.
//! Preparation attaches a UI ACK port before a real cancellable worker can start.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::code0_runtime::{MainThreadExecutor, UiJob, mark_tick, tick_attached};

const INTERVAL: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TickExit {
    Cancelled,
    ControlLost,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TickStartFailure {
    NotAttached,
    AlreadyStarted,
    Cancelled,
    SpawnFailed,
    ControlLost,
}
struct Signals {
    attached: AtomicBool,
    posted: AtomicBool,
    cancelled: AtomicBool,
    fault: AtomicBool,
}
#[derive(Clone)]
pub(super) struct TickAcknowledgement {
    binding: u64,
    signals: Arc<Signals>,
}
impl TickAcknowledgement {
    pub(super) fn attach(&self, binding: u64) -> bool {
        self.binding == binding
            && !self.signals.cancelled.load(Ordering::Acquire)
            && self
                .signals
                .attached
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
    }
    pub(super) fn matches(&self, other: &Self) -> bool {
        self.binding == other.binding && Arc::ptr_eq(&self.signals, &other.signals)
    }
    pub(super) fn delivered(&self, binding: u64) -> bool {
        self.binding == binding
            && !self.signals.cancelled.load(Ordering::Acquire)
            && self.signals.posted.swap(false, Ordering::AcqRel)
    }
    pub(super) fn faulted(&self) -> bool {
        self.signals.fault.load(Ordering::Acquire)
    }
}
enum Control {
    Start,
    Cancel,
}

#[must_use = "keep the producer and transfer its real join handle before retirement"]
pub(super) struct TickProducer {
    binding: u64,
    executor: Arc<dyn MainThreadExecutor>,
    signals: Arc<Signals>,
    control: Option<Sender<Control>>,
    worker: Option<JoinHandle<TickExit>>,
    started: bool,
}
impl TickProducer {
    pub(super) fn prepare(binding: u64, executor: Arc<dyn MainThreadExecutor>) -> Self {
        Self {
            binding,
            executor,
            signals: Arc::new(Signals {
                attached: AtomicBool::new(false),
                posted: AtomicBool::new(false),
                cancelled: AtomicBool::new(false),
                fault: AtomicBool::new(false),
            }),
            control: None,
            worker: None,
            started: false,
        }
    }
    pub(super) fn acknowledgement(&self) -> TickAcknowledgement {
        TickAcknowledgement {
            binding: self.binding,
            signals: Arc::clone(&self.signals),
        }
    }
    pub(super) fn start(&mut self) -> Result<(), TickStartFailure> {
        if self.started {
            return Err(TickStartFailure::AlreadyStarted);
        }
        if self.signals.cancelled.load(Ordering::Acquire) {
            return Err(TickStartFailure::Cancelled);
        }
        if !self.signals.attached.load(Ordering::Acquire)
            || !tick_attached(self.binding, &self.acknowledgement())
        {
            return Err(TickStartFailure::NotAttached);
        }
        let (send, receive) = mpsc::channel();
        let binding = self.binding;
        let executor = Arc::clone(&self.executor);
        let signals = Arc::clone(&self.signals);
        let worker = thread::Builder::new()
            .name("code0-tick".into())
            .spawn(move || run(binding, executor, signals, receive))
            .map_err(|_| TickStartFailure::SpawnFailed)?;
        // The spawned thread waits for Start. Its cancel endpoint and real join
        // ownership are stored before that message can enable interval waiting.
        self.worker = Some(worker);
        self.control = Some(send);
        self.started = true;
        if self
            .control
            .as_ref()
            .is_none_or(|send| send.send(Control::Start).is_err())
        {
            self.signals.fault.store(true, Ordering::Release);
            return Err(TickStartFailure::ControlLost);
        }
        Ok(())
    }
    pub(super) fn cancel(&self) {
        if !self.signals.cancelled.swap(true, Ordering::AcqRel)
            && let Some(send) = &self.control
            && send.send(Control::Cancel).is_err()
        {
            self.signals.fault.store(true, Ordering::Release);
        }
    }
    // Caller must perform this actual join off UI after real quiescence. Taking
    // the handle also closes control, so lost cancellation ends as ControlLost.
    // Neither taking/dropping a handle nor cancel() is a successful join receipt.
    pub(super) fn take_join(&mut self) -> Option<JoinHandle<TickExit>> {
        drop(self.control.take());
        self.worker.take()
    }
}
impl Drop for TickProducer {
    fn drop(&mut self) {
        self.cancel();
        // No UI blocking join, detached-success claim or terminal permission.
        // The future owner must retain/take the handle before dropping this value.
    }
}
fn binding_job(binding: u64) -> UiJob {
    Box::new(move || mark_tick(binding))
}
fn run(
    binding: u64,
    _executor: Arc<dyn MainThreadExecutor>,
    signals: Arc<Signals>,
    control: Receiver<Control>,
) -> TickExit {
    match control.recv() {
        Ok(Control::Start) => {}
        Ok(Control::Cancel) => return TickExit::Cancelled,
        Err(_) => {
            signals.fault.store(true, Ordering::Release);
            return TickExit::ControlLost;
        }
    }
    loop {
        match control.recv_timeout(INTERVAL) {
            Ok(Control::Cancel) => return TickExit::Cancelled,
            Ok(Control::Start) | Err(RecvTimeoutError::Disconnected) => {
                signals.fault.store(true, Ordering::Release);
                return TickExit::ControlLost;
            }
            Err(RecvTimeoutError::Timeout) => {
                // Intentional RED baseline: no post, posted-CAS or error recovery.
                // The binding-only mark job exists but is never submitted.
                drop(binding_job(binding));
            }
        }
    }
}
