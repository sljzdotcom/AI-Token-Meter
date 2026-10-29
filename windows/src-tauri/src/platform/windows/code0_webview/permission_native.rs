// Selected debug-only permission adapter. Compilation/tests/native are UNRUN.
use super::super::permission_protocol::{
    self as semantic, Boundary, Command, CommandTicket, CompletionValue, CoverageKey, CutoffReason,
    Id, Identity, Lifecycle, NativeCheck, NativeTicket, NativeToken, Phase, Problem, Protocol,
    Receiver, RemovalTicket,
};
use super::super::permission_wire as wire;
use super::*;
use serde::Serialize;
use std::cell::Cell;
use std::sync::atomic::{AtomicU32, AtomicU64};
use std::time::{Duration, Instant};
use windows::core::BOOL;

pub const SELECTOR: &str = "refusal-notification-owned-https";
const ALL: u32 = 0x00ff_ffff;

// Return false on the first duplicate/out-of-normal-budget entry as well as on
// further saturation. A representation is a lower bound, never more authority.
fn increment(value: &mut u16, normal: u16) -> bool {
    *value = value.saturating_add(1).min(normal + 1);
    *value <= normal
}
fn counted(value: &mut u16, normal: u16, overflow: &mut bool, failure: &mut Option<&'static str>) {
    if !increment(value, normal) {
        *overflow = true;
        if failure.is_none() {
            *failure = Some("permissionOverflow");
        }
    }
}
#[cfg(test)]
fn s16(value: &mut u16) -> bool {
    if *value == u16::MAX {
        false
    } else {
        *value += 1;
        true
    }
}
macro_rules! record {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Default, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct $name { $(pub $field: $ty),* }
    };
}
record!(Setup {hooks1023:u16,settings_ack:Option<bool>,responder_ready:Option<bool>,receivers_ready:Option<bool>,domains_ack:Option<bool>,document_bound:Option<bool>,lifecycle_assembly_complete:Option<bool>,blank_branch_closed:Option<bool>,observation_coverage_intact_at_arm:Option<bool>,original_deadline_bound:Option<bool>});
record!(Counts {
    navigation: u16,
    main_responses: u16,
    permission_entries: u16,
    kind_reads: u16,
    gesture_reads: u16,
    origin_reads: u16,
    state_before_reads: u16,
    deny_attempts: u16,
    deny_returns: u16,
    deny_successes: u16,
    state_after_reads: u16,
    action_receipts: u16,
    evaluate_submissions: u16,
    evaluate_completions: u16,
    effect_receipts: u16,
    session_policy_failures: u16,
    final_receipts: u16,
    unexpected_requests: u16,
    receiver_adds: u16,
    receiver_removes: u16,
    command_issued: u16,
    command_returned: u16,
    command_completed: u16,
    command_consumed: u16
});
record!(Native {sender_matched:Option<bool>,request_installation_matched:Option<bool>,kind_notifications:Option<bool>,non_user:Option<bool>,origin_matched:Option<bool>,state_before_default:Option<bool>,deny_succeeded:Option<bool>,state_after_deny:Option<bool>,original_timely:Option<bool>,post_drop_matched:Option<bool>,entered_before_observation_cutoff:Option<bool>,captured_before_observation_cutoff:Option<bool>,qualified_before_observation_cutoff:Option<bool>});
record!(Effect {success_hresult:Option<bool>,closed_denied_result:Option<bool>,same_ticket:Option<bool>,same_frame_loader_context:Option<bool>,source_matched:Option<bool>,invoked_after_native_action:Option<bool>,invoked_before_stop:Option<bool>,captured_before_stop:Option<bool>,qualified_before_stop:Option<bool>,entered_before_observation_cutoff:Option<bool>,captured_before_observation_cutoff:Option<bool>,qualified_before_observation_cutoff:Option<bool>,original_timely:Option<bool>,close_timely:Option<bool>,epoch_unchanged_through_qualification:Option<bool>});
record!(OrderingFacts {permanent_revocation_at_refusal:Option<bool>,first_close_at_refusal:Option<bool>,no_action_reauthorized:Option<bool>,no_wait_for_effect:Option<bool>,actual_policy_failure_after_action:Option<bool>,cutoff_latched:Option<bool>,cutoff_before_any_required_removal:Option<bool>,qualified_observations_are_historical:Option<bool>,physical_close_observed:Option<bool>});
record!(Drain {obligations_empty:Option<bool>,handlers_empty:Option<bool>,text_empty:Option<bool>,receiver_tokens_removed:Option<bool>,resource_and_filter_removed:Option<bool>,controls_empty:Option<bool>});
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Coverage {
    pub required_mask: u32,
    pub added_mask: u32,
    pub remove_attempt_mask: u32,
    pub removed_mask: u32,
    pub uncertain_mask: u32,
    pub owned_mask: u32,
    pub add_attempts: [u16; 24],
    pub add_acks: [u16; 24],
    pub remove_attempts: [u16; 24],
    pub remove_acks: [u16; 24],
    pub add_returns: [Option<bool>; 24],
    pub remove_returns: [Option<bool>; 24],
    #[serde(skip)]
    overflow: bool,
}
impl Default for Coverage {
    fn default() -> Self {
        Self {
            required_mask: ALL,
            added_mask: 0,
            remove_attempt_mask: 0,
            removed_mask: 0,
            uncertain_mask: 0,
            owned_mask: 0,
            add_attempts: [0; 24],
            add_acks: [0; 24],
            remove_attempts: [0; 24],
            remove_acks: [0; 24],
            add_returns: [None; 24],
            remove_returns: [None; 24],
            overflow: false,
        }
    }
}
impl Coverage {
    fn add_entered(&mut self, slot: usize) {
        self.overflow |= !increment(&mut self.add_attempts[slot], 1);
        self.owned_mask |= 1 << slot;
        self.uncertain_mask |= 1 << slot;
    }
    fn add_returned(&mut self, slot: usize, success: bool) {
        if self.add_returns[slot].is_none() {
            self.add_returns[slot] = Some(success);
        } else {
            self.overflow = true;
        }
    }
    fn add_qualified(&mut self, slot: usize, current: bool) {
        if current && self.add_attempts[slot] == 1 && self.add_returns[slot] == Some(true) {
            self.overflow |= !increment(&mut self.add_acks[slot], 1);
            self.added_mask |= 1 << slot;
            self.uncertain_mask &= !(1 << slot);
        }
    }
    fn remove_entered(&mut self, slot: usize) {
        self.overflow |= !increment(&mut self.remove_attempts[slot], 1);
        self.remove_attempt_mask |= 1 << slot;
        self.uncertain_mask |= 1 << slot;
    }
    fn remove_returned(&mut self, slot: usize, success: bool) {
        if self.remove_returns[slot].is_none() {
            self.remove_returns[slot] = Some(success);
        } else {
            self.overflow = true;
        }
    }
    fn remove_qualified(&mut self, slot: usize, current: bool) {
        if current && self.remove_attempts[slot] == 1 && self.remove_returns[slot] == Some(true) {
            self.overflow |= !increment(&mut self.remove_acks[slot], 1);
            self.removed_mask |= 1 << slot;
            self.owned_mask &= !(1 << slot);
            self.uncertain_mask &= !(1 << slot);
        }
    }
}
record!(Commands {issued:[u16;4],returned:[u16;4],completed:[u16;4],consumed:[u16;4],handler_drops:[u16;4],submission_succeeded:[Option<bool>;4],completion_succeeded:[Option<bool>;4],qualified:[Option<bool>;4],values:[Option<&'static str>;4],obligations:Option<u16>,handlers:Option<u16>,in_flight:Option<u16>});
record!(Observations {blank_branch:Option<&'static str>,bootstrap_deliveries:u16,disposal_deliveries:u16,deliveries:u16,post_arm_pre_cutoff:u16,by_receiver:[u16;10],counter_overflow:bool});
#[cfg(test)]
impl Observations {
    fn enter(&mut self, slot: usize, phase: Phase, cutoff: bool) {
        self.counter_overflow |= !s16(&mut self.deliveries);
        self.counter_overflow |= !s16(&mut self.by_receiver[slot]);
        let partition = if cutoff {
            &mut self.disposal_deliveries
        } else if matches!(
            phase,
            Phase::Registering | Phase::Enabling | Phase::Navigating
        ) {
            &mut self.bootstrap_deliveries
        } else {
            &mut self.post_arm_pre_cutoff
        };
        self.counter_overflow |= !s16(partition);
    }
}
// The cursor CAS is both the entry linearization point and the phase read.
// UI publishes only after a pure Protocol transaction, before any native/Drop
// boundary. Per-row increments finish under the already-entered actual Active;
// only the final physically quiescent snapshot can claim an exact partition.
struct LifecycleEntries {
    cursor: AtomicU64,
    rows: [AtomicU32; 30],
    overflow: AtomicBool,
}
impl Default for LifecycleEntries {
    fn default() -> Self {
        Self {
            cursor: AtomicU64::new(0),
            rows: std::array::from_fn(|_| AtomicU32::new(0)),
            overflow: AtomicBool::new(false),
        }
    }
}
impl LifecycleEntries {
    fn publish(&self, phase: Phase, cutoff: bool) {
        let partition = if cutoff {
            2
        } else if matches!(
            phase,
            Phase::Registering | Phase::Enabling | Phase::Navigating
        ) {
            0
        } else {
            1
        };
        let _ = self
            .cursor
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old| {
                Some((old & !3) | partition)
            });
    }
    fn enter(&self, receiver: usize) {
        // Fixed receiver enum supplies 0..10; no ABI argument is inspected here.
        let old = self
            .cursor
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old| {
                let total = old >> 2;
                Some(((total + 1).min(u64::from(u16::MAX)) << 2) | (old & 3))
            })
            .expect("infallible fixed atomic entry update");
        if old >> 2 == u64::from(u16::MAX) {
            self.overflow.store(true, Ordering::SeqCst);
        }
        let row = &self.rows[receiver * 3 + (old & 3) as usize];
        let old = row
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old| {
                Some((old + 1).min(u32::from(u16::MAX)))
            })
            .expect("infallible fixed atomic receiver update");
        if old == u32::from(u16::MAX) {
            self.overflow.store(true, Ordering::SeqCst);
        }
    }
    fn snapshot(&self) -> Observations {
        let mut result = Observations {
            deliveries: (self.cursor.load(Ordering::SeqCst) >> 2) as u16,
            counter_overflow: self.overflow.load(Ordering::SeqCst),
            ..Observations::default()
        };
        let mut partitions = [0u32; 3];
        for (receiver, count) in result.by_receiver.iter_mut().enumerate() {
            let mut sum = 0u32;
            for (partition, total) in partitions.iter_mut().enumerate() {
                let value = self.rows[receiver * 3 + partition].load(Ordering::SeqCst);
                sum += value;
                *total += value;
            }
            result.counter_overflow |= sum > u32::from(u16::MAX);
            *count = sum.min(u32::from(u16::MAX)) as u16;
        }
        result.counter_overflow |= partitions.iter().any(|n| *n > u32::from(u16::MAX));
        result.bootstrap_deliveries = partitions[0].min(u32::from(u16::MAX)) as u16;
        result.post_arm_pre_cutoff = partitions[1].min(u32::from(u16::MAX)) as u16;
        result.disposal_deliveries = partitions[2].min(u32::from(u16::MAX)) as u16;
        result
    }
}
record!(Inventory {text_bytes:Option<u16>,text_peak_bytes:Option<u16>,text_reserve_failures:u16,text_accounting_complete:Option<bool>,active_guards:Option<u32>,aggregate_handlers:Option<u32>,new_event_handlers:Option<u16>,command_handlers:Option<u16>,handler_accounting_complete:Option<bool>,record_slots:Option<u16>,record_slots_peak:Option<u16>,completion_slot:Option<u16>,document_record:Option<u16>,native_record:Option<u16>});
record!(Times {native_entered_at_ms:Option<u64>,native_captured_at_ms:Option<u64>,native_qualified_at_ms:Option<u64>,effect_entered_at_ms:Option<u64>,effect_captured_at_ms:Option<u64>,effect_qualified_at_ms:Option<u64>,first_close_at_ms:Option<u64>,observation_cutoff_at_ms:Option<u64>,stop_entered_at_ms:Option<u64>,controller_close_entered_at_ms:Option<u64>,released_at_ms:Option<u64>,final_at_ms:Option<u64>,cleanup_elapsed_ms:Option<u64>,final_within_close_deadline:Option<bool>,same_original_clock:Option<bool>,first_close_adopted:Option<bool>});
record!(StatusFacts {protocol_phase:Option<&'static str>,protocol_failure:Option<&'static str>,observation_epoch:Option<u64>,cutoff_reason:Option<&'static str>,native_qualified:Option<bool>,effect_qualified:Option<bool>,effect_result:Option<&'static str>,session_reason:Option<&'static str>,navigation_succeeded:Option<bool>,main_response_succeeded:Option<bool>,terminal_result:Option<&'static str>});
record!(Cleanup {browser_process_exits:u16,window_destroyed:u16,wake_joins:u16,temp_dir_close_attempts:u16,released_ref_zero:Option<bool>,wake_shutdown_issued:Option<bool>,wake_join_succeeded:Option<bool>,temp_dir_close_succeeded:Option<bool>,temp_dir_result_dropped:Option<bool>});
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Facts {
    pub version: u8,
    pub selector: &'static str,
    pub phase: &'static str,
    pub unknown: bool,
    pub overflow: bool,
    pub setup: Setup,
    pub counts: Counts,
    pub native: Native,
    pub effect: Effect,
    pub ordering: OrderingFacts,
    pub drain: Drain,
    pub coverage: Coverage,
    pub commands: Commands,
    pub observations: Observations,
    pub inventory: Inventory,
    pub timing: Times,
    pub status: StatusFacts,
    pub cleanup: Cleanup,
    #[serde(skip)]
    pub failure: Option<&'static str>,
}
impl Default for Facts {
    fn default() -> Self {
        Self {
            version: 1,
            selector: SELECTOR,
            phase: "Registering",
            unknown: false,
            overflow: false,
            setup: Setup::default(),
            counts: Counts::default(),
            native: Native::default(),
            effect: Effect::default(),
            ordering: OrderingFacts::default(),
            drain: Drain::default(),
            coverage: Coverage::default(),
            commands: Commands::default(),
            observations: Observations::default(),
            inventory: Inventory::default(),
            timing: Times::default(),
            status: StatusFacts::default(),
            cleanup: Cleanup::default(),
            failure: None,
        }
    }
}
impl Facts {
    fn fail(&mut self, why: &'static str) {
        if self.failure.is_none() {
            self.failure = Some(why);
        }
    }
}
#[derive(Default)]
struct TextLedger {
    clock: std::sync::OnceLock<Arc<FailureClock>>,
    live: AtomicUsize,
    peak: AtomicUsize,
    failures: AtomicUsize,
    failed: AtomicBool,
}
struct TextCharge {
    ledger: Arc<TextLedger>,
    bytes: usize,
}
impl TextCharge {
    fn reserve(ledger: &Arc<TextLedger>, bytes: usize) -> Option<Self> {
        let changed = ledger
            .live
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old| {
                old.checked_add(bytes).filter(|sum| *sum <= 4096)
            });
        match changed {
            Ok(old) => {
                ledger.peak.fetch_max(old + bytes, Ordering::SeqCst);
                Some(Self {
                    ledger: Arc::clone(ledger),
                    bytes,
                })
            }
            Err(_) => {
                if let Some(clock) = ledger.clock.get() {
                    clock.record(Instant::now());
                }
                ledger.failed.store(true, Ordering::SeqCst);
                let _ = ledger
                    .failures
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old| {
                        Some(old.saturating_add(1).min(2))
                    });
                None
            }
        }
    }
}
impl Drop for TextCharge {
    fn drop(&mut self) {
        self.ledger.live.fetch_sub(self.bytes, Ordering::SeqCst);
    }
}
// Both closures run exactly once even if the capture or DENY HRESULT fails.
// Actual metadata/native calls are supplied by the selected native delivery.
fn restrictive_sequence<T>(
    capture: impl FnOnce() -> T,
    deny: impl FnOnce() -> bool,
    readback: impl FnOnce() -> Option<bool>,
) -> (T, bool, Option<bool>) {
    let captured = capture();
    let denied = deny();
    let after = readback();
    (captured, denied, after)
}
#[derive(Clone)]
pub struct Timing {
    started: Instant,
    end: Option<Instant>,
    first_close: Option<Instant>,
}
impl Timing {
    fn new(started: Instant) -> Self {
        Self {
            started,
            end: started.checked_add(Duration::from_secs(45)),
            first_close: None,
        }
    }
    fn before(&self, now: Instant) -> bool {
        now >= self.started && self.end.is_some_and(|end| now < end)
    }
    fn close(&mut self, at: Instant) {
        self.first_close = Some(self.first_close.map_or(at, |old| old.min(at)));
    }
    pub fn first_close(&self) -> Option<Instant> {
        self.first_close
    }
    fn close_before(&self, now: Instant) -> bool {
        self.first_close.is_some_and(|at| {
            now >= at
                && at
                    .checked_add(Duration::from_secs(10))
                    .is_some_and(|end| now < end)
        })
    }
    pub fn matches_start(&self, started: Instant) -> bool {
        self.started == started
    }
    fn ms(&self, now: Instant) -> Option<u64> {
        now.checked_duration_since(self.started)
            .and_then(|d| u64::try_from(d.as_millis()).ok())
    }
}

// Shared production carrier: no observer/native work or owned native Drop.
fn latch_close(
    timing: &mut Timing,
    history: &mut Option<Instant>,
    failed: &mut bool,
    closing: &mut bool,
    at: Instant,
) {
    let earliest = history.map_or(at, |old| old.min(at));
    timing.close(earliest);
    *history = timing.first_close;
    *failed = true;
    *closing = true;
}

// One pure negative transition shared by metadata/count/Protocol callers.
// `at` is sampled at the first known failure boundary, never reconstructed from ms.
fn settle_failure(
    protocol: &mut Protocol,
    facts: &mut Facts,
    timing: &mut Timing,
    identity: Identity,
    at: Instant,
) {
    if protocol.status().failure.is_some() {
        facts.fail("permissionProtocolRejected");
    }
    if facts.failure.is_some() || facts.overflow || facts.coverage.overflow {
        timing.close(at);
        facts.fail("permissionOverflow");
        if protocol.status().failure.is_none() {
            protocol.native_adapter_failed(Boundary {
                identity,
                ms: timing.ms(at).unwrap_or(u64::MAX),
            });
        }
    }
    let status = protocol.status();
    if status.first_close_ms.is_some() || status.cutoff.is_some() {
        timing.close(at);
    }
}

// Pre-selection history is written by the original S2/message producers. Tokens
// are adopted only after their genuine add-return and protected postDrop ACK.
#[derive(Default)]
pub(super) struct History {
    pub(super) facts: Coverage,
    settings: Option<bool>,
    inventory: Arc<NativeInventory>,
    failure: Option<&'static str>,
    first_close: Option<Instant>,
    unknown: bool,
}
pub(super) fn add_entered(owner: &Rc<RefCell<Owner>>, slot: usize) {
    let overflow = if let Some(i) = owner.borrow_mut().instance.as_mut() {
        i.permission_history.facts.add_entered(slot);
        i.permission_history.facts.overflow
    } else {
        false
    };
    with_state(owner, |s| s.facts.coverage.add_entered(slot));
    if overflow {
        reject(owner, "permissionOverflow", false);
    }
}
pub(super) fn add_returned(owner: &Rc<RefCell<Owner>>, slot: usize, success: bool) {
    let at = Instant::now();
    let overflow = if let Some(i) = owner.borrow_mut().instance.as_mut() {
        i.permission_history.facts.add_returned(slot, success);
        i.permission_history.facts.overflow
    } else {
        false
    };
    with_state(owner, |s| {
        s.facts.coverage.add_returned(slot, success);
        if !success {
            s.fail_at("permissionSetupRejected", at);
        }
    });
    if !success || overflow {
        reject_at(owner, "permissionSetupRejected", !success, at);
    }
}
pub(super) fn add_qualified(owner: &Rc<RefCell<Owner>>, slot: usize, current: bool) {
    let token = {
        let mut o = owner.borrow_mut();
        let Some(i) = o.instance.as_mut() else { return };
        i.permission_history.facts.add_qualified(slot, current);
        if slot == 23 {
            i.message_token
        } else {
            i.refusal_registry.permission_token(slot)
        }
    };
    with_state(owner, |s| {
        s.facts.coverage.add_qualified(slot, current);
        let boundary = Boundary {
            identity: s.identity,
            ms: s.timing.ms(Instant::now()).unwrap_or(u64::MAX),
        };
        if current {
            if let Some(token) = token {
                if s.protocol
                    .registration_added(key(slot), NativeToken::Event(token), boundary)
                    .is_err()
                {
                    s.fail("permissionSetupRejected");
                }
            } else {
                s.fail("permissionSetupRejected");
            }
        } else {
            s.protocol.cutoff(CutoffReason::CoverageLost, boundary);
            s.fail("permissionSetupRejected");
        }
    });
}
pub(super) fn settings_acknowledged(owner: &Rc<RefCell<Owner>>) {
    if let Some(i) = owner.borrow_mut().instance.as_mut() {
        i.permission_history.settings = Some(true);
    }
    with_state(owner, |s| {
        s.facts.setup.settings_ack = Some(true);
        let boundary = Boundary {
            identity: s.identity,
            ms: s.timing.ms(Instant::now()).unwrap_or(u64::MAX),
        };
        let _ = s.protocol.settings_acknowledged(boundary);
    });
}

// Bound once to the real selected instance's original attempt, before any of
// its handlers are constructed. Only immutable Instant + fixed atomics off UI.
pub(super) struct FailureClock {
    started: std::sync::OnceLock<Instant>,
    nanos: AtomicU64,
    failed: AtomicBool,
    invalid: AtomicBool,
}
impl Default for FailureClock {
    fn default() -> Self {
        Self {
            started: std::sync::OnceLock::new(),
            nanos: AtomicU64::new(u64::MAX),
            failed: AtomicBool::new(false),
            invalid: AtomicBool::new(false),
        }
    }
}
impl FailureClock {
    fn bind(&self, started: Instant) -> bool {
        if self.failed.load(Ordering::SeqCst) || self.started.set(started).is_err() {
            self.invalid.store(true, Ordering::SeqCst);
            self.failed.store(true, Ordering::SeqCst);
            false
        } else {
            true
        }
    }
    fn invalidate(&self, at: Instant) {
        self.invalid.store(true, Ordering::SeqCst);
        self.record(at);
    }
    fn matches(&self, started: Instant) -> bool {
        self.started.get() == Some(&started) && !self.invalid.load(Ordering::SeqCst)
    }
    pub(super) fn record(&self, at: Instant) {
        let nanos = self
            .started
            .get()
            .and_then(|start| at.checked_duration_since(*start))
            .and_then(|duration| u64::try_from(duration.as_nanos()).ok());
        if let Some(nanos) = nanos {
            self.nanos.fetch_min(nanos, Ordering::SeqCst);
        } else {
            self.invalid.store(true, Ordering::SeqCst);
        }
        // A visible failure flag always follows recording the real origin/error.
        self.failed.store(true, Ordering::SeqCst);
    }
    fn origin(&self) -> Result<Option<Instant>, ()> {
        if self.invalid.load(Ordering::SeqCst) {
            return Err(());
        }
        if !self.failed.load(Ordering::SeqCst) {
            return Ok(None);
        }
        let at = self.started.get().and_then(|start| {
            start.checked_add(Duration::from_nanos(self.nanos.load(Ordering::SeqCst)))
        });
        if at.is_none() {
            self.invalid.store(true, Ordering::SeqCst);
        }
        if self.invalid.load(Ordering::SeqCst) {
            Err(())
        } else {
            at.map(Some).ok_or(())
        }
    }
    fn adopt(&self, timing: &mut Timing) -> Result<(), ()> {
        if !self.matches(timing.started) {
            return Err(());
        }
        if let Some(at) = self.origin()? {
            timing.close(at);
        }
        Ok(())
    }
}
fn affinity_entry(ui: ThreadId, callbacks: &Callbacks, clock: &FailureClock, code: u16) -> bool {
    // Short-circuit before even apartment inspection off UI. The same actual
    // validity check as callback_on_ui, with the original clock stored first.
    let valid = std::thread::current().id() == ui && is_sta();
    if !valid {
        clock.record(Instant::now());
        n4_note(
            callbacks,
            crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
            code,
            0,
        );
        callbacks.affinity_failed.store(true, Ordering::SeqCst);
        if let Some(diagnostic) = callbacks.diagnostics.as_ref() {
            diagnostic.failure(DP::UiAffinity, 0);
        }
    }
    valid
}
impl History {
    pub(super) fn clock(&self) -> Arc<FailureClock> {
        Arc::clone(&self.inventory.clock)
    }
}
pub(super) fn history_for_create(
    owner: &Owner,
    request: &Code0WindowRequest,
    selected: bool,
) -> Result<History, Code0SessionFailure> {
    let history = History::default();
    if selected {
        let intent = owner
            .permission_intent
            .as_ref()
            .filter(|intent| &intent.request == request && !intent.failed)
            .ok_or(Code0SessionFailure::PolicyRejected)?;
        if !history.inventory.clock.bind(intent.started)
            || history
                .inventory
                .text
                .clock
                .set(Arc::clone(&history.inventory.clock))
                .is_err()
        {
            return Err(Code0SessionFailure::PolicyRejected);
        }
    }
    Ok(history)
}
pub(super) fn clock_for(owner: &Rc<RefCell<Owner>>) -> Option<Arc<FailureClock>> {
    let clock = {
        let o = owner.borrow();
        let i = o.instance.as_ref()?;
        let clock = i.permission_history.clock();
        if i.callbacks
            .permission_clock
            .get()
            .is_some_and(|bound| Arc::ptr_eq(bound, &clock))
        {
            Some(clock)
        } else {
            clock.invalidate(Instant::now());
            None
        }
    };
    if clock.is_none() {
        reject(owner, "permissionIdentityChanged", true);
    }
    clock
}
pub(super) fn native_failed(owner: &Rc<RefCell<Owner>>) {
    if selected(owner) {
        reject(owner, "permissionNativeMetadataFailed", true);
    }
}

#[derive(Default)]
struct NativeInventory {
    text: Arc<TextLedger>,
    clock: Arc<FailureClock>,
    new_handlers: AtomicUsize,
    command_handlers: AtomicUsize,
    command_drop_bits: AtomicU32,
    affinity_cutoff: AtomicBool,
    lifecycle_entries: LifecycleEntries,
    action_slots: AtomicU32,
    slot_failed: AtomicBool,
}
struct EventLifetime {
    inventory: Arc<NativeInventory>,
    _aggregate: HandlerLifetime,
}
impl EventLifetime {
    fn new(callbacks: &Arc<Callbacks>, inventory: &Arc<NativeInventory>) -> Self {
        inventory.new_handlers.fetch_add(1, Ordering::SeqCst);
        Self {
            inventory: Arc::clone(inventory),
            _aggregate: HandlerLifetime::new(callbacks),
        }
    }
}
impl Drop for EventLifetime {
    fn drop(&mut self) {
        self.inventory.new_handlers.fetch_sub(1, Ordering::SeqCst);
    }
}
struct CommandLifetime {
    inventory: Arc<NativeInventory>,
    _aggregate: HandlerLifetime,
    index: usize,
    armed: Arc<AtomicBool>,
}
impl CommandLifetime {
    fn new(
        callbacks: &Arc<Callbacks>,
        inventory: &Arc<NativeInventory>,
        index: usize,
        armed: Arc<AtomicBool>,
    ) -> Self {
        inventory.command_handlers.fetch_add(1, Ordering::SeqCst);
        Self {
            inventory: Arc::clone(inventory),
            _aggregate: HandlerLifetime::new(callbacks),
            index,
            armed,
        }
    }
}
impl Drop for CommandLifetime {
    fn drop(&mut self) {
        self.inventory
            .command_handlers
            .fetch_sub(1, Ordering::SeqCst);
        // An unissued prepared handler is a real object, but not an operation's
        // handler Drop. This distinction never consumes an absent callback.
        if self.armed.load(Ordering::SeqCst) {
            self.inventory
                .command_drop_bits
                .fetch_or(1 << self.index, Ordering::SeqCst);
        }
    }
}
// This fixed slot never owns a COM strong reference. The one permitted handoff
// is prepare's original Rc to the canonically verified S2 Rc at begin.
struct DisposalAnchor<T> {
    core: RefCell<Weak<T>>,
    rebound: Cell<bool>,
}
impl<T> DisposalAnchor<T> {
    fn new(core: &Rc<T>) -> Self {
        Self {
            core: RefCell::new(Rc::downgrade(core)),
            rebound: Cell::new(false),
        }
    }
    fn rebind_once(&self, core: &Rc<T>, cutoff: bool) -> bool {
        if self.rebound.replace(true) || cutoff {
            return false;
        }
        *self.core.borrow_mut() = Rc::downgrade(core);
        true
    }
    fn for_disposal(&self, cutoff: bool) -> Option<Rc<T>> {
        if cutoff {
            self.core.borrow().upgrade()
        } else {
            None
        }
    }
}
#[derive(Clone)]
struct Context {
    owner: Weak<RefCell<Owner>>,
    request: Code0WindowRequest,
    installation: Arc<AtomicBool>,
    callbacks: Arc<Callbacks>,
    inventory: Arc<NativeInventory>,
    ui: ThreadId,
    disposal: Rc<DisposalAnchor<ICoreWebView2>>,
}
impl Context {
    fn ui(&self) -> bool {
        if !affinity_entry(self.ui, &self.callbacks, &self.inventory.clock, 8701) {
            self.inventory.affinity_cutoff.store(true, Ordering::SeqCst);
            false
        } else {
            true
        }
    }
    fn same(&self, owner: &Rc<RefCell<Owner>>) -> bool {
        let o = owner.borrow();
        !o.unmatched
            && o.instance.as_ref().is_some_and(|i| {
                i.request == self.request
                    && Arc::ptr_eq(&i.callbacks, &self.callbacks)
                    && i.permission.as_ref().is_some_and(|s| {
                        Arc::ptr_eq(&s.ctx.inventory, &self.inventory)
                            && Arc::ptr_eq(&s.ctx.installation, &self.installation)
                    })
                    && !i.released
            })
    }
    fn sender(&self, owner: &Rc<RefCell<Owner>>, sender: Option<&ICoreWebView2>) -> bool {
        if !self.same(owner) {
            return false;
        }
        let core = owner
            .borrow()
            .instance
            .as_ref()
            .and_then(|i| i.permission.as_ref())
            .and_then(|s| s.core.as_ref())
            .map(Rc::clone);
        let good = core.as_ref().zip(sender).is_some_and(|(core, sender)| {
            let expected = core.cast::<IUnknown>();
            native_result(owner, expected.is_ok(), "permissionIdentityChanged", true);
            let before = self.same(owner) && expected.is_ok();
            let actual = if before {
                sender.cast::<IUnknown>()
            } else {
                Err(rejection())
            };
            let good = before
                && self.same(owner)
                && expected
                    .as_ref()
                    .ok()
                    .zip(actual.as_ref().ok())
                    .is_some_and(|(a, b)| a.as_raw() == b.as_raw());
            native_result(owner, good, "permissionIdentityChanged", true);
            drop(actual);
            let after_actual = self.same(owner);
            drop(expected);
            good && after_actual && self.same(owner)
        });
        drop(core);
        good && self.same(owner)
    }
    fn disposal_sender(&self, owner: &Rc<RefCell<Owner>>, sender: Option<&ICoreWebView2>) -> bool {
        if !self.same(owner) || self.disposal_boundary(owner).is_none() {
            return false;
        }
        let cutoff = with_state(owner, |s| s.protocol.status().cutoff.is_some()).unwrap_or(false);
        let core = self.disposal.for_disposal(cutoff);
        let good = core.as_ref().zip(sender).is_some_and(|(core, sender)| {
            let expected = core.cast::<IUnknown>();
            native_result(owner, expected.is_ok(), "permissionIdentityChanged", true);
            let before =
                self.same(owner) && self.disposal_boundary(owner).is_some() && expected.is_ok();
            let actual = if before {
                sender.cast::<IUnknown>()
            } else {
                Err(rejection())
            };
            let good = before
                && self.same(owner)
                && expected
                    .as_ref()
                    .ok()
                    .zip(actual.as_ref().ok())
                    .is_some_and(|(a, b)| a.as_raw() == b.as_raw());
            native_result(owner, good, "permissionIdentityChanged", true);
            drop(actual);
            let after_actual = self.same(owner);
            drop(expected);
            good && after_actual && self.same(owner)
        });
        // Reentry during the final Rc/COM release cannot borrow a dead anchor.
        drop(core);
        good && self.same(owner) && self.disposal_boundary(owner).is_some()
    }
    fn boundary(&self, owner: &Rc<RefCell<Owner>>) -> Option<Boundary> {
        if !self.same(owner) {
            return None;
        }
        let o = owner.borrow();
        let s = o.instance.as_ref()?.permission.as_ref()?;
        Some(Boundary {
            identity: s.identity,
            ms: s.timing.ms(Instant::now())?,
        })
    }
    fn disposal_boundary(&self, owner: &Rc<RefCell<Owner>>) -> Option<Boundary> {
        let boundary = self.boundary(owner)?;
        let o = owner.borrow();
        let s = o.instance.as_ref()?.permission.as_ref()?;
        if s.timing.first_close.is_some() && !s.timing.close_before(Instant::now()) {
            return None;
        }
        Some(boundary)
    }
    fn positive(&self, owner: &Rc<RefCell<Owner>>) -> bool {
        if !self.same(owner) {
            return false;
        }
        let o = owner.borrow();
        let i = o.instance.as_ref().unwrap();
        let s = i.permission.as_ref().unwrap();
        s.core.as_ref().is_some_and(|core| {
            i.refusal_registry.permission_core_matches(core)
                && i.core
                    .as_ref()
                    .is_some_and(|current| current.as_raw() == core.as_raw())
        }) && s.timing.before(Instant::now())
            && s.facts.failure.is_none()
            && s.protocol.status().cutoff.is_none()
            && s.protocol.status().failure.is_none()
            && i.refusal_registry.complete()
            && !i.cleanup_blocked
            && !i.destroyed
            && !self.installation.load(Ordering::SeqCst)
            && !self.callbacks.affinity_failed.load(Ordering::SeqCst)
            && !self.inventory.affinity_cutoff.load(Ordering::SeqCst)
            && !self.inventory.clock.failed.load(Ordering::SeqCst)
    }
}
// Logical row metadata owns no receiver/core/handler, so it cannot form a COM
// cycle. One private seal is created only after the actual receiver's live QI
// and postDrop checks, then shared by that original emitter and its one handler.
struct ReceiverBinding {
    receiver: Receiver,
    canonical_attested: Cell<bool>,
    handler_created: Cell<bool>,
    add_started: Cell<bool>,
    returned: Cell<Option<bool>>,
    token: Cell<Option<i64>>,
    acked: Cell<bool>,
}
impl ReceiverBinding {
    fn new(receiver: Receiver) -> Self {
        Self {
            receiver,
            canonical_attested: Cell::new(false),
            handler_created: Cell::new(false),
            add_started: Cell::new(false),
            returned: Cell::new(None),
            token: Cell::new(None),
            acked: Cell::new(false),
        }
    }
    fn reserve_handler(&self) -> bool {
        !self.handler_created.replace(true)
    }
    fn matches(self: &Rc<Self>, expected: &Rc<Self>, receiver: Receiver) -> bool {
        Rc::ptr_eq(self, expected)
            && self.receiver == receiver
            && self.canonical_attested.get()
            && self.handler_created.get()
    }
    fn registered(&self) -> bool {
        self.canonical_attested.get()
            && self.handler_created.get()
            && self.add_started.get()
            && self.returned.get() == Some(true)
            && self.token.get().is_some()
            && self.acked.get()
    }
}
struct ReceiverRegistration {
    receiver: Rc<ICoreWebView2DevToolsProtocolEventReceiver>,
    binding: Rc<ReceiverBinding>,
    token: Option<i64>,
    attempted: bool,
}
pub(super) struct State {
    ctx: Context,
    identity: Identity,
    protocol: Protocol,
    pub(super) facts: Facts,
    timing: Timing,
    core: Option<Rc<ICoreWebView2>>,
    core22: Option<Rc<ICoreWebView2_22>>,
    environment: Option<Rc<ICoreWebView2Environment>>,
    receivers: Vec<ReceiverRegistration>,
    receiver_bindings: Vec<Rc<ReceiverBinding>>,
    completed: Option<i64>,
    resource: Option<i64>,
    filter: bool,
    completed_remove: bool,
    resource_remove: bool,
    filter_remove: bool,
    tickets: [Option<CommandTicket>; 4],
    completed_pending: [bool; 4],
    navigation_reserved: bool,
    source_checked: bool,
    first_close_published: bool,
    native_reserved: bool,
    native_frozen: bool,
    effect_frozen: bool,
    final_frozen: bool,
    prepared: Option<Prepared>,
    requests: u16,
    removals: [Option<RemovalTicket>; 24],
    lifecycle_order: LifecycleOrder,
    top_entries: u16,
    main_reserved: bool,
    begun: bool,
}
impl State {
    fn close_at(&mut self, at: Instant) {
        if self.ctx.inventory.clock.adopt(&mut self.timing).is_ok() {
            self.timing.close(at);
        } else {
            self.facts.unknown = true;
            self.facts.fail("permissionProtocolRejected");
        }
    }
    fn fail(&mut self, why: &'static str) {
        self.fail_at(why, Instant::now());
    }
    fn fail_at(&mut self, why: &'static str, at: Instant) {
        self.ctx.inventory.clock.record(at);
        self.facts.fail(why);
        if self.ctx.inventory.clock.adopt(&mut self.timing).is_err() {
            self.facts.unknown = true;
        }
        let boundary = Boundary {
            identity: self.identity,
            ms: self.timing.ms(at).unwrap_or(u64::MAX),
        };
        if self.protocol.status().failure.is_none() {
            self.protocol.native_adapter_failed(boundary);
        }
    }
    // End of one pure UI transaction. No native work/observer/reentrant Drop
    // may be inserted between state mutation and this publication.
    fn settle(&mut self, at: Instant) {
        let clock_valid = self.ctx.inventory.clock.adopt(&mut self.timing).is_ok();
        if !clock_valid {
            self.facts.unknown = true;
            self.facts.fail("permissionProtocolRejected");
            self.protocol.native_clock_failed();
        } else {
            if self.ctx.inventory.clock.failed.load(Ordering::SeqCst) {
                self.facts.fail(
                    if self.ctx.callbacks.affinity_failed.load(Ordering::SeqCst) {
                        "uiAffinityFailed"
                    } else if self.ctx.inventory.text.failed.load(Ordering::SeqCst) {
                        "permissionOverflow"
                    } else {
                        "permissionProtocolRejected"
                    },
                );
            }
            if self.facts.failure.is_some() || self.facts.overflow || self.facts.coverage.overflow {
                self.ctx.inventory.clock.record(at);
            }
            settle_failure(
                &mut self.protocol,
                &mut self.facts,
                &mut self.timing,
                self.identity,
                at,
            );
            if self.facts.failure.is_some() {
                self.ctx.inventory.clock.record(at);
            }
            if let Some(ms) = self
                .timing
                .first_close
                .and_then(|origin| self.timing.ms(origin))
            {
                self.protocol.adopt_native_close_origin(ms);
            }
        }
        let status = self.protocol.status();
        self.ctx
            .inventory
            .lifecycle_entries
            .publish(status.phase, status.cutoff.is_some());
    }
    fn sync(&mut self) {
        self.facts.observations = self.ctx.inventory.lifecycle_entries.snapshot();
        self.facts.overflow |=
            self.facts.coverage.overflow || self.facts.observations.counter_overflow;
        if self.ctx.inventory.text.failed.load(Ordering::SeqCst) {
            self.facts.unknown = true;
            self.fail("permissionOverflow");
        }
        if self.ctx.inventory.slot_failed.load(Ordering::SeqCst) {
            self.facts.unknown = true;
            self.fail("permissionCleanupFailed");
        }
        if self.facts.overflow {
            self.fail("permissionOverflow");
        }
        if self.protocol.status().failure.is_some() && self.facts.failure.is_none() {
            self.fail("permissionProtocolRejected");
        }
        self.settle(Instant::now());
        let status = self.protocol.status();
        self.facts.phase = phase(status.phase);
        self.facts.status.protocol_phase = Some(phase(status.phase));
        self.facts.status.protocol_failure = status.failure.map(problem);
        self.facts.status.observation_epoch = Some(status.epoch);
        self.facts.status.native_qualified = status.native_qualified;
        self.facts.status.effect_qualified = status.effect_qualified;
        self.facts.status.cutoff_reason = status.cutoff.map(|c| cutoff_reason(c.reason));
        self.facts.observations.blank_branch = status.blank_branch.map(|b| match b {
            semantic::BlankBranch::ZeroObserved => "ZeroObserved",
            semantic::BlankBranch::Observed => "Observed",
        });
        self.facts.commands.obligations = Some(u16::from(status.obligations));
        self.facts.commands.handlers = Some(u16::from(status.command_handlers));
        self.facts.commands.in_flight =
            Some(u16::from(self.tickets.iter().enumerate().any(|(i, t)| {
                t.is_some() && self.facts.commands.consumed[i] == 0
            })));
        self.facts.timing.first_close_at_ms =
            self.timing.first_close.and_then(|at| self.timing.ms(at));
        self.facts.timing.observation_cutoff_at_ms = status.cutoff.and_then(|c| c.ms);
        self.facts.ordering.cutoff_latched = Some(status.cutoff.is_some());
        let text = &self.ctx.inventory.text;
        self.facts.inventory.text_bytes = Some(text.live.load(Ordering::SeqCst).min(4097) as u16);
        self.facts.inventory.text_peak_bytes =
            Some(text.peak.load(Ordering::SeqCst).min(4097) as u16);
        self.facts.inventory.text_reserve_failures =
            text.failures.load(Ordering::SeqCst).min(2) as u16;
        self.facts.inventory.text_accounting_complete = Some(!text.failed.load(Ordering::SeqCst));
        self.facts.inventory.new_event_handlers = Some(
            self.ctx
                .inventory
                .new_handlers
                .load(Ordering::SeqCst)
                .min(13) as u16,
        );
        self.facts.inventory.command_handlers = Some(
            self.ctx
                .inventory
                .command_handlers
                .load(Ordering::SeqCst)
                .min(5) as u16,
        );
    }
}
fn phase(p: Phase) -> &'static str {
    match p {
        Phase::Registering => "Registering",
        Phase::Enabling => "Enabling",
        Phase::Navigating => "Navigating",
        Phase::Armed => "Armed",
        Phase::Issued => "Issued",
        Phase::Closing => "Closing",
        Phase::Cutoff => "Cutoff",
        Phase::Released => "Released",
    }
}
fn problem(p: Problem) -> &'static str {
    match p {
        Problem::NotReady => "NotReady",
        Problem::Shape => "Shape",
        Problem::Identity => "Identity",
        Problem::Affinity => "Affinity",
        Problem::Clock => "Clock",
        Problem::Deadline => "Deadline",
        Problem::CloseDeadline => "CloseDeadline",
        Problem::Coverage => "Coverage",
        Problem::Phase => "Phase",
        Problem::Replay => "Replay",
        Problem::Root => "Root",
        Problem::Context => "Context",
        Problem::Lifecycle => "Lifecycle",
        Problem::Budget => "Budget",
        Problem::Native => "Native",
        Problem::Result => "Result",
        Problem::Released => "Released",
        Problem::Overflow => "Overflow",
    }
}
fn cutoff_reason(r: CutoffReason) -> &'static str {
    match r {
        CutoffReason::RequiredRemovalAttempt => "RequiredRemovalAttempt",
        CutoffReason::CoverageLost => "CoverageLost",
        CutoffReason::ControlsDisposal => "ControlsDisposal",
        CutoffReason::StopEntry => "StopEntry",
        CutoffReason::ControllerCloseEntry => "ControllerCloseEntry",
        CutoffReason::AffinityLost => "AffinityLost",
    }
}

struct OwnedText {
    value: String,
    _charge: TextCharge,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CaptureError {
    Null,
    Unterminated,
    Utf16,
    ByteCap,
    Reservation,
}
unsafe fn capture_text(
    raw: PCWSTR,
    units: usize,
    bytes: usize,
    ledger: &Arc<TextLedger>,
) -> Result<OwnedText, CaptureError> {
    let captured = (|| {
        if raw.is_null() {
            return Err(CaptureError::Null);
        }
        let mut length = None;
        // units includes the required NUL; no inclusive upper-bound scan.
        for n in 0..units {
            if unsafe { *raw.0.add(n) } == 0 {
                length = Some(n);
                break;
            }
        }
        let length = length.ok_or(CaptureError::Unterminated)?;
        let slice = unsafe { std::slice::from_raw_parts(raw.0, length) };
        let mut utf8 = 0usize;
        for ch in char::decode_utf16(slice.iter().copied()) {
            let ch = ch.map_err(|_| CaptureError::Utf16)?;
            utf8 = utf8
                .checked_add(ch.len_utf8())
                .ok_or(CaptureError::ByteCap)?;
            if utf8 > bytes {
                return Err(CaptureError::ByteCap);
            }
        }
        let charge = TextCharge::reserve(ledger, utf8).ok_or(CaptureError::Reservation)?;
        let mut value = String::with_capacity(utf8);
        for ch in char::decode_utf16(slice.iter().copied()) {
            value.push(ch.map_err(|_| CaptureError::Utf16)?);
        }
        Ok(OwnedText {
            value,
            _charge: charge,
        })
    })();
    if captured.is_err() {
        // A failed bounded capture is unavailable accounting, independently of
        // whether reserve was actually attempted. Only reserve increments failures.
        if let Some(clock) = ledger.clock.get() {
            clock.record(Instant::now());
        }
        ledger.failed.store(true, Ordering::SeqCst);
    }
    captured
}
unsafe fn copy_text(
    raw: PCWSTR,
    units: usize,
    bytes: usize,
    ledger: &Arc<TextLedger>,
) -> Option<OwnedText> {
    unsafe { capture_text(raw, units, bytes, ledger) }.ok()
}
fn allocated(
    ctx: &Context,
    owner: &Rc<RefCell<Owner>>,
    units: usize,
    call: impl FnOnce(*mut PWSTR) -> windows::core::Result<()>,
) -> Option<OwnedText> {
    let Some(native) = TextCharge::reserve(&ctx.inventory.text, 2048) else {
        reject(owner, "permissionOverflow", true);
        return None;
    };
    let mut raw = PWSTR::null();
    let result = call(&mut raw);
    let ok = native_result(
        owner,
        result.is_ok(),
        "permissionNativeMetadataFailed",
        true,
    ) && native_result(owner, ctx.same(owner), "permissionIdentityChanged", true);
    drop(result);
    let copied = if ok && ctx.same(owner) {
        unsafe { copy_text(PCWSTR(raw.0), units, 2048, &ctx.inventory.text) }
    } else {
        None
    };
    if (ok && copied.is_none()) || (!ok && !raw.is_null()) {
        ctx.inventory.text.failed.store(true, Ordering::SeqCst);
        reject(owner, "permissionOverflow", true);
    }
    // Native allocations are released on both success and failed HRESULT.
    if !raw.is_null() {
        unsafe { windows::Win32::System::Com::CoTaskMemFree(Some(raw.0.cast())) };
    }
    drop(native);
    if !ctx.same(owner) {
        drop(copied);
        return None;
    }
    copied
}
fn source(ctx: &Context, owner: &Rc<RefCell<Owner>>) -> Option<bool> {
    let core = owner
        .borrow()
        .instance
        .as_ref()
        .and_then(|i| i.permission.as_ref())
        .and_then(|s| s.core.as_ref())
        .map(Rc::clone);
    let text = core
        .as_ref()
        .and_then(|c| allocated(ctx, owner, 2048, |raw| unsafe { c.Source(raw) }));
    let good = text.as_ref().map(|text| text.value == semantic::FIXED_URL);
    native_result(
        owner,
        good == Some(true),
        "permissionIdentityChanged",
        good.is_none(),
    );
    drop(text);
    let after_text = ctx.same(owner);
    drop(core);
    if after_text && ctx.same(owner) {
        good
    } else {
        None
    }
}
fn carry_close(i: &mut Instance) {
    let Some(s) = i.permission.as_mut() else {
        return;
    };
    s.settle(Instant::now());
    if s.facts.failure.is_some() {
        i.failed = true;
        i.closing = true;
    }
    if let Some(at) = s.timing.first_close {
        latch_close(
            &mut s.timing,
            &mut i.permission_history.first_close,
            &mut i.failed,
            &mut i.closing,
            at,
        );
        if i.permission_history.failure.is_none() {
            i.permission_history.failure = s.facts.failure;
        }
    }
}
fn commit_close(i: &mut Instance) -> Option<Instant> {
    let s = i.permission.as_mut()?;
    if s.facts.failure.is_some() {
        i.failed = true;
        i.closing = true;
    }
    let at = s.timing.first_close?;
    latch_close(
        &mut s.timing,
        &mut i.permission_history.first_close,
        &mut i.failed,
        &mut i.closing,
        at,
    );
    if i.permission_history.failure.is_none() {
        i.permission_history.failure = s.facts.failure;
    }
    if !s.first_close_published {
        s.first_close_published = true;
        s.timing.first_close
    } else {
        None
    }
}
fn with_state<T>(owner: &Rc<RefCell<Owner>>, f: impl FnOnce(&mut State) -> T) -> Option<T> {
    let (result, notification) = {
        let mut o = owner.borrow_mut();
        let i = o.instance.as_mut()?;
        let s = i.permission.as_mut()?;
        let result = f(s);
        s.settle(Instant::now());
        let at = commit_close(i);
        (
            result,
            at.and_then(|at| i.observer.clone().map(|observer| (observer, at))),
        )
    };
    if let Some((observer, at)) = notification {
        observer(NativeSmokeObservation::PermissionClosing { at });
    }
    Some(result)
}
fn reject_at(owner: &Rc<RefCell<Owner>>, why: &'static str, unknown: bool, at: Instant) {
    let notification = {
        let mut o = owner.borrow_mut();
        let Some(i) = o.instance.as_mut() else { return };
        if !i.permission_selected.load(Ordering::SeqCst) {
            return;
        }
        i.permission_history.inventory.clock.record(at);
        i.permission_history.unknown |= unknown;
        if i.permission_history.failure.is_none() {
            i.permission_history.failure = Some(why);
        }
        if let Some(s) = i.permission.as_mut() {
            s.fail_at(why, at);
            s.facts.unknown |= unknown;
            s.settle(at);
            commit_close(i).and_then(|at| i.observer.clone().map(|observer| (observer, at)))
        } else {
            let first = i.permission_history.first_close.is_none();
            if let Ok(Some(original)) = i.permission_history.inventory.clock.origin() {
                let earliest = i
                    .permission_history
                    .first_close
                    .map_or(original, |old| old.min(original));
                i.permission_history.first_close = Some(earliest);
            }
            i.failed = true;
            i.closing = true;
            if first {
                i.permission_history
                    .first_close
                    .and_then(|original| i.observer.clone().map(|observer| (observer, original)))
            } else {
                None
            }
        }
    };
    if let Some((observer, at)) = notification {
        observer(NativeSmokeObservation::PermissionClosing { at });
    }
}
fn reject(owner: &Rc<RefCell<Owner>>, why: &'static str, unknown: bool) {
    reject_at(owner, why, unknown, Instant::now());
}
// Called immediately after the actual native return/value is known, before the
// returned Error/interface/text or another native call can be dropped/entered.
fn native_result(owner: &Rc<RefCell<Owner>>, good: bool, why: &'static str, unknown: bool) -> bool {
    if !good {
        reject_at(owner, why, unknown, Instant::now());
    }
    good
}
pub(super) fn selected(owner: &Rc<RefCell<Owner>>) -> bool {
    owner
        .borrow()
        .instance
        .as_ref()
        .is_some_and(|i| i.permission_selected.load(Ordering::SeqCst))
}
fn context(owner: &Rc<RefCell<Owner>>) -> Option<Context> {
    owner
        .borrow()
        .instance
        .as_ref()?
        .permission
        .as_ref()
        .map(|s| s.ctx.clone())
}
pub(super) fn cutoff(owner: &Rc<RefCell<Owner>>, reason: CutoffReason) {
    let Some(ctx) = context(owner) else { return };
    let now = Instant::now();
    let Some(boundary) = ctx.boundary(owner) else {
        return;
    };
    with_state(owner, |s| {
        let before = s.protocol.status().cutoff.is_none();
        s.protocol.cutoff(reason, boundary);
        s.close_at(now);
        if before && !s.effect_frozen {
            s.fail(if s.native_reserved {
                "permissionEffectMissing"
            } else {
                "permissionNativeTargetMissing"
            });
        }
        if reason == CutoffReason::StopEntry {
            s.facts.timing.stop_entered_at_ms = Some(boundary.ms);
        }
        if reason == CutoffReason::ControllerCloseEntry {
            s.facts.timing.controller_close_entered_at_ms = Some(boundary.ms);
            s.facts.ordering.physical_close_observed = Some(true);
        }
        if before && reason == CutoffReason::RequiredRemovalAttempt {
            s.facts.ordering.cutoff_before_any_required_removal = Some(true);
        }
        s.sync();
    });
}
fn note_close(timing: &mut Timing, closing: bool, now: Instant) -> bool {
    if !closing {
        return false;
    }
    timing.close(now);
    true
}
pub(super) fn external_close(i: &mut Instance) {
    if let Some(s) = i.permission.as_mut() {
        let now = Instant::now();
        if !i.closing {
            return;
        }
        if s.ctx.inventory.clock.adopt(&mut s.timing).is_ok() {
            note_close(&mut s.timing, true, now);
        } else {
            s.facts.unknown = true;
            s.facts.fail("permissionProtocolRejected");
        }
        if i.callbacks.affinity_failed.load(Ordering::SeqCst) {
            s.protocol.affinity_lost();
            s.fail("uiAffinityFailed");
        } else if i.destroyed || i.controls_disposed {
            let boundary = Boundary {
                identity: s.identity,
                ms: s.timing.ms(now).unwrap_or(u64::MAX),
            };
            s.protocol.cutoff(CutoffReason::ControlsDisposal, boundary);
        }
        if !s.native_reserved {
            s.fail("permissionNativeTargetMissing");
        }
    }
    carry_close(i);
}
fn check_native(
    owner: &Rc<RefCell<Owner>>,
    ticket: Option<NativeTicket>,
    check: NativeCheck,
    good: bool,
) {
    let at = Instant::now();
    with_state(owner, |s| {
        let boundary = Boundary {
            identity: s.identity,
            ms: s.timing.ms(at).unwrap_or(u64::MAX),
        };
        let good = good && s.timing.before(at);
        if !good {
            s.fail_at("permissionNativeMetadataFailed", at);
        }
        if let Some(ticket) = ticket {
            if s.protocol
                .native_checked(ticket, check, good, boundary)
                .is_err()
            {
                s.fail_at("permissionProtocolRejected", at);
            }
        } else {
            s.fail_at("permissionProtocolRejected", at);
        }
    });
}
struct NativeCandidate {
    ctx: Context,
    ticket: Option<NativeTicket>,
    first: bool,
    timely: bool,
    entry_epoch: u64,
}
fn native_capture(
    ctx: &Context,
    owner: &Rc<RefCell<Owner>>,
    sender: Option<&ICoreWebView2>,
    args: Option<&ICoreWebView2PermissionRequestedEventArgs>,
    source_binding: bool,
    entry: Instant,
) -> NativeCandidate {
    let first = with_state(owner, |s| {
        counted(
            &mut s.facts.counts.permission_entries,
            1,
            &mut s.facts.overflow,
            &mut s.facts.failure,
        );
        take_once(&mut s.native_reserved)
    })
    .unwrap_or(false);
    let entry_epoch = with_state(owner, |s| s.protocol.status().epoch).unwrap_or(u64::MAX);
    let boundary = ctx.boundary(owner);
    let ticket = if first {
        boundary.and_then(|b| with_state(owner, |s| s.protocol.native_entered(b).ok()).flatten())
    } else {
        None
    };
    with_state(owner, |s| {
        if first {
            s.facts.timing.native_entered_at_ms = s.timing.ms(entry);
            s.facts.native.entered_before_observation_cutoff =
                Some(s.protocol.status().cutoff.is_none());
        }
    });
    let sender_good = ctx.sender(owner, sender);
    let binding = source_binding && ctx.same(owner);
    let timely = with_state(owner, |s| s.timing.before(entry)).unwrap_or(false);
    with_state(owner, |s| {
        if first {
            s.facts.native.sender_matched = Some(sender_good);
            s.facts.native.request_installation_matched = Some(binding);
        }
    });
    if !sender_good || !binding {
        reject(owner, "permissionIdentityChanged", true);
    }
    let Some(args) = args else {
        reject(owner, "permissionNativeMetadataFailed", true);
        return NativeCandidate {
            ctx: ctx.clone(),
            ticket,
            first,
            timely,
            entry_epoch,
        };
    };
    // Every usable safe-UI delivery reaches the sole DENY and readback. The
    // first selected record is immutable; later deliveries only count/fail.
    let captured = RefCell::new(Native::default());
    let (_, denied, after) = restrictive_sequence(
        || {
            let mut kind = COREWEBVIEW2_PERMISSION_KIND(-1);
            with_state(owner, |s| {
                counted(
                    &mut s.facts.counts.kind_reads,
                    1,
                    &mut s.facts.overflow,
                    &mut s.facts.failure,
                );
            });
            let result = unsafe { args.PermissionKind(&mut kind) };
            let ok = result.is_ok();
            let kind = ok.then_some(kind == COREWEBVIEW2_PERMISSION_KIND_NOTIFICATIONS);
            captured.borrow_mut().kind_notifications = kind;
            check_native(
                owner,
                ticket,
                NativeCheck::KindNotifications,
                kind == Some(true) && ctx.positive(owner),
            );
            drop(result);
            let mut gesture = BOOL(2);
            with_state(owner, |s| {
                counted(
                    &mut s.facts.counts.gesture_reads,
                    1,
                    &mut s.facts.overflow,
                    &mut s.facts.failure,
                );
            });
            let result = unsafe { args.IsUserInitiated(&mut gesture) };
            let ok = result.is_ok();
            let non_user = ok.then_some(gesture.0 == 0);
            captured.borrow_mut().non_user = non_user;
            check_native(
                owner,
                ticket,
                NativeCheck::NonUser,
                non_user == Some(true) && ctx.positive(owner),
            );
            drop(result);
            with_state(owner, |s| {
                counted(
                    &mut s.facts.counts.origin_reads,
                    1,
                    &mut s.facts.overflow,
                    &mut s.facts.failure,
                );
            });
            let origin = allocated(ctx, owner, 256, |raw| unsafe { args.Uri(raw) });
            let matched = origin
                .as_ref()
                .map(|s| semantic::permission_origin_matches(&s.value));
            captured.borrow_mut().origin_matched = matched;
            check_native(
                owner,
                ticket,
                NativeCheck::Origin,
                matched == Some(true) && ctx.positive(owner),
            );
            drop(origin);
            let mut state = COREWEBVIEW2_PERMISSION_STATE(-1);
            with_state(owner, |s| {
                counted(
                    &mut s.facts.counts.state_before_reads,
                    1,
                    &mut s.facts.overflow,
                    &mut s.facts.failure,
                );
            });
            let result = unsafe { args.State(&mut state) };
            let ok = result.is_ok();
            let before = ok.then_some(state == COREWEBVIEW2_PERMISSION_STATE_DEFAULT);
            captured.borrow_mut().state_before_default = before;
            check_native(
                owner,
                ticket,
                NativeCheck::BeforeDefault,
                before == Some(true) && ctx.positive(owner),
            );
            drop(result);
        },
        || {
            check_native(owner, ticket, NativeCheck::DenyAttempted, true);
            // Immediate refusal boundary, before the native setter and any
            // observer/Session wait. Normal refusal does not remove coverage.
            let at = Instant::now();
            with_state(owner, |s| {
                let first_close = s.timing.first_close.is_none();
                s.close_at(at);
                counted(
                    &mut s.facts.counts.deny_attempts,
                    1,
                    &mut s.facts.overflow,
                    &mut s.facts.failure,
                );
                let boundary = Boundary {
                    identity: s.identity,
                    ms: s.timing.ms(at).unwrap_or(u64::MAX),
                };
                if let Some(ticket) = ticket {
                    let _ = s.protocol.native_refused(ticket, boundary);
                }
                if first {
                    s.facts.ordering.permanent_revocation_at_refusal = Some(true);
                    s.facts.ordering.first_close_at_refusal = Some(first_close);
                    s.facts.ordering.no_action_reauthorized = Some(true);
                    s.facts.ordering.no_wait_for_effect = Some(true);
                }
            });
            let result = unsafe { args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY) };
            let ok = native_result(owner, result.is_ok(), "permissionNativeActionFailed", true);
            drop(result);
            with_state(owner, |s| {
                counted(
                    &mut s.facts.counts.deny_returns,
                    1,
                    &mut s.facts.overflow,
                    &mut s.facts.failure,
                );
                if ok {
                    counted(
                        &mut s.facts.counts.deny_successes,
                        1,
                        &mut s.facts.overflow,
                        &mut s.facts.failure,
                    );
                }
            });
            check_native(
                owner,
                ticket,
                NativeCheck::DenyReturned,
                ctx.positive(owner),
            );
            check_native(
                owner,
                ticket,
                NativeCheck::DenySucceeded,
                ok && ctx.positive(owner),
            );
            ok
        },
        || {
            let mut state = COREWEBVIEW2_PERMISSION_STATE(-1);
            with_state(owner, |s| {
                counted(
                    &mut s.facts.counts.state_after_reads,
                    1,
                    &mut s.facts.overflow,
                    &mut s.facts.failure,
                );
            });
            let result = unsafe { args.State(&mut state) };
            let ok = result.is_ok();
            let after = ok.then_some(state == COREWEBVIEW2_PERMISSION_STATE_DENY);
            check_native(
                owner,
                ticket,
                NativeCheck::AfterDeny,
                after == Some(true) && ctx.positive(owner),
            );
            drop(result);
            after
        },
    );
    let facts = captured.into_inner();
    with_state(owner, |s| {
        if first {
            s.facts.native.kind_notifications = facts.kind_notifications;
            s.facts.native.non_user = facts.non_user;
            s.facts.native.origin_matched = facts.origin_matched;
            s.facts.native.state_before_default = facts.state_before_default;
            s.facts.native.deny_succeeded = Some(denied);
            s.facts.native.state_after_deny = after;
            s.facts.timing.native_captured_at_ms = s.timing.ms(Instant::now());
            s.facts.native.captured_before_observation_cutoff =
                Some(s.protocol.status().cutoff.is_none());
            if facts.kind_notifications != Some(true)
                || facts.non_user != Some(true)
                || facts.origin_matched != Some(true)
                || facts.state_before_default != Some(true)
            {
                s.fail("permissionNativeMetadataFailed");
            } else if !denied {
                s.fail("permissionNativeActionFailed");
            } else if after != Some(true) {
                s.fail("permissionNativeStateMismatch");
            }
        } else {
            s.fail("permissionNativeMetadataFailed");
        }
    });
    NativeCandidate {
        ctx: ctx.clone(),
        ticket,
        first,
        timely: timely && with_state(owner, |s| s.timing.before(Instant::now())).unwrap_or(false),
        entry_epoch,
    }
}
fn finish_native(candidate: NativeCandidate) {
    let ctx = &candidate.ctx;
    let Some(owner) = ctx.owner.upgrade() else {
        return;
    };
    let current = ctx.same(&owner);
    let positive = ctx.positive(&owner);
    let boundary = ctx.boundary(&owner);
    with_state(&owner, |s| {
        if candidate.first {
            if let (Some(ticket), Some(boundary)) = (candidate.ticket, boundary) {
                let _ = s.protocol.native_qualified(ticket, boundary);
            }
            s.native_frozen = true;
            counted(
                &mut s.facts.counts.action_receipts,
                1,
                &mut s.facts.overflow,
                &mut s.facts.failure,
            );
            s.facts.native.post_drop_matched =
                Some(current && s.protocol.status().epoch == candidate.entry_epoch);
            s.facts.native.original_timely =
                Some(candidate.timely && s.timing.before(Instant::now()));
            s.facts.native.qualified_before_observation_cutoff =
                Some(s.protocol.status().cutoff.is_none());
            s.facts.timing.native_qualified_at_ms = s.timing.ms(Instant::now());
            s.sync();
        }
    });
    if !positive {
        reject(&owner, "permissionIdentityChanged", !current);
    }
    publish(&owner);
}

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
fn new_id() -> Option<u64> {
    NEXT_ID
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_add(1))
        .ok()
}
fn key(slot: usize) -> CoverageKey {
    match slot {
        0..=9 => CoverageKey::S2Hook(slot as u8),
        10..=19 => CoverageKey::Receiver(Receiver::ALL[slot - 10]),
        20 => CoverageKey::NavigationCompleted,
        21 => CoverageKey::ResourceRequested,
        22 => CoverageKey::AllSourceFilter,
        _ => CoverageKey::CoreMessage,
    }
}
fn registration_ack(owner: &Rc<RefCell<Owner>>, slot: usize, token: NativeToken, ok: bool) {
    let Some(ctx) = context(owner) else { return };
    let current = ctx.same(owner) && ctx.positive(owner);
    let boundary = ctx.boundary(owner);
    with_state(owner, |s| {
        s.facts.coverage.add_qualified(slot, ok && current);
        if let Some(boundary) = boundary {
            if ok && current {
                if s.protocol
                    .registration_added(key(slot), token, boundary)
                    .is_err()
                {
                    s.fail("permissionSetupRejected");
                }
            } else {
                s.protocol.cutoff(CutoffReason::CoverageLost, boundary);
                s.fail("permissionSetupRejected");
            }
        } else {
            s.fail("permissionIdentityChanged");
        }
    });
}
#[derive(Default)]
struct LifecycleOrder {
    next: u64,
    active: u16,
    invalid: bool,
}
impl LifecycleOrder {
    fn enter(&mut self) -> Option<u64> {
        self.next = match self.next.checked_add(1) {
            Some(next) => next,
            None => {
                self.invalid = true;
                return None;
            }
        };
        if self.active != 0 {
            self.invalid = true;
        }
        self.active = match self.active.checked_add(1) {
            Some(active) => active,
            None => {
                self.invalid = true;
                return None;
            }
        };
        Some(self.next)
    }
    fn complete(&mut self, stamp: u64) -> bool {
        let good = !self.invalid && self.active == 1 && stamp == self.next;
        self.active = self.active.saturating_sub(1);
        if !good {
            self.invalid = true;
        }
        good
    }
}
fn forbidden_lifecycle_entry(phase: Phase, cutoff: bool) -> bool {
    !cutoff && !matches!(phase, Phase::Enabling | Phase::Navigating)
}
struct LifecycleCandidate {
    receiver: Receiver,
    event: Option<Lifecycle>,
    stamp: Option<u64>,
    epoch: u64,
    cutoff: Option<semantic::Cutoff>,
    phase: Phase,
    rejected_at_entry: bool,
}
fn delivery(
    ctx: &Context,
    binding: &Rc<ReceiverBinding>,
    receiver: Receiver,
    sender: Option<&ICoreWebView2>,
    args: Option<&ICoreWebView2DevToolsProtocolEventReceivedEventArgs>,
) -> Option<LifecycleCandidate> {
    if !ctx.ui() {
        return None;
    }
    let owner = ctx.owner.upgrade()?;
    let (stamp, epoch, cutoff, phase, nested, rejected_at_entry) = with_state(&owner, |s| {
        let status = s.protocol.status();
        let stamp = s.lifecycle_order.enter();
        let rejected_at_entry = forbidden_lifecycle_entry(status.phase, status.cutoff.is_some());
        if rejected_at_entry {
            let boundary = Boundary {
                identity: s.identity,
                ms: s.timing.ms(Instant::now()).unwrap_or(u64::MAX),
            };
            let _ = s
                .protocol
                .lifecycle(Lifecycle::Unsupported(receiver), boundary);
            s.facts.unknown = true;
            s.fail("permissionProtocolRejected");
        }
        (
            stamp,
            status.epoch,
            status.cutoff,
            status.phase,
            s.lifecycle_order.invalid,
            rejected_at_entry,
        )
    })?;
    if nested {
        reject(&owner, "permissionProtocolRejected", true);
    }
    let bound = with_state(&owner, |s| {
        s.receiver_bindings
            .get(receiver as usize)
            .is_some_and(|expected| {
                binding.matches(expected, receiver) && binding.add_started.get()
            })
    })
    .unwrap_or(false);
    let identity = !nested
        && bound
        && if cutoff.is_some() {
            ctx.disposal_sender(&owner, sender)
        } else {
            ctx.sender(&owner, sender)
        };
    let event = if identity {
        args.and_then(|args| {
            let cast = args.cast::<ICoreWebView2DevToolsProtocolEventReceivedEventArgs2>();
            let args2 = match cast {
                Ok(args) => args,
                Err(error) => {
                    reject(&owner, "permissionProtocolRejected", true);
                    drop(error);
                    return None;
                }
            };
            let session = allocated(ctx, &owner, 2048, |raw| unsafe { args2.SessionId(raw) });
            let valid = session.as_ref().is_some_and(|s| s.value.is_empty());
            native_result(
                &owner,
                valid,
                "permissionProtocolRejected",
                session.is_none(),
            );
            drop(session);
            drop(args2);
            if !valid || !ctx.same(&owner) {
                return None;
            }
            let json = allocated(ctx, &owner, 2048, |raw| unsafe {
                args.ParameterObjectAsJson(raw)
            });
            let decoded = json
                .as_ref()
                .and_then(|json| wire::decode_lifecycle(receiver, json.value.as_bytes()).ok());
            native_result(
                &owner,
                decoded.is_some(),
                "permissionProtocolRejected",
                json.is_none(),
            );
            drop(json);
            if ctx.same(&owner) { decoded } else { None }
        })
    } else {
        None
    };
    Some(LifecycleCandidate {
        receiver,
        event,
        stamp,
        epoch,
        cutoff,
        phase,
        rejected_at_entry,
    })
}
fn finish_delivery(ctx: &Context, candidate: LifecycleCandidate) {
    if !ctx.ui() {
        return;
    }
    let Some(owner) = ctx.owner.upgrade() else {
        return;
    };
    let current = ctx.same(&owner);
    let now = Instant::now();
    let boundary = ctx.boundary(&owner);
    with_state(&owner, |s| {
        let ordered = candidate
            .stamp
            .is_some_and(|stamp| s.lifecycle_order.complete(stamp));
        let status = s.protocol.status();
        let unchanged = current
            && ordered
            && status.epoch == candidate.epoch
            && status.cutoff == candidate.cutoff
            && status.phase == candidate.phase;
        let eligible = unchanged
            && if candidate.cutoff.is_some() {
                s.timing.close_before(now)
            } else {
                s.timing.before(now) && s.facts.failure.is_none()
            };
        let event = if eligible {
            candidate
                .event
                .unwrap_or(Lifecycle::Unsupported(candidate.receiver))
        } else {
            Lifecycle::Unsupported(candidate.receiver)
        };
        if !candidate.rejected_at_entry {
            if let Some(boundary) = boundary {
                let _ = s.protocol.lifecycle(event, boundary);
            }
        }
        if !eligible || candidate.event.is_none() {
            s.facts.unknown = true;
            s.fail("permissionProtocolRejected");
        }
        if document_credit(
            candidate.cutoff.is_some(),
            matches!(
                candidate.event,
                Some(Lifecycle::ContextCreated(semantic::Document::Selected, _))
            ),
            s.protocol.selected_unique_context().is_some(),
            eligible,
        ) {
            s.facts.inventory.document_record = Some(1);
        }
        s.sync();
    });
    publish(&owner);
}
fn document_credit(
    cutoff: bool,
    selected_created: bool,
    protocol_selected: bool,
    eligible: bool,
) -> bool {
    !cutoff && selected_created && protocol_selected && eligible
}
pub(super) struct Intent {
    request: Code0WindowRequest,
    started: Instant,
    prepared: bool,
    begun: bool,
    failed: bool,
}
impl Intent {
    fn new(request: Code0WindowRequest, started: Instant) -> Self {
        Self {
            request,
            started,
            prepared: false,
            begun: false,
            failed: false,
        }
    }
    fn matches(&self, request: &Code0WindowRequest, started: Instant) -> bool {
        &self.request == request && self.started == started
    }
    fn reserve_prepare(&mut self, request: &Code0WindowRequest, started: Instant) -> bool {
        if self.failed || self.prepared || self.begun || !self.matches(request, started) {
            self.failed = true;
            false
        } else {
            self.prepared = true;
            true
        }
    }
    fn reserve_begin(&mut self, request: &Code0WindowRequest, started: Instant) -> bool {
        if self.failed || !self.prepared || self.begun || !self.matches(request, started) {
            self.failed = true;
            false
        } else {
            self.begun = true;
            true
        }
    }
}
pub(super) fn select_intent(
    owner: &Rc<RefCell<Owner>>,
    request: &Code0WindowRequest,
    started: Instant,
) -> Result<(), Code0SessionFailure> {
    if !on_ui(owner) {
        return Err(Code0SessionFailure::PolicyRejected);
    }
    let mut o = owner.borrow_mut();
    if o.unmatched
        || o.instance.is_some()
        || o.permission_intent.is_some()
        || !Timing::new(started).before(Instant::now())
    {
        return Err(Code0SessionFailure::PolicyRejected);
    }
    o.permission_intent = Some(Intent::new(request.clone(), started));
    Ok(())
}
pub(super) fn intent_for(owner: &Owner, request: &Code0WindowRequest) -> bool {
    owner
        .permission_intent
        .as_ref()
        .is_some_and(|intent| &intent.request == request && !intent.failed)
}
pub(super) fn install_allowed(i: &Instance) -> bool {
    i.permission_selected.load(Ordering::SeqCst)
        && i.permission.as_ref().is_some_and(|s| {
            !s.begun && s.protocol.status().phase == Phase::Registering && s.facts.failure.is_none()
        })
}
pub(super) fn prepare(
    owner: &Rc<RefCell<Owner>>,
    request: &Code0WindowRequest,
    started: Instant,
) -> Result<(), Code0SessionFailure> {
    if !on_ui(owner) {
        return Err(Code0SessionFailure::PolicyRejected);
    }
    let (callbacks, ui, installation, inventory, history, settings, original, environment) = {
        let mut o = owner.borrow_mut();
        let reserved = o
            .permission_intent
            .as_mut()
            .is_some_and(|intent| intent.reserve_prepare(request, started));
        if !reserved || o.unmatched {
            return Err(Code0SessionFailure::PolicyRejected);
        }
        let ui = o.ui_thread;
        let i = o
            .instance
            .as_mut()
            .filter(|i| &i.request == request)
            .ok_or(Code0SessionFailure::PolicyRejected)?;
        let same_clock = i.permission_history.inventory.clock.matches(started)
            && i.callbacks
                .permission_clock
                .get()
                .is_some_and(|clock| Arc::ptr_eq(clock, &i.permission_history.inventory.clock));
        if !same_clock {
            i.permission_history
                .inventory
                .clock
                .invalidate(Instant::now());
            if i.permission_history.failure.is_none() {
                i.permission_history.failure = Some("permissionIdentityChanged");
            }
            i.permission_history.unknown = true;
            i.failed = true;
            i.closing = true;
            carry_close(i);
            return Err(Code0SessionFailure::PolicyRejected);
        }
        if !i.permission_selected.load(Ordering::SeqCst)
            || i.permission.is_some()
            || i.policy.is_some()
            || i.policy_mailbox.is_some()
            || i.http401.is_some()
            || i.chooser.is_some()
            || i.capability.is_some()
            || i.inline_loaded
            || i.failed
            || i.closing
            || i.released
            || i.destroyed
            || i.refusal_registry.http_generation() != 0
            || i.core.is_none()
            || i.environment.is_none()
        {
            return Err(Code0SessionFailure::PolicyRejected);
        }
        (
            Arc::clone(&i.callbacks),
            ui,
            Arc::clone(&i.refusal_registry.unknown),
            Arc::clone(&i.permission_history.inventory),
            i.permission_history.facts.clone(),
            i.permission_history.settings,
            i.core.take().unwrap(),
            i.environment.take().unwrap(),
        )
    };
    let active = Active::enter(&callbacks);
    let core = Rc::new(original.clone());
    let cached_environment = Rc::new(environment.clone());
    let replaced = {
        let mut o = owner.borrow_mut();
        let i = o.instance.as_mut().unwrap();
        (i.core.replace(original), i.environment.replace(environment))
    };
    let unexpected = replaced.0.is_some() || replaced.1.is_some();
    native_result(owner, !unexpected, "permissionIdentityChanged", true);
    drop(replaced);
    if unexpected
        || owner.borrow().instance.as_ref().is_none_or(|i| {
            i.failed
                || i.closing
                || i.released
                || i.callbacks.affinity_failed.load(Ordering::SeqCst)
        })
    {
        reject(owner, "permissionIdentityChanged", true);
        return Err(Code0SessionFailure::PolicyRejected);
    }
    let ctx = Context {
        owner: Rc::downgrade(owner),
        request: request.clone(),
        installation,
        callbacks: Arc::clone(&callbacks),
        inventory,
        ui,
        disposal: Rc::new(DisposalAnchor::new(&core)),
    };
    let identity = Identity {
        request: new_id().ok_or_else(|| {
            reject(owner, "permissionOverflow", true);
            Code0SessionFailure::PolicyRejected
        })?,
        generation: new_id().ok_or_else(|| {
            reject(owner, "permissionOverflow", true);
            Code0SessionFailure::PolicyRejected
        })?,
        installation: new_id().ok_or_else(|| {
            reject(owner, "permissionOverflow", true);
            Code0SessionFailure::PolicyRejected
        })?,
        core: new_id().ok_or_else(|| {
            reject(owner, "permissionOverflow", true);
            Code0SessionFailure::PolicyRejected
        })?,
    };
    let protocol = Protocol::new(identity, 0).map_err(|_| Code0SessionFailure::PolicyRejected)?;
    let mut facts = Facts {
        coverage: history,
        ..Facts::default()
    };
    facts.setup.settings_ack = settings;
    facts.setup.original_deadline_bound = Some(Timing::new(started).before(Instant::now()));
    {
        let mut o = owner.borrow_mut();
        let i = o.instance.as_mut().unwrap();
        i.permission = Some(State {
            ctx: ctx.clone(),
            identity,
            protocol,
            facts,
            timing: Timing::new(started),
            core: Some(Rc::clone(&core)),
            core22: None,
            environment: None,
            receivers: Vec::with_capacity(10),
            receiver_bindings: Vec::with_capacity(10),
            completed: None,
            resource: None,
            filter: false,
            completed_remove: false,
            resource_remove: false,
            filter_remove: false,
            tickets: [None; 4],
            completed_pending: [false; 4],
            navigation_reserved: false,
            source_checked: false,
            first_close_published: false,
            native_reserved: false,
            native_frozen: false,
            effect_frozen: false,
            final_frozen: false,
            prepared: None,
            requests: 0,
            removals: [None; 24],
            lifecycle_order: LifecycleOrder::default(),
            top_entries: 0,
            main_reserved: false,
            begun: false,
        });
        i.permission.as_mut().unwrap().environment = Some(cached_environment);
    }
    let message = owner
        .borrow()
        .instance
        .as_ref()
        .and_then(|i| i.message_token);
    if let Some(boundary) = ctx.boundary(owner) {
        with_state(owner, |s| {
            let row = &s.facts.coverage;
            if row.add_attempts[23] == 1
                && row.add_returns[23] == Some(true)
                && row.add_acks[23] == 1
            {
                if let Some(token) = message {
                    let _ = s.protocol.registration_added(
                        CoverageKey::CoreMessage,
                        NativeToken::Event(token),
                        boundary,
                    );
                } else {
                    s.fail("permissionSetupRejected");
                }
            } else {
                s.fail("permissionSetupRejected");
            }
        });
    }
    drop(core);
    drop(active);
    Ok(())
}
pub(super) fn begin(
    owner: &Rc<RefCell<Owner>>,
    request: &Code0WindowRequest,
    started: Instant,
) -> Result<(), Code0SessionFailure> {
    let receipt = require_native_refusal_install_checked(owner, request, true)?;
    if receipt.installed_mask() != 1023 || receipt.load_rejected() {
        return Err(Code0SessionFailure::PolicyRejected);
    }
    let reserved = {
        let mut o = owner.borrow_mut();
        o.permission_intent
            .as_mut()
            .is_some_and(|intent| intent.reserve_begin(request, started))
    };
    if !reserved {
        reject(owner, "permissionSetupRejected", false);
        return Err(Code0SessionFailure::PolicyRejected);
    }
    let ctx = context(owner).ok_or(Code0SessionFailure::PolicyRejected)?;
    let active = Active::enter(&ctx.callbacks);
    let core = owner
        .borrow()
        .instance
        .as_ref()
        .and_then(|i| i.refusal_registry.http_core())
        .ok_or(Code0SessionFailure::PolicyRejected)?;
    if !ctx.sender(owner, Some(core.as_ref())) {
        reject(owner, "permissionIdentityChanged", true);
        return Err(Code0SessionFailure::PolicyRejected);
    }
    let handoff = with_state(owner, |s| {
        if s.begun
            || s.protocol.status().cutoff.is_some()
            || !s.ctx.disposal.rebind_once(&core, false)
        {
            s.fail("permissionIdentityChanged");
            return None;
        }
        s.begun = true;
        s.facts.setup.hooks1023 = receipt.installed_mask();
        Some(s.core.replace(Rc::clone(&core)))
    })
    .flatten();
    let Some(old) = handoff else {
        reject(owner, "permissionIdentityChanged", true);
        return Err(Code0SessionFailure::PolicyRejected);
    };
    drop(old);
    if !ctx.positive(owner) {
        reject(owner, "permissionSetupRejected", true);
        return Err(Code0SessionFailure::PolicyRejected);
    }
    let query = core.cast::<ICoreWebView2_22>();
    let core22 = match query {
        Ok(value) => Rc::new(value),
        Err(error) => {
            reject(owner, "permissionSetupRejected", true);
            drop(error);
            return Err(Code0SessionFailure::PolicyRejected);
        }
    };
    with_state(owner, |s| s.core22 = Some(Rc::clone(&core22)));
    for receiver in Receiver::ALL {
        if !ctx.positive(owner) {
            return Err(Code0SessionFailure::PolicyRejected);
        }
        let charge = TextCharge::reserve(&ctx.inventory.text, receiver.name().len() + 1)
            .ok_or(Code0SessionFailure::PolicyRejected)?;
        let name: Vec<u16> = receiver.name().encode_utf16().chain(Some(0)).collect();
        let result = unsafe { core.GetDevToolsProtocolEventReceiver(PCWSTR(name.as_ptr())) };
        let emitter = match result {
            Ok(value) => Rc::new(value),
            Err(error) => {
                reject(owner, "permissionSetupRejected", true);
                drop(error);
                return Err(Code0SessionFailure::PolicyRejected);
            }
        };
        drop(name);
        drop(charge);
        let canonical = emitter.cast::<IUnknown>();
        let identity_observed = canonical
            .as_ref()
            .is_ok_and(|identity| !identity.as_raw().is_null());
        if !identity_observed || !ctx.positive(owner) {
            reject(owner, "permissionSetupRejected", true);
            drop(canonical);
            drop(emitter);
            return Err(Code0SessionFailure::PolicyRejected);
        }
        // The successful canonical query is still owned while this exact
        // original emitter is bound to its one fixed row and private seal.
        let binding = Rc::new(ReceiverBinding::new(receiver));
        let inserted = with_state(owner, |s| {
            if s.receivers.len() != receiver as usize
                || s.receiver_bindings.len() != receiver as usize
            {
                return false;
            }
            s.receivers.push(ReceiverRegistration {
                receiver: Rc::clone(&emitter),
                binding: Rc::clone(&binding),
                token: None,
                attempted: false,
            });
            s.receiver_bindings.push(Rc::clone(&binding));
            true
        })
        .unwrap_or(false);
        native_result(owner, inserted, "permissionSetupRejected", true);
        drop(canonical);
        let still_bound = with_state(owner, |s| {
            s.receivers.get(receiver as usize).is_some_and(|row| {
                Rc::ptr_eq(&row.receiver, &emitter) && Rc::ptr_eq(&row.binding, &binding)
            })
        })
        .unwrap_or(false);
        if !inserted || !still_bound || !ctx.positive(owner) {
            reject(owner, "permissionSetupRejected", true);
            drop(emitter);
            return Err(Code0SessionFailure::PolicyRejected);
        }
        binding.canonical_attested.set(true);
        if !binding.reserve_handler() {
            reject(owner, "permissionSetupRejected", true);
            drop(emitter);
            return Err(Code0SessionFailure::PolicyRejected);
        }
        let handler: ICoreWebView2DevToolsProtocolEventReceivedEventHandler = LifecycleHandler {
            ctx: ctx.clone(),
            receiver,
            binding: Rc::clone(&binding),
            _lifetime: EventLifetime::new(&ctx.callbacks, &ctx.inventory),
        }
        .into();
        let slot = 10 + receiver as usize;
        binding.add_started.set(true);
        with_state(owner, |s| s.facts.coverage.add_entered(slot));
        let mut token = 0;
        let result = unsafe { emitter.add_DevToolsProtocolEventReceived(&handler, &mut token) };
        let ok = result.is_ok();
        binding.returned.set(Some(ok));
        if ok {
            binding.token.set(Some(token));
        }
        with_state(owner, |s| {
            s.facts.coverage.add_returned(slot, ok);
            if !ok {
                s.fail("permissionSetupRejected");
            }
            if ok {
                s.receivers[receiver as usize].token = Some(token);
            }
        });
        drop(result);
        drop(handler);
        drop(emitter);
        registration_ack(owner, slot, NativeToken::Event(token), ok);
        let ack = with_state(owner, |s| {
            s.facts.coverage.add_acks[slot] == 1
                && s.receiver_bindings
                    .get(receiver as usize)
                    .is_some_and(|expected| binding.matches(expected, receiver))
        })
        .unwrap_or(false);
        binding.acked.set(ack);
        if !ctx.positive(owner) {
            return Err(Code0SessionFailure::PolicyRejected);
        }
    }
    register_document_routes(owner, &ctx, &core, &core22)?;
    drop(core22);
    drop(core);
    drop(active);
    with_state(owner, |s| {
        s.facts.setup.receivers_ready = Some(
            s.receivers.len() == 10
                && s.receiver_bindings.len() == 10
                && s.receiver_bindings.iter().all(|b| b.registered())
                && s.facts.coverage.add_acks[10..20].iter().all(|n| *n == 1),
        );
        s.sync();
    });
    publish(owner);
    Ok(())
}

pub(super) fn native_borrowed(
    owner: &Rc<RefCell<Owner>>,
    sender: Option<&ICoreWebView2>,
    args: Option<&ICoreWebView2PermissionRequestedEventArgs>,
    binding: bool,
    entry: Instant,
) {
    let Some(ctx) = context(owner) else { return };
    if !ctx.ui() {
        return;
    }
    if !binding {
        reject(owner, "permissionIdentityChanged", true);
    }
    let candidate = native_capture(&ctx, owner, sender, args, binding, entry);
    finish_native(candidate);
}
pub(super) fn unexpected(owner: &Rc<RefCell<Owner>>) {
    if selected(owner) {
        reject(owner, "permissionProtocolRejected", false);
        cutoff(owner, CutoffReason::CoverageLost);
    }
}
pub(super) fn top_borrowed(
    owner: &Rc<RefCell<Owner>>,
    sender: Option<&ICoreWebView2>,
    args: Option<&ICoreWebView2NavigationStartingEventArgs>,
    binding: bool,
) {
    let Some(ctx) = context(owner) else { return };
    if !ctx.ui() {
        return;
    }
    if !binding {
        reject(owner, "permissionIdentityChanged", true);
    }
    let first = with_state(owner, |s| {
        let first = increment(&mut s.top_entries, 1) && s.navigation_reserved;
        if !first {
            s.fail("permissionProtocolRejected");
        }
        first
    })
    .unwrap_or(false);
    let live = first && ctx.positive(owner) && ctx.sender(owner, sender);
    let mut observed = None;
    if let Some(args) = args {
        let uri = allocated(&ctx, owner, 2048, |raw| unsafe { args.Uri(raw) });
        let exact = uri.as_ref().is_some_and(|s| s.value == semantic::FIXED_URL);
        native_result(owner, exact, "permissionProtocolRejected", uri.is_none());
        drop(uri);
        let mut navigation = 0;
        let r = unsafe { args.NavigationId(&mut navigation) };
        let id_ok = native_result(owner, r.is_ok(), "permissionProtocolRejected", true);
        drop(r);
        let mut redirected = BOOL(1);
        let r = unsafe { args.IsRedirected(&mut redirected) };
        let redirect_ok = native_result(
            owner,
            r.is_ok() && redirected.0 == 0,
            "permissionProtocolRejected",
            r.is_err(),
        );
        drop(r);
        let mut cancel = BOOL(1);
        let r = unsafe { args.Cancel(&mut cancel) };
        let cancel_ok = native_result(
            owner,
            r.is_ok() && cancel.0 == 0,
            "permissionProtocolRejected",
            r.is_err(),
        );
        drop(r);
        if live && exact && id_ok && redirect_ok && cancel_ok && ctx.positive(owner) {
            observed = Some(navigation);
        } else {
            let r = unsafe { args.SetCancel(true) };
            let unknown = r.is_err();
            reject(owner, "permissionProtocolRejected", unknown);
            drop(r);
        }
    } else {
        reject(owner, "permissionProtocolRejected", true);
    }
    if let (Some(id), Some(boundary)) = (observed, ctx.boundary(owner)) {
        let current = ctx.positive(owner);
        with_state(owner, |s| {
            if current {
                if s.protocol
                    .navigation_started(id, semantic::FIXED_URL, false, boundary)
                    .is_err()
                {
                    s.fail("permissionProtocolRejected");
                }
            } else {
                s.fail("permissionIdentityChanged");
            }
        });
    }
}
fn completed_borrowed(
    ctx: &Context,
    sender: Option<&ICoreWebView2>,
    args: Option<&ICoreWebView2NavigationCompletedEventArgs>,
) {
    if !ctx.ui() {
        return;
    }
    let Some(owner) = ctx.owner.upgrade() else {
        return;
    };
    let current = ctx.sender(&owner, sender) && ctx.positive(&owner);
    let observed = args.and_then(|args| {
        let mut id = 0;
        let result = unsafe { args.NavigationId(&mut id) };
        let ok = native_result(&owner, result.is_ok(), "permissionProtocolRejected", true);
        drop(result);
        let mut success = BOOL(0);
        let result = unsafe { args.IsSuccess(&mut success) };
        let good = native_result(
            &owner,
            result.is_ok() && success.0 == 1,
            "permissionProtocolRejected",
            result.is_err(),
        );
        drop(result);
        if ok && current && ctx.positive(&owner) {
            Some((id, good))
        } else {
            None
        }
    });
    if let Some(boundary) = ctx.boundary(&owner) {
        let positive = ctx.positive(&owner);
        with_state(&owner, |s| {
            if let Some((id, good)) = observed {
                s.facts.status.navigation_succeeded = Some(good && positive);
                if s.protocol
                    .navigation_completed(id, good && positive, boundary)
                    .is_err()
                {
                    s.fail("permissionProtocolRejected");
                }
            } else {
                s.facts.status.navigation_succeeded = Some(false);
                s.fail("permissionProtocolRejected");
            }
        });
    }
}
const HTML:&[u8]=b"<!doctype html><html><head><meta charset=\"utf-8\"><title>Code0 permission fixture</title></head><body></body></html>";
fn response(
    ctx: &Context,
    owner: &Rc<RefCell<Owner>>,
    args: &ICoreWebView2WebResourceRequestedEventArgs,
    main: bool,
) -> bool {
    let environment = owner
        .borrow()
        .instance
        .as_ref()
        .and_then(|i| i.permission.as_ref())
        .and_then(|s| s.environment.as_ref())
        .map(Rc::clone);
    let body = if main {
        HTML
    } else {
        b"Code0 permission fixture refusal"
    };
    let Some(charge) = TextCharge::reserve(&ctx.inventory.text, body.len() + 1024) else {
        reject(owner, "permissionOverflow", true);
        return false;
    };
    let Some(stream) = (unsafe { windows::Win32::UI::Shell::SHCreateMemStream(Some(body)) }) else {
        reject(owner, "permissionUnknown", true);
        return false;
    };
    let result = unsafe { stream.Seek(0, windows::Win32::System::Com::STREAM_SEEK_SET, None) };
    let seek = native_result(owner, result.is_ok(), "permissionUnknown", true);
    drop(result);
    let response=environment.as_ref().map(|environment|unsafe{environment.CreateWebResourceResponse(&stream,if main{200}else{403},if main{windows::core::w!("OK")}else{windows::core::w!("Forbidden")},if main{windows::core::w!("Content-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'none'; script-src 'unsafe-eval'; connect-src 'none'; img-src 'none'; style-src 'none'; font-src 'none'; media-src 'none'; object-src 'none'; frame-src 'none'; worker-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'\r\n")}else{windows::core::w!("Content-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'none'\r\n")} )});
    native_result(
        owner,
        response.as_ref().is_some_and(|result| result.is_ok()),
        "permissionUnknown",
        true,
    );
    let mut good = false;
    if seek {
        if let Some(Ok(response)) = response {
            if main {
                with_state(owner, |s| {
                    counted(
                        &mut s.facts.counts.main_responses,
                        1,
                        &mut s.facts.overflow,
                        &mut s.facts.failure,
                    );
                });
            }
            let result = unsafe { args.SetResponse(&response) };
            let set = native_result(owner, result.is_ok(), "permissionUnknown", true);
            drop(result);
            // Read back the actual response object independently of HRESULT alone.
            let read = unsafe { args.Response() };
            native_result(owner, read.is_ok(), "permissionUnknown", true);
            if let Ok(read) = read {
                let expected = response.cast::<IUnknown>();
                native_result(owner, expected.is_ok(), "permissionIdentityChanged", true);
                let actual = read.cast::<IUnknown>();
                good = set
                    && expected
                        .as_ref()
                        .ok()
                        .zip(actual.as_ref().ok())
                        .is_some_and(|(a, b)| a.as_raw() == b.as_raw());
                native_result(owner, good, "permissionIdentityChanged", true);
                drop(actual);
                drop(expected);
                drop(read);
            }
            drop(response);
        }
    }
    drop(stream);
    drop(environment);
    drop(charge);
    good
}
fn resource_borrowed(
    ctx: &Context,
    sender: Option<&ICoreWebView2>,
    args: Option<&ICoreWebView2WebResourceRequestedEventArgs>,
) {
    if !ctx.ui() {
        return;
    }
    let Some(owner) = ctx.owner.upgrade() else {
        return;
    };
    let eligible = with_state(&owner, |s| {
        let bounded = increment(&mut s.requests, 16);
        let first = take_once(&mut s.main_reserved);
        if !bounded {
            s.facts.overflow = true;
            s.fail("permissionOverflow");
        }
        bounded && first && s.navigation_reserved
    })
    .unwrap_or(false)
        && ctx.positive(&owner)
        && ctx.sender(&owner, sender);
    let mut main = false;
    let mut installed = false;
    if let Some(args) = args {
        let request = unsafe { args.Request() };
        native_result(&owner, request.is_ok(), "permissionUnknown", true);
        if let Ok(request) = request {
            let uri = allocated(ctx, &owner, 2048, |raw| unsafe { request.Uri(raw) });
            let exact = uri.as_ref().is_some_and(|s| s.value == semantic::FIXED_URL);
            native_result(&owner, exact, "permissionProtocolRejected", uri.is_none());
            drop(uri);
            let method = allocated(ctx, &owner, 2048, |raw| unsafe { request.Method(raw) });
            let get = method.as_ref().is_some_and(|s| s.value == "GET");
            native_result(&owner, get, "permissionProtocolRejected", method.is_none());
            drop(method);
            let mut context = COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL;
            let r = unsafe { args.ResourceContext(&mut context) };
            let document = native_result(
                &owner,
                r.is_ok() && context == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT,
                "permissionProtocolRejected",
                r.is_err(),
            );
            drop(r);
            drop(request);
            main = eligible && exact && get && document && ctx.positive(&owner);
        }
        // Revoke before refusal/native response construction can reenter. A
        // failed SetResponse is explicit unknown, never containment proof.
        if !main {
            with_state(&owner, |s| {
                counted(
                    &mut s.facts.counts.unexpected_requests,
                    16,
                    &mut s.facts.overflow,
                    &mut s.facts.failure,
                );
            });
            reject(&owner, "permissionProtocolRejected", false);
        }
        installed = response(ctx, &owner, args, main);
    }
    if main {
        if let Some(boundary) = ctx.boundary(&owner) {
            let current = ctx.positive(&owner);
            with_state(&owner, |s| {
                s.facts.status.main_response_succeeded = Some(installed && current);
                let _ = s
                    .protocol
                    .main_response_installed(installed && current, boundary);
            });
        }
    }
    if !installed {
        reject(&owner, "permissionUnknown", true);
    }
}
fn register_document_routes(
    owner: &Rc<RefCell<Owner>>,
    ctx: &Context,
    core: &Rc<ICoreWebView2>,
    core22: &Rc<ICoreWebView2_22>,
) -> Result<(), Code0SessionFailure> {
    let completed: ICoreWebView2NavigationCompletedEventHandler = CompletedHandler {
        ctx: ctx.clone(),
        _lifetime: EventLifetime::new(&ctx.callbacks, &ctx.inventory),
    }
    .into();
    with_state(owner, |s| s.facts.coverage.add_entered(20));
    let mut token = 0;
    let result = unsafe { core.add_NavigationCompleted(&completed, &mut token) };
    let ok = result.is_ok();
    with_state(owner, |s| {
        s.facts.coverage.add_returned(20, ok);
        if !ok {
            s.fail("permissionSetupRejected");
        }
        if ok {
            s.completed = Some(token);
        }
    });
    drop(result);
    drop(completed);
    registration_ack(owner, 20, NativeToken::Event(token), ok);
    if !ctx.positive(owner) {
        return Err(Code0SessionFailure::PolicyRejected);
    }
    let resource: ICoreWebView2WebResourceRequestedEventHandler = ResourceHandler {
        ctx: ctx.clone(),
        _lifetime: EventLifetime::new(&ctx.callbacks, &ctx.inventory),
    }
    .into();
    with_state(owner, |s| s.facts.coverage.add_entered(21));
    let mut token = 0;
    let result = unsafe { core.add_WebResourceRequested(&resource, &mut token) };
    let ok = result.is_ok();
    with_state(owner, |s| {
        s.facts.coverage.add_returned(21, ok);
        if !ok {
            s.fail("permissionSetupRejected");
        }
        if ok {
            s.resource = Some(token);
        }
    });
    drop(result);
    drop(resource);
    registration_ack(owner, 21, NativeToken::Event(token), ok);
    if !ctx.positive(owner) {
        return Err(Code0SessionFailure::PolicyRejected);
    }
    with_state(owner, |s| s.facts.coverage.add_entered(22));
    let result = unsafe {
        core22.AddWebResourceRequestedFilterWithRequestSourceKinds(
            windows::core::w!("*"),
            COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
            COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
        )
    };
    let ok = result.is_ok();
    with_state(owner, |s| {
        s.facts.coverage.add_returned(22, ok);
        if !ok {
            s.fail("permissionSetupRejected");
        }
        if ok {
            s.filter = true;
        }
    });
    drop(result);
    registration_ack(owner, 22, NativeToken::Filter, ok);
    let ready = ctx.positive(owner);
    with_state(owner, |s| {
        s.facts.setup.responder_ready = Some(ready && s.resource.is_some() && s.filter)
    });
    if ready {
        Ok(())
    } else {
        Err(Code0SessionFailure::PolicyRejected)
    }
}

struct Prepared {
    command: Command,
    unique: Option<Id>,
    method: Vec<u16>,
    parameters: Vec<u16>,
    core: Rc<ICoreWebView2>,
    handler: ICoreWebView2CallDevToolsProtocolMethodCompletedHandler,
    ticket: Rc<Cell<Option<CommandTicket>>>,
    armed: Arc<AtomicBool>,
    _method_charge: TextCharge,
    _parameter_charge: TextCharge,
}
#[windows::core::implement(ICoreWebView2CallDevToolsProtocolMethodCompletedHandler, Agile = false)]
struct Completion {
    ctx: Context,
    command: Command,
    ticket: Rc<Cell<Option<CommandTicket>>>,
    _lifetime: CommandLifetime,
}
impl ICoreWebView2CallDevToolsProtocolMethodCompletedHandler_Impl for Completion_Impl {
    fn Invoke(&self, hresult: windows::core::HRESULT, raw: &PCWSTR) -> windows::core::Result<()> {
        let ctx = &self.ctx;
        let active = Active::enter(&ctx.callbacks);
        let entry = Instant::now();
        if !ctx.ui() {
            return Ok(());
        }
        let Some(owner) = ctx.owner.upgrade() else {
            return Ok(());
        };
        let Some(ticket) = self.ticket.get() else {
            reject(&owner, "permissionUnknown", true);
            return Ok(());
        };
        native_result(&owner, hresult.is_ok(), "permissionProtocolRejected", true);
        let index = self.command as usize;
        let first = with_state(&owner, |s| s.facts.commands.completed[index] == 0).unwrap_or(false);
        with_state(&owner, |s| {
            counted(
                &mut s.facts.commands.completed[index],
                1,
                &mut s.facts.overflow,
                &mut s.facts.failure,
            );
            counted(
                &mut s.facts.counts.command_completed,
                4,
                &mut s.facts.overflow,
                &mut s.facts.failure,
            );
            if index == 3 {
                counted(
                    &mut s.facts.counts.evaluate_completions,
                    1,
                    &mut s.facts.overflow,
                    &mut s.facts.failure,
                );
            }
            if index == 3 && first {
                s.facts.timing.effect_entered_at_ms = s.timing.ms(entry);
                s.facts.effect.invoked_after_native_action = Some(
                    s.native_frozen
                        && s.protocol.status().native_qualified == Some(true)
                        && native_positive(&s.facts.native),
                );
                s.facts.effect.invoked_before_stop = Some(
                    s.facts.timing.stop_entered_at_ms.is_none()
                        && s.facts.timing.controller_close_entered_at_ms.is_none(),
                );
                s.facts.effect.entered_before_observation_cutoff =
                    Some(s.protocol.status().cutoff.is_none());
            }
        });
        let callback = ctx.disposal_boundary(&owner).and_then(|boundary| {
            with_state(&owner, |s| {
                s.protocol.completion_entered(ticket, boundary).ok()
            })
            .flatten()
        });
        let cap = [64, 64, 2048, 512][index];
        // Borrowed completion PCWSTR is scanned/copy-charged and never freed.
        // Error HRESULTs still carry actual malformed/decoded disposal facts.
        let text = unsafe { copy_text(*raw, 2048, cap, &ctx.inventory.text) };
        native_result(&owner, text.is_some(), "permissionOverflow", true);
        let decoded = text
            .as_ref()
            .and_then(|text| wire::decode_completion(self.command, text.value.as_bytes()).ok());
        native_result(
            &owner,
            decoded.is_some(),
            "permissionProtocolRejected",
            false,
        );
        let value = decoded
            .as_ref()
            .map_or(CompletionValue::Malformed, |d| d.as_semantic());
        let spelling = match value {
            CompletionValue::EmptyAck => "emptyAck",
            CompletionValue::FrameTree(_) => "frameTree",
            CompletionValue::Denied => "denied",
            CompletionValue::Precondition => "precondition",
            CompletionValue::Default => "default",
            CompletionValue::Granted => "granted",
            CompletionValue::DocumentChanged => "documentChanged",
            CompletionValue::Invalid => "invalid",
            CompletionValue::Malformed => "malformed",
        };
        if index == 3 && first {
            if let Some(why) = effect_failure(
                with_state(&owner, |s| s.native_reserved).unwrap_or(false),
                hresult.is_ok(),
                spelling,
            ) {
                reject(&owner, why, false);
            }
        }
        let source_matched = if index == 3 {
            source(ctx, &owner)
        } else {
            None
        };
        with_state(&owner, |s| {
            if first {
                s.facts.commands.completion_succeeded[index] = Some(hresult.is_ok());
                s.facts.commands.values[index] = Some(spelling);
            }
        });
        if index == 3 && first {
            with_state(&owner, |s| {
                if let Some(why) = effect_failure(s.native_reserved, hresult.is_ok(), spelling) {
                    s.fail(why);
                }
            });
        }
        if let (Some(callback), Some(boundary)) = (callback, ctx.disposal_boundary(&owner)) {
            let positive = ctx.positive(&owner);
            with_state(&owner, |s| {
                if first {
                    s.facts.commands.completion_succeeded[index] = Some(hresult.is_ok());
                    s.facts.commands.values[index] = Some(spelling);
                }
                let _ = s
                    .protocol
                    .completion_captured(callback, hresult.is_ok(), value, boundary);
                if index == 3 && first {
                    s.facts.effect.success_hresult = Some(hresult.is_ok());
                    s.facts.effect.closed_denied_result = Some(spelling == "denied");
                    s.facts.effect.same_ticket = Some(true);
                    s.facts.effect.source_matched = source_matched;
                    s.facts.effect.same_frame_loader_context =
                        Some(positive && source_matched == Some(true));
                    s.facts.effect.captured_before_stop = Some(
                        s.facts.timing.stop_entered_at_ms.is_none()
                            && s.facts.timing.controller_close_entered_at_ms.is_none(),
                    );
                    s.facts.effect.captured_before_observation_cutoff =
                        Some(s.protocol.status().cutoff.is_none());
                    s.facts.timing.effect_captured_at_ms = s.timing.ms(Instant::now());
                }
            });
        }
        drop(text);
        if let (Some(callback), Some(boundary)) = (callback, ctx.disposal_boundary(&owner)) {
            let positive = ctx.positive(&owner);
            let current = ctx.same(&owner);
            with_state(&owner, |s| {
                let qualified = s.protocol.completion_qualified(callback, boundary).is_ok();
                let status = s.protocol.status();
                let qualified = qualified
                    && positive
                    && current
                    && (index != 3
                        || source_matched == Some(true)
                            && s.facts.effect.invoked_after_native_action == Some(true));
                s.facts.commands.qualified[index] = Some(qualified);
                s.completed_pending[index] = true;
                if index == 3 {
                    s.effect_frozen = true;
                    counted(
                        &mut s.facts.counts.effect_receipts,
                        1,
                        &mut s.facts.overflow,
                        &mut s.facts.failure,
                    );
                    s.facts.status.effect_result = Some(spelling);
                    s.facts.effect.qualified_before_stop = Some(
                        s.facts.timing.stop_entered_at_ms.is_none()
                            && s.facts.timing.controller_close_entered_at_ms.is_none(),
                    );
                    s.facts.effect.qualified_before_observation_cutoff =
                        Some(status.cutoff.is_none());
                    s.facts.effect.original_timely =
                        Some(s.timing.before(entry) && s.timing.before(Instant::now()));
                    s.facts.effect.close_timely = Some(s.timing.close_before(Instant::now()));
                    s.facts.effect.epoch_unchanged_through_qualification =
                        Some(qualified && status.effect_qualified == Some(true));
                    s.facts.timing.effect_qualified_at_ms = s.timing.ms(Instant::now());
                    if !qualified {
                        s.fail("permissionEffectLate");
                    }
                }
                s.sync();
            });
        } else {
            reject(&owner, "permissionIdentityChanged", true);
        }
        publish(&owner);
        drop(active);
        Ok(())
    }
}
fn prepare_command(
    owner: &Rc<RefCell<Owner>>,
    ctx: &Context,
    command: Command,
) -> Option<Prepared> {
    let unique = if command == Command::Evaluate {
        with_state(owner, |s| s.protocol.selected_unique_context()).flatten()
    } else {
        None
    };
    if command == Command::Evaluate && unique.is_none() {
        return None;
    }
    let serialization = TextCharge::reserve(&ctx.inventory.text, 2047)?;
    let request = match wire::command_request(command, unique) {
        Ok(value) => value,
        Err(_) => {
            reject(owner, "permissionProtocolRejected", false);
            return None;
        }
    };
    let parameter_charge =
        TextCharge::reserve(&ctx.inventory.text, request.parameters().len() + 1)?;
    let method_charge = TextCharge::reserve(&ctx.inventory.text, request.method().len() + 1)?;
    let method = request.method().encode_utf16().chain(Some(0)).collect();
    let parameters = request
        .parameters()
        .iter()
        .map(|b| u16::from(*b))
        .chain(Some(0))
        .collect();
    drop(request);
    drop(serialization);
    let core = owner
        .borrow()
        .instance
        .as_ref()?
        .permission
        .as_ref()?
        .core
        .as_ref()
        .map(Rc::clone)?;
    let ticket = Rc::new(Cell::new(None));
    let armed = Arc::new(AtomicBool::new(false));
    let lifetime = CommandLifetime::new(
        &ctx.callbacks,
        &ctx.inventory,
        command as usize,
        Arc::clone(&armed),
    );
    let handler: ICoreWebView2CallDevToolsProtocolMethodCompletedHandler = Completion {
        ctx: ctx.clone(),
        command,
        ticket: Rc::clone(&ticket),
        _lifetime: lifetime,
    }
    .into();
    Some(Prepared {
        command,
        unique,
        method,
        parameters,
        core,
        handler,
        ticket,
        armed,
        _method_charge: method_charge,
        _parameter_charge: parameter_charge,
    })
}
fn issue(owner: &Rc<RefCell<Owner>>, ctx: &Context, command: Command) {
    let mut prepared = with_state(owner, |s| s.prepared.take()).flatten();
    if prepared.is_none() {
        prepared = prepare_command(owner, ctx, command);
    }
    let Some(prepared) = prepared else {
        reject(owner, "permissionSetupRejected", true);
        return;
    };
    if prepared.command != command || !ctx.positive(owner) {
        reject(owner, "permissionIdentityChanged", true);
        drop(prepared);
        return;
    }
    // Every native allocation/conversion/handler preparation is finished. The
    // final actual Source call and its drops occur before the ticket reservation.
    if command == Command::Evaluate && source(ctx, owner) != Some(true) {
        reject(owner, "permissionIdentityChanged", true);
        drop(prepared);
        return;
    }
    let Some(boundary) = ctx.boundary(owner) else {
        reject(owner, "permissionIdentityChanged", true);
        drop(prepared);
        return;
    };
    let index = command as usize;
    let entered = with_state(owner, |s| {
        let result = if command == Command::Evaluate {
            s.protocol
                .evaluation_entered(semantic::FIXED_URL, boundary)
                .and_then(|evaluation| {
                    prepared.ticket.set(Some(evaluation.ticket));
                    prepared.armed.store(true, Ordering::SeqCst);
                    s.tickets[index] = Some(evaluation.ticket);
                    // Copy/Eq only; no native work/allocation/opaque permit here.
                    if Some(evaluation.unique_context) == prepared.unique {
                        Ok(evaluation.ticket)
                    } else {
                        Err(Problem::Context)
                    }
                })
        } else {
            s.protocol.command_entered(command, boundary)
        };
        if let Ok(ticket) = result {
            prepared.ticket.set(Some(ticket));
            prepared.armed.store(true, Ordering::SeqCst);
            s.tickets[index] = Some(ticket);
            counted(
                &mut s.facts.commands.issued[index],
                1,
                &mut s.facts.overflow,
                &mut s.facts.failure,
            );
            counted(
                &mut s.facts.counts.command_issued,
                4,
                &mut s.facts.overflow,
                &mut s.facts.failure,
            );
            if index == 3 {
                counted(
                    &mut s.facts.counts.evaluate_submissions,
                    1,
                    &mut s.facts.overflow,
                    &mut s.facts.failure,
                );
            }
        }
        result
    });
    match entered {
        Some(Ok(ticket)) => {
            // No native call, allocation, parameter update or reentrant Drop
            // intervenes between the reservation and this exact fixed method.
            let result = unsafe {
                prepared.core.CallDevToolsProtocolMethod(
                    PCWSTR(prepared.method.as_ptr()),
                    PCWSTR(prepared.parameters.as_ptr()),
                    &prepared.handler,
                )
            };
            let ok = result.is_ok();
            let boundary = ctx.boundary(owner);
            with_state(owner, |s| {
                counted(
                    &mut s.facts.commands.returned[index],
                    1,
                    &mut s.facts.overflow,
                    &mut s.facts.failure,
                );
                counted(
                    &mut s.facts.counts.command_returned,
                    4,
                    &mut s.facts.overflow,
                    &mut s.facts.failure,
                );
                s.facts.commands.submission_succeeded[index] = Some(ok);
                if let Some(boundary) = boundary {
                    let _ = s.protocol.command_returned(ticket, ok, boundary);
                }
                if !ok {
                    s.fail("permissionProtocolRejected");
                }
            });
            drop(result);
            drop(prepared);
            if !ctx.same(owner) {
                reject(owner, "permissionIdentityChanged", true);
            }
        }
        Some(Err(Problem::NotReady)) => {
            with_state(owner, |s| s.prepared = Some(prepared));
        }
        _ => {
            reject(owner, "permissionProtocolRejected", false);
            drop(prepared);
        }
    }
}
pub(super) fn advance(owner: &Rc<RefCell<Owner>>) {
    let Some(ctx) = context(owner) else { return };
    if !ctx.ui() {
        return;
    }
    let active = Active::enter(&ctx.callbacks);
    let expired = with_state(owner, |s| {
        !s.timing.before(Instant::now()) && s.timing.first_close.is_none()
    })
    .unwrap_or(true);
    if expired {
        reject(owner, "permissionDeadlineExceeded", false);
    }
    if ctx.inventory.affinity_cutoff.load(Ordering::SeqCst)
        || ctx.callbacks.affinity_failed.load(Ordering::SeqCst)
    {
        with_state(owner, |s| {
            s.protocol.affinity_lost();
            s.fail("uiAffinityFailed");
        });
    }
    // Actual command handler-object drops are atomically reported by Drop,
    // independently of callback invocation/consumption and on the next UI turn.
    let drops = ctx.inventory.command_drop_bits.swap(0, Ordering::SeqCst);
    for index in 0..4 {
        if drops & (1 << index) != 0 {
            if let Some(boundary) = ctx.boundary(owner) {
                with_state(owner, |s| {
                    if let Some(ticket) = s.tickets[index] {
                        let _ = s.protocol.command_handler_dropped(ticket, boundary);
                        counted(
                            &mut s.facts.commands.handler_drops[index],
                            1,
                            &mut s.facts.overflow,
                            &mut s.facts.failure,
                        );
                    }
                });
            }
        }
        let pending = with_state(owner, |s| s.completed_pending[index]).unwrap_or(false);
        if pending {
            if let Some(boundary) = ctx.disposal_boundary(owner) {
                with_state(owner, |s| {
                    if let Some(ticket) = s.tickets[index] {
                        if s.protocol.completion_consumed(ticket, boundary).is_ok() {
                            s.completed_pending[index] = false;
                            counted(
                                &mut s.facts.commands.consumed[index],
                                1,
                                &mut s.facts.overflow,
                                &mut s.facts.failure,
                            );
                            counted(
                                &mut s.facts.counts.command_consumed,
                                4,
                                &mut s.facts.overflow,
                                &mut s.facts.failure,
                            );
                        }
                    }
                });
            }
        }
    }
    if ctx.positive(owner)
        && with_state(owner, |s| s.begun && s.facts.coverage.added_mask == ALL).unwrap_or(false)
    {
        let next = with_state(owner, |s| {
            if s.facts.commands.issued[0] == 0 {
                Some(Command::PageEnable)
            } else if s.facts.commands.consumed[0] == 1 && s.facts.commands.issued[1] == 0 {
                Some(Command::RuntimeEnable)
            } else if s.facts.commands.consumed[1] == 1
                && s.navigation_reserved
                && s.facts.commands.issued[2] == 0
            {
                Some(Command::GetFrameTree)
            } else if s.protocol.status().phase == Phase::Armed && s.facts.commands.issued[3] == 0 {
                Some(Command::Evaluate)
            } else {
                None
            }
        })
        .flatten();
        if let Some(command) = next {
            issue(owner, &ctx, command);
        }
        let navigate = with_state(owner, |s| {
            s.facts.commands.consumed[1] == 1 && !s.navigation_reserved
        })
        .unwrap_or(false);
        if navigate {
            let wide_charge =
                TextCharge::reserve(&ctx.inventory.text, semantic::FIXED_URL.len() + 1);
            let wide: Vec<u16> = semantic::FIXED_URL.encode_utf16().chain(Some(0)).collect();
            let core = owner
                .borrow()
                .instance
                .as_ref()
                .and_then(|i| i.permission.as_ref())
                .and_then(|s| s.core.as_ref())
                .map(Rc::clone);
            if let (Some(_), Some(core), Some(boundary)) =
                (wide_charge.as_ref(), core.as_ref(), ctx.boundary(owner))
            {
                let reserved = with_state(owner, |s| {
                    let result = s.protocol.navigation_reserved(boundary);
                    if result.is_ok() {
                        s.navigation_reserved = true;
                        counted(
                            &mut s.facts.counts.navigation,
                            1,
                            &mut s.facts.overflow,
                            &mut s.facts.failure,
                        );
                    }
                    result.is_ok()
                })
                .unwrap_or(false);
                if reserved {
                    let result = unsafe { core.Navigate(PCWSTR(wide.as_ptr())) };
                    let ok =
                        native_result(owner, result.is_ok(), "permissionProtocolRejected", true);
                    drop(result);
                    if let Some(boundary) = ctx.boundary(owner) {
                        with_state(owner, |s| {
                            let _ = s.protocol.navigation_returned(ok, boundary);
                            if !ok {
                                s.fail("permissionProtocolRejected");
                            }
                        });
                    }
                }
            }
            drop(core);
            drop(wide);
            drop(wide_charge);
        }
        let arm = with_state(owner, |s| {
            s.facts.commands.consumed[2] == 1 && !s.source_checked
        })
        .unwrap_or(false);
        let source_result = if arm { source(&ctx, owner) } else { None };
        if arm && source_result != Some(true) {
            reject(owner, "permissionIdentityChanged", source_result.is_none());
        }
        if arm && source_result == Some(true) {
            if let Some(boundary) = ctx.boundary(owner) {
                with_state(owner, |s| {
                    if s.protocol
                        .source_after_navigation(semantic::FIXED_URL, boundary)
                        .is_ok()
                        && s.protocol.arm(boundary).is_ok()
                    {
                        s.source_checked = true;
                        s.facts.setup.domains_ack = Some(
                            s.facts.commands.qualified[0] == Some(true)
                                && s.facts.commands.qualified[1] == Some(true),
                        );
                        s.facts.setup.document_bound = Some(true);
                        s.facts.setup.lifecycle_assembly_complete = Some(true);
                        s.facts.setup.blank_branch_closed = Some(true);
                        s.facts.setup.observation_coverage_intact_at_arm = Some(
                            s.facts.coverage.added_mask == ALL
                                && s.facts.coverage.remove_attempt_mask == 0,
                        );
                    }
                });
            }
        }
    } else {
        let prepared = with_state(owner, |s| s.prepared.take()).flatten();
        drop(prepared);
    }
    with_state(owner, State::sync);
    drop(active);
    publish(owner);
}

pub(super) fn remove_entered(owner: &Rc<RefCell<Owner>>, slot: usize) {
    cutoff(owner, CutoffReason::RequiredRemovalAttempt);
    if let Some(i) = owner.borrow_mut().instance.as_mut() {
        i.permission_history.facts.remove_entered(slot);
    }
    if let Some(ctx) = context(owner) {
        if let Some(boundary) = ctx.boundary(owner) {
            with_state(owner, |s| {
                s.facts.coverage.remove_entered(slot);
                s.removals[slot] = s.protocol.removal_entered(key(slot), boundary).ok();
            });
        }
    }
}
pub(super) fn remove_returned(owner: &Rc<RefCell<Owner>>, slot: usize, success: bool) {
    if let Some(i) = owner.borrow_mut().instance.as_mut() {
        i.permission_history.facts.remove_returned(slot, success);
    }
    with_state(owner, |s| s.facts.coverage.remove_returned(slot, success));
}
pub(super) fn remove_qualified(owner: &Rc<RefCell<Owner>>, slot: usize) {
    {
        let mut o = owner.borrow_mut();
        let matched = !o.unmatched;
        if let Some(i) = o.instance.as_mut() {
            let timely = i
                .permission
                .as_ref()
                .map(|s| s.timing.close_before(Instant::now()))
                .unwrap_or_else(|| {
                    i.permission_history.first_close.is_some_and(|at| {
                        Instant::now()
                            .checked_duration_since(at)
                            .is_some_and(|elapsed| elapsed < Duration::from_secs(10))
                    })
                });
            let current = matched && !i.callbacks.affinity_failed.load(Ordering::SeqCst) && timely;
            i.permission_history.facts.remove_qualified(slot, current);
        }
    }
    let Some(ctx) = context(owner) else { return };
    let current = ctx.same(owner)
        && with_state(owner, |s| s.timing.close_before(Instant::now())).unwrap_or(false);
    if let Some(boundary) = ctx.boundary(owner) {
        with_state(owner, |s| {
            let success = s.facts.coverage.remove_returns[slot] == Some(true) && current;
            s.facts.coverage.remove_qualified(slot, success);
            if let Some(ticket) = s.removals[slot] {
                let _ = s.protocol.removal_returned(ticket, success, boundary);
            }
            if !success {
                s.fail("permissionCleanupFailed");
            }
        });
    }
}
pub(super) fn cleanup_guard(owner: &Rc<RefCell<Owner>>) -> Option<Active> {
    let o = owner.borrow();
    let i = o.instance.as_ref()?;
    if i.permission_selected.load(Ordering::SeqCst) {
        Some(Active::enter(&i.callbacks))
    } else {
        None
    }
}
// Every temporary physical slot stays visibly reserved until restoration and
// same-actor revalidation. Unknown/missing/restoration failures never clear it.
struct ActionSlot {
    inventory: Arc<NativeInventory>,
    callbacks: Arc<Callbacks>,
    bit: u32,
    resolved: bool,
}
impl ActionSlot {
    fn new(callbacks: &Arc<Callbacks>, inventory: &Arc<NativeInventory>, bit: u32) -> Option<Self> {
        if inventory.action_slots.fetch_or(bit, Ordering::SeqCst) & bit != 0 {
            inventory.slot_failed.store(true, Ordering::SeqCst);
            return None;
        }
        Some(Self {
            inventory: Arc::clone(inventory),
            callbacks: Arc::clone(callbacks),
            bit,
            resolved: false,
        })
    }
    fn matches(&self, owner: &Rc<RefCell<Owner>>) -> bool {
        let o = owner.borrow();
        !o.unmatched
            && o.instance.as_ref().is_some_and(|i| {
                Arc::ptr_eq(&i.callbacks, &self.callbacks)
                    && Arc::ptr_eq(&i.permission_history.inventory, &self.inventory)
                    && i.permission_selected.load(Ordering::SeqCst)
                    && !i.released
            })
    }
    fn resolve(mut self) {
        self.inventory
            .action_slots
            .fetch_and(!self.bit, Ordering::SeqCst);
        self.resolved = true;
    }
}
impl Drop for ActionSlot {
    fn drop(&mut self) {
        if !self.resolved {
            self.inventory.slot_failed.store(true, Ordering::SeqCst);
        }
    }
}
fn action_slot(owner: &Rc<RefCell<Owner>>, bit: u32) -> Option<ActionSlot> {
    let o = owner.borrow();
    let i = o.instance.as_ref()?;
    ActionSlot::new(&i.callbacks, &i.permission_history.inventory, bit)
}
// Caller holds cleanup_guard through the returned native reference's final
// Drop. inventory_empty also checks that actual Active guard, so a temporary
// taken field can never look object-free/Released to reentrant code.
pub(super) fn action_core(owner: &Rc<RefCell<Owner>>) -> Option<ICoreWebView2> {
    let slot = action_slot(owner, 1)?;
    let cached = owner
        .borrow()
        .instance
        .as_ref()
        .and_then(|i| i.permission.as_ref())
        .and_then(|s| s.core.as_ref())
        .map(Rc::clone);
    if let Some(core) = cached {
        let copy = core.as_ref().clone();
        drop(core);
        if slot.matches(owner) {
            slot.resolve();
            return Some(copy);
        }
        reject(owner, "permissionIdentityChanged", true);
        return None;
    }
    let original = owner
        .borrow_mut()
        .instance
        .as_mut()
        .and_then(|i| i.core.take());
    let Some(original) = original else {
        reject(owner, "permissionCleanupFailed", true);
        return None;
    };
    let copy = original.clone();
    let mut original = Some(original);
    let (replaced, restored) = {
        let mut o = owner.borrow_mut();
        if let Some(i) = o.instance.as_mut() {
            (i.core.replace(original.take().unwrap()), true)
        } else {
            (None, false)
        }
    };
    let changed = replaced.is_some();
    native_result(
        owner,
        restored && !changed && slot.matches(owner),
        "permissionIdentityChanged",
        true,
    );
    drop(replaced);
    drop(original);
    if restored && !changed && slot.matches(owner) {
        slot.resolve();
        Some(copy)
    } else {
        reject(owner, "permissionIdentityChanged", true);
        None
    }
}
pub(super) fn action_controller(owner: &Rc<RefCell<Owner>>) -> Option<ICoreWebView2Controller> {
    let slot = action_slot(owner, 2)?;
    let original = owner
        .borrow_mut()
        .instance
        .as_mut()
        .and_then(|i| i.controller.take());
    let Some(original) = original else {
        reject(owner, "permissionCleanupFailed", true);
        return None;
    };
    let copy = original.clone();
    let mut original = Some(original);
    let (replaced, restored) = {
        let mut o = owner.borrow_mut();
        if let Some(i) = o.instance.as_mut() {
            (i.controller.replace(original.take().unwrap()), true)
        } else {
            (None, false)
        }
    };
    let changed = replaced.is_some();
    native_result(
        owner,
        restored && !changed && slot.matches(owner),
        "permissionIdentityChanged",
        true,
    );
    drop(replaced);
    drop(original);
    if restored && !changed && slot.matches(owner) {
        slot.resolve();
        Some(copy)
    } else {
        reject(owner, "permissionIdentityChanged", true);
        None
    }
}
pub(super) fn action_environment5(owner: &Rc<RefCell<Owner>>) -> Option<ICoreWebView2Environment5> {
    let slot = action_slot(owner, 4)?;
    let original = owner
        .borrow_mut()
        .instance
        .as_mut()
        .and_then(|i| i.environment5.take());
    let Some(original) = original else {
        reject(owner, "permissionCleanupFailed", true);
        return None;
    };
    let copy = original.clone();
    let mut original = Some(original);
    let (replaced, restored) = {
        let mut o = owner.borrow_mut();
        if let Some(i) = o.instance.as_mut() {
            (i.environment5.replace(original.take().unwrap()), true)
        } else {
            (None, false)
        }
    };
    let changed = replaced.is_some();
    native_result(
        owner,
        restored && !changed && slot.matches(owner),
        "permissionIdentityChanged",
        true,
    );
    drop(replaced);
    drop(original);
    if restored && !changed && slot.matches(owner) {
        slot.resolve();
        Some(copy)
    } else {
        reject(owner, "permissionIdentityChanged", true);
        None
    }
}
pub(super) fn action_environment(owner: &Rc<RefCell<Owner>>) -> Option<ICoreWebView2Environment> {
    let slot = action_slot(owner, 8)?;
    let original = owner
        .borrow_mut()
        .instance
        .as_mut()
        .and_then(|i| i.environment.take());
    let Some(original) = original else {
        reject(owner, "permissionCleanupFailed", true);
        return None;
    };
    let copy = original.clone();
    let mut original = Some(original);
    let (replaced, restored) = {
        let mut o = owner.borrow_mut();
        if let Some(i) = o.instance.as_mut() {
            (i.environment.replace(original.take().unwrap()), true)
        } else {
            (None, false)
        }
    };
    let changed = replaced.is_some();
    native_result(
        owner,
        restored && !changed && slot.matches(owner),
        "permissionIdentityChanged",
        true,
    );
    drop(replaced);
    drop(original);
    if restored && !changed && slot.matches(owner) {
        slot.resolve();
        Some(copy)
    } else {
        reject(owner, "permissionIdentityChanged", true);
        None
    }
}
pub(super) fn remove(owner: &Rc<RefCell<Owner>>) {
    let Some(ctx) = context(owner) else { return };
    cutoff(owner, CutoffReason::RequiredRemovalAttempt);
    let active = Active::enter(&ctx.callbacks);
    for index in 0..10 {
        let action = with_state(owner, |s| {
            s.receivers.get_mut(index).and_then(|r| {
                if r.attempted {
                    return None;
                }
                let token = r.token?;
                r.attempted = true;
                if r.binding.receiver as usize != index || r.binding.token.get() != Some(token) {
                    s.facts.unknown = true;
                    s.facts.fail("permissionIdentityChanged");
                }
                Some((Rc::clone(&r.receiver), token))
            })
        })
        .flatten();
        if let Some((receiver, token)) = action {
            remove_entered(owner, index + 10);
            let result = unsafe { receiver.remove_DevToolsProtocolEventReceived(token) };
            let ok = result.is_ok();
            remove_returned(owner, index + 10, ok);
            if ok {
                with_state(owner, |s| s.receivers[index].token = None);
            }
            drop(result);
            drop(receiver);
            remove_qualified(owner, index + 10);
        }
    }
    for slot in [20usize, 21usize] {
        let action = with_state(owner, |s| {
            let attempted = if slot == 20 {
                &mut s.completed_remove
            } else {
                &mut s.resource_remove
            };
            let token = if slot == 20 { s.completed } else { s.resource };
            if *attempted {
                return None;
            }
            let token = token?;
            *attempted = true;
            s.core.as_ref().map(|c| (Rc::clone(c), token))
        })
        .flatten();
        if let Some((core, token)) = action {
            remove_entered(owner, slot);
            let result = if slot == 20 {
                unsafe { core.remove_NavigationCompleted(token) }
            } else {
                unsafe { core.remove_WebResourceRequested(token) }
            };
            let ok = result.is_ok();
            remove_returned(owner, slot, ok);
            if ok {
                with_state(owner, |s| {
                    if slot == 20 {
                        s.completed = None
                    } else {
                        s.resource = None
                    }
                });
            }
            drop(result);
            drop(core);
            remove_qualified(owner, slot);
        }
    }
    let filter = with_state(owner, |s| {
        if !s.filter || s.filter_remove {
            return None;
        }
        s.filter_remove = true;
        s.core22.as_ref().map(Rc::clone)
    })
    .flatten();
    if let Some(core22) = filter {
        remove_entered(owner, 22);
        let result = unsafe {
            core22.RemoveWebResourceRequestedFilterWithRequestSourceKinds(
                windows::core::w!("*"),
                COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
            )
        };
        let ok = result.is_ok();
        remove_returned(owner, 22, ok);
        if ok {
            with_state(owner, |s| s.filter = false);
        }
        drop(result);
        drop(core22);
        remove_qualified(owner, 22);
    }
    drop(active);
    publish(owner);
}
pub(super) fn removed(i: &Instance) -> bool {
    i.permission.as_ref().is_none_or(|s| {
        s.receivers.iter().all(|r| r.token.is_none())
            && s.completed.is_none()
            && s.resource.is_none()
            && !s.filter
    })
}
fn finite_disposal<A, B, C, D, E>(prepared: A, receivers: B, core22: C, environment: D, core: E) {
    drop(prepared);
    drop(receivers);
    drop(core22);
    drop(environment);
    drop(core);
}
pub(super) struct Controls {
    _core: Option<Rc<ICoreWebView2>>,
    _core22: Option<Rc<ICoreWebView2_22>>,
    _environment: Option<Rc<ICoreWebView2Environment>>,
    _receivers: Vec<ReceiverRegistration>,
    _prepared: Option<Prepared>,
}
impl Drop for Controls {
    fn drop(&mut self) {
        finite_disposal(
            self._prepared.take(),
            std::mem::take(&mut self._receivers),
            self._core22.take(),
            self._environment.take(),
            self._core.take(),
        );
    }
}
pub(super) fn take_controls(i: &mut Instance) -> Controls {
    let controls = if let Some(s) = i.permission.as_mut() {
        let boundary = Boundary {
            identity: s.identity,
            ms: s.timing.ms(Instant::now()).unwrap_or(u64::MAX),
        };
        s.protocol.cutoff(CutoffReason::ControlsDisposal, boundary);
        Controls {
            _core: s.core.take(),
            _core22: s.core22.take(),
            _environment: s.environment.take(),
            _receivers: std::mem::take(&mut s.receivers),
            _prepared: s.prepared.take(),
        }
    } else {
        Controls {
            _core: None,
            _core22: None,
            _environment: None,
            _receivers: Vec::new(),
            _prepared: None,
        }
    };
    carry_close(i);
    controls
}
pub(super) fn quiescent(i: &Instance) -> bool {
    if i.permission_selected.load(Ordering::SeqCst)
        && (i
            .permission_history
            .inventory
            .action_slots
            .load(Ordering::SeqCst)
            != 0
            || i.permission_history
                .inventory
                .slot_failed
                .load(Ordering::SeqCst))
    {
        return false;
    }
    i.permission.as_ref().is_none_or(|s| {
        let status = s.protocol.status();
        status.obligations == 0
            && status.command_handlers == 0
            && status.owned_registrations == 0
            && s.facts.coverage.owned_mask == 0
            && s.facts.coverage.uncertain_mask == 0
            && s.ctx.inventory.new_handlers.load(Ordering::SeqCst) == 0
            && s.ctx.inventory.command_handlers.load(Ordering::SeqCst) == 0
            && s.ctx.inventory.command_drop_bits.load(Ordering::SeqCst) == 0
            && s.ctx.inventory.text.live.load(Ordering::SeqCst) == 0
            && s.core.is_none()
            && s.core22.is_none()
            && s.environment.is_none()
            && s.receivers.is_empty()
            && s.prepared.is_none()
    })
}
pub struct Receipt {
    request: Code0WindowRequest,
    installation: Arc<AtomicBool>,
    inventory: Arc<NativeInventory>,
    facts: Facts,
    timing: Timing,
    partial: bool,
    callback_affinity: bool,
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
        self.request == previous.request && Arc::ptr_eq(&self.inventory, &previous.inventory)
    }
    pub fn callback_affinity(&self) -> bool {
        self.callback_affinity
    }
    pub fn facts(&self) -> &Facts {
        &self.facts
    }
    pub fn timing(&self) -> Timing {
        self.timing.clone()
    }
    pub fn matches_setup_failure(&self, request: &Code0WindowRequest, started: Instant) -> bool {
        self.partial
            && &self.request == request
            && self.timing.matches_start(started)
            && self.facts.failure.is_some()
    }
}
fn publish(owner: &Rc<RefCell<Owner>>) {
    let snapshot = {
        let mut o = owner.borrow_mut();
        let Some(i) = o.instance.as_mut() else { return };
        let Some(s) = i.permission.as_mut() else {
            return;
        };
        s.sync();
        s.facts.counts.receiver_adds = s.facts.coverage.add_acks[10..20]
            .iter()
            .sum::<u16>()
            .min(11);
        s.facts.counts.receiver_removes = s.facts.coverage.remove_acks[10..20]
            .iter()
            .sum::<u16>()
            .min(11);
        let active = i.callbacks.active.load(Ordering::SeqCst);
        let handlers = i.callbacks.handlers.load(Ordering::SeqCst);
        s.facts.inventory.active_guards = Some(active.min(65536) as u32);
        s.facts.inventory.aggregate_handlers = Some(handlers.min(65536) as u32);
        s.facts.inventory.handler_accounting_complete = Some(
            active <= 65535
                && handlers <= 65535
                && !s.ctx.inventory.affinity_cutoff.load(Ordering::SeqCst),
        );
        if active > 65535 || handlers > 65535 {
            s.facts.overflow = true;
            s.fail("permissionOverflow");
        }
        let document = u16::from(s.facts.inventory.document_record == Some(1));
        let native = u16::from(s.native_reserved);
        let completion = u16::from(s.completed_pending.iter().any(|p| *p));
        s.facts.inventory.document_record = Some(document);
        s.facts.inventory.native_record = Some(native);
        s.facts.inventory.completion_slot = Some(completion);
        s.facts.inventory.record_slots = Some(document + native + completion);
        s.facts.inventory.record_slots_peak = Some(
            s.facts
                .inventory
                .record_slots_peak
                .unwrap_or(0)
                .max(document + native + completion),
        );
        s.settle(Instant::now());
        s.sync();
        if let Some(at) = s.timing.first_close {
            latch_close(
                &mut s.timing,
                &mut i.permission_history.first_close,
                &mut i.failed,
                &mut i.closing,
                at,
            );
            if i.permission_history.failure.is_none() {
                i.permission_history.failure = s.facts.failure;
            }
        }
        let closing = if !s.first_close_published {
            s.timing.first_close.inspect(|_| {
                s.first_close_published = true;
            })
        } else {
            None
        };
        if s.facts.failure.is_some() {
            i.failed = true;
            i.closing = true;
        }
        (
            i.observer.clone(),
            Receipt {
                request: i.request.clone(),
                installation: Arc::clone(&s.ctx.installation),
                inventory: Arc::clone(&s.ctx.inventory),
                facts: s.facts.clone(),
                timing: s.timing.clone(),
                partial: false,
                callback_affinity: !i.callbacks.affinity_failed.load(Ordering::SeqCst),
            },
            closing,
        )
    };
    if let Some(observer) = snapshot.0 {
        if let Some(at) = snapshot.2 {
            observer(NativeSmokeObservation::PermissionClosing { at });
        }
        observer(NativeSmokeObservation::Permission(Box::new(snapshot.1)));
    }
}
pub(super) fn publish_final(owner: &Rc<RefCell<Owner>>) {
    let mut o = owner.borrow_mut();
    let Some(i) = o.instance.as_mut() else { return };
    let drained = inventory_empty(i) && i.released;
    let active = i.callbacks.active.load(Ordering::SeqCst) == 0;
    let handlers = i.callbacks.handlers.load(Ordering::SeqCst) == 0;
    let Some(s) = i.permission.as_mut() else {
        return;
    };
    if s.final_frozen {
        return;
    }
    s.final_frozen = true;
    let boundary = Boundary {
        identity: s.identity,
        ms: s.timing.ms(Instant::now()).unwrap_or(u64::MAX),
    };
    s.protocol.released(boundary);
    s.facts.timing.released_at_ms = Some(boundary.ms);
    counted(
        &mut s.facts.counts.final_receipts,
        1,
        &mut s.facts.overflow,
        &mut s.facts.failure,
    );
    s.facts.cleanup.released_ref_zero = Some(drained && active && handlers);
    s.facts.drain.obligations_empty = Some(s.protocol.status().obligations == 0);
    s.facts.drain.handlers_empty = Some(handlers);
    s.facts.drain.text_empty = Some(s.ctx.inventory.text.live.load(Ordering::SeqCst) == 0);
    s.facts.drain.receiver_tokens_removed =
        Some(s.facts.coverage.removed_mask & 0x000f_fc00 == 0x000f_fc00);
    s.facts.drain.resource_and_filter_removed =
        Some(s.facts.coverage.removed_mask & ((1 << 21) | (1 << 22)) == ((1 << 21) | (1 << 22)));
    s.facts.drain.controls_empty = Some(
        s.core.is_none() && s.core22.is_none() && s.environment.is_none() && s.receivers.is_empty(),
    );
    s.facts.ordering.qualified_observations_are_historical = Some(true);
    s.sync();
    if !drained || !active || !handlers {
        s.fail("permissionCleanupFailed");
    }
    carry_close(i);
    drop(o);
    publish(owner);
}
fn native_positive(n: &Native) -> bool {
    [
        n.sender_matched,
        n.request_installation_matched,
        n.kind_notifications,
        n.non_user,
        n.origin_matched,
        n.state_before_default,
        n.deny_succeeded,
        n.state_after_deny,
        n.original_timely,
        n.post_drop_matched,
        n.entered_before_observation_cutoff,
        n.captured_before_observation_cutoff,
        n.qualified_before_observation_cutoff,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn effect_positive(e: &Effect) -> bool {
    [
        e.success_hresult,
        e.closed_denied_result,
        e.same_ticket,
        e.same_frame_loader_context,
        e.source_matched,
        e.invoked_after_native_action,
        e.invoked_before_stop,
        e.captured_before_stop,
        e.qualified_before_stop,
        e.entered_before_observation_cutoff,
        e.captured_before_observation_cutoff,
        e.qualified_before_observation_cutoff,
        e.original_timely,
        e.close_timely,
        e.epoch_unchanged_through_qualification,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}

// The fixture remains the sole producer of Session, BrowserExited, Window
// Destroyed and post-run cleanup. Later native snapshots cannot erase them.
pub fn merge_native_facts(local: &mut Facts, native: &Facts) {
    let failures = local.counts.session_policy_failures;
    let reason = local.status.session_reason;
    let ordered = local.ordering.actual_policy_failure_after_action;
    let cleanup = local.cleanup.clone();
    let failure = local.failure;
    let unknown = local.unknown;
    let overflow = local.overflow;
    *local = native.clone();
    local.counts.session_policy_failures = failures;
    local.status.session_reason = reason;
    local.ordering.actual_policy_failure_after_action = ordered;
    local.cleanup.browser_process_exits = cleanup.browser_process_exits;
    local.cleanup.window_destroyed = cleanup.window_destroyed;
    local.cleanup.wake_joins = cleanup.wake_joins;
    local.cleanup.temp_dir_close_attempts = cleanup.temp_dir_close_attempts;
    local.cleanup.wake_shutdown_issued = cleanup.wake_shutdown_issued;
    local.cleanup.wake_join_succeeded = cleanup.wake_join_succeeded;
    local.cleanup.temp_dir_close_succeeded = cleanup.temp_dir_close_succeeded;
    local.cleanup.temp_dir_result_dropped = cleanup.temp_dir_result_dropped;
    local.unknown |= unknown;
    local.overflow |= overflow;
    if failure.is_some() {
        local.failure = failure;
    }
}
pub fn record_session_failure(
    facts: &mut Facts,
    reason: &Code0SessionFailure,
    request_matches: bool,
    action_before: bool,
) {
    counted(
        &mut facts.counts.session_policy_failures,
        1,
        &mut facts.overflow,
        &mut facts.failure,
    );
    if facts.status.session_reason.is_none() {
        facts.status.session_reason = Some(if *reason == Code0SessionFailure::PolicyRejected {
            "PolicyRejected"
        } else {
            "other"
        });
        facts.ordering.actual_policy_failure_after_action = Some(request_matches && action_before);
    }
    if facts.counts.session_policy_failures != 1
        || facts.status.session_reason != Some("PolicyRejected")
        || !request_matches
        || !action_before
    {
        facts.fail("permissionIdentityChanged");
    }
}
pub fn record_browser_exit(facts: &mut Facts) {
    counted(
        &mut facts.cleanup.browser_process_exits,
        1,
        &mut facts.overflow,
        &mut facts.failure,
    );
}
pub fn record_destroyed(facts: &mut Facts) {
    counted(
        &mut facts.cleanup.window_destroyed,
        1,
        &mut facts.overflow,
        &mut facts.failure,
    );
}
impl Timing {
    pub fn record_final(&self, facts: &mut Facts, now: Instant, adopted: Option<Instant>) {
        let same = adopted.is_some() && adopted == self.first_close;
        facts.timing.first_close_at_ms = self.first_close.and_then(|at| self.ms(at));
        facts.timing.final_at_ms = self.ms(now);
        facts.timing.cleanup_elapsed_ms = self
            .first_close
            .and_then(|at| now.checked_duration_since(at))
            .and_then(|d| u64::try_from(d.as_millis()).ok());
        facts.timing.same_original_clock = Some(self.ms(now).is_some());
        facts.timing.first_close_adopted = Some(same);
        facts.timing.final_within_close_deadline = Some(self.close_before(now));
        if !same || !self.close_before(now) || facts.timing.final_at_ms.is_none() {
            facts.fail("permissionCleanupFailed");
        }
    }
}
impl Facts {
    pub fn qualified_action_history(&self) -> bool {
        self.counts.action_receipts >= 1
            && self.status.native_qualified == Some(true)
            && native_positive(&self.native)
    }
    pub fn target_positive(&self) -> bool {
        self.failure.is_none()
            && !self.unknown
            && !self.overflow
            && self.counts.action_receipts == 1
            && self.counts.permission_entries == 1
            && self.status.native_qualified == Some(true)
            && native_positive(&self.native)
    }
    pub fn native_positive(&self) -> bool {
        let c = &self.counts;
        let cov = &self.coverage;
        let cmd = &self.commands;
        let inv = &self.inventory;
        let obs = &self.observations;
        let counts = [
            c.navigation,
            c.main_responses,
            c.permission_entries,
            c.kind_reads,
            c.gesture_reads,
            c.origin_reads,
            c.state_before_reads,
            c.deny_attempts,
            c.deny_returns,
            c.deny_successes,
            c.state_after_reads,
            c.action_receipts,
            c.evaluate_submissions,
            c.evaluate_completions,
            c.effect_receipts,
            c.session_policy_failures,
            c.final_receipts,
        ];
        self.target_positive()
            && effect_positive(&self.effect)
            && self.status.effect_qualified == Some(true)
            && self.status.effect_result == Some("denied")
            && self.setup.hooks1023 == 1023
            && [
                self.setup.settings_ack,
                self.setup.responder_ready,
                self.setup.receivers_ready,
                self.setup.domains_ack,
                self.setup.document_bound,
                self.setup.lifecycle_assembly_complete,
                self.setup.blank_branch_closed,
                self.setup.observation_coverage_intact_at_arm,
                self.setup.original_deadline_bound,
            ]
            .into_iter()
            .all(|v| v == Some(true))
            && counts.into_iter().all(|v| v == 1)
            && c.unexpected_requests == 0
            && c.receiver_adds == 10
            && c.receiver_removes == 10
            && [
                c.command_issued,
                c.command_returned,
                c.command_completed,
                c.command_consumed,
            ]
            .into_iter()
            .all(|v| v == 4)
            && cov.required_mask == ALL
            && cov.added_mask == ALL
            && cov.remove_attempt_mask == ALL
            && cov.removed_mask == ALL
            && cov.uncertain_mask == 0
            && cov.owned_mask == 0
            && [
                &cov.add_attempts,
                &cov.add_acks,
                &cov.remove_attempts,
                &cov.remove_acks,
            ]
            .into_iter()
            .all(|a| a.iter().all(|n| *n == 1))
            && [&cov.add_returns, &cov.remove_returns]
                .into_iter()
                .all(|a| a.iter().all(|v| *v == Some(true)))
            && [
                &cmd.issued,
                &cmd.returned,
                &cmd.completed,
                &cmd.consumed,
                &cmd.handler_drops,
            ]
            .into_iter()
            .all(|a| a.iter().all(|n| *n == 1))
            && [
                &cmd.submission_succeeded,
                &cmd.completion_succeeded,
                &cmd.qualified,
            ]
            .into_iter()
            .all(|a| a.iter().all(|v| *v == Some(true)))
            && cmd.values
                == [
                    Some("emptyAck"),
                    Some("emptyAck"),
                    Some("frameTree"),
                    Some("denied"),
                ]
            && cmd.obligations == Some(0)
            && cmd.handlers == Some(0)
            && cmd.in_flight == Some(0)
            && matches!(obs.blank_branch, Some("ZeroObserved" | "Observed"))
            && (4..=16).contains(&obs.bootstrap_deliveries)
            && obs.post_arm_pre_cutoff == 0
            && !obs.counter_overflow
            && u32::from(obs.deliveries)
                == u32::from(obs.bootstrap_deliveries) + u32::from(obs.disposal_deliveries)
            && obs.by_receiver.iter().map(|n| u32::from(*n)).sum::<u32>()
                == u32::from(obs.deliveries)
            && inv.text_bytes == Some(0)
            && inv.text_peak_bytes.is_some_and(|n| (1..=4096).contains(&n))
            && inv.text_reserve_failures == 0
            && inv.text_accounting_complete == Some(true)
            && inv.active_guards == Some(0)
            && inv.aggregate_handlers == Some(0)
            && inv.new_event_handlers == Some(0)
            && inv.command_handlers == Some(0)
            && inv.handler_accounting_complete == Some(true)
            && inv.completion_slot == Some(0)
            && inv.document_record.is_some_and(|n| n <= 1)
            && inv.native_record.is_some_and(|n| n <= 1)
            && inv.record_slots
                == inv
                    .document_record
                    .zip(inv.native_record)
                    .map(|(d, n)| d + n)
            && inv
                .record_slots_peak
                .is_some_and(|n| (2..=3).contains(&n) && inv.record_slots.is_some_and(|r| r <= n))
            && [
                self.ordering.permanent_revocation_at_refusal,
                self.ordering.first_close_at_refusal,
                self.ordering.no_action_reauthorized,
                self.ordering.no_wait_for_effect,
                self.ordering.actual_policy_failure_after_action,
                self.ordering.cutoff_latched,
                self.ordering.cutoff_before_any_required_removal,
                self.ordering.qualified_observations_are_historical,
                self.ordering.physical_close_observed,
                self.drain.obligations_empty,
                self.drain.handlers_empty,
                self.drain.text_empty,
                self.drain.receiver_tokens_removed,
                self.drain.resource_and_filter_removed,
                self.drain.controls_empty,
            ]
            .into_iter()
            .all(|v| v == Some(true))
            && self.status.protocol_phase == Some("Released")
            && self.status.protocol_failure.is_none()
            && self.status.observation_epoch == Some(1)
            && matches!(
                self.status.cutoff_reason,
                Some(
                    "RequiredRemovalAttempt"
                        | "ControlsDisposal"
                        | "StopEntry"
                        | "ControllerCloseEntry"
                )
            )
            && self.status.session_reason == Some("PolicyRejected")
            && self.status.navigation_succeeded == Some(true)
            && self.status.main_response_succeeded == Some(true)
            && self.cleanup.browser_process_exits == 1
            && self.cleanup.window_destroyed == 1
            && self.cleanup.released_ref_zero == Some(true)
    }
    pub fn positive(&self) -> bool {
        self.native_positive()
            && self.phase == "Closed"
            && self.status.terminal_result == Some("succeeded")
            && self.cleanup.wake_joins == 1
            && self.cleanup.temp_dir_close_attempts == 1
            && [
                self.cleanup.wake_shutdown_issued,
                self.cleanup.wake_join_succeeded,
                self.cleanup.temp_dir_close_succeeded,
                self.cleanup.temp_dir_result_dropped,
                self.timing.final_within_close_deadline,
                self.timing.same_original_clock,
                self.timing.first_close_adopted,
            ]
            .into_iter()
            .all(|v| v == Some(true))
            && self.timing.final_at_ms.is_some()
            && self.timing.cleanup_elapsed_ms.is_some_and(|ms| ms < 10_000)
    }
}
fn setup_failure_facts(
    coverage: &Coverage,
    hooks: u16,
    settings: Option<bool>,
    active: u32,
    handlers: u32,
) -> Facts {
    let mut facts = Facts {
        coverage: coverage.clone(),
        ..Facts::default()
    };
    facts.setup.hooks1023 = hooks;
    facts.setup.settings_ack = settings;
    facts.inventory.active_guards = Some(active);
    facts.inventory.aggregate_handlers = Some(handlers);
    facts.overflow = coverage.overflow || active > 65535 || handlers > 65535;
    facts.failure = Some("permissionSetupRejected");
    facts
}
pub(super) fn setup_failure(
    owner: &Rc<RefCell<Owner>>,
    request: &Code0WindowRequest,
    started: Instant,
) -> Result<Receipt, Code0SessionFailure> {
    if !on_ui(owner) {
        return Err(Code0SessionFailure::PolicyRejected);
    }
    let o = owner.borrow();
    if !o
        .permission_intent
        .as_ref()
        .is_some_and(|intent| intent.matches(request, started))
    {
        return Err(Code0SessionFailure::PolicyRejected);
    }
    let i = o
        .instance
        .as_ref()
        .filter(|i| &i.request == request)
        .ok_or(Code0SessionFailure::PolicyRejected)?;
    // Read-only sealed partial observation; it never creates Protocol, ticket,
    // registration or permission authority and can never become a positive.
    let (mut facts, timing, inventory) = if let Some(s) = i.permission.as_ref() {
        (
            s.facts.clone(),
            s.timing.clone(),
            Arc::clone(&s.ctx.inventory),
        )
    } else {
        (
            setup_failure_facts(
                &i.permission_history.facts,
                i.refusal_registry.installed_mask(),
                i.permission_history.settings,
                i.callbacks.active.load(Ordering::SeqCst).min(65536) as u32,
                i.callbacks.handlers.load(Ordering::SeqCst).min(65536) as u32,
            ),
            Timing::new(started),
            Arc::clone(&i.permission_history.inventory),
        )
    };
    let mut timing = timing;
    if inventory.clock.adopt(&mut timing).is_err() {
        facts.unknown = true;
    }
    if i.permission.is_none() {
        if let Some(at) = i.permission_history.first_close {
            timing.close(at);
        }
    }
    facts.unknown |= i.permission_history.unknown;
    facts.fail("permissionSetupRejected");
    if i.permission.is_none() {
        facts.failure = i
            .permission_history
            .failure
            .or_else(|| {
                i.callbacks
                    .affinity_failed
                    .load(Ordering::SeqCst)
                    .then_some("uiAffinityFailed")
            })
            .or(facts.failure);
    }
    Ok(Receipt {
        request: request.clone(),
        installation: Arc::clone(&i.refusal_registry.unknown),
        inventory,
        facts,
        timing,
        partial: true,
        callback_affinity: !i.callbacks.affinity_failed.load(Ordering::SeqCst),
    })
}

fn take_once(reserved: &mut bool) -> bool {
    let first = !*reserved;
    *reserved = true;
    first
}
fn effect_failure(native: bool, hresult: bool, value: &str) -> Option<&'static str> {
    if !native {
        Some(if value == "denied" {
            "permissionNativeTargetMissing"
        } else if matches!(
            value,
            "precondition" | "default" | "granted" | "documentChanged" | "invalid"
        ) {
            "permissionPreflightFailed"
        } else {
            "permissionProtocolRejected"
        })
    } else if !hresult {
        Some("permissionProtocolRejected")
    } else if value != "denied" {
        Some("permissionPreflightFailed")
    } else {
        None
    }
}

// Stack-only order seam. This stores no callback, queue or authority. A later
// valid-UI delivery still runs its restriction after an earlier affinity loss.
pub(super) fn selected_ui_entry(
    failure: &AtomicBool,
    guard: impl FnOnce() -> bool,
    native: impl FnOnce(),
) -> bool {
    if !guard() {
        failure.store(true, Ordering::SeqCst);
        return false;
    }
    native();
    !failure.load(Ordering::SeqCst)
}
#[windows::core::implement(ICoreWebView2DevToolsProtocolEventReceivedEventHandler, Agile = false)]
struct LifecycleHandler {
    ctx: Context,
    receiver: Receiver,
    binding: Rc<ReceiverBinding>,
    _lifetime: EventLifetime,
}
impl ICoreWebView2DevToolsProtocolEventReceivedEventHandler_Impl for LifecycleHandler_Impl {
    fn Invoke(
        &self,
        sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2DevToolsProtocolEventReceivedEventArgs>,
    ) -> windows::core::Result<()> {
        let active = Active::enter(&self.ctx.callbacks);
        self.ctx
            .inventory
            .lifecycle_entries
            .enter(self.receiver as usize);
        selected_ui_entry(
            &self.ctx.callbacks.affinity_failed,
            || self.ctx.ui(),
            || {
                let candidate = delivery(
                    &self.ctx,
                    &self.binding,
                    self.receiver,
                    sender.as_ref(),
                    args.as_ref(),
                );
                // ABI Ref values are external borrows, not owned clones. All Args2,
                // PWSTR/error/text/interface temporaries acquired above ended there.
                if let Some(candidate) = candidate {
                    finish_delivery(&self.ctx, candidate);
                }
            },
        );
        drop(active);
        Ok(())
    }
}
#[windows::core::implement(ICoreWebView2NavigationCompletedEventHandler, Agile = false)]
struct CompletedHandler {
    ctx: Context,
    _lifetime: EventLifetime,
}
impl ICoreWebView2NavigationCompletedEventHandler_Impl for CompletedHandler_Impl {
    fn Invoke(
        &self,
        sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2NavigationCompletedEventArgs>,
    ) -> windows::core::Result<()> {
        let active = Active::enter(&self.ctx.callbacks);
        selected_ui_entry(
            &self.ctx.callbacks.affinity_failed,
            || self.ctx.ui(),
            || completed_borrowed(&self.ctx, sender.as_ref(), args.as_ref()),
        );
        drop(active);
        Ok(())
    }
}
#[windows::core::implement(ICoreWebView2WebResourceRequestedEventHandler, Agile = false)]
struct ResourceHandler {
    ctx: Context,
    _lifetime: EventLifetime,
}
impl ICoreWebView2WebResourceRequestedEventHandler_Impl for ResourceHandler_Impl {
    fn Invoke(
        &self,
        sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2WebResourceRequestedEventArgs>,
    ) -> windows::core::Result<()> {
        let active = Active::enter(&self.ctx.callbacks);
        selected_ui_entry(
            &self.ctx.callbacks.affinity_failed,
            || self.ctx.ui(),
            || resource_borrowed(&self.ctx, sender.as_ref(), args.as_ref()),
        );
        drop(active);
        Ok(())
    }
}
pub(super) struct S2Guard {
    pub(super) owner: Weak<RefCell<Owner>>,
    pub(super) request: Code0WindowRequest,
    pub(super) generation: u64,
    pub(super) ui: ThreadId,
    pub(super) callbacks: Arc<Callbacks>,
    pub(super) unknown: Arc<AtomicBool>,
    pub(super) clock: Arc<FailureClock>,
    pub(super) _lifetime: HandlerLifetime,
}
impl S2Guard {
    fn owner(&self) -> Option<(Rc<RefCell<Owner>>, bool)> {
        let owner = self.owner.upgrade()?;
        let same = {
            let o = owner.borrow();
            !o.unmatched
                && o.instance.as_ref().is_some_and(|i| {
                    i.request == self.request
                        && i.refusal_registry.http_generation() == self.generation
                        && Arc::ptr_eq(&i.callbacks, &self.callbacks)
                        && Arc::ptr_eq(&i.refusal_registry.unknown, &self.unknown)
                })
        };
        if !same {
            reject(&owner, "permissionIdentityChanged", true);
        }
        Some((owner, same))
    }
    fn failed_action(&self, owner: Option<&Rc<RefCell<Owner>>>, ok: bool) {
        if !ok {
            self.unknown.store(true, Ordering::SeqCst);
            if let Some(owner) = owner {
                reject(owner, "permissionUnknown", true);
                if let Some(i) = owner.borrow_mut().instance.as_mut() {
                    i.cleanup_blocked = true;
                }
            }
        }
    }
}
#[windows::core::implement(ICoreWebView2NavigationStartingEventHandler, Agile = false)]
struct S2Navigation {
    guard: S2Guard,
    child: bool,
}
impl ICoreWebView2NavigationStartingEventHandler_Impl for S2Navigation_Impl {
    fn Invoke(
        &self,
        sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2NavigationStartingEventArgs>,
    ) -> windows::core::Result<()> {
        let g = &self.guard;
        let active = Active::enter(&g.callbacks);
        selected_ui_entry(
            &g.callbacks.affinity_failed,
            || affinity_entry(g.ui, &g.callbacks, &g.clock, 8710),
            || {
                let owner = g.owner();
                if !self.child {
                    if let Some((owner, same)) = owner.as_ref() {
                        top_borrowed(owner, sender.as_ref(), args.as_ref(), *same);
                        return;
                    }
                }
                if let Some((owner, _)) = owner.as_ref() {
                    unexpected(owner);
                }
                let ok = args.as_ref().is_some_and(|args| {
                    let result = unsafe { args.SetCancel(true) };
                    let ok = result.is_ok();
                    drop(result);
                    ok
                });
                g.failed_action(owner.as_ref().map(|(owner, _)| owner), ok);
            },
        );
        drop(active);
        Ok(())
    }
}
pub(super) fn s2_navigation(
    guard: S2Guard,
    child: bool,
) -> ICoreWebView2NavigationStartingEventHandler {
    S2Navigation { guard, child }.into()
}
#[windows::core::implement(ICoreWebView2PermissionRequestedEventHandler, Agile = false)]
struct S2Permission {
    guard: S2Guard,
}
impl ICoreWebView2PermissionRequestedEventHandler_Impl for S2Permission_Impl {
    fn Invoke(
        &self,
        sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2PermissionRequestedEventArgs>,
    ) -> windows::core::Result<()> {
        let g = &self.guard;
        let active = Active::enter(&g.callbacks);
        let entry = Instant::now();
        selected_ui_entry(
            &g.callbacks.affinity_failed,
            || affinity_entry(g.ui, &g.callbacks, &g.clock, 8711),
            || {
                if let Some((owner, same)) = g.owner() {
                    native_borrowed(&owner, sender.as_ref(), args.as_ref(), same, entry);
                } else {
                    let ok = args.as_ref().is_some_and(|args| {
                        let result = unsafe { args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY) };
                        let ok = result.is_ok();
                        drop(result);
                        let mut state = COREWEBVIEW2_PERMISSION_STATE(-1);
                        let result = unsafe { args.State(&mut state) };
                        drop(result);
                        ok
                    });
                    g.failed_action(None, ok);
                    g.unknown.store(true, Ordering::SeqCst);
                }
            },
        );
        drop(active);
        Ok(())
    }
}
pub(super) fn s2_permission(guard: S2Guard) -> ICoreWebView2PermissionRequestedEventHandler {
    S2Permission { guard }.into()
}

#[windows::core::implement(ICoreWebView2LaunchingExternalUriSchemeEventHandler, Agile = false)]
struct S2External {
    guard: S2Guard,
}
impl ICoreWebView2LaunchingExternalUriSchemeEventHandler_Impl for S2External_Impl {
    fn Invoke(
        &self,
        _sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2LaunchingExternalUriSchemeEventArgs>,
    ) -> windows::core::Result<()> {
        let g = &self.guard;
        let active = Active::enter(&g.callbacks);
        selected_ui_entry(
            &g.callbacks.affinity_failed,
            || affinity_entry(g.ui, &g.callbacks, &g.clock, 8712),
            || {
                let owner = g.owner();
                if let Some((owner, _)) = owner.as_ref() {
                    unexpected(owner);
                }
                let ok = args.as_ref().is_some_and(|args| {
                    let result = unsafe { args.SetCancel(true) };
                    let ok = result.is_ok();
                    drop(result);
                    ok
                });
                g.failed_action(owner.as_ref().map(|(owner, _)| owner), ok);
            },
        );
        drop(active);
        Ok(())
    }
}
pub(super) fn s2_external(guard: S2Guard) -> ICoreWebView2LaunchingExternalUriSchemeEventHandler {
    S2External { guard }.into()
}

#[windows::core::implement(ICoreWebView2NewWindowRequestedEventHandler, Agile = false)]
struct S2Popup {
    guard: S2Guard,
}
impl ICoreWebView2NewWindowRequestedEventHandler_Impl for S2Popup_Impl {
    fn Invoke(
        &self,
        _sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2NewWindowRequestedEventArgs>,
    ) -> windows::core::Result<()> {
        let g = &self.guard;
        let active = Active::enter(&g.callbacks);
        selected_ui_entry(
            &g.callbacks.affinity_failed,
            || affinity_entry(g.ui, &g.callbacks, &g.clock, 8712),
            || {
                let owner = g.owner();
                if let Some((owner, _)) = owner.as_ref() {
                    unexpected(owner);
                }
                let ok = args.as_ref().is_some_and(|args| {
                    let result = unsafe { args.SetHandled(true) };
                    let ok = result.is_ok();
                    drop(result);
                    ok
                });
                g.failed_action(owner.as_ref().map(|(owner, _)| owner), ok);
            },
        );
        drop(active);
        Ok(())
    }
}
pub(super) fn s2_popup(guard: S2Guard) -> ICoreWebView2NewWindowRequestedEventHandler {
    S2Popup { guard }.into()
}

#[windows::core::implement(ICoreWebView2DownloadStartingEventHandler, Agile = false)]
struct S2Download {
    guard: S2Guard,
}
impl ICoreWebView2DownloadStartingEventHandler_Impl for S2Download_Impl {
    fn Invoke(
        &self,
        _sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2DownloadStartingEventArgs>,
    ) -> windows::core::Result<()> {
        let g = &self.guard;
        let active = Active::enter(&g.callbacks);
        selected_ui_entry(
            &g.callbacks.affinity_failed,
            || affinity_entry(g.ui, &g.callbacks, &g.clock, 8712),
            || {
                let owner = g.owner();
                if let Some((owner, _)) = owner.as_ref() {
                    unexpected(owner);
                }
                let ok = args.as_ref().is_some_and(|args| {
                    let cancel = unsafe { args.SetCancel(true) };
                    let handled = unsafe { args.SetHandled(true) };
                    let ok = cancel.is_ok() && handled.is_ok();
                    drop(handled);
                    drop(cancel);
                    ok
                });
                g.failed_action(owner.as_ref().map(|(owner, _)| owner), ok);
            },
        );
        drop(active);
        Ok(())
    }
}
pub(super) fn s2_download(guard: S2Guard) -> ICoreWebView2DownloadStartingEventHandler {
    S2Download { guard }.into()
}

#[windows::core::implement(ICoreWebView2ScriptDialogOpeningEventHandler, Agile = false)]
struct S2Dialog {
    guard: S2Guard,
}
impl ICoreWebView2ScriptDialogOpeningEventHandler_Impl for S2Dialog_Impl {
    fn Invoke(
        &self,
        _sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2ScriptDialogOpeningEventArgs>,
    ) -> windows::core::Result<()> {
        let g = &self.guard;
        let active = Active::enter(&g.callbacks);
        selected_ui_entry(
            &g.callbacks.affinity_failed,
            || affinity_entry(g.ui, &g.callbacks, &g.clock, 8712),
            || {
                let owner = g.owner();
                if let Some((owner, _)) = owner.as_ref() {
                    unexpected(owner);
                }
                let ok = args.as_ref().is_some_and(|args| {
                    let mut kind = COREWEBVIEW2_SCRIPT_DIALOG_KIND(-1);
                    let result = unsafe { args.Kind(&mut kind) };
                    let ok = result.is_ok();
                    drop(result);
                    ok
                });
                g.failed_action(owner.as_ref().map(|(owner, _)| owner), ok);
            },
        );
        drop(active);
        Ok(())
    }
}
pub(super) fn s2_dialog(guard: S2Guard) -> ICoreWebView2ScriptDialogOpeningEventHandler {
    S2Dialog { guard }.into()
}

#[windows::core::implement(ICoreWebView2BasicAuthenticationRequestedEventHandler, Agile = false)]
struct S2Authentication {
    guard: S2Guard,
}
impl ICoreWebView2BasicAuthenticationRequestedEventHandler_Impl for S2Authentication_Impl {
    fn Invoke(
        &self,
        _sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2BasicAuthenticationRequestedEventArgs>,
    ) -> windows::core::Result<()> {
        let g = &self.guard;
        let active = Active::enter(&g.callbacks);
        selected_ui_entry(
            &g.callbacks.affinity_failed,
            || affinity_entry(g.ui, &g.callbacks, &g.clock, 8712),
            || {
                let owner = g.owner();
                if let Some((owner, _)) = owner.as_ref() {
                    unexpected(owner);
                }
                let ok = args.as_ref().is_some_and(|args| {
                    let result = unsafe { args.SetCancel(true) };
                    let ok = result.is_ok();
                    drop(result);
                    ok
                });
                g.failed_action(owner.as_ref().map(|(owner, _)| owner), ok);
            },
        );
        drop(active);
        Ok(())
    }
}
pub(super) fn s2_authentication(
    guard: S2Guard,
) -> ICoreWebView2BasicAuthenticationRequestedEventHandler {
    S2Authentication { guard }.into()
}

#[windows::core::implement(ICoreWebView2ClientCertificateRequestedEventHandler, Agile = false)]
struct S2Certificate {
    guard: S2Guard,
}
impl ICoreWebView2ClientCertificateRequestedEventHandler_Impl for S2Certificate_Impl {
    fn Invoke(
        &self,
        _sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2ClientCertificateRequestedEventArgs>,
    ) -> windows::core::Result<()> {
        let g = &self.guard;
        let active = Active::enter(&g.callbacks);
        selected_ui_entry(
            &g.callbacks.affinity_failed,
            || affinity_entry(g.ui, &g.callbacks, &g.clock, 8712),
            || {
                let owner = g.owner();
                if let Some((owner, _)) = owner.as_ref() {
                    unexpected(owner);
                }
                let ok = args.as_ref().is_some_and(|args| {
                    let cancel = unsafe { args.SetCancel(true) };
                    let handled = unsafe { args.SetHandled(true) };
                    let ok = cancel.is_ok() && handled.is_ok();
                    drop(handled);
                    drop(cancel);
                    ok
                });
                g.failed_action(owner.as_ref().map(|(owner, _)| owner), ok);
            },
        );
        drop(active);
        Ok(())
    }
}
pub(super) fn s2_certificate(
    guard: S2Guard,
) -> ICoreWebView2ClientCertificateRequestedEventHandler {
    S2Certificate { guard }.into()
}

#[windows::core::implement(ICoreWebView2ServerCertificateErrorDetectedEventHandler, Agile = false)]
struct S2Tls {
    guard: S2Guard,
}
impl ICoreWebView2ServerCertificateErrorDetectedEventHandler_Impl for S2Tls_Impl {
    fn Invoke(
        &self,
        _sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2ServerCertificateErrorDetectedEventArgs>,
    ) -> windows::core::Result<()> {
        let g = &self.guard;
        let active = Active::enter(&g.callbacks);
        selected_ui_entry(
            &g.callbacks.affinity_failed,
            || affinity_entry(g.ui, &g.callbacks, &g.clock, 8712),
            || {
                let owner = g.owner();
                if let Some((owner, _)) = owner.as_ref() {
                    unexpected(owner);
                }
                let ok = args.as_ref().is_some_and(|args| {
                    let result = unsafe {
                        args.SetAction(COREWEBVIEW2_SERVER_CERTIFICATE_ERROR_ACTION_CANCEL)
                    };
                    let ok = result.is_ok();
                    drop(result);
                    ok
                });
                g.failed_action(owner.as_ref().map(|(owner, _)| owner), ok);
            },
        );
        drop(active);
        Ok(())
    }
}
pub(super) fn s2_tls(guard: S2Guard) -> ICoreWebView2ServerCertificateErrorDetectedEventHandler {
    S2Tls { guard }.into()
}

#[windows::core::implement(ICoreWebView2BrowserProcessExitedEventHandler, Agile = false)]
struct ExitHandler {
    owner: Weak<RefCell<Owner>>,
    request: Code0WindowRequest,
    callbacks: Arc<Callbacks>,
    ui: ThreadId,
    clock: Arc<FailureClock>,
    _lifetime: HandlerLifetime,
}
impl ICoreWebView2BrowserProcessExitedEventHandler_Impl for ExitHandler_Impl {
    fn Invoke(
        &self,
        sender: windows::core::Ref<'_, ICoreWebView2Environment>,
        args: windows::core::Ref<'_, ICoreWebView2BrowserProcessExitedEventArgs>,
    ) -> windows::core::Result<()> {
        let active = Active::enter(&self.callbacks);
        selected_ui_entry(
            &self.callbacks.affinity_failed,
            || affinity_entry(self.ui, &self.callbacks, &self.clock, 8714),
            || {
                if let Some(owner) = self.owner.upgrade() {
                    let environment = action_environment(&owner);
                    let mut pid = 0;
                    let mut kind = COREWEBVIEW2_BROWSER_PROCESS_EXIT_KIND(-1);
                    let observed = environment
                        .as_ref()
                        .zip(sender.as_ref())
                        .zip(args.as_ref())
                        .is_some_and(|((environment, sender), args)| {
                            let expected = environment.cast::<IUnknown>();
                            let actual = sender.cast::<IUnknown>();
                            let identity = expected
                                .as_ref()
                                .ok()
                                .zip(actual.as_ref().ok())
                                .is_some_and(|(a, b)| a.as_raw() == b.as_raw());
                            drop(actual);
                            drop(expected);
                            let p = unsafe { args.BrowserProcessId(&mut pid) };
                            let k = unsafe { args.BrowserProcessExitKind(&mut kind) };
                            let good = identity && p.is_ok() && k.is_ok();
                            drop(k);
                            drop(p);
                            good
                        });
                    drop(environment);
                    let observer = {
                        let mut o = owner.borrow_mut();
                        let matching = !o.unmatched;
                        let i = o.instance.as_mut();
                        if let Some(i) = i {
                            if matching
                                && observed
                                && i.request == self.request
                                && Arc::ptr_eq(&i.callbacks, &self.callbacks)
                                && !i.released
                                && !i.callbacks.affinity_failed.load(Ordering::SeqCst)
                                && i.closing
                                && matches!(i.browser_exit,BrowserExitState::Armed(expected) if expected==pid&&pid!=0)
                                && kind == COREWEBVIEW2_BROWSER_PROCESS_EXIT_KIND_NORMAL
                            {
                                i.browser_exit = BrowserExitState::Observed;
                                i.observer.clone()
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };
                    if let Some(observer) = observer {
                        observer(NativeSmokeObservation::BrowserProcessExited);
                    } else {
                        reject(&owner, "permissionCleanupFailed", true);
                        browser_exit_unknown(&owner, &self.request);
                    }
                }
            },
        );
        drop(active);
        Ok(())
    }
}
pub(super) fn browser_exit_handler(
    owner: Weak<RefCell<Owner>>,
    request: Code0WindowRequest,
    callbacks: Arc<Callbacks>,
    ui: ThreadId,
    lifetime: HandlerLifetime,
    clock: Arc<FailureClock>,
) -> ICoreWebView2BrowserProcessExitedEventHandler {
    ExitHandler {
        owner,
        request,
        callbacks,
        ui,
        clock,
        _lifetime: lifetime,
    }
    .into()
}
#[windows::core::implement(ICoreWebView2WebMessageReceivedEventHandler, Agile = false)]
struct MessageHandler {
    owner: Weak<RefCell<Owner>>,
    request: Code0WindowRequest,
    callbacks: Arc<Callbacks>,
    ui: ThreadId,
    clock: Arc<FailureClock>,
    _lifetime: HandlerLifetime,
}
impl ICoreWebView2WebMessageReceivedEventHandler_Impl for MessageHandler_Impl {
    fn Invoke(
        &self,
        _sender: windows::core::Ref<'_, ICoreWebView2>,
        _args: windows::core::Ref<'_, ICoreWebView2WebMessageReceivedEventArgs>,
    ) -> windows::core::Result<()> {
        let active = Active::enter(&self.callbacks);
        selected_ui_entry(
            &self.callbacks.affinity_failed,
            || affinity_entry(self.ui, &self.callbacks, &self.clock, 8713),
            || {
                if let Some(owner) = self.owner.upgrade() {
                    let same = owner.borrow().instance.as_ref().is_some_and(|i| {
                        i.request == self.request && Arc::ptr_eq(&i.callbacks, &self.callbacks)
                    });
                    reject(
                        &owner,
                        if same {
                            "permissionProtocolRejected"
                        } else {
                            "permissionIdentityChanged"
                        },
                        !same,
                    );
                }
            },
        );
        drop(active);
        Ok(())
    }
}
pub(super) fn message_handler(
    owner: Weak<RefCell<Owner>>,
    request: Code0WindowRequest,
    callbacks: Arc<Callbacks>,
    ui: ThreadId,
    lifetime: HandlerLifetime,
    clock: Arc<FailureClock>,
) -> ICoreWebView2WebMessageReceivedEventHandler {
    MessageHandler {
        owner,
        request,
        callbacks,
        ui,
        clock,
        _lifetime: lifetime,
    }
    .into()
}

#[cfg(test)]
mod permission_native_tests {
    use super::*;

    // Mutation: a known forbidden entry stays authoritative during later native/args Drop.
    #[test]
    fn forbidden_lifecycle_phase_rejects_before_any_capture_or_reentrant_drop() {
        for phase in [
            Phase::Registering,
            Phase::Armed,
            Phase::Issued,
            Phase::Closing,
        ] {
            assert!(forbidden_lifecycle_entry(phase, false));
            assert!(!forbidden_lifecycle_entry(phase, true));
        }
        assert!(!forbidden_lifecycle_entry(Phase::Enabling, false));
        assert!(!forbidden_lifecycle_entry(Phase::Navigating, false));
        let identity = Identity {
            request: 1,
            generation: 2,
            installation: 3,
            core: 4,
        };
        let boundary = Boundary { identity, ms: 1 };
        let mut protocol = Protocol::new(identity, 0).unwrap();
        let status = protocol.status();
        assert!(forbidden_lifecycle_entry(
            status.phase,
            status.cutoff.is_some()
        ));
        assert!(
            protocol
                .lifecycle(Lifecycle::Unsupported(Receiver::ALL[0]), boundary)
                .is_err()
        );
        // The first owned/native Drop would be after this failure is already sticky.
        assert!(protocol.status().failure.is_some());
        assert!(protocol.native_entered(boundary).is_err());
        assert_eq!(protocol.status().obligations, 0);
    }

    // Mutation: a later effect/cleanup failure erases the earlier actual qualified action.
    #[test]
    fn qualified_action_history_survives_later_failure_without_making_a_pass() {
        let mut facts = Facts::default();
        facts.counts.action_receipts = 1;
        facts.status.native_qualified = Some(true);
        facts.native = Native {
            sender_matched: Some(true),
            request_installation_matched: Some(true),
            kind_notifications: Some(true),
            non_user: Some(true),
            origin_matched: Some(true),
            state_before_default: Some(true),
            deny_succeeded: Some(true),
            state_after_deny: Some(true),
            original_timely: Some(true),
            post_drop_matched: Some(true),
            entered_before_observation_cutoff: Some(true),
            captured_before_observation_cutoff: Some(true),
            qualified_before_observation_cutoff: Some(true),
        };
        assert!(facts.qualified_action_history());
        facts.fail("permissionEffectLate");
        assert!(facts.qualified_action_history());
        assert!(!facts.target_positive());
        assert!(!facts.positive());
        facts.native.post_drop_matched = Some(false);
        assert!(!facts.qualified_action_history());
    }

    // Mutation: ordinary polling starts close10 or fails the target before a real close.
    #[test]
    fn ordinary_open_poll_does_not_create_a_close_boundary() {
        let start = Instant::now();
        let mut timing = Timing::new(start);
        assert!(!note_close(
            &mut timing,
            false,
            start + Duration::from_secs(2)
        ));
        assert_eq!(timing.first_close(), None);
        assert!(note_close(
            &mut timing,
            true,
            start + Duration::from_secs(3)
        ));
        assert!(note_close(
            &mut timing,
            true,
            start + Duration::from_secs(4)
        ));
        assert_eq!(timing.first_close(), Some(start + Duration::from_secs(3)));
    }

    // Mutation: Weak becomes positive authority or can be rebound after cutoff/replay.
    #[test]
    fn disposal_anchor_is_weak_cutoff_only_and_rebinds_exactly_once() {
        let original = Rc::new(11u8);
        let selected = Rc::new(11u8);
        let anchor = DisposalAnchor::new(&original);
        assert!(anchor.for_disposal(false).is_none());
        assert!(Rc::ptr_eq(&anchor.for_disposal(true).unwrap(), &original));
        assert!(anchor.rebind_once(&selected, false));
        assert!(Rc::ptr_eq(&anchor.for_disposal(true).unwrap(), &selected));
        assert!(!anchor.rebind_once(&original, false));
        let late = DisposalAnchor::new(&original);
        assert!(!late.rebind_once(&selected, true));
        assert!(!late.rebind_once(&selected, false));
        drop(original);
        drop(selected);
        assert!(anchor.for_disposal(true).is_none());
        assert!(late.for_disposal(true).is_none());
    }

    // Mutation: nonzero physical handlers postpone Core release, or Core drops before receivers.
    #[test]
    fn finite_controls_release_core_last_without_waiting_for_handlers() {
        struct Probe<'a>(&'static str, &'a RefCell<Vec<&'static str>>);
        impl Drop for Probe<'_> {
            fn drop(&mut self) {
                self.1.borrow_mut().push(self.0);
            }
        }
        let order = RefCell::new(Vec::new());
        let handlers = Cell::new(7);
        finite_disposal(
            Probe("prepared", &order),
            Probe("receivers", &order),
            Probe("core22", &order),
            Probe("environment", &order),
            Probe("core", &order),
        );
        assert_eq!(
            &*order.borrow(),
            &["prepared", "receivers", "core22", "environment", "core"]
        );
        assert_eq!(handlers.get(), 7);
    }

    // Mutation: another receiver-row seal or ACK-less return impersonates the original registration.
    #[test]
    fn receiver_binding_is_unique_once_and_preserves_original_zero_token() {
        let row = Receiver::ALL[0];
        let binding = Rc::new(ReceiverBinding::new(row));
        let other = Rc::new(ReceiverBinding::new(row));
        binding.canonical_attested.set(true);
        other.canonical_attested.set(true);
        assert!(binding.reserve_handler());
        assert!(!binding.reserve_handler());
        assert!(binding.matches(&binding, row));
        assert!(!binding.matches(&other, row));
        assert!(!binding.add_started.get());
        binding.add_started.set(true);
        binding.returned.set(Some(true));
        binding.token.set(Some(0));
        assert_eq!(binding.token.get(), Some(0));
        assert!(!binding.acked.get());
        binding.acked.set(true);
        assert!(binding.registered());
        binding.returned.set(Some(false));
        assert!(!binding.registered());
    }

    // Mutation: supported X ContextCreated manufactures a record ignored by Protocol.
    #[test]
    fn disposal_context_never_creates_document_credit() {
        assert!(document_credit(false, true, true, true));
        for cutoff in [false, true] {
            assert!(!document_credit(cutoff, false, true, true));
            assert!(!document_credit(cutoff, true, false, true));
            assert!(!document_credit(cutoff, true, true, false));
        }
        assert!(!document_credit(true, true, true, true));
    }

    // Mutation: a later metadata/setter failure replaces an earlier count overflow.
    #[test]
    fn counted_boundary_latches_first_cause_immediately_and_preserves_prior_failures() {
        let mut facts = Facts::default();
        counted(
            &mut facts.counts.permission_entries,
            1,
            &mut facts.overflow,
            &mut facts.failure,
        );
        counted(
            &mut facts.counts.permission_entries,
            1,
            &mut facts.overflow,
            &mut facts.failure,
        );
        facts.fail("permissionNativeActionFailed");
        assert_eq!(facts.counts.permission_entries, 2);
        assert!(facts.overflow);
        assert_eq!(facts.failure, Some("permissionOverflow"));
        let mut earlier = Facts::default();
        earlier.fail("permissionNativeMetadataFailed");
        for _ in 0..3 {
            counted(
                &mut earlier.counts.deny_attempts,
                1,
                &mut earlier.overflow,
                &mut earlier.failure,
            );
        }
        assert_eq!(earlier.counts.deny_attempts, 2);
        assert!(earlier.overflow);
        assert_eq!(earlier.failure, Some("permissionNativeMetadataFailed"));
    }

    // Mutation: taking a control slot transiently permits ref0 or nested reuse.
    #[test]
    fn action_slot_reservation_blocks_nested_reuse_and_failed_ownership_stays_failed() {
        let callbacks = Arc::new(Callbacks::default());
        let inventory = Arc::new(NativeInventory::default());
        let outer = ActionSlot::new(&callbacks, &inventory, 1).unwrap();
        assert_eq!(inventory.action_slots.load(Ordering::SeqCst), 1);
        assert!(ActionSlot::new(&callbacks, &inventory, 1).is_none());
        outer.resolve();
        assert_eq!(inventory.action_slots.load(Ordering::SeqCst), 0);
        assert!(inventory.slot_failed.load(Ordering::SeqCst));
        let clean = Arc::new(NativeInventory::default());
        ActionSlot::new(&callbacks, &clean, 2).unwrap().resolve();
        assert_eq!(clean.action_slots.load(Ordering::SeqCst), 0);
        assert!(!clean.slot_failed.load(Ordering::SeqCst));
    }

    // Mutation: error/unwind silently zeroes a missing/restoration-unknown slot.
    #[test]
    fn unresolved_action_slot_drop_retains_physical_liability() {
        let callbacks = Arc::new(Callbacks::default());
        let inventory = Arc::new(NativeInventory::default());
        drop(ActionSlot::new(&callbacks, &inventory, 4).unwrap());
        assert_eq!(inventory.action_slots.load(Ordering::SeqCst), 4);
        assert!(inventory.slot_failed.load(Ordering::SeqCst));
    }

    fn intent_request() -> Code0WindowRequest {
        Code0WindowRequest {
            instance_id: "permission-test".into(),
            connection_epoch: "epoch".into(),
            nonce: "nonce".into(),
            document_id: "document".into(),
            mode: crate::platform::windows::code0_web_session::Code0WindowMode::Sync,
        }
    }

    // Mutation: marker equality is omitted or a mismatch can later retry/restart the clock.
    #[test]
    fn intent_is_same_request_original_clock_and_failed_mismatch_is_sticky() {
        let request = intent_request();
        let start = Instant::now();
        for changed_clock in [false, true] {
            let mut intent = Intent::new(request.clone(), start);
            let mut other = request.clone();
            other.document_id.push('x');
            assert!(!intent.reserve_prepare(
                if changed_clock { &request } else { &other },
                if changed_clock {
                    start + Duration::from_nanos(1)
                } else {
                    start
                }
            ));
            assert!(!intent.reserve_prepare(&request, start));
            assert!(intent.failed);
        }
    }

    // Mutation: begin before prepare/duplicate prepare/begin silently reuses the same mode.
    #[test]
    fn intent_prepare_and_begin_are_separate_once_only_steps_without_native_authority() {
        let request = intent_request();
        let start = Instant::now();
        let mut out_of_order = Intent::new(request.clone(), start);
        assert!(!out_of_order.reserve_begin(&request, start));
        assert!(!out_of_order.reserve_prepare(&request, start));
        let mut repeated_prepare = Intent::new(request.clone(), start);
        assert!(repeated_prepare.reserve_prepare(&request, start));
        assert!(!repeated_prepare.reserve_prepare(&request, start));
        assert!(!repeated_prepare.reserve_begin(&request, start));
        let mut normal = Intent::new(request.clone(), start);
        assert!(normal.reserve_prepare(&request, start));
        assert!(normal.reserve_begin(&request, start));
        assert!(!normal.reserve_begin(&request, start));
        let protocol = Protocol::new(
            Identity {
                request: 1,
                generation: 2,
                installation: 3,
                core: 4,
            },
            0,
        )
        .unwrap();
        assert_eq!(protocol.status().phase, Phase::Registering);
        assert_eq!(protocol.status().obligations, 0);
        assert_eq!(protocol.status().owned_registrations, 0);
    }

    // Mutation: an early Registering delivery is treated as a real selected action/effect.
    #[test]
    fn registering_permission_delivery_is_not_authority_but_restriction_still_runs_once() {
        let identity = Identity {
            request: 1,
            generation: 2,
            installation: 3,
            core: 4,
        };
        let mut protocol = Protocol::new(identity, 0).unwrap();
        assert!(
            protocol
                .native_entered(Boundary { identity, ms: 1 })
                .is_err()
        );
        let calls = Cell::new(0);
        let _ = restrictive_sequence(
            || false,
            || {
                calls.set(calls.get() + 1);
                false
            },
            || Some(false),
        );
        assert_eq!(calls.get(), 1);
        assert_ne!(protocol.status().native_qualified, Some(true));
        assert_eq!(protocol.status().obligations, 0);
    }

    // Mutation: conversion/getter work runs before checking the actual UI guard.
    #[test]
    fn selected_ui_entry_checks_guard_before_any_conversion_or_native_work() {
        let failure = AtomicBool::new(false);
        let trace = RefCell::new(Vec::new());
        assert!(selected_ui_entry(
            &failure,
            || {
                trace.borrow_mut().push("guard");
                true
            },
            || {
                trace.borrow_mut().push("convert/getter");
            }
        ));
        assert_eq!(&*trace.borrow(), &["guard", "convert/getter"]);
        trace.borrow_mut().clear();
        assert!(!selected_ui_entry(
            &failure,
            || {
                trace.borrow_mut().push("guard");
                false
            },
            || {
                panic!("off-UI must not convert/AddRef/read native args");
            }
        ));
        assert_eq!(&*trace.borrow(), &["guard"]);
        assert!(failure.load(Ordering::SeqCst));
    }

    // Mutation: a later/outer successful guard clears a nested affinity failure.
    #[test]
    fn selected_ui_entry_keeps_nested_failure_sticky_and_still_runs_safe_ui_restriction() {
        let failure = AtomicBool::new(false);
        let calls = Cell::new(0);
        assert!(!selected_ui_entry(
            &failure,
            || true,
            || {
                assert!(!selected_ui_entry(
                    &failure,
                    || false,
                    || {
                        panic!("wrong thread getter");
                    }
                ));
            }
        ));
        assert!(!selected_ui_entry(
            &failure,
            || true,
            || {
                calls.set(calls.get() + 1);
            }
        ));
        assert_eq!(calls.get(), 1); // A usable UI delivery still gets its refusal.
        assert!(failure.load(Ordering::SeqCst));
    }

    fn lifetime_protocol() -> (Protocol, Boundary) {
        let identity = Identity {
            request: 91,
            generation: 92,
            installation: 93,
            core: 94,
        };
        let boundary = Boundary { identity, ms: 1 };
        let mut protocol = Protocol::new(identity, 0).unwrap();
        for slot in 0..24 {
            protocol
                .registration_added(
                    key(slot),
                    if slot == 22 {
                        NativeToken::Filter
                    } else {
                        NativeToken::Event(0)
                    },
                    boundary,
                )
                .unwrap();
        }
        protocol.settings_acknowledged(boundary).unwrap();
        (protocol, boundary)
    }

    // Mutation: Invoke/consumed releases the field lifetime as a FnOnce wrapper would.
    #[test]
    fn completion_invocation_and_consumption_do_not_drop_the_com_object_field() {
        let callbacks = Arc::new(Callbacks::default());
        let inventory = Arc::new(NativeInventory::default());
        let armed = Arc::new(AtomicBool::new(true));
        let lifetime = Rc::new(CommandLifetime::new(&callbacks, &inventory, 0, armed));
        let retained_object = Rc::clone(&lifetime);
        let (mut protocol, boundary) = lifetime_protocol();
        let ticket = protocol
            .command_entered(Command::PageEnable, boundary)
            .unwrap();
        protocol.command_returned(ticket, true, boundary).unwrap();
        let callback = protocol.completion_entered(ticket, boundary).unwrap();
        protocol
            .completion_captured(callback, true, CompletionValue::EmptyAck, boundary)
            .unwrap();
        protocol.completion_qualified(callback, boundary).unwrap();
        protocol.completion_consumed(ticket, boundary).unwrap();
        drop(lifetime);
        assert_eq!(protocol.status().obligations, 0);
        assert_eq!(protocol.status().command_handlers, 1);
        assert_eq!(inventory.command_handlers.load(Ordering::SeqCst), 1);
        assert_eq!(callbacks.handlers.load(Ordering::SeqCst), 1);
        assert_eq!(inventory.command_drop_bits.load(Ordering::SeqCst), 0);
        drop(retained_object);
        assert_eq!(inventory.command_handlers.load(Ordering::SeqCst), 0);
        assert_eq!(callbacks.handlers.load(Ordering::SeqCst), 0);
        assert_eq!(inventory.command_drop_bits.load(Ordering::SeqCst), 1);
        protocol.command_handler_dropped(ticket, boundary).unwrap();
        assert_eq!(protocol.status().command_handlers, 0);
    }

    // Mutation: actual handler Drop is treated as an invocation/cancelled obligation.
    #[test]
    fn completion_object_field_drop_without_invoke_never_consumes_the_obligation() {
        let callbacks = Arc::new(Callbacks::default());
        let inventory = Arc::new(NativeInventory::default());
        let (mut protocol, boundary) = lifetime_protocol();
        let ticket = protocol
            .command_entered(Command::PageEnable, boundary)
            .unwrap();
        let lifetime =
            CommandLifetime::new(&callbacks, &inventory, 0, Arc::new(AtomicBool::new(true)));
        drop(lifetime);
        assert_eq!(inventory.command_drop_bits.load(Ordering::SeqCst), 1);
        protocol.command_handler_dropped(ticket, boundary).unwrap();
        protocol.cutoff(CutoffReason::StopEntry, boundary);
        assert_eq!(protocol.status().command_handlers, 0);
        assert_eq!(protocol.status().obligations, 1);
        assert!(protocol.completion_consumed(ticket, boundary).is_err());
        assert_eq!(protocol.status().obligations, 1);
    }

    // Mutation: reserve after fallible metadata and let a reentrant delivery win again.
    #[test]
    fn first_delivery_reservation_is_consumed_before_metadata_without_replacement() {
        let mut reserved = false;
        assert!(take_once(&mut reserved));
        // The first metadata result can fail; neither nested nor later valid
        // metadata may select a replacement passing record/main response.
        assert!(!take_once(&mut reserved));
        assert!(!take_once(&mut reserved));
        assert!(reserved);
    }

    // Mutation: a denied string supplies a missing native action or overrides HRESULT.
    #[test]
    fn effect_failure_mapping_keeps_denied_hresult_and_native_target_independent() {
        assert_eq!(
            effect_failure(false, true, "denied"),
            Some("permissionNativeTargetMissing")
        );
        assert_eq!(
            effect_failure(true, false, "denied"),
            Some("permissionProtocolRejected")
        );
        for value in [
            "precondition",
            "default",
            "granted",
            "documentChanged",
            "invalid",
        ] {
            assert_eq!(
                effect_failure(false, true, value),
                Some("permissionPreflightFailed")
            );
            assert_eq!(
                effect_failure(true, true, value),
                Some("permissionPreflightFailed")
            );
        }
        assert_eq!(effect_failure(true, true, "denied"), None);
    }

    // Mutation: setup failure rebuilds empty coverage instead of the original producer's facts.
    #[test]
    fn failed_setup_snapshot_preserves_actual_partial_return_and_uncertain_ownership() {
        let mut coverage = Coverage::default();
        coverage.add_entered(0);
        coverage.add_returned(0, true);
        coverage.add_qualified(0, true);
        coverage.add_entered(1);
        coverage.add_returned(1, false);
        let facts = setup_failure_facts(&coverage, 1, Some(true), 2, 3);
        assert_eq!(facts.coverage.add_attempts[0..2], [1, 1]);
        assert_eq!(facts.coverage.add_returns[0..2], [Some(true), Some(false)]);
        assert_eq!(facts.coverage.add_acks[0..2], [1, 0]);
        assert_eq!(facts.coverage.owned_mask, 3);
        assert_eq!(facts.coverage.uncertain_mask, 2);
        assert_eq!(facts.setup.hooks1023, 1);
        assert_eq!(facts.setup.settings_ack, Some(true));
        assert_eq!(facts.native.kind_notifications, None);
        assert_eq!(facts.failure, Some("permissionSetupRejected"));
        assert!(!facts.native_positive());
    }

    // Mutation: inner completion silently reorders the earlier outer delivery.
    #[test]
    fn nested_lifecycle_entry_cannot_complete_or_repair_the_outer_entry() {
        let mut order = LifecycleOrder::default();
        let outer = order.enter().unwrap();
        let inner = order.enter().unwrap();
        assert!(!order.complete(inner));
        assert!(!order.complete(outer));
        assert!(order.invalid);
        let later = order.enter().unwrap();
        assert!(!order.complete(later));
        assert!(order.invalid);
        assert_eq!(order.active, 0);
    }

    // Mutation: treating every sequential delivery as reentry or resetting stamps.
    #[test]
    fn sequential_lifecycle_entries_complete_without_queue_or_hidden_retry() {
        let mut order = LifecycleOrder::default();
        let first = order.enter().unwrap();
        assert!(order.complete(first));
        let second = order.enter().unwrap();
        assert!(second > first);
        assert!(order.complete(second));
        assert!(!order.invalid);
        assert_eq!(order.active, 0);
    }

    // Mutation: native snapshot overwrites independent Session/exit/overflow.
    #[test]
    fn snapshot_merge_preserves_independent_fixture_observations_and_sticky_failure() {
        let mut local = Facts::default();
        local.counts.session_policy_failures = 1;
        local.status.session_reason = Some("PolicyRejected");
        local.ordering.actual_policy_failure_after_action = Some(true);
        local.cleanup.browser_process_exits = 1;
        local.cleanup.window_destroyed = 1;
        local.overflow = true;
        local.failure = Some("permissionOverflow");
        merge_native_facts(&mut local, &Facts::default());
        assert_eq!(local.counts.session_policy_failures, 1);
        assert_eq!(local.status.session_reason, Some("PolicyRejected"));
        assert_eq!(
            local.ordering.actual_policy_failure_after_action,
            Some(true)
        );
        assert_eq!(local.cleanup.browser_process_exits, 1);
        assert_eq!(local.cleanup.window_destroyed, 1);
        assert!(local.overflow);
        assert_eq!(local.failure, Some("permissionOverflow"));
    }

    // Mutation: the signed/returned final clock is replaced by rounded/native release time.
    #[test]
    fn final_clock_requires_actual_same_earliest_origin_after_all_cleanup() {
        let start = Instant::now();
        let mut timing = Timing::new(start);
        timing.close(start + Duration::from_secs(40));
        let mut facts = Facts::default();
        timing.record_final(
            &mut facts,
            start + Duration::from_secs(49),
            timing.first_close(),
        );
        assert_eq!(facts.timing.final_within_close_deadline, Some(true));
        assert_eq!(facts.timing.final_at_ms, Some(49_000));
        timing.record_final(
            &mut facts,
            start + Duration::from_secs(50),
            timing.first_close(),
        );
        assert_eq!(facts.timing.final_within_close_deadline, Some(false));
        assert_eq!(facts.failure, Some("permissionCleanupFailed"));
        let mut missing = Facts::default();
        timing.record_final(&mut missing, start + Duration::from_secs(49), None);
        assert_eq!(missing.timing.first_close_adopted, Some(false));
        assert_eq!(missing.failure, Some("permissionCleanupFailed"));
    }

    // Mutation: wrap or erase a duplicate instead of retaining the lower bound.
    #[test]
    fn duplicate_counts_saturate_and_never_recover_to_one() {
        let mut count = 0;
        assert!(increment(&mut count, 1));
        assert_eq!(count, 1);
        assert!(!increment(&mut count, 1));
        assert_eq!(count, 2);
        for _ in 0..5 {
            assert!(!increment(&mut count, 1));
        }
        assert_eq!(count, 2);
    }

    // Mutation: Protocol's broad disposal counter is copied into both groups.
    #[test]
    fn lifecycle_entry_partition_is_disjoint_even_when_decode_later_fails() {
        let mut facts = Observations::default();
        facts.enter(0, Phase::Enabling, false);
        facts.enter(0, Phase::Armed, false);
        facts.enter(0, Phase::Cutoff, true);
        assert_eq!(facts.deliveries, 3);
        assert_eq!(facts.bootstrap_deliveries, 1);
        assert_eq!(facts.post_arm_pre_cutoff, 1);
        assert_eq!(facts.disposal_deliveries, 1);
        assert_eq!(facts.by_receiver[0], 3);
    }

    // Mutation: saturation fabricates an exact relation between lower bounds.
    #[test]
    fn lifecycle_overflow_keeps_each_actual_lower_bound() {
        let mut facts = Observations {
            deliveries: u16::MAX,
            ..Observations::default()
        };
        facts.enter(2, Phase::Registering, false);
        assert_eq!(facts.deliveries, u16::MAX);
        assert_eq!(facts.bootstrap_deliveries, 1);
        assert_eq!(facts.by_receiver[2], 1);
        assert!(facts.counter_overflow);
    }

    // Mutation: each callback gets its own4096 ledger or reservations occur late.
    #[test]
    fn text_charges_share_one_concurrent_budget_and_real_drop_releases_it() {
        let ledger = Arc::new(TextLedger::default());
        let first = TextCharge::reserve(&ledger, 2048).unwrap();
        let second = TextCharge::reserve(&ledger, 2048).unwrap();
        assert!(TextCharge::reserve(&ledger, 1).is_none());
        assert_eq!(ledger.live.load(Ordering::SeqCst), 4096);
        drop(first);
        assert_eq!(ledger.live.load(Ordering::SeqCst), 2048);
        drop(second);
        assert_eq!(ledger.live.load(Ordering::SeqCst), 0);
        assert_eq!(ledger.peak.load(Ordering::SeqCst), 4096);
        assert!(ledger.failed.load(Ordering::SeqCst));
    }

    // Mutation: successful HRESULT alone creates an ACK or clears uncertainty.
    #[test]
    fn coverage_history_keeps_successful_return_and_failed_post_drop_distinct() {
        let mut history = Coverage::default();
        history.add_entered(6);
        history.add_returned(6, true);
        history.add_qualified(6, false);
        assert_eq!(history.add_attempts[6], 1);
        assert_eq!(history.add_returns[6], Some(true));
        assert_eq!(history.add_acks[6], 0);
        assert_eq!(history.owned_mask, 1 << 6);
        assert_eq!(history.uncertain_mask, 1 << 6);
    }

    // Mutation: token0 is absence, failed remove releases liability or clears cutoff.
    #[test]
    fn failed_removal_retains_actual_original_registration_liability() {
        let mut history = Coverage::default();
        history.add_entered(10);
        history.add_returned(10, true);
        history.add_qualified(10, true);
        history.remove_entered(10);
        history.remove_returned(10, false);
        history.remove_qualified(10, false);
        assert_eq!(history.added_mask, 1 << 10);
        assert_eq!(history.remove_attempt_mask, 1 << 10);
        assert_eq!(history.removed_mask, 0);
        assert_eq!(history.owned_mask, 1 << 10);
        assert_eq!(history.uncertain_mask, 1 << 10);
    }

    // Mutation: a cached denied/default state suppresses a real restrictive action.
    #[test]
    fn failed_capture_still_performs_one_deny_and_one_readback_per_delivery() {
        use std::cell::RefCell;
        let calls = RefCell::new(Vec::new());
        for _ in 0..2 {
            let result = restrictive_sequence(
                || {
                    calls.borrow_mut().push("capture");
                    false
                },
                || {
                    calls.borrow_mut().push("deny");
                    false
                },
                || {
                    calls.borrow_mut().push("readback");
                    Some(false)
                },
            );
            assert_eq!(result, (false, false, Some(false)));
        }
        assert_eq!(
            &*calls.borrow(),
            &["capture", "deny", "readback", "capture", "deny", "readback"]
        );
    }

    // Mutation: restarting45 or first-close10/equality admitted by rounded times.
    #[test]
    fn precise_clock_uses_original_origin_and_earliest_close() {
        let start = Instant::now();
        let mut timing = Timing::new(start);
        assert!(timing.before(start + Duration::from_secs(45) - Duration::from_nanos(1)));
        assert!(!timing.before(start + Duration::from_secs(45)));
        timing.close(start + Duration::from_secs(1));
        timing.close(start + Duration::from_secs(5));
        assert_eq!(timing.first_close(), Some(start + Duration::from_secs(1)));
        assert!(timing.close_before(start + Duration::from_secs(11) - Duration::from_nanos(1)));
        assert!(!timing.close_before(start + Duration::from_secs(11)));
    }
}

#[cfg(test)]
mod fix2_tests {
    use super::*;

    // Mutation: later metadata/DENY replaces the precise first known failure.
    #[test]
    fn first_failure_carrier_revokes_parent_before_later_restriction_and_keeps_t1() {
        let start = Instant::now();
        let t1 = start + Duration::from_secs(1);
        let t2 = start + Duration::from_secs(4);
        let mut timing = Timing::new(start);
        let mut history = None;
        let mut failed = false;
        let mut closing = false;
        latch_close(&mut timing, &mut history, &mut failed, &mut closing, t1);
        assert!(failed && closing);
        let calls = Cell::new(0);
        let result = restrictive_sequence(
            || {
                assert_eq!(history, Some(t1));
                assert_eq!(timing.first_close(), Some(t1));
                false
            },
            || {
                calls.set(calls.get() + 1);
                false
            },
            || {
                calls.set(calls.get() + 1);
                Some(false)
            },
        );
        assert_eq!(result, (false, false, Some(false)));
        assert_eq!(calls.get(), 2);
        latch_close(&mut timing, &mut history, &mut failed, &mut closing, t2);
        assert_eq!(history, Some(t1));
        assert_eq!(timing.first_close(), Some(t1));
        assert!(timing.close_before(start + Duration::from_secs(11) - Duration::from_nanos(1)));
        assert!(!timing.close_before(start + Duration::from_secs(11)));
    }

    // Mutation: a pre-State setup failure is discarded when State first closes.
    #[test]
    fn close_carrier_preserves_earlier_history_without_restarting_native_clock() {
        let start = Instant::now();
        let t1 = start + Duration::from_secs(1);
        let mut timing = Timing::new(start);
        let mut history = Some(t1);
        let mut failed = false;
        let mut closing = false;
        latch_close(
            &mut timing,
            &mut history,
            &mut failed,
            &mut closing,
            start + Duration::from_secs(3),
        );
        assert!(failed && closing);
        assert_eq!(timing.first_close(), Some(t1));
        assert_eq!(history, Some(t1));
    }

    // Mutation: cap+1/invalid native capture stays complete or invents reserve failure.
    #[test]
    fn borrowed_capture_failures_are_sticky_and_reservation_failures_are_truthful() {
        let cases = [
            (
                vec![65u16; 65].into_iter().chain([0]).collect::<Vec<_>>(),
                64,
                CaptureError::ByteCap,
            ),
            (
                vec![65u16; 513].into_iter().chain([0]).collect::<Vec<_>>(),
                512,
                CaptureError::ByteCap,
            ),
            (vec![65u16; 2048], 2048, CaptureError::Unterminated),
            (vec![0xd800u16, 0], 64, CaptureError::Utf16),
        ];
        for (wide, cap, want) in cases {
            let ledger = Arc::new(TextLedger::default());
            let got = unsafe { capture_text(PCWSTR(wide.as_ptr()), 2048, cap, &ledger) };
            assert_eq!(got.err(), Some(want));
            assert!(ledger.failed.load(Ordering::SeqCst));
            assert_eq!(ledger.failures.load(Ordering::SeqCst), 0);
            assert_eq!(ledger.live.load(Ordering::SeqCst), 0);
            let valid = [123u16, 125, 0];
            let later = unsafe { capture_text(PCWSTR(valid.as_ptr()), 2048, 64, &ledger) }.unwrap();
            drop(later);
            assert!(ledger.failed.load(Ordering::SeqCst));
            assert_eq!(ledger.live.load(Ordering::SeqCst), 0);
        }
        let ledger = Arc::new(TextLedger::default());
        let held = TextCharge::reserve(&ledger, 4096).unwrap();
        let wide = [65u16, 0];
        assert_eq!(
            unsafe { capture_text(PCWSTR(wide.as_ptr()), 2048, 64, &ledger) }.err(),
            Some(CaptureError::Reservation)
        );
        assert_eq!(ledger.failures.load(Ordering::SeqCst), 1);
        drop(held);
        assert_eq!(ledger.live.load(Ordering::SeqCst), 0);
        assert!(ledger.failed.load(Ordering::SeqCst));
    }

    // Mutation: successfully copied malformed JSON is mislabeled unavailable accounting.
    #[test]
    fn captured_malformed_json_keeps_truthful_complete_accounting_and_has_no_ack() {
        let ledger = Arc::new(TextLedger::default());
        let wide = [123u16, 0];
        let text = unsafe { capture_text(PCWSTR(wide.as_ptr()), 2048, 64, &ledger) }.unwrap();
        assert!(wire::decode_completion(Command::PageEnable, text.value.as_bytes()).is_err());
        assert_eq!(ledger.live.load(Ordering::SeqCst), 1);
        drop(text);
        assert_eq!(ledger.live.load(Ordering::SeqCst), 0);
        assert_eq!(ledger.failures.load(Ordering::SeqCst), 0);
        assert!(!ledger.failed.load(Ordering::SeqCst));
    }

    // Mutation: phase publication and Invoke entries are counted from separate guessed phases.
    #[test]
    fn atomic_lifecycle_entries_partition_at_the_same_publication_boundary() {
        let entries = LifecycleEntries::default();
        entries.enter(0);
        entries.publish(Phase::Enabling, false);
        entries.enter(1);
        entries.publish(Phase::Armed, false);
        entries.enter(0);
        entries.publish(Phase::Closing, true);
        entries.enter(9);
        let observed = entries.snapshot();
        assert_eq!(observed.deliveries, 4);
        assert_eq!(observed.bootstrap_deliveries, 2);
        assert_eq!(observed.post_arm_pre_cutoff, 1);
        assert_eq!(observed.disposal_deliveries, 1);
        assert_eq!(observed.by_receiver, [2, 1, 0, 0, 0, 0, 0, 0, 0, 1]);
        assert!(!observed.counter_overflow);
        let failed = AtomicBool::new(false);
        entries.enter(3);
        let native = Cell::new(false);
        assert!(!selected_ui_entry(&failed, || false, || native.set(true)));
        assert!(!native.get());
        assert!(failed.load(Ordering::SeqCst));
        let after = entries.snapshot();
        assert_eq!(after.deliveries, 5);
        assert_eq!(after.disposal_deliveries, 2);
        assert_eq!(after.by_receiver[3], 1);
    }

    // Mutation: concurrent entry/publication loses entries or invents exact sums after truncation.
    #[test]
    fn atomic_lifecycle_entries_concurrent_publication_and_saturation_keep_truthful_rows() {
        let entries = Arc::new(LifecycleEntries::default());
        let worker = Arc::clone(&entries);
        let join = std::thread::spawn(move || {
            for _ in 0..1000 {
                worker.enter(2);
            }
        });
        for _ in 0..1000 {
            entries.publish(Phase::Armed, false);
            entries.publish(Phase::Cutoff, true);
        }
        join.join().unwrap();
        let observed = entries.snapshot();
        assert_eq!(observed.deliveries, 1000);
        assert_eq!(observed.by_receiver[2], 1000);
        assert_eq!(
            u32::from(observed.bootstrap_deliveries)
                + u32::from(observed.post_arm_pre_cutoff)
                + u32::from(observed.disposal_deliveries),
            1000
        );
        assert!(!observed.counter_overflow);
        let saturated = LifecycleEntries::default();
        for _ in 0..=u16::MAX {
            saturated.enter(4);
        }
        let observed = saturated.snapshot();
        assert_eq!(observed.deliveries, u16::MAX);
        assert_eq!(observed.by_receiver[4], u16::MAX);
        assert!(observed.counter_overflow);
    }
}

#[cfg(test)]
mod failure_boundary_fix2_tests {
    use super::*;

    // Mutation: a counted duplicate remains open until DENY/sync after later native work.
    #[test]
    fn counted_failure_closes_shared_semantic_native_and_parent_carriers_at_t1() {
        let start = Instant::now();
        let at = start + Duration::from_secs(1);
        let identity = Identity {
            request: 1,
            generation: 2,
            installation: 3,
            core: 4,
        };
        let mut protocol = Protocol::new(identity, 0).unwrap();
        let mut facts = Facts::default();
        let mut timing = Timing::new(start);
        let mut history = None;
        let mut failed = false;
        let mut closing = false;
        for _ in 0..2 {
            counted(
                &mut facts.counts.permission_entries,
                1,
                &mut facts.overflow,
                &mut facts.failure,
            );
        }
        settle_failure(&mut protocol, &mut facts, &mut timing, identity, at);
        latch_close(&mut timing, &mut history, &mut failed, &mut closing, at);
        assert!(failed && closing);
        assert_eq!(facts.failure, Some("permissionOverflow"));
        assert_eq!(protocol.status().first_close_ms, Some(1000));
        assert!(protocol.status().action_revoked);
        settle_failure(
            &mut protocol,
            &mut facts,
            &mut timing,
            identity,
            start + Duration::from_secs(4),
        );
        assert_eq!(timing.first_close(), Some(at));
        assert_eq!(history, Some(at));
        assert!(!timing.close_before(start + Duration::from_secs(11)));
    }

    // Mutation: a pure Protocol rejection is noticed only by a later snapshot sync.
    #[test]
    fn pure_protocol_failure_is_closed_before_any_later_operation_or_snapshot() {
        let start = Instant::now();
        let at = start + Duration::from_secs(1);
        let identity = Identity {
            request: 1,
            generation: 2,
            installation: 3,
            core: 4,
        };
        let mut protocol = Protocol::new(identity, 0).unwrap();
        let mut facts = Facts::default();
        let mut timing = Timing::new(start);
        assert!(
            protocol
                .lifecycle(
                    Lifecycle::Unsupported(Receiver::ALL[0]),
                    Boundary { identity, ms: 1000 }
                )
                .is_err()
        );
        settle_failure(&mut protocol, &mut facts, &mut timing, identity, at);
        assert_eq!(timing.first_close(), Some(at));
        assert!(protocol.status().action_revoked);
        assert_eq!(protocol.status().first_close_ms, Some(1000));
        settle_failure(
            &mut protocol,
            &mut facts,
            &mut timing,
            identity,
            start + Duration::from_secs(4),
        );
        assert_eq!(timing.first_close(), Some(at));
    }
}

#[cfg(test)]
mod failure_clock_fix2_tests {
    use super::*;

    // Mutation: off-UI failure is timed by a later UI poll or rounded to milliseconds.
    #[test]
    fn failure_clock_preserves_precise_original_origin_and_atomic_earliest_failure() {
        let start = Instant::now();
        let clock = Arc::new(FailureClock::default());
        assert!(clock.bind(start));
        let worker = Arc::clone(&clock);
        let at = start + Duration::from_nanos(1_234_567);
        std::thread::spawn(move || worker.record(at))
            .join()
            .unwrap();
        clock.record(start + Duration::from_secs(4));
        assert_eq!(clock.origin(), Ok(Some(at)));
        assert!(clock.matches(start));
        assert!(!clock.matches(start + Duration::from_nanos(1)));
        let mut timing = Timing::new(start);
        clock.adopt(&mut timing).unwrap();
        assert_eq!(timing.first_close(), Some(at));
        assert!(!timing.close_before(at + Duration::from_secs(10)));
        assert!(timing.close_before(at + Duration::from_secs(10) - Duration::from_nanos(1)));
    }

    // Mutation: missing/changed origin or backwards time is healed by binding/UI now.
    #[test]
    fn failure_clock_never_heals_unbound_rebound_or_backwards_failure() {
        let start = Instant::now();
        let missing = FailureClock::default();
        missing.record(start);
        assert_eq!(missing.origin(), Err(()));
        assert!(!missing.bind(start));
        let rebound = FailureClock::default();
        assert!(rebound.bind(start));
        assert!(!rebound.bind(start + Duration::from_nanos(1)));
        assert_eq!(rebound.origin(), Err(()));
        let backwards = FailureClock::default();
        assert!(backwards.bind(start + Duration::from_secs(1)));
        backwards.record(start);
        assert_eq!(backwards.origin(), Err(()));
        let mut timing = Timing::new(start);
        assert!(backwards.adopt(&mut timing).is_err());
        assert_eq!(timing.first_close(), None);
        let inconsistent = FailureClock::default();
        assert!(inconsistent.bind(start));
        inconsistent.invalidate(start + Duration::from_nanos(1));
        assert_eq!(inconsistent.origin(), Err(()));
        inconsistent.record(start + Duration::from_secs(1));
        assert!(!inconsistent.matches(start));
        assert_eq!(inconsistent.origin(), Err(()));
    }

    // Mutation: affinity flag is visible before the precise failure carrier is recorded.
    #[test]
    fn failed_affinity_records_original_clock_before_publishing_failure_and_skips_native_work() {
        let callbacks = Arc::new(Callbacks::default());
        let clock = Arc::new(FailureClock::default());
        let start = Instant::now();
        assert!(clock.bind(start));
        let ui = std::thread::current().id();
        let c = Arc::clone(&clock);
        let cb = Arc::clone(&callbacks);
        std::thread::spawn(move || {
            assert!(!affinity_entry(ui, &cb, &c, 8799));
            assert!(cb.affinity_failed.load(Ordering::SeqCst));
            assert!(c.origin().unwrap().is_some());
        })
        .join()
        .unwrap();
        assert!(callbacks.affinity_failed.load(Ordering::SeqCst));
        let origin = clock.origin().unwrap().unwrap();
        assert!(origin >= start);
    }
}

#[cfg(test)]
mod parent_affinity_fix2_tests {
    use super::*;

    // Mutation: early selected creation-completed guard publishes failure without its real clock.
    #[test]
    fn parent_affinity_guard_uses_the_same_once_bound_clock_before_failure_flag() {
        let callbacks = Arc::new(Callbacks::default());
        let clock = Arc::new(FailureClock::default());
        assert!(clock.bind(Instant::now()));
        assert!(callbacks.permission_clock.set(Arc::clone(&clock)).is_ok());
        let ui = std::thread::current().id();
        let cb = Arc::clone(&callbacks);
        let c = Arc::clone(&clock);
        std::thread::spawn(move || {
            let native = Cell::new(false);
            assert!(!selected_ui_entry(
                &cb.affinity_failed,
                || callback_on_ui(ui, &cb, 8798),
                || native.set(true),
            ));
            assert!(!native.get());
            assert!(cb.affinity_failed.load(Ordering::SeqCst));
            assert!(c.origin().unwrap().is_some());
        })
        .join()
        .unwrap();
        assert!(callbacks.permission_clock.set(Arc::clone(&clock)).is_err());
        assert!(clock.origin().unwrap().is_some());
    }
}
