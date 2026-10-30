// Selected owned HTTP401 native adapter. All new tests/native behavior UNRUN.
use super::super::http401_protocol::{Http401Protocol, ReceiveStatus, SendOutcome};
use super::*;
use serde::Serialize;
use std::cell::Cell;
use std::sync::atomic::{AtomicU8, AtomicU16, AtomicU32, AtomicU64};
use std::time::{Duration, Instant};
use windows::Win32::System::Com::{STATFLAG_NONAME, STATSTG, STREAM_SEEK_SET};
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
    let temp_dir_close_count = facts.cleanup.temp_dir_close_count;
    let adopted = facts.cleanup.first_close_adopted;
    let within_close = facts.cleanup.within_close_deadline;
    let target_at = facts.cleanup.target_at_ms;
    let close_at = facts.cleanup.first_close_at_ms;
    let final_at = facts.cleanup.final_at_ms;
    let cleanup_elapsed = facts.cleanup.cleanup_elapsed_ms;
    let failure = facts.failure;
    let overflow = facts.overflow;
    let unknown = facts.unknown;
    *facts = native.clone();
    facts.session = session;
    facts.cleanup.browser_process_exited_once = exited;
    facts.cleanup.temp_dir_close_count = temp_dir_close_count;
    facts.cleanup.first_close_adopted = adopted;
    facts.cleanup.within_close_deadline = within_close;
    facts.cleanup.target_at_ms = target_at;
    facts.cleanup.first_close_at_ms = close_at;
    facts.cleanup.final_at_ms = final_at;
    facts.cleanup.cleanup_elapsed_ms = cleanup_elapsed;
    facts.overflow |= overflow;
    facts.unknown |= unknown;
    if failure.is_some() {
        facts.failure = failure;
    }
}
fn observed_once<T: PartialEq>(old: &Option<T>, new: &Option<T>) -> bool {
    old.as_ref().is_none_or(|v| new.as_ref() == Some(v))
}
fn qualified_progress(old: Option<bool>, new: Option<bool>) -> bool {
    match old {
        None => true,
        Some(true) => new.is_some(),
        Some(false) => new == Some(false),
    }
}
fn common_progress(old: &DisposalCommon, new: &DisposalCommon) -> bool {
    let before = [
        old.ui_matched,
        old.target_established_at_entry,
        old.closing_admitted_at_entry,
        old.request_matched,
        old.generation_matched,
        old.core_matched,
        old.s2_install_matched,
        old.guard_install_matched,
        old.registration_matched,
        old.not_released,
        old.not_destroyed,
        old.no_prior_failure,
        old.entry_order_current,
        old.post_drop_current,
        old.within_attempt_deadline,
        old.within_close_deadline,
    ];
    let after = [
        new.ui_matched,
        new.target_established_at_entry,
        new.closing_admitted_at_entry,
        new.request_matched,
        new.generation_matched,
        new.core_matched,
        new.s2_install_matched,
        new.guard_install_matched,
        new.registration_matched,
        new.not_released,
        new.not_destroyed,
        new.no_prior_failure,
        new.entry_order_current,
        new.post_drop_current,
        new.within_attempt_deadline,
        new.within_close_deadline,
    ];
    before
        .into_iter()
        .zip(after)
        .all(|(a, b)| qualified_progress(a, b))
        && observed_once(&old.entry_at_ms, &new.entry_at_ms)
        && observed_once(&old.settled_at_ms, &new.settled_at_ms)
}
fn row_progress<P>(
    old: &DisposalRow<P>,
    new: &DisposalRow<P>,
    proof: impl FnOnce(&P, &P) -> bool,
) -> bool {
    new.deliveries >= old.deliveries
        && new.active >= old.active
        && new.qualified_disposal >= old.qualified_disposal
        && new.invalid >= old.invalid
        && new.candidates >= old.candidates
        && match (&old.proof, &new.proof) {
            (None, _) => true,
            (Some(a), Some(b)) => proof(a, b),
            _ => false,
        }
}
pub fn http401_snapshot_monotonic(old: &Facts, new: &Facts) -> bool {
    old.version == new.version
        && old.selector == new.selector
        && old.post_target.version == new.post_target.version
        && new.receipts.target >= old.receipts.target
        && new.receipts.final_receipt >= old.receipts.final_receipt
        && (!old.unknown || new.unknown)
        && (!old.overflow || new.overflow)
        && observed_once(&old.failure, &new.failure)
        && new.navigation.top_events >= old.navigation.top_events
        && new.navigation.attempts >= old.navigation.attempts
        && new.navigation.grants >= old.navigation.grants
        && new.resource.total >= old.resource.total
        && new.resource.passed >= old.resource.passed
        && new.resource.refused >= old.resource.refused
        && observed_once(
            &old.navigation.qualification_matched,
            &new.navigation.qualification_matched,
        )
        && observed_once(
            &old.navigation.navigation_id_readable,
            &new.navigation.navigation_id_readable,
        )
        && observed_once(&old.navigation.cancel_false, &new.navigation.cancel_false)
        && observed_once(
            &old.resource.exact_document_get,
            &new.resource.exact_document_get,
        )
        && observed_once(
            &old.resource.qualification_matched,
            &new.resource.qualification_matched,
        )
        && new.post_target.admission.count >= old.post_target.admission.count
        && observed_once(
            &old.post_target.admission.qualified,
            &new.post_target.admission.qualified,
        )
        && observed_once(
            &old.post_target.admission.first_close_matched,
            &new.post_target.admission.first_close_matched,
        )
        && row_progress(&old.post_target.top, &new.post_target.top, |a, b| {
            common_progress(&a.common, &b.common)
                && b.cancel_attempts >= a.cancel_attempts
                && b.cancel_successes >= a.cancel_successes
                && b.cancel_readbacks >= a.cancel_readbacks
                && observed_once(&a.cancel_true, &b.cancel_true)
        })
        && row_progress(
            &old.post_target.resource,
            &new.post_target.resource,
            |a, b| {
                common_progress(&a.common, &b.common)
                    && b.response_set_attempts >= a.response_set_attempts
                    && b.response_set_successes >= a.response_set_successes
                    && b.response_readbacks >= a.response_readbacks
                    && observed_once(&a.empty_stream_created, &b.empty_stream_created)
                    && observed_once(&a.seek_succeeded, &b.seek_succeeded)
                    && observed_once(&a.fixed_response_created, &b.fixed_response_created)
                    && observed_once(&a.response_matched, &b.response_matched)
                    && observed_once(&a.status_code, &b.status_code)
                    && observed_once(&a.content_query_succeeded, &b.content_query_succeeded)
                    && observed_once(&a.stat_succeeded, &b.stat_succeeded)
                    && observed_once(&a.content_bytes, &b.content_bytes)
            },
        )
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
        #[derive(Clone, Debug, Default, Serialize, PartialEq)]
        #[serde(rename_all = "camelCase")]
        pub struct $name { $(pub $field: $ty),* }
    };
}
#[path = "http401_transport.rs"]
mod http401_transport;
use http401_transport::*;
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

record!(Admission {count:u16,qualified:Option<bool>,first_close_matched:Option<bool>});
impl Admission {
    fn record_attempt(&mut self, qualified: Option<bool>, close: Option<bool>) -> bool {
        let first = self.count == 0;
        let bounded = bump(&mut self.count, 1);
        if first {
            self.qualified = qualified;
            self.first_close_matched = close;
        }
        first && bounded
    }
}
record!(DisposalCommon {ui_matched:Option<bool>,target_established_at_entry:Option<bool>,closing_admitted_at_entry:Option<bool>,request_matched:Option<bool>,generation_matched:Option<bool>,core_matched:Option<bool>,s2_install_matched:Option<bool>,guard_install_matched:Option<bool>,registration_matched:Option<bool>,not_released:Option<bool>,not_destroyed:Option<bool>,no_prior_failure:Option<bool>,entry_order_current:Option<bool>,post_drop_current:Option<bool>,within_attempt_deadline:Option<bool>,within_close_deadline:Option<bool>,entry_at_ms:Option<u64>,settled_at_ms:Option<u64>});
impl DisposalCommon {
    fn positive(&self) -> bool {
        [
            self.ui_matched,
            self.target_established_at_entry,
            self.closing_admitted_at_entry,
            self.request_matched,
            self.generation_matched,
            self.core_matched,
            self.s2_install_matched,
            self.guard_install_matched,
            self.registration_matched,
            self.not_released,
            self.not_destroyed,
            self.no_prior_failure,
            self.entry_order_current,
            self.post_drop_current,
            self.within_attempt_deadline,
            self.within_close_deadline,
        ]
        .into_iter()
        .all(|v| v == Some(true))
            && self
                .entry_at_ms
                .zip(self.settled_at_ms)
                .is_some_and(|(a, b)| a <= b && b < 45_000)
    }
    fn final_order(&self, close: u64, target: u64, final_at: u64) -> bool {
        self.positive()
            && self
                .entry_at_ms
                .zip(self.settled_at_ms)
                .is_some_and(|(a, b)| {
                    close <= target
                        && target <= a
                        && a <= b
                        && b <= final_at
                        && b.checked_sub(close).is_some_and(|v| v <= 10_000)
                })
    }
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopDisposalProof {
    #[serde(flatten)]
    pub common: DisposalCommon,
    pub cancel_attempts: u16,
    pub cancel_successes: u16,
    pub cancel_readbacks: u16,
    pub cancel_true: Option<bool>,
}
impl TopDisposalProof {
    fn positive(&self) -> bool {
        self.common.positive()
            && self.cancel_attempts == 1
            && self.cancel_successes == 1
            && self.cancel_readbacks == 1
            && self.cancel_true == Some(true)
    }
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDisposalProof {
    #[serde(flatten)]
    pub common: DisposalCommon,
    pub empty_stream_created: Option<bool>,
    pub seek_succeeded: Option<bool>,
    pub fixed_response_created: Option<bool>,
    pub response_set_attempts: u16,
    pub response_set_successes: u16,
    pub response_readbacks: u16,
    pub response_matched: Option<bool>,
    pub status_code: Option<i32>,
    pub content_query_succeeded: Option<bool>,
    pub stat_succeeded: Option<bool>,
    pub content_bytes: Option<u64>,
}
impl ResourceDisposalProof {
    fn positive(&self) -> bool {
        self.common.positive()
            && [
                self.empty_stream_created,
                self.seek_succeeded,
                self.fixed_response_created,
                self.response_matched,
                self.content_query_succeeded,
                self.stat_succeeded,
            ]
            .into_iter()
            .all(|v| v == Some(true))
            && self.response_set_attempts == 1
            && self.response_set_successes == 1
            && self.response_readbacks == 1
            && self.status_code == Some(403)
            && self.content_bytes == Some(0)
    }
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisposalRow<P> {
    pub deliveries: u16,
    pub active: u16,
    pub qualified_disposal: u16,
    pub invalid: u16,
    pub pending: u16,
    pub candidates: u16,
    pub proof: Option<P>,
}
impl<P> DisposalRow<P> {
    fn record_candidate(&mut self, proof: P) -> bool {
        let first = self.candidates == 0;
        let bounded = bump(&mut self.candidates, 1);
        if first {
            self.proof = Some(proof);
        }
        first && bounded
    }
    fn replace_counts(&mut self, p: Partition) {
        self.deliveries = p.deliveries;
        self.active = p.active;
        self.qualified_disposal = p.qualified_disposal;
        self.invalid = p.invalid;
        self.pending = p.pending;
    }
    fn positive(&self, check: impl FnOnce(&P) -> bool) -> bool {
        self.active == 1
            && self.invalid == 0
            && self.pending == 0
            && self.qualified_disposal <= 1
            && self.deliveries == 1 + self.qualified_disposal
            && self.candidates == self.qualified_disposal
            && match (self.candidates, &self.proof) {
                (0, None) => true,
                (1, Some(p)) => check(p),
                _ => false,
            }
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostTarget {
    pub version: u8,
    pub admission: Admission,
    pub top: DisposalRow<TopDisposalProof>,
    pub resource: DisposalRow<ResourceDisposalProof>,
}
impl Default for PostTarget {
    fn default() -> Self {
        Self {
            version: 1,
            admission: Admission::default(),
            top: DisposalRow::default(),
            resource: DisposalRow::default(),
        }
    }
}
impl PostTarget {
    fn positive(&self) -> bool {
        self.version == 1
            && self.admission.count == 1
            && self.admission.qualified == Some(true)
            && self.admission.first_close_matched == Some(true)
            && self.top.positive(TopDisposalProof::positive)
            && self.resource.positive(ResourceDisposalProof::positive)
    }
    fn final_order(&self, close: u64, target: u64, final_at: u64) -> bool {
        close <= target
            && self
                .top
                .proof
                .as_ref()
                .is_none_or(|p| p.common.final_order(close, target, final_at))
            && self
                .resource
                .proof
                .as_ref()
                .is_none_or(|p| p.common.final_order(close, target, final_at))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EntryKind {
    Top,
    Resource,
    Auth,
    Other,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntryClass {
    Active,
    Disposal,
    Invalid,
}
#[derive(Clone, Copy, Default)]
struct Partition {
    deliveries: u16,
    active: u16,
    qualified_disposal: u16,
    invalid: u16,
    pending: u16,
}
#[derive(Clone, Copy)]
pub(super) struct EventEntry {
    kind: EntryKind,
    ordinal: u16,
    serial: Option<u64>,
    at: Instant,
}
impl EntryKind {
    fn index(self) -> Option<usize> {
        match self {
            Self::Top => Some(0),
            Self::Resource => Some(1),
            Self::Auth | Self::Other => None,
        }
    }
    fn cap(self) -> u16 {
        if self == Self::Top { 2 } else { 4 }
    }
}
#[derive(Default)]
pub(super) struct EntryLedger {
    serial: AtomicU64,
    counts: [AtomicU16; 2],
    slots: [[AtomicU8; 2]; 2],
    pending: AtomicU64,
    auth_count: AtomicU16,
    resource_refused: AtomicU16,
    started: Option<Instant>,
    failure_ns_plus_one: AtomicU64,
    uncertain: AtomicBool,
    interrupted: AtomicBool,
    serial_overflow: AtomicBool,
    action_slots: AtomicU32,
    slot_failed: AtomicBool,
}
impl EntryLedger {
    fn with_start(started: Instant) -> Self {
        Self {
            started: Some(started),
            ..Self::default()
        }
    }
    fn note_failure(&self, at: Instant) {
        let Some(started) = self.started else {
            self.uncertain.store(true, Ordering::SeqCst);
            return;
        };
        let value = at
            .checked_duration_since(started)
            .filter(|_| at >= started)
            .and_then(|d| u64::try_from(d.as_nanos()).ok())
            .and_then(|n| n.checked_add(1));
        if let Some(value) = value {
            let _ =
                self.failure_ns_plus_one
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old| {
                        Some(if old == 0 { value } else { old.min(value) })
                    });
        } else {
            self.serial_overflow.store(true, Ordering::SeqCst);
        }
        self.uncertain.store(true, Ordering::SeqCst);
    }
    fn failure_at(&self) -> Option<Instant> {
        let nanos = self
            .failure_ns_plus_one
            .load(Ordering::SeqCst)
            .checked_sub(1)?;
        self.started?.checked_add(Duration::from_nanos(nanos))
    }
    fn enter(&self, kind: EntryKind) -> EventEntry {
        let at = Instant::now();
        let serial = self
            .serial
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_add(1))
            .ok()
            .and_then(|n| n.checked_add(1));
        if serial.is_none() {
            self.serial_overflow.store(true, Ordering::SeqCst);
            self.note_failure(at);
        }
        if self.pending.load(Ordering::SeqCst) != 0 {
            self.interrupted.store(true, Ordering::SeqCst);
            self.note_failure(at);
        }
        let ordinal = if kind == EntryKind::Auth {
            self.auth_count
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
                    Some(n.saturating_add(1).min(2))
                })
                .unwrap_or(2)
                .saturating_add(1)
                .min(2)
        } else {
            kind.index().map_or(0, |index| {
                let cap = kind.cap() + 1;
                let old = self.counts[index]
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
                        Some(n.saturating_add(1).min(cap))
                    })
                    .unwrap_or(cap);
                let value = old.saturating_add(1).min(cap);
                if value <= 2 {
                    self.slots[index][usize::from(value - 1)].store(1, Ordering::SeqCst);
                }
                value
            })
        };
        EventEntry {
            kind,
            ordinal,
            serial,
            at,
        }
    }
    fn current(&self, entry: EventEntry) -> bool {
        entry
            .serial
            .is_some_and(|n| self.serial.load(Ordering::SeqCst) == n)
            && !self.uncertain()
            && !self.interrupted()
            && !self.overflowed()
    }
    fn settle(&self, entry: EventEntry, class: EntryClass) -> bool {
        let Some(index) = entry.kind.index() else {
            return true;
        };
        if entry.ordinal > 2 {
            return class == EntryClass::Invalid;
        }
        if entry.ordinal == 0 {
            return false;
        }
        let value = match class {
            EntryClass::Active => 2,
            EntryClass::Disposal => 3,
            EntryClass::Invalid => 4,
        };
        self.slots[index][usize::from(entry.ordinal - 1)]
            .compare_exchange(1, value, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }
    fn reject(&self, entry: EventEntry) {
        self.note_failure(Instant::now());
        self.settle(entry, EntryClass::Invalid);
    }
    fn reserve_disposal(&self, entry: EventEntry) -> bool {
        if entry.ordinal != 2 || entry.kind == EntryKind::Other || !self.current(entry) {
            return false;
        }
        let Some(serial) = entry.serial else {
            return false;
        };
        if self
            .pending
            .compare_exchange(0, serial, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            self.interrupted.store(true, Ordering::SeqCst);
            false
        } else {
            true
        }
    }
    fn finish_disposal(&self, entry: EventEntry) -> bool {
        entry.serial.is_some_and(|n| {
            self.pending
                .compare_exchange(n, 0, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
        })
    }
    fn uncertain(&self) -> bool {
        self.uncertain.load(Ordering::SeqCst)
    }
    fn interrupted(&self) -> bool {
        self.interrupted.load(Ordering::SeqCst)
    }
    fn overflowed(&self) -> bool {
        self.serial_overflow.load(Ordering::SeqCst)
            || self.auth_count.load(Ordering::SeqCst) > 1
            || self.counts.iter().any(|n| n.load(Ordering::SeqCst) > 2)
    }
    fn partition(&self, kind: EntryKind) -> Partition {
        let Some(index) = kind.index() else {
            return Partition::default();
        };
        let delivered = self.counts[index].load(Ordering::SeqCst);
        let mut row = Partition {
            deliveries: delivered,
            invalid: delivered.saturating_sub(2),
            ..Partition::default()
        };
        for slot in self.slots[index].iter().take(usize::from(delivered.min(2))) {
            match slot.load(Ordering::SeqCst) {
                2 => row.active += 1,
                3 => row.qualified_disposal += 1,
                4 => row.invalid += 1,
                _ => row.pending += 1,
            }
        }
        row
    }
    fn unsettled(&self) -> bool {
        self.pending.load(Ordering::SeqCst) != 0
            || self.partition(EntryKind::Top).pending != 0
            || self.partition(EntryKind::Resource).pending != 0
            || self.action_slots.load(Ordering::SeqCst) != 0
            || self.slot_failed.load(Ordering::SeqCst)
    }
}
// Optional first-extra diagnostics use the actual entry ledger, before this
// entry's own count, rather than the last published receipt's stale totals.
fn diagnostic_extra_counts(
    ledger: &EntryLedger,
    entry: EventEntry,
    navigation: &Navigation,
) -> Option<(usize, u64)> {
    if entry.ordinal != 2 {
        return None;
    }
    let kind = entry.kind.index()?;
    let mut counts = ledger.counts.each_ref().map(|n| n.load(Ordering::SeqCst));
    counts[kind] = entry.ordinal - 1;
    Some((
        kind,
        u64::from(counts[0])
            | (u64::from(counts[1]) << 16)
            | (u64::from(navigation.attempts) << 32)
            | (u64::from(navigation.grants) << 48),
    ))
}
fn refresh_delivery_facts(facts: &mut Facts, ledger: &EntryLedger) {
    let top = ledger.partition(EntryKind::Top);
    let resource = ledger.partition(EntryKind::Resource);
    facts.navigation.top_events = top.deliveries;
    facts.resource.total = resource.deliveries;
    facts.auth.callbacks = ledger.auth_count.load(Ordering::SeqCst);
    facts.resource.refused = ledger.resource_refused.load(Ordering::SeqCst);
    facts.post_target.top.replace_counts(top);
    facts.post_target.resource.replace_counts(resource);
    if ledger.overflowed() {
        facts.overflow = true;
        if facts.failure.is_none() {
            facts.failure = Some("http401Overflow")
        }
    }
    if ledger.uncertain() || ledger.interrupted() {
        facts.unknown = true;
        if facts.failure.is_none() {
            facts.failure = Some("http401Binding")
        }
    }
}
#[derive(Default)]
struct NavigationProgress {
    entered: bool,
    returned: Option<bool>,
    settled: bool,
    invalid: bool,
}
impl NavigationProgress {
    fn enter(&mut self) -> bool {
        if self.entered {
            self.invalid = true;
            false
        } else {
            self.entered = true;
            true
        }
    }
    fn returned(&mut self, good: bool) {
        if self.returned.is_some() {
            self.invalid = true
        } else {
            self.returned = Some(good)
        }
    }
    fn settle(&mut self, current: bool) {
        if self.entered && self.returned == Some(true) && current && !self.invalid {
            self.settled = true
        } else {
            self.invalid = true
        }
    }
    fn sealable(&self) -> bool {
        self.entered && self.returned == Some(true) && self.settled && !self.invalid
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RegistrationToken {
    Event(i64),
    Filter,
}
pub(super) struct RegistrationBinding {
    key: u8,
    entered: Cell<bool>,
    returned: Cell<Option<bool>>,
    token: Cell<Option<RegistrationToken>>,
    ack: Cell<bool>,
    removing: Cell<bool>,
    remove_returned: Cell<Option<bool>>,
    remove_ack: Cell<bool>,
    invalid: Cell<bool>,
}
impl RegistrationBinding {
    pub(super) fn new(key: u8) -> Self {
        Self {
            key,
            entered: Cell::new(false),
            returned: Cell::new(None),
            token: Cell::new(None),
            ack: Cell::new(false),
            removing: Cell::new(false),
            remove_returned: Cell::new(None),
            remove_ack: Cell::new(false),
            invalid: Cell::new(false),
        }
    }
    pub(super) fn add_entered(&self) {
        if self.entered.replace(true) {
            self.invalid.set(true)
        }
    }
    pub(super) fn add_returned(&self, good: bool, token: Option<RegistrationToken>) {
        if self.returned.replace(Some(good)).is_some() {
            self.invalid.set(true)
        }
        if good {
            self.token.set(token)
        }
    }
    pub(super) fn add_ack(&self, current: bool) {
        if self.ack.get() {
            self.invalid.set(true)
        }
        self.ack.set(
            current
                && self.entered.get()
                && self.returned.get() == Some(true)
                && self.token.get().is_some(),
        );
    }
    pub(super) fn remove_entered(&self) {
        if self.removing.replace(true) {
            self.invalid.set(true)
        }
    }
    pub(super) fn remove_returned(&self, good: bool) {
        if self.remove_returned.replace(Some(good)).is_some() {
            self.invalid.set(true)
        }
    }
    pub(super) fn remove_ack(&self, current: bool) {
        self.remove_ack
            .set(current && self.remove_returned.get() == Some(true))
    }
    fn usable(&self) -> bool {
        !self.invalid.get()
            && self.entered.get()
            && self.returned.get() == Some(true)
            && self.token.get().is_some()
            && self.ack.get()
            && (!self.removing.get()
                || (self.remove_returned.get() == Some(true) && self.remove_ack.get()))
    }
}
fn record_first_top_metadata(
    navigation: &mut Navigation,
    first: bool,
    qualified: bool,
    id: bool,
    cancel: Option<bool>,
) {
    if first {
        navigation.qualification_matched = Some(qualified);
        navigation.navigation_id_readable = Some(id);
        navigation.cancel_false = cancel
    }
}
fn observed_stat_bytes(success: bool, bytes: u64) -> Option<u64> {
    success.then_some(bytes)
}

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
    pub post_target: PostTarget,
    #[serde(skip)]
    pub failure: Option<&'static str>,
}
impl Default for Facts {
    fn default() -> Self {
        Self {
            version: 2,
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
            post_target: PostTarget::default(),
            failure: None,
        }
    }
}
impl Facts {
    // Historical checked action only. This does not authorize any action and is
    // deliberately separate from final success after a later invalid delivery.
    pub fn target_history(&self) -> bool {
        self.receipts.target == 1
            && self.post_target.admission.count == 1
            && self.post_target.admission.qualified == Some(true)
            && self.post_target.admission.first_close_matched == Some(true)
            && self.navigation.attempts == 1
            && self.navigation.grants == 1
            && self.resource.passed == 1
            && self.auth.callbacks == 1
            && self.auth.cancel_attempts == 1
            && self.auth.cancel_successes == 1
            && self.auth.cancel_readbacks == 1
            && self.server.request_valid == Some(true)
            && self.server.response_complete == Some(true)
            && self.server.sent_bytes == 139
            && [
                self.navigation.hresult_succeeded,
                self.navigation.qualification_matched,
                self.navigation.navigation_id_readable,
                self.navigation.cancel_false,
                self.resource.exact_document_get,
                self.resource.qualification_matched,
                self.auth.cancel_true,
                self.auth.uri_matched,
                self.auth.challenge_matched,
                self.auth.sender_matched,
                self.auth.post_drop_current,
                self.auth.within_attempt_deadline,
                self.binding.request_matched,
                self.binding.generation_matched,
                self.binding.core_matched,
                self.binding.s2_install_matched,
                self.binding.guard_install_matched,
                self.binding.post_drop_current,
            ]
            .into_iter()
            .all(|v| v == Some(true))
    }
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
                n.grants,
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
            && self.post_target.positive()
            && n.top_events == self.post_target.top.deliveries
            && r.total == self.post_target.resource.deliveries
            && r.refused == self.post_target.resource.qualified_disposal
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
            && self
                .cleanup
                .first_close_at_ms
                .zip(self.cleanup.target_at_ms)
                .zip(self.cleanup.final_at_ms)
                .zip(self.cleanup.cleanup_elapsed_ms)
                .is_some_and(|(((close, target), final_at), elapsed)| {
                    close <= target
                        && target <= final_at
                        && elapsed < 10_000
                        && final_at.checked_sub(close).is_some_and(|difference| {
                            difference == elapsed || difference == elapsed + 1
                        })
                        && self.post_target.final_order(close, target, final_at)
                })
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
    fn disposal_clock_facts(&self, entry: Instant, now: Instant) -> (bool, bool) {
        let ordered = now >= entry;
        (
            self.before(entry) && self.before(now),
            ordered && self.disposal_before(entry) && self.disposal_before(now),
        )
    }
    fn close(&mut self, at: Instant) {
        if self.first_close.is_none() {
            self.first_close = Some(at);
        }
    }
    fn target_at_entry(&self, entry: Instant) -> bool {
        self.target.is_some_and(|target| target <= entry)
    }
    fn disposal_before(&self, now: Instant) -> bool {
        self.before(now)
            && self.target.is_some_and(|target| now >= target)
            && self
                .first_close
                .zip(self.target)
                .is_some_and(|(close, target)| {
                    close <= target
                        && now >= close
                        && close
                            .checked_add(Duration::from_secs(10))
                            .is_some_and(|end| now < end)
                })
    }
    fn ms(&self, at: Instant) -> Option<u64> {
        at.checked_duration_since(self.started)
            .and_then(|v| u64::try_from(v.as_millis()).ok())
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
                    && self
                        .target
                        .is_none_or(|target| at <= target && target <= now)
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
    entries: Arc<EntryLedger>,
    partial: bool,
}
impl Receipt {
    pub fn matches(
        &self,
        request: &Code0WindowRequest,
        install: &super::super::fixture::NativeRefusalInstallReceipt,
    ) -> bool {
        !self.partial
            && &self.request == request
            && install.matches(request)
            && Arc::ptr_eq(&self.installation, &install.installation)
    }
    pub fn same_guard(&self, previous: &Self) -> bool {
        self.request == previous.request && Arc::ptr_eq(&self.guard, &previous.guard)
    }
    pub fn facts(&self) -> &Facts {
        &self.facts
    }
    pub fn live_facts(&self) -> Facts {
        let mut facts = self.facts.clone();
        refresh_delivery_facts(&mut facts, &self.entries);
        facts
    }
    pub fn partial_matches(&self, request: &Code0WindowRequest, started: Instant) -> bool {
        self.partial
            && &self.request == request
            && self.timing.matches_start(started)
            && self.facts.failure.is_some()
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

pub(super) struct Intent {
    request: Code0WindowRequest,
    started: Instant,
    entries: Arc<EntryLedger>,
    failure: Option<&'static str>,
    first_close: Option<Instant>,
    close_published: bool,
}
impl Intent {
    pub(super) fn new(request: Code0WindowRequest, started: Instant) -> Self {
        Self {
            request,
            started,
            entries: Arc::new(EntryLedger::with_start(started)),
            failure: None,
            first_close: None,
            close_published: false,
        }
    }
    pub(super) fn matches(&self, request: &Code0WindowRequest, started: Instant) -> bool {
        &self.request == request && self.started == started
    }
    pub(super) fn for_request(&self, request: &Code0WindowRequest) -> bool {
        &self.request == request && Timing::new(self.started).before(Instant::now())
    }
    pub(super) fn entries(&self) -> Arc<EntryLedger> {
        Arc::clone(&self.entries)
    }
}
pub(in super::super) fn select_intent(
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
        || o.http401_intent.is_some()
        || !Timing::new(started).before(Instant::now())
    {
        return Err(Code0SessionFailure::PolicyRejected);
    }
    o.http401_intent = Some(Intent::new(request.clone(), started));
    Ok(())
}
struct HistoricalAuthTarget {
    ctx: Context,
    s2_rows: Vec<Rc<RegistrationBinding>>,
    resource: Rc<RegistrationBinding>,
    completed: Rc<RegistrationBinding>,
    filter: Rc<RegistrationBinding>,
    started: Instant,
    end: Instant,
    first_close: Instant,
    target: Instant,
    guard: Arc<AtomicBool>,
    navigation: Navigation,
    resource_history: Resource,
    auth: Auth,
    nav_id: u64,
    request_valid: Option<bool>,
    response_complete: Option<bool>,
    sent_bytes: u16,
}
// The guard owns each action-local COM reference until its own final Drop.
// Actual Active remains live after the value's Release; no Owner borrow escapes.
pub(super) struct Guarded<T> {
    value: T,
    _active: Option<Active>,
}
impl<T> Guarded<T> {
    pub(super) fn untracked(value: T) -> Self {
        Self {
            value,
            _active: None,
        }
    }
}
impl<T> std::ops::Deref for Guarded<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}
fn clone_slot<T: Clone>(
    owner: &Rc<RefCell<Owner>>,
    bit: u32,
    take: fn(&mut Instance) -> Option<T>,
    restore: fn(&mut Instance, T) -> Option<T>,
) -> Option<Guarded<T>> {
    let (entries, callbacks, request) = {
        let o = owner.borrow();
        let i = o.instance.as_ref()?;
        (
            Arc::clone(&i.http401_entries),
            Arc::clone(&i.callbacks),
            i.request.clone(),
        )
    };
    if entries.action_slots.fetch_or(bit, Ordering::SeqCst) & bit != 0 {
        entries.slot_failed.store(true, Ordering::SeqCst);
        fail(owner, "http401Cleanup", true);
        return None;
    }
    let active = Active::enter(&callbacks);
    let original = {
        let mut o = owner.borrow_mut();
        o.instance.as_mut().and_then(take)
    };
    let Some(original) = original else {
        entries.slot_failed.store(true, Ordering::SeqCst);
        fail(owner, "http401Cleanup", true);
        return None;
    };
    let value = original.clone();
    let mut original = Some(original);
    let (replaced, matched) = {
        let mut o = owner.borrow_mut();
        let matched = !o.unmatched;
        if let Some(i) = o.instance.as_mut() {
            let same = matched
                && i.request == request
                && Arc::ptr_eq(&i.callbacks, &callbacks)
                && Arc::ptr_eq(&i.http401_entries, &entries)
                && !i.released;
            if same {
                (restore(i, original.take().unwrap()), true)
            } else {
                (None, false)
            }
        } else {
            (None, false)
        }
    };
    let replaced_any = replaced.is_some();
    drop(replaced);
    drop(original);
    if matched && !replaced_any {
        entries.action_slots.fetch_and(!bit, Ordering::SeqCst);
        Some(Guarded {
            value,
            _active: Some(active),
        })
    } else {
        entries.slot_failed.store(true, Ordering::SeqCst);
        drop(value);
        fail(owner, "http401Binding", true);
        None
    }
}
pub(super) fn action_core(owner: &Rc<RefCell<Owner>>) -> Option<Guarded<ICoreWebView2>> {
    clone_slot(
        owner,
        1,
        |i| i.core.take(),
        |i, v| {
            if i.core.is_none() {
                i.core = Some(v);
                None
            } else {
                Some(v)
            }
        },
    )
}
pub(super) fn action_controller(
    owner: &Rc<RefCell<Owner>>,
) -> Option<Guarded<ICoreWebView2Controller>> {
    clone_slot(
        owner,
        2,
        |i| i.controller.take(),
        |i, v| {
            if i.controller.is_none() {
                i.controller = Some(v);
                None
            } else {
                Some(v)
            }
        },
    )
}
pub(super) fn action_environment(
    owner: &Rc<RefCell<Owner>>,
) -> Option<Guarded<ICoreWebView2Environment>> {
    clone_slot(
        owner,
        4,
        |i| i.environment.take(),
        |i, v| {
            if i.environment.is_none() {
                i.environment = Some(v);
                None
            } else {
                Some(v)
            }
        },
    )
}
pub(super) fn action_environment5(
    owner: &Rc<RefCell<Owner>>,
) -> Option<Guarded<ICoreWebView2Environment5>> {
    clone_slot(
        owner,
        8,
        |i| i.environment5.take(),
        |i, v| {
            if i.environment5.is_none() {
                i.environment5 = Some(v);
                None
            } else {
                Some(v)
            }
        },
    )
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
    entries: Arc<EntryLedger>,
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
            entries: Arc::clone(&i.http401_entries),
        }
    }
    pub(super) fn selected(&self) -> bool {
        self.selected.load(Ordering::SeqCst)
    }
    fn actor_same(&self, owner: &Rc<RefCell<Owner>>) -> bool {
        let o = owner.borrow();
        o.instance.as_ref().is_some_and(|i| {
            i.request == self.request
                && i.refusal_registry.http_generation() == self.generation
                && Arc::ptr_eq(&i.callbacks, &self.callbacks)
                && Arc::ptr_eq(&i.refusal_registry.unknown, &self.installation)
                && Arc::ptr_eq(&i.http401_selected, &self.selected)
                && Arc::ptr_eq(&i.http401_entries, &self.entries)
        })
    }
    fn same(&self, owner: &Rc<RefCell<Owner>>) -> bool {
        self.actor_same(owner)
            && owner
                .borrow()
                .instance
                .as_ref()
                .and_then(|i| i.http401.as_ref())
                .is_some_and(|s| {
                    s.core
                        .as_ref()
                        .zip(self.core.upgrade().as_ref())
                        .is_some_and(|(a, b)| Rc::ptr_eq(a, b))
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
            && !self.entries.uncertain()
            && !self.entries.interrupted()
            && !self.entries.overflowed()
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
        let owner = self.owner.upgrade().filter(|o| self.actor_same(o));
        let Some(sender) = sender else {
            if let Some(o) = owner.as_ref() {
                fail(o, "http401Binding", true);
            }
            return false;
        };
        let Some(core) = self.core.upgrade() else {
            if let Some(o) = owner.as_ref() {
                fail(o, "http401Binding", true);
            }
            return false;
        };
        let expected = core.cast::<IUnknown>();
        if expected.is_err()
            && let Some(o) = owner.as_ref()
        {
            fail(o, "http401Binding", true);
        }
        let actual = if expected.is_ok() {
            sender.cast::<IUnknown>()
        } else {
            Err(rejection())
        };
        let matched = expected
            .as_ref()
            .ok()
            .zip(actual.as_ref().ok())
            .map(|(a, b)| a.as_raw() == b.as_raw());
        if matched != Some(true)
            && let Some(o) = owner.as_ref()
        {
            fail(o, "http401Binding", matched.is_none());
        }
        drop(actual);
        drop(expected);
        drop(core);
        matched == Some(true)
    }
    pub(super) fn lost(&self) {
        self.entries.note_failure(Instant::now());
        self.installation.store(true, Ordering::SeqCst);
    }
}

// These hooks are called at the actual S2 native call/return and final Drop ACK.
pub(super) fn registration_failure(owner: &Rc<RefCell<Owner>>) {
    fail(owner, "http401Install", true);
    publish_close(owner);
}
pub(super) fn s2_add_entered(owner: &Rc<RefCell<Owner>>, index: usize) {
    let binding = owner
        .borrow()
        .instance
        .as_ref()
        .and_then(|i| i.refusal_registry.http_binding(index));
    if let Some(binding) = binding {
        binding.add_entered()
    }
}
pub(super) fn s2_add_returned(owner: &Rc<RefCell<Owner>>, index: usize, good: bool, token: i64) {
    let binding = owner
        .borrow()
        .instance
        .as_ref()
        .and_then(|i| i.refusal_registry.http_binding(index));
    if let Some(binding) = binding {
        binding.add_returned(good, good.then_some(RegistrationToken::Event(token)))
    }
}
pub(super) fn s2_add_ack(owner: &Rc<RefCell<Owner>>, index: usize, good: bool) {
    let o = owner.borrow();
    if let Some(i) = o.instance.as_ref()
        && let Some(binding) = i.refusal_registry.http_binding(index)
    {
        binding.add_ack(
            good && !o.unmatched
                && !i.failed
                && !i.closing
                && !i.released
                && !i.cleanup_blocked
                && !i.callbacks.affinity_failed.load(Ordering::SeqCst)
                && !i.refusal_registry.unknown.load(Ordering::SeqCst),
        );
    }
}
pub(super) fn s2_remove_returned(owner: &Rc<RefCell<Owner>>, index: usize, good: bool) {
    let binding = owner
        .borrow()
        .instance
        .as_ref()
        .and_then(|i| i.refusal_registry.http_binding(index));
    if let Some(binding) = binding {
        binding.remove_returned(good)
    }
}
pub(super) fn s2_remove_ack(owner: &Rc<RefCell<Owner>>, index: usize) {
    let o = owner.borrow();
    if let Some(i) = o.instance.as_ref()
        && let Some(binding) = i.refusal_registry.http_binding(index)
    {
        binding.remove_ack(
            !o.unmatched
                && !i.released
                && !i.destroyed
                && !i.cleanup_blocked
                && !i.callbacks.affinity_failed.load(Ordering::SeqCst)
                && !i.refusal_registry.unknown.load(Ordering::SeqCst),
        );
    }
}
fn disposal_snapshot(
    ctx: &Context,
    owner: &Rc<RefCell<Owner>>,
    entry: EventEntry,
    row: &Rc<RegistrationBinding>,
    now: Instant,
) -> (DisposalCommon, bool) {
    let mut proof = DisposalCommon {
        ui_matched: Some(true),
        entry_order_current: Some(ctx.entries.current(entry)),
        ..DisposalCommon::default()
    };
    if !ctx.actor_same(owner) {
        return (proof, false);
    }
    let o = owner.borrow();
    let Some(i) = o.instance.as_ref() else {
        return (proof, false);
    };
    let Some(s) = i.http401.as_ref() else {
        return (proof, false);
    };
    proof.entry_at_ms = s.timing.ms(entry.at);
    proof.target_established_at_entry =
        Some(s.facts.receipts.target == 1 && s.timing.target_at_entry(entry.at));
    proof.closing_admitted_at_entry = Some(
        s.historical
            .as_ref()
            .is_some_and(|history| history.target <= entry.at)
            && s.facts.post_target.admission.count == 1
            && s.facts.post_target.admission.qualified == Some(true)
            && s.facts.post_target.admission.first_close_matched == Some(true),
    );
    proof.request_matched = Some(!o.unmatched && i.request == ctx.request);
    proof.generation_matched = Some(i.refusal_registry.http_generation() == ctx.generation);
    proof.not_released = Some(!i.released);
    proof.not_destroyed = Some(!i.destroyed);
    proof.no_prior_failure = Some(
        s.facts.failure.is_none()
            && !s.facts.unknown
            && !s.facts.overflow
            && !i.cleanup_blocked
            && !ctx.entries.uncertain()
            && !ctx.entries.interrupted()
            && !ctx.entries.overflowed()
            && !ctx.callbacks.affinity_failed.load(Ordering::SeqCst)
            && !ctx.installation.load(Ordering::SeqCst)
            && !s.guard.load(Ordering::SeqCst),
    );
    let (attempt, close) = s.timing.disposal_clock_facts(entry.at, now);
    proof.within_attempt_deadline = Some(attempt);
    proof.within_close_deadline = Some(close);
    let core_bound = s.core.as_ref().is_some_and(|core| {
        ctx.core
            .upgrade()
            .as_ref()
            .is_some_and(|expected| Rc::ptr_eq(core, expected))
    });
    if let Some(history) = s.historical.as_ref() {
        let original = history.ctx.request == ctx.request
            && history.ctx.generation == ctx.generation
            && Arc::ptr_eq(&history.ctx.callbacks, &ctx.callbacks)
            && Arc::ptr_eq(&history.ctx.installation, &ctx.installation)
            && Arc::ptr_eq(&history.ctx.selected, &ctx.selected)
            && Arc::ptr_eq(&history.ctx.entries, &ctx.entries)
            && s.timing.started == history.started
            && s.timing.end == Some(history.end)
            && s.timing.first_close == Some(history.first_close)
            && s.timing.target == Some(history.target)
            && Arc::ptr_eq(&history.guard, &s.guard)
            && s.nav_id == Some(history.nav_id)
            && s.facts.auth == history.auth
            && s.facts.navigation.attempts == history.navigation.attempts
            && s.facts.navigation.grants == history.navigation.grants
            && s.facts.navigation.hresult_succeeded == history.navigation.hresult_succeeded
            && s.facts.navigation.qualification_matched == history.navigation.qualification_matched
            && s.facts.navigation.navigation_id_readable
                == history.navigation.navigation_id_readable
            && s.facts.navigation.cancel_false == history.navigation.cancel_false
            && s.facts.resource.passed == history.resource_history.passed
            && s.facts.resource.exact_document_get == history.resource_history.exact_document_get
            && s.facts.resource.qualification_matched
                == history.resource_history.qualification_matched
            && s.facts.server.request_valid == history.request_valid
            && s.facts.server.response_complete == history.response_complete
            && s.facts.server.sent_bytes == history.sent_bytes;
        let s2 = original
            && i.refusal_registry.http_rows_match(&history.s2_rows)
            && history.s2_rows.iter().all(|r| r.usable());
        let guards = original
            && Rc::ptr_eq(&history.resource, &s.resource_binding)
            && Rc::ptr_eq(&history.completed, &s.completed_binding)
            && Rc::ptr_eq(&history.filter, &s.filter_binding)
            && history.resource.usable()
            && history.completed.usable()
            && history.filter.usable();
        proof.s2_install_matched = Some(s2);
        proof.guard_install_matched = Some(guards);
        let registration = match entry.kind {
            EntryKind::Top => {
                history.s2_rows.first().is_some_and(|r| Rc::ptr_eq(r, row)) && row.key == 0
            }
            EntryKind::Resource => Rc::ptr_eq(&history.resource, row) && row.key == 10,
            EntryKind::Auth | EntryKind::Other => false,
        };
        proof.registration_matched = Some(registration && row.usable());
    }
    let ready = core_bound
        && [
            proof.ui_matched,
            proof.target_established_at_entry,
            proof.closing_admitted_at_entry,
            proof.request_matched,
            proof.generation_matched,
            proof.s2_install_matched,
            proof.guard_install_matched,
            proof.registration_matched,
            proof.not_released,
            proof.not_destroyed,
            proof.no_prior_failure,
            proof.entry_order_current,
            proof.within_attempt_deadline,
            proof.within_close_deadline,
        ]
        .into_iter()
        .all(|v| v == Some(true));
    (proof, ready)
}
fn disposal_current(
    ctx: &Context,
    owner: &Rc<RefCell<Owner>>,
    entry: EventEntry,
    row: &Rc<RegistrationBinding>,
) -> bool {
    let (proof, ready) = disposal_snapshot(ctx, owner, entry, row, Instant::now());
    record_disposal_deadline(ctx, owner, &proof);
    ready
        && entry
            .serial
            .is_some_and(|n| ctx.entries.pending.load(Ordering::SeqCst) == n)
}
fn sender_after_refusal(
    ctx: &Context,
    owner: &Rc<RefCell<Owner>>,
    entry: EventEntry,
    row: &Rc<RegistrationBinding>,
    sender: Option<&ICoreWebView2>,
    current: &mut bool,
) -> Option<bool> {
    if !*current || !disposal_current(ctx, owner, entry, row) {
        *current = false;
        return None;
    }
    let core = owner
        .borrow()
        .instance
        .as_ref()
        .and_then(|i| i.http401.as_ref())
        .and_then(|s| s.core.as_ref())
        .map(Rc::clone);
    // State.core is the only expected interface source. A successful Weak upgrade
    // from a Controls local cannot enter this path after State has taken it.
    let observed = core.as_ref().zip(sender).and_then(|(core, sender)| {
        let expected = core.cast::<IUnknown>();
        if expected.is_err() {
            fail(owner, "http401Binding", true);
        }
        *current &= disposal_current(ctx, owner, entry, row);
        let actual = if *current {
            sender.cast::<IUnknown>()
        } else {
            Err(rejection())
        };
        if actual.is_err() {
            fail(owner, "http401Binding", true);
        }
        *current &= disposal_current(ctx, owner, entry, row);
        let result = expected
            .as_ref()
            .ok()
            .zip(actual.as_ref().ok())
            .map(|(a, b)| a.as_raw() == b.as_raw());
        if result != Some(true) {
            fail(owner, "http401Binding", result.is_none());
        }
        drop(actual);
        *current &= disposal_current(ctx, owner, entry, row);
        drop(expected);
        *current &= disposal_current(ctx, owner, entry, row);
        result
    });
    drop(core);
    *current &= disposal_current(ctx, owner, entry, row);
    observed
}

pub(super) struct S2Guard {
    pub(super) ctx: Context,
    pub(super) row: Rc<RegistrationBinding>,
    pub(super) _lifetime: HandlerLifetime,
}
impl S2Guard {
    fn enter(&self, kind: EntryKind) -> (Active, EventEntry, bool) {
        let active = Active::enter(&self.ctx.callbacks);
        let entry = self.ctx.entries.enter(kind);
        // Immutable selected-handler mode and fixed atomics are all that is
        // consulted before UI/STA. No Weak/Rc/RefCell/COM/Ref.clone on failure.
        let ui = thread::current().id() == self.ctx.ui && is_sta();
        if !ui {
            self.ctx.entries.reject(entry);
            self.ctx
                .callbacks
                .affinity_failed
                .store(true, Ordering::SeqCst);
        }
        if ui && self.ctx.entries.overflowed() {
            reject_entry(&self.ctx, entry, "http401Overflow", false);
        } else if ui && kind == EntryKind::Other {
            reject_entry(
                &self.ctx,
                entry,
                if self.row.key == 1 {
                    "http401Navigation"
                } else {
                    "http401Precondition"
                },
                false,
            );
        }
        (active, entry, ui)
    }
    fn finish_unexpected(&self, entry: EventEntry) {
        self.ctx.entries.reject(entry);
        if let Some(owner) = entry_owner(&self.ctx, entry) {
            cleanup(&owner);
            publish_close(&owner);
            emit(&owner);
        }
    }
}

#[windows::core::implement(ICoreWebView2NavigationStartingEventHandler, Agile = false)]
pub(super) struct HttpTop {
    pub(super) guard: S2Guard,
}
impl ICoreWebView2NavigationStartingEventHandler_Impl for HttpTop_Impl {
    fn Invoke(
        &self,
        sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2NavigationStartingEventArgs>,
    ) -> windows::core::Result<()> {
        let (active, entry, ui) = self.guard.enter(EntryKind::Top);
        if !ui {
            return Ok(());
        }
        let prepared = prepare_top(&self.guard.ctx, entry, Some(&self.guard.row));
        let sender = sender.cloned();
        let args = args.cloned();

        let candidate = capture_top(
            &self.guard.ctx,
            prepared,
            Some(&self.guard.row),
            sender.as_ref(),
            args.as_ref(),
        );
        drop(args);
        drop(sender);
        finish_top(&self.guard.ctx, candidate);
        drop(active);
        Ok(())
    }
}

#[windows::core::implement(ICoreWebView2NavigationStartingEventHandler, Agile = false)]
pub(super) struct HttpChild {
    pub(super) guard: S2Guard,
}
impl ICoreWebView2NavigationStartingEventHandler_Impl for HttpChild_Impl {
    fn Invoke(
        &self,
        sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2NavigationStartingEventArgs>,
    ) -> windows::core::Result<()> {
        let (active, entry, ui) = self.guard.enter(EntryKind::Other);
        if !ui {
            return Ok(());
        }
        let sender = sender.cloned();
        let args = args.cloned();
        reject_entry(&self.guard.ctx, entry, "http401Navigation", false);
        let candidate = child_callback(&self.guard.ctx, args.as_ref());
        drop(args);
        drop(sender);
        finish_child(&self.guard.ctx, candidate);
        self.guard.finish_unexpected(entry);
        drop(active);
        Ok(())
    }
}

#[windows::core::implement(ICoreWebView2BasicAuthenticationRequestedEventHandler, Agile = false)]
pub(super) struct HttpAuth {
    pub(super) guard: S2Guard,
}
impl ICoreWebView2BasicAuthenticationRequestedEventHandler_Impl for HttpAuth_Impl {
    fn Invoke(
        &self,
        sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2BasicAuthenticationRequestedEventArgs>,
    ) -> windows::core::Result<()> {
        let (active, entry, ui) = self.guard.enter(EntryKind::Auth);
        if !ui {
            return Ok(());
        }
        let prepared = prepare_auth(&self.guard.ctx, entry);
        let sender = sender.cloned();
        let args = args.cloned();

        let candidate = capture_auth(&self.guard.ctx, prepared, sender.as_ref(), args.as_ref());
        drop(args);
        drop(sender);
        finish_auth(&self.guard.ctx, candidate);
        drop(active);
        Ok(())
    }
}

#[windows::core::implement(ICoreWebView2LaunchingExternalUriSchemeEventHandler, Agile = false)]
pub(super) struct HttpExternal {
    pub(super) guard: S2Guard,
}
impl ICoreWebView2LaunchingExternalUriSchemeEventHandler_Impl for HttpExternal_Impl {
    fn Invoke(
        &self,
        sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2LaunchingExternalUriSchemeEventArgs>,
    ) -> windows::core::Result<()> {
        let (active, entry, ui) = self.guard.enter(EntryKind::Other);
        if !ui {
            return Ok(());
        }
        let sender = sender.cloned();
        let args = args.cloned();
        reject_entry(
            &self.guard.ctx,
            entry,
            "http401Precondition",
            args.is_none(),
        );
        if let Some(args) = args.as_ref() {
            let result: windows::core::Result<()> = unsafe { args.SetCancel(true) };
            if result.is_err() {
                reject_entry(&self.guard.ctx, entry, "http401Unknown", true);
            }
            drop(result);
        }
        drop(args);
        drop(sender);
        self.guard.finish_unexpected(entry);
        drop(active);
        Ok(())
    }
}

#[windows::core::implement(ICoreWebView2NewWindowRequestedEventHandler, Agile = false)]
pub(super) struct HttpPopup {
    pub(super) guard: S2Guard,
}
impl ICoreWebView2NewWindowRequestedEventHandler_Impl for HttpPopup_Impl {
    fn Invoke(
        &self,
        sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2NewWindowRequestedEventArgs>,
    ) -> windows::core::Result<()> {
        let (active, entry, ui) = self.guard.enter(EntryKind::Other);
        if !ui {
            return Ok(());
        }
        let sender = sender.cloned();
        let args = args.cloned();
        reject_entry(
            &self.guard.ctx,
            entry,
            "http401Precondition",
            args.is_none(),
        );
        if let Some(args) = args.as_ref() {
            let result: windows::core::Result<()> = unsafe { args.SetHandled(true) };
            if result.is_err() {
                reject_entry(&self.guard.ctx, entry, "http401Unknown", true);
            }
            drop(result);
        }
        drop(args);
        drop(sender);
        self.guard.finish_unexpected(entry);
        drop(active);
        Ok(())
    }
}

#[windows::core::implement(ICoreWebView2DownloadStartingEventHandler, Agile = false)]
pub(super) struct HttpDownload {
    pub(super) guard: S2Guard,
}
impl ICoreWebView2DownloadStartingEventHandler_Impl for HttpDownload_Impl {
    fn Invoke(
        &self,
        sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2DownloadStartingEventArgs>,
    ) -> windows::core::Result<()> {
        let (active, entry, ui) = self.guard.enter(EntryKind::Other);
        if !ui {
            return Ok(());
        }
        let sender = sender.cloned();
        let args = args.cloned();
        reject_entry(
            &self.guard.ctx,
            entry,
            "http401Precondition",
            args.is_none(),
        );
        if let Some(args) = args.as_ref() {
            let result: windows::core::Result<()> = unsafe { args.SetCancel(true) };
            if result.is_err() {
                reject_entry(&self.guard.ctx, entry, "http401Unknown", true);
            }
            drop(result);
        }
        drop(args);
        drop(sender);
        self.guard.finish_unexpected(entry);
        drop(active);
        Ok(())
    }
}

#[windows::core::implement(ICoreWebView2ScriptDialogOpeningEventHandler, Agile = false)]
pub(super) struct HttpDialog {
    pub(super) guard: S2Guard,
}
impl ICoreWebView2ScriptDialogOpeningEventHandler_Impl for HttpDialog_Impl {
    fn Invoke(
        &self,
        sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2ScriptDialogOpeningEventArgs>,
    ) -> windows::core::Result<()> {
        let (active, entry, ui) = self.guard.enter(EntryKind::Other);
        if !ui {
            return Ok(());
        }
        let sender = sender.cloned();
        let args = args.cloned();
        reject_entry(
            &self.guard.ctx,
            entry,
            "http401Precondition",
            args.is_none(),
        );
        if let Some(args) = args.as_ref() {
            let result: windows::core::Result<()> = {
                let _ = args;
                Ok(())
            };
            if result.is_err() {
                reject_entry(&self.guard.ctx, entry, "http401Unknown", true);
            }
            drop(result);
        }
        drop(args);
        drop(sender);
        self.guard.finish_unexpected(entry);
        drop(active);
        Ok(())
    }
}

#[windows::core::implement(ICoreWebView2PermissionRequestedEventHandler, Agile = false)]
pub(super) struct HttpPermission {
    pub(super) guard: S2Guard,
}
impl ICoreWebView2PermissionRequestedEventHandler_Impl for HttpPermission_Impl {
    fn Invoke(
        &self,
        sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2PermissionRequestedEventArgs>,
    ) -> windows::core::Result<()> {
        let (active, entry, ui) = self.guard.enter(EntryKind::Other);
        if !ui {
            return Ok(());
        }
        let sender = sender.cloned();
        let args = args.cloned();
        reject_entry(
            &self.guard.ctx,
            entry,
            "http401Precondition",
            args.is_none(),
        );
        if let Some(args) = args.as_ref() {
            let result: windows::core::Result<()> =
                unsafe { args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY) };
            if result.is_err() {
                reject_entry(&self.guard.ctx, entry, "http401Unknown", true);
            }
            drop(result);
        }
        drop(args);
        drop(sender);
        self.guard.finish_unexpected(entry);
        drop(active);
        Ok(())
    }
}

#[windows::core::implement(ICoreWebView2ClientCertificateRequestedEventHandler, Agile = false)]
pub(super) struct HttpCertificate {
    pub(super) guard: S2Guard,
}
impl ICoreWebView2ClientCertificateRequestedEventHandler_Impl for HttpCertificate_Impl {
    fn Invoke(
        &self,
        sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2ClientCertificateRequestedEventArgs>,
    ) -> windows::core::Result<()> {
        let (active, entry, ui) = self.guard.enter(EntryKind::Other);
        if !ui {
            return Ok(());
        }
        let sender = sender.cloned();
        let args = args.cloned();
        reject_entry(
            &self.guard.ctx,
            entry,
            "http401Precondition",
            args.is_none(),
        );
        if let Some(args) = args.as_ref() {
            let result: windows::core::Result<()> = {
                let set = unsafe { args.SetCancel(true) };
                let handled = unsafe { args.SetHandled(true) };
                let good = set.is_ok() && handled.is_ok();
                drop(handled);
                drop(set);
                if good { Ok(()) } else { Err(rejection()) }
            };
            if result.is_err() {
                reject_entry(&self.guard.ctx, entry, "http401Unknown", true);
            }
            drop(result);
        }
        drop(args);
        drop(sender);
        self.guard.finish_unexpected(entry);
        drop(active);
        Ok(())
    }
}

#[windows::core::implement(ICoreWebView2ServerCertificateErrorDetectedEventHandler, Agile = false)]
pub(super) struct HttpTls {
    pub(super) guard: S2Guard,
}
impl ICoreWebView2ServerCertificateErrorDetectedEventHandler_Impl for HttpTls_Impl {
    fn Invoke(
        &self,
        sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2ServerCertificateErrorDetectedEventArgs>,
    ) -> windows::core::Result<()> {
        let (active, entry, ui) = self.guard.enter(EntryKind::Other);
        if !ui {
            return Ok(());
        }
        let sender = sender.cloned();
        let args = args.cloned();
        reject_entry(
            &self.guard.ctx,
            entry,
            "http401Precondition",
            args.is_none(),
        );
        if let Some(args) = args.as_ref() {
            let result: windows::core::Result<()> =
                unsafe { args.SetAction(COREWEBVIEW2_SERVER_CERTIFICATE_ERROR_ACTION_CANCEL) };
            if result.is_err() {
                reject_entry(&self.guard.ctx, entry, "http401Unknown", true);
            }
            drop(result);
        }
        drop(args);
        drop(sender);
        self.guard.finish_unexpected(entry);
        drop(active);
        Ok(())
    }
}

#[windows::core::implement(ICoreWebView2WebResourceRequestedEventHandler, Agile = false)]
struct HttpResource {
    ctx: Context,
    row: Rc<RegistrationBinding>,
    environment: GuardEnvironment,
}
impl ICoreWebView2WebResourceRequestedEventHandler_Impl for HttpResource_Impl {
    fn Invoke(
        &self,
        sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2WebResourceRequestedEventArgs>,
    ) -> windows::core::Result<()> {
        let active = Active::enter(&self.ctx.callbacks);
        let entry = self.ctx.entries.enter(EntryKind::Resource);
        if thread::current().id() != self.ctx.ui || !is_sta() {
            self.ctx.entries.reject(entry);
            self.ctx
                .callbacks
                .affinity_failed
                .store(true, Ordering::SeqCst);
            return Ok(());
        }
        if self.ctx.entries.overflowed() {
            reject_entry(&self.ctx, entry, "http401Overflow", false);
        }
        let prepared = prepare_resource(&self.ctx, entry, Some(&self.row));
        let sender = sender.cloned();
        let args = args.cloned();
        let candidate = capture_resource(
            &self.ctx,
            prepared,
            Some(&self.row),
            sender.as_ref(),
            args.as_ref(),
            self.environment.environment.as_deref(),
        );
        drop(args);
        drop(sender);
        finish_resource(&self.ctx, candidate);
        drop(active);
        Ok(())
    }
}
#[windows::core::implement(ICoreWebView2NavigationCompletedEventHandler, Agile = false)]
struct HttpCompleted {
    ctx: Context,
    _row: Rc<RegistrationBinding>,
    _lifetime: HandlerLifetime,
}
impl ICoreWebView2NavigationCompletedEventHandler_Impl for HttpCompleted_Impl {
    fn Invoke(
        &self,
        sender: windows::core::Ref<'_, ICoreWebView2>,
        args: windows::core::Ref<'_, ICoreWebView2NavigationCompletedEventArgs>,
    ) -> windows::core::Result<()> {
        let active = Active::enter(&self.ctx.callbacks);
        let entry = self.ctx.entries.enter(EntryKind::Other);
        if thread::current().id() != self.ctx.ui || !is_sta() {
            self.ctx.entries.reject(entry);
            self.ctx
                .callbacks
                .affinity_failed
                .store(true, Ordering::SeqCst);
            return Ok(());
        }
        let sender = sender.cloned();
        let args = args.cloned();
        let candidate = completed_callback(&self.ctx, sender.as_ref(), args.as_ref());
        drop(args);
        drop(sender);
        if !self.ctx.entries.current(entry) {
            reject_entry(&self.ctx, entry, "http401Completion", false);
        }
        finish_completed(&self.ctx, candidate);
        drop(active);
        Ok(())
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
    transport_observation: TransportObservation,
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
    navigation_progress: NavigationProgress,
    historical: Option<HistoricalAuthTarget>,
    resource_binding: Rc<RegistrationBinding>,
    completed_binding: Rc<RegistrationBinding>,
    filter_binding: Rc<RegistrationBinding>,
}
impl State {
    // Selected UI entry only, before preparation reservation/native capture.
    // kind0=first extra top, kind1=first extra resource. No URI/ID/native query.
    fn diagnostic_extra(&self, entry: EventEntry, native: [bool; 2], live: bool) {
        let Some(diagnostic) = &self.ctx.callbacks.diagnostics else {
            return;
        };
        if !diagnostic.http401_enabled() {
            return;
        }
        let f = &self.facts;
        let Some((kind, counts)) = diagnostic_extra_counts(&self.ctx.entries, entry, &f.navigation)
        else {
            return;
        };
        let phase = match f.phase {
            "Preparing" => 1,
            "Armed" => 2,
            "Writing" => 3,
            "AwaitAuth" => 4,
            "Passed" => 5,
            "Revoked" => 6,
            _ => 0,
        };
        let active = self.ctx.callbacks.active.load(Ordering::SeqCst);
        let flags = u64::from(live)
            | (u64::from(native[0]) << 1)
            | (u64::from(native[1]) << 2)
            | (u64::from(self.closing) << 3)
            | (u64::from(self.in_action) << 4)
            | (u64::from(f.overflow) << 5)
            | (u64::from(f.unknown) << 6)
            | (u64::from(f.failure.is_some()) << 7)
            | (phase << 8)
            | (u64::from(active > 65535) << 16);
        let auth = u64::from(self.ctx.entries.auth_count.load(Ordering::SeqCst))
            | (u64::from(f.auth.cancel_attempts) << 16)
            | (u64::from(f.auth.cancel_successes) << 32)
            | (u64::from(f.auth.cancel_readbacks) << 48);
        let target_site_depth = u64::from(f.receipts.target)
            | (u64::from(diagnostic.http401_failure_line()) << 16)
            | ((active.min(65535) as u64) << 48);
        diagnostic.http401_extra_entry(kind, [flags, counts, auth, target_site_depth]);
    }
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
    #[track_caller]
    fn fail(&mut self, reason: &'static str, at: Instant, unknown: bool) {
        if retain_first_failure(&mut self.facts.failure, reason)
            && let Some(diagnostic) = &self.ctx.callbacks.diagnostics
        {
            diagnostic.http401_failure_site(std::panic::Location::caller().line());
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
            && !self.ctx.entries.unsettled()
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
#[track_caller]
fn fail(owner: &Rc<RefCell<Owner>>, reason: &'static str, unknown: bool) {
    let now = Instant::now();
    let at = owner
        .borrow()
        .instance
        .as_ref()
        .and_then(|i| i.http401_entries.failure_at())
        .unwrap_or(now);
    let mut o = owner.borrow_mut();
    let mut early = false;
    if let Some(i) = o.instance.as_mut() {
        if !i.http401_selected.load(Ordering::SeqCst) {
            return;
        }
        if let Some(s) = i.http401.as_mut() {
            s.fail(reason, at, unknown)
        } else {
            early = true
        }
        i.failed = true;
        i.closing = true;
        if unknown {
            i.cleanup_blocked = true;
            i.http401_entries.uncertain.store(true, Ordering::SeqCst)
        }
    }
    if early && let Some(intent) = o.http401_intent.as_mut() {
        if intent.failure.is_none() {
            intent.failure = Some(reason)
        }
        if intent.first_close.is_none() {
            intent.first_close = Some(at)
        }
    }
}
pub(super) fn publish_close(owner: &Rc<RefCell<Owner>>) {
    let notification = {
        let mut o = owner.borrow_mut();
        let Some(i) = o.instance.as_mut() else { return };
        let observer = i.observer.clone();
        let early = if let Some(s) = i.http401.as_mut() {
            if s.close_published {
                return;
            }
            let Some(at) = s.timing.first_close else {
                return;
            };
            s.close_published = true;
            Some(at)
        } else {
            None
        };
        let at = if let Some(at) = early {
            Some(at)
        } else if let Some(intent) = o.http401_intent.as_mut() {
            if intent.close_published {
                return;
            }
            if intent.first_close.is_none() {
                intent.first_close = intent.entries.failure_at();
            }
            if intent.first_close.is_some() {
                intent.close_published = true
            }
            intent.first_close
        } else {
            None
        };
        observer.zip(at)
    };
    if let Some((observer, at)) = notification {
        observer(super::super::fixture::NativeSmokeObservation::Http401Closing { at })
    }
}
fn emit(owner: &Rc<RefCell<Owner>>) {
    {
        let mut o = owner.borrow_mut();
        if let Some(i) = o.instance.as_mut()
            && let Some(s) = i.http401.as_mut()
        {
            refresh_delivery_facts(&mut s.facts, &s.ctx.entries);
            if let Some(reason) = s.facts.failure {
                let at = s.ctx.entries.failure_at().unwrap_or_else(Instant::now);
                s.fail(reason, at, s.facts.unknown);
                i.failed = true;
                i.closing = true;
            }
        }
    }
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
                    entries: Arc::clone(&s.ctx.entries),
                    partial: false,
                },
            )
        })
    };
    if let Some((observer, receipt)) = notification {
        observer(super::super::fixture::NativeSmokeObservation::Http401(
            Box::new(receipt),
        ));
    }
}

pub(in super::super) fn setup_failure(
    owner: &Rc<RefCell<Owner>>,
    request: &Code0WindowRequest,
    started: Instant,
) -> Result<Receipt, Code0SessionFailure> {
    if !on_ui(owner) {
        return Err(Code0SessionFailure::PolicyRejected);
    }
    let o = owner.borrow();
    let intent = o
        .http401_intent
        .as_ref()
        .filter(|v| v.matches(request, started))
        .ok_or(Code0SessionFailure::PolicyRejected)?;
    let i = o
        .instance
        .as_ref()
        .filter(|i| &i.request == request)
        .ok_or(Code0SessionFailure::PolicyRejected)?;
    let (mut facts, mut timing, guard) = if let Some(s) = i.http401.as_ref() {
        (s.facts.clone(), s.timing.clone(), Arc::clone(&s.guard))
    } else {
        (
            Facts::default(),
            Timing::new(started),
            Arc::clone(&i.refusal_registry.unknown),
        )
    };
    if timing.first_close.is_none() {
        timing.first_close = intent
            .first_close
            .or_else(|| i.http401_entries.failure_at())
    }
    if facts.failure.is_none() {
        facts.failure = Some(intent.failure.unwrap_or("http401Install"))
    }
    refresh_delivery_facts(&mut facts, &i.http401_entries);
    Ok(Receipt {
        request: request.clone(),
        installation: Arc::clone(&i.refusal_registry.unknown),
        guard,
        facts,
        timing,
        entries: Arc::clone(&i.http401_entries),
        partial: true,
    })
}

pub(in super::super) fn begin(
    owner: &Rc<RefCell<Owner>>,
    request: &Code0WindowRequest,
    started: Instant,
) -> Result<(), Code0SessionFailure> {
    if !owner
        .borrow()
        .http401_intent
        .as_ref()
        .is_some_and(|intent| intent.matches(request, started) && intent.failure.is_none())
    {
        return Err(Code0SessionFailure::PolicyRejected);
    }
    let receipt = require_native_refusal_install(owner, request)?;
    if receipt.installed_mask() != 1023
        || receipt.load_rejected()
        || !Timing::new(started).before(Instant::now())
    {
        return Err(Code0SessionFailure::PolicyRejected);
    }
    let ctx = Context::new(owner, request);
    let _active = Active::enter(&ctx.callbacks);
    {
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
        if let Some(diagnostic) = &ctx.callbacks.diagnostics {
            diagnostic.arm_http401();
        }
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
            transport_observation: TransportObservation::new(),
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
            navigation_progress: NavigationProgress::default(),
            historical: None,
            resource_binding: Rc::new(RegistrationBinding::new(10)),
            completed_binding: Rc::new(RegistrationBinding::new(11)),
            filter_binding: Rc::new(RegistrationBinding::new(12)),
        });
    }
    // Reuse the selected slot guard: reserve before Native AddRef, restore only
    // the original empty matching slot, and keep Release outside Owner borrows.
    let Guarded {
        value: environment,
        _active: _environment_active,
    } = action_environment(owner).ok_or(Code0SessionFailure::PolicyRejected)?;
    let cached = Rc::new(environment);
    if !setup_current(&ctx, owner) {
        drop(cached);
        fail(owner, "http401Binding", false);
        cleanup(owner);
        return Err(Code0SessionFailure::PolicyRejected);
    }
    {
        let mut o = owner.borrow_mut();
        o.instance
            .as_mut()
            .unwrap()
            .http401
            .as_mut()
            .unwrap()
            .environment = Some(cached);
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
    let resource_row = owner
        .borrow()
        .instance
        .as_ref()
        .unwrap()
        .http401
        .as_ref()
        .map(|s| Rc::clone(&s.resource_binding))
        .unwrap();
    let handler: ICoreWebView2WebResourceRequestedEventHandler = HttpResource {
        ctx: resource_ctx,
        row: Rc::clone(&resource_row),
        environment: resource_environment,
    }
    .into();
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
    resource_row.add_entered();
    let result = unsafe { core.add_WebResourceRequested(&handler, &mut token) };
    resource_row.add_returned(
        result.is_ok(),
        result.is_ok().then_some(RegistrationToken::Event(token)),
    );
    if result.is_err() {
        fail(owner, "http401Install", true);
    }

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
    resource_row.add_ack(ok && setup_current(&ctx, owner));
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
    let filter_row = owner
        .borrow()
        .instance
        .as_ref()
        .unwrap()
        .http401
        .as_ref()
        .map(|s| Rc::clone(&s.filter_binding))
        .unwrap();
    filter_row.add_entered();
    let filtered = unsafe {
        core22.AddWebResourceRequestedFilterWithRequestSourceKinds(
            windows::core::w!("*"),
            COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
            COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
        )
    };
    filter_row.add_returned(
        filtered.is_ok(),
        filtered.is_ok().then_some(RegistrationToken::Filter),
    );
    if filtered.is_err() {
        fail(owner, "http401Install", true);
    }
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
    filter_row.add_ack(ok && setup_current(&ctx, owner));
    if !ok || !setup_current(&ctx, owner) {
        fail(owner, "http401Install", !ok);
        emit(owner);
        return Err(Code0SessionFailure::PolicyRejected);
    }
    let completed_ctx = ctx.clone();
    let completed_lifetime = HandlerLifetime::new(&ctx.callbacks);
    let completed_row = owner
        .borrow()
        .instance
        .as_ref()
        .unwrap()
        .http401
        .as_ref()
        .map(|s| Rc::clone(&s.completed_binding))
        .unwrap();
    let handler: ICoreWebView2NavigationCompletedEventHandler = HttpCompleted {
        ctx: completed_ctx,
        _row: Rc::clone(&completed_row),
        _lifetime: completed_lifetime,
    }
    .into();
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
    completed_row.add_entered();
    let result = unsafe { core.add_NavigationCompleted(&handler, &mut token) };
    completed_row.add_returned(
        result.is_ok(),
        result.is_ok().then_some(RegistrationToken::Event(token)),
    );
    if result.is_err() {
        fail(owner, "http401Install", true);
    }

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
    completed_row.add_ack(ok && setup_current(&ctx, owner));
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
        && !ctx.entries.uncertain()
        && !ctx.entries.interrupted()
        && !ctx.entries.overflowed()
        && !ctx.entries.slot_failed.load(Ordering::SeqCst)
        && s.timing.before(Instant::now())
}

// A generic native failure is distinct from the original admitted auth-close.
// In particular it cannot be hidden by the expected Instance.failed=true seal.
pub(super) fn external_failure(i: &mut Instance) {
    if i.http401_selected.load(Ordering::SeqCst) {
        let now = Instant::now();
        i.http401_entries.note_failure(now);
        if let Some(s) = i.http401.as_mut() {
            s.fail(
                "http401Cleanup",
                i.http401_entries.failure_at().unwrap_or(now),
                false,
            );
        }
    }
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
            core_close_reserve(&mut s.transport_parts(), listener)
        };
        if let Some(handle) = action {
            let r = unsafe { ws::closesocket(handle) };
            let _error = socket_error(r);
            let ok = r == 0;
            {
                let mut o = owner.borrow_mut();
                let i = o.instance.as_mut().unwrap();
                let s = i.http401.as_mut().unwrap();
                core_close_returned(&mut s.transport_parts(), listener, ok);
                if !ok {
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
        core_cleanup_reserve(&mut s.transport_parts())
    };
    if balance {
        let r = unsafe { ws::WSACleanup() };
        let _error = socket_error(r);
        let mut o = owner.borrow_mut();
        let i = o.instance.as_mut().unwrap();
        let s = i.http401.as_mut().unwrap();
        core_cleanup_returned(&mut s.transport_parts(), r == 0);
        if r != 0 {
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
        let operation = match core_next(
            s.navigate_reserved,
            s.top.committed && s.resource.committed,
            &s.transport_parts(),
        ) {
            CoreNext::Return => return,
            CoreNext::Idle => None,
            CoreNext::Operation(operation) => Some(operation),
        };
        if let Some(operation) = operation {
            match core_reserve(&mut s.transport_parts(), operation, Instant::now()) {
                Ok(false) => return,
                Err(()) => {
                    s.facts.overflow = true;
                    s.fail("http401Overflow", Instant::now(), false);
                    None
                }
                Ok(true) => Some(operation),
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
            core_inventory_return(&mut s.transport_parts(), operation, &outcome);
            s.inventory();
        }
        let current = setup_current(&context, owner) && {
            let mut o = owner.borrow_mut();
            let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
            core_matches(&s.transport_parts(), operation)
        };
        let failed = {
            let mut o = owner.borrow_mut();
            let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
            let timely = s.timing.before(Instant::now());
            core_settle(
                &mut s.transport_parts(),
                operation,
                &outcome,
                current,
                timely,
            )
        };
        // Returned handles are closed on every path, never dropped as a receipt.
        if core_retire_listener(operation, &outcome) {
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
    let once = {
        let mut o = owner.borrow_mut();
        o.instance
            .as_mut()
            .unwrap()
            .http401
            .as_mut()
            .unwrap()
            .navigation_progress
            .enter()
    };
    if !once {
        fail(owner, "http401Navigation", false);
        return;
    }
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
        s.navigation_progress.returned(good);
    }
    if !good {
        fail(owner, "http401Navigation", false);
    }
    drop(result);
    drop(wide);
    drop(url);
    drop(core);
    let settled = good && context.qualified(owner, true, Instant::now());
    owner
        .borrow_mut()
        .instance
        .as_mut()
        .unwrap()
        .http401
        .as_mut()
        .unwrap()
        .navigation_progress
        .settle(settled);
    if !settled {
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
fn joined_fact(a: Option<bool>, b: Option<bool>) -> Option<bool> {
    match (a, b) {
        (Some(false), _) | (_, Some(false)) => Some(false),
        (Some(true), Some(true)) => Some(true),
        _ => None,
    }
}
fn retain_first_failure(failure: &mut Option<&'static str>, reason: &'static str) -> bool {
    if failure.is_none() {
        *failure = Some(reason);
        true
    } else {
        false
    }
}
fn disposal_clock_failed(proof: &DisposalCommon) -> bool {
    proof.within_attempt_deadline == Some(false)
        || (proof.target_established_at_entry == Some(true)
            && proof.closing_admitted_at_entry == Some(true)
            && proof.within_close_deadline == Some(false))
}
fn disposal_failure_reason(
    proof: &DisposalCommon,
    overflow: bool,
    fallback: &'static str,
) -> &'static str {
    // Entry overflow is already observed by the entry seam before clock capture.
    if overflow {
        "http401Overflow"
    } else if disposal_clock_failed(proof) {
        "http401Deadline"
    } else {
        fallback
    }
}
fn record_disposal_deadline(ctx: &Context, owner: &Rc<RefCell<Owner>>, proof: &DisposalCommon) {
    if disposal_clock_failed(proof) {
        // State::fail retains any earlier actual cause; this adds no cleanup,
        // native action, clock origin or retry at a current/settlement check.
        let reason = disposal_failure_reason(proof, ctx.entries.overflowed(), "http401Deadline");
        fail(owner, reason, false);
    }
}
fn settle_common(
    ctx: &Context,
    owner: &Rc<RefCell<Owner>>,
    entry: EventEntry,
    row: &Rc<RegistrationBinding>,
    proof: &mut DisposalCommon,
) -> bool {
    let now = Instant::now();
    let (last, ready) = disposal_snapshot(ctx, owner, entry, row, now);
    record_disposal_deadline(ctx, owner, &last);
    let sender_matched = proof.core_matched;
    macro_rules! retain {
        ($($field:ident),+ $(,)?) => { $(proof.$field = joined_fact(proof.$field, last.$field);)+ };
    }
    retain!(
        ui_matched,
        target_established_at_entry,
        closing_admitted_at_entry,
        request_matched,
        generation_matched,
        s2_install_matched,
        guard_install_matched,
        registration_matched,
        not_released,
        not_destroyed,
        no_prior_failure,
        entry_order_current,
        within_attempt_deadline,
        within_close_deadline
    );
    // Only a real live sender QI pair supplies coreMatched. A last Rc check may
    // invalidate it; it can never replace that native observation with true.
    proof.core_matched = sender_matched.map(|v| v && ready);
    proof.post_drop_current = Some(ready && disposal_current(ctx, owner, entry, row));
    proof.settled_at_ms = owner
        .borrow()
        .instance
        .as_ref()
        .and_then(|i| i.http401.as_ref())
        .and_then(|s| s.timing.ms(now));
    proof.positive()
}
fn entry_owner(ctx: &Context, entry: EventEntry) -> Option<Rc<RefCell<Owner>>> {
    let owner = ctx.owner.upgrade()?;
    if ctx.actor_same(&owner) {
        Some(owner)
    } else {
        ctx.entries.reject(entry);
        ctx.lost();
        None
    }
}
fn reject_entry(ctx: &Context, _entry: EventEntry, reason: &'static str, unknown: bool) {
    ctx.entries.note_failure(Instant::now());
    if let Some(owner) = ctx.owner.upgrade().filter(|o| ctx.actor_same(o)) {
        fail(&owner, reason, unknown);
        cleanup(&owner);
        publish_close(&owner);
    } else {
        ctx.lost();
    }
}
fn capture_top_refusal(
    ctx: &Context,
    entry: EventEntry,
    row: &Rc<RegistrationBinding>,
    sender: Option<&ICoreWebView2>,
    args: Option<&ICoreWebView2NavigationStartingEventArgs>,
    proof: &mut TopDisposalProof,
    qualifying: bool,
) {
    let Some(args) = args else {
        reject_entry(ctx, entry, "http401Navigation", true);
        return;
    };
    // This existing production helper guarantees the getter even on setter Err.
    let mut cancel = BOOL(0);
    let (set, read) = restrictive_pair(
        || unsafe { args.SetCancel(true) },
        || unsafe { args.Cancel(&mut cancel) },
    );
    proof.cancel_attempts = 1;
    proof.cancel_successes = u16::from(set.is_ok());
    proof.cancel_readbacks = 1;
    proof.cancel_true = read.is_ok().then_some(cancel.as_bool());
    if set.is_err() || read.is_err() || proof.cancel_true != Some(true) {
        reject_entry(
            ctx,
            entry,
            "http401Navigation",
            set.is_err() || read.is_err(),
        );
    }
    drop(read);
    drop(set);
    if qualifying
        && proof.cancel_successes == 1
        && proof.cancel_true == Some(true)
        && let Some(owner) = entry_owner(ctx, entry)
    {
        let mut current = disposal_current(ctx, &owner, entry, row);
        proof.common.core_matched =
            sender_after_refusal(ctx, &owner, entry, row, sender, &mut current);
        if !current || proof.common.core_matched != Some(true) {
            reject_entry(
                ctx,
                entry,
                "http401Binding",
                proof.common.core_matched.is_none(),
            );
        }
    }
}
pub(super) struct TopCandidate {
    entry: EventEntry,
    active_path: bool,
    eligible: bool,
    qualified: bool,
    id: Option<u64>,
    cancel_false: Option<bool>,
    proof: Option<TopDisposalProof>,
    reserved: bool,
    recorded: bool,
    url: Option<String>,
}
pub(super) fn top_callback(
    ctx: &Context,
    sender: Option<&ICoreWebView2>,
    args: Option<&ICoreWebView2NavigationStartingEventArgs>,
) -> TopCandidate {
    let entry = ctx.entries.enter(EntryKind::Top);
    let row = ctx.owner.upgrade().and_then(|o| {
        o.borrow()
            .instance
            .as_ref()
            .and_then(|i| i.refusal_registry.http_binding(0))
    });
    top_entered(ctx, entry, row.as_ref(), sender, args)
}
fn top_entered(
    ctx: &Context,
    entry: EventEntry,
    row: Option<&Rc<RegistrationBinding>>,
    sender: Option<&ICoreWebView2>,
    args: Option<&ICoreWebView2NavigationStartingEventArgs>,
) -> TopCandidate {
    capture_top(ctx, prepare_top(ctx, entry, row), row, sender, args)
}
fn prepare_top(
    ctx: &Context,
    entry: EventEntry,
    row: Option<&Rc<RegistrationBinding>>,
) -> TopCandidate {
    let mut candidate = TopCandidate {
        entry,
        active_path: false,
        eligible: false,
        qualified: false,
        id: None,
        cancel_false: None,
        proof: None,
        reserved: false,
        recorded: false,
        url: None,
    };
    let owner = entry_owner(ctx, entry);
    let has_state = owner.as_ref().is_some_and(|o| {
        o.borrow()
            .instance
            .as_ref()
            .is_some_and(|i| i.http401.is_some())
    });
    if entry.ordinal == 2
        && let Some(owner) = owner.as_ref().filter(|_| has_state)
    {
        let live = ctx.qualified(owner, true, Instant::now());
        let o = owner.borrow();
        let i = o.instance.as_ref().unwrap();
        i.http401
            .as_ref()
            .unwrap()
            .diagnostic_extra(entry, [i.closing, i.failed], live);
    }
    if entry.ordinal != 1 || !has_state {
        let mut proof = TopDisposalProof::default();
        let mut qualifying = false;
        if let Some(owner) = owner.as_ref().filter(|_| has_state)
            && let Some(row) = row
        {
            let (common, ready) = disposal_snapshot(ctx, owner, entry, row, Instant::now());
            proof.common = common;
            let post_target = proof.common.target_established_at_entry == Some(true)
                && proof.common.closing_admitted_at_entry == Some(true);
            if post_target {
                candidate.reserved = ctx.entries.reserve_disposal(entry);
                candidate.recorded = {
                    let mut o = owner.borrow_mut();
                    let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
                    let first = s.facts.post_target.top.record_candidate(proof.clone());
                    if !first {
                        s.facts.overflow = true;
                    }
                    first
                };
                qualifying = ready && candidate.reserved && candidate.recorded;
            }
        }
        if !qualifying {
            reject_entry(
                ctx,
                entry,
                disposal_failure_reason(
                    &proof.common,
                    ctx.entries.overflowed(),
                    "http401Navigation",
                ),
                !has_state,
            );
        }
        candidate.eligible = qualifying;
        candidate.proof = Some(proof);
        return candidate;
    }
    let owner = owner.unwrap();
    candidate.active_path = true;
    let live = ctx.qualified(&owner, true, entry.at)
        && ctx.qualified(&owner, true, Instant::now())
        && ctx.entries.current(entry);
    let (eligible, url) = {
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        let eligible =
            s.navigate_reserved && s.facts.navigation.attempts == 1 && s.top.reserve(live);
        (eligible, s.url.clone())
    };
    candidate.eligible = eligible;
    candidate.url = url;
    if !eligible {
        reject_entry(ctx, entry, "http401Navigation", false);
    }
    candidate
}
fn capture_top(
    ctx: &Context,
    mut candidate: TopCandidate,
    row: Option<&Rc<RegistrationBinding>>,
    sender: Option<&ICoreWebView2>,
    args: Option<&ICoreWebView2NavigationStartingEventArgs>,
) -> TopCandidate {
    let entry = candidate.entry;
    if !candidate.active_path {
        let mut proof = candidate.proof.take().unwrap_or_default();
        if let Some(row) = row {
            capture_top_refusal(
                ctx,
                entry,
                row,
                sender,
                args,
                &mut proof,
                candidate.eligible,
            );
        } else if let Some(args) = args {
            let mut cancel = BOOL(0);
            let (set, read) = restrictive_pair(
                || unsafe { args.SetCancel(true) },
                || unsafe { args.Cancel(&mut cancel) },
            );
            drop(read);
            drop(set);
        }
        candidate.proof = Some(proof);
        return candidate;
    }
    let Some(owner) = entry_owner(ctx, entry) else {
        if let Some(args) = args {
            let mut cancel = BOOL(0);
            let (set, read) = restrictive_pair(
                || unsafe { args.SetCancel(true) },
                || unsafe { args.Cancel(&mut cancel) },
            );
            drop(read);
            drop(set);
        }
        return candidate;
    };
    let url = candidate.url.take();
    let Some(args) = args else {
        reject_entry(ctx, entry, "http401Navigation", true);
        return candidate;
    };
    if candidate.eligible
        && ctx.entries.current(entry)
        && ctx.qualified(&owner, true, Instant::now())
        && ctx.sender(sender)
        && ctx.qualified(&owner, true, Instant::now())
        && ctx.entries.current(entry)
    {
        let uri = allocated_string(64, |raw| unsafe { args.Uri(raw) });
        let uri_good = uri
            .as_ref()
            .ok()
            .zip(url.as_ref())
            .is_some_and(|(a, b)| a == b);
        if !uri_good {
            reject_entry(ctx, entry, "http401Navigation", uri.is_err());
        }
        drop(uri);
        if uri_good && ctx.qualified(&owner, true, Instant::now()) && ctx.entries.current(entry) {
            let mut id = 0;
            let result = unsafe { args.NavigationId(&mut id) };
            let good = result.is_ok();
            if good {
                candidate.id = Some(id)
            } else {
                reject_entry(ctx, entry, "http401Navigation", true);
            }
            drop(result);
            if good && ctx.qualified(&owner, true, Instant::now()) && ctx.entries.current(entry) {
                let mut redirected = BOOL(0);
                let result = unsafe { args.IsRedirected(&mut redirected) };
                let good = result.is_ok() && !redirected.as_bool();
                if !good {
                    reject_entry(ctx, entry, "http401Navigation", result.is_err());
                }
                drop(result);
                if good && ctx.qualified(&owner, true, Instant::now()) && ctx.entries.current(entry)
                {
                    let mut cancel = BOOL(1);
                    let result = unsafe { args.Cancel(&mut cancel) };
                    candidate.cancel_false = result.is_ok().then_some(!cancel.as_bool());
                    if candidate.cancel_false != Some(true) {
                        reject_entry(ctx, entry, "http401Navigation", result.is_err());
                    }
                    drop(result);
                    candidate.qualified = candidate.cancel_false == Some(true)
                        && ctx.qualified(&owner, true, Instant::now())
                        && ctx.entries.current(entry);
                }
            }
        }
    }
    drop(url);
    if !candidate.qualified {
        reject_entry(ctx, entry, "http401Navigation", false);
        let mut cancel = BOOL(0);
        let (set, read) = restrictive_pair(
            || unsafe { args.SetCancel(true) },
            || unsafe { args.Cancel(&mut cancel) },
        );
        if set.is_err() || read.is_err() {
            reject_entry(ctx, entry, "http401Navigation", true)
        }
        drop(read);
        drop(set);
    }
    candidate
}
pub(super) fn finish_top(ctx: &Context, mut candidate: TopCandidate) {
    let entry = candidate.entry;
    let Some(owner) = entry_owner(ctx, entry) else {
        if candidate.reserved {
            let _ = ctx.entries.finish_disposal(entry);
        }
        ctx.entries.reject(entry);
        return;
    };
    let mut good = false;
    if candidate.active_path && ctx.same(&owner) {
        let current = candidate.eligible
            && candidate.qualified
            && ctx.qualified(&owner, true, Instant::now())
            && ctx.entries.current(entry);
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        record_first_top_metadata(
            &mut s.facts.navigation,
            entry.ordinal == 1,
            current,
            candidate.id.is_some(),
            candidate.cancel_false,
        );
        if current && s.top.commit(true) {
            s.nav_id = candidate.id;
            s.facts.navigation.grants += 1;
            good = true
        } else {
            s.top.revoke();
        }
    } else if let Some(proof) = candidate.proof.as_mut() {
        let row = {
            owner
                .borrow()
                .instance
                .as_ref()
                .and_then(|i| i.refusal_registry.http_binding(0))
        };
        if candidate.recorded
            && candidate.reserved
            && let Some(row) = row
        {
            good = settle_common(ctx, &owner, entry, &row, &mut proof.common) && proof.positive();
        }
        if candidate.recorded
            && let Some(s) = owner
                .borrow_mut()
                .instance
                .as_mut()
                .and_then(|i| i.http401.as_mut())
        {
            s.facts.post_target.top.proof = Some(proof.clone());
        }
    }
    if candidate.reserved {
        good &= ctx.entries.finish_disposal(entry);
    }
    if good {
        good = ctx.entries.settle(
            entry,
            if candidate.active_path {
                EntryClass::Active
            } else {
                EntryClass::Disposal
            },
        );
    }
    if !good {
        ctx.entries.reject(entry);
        let reason = candidate
            .proof
            .as_ref()
            .map_or("http401Navigation", |proof| {
                disposal_failure_reason(
                    &proof.common,
                    ctx.entries.overflowed(),
                    "http401Navigation",
                )
            });
        fail(&owner, reason, false);
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
struct StatName(windows::core::PWSTR);
impl Drop for StatName {
    fn drop(&mut self) {
        if !self.0.is_null() {
            let raw = std::mem::replace(&mut self.0, windows::core::PWSTR::null());
            unsafe { windows::Win32::System::Com::CoTaskMemFree(Some(raw.0.cast())) };
        }
    }
}
struct ResourceRefusalArgs<'a> {
    sender: Option<&'a ICoreWebView2>,
    args: Option<&'a ICoreWebView2WebResourceRequestedEventArgs>,
    environment: Option<&'a ICoreWebView2Environment>,
}
fn capture_resource_refusal(
    ctx: &Context,
    entry: EventEntry,
    row: &Rc<RegistrationBinding>,
    native: ResourceRefusalArgs<'_>,
    proof: &mut ResourceDisposalProof,
    qualifying: bool,
) {
    let ResourceRefusalArgs {
        sender,
        args,
        environment,
    } = native;
    let _ = ctx
        .entries
        .resource_refused
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
            Some(n.saturating_add(1).min(5))
        });
    let Some((environment, args)) = environment.zip(args) else {
        reject_entry(ctx, entry, "http401Resource", true);
        return;
    };
    let owner = entry_owner(ctx, entry);
    let mut current = qualifying
        && owner
            .as_ref()
            .is_some_and(|o| disposal_current(ctx, o, entry, row));
    let Some(stream) = (unsafe { SHCreateMemStream(None) }) else {
        proof.empty_stream_created = Some(false);
        reject_entry(ctx, entry, "http401Resource", true);
        return;
    };
    proof.empty_stream_created = Some(true);
    current &= owner
        .as_ref()
        .is_some_and(|o| disposal_current(ctx, o, entry, row));
    let seek = unsafe { stream.Seek(0, STREAM_SEEK_SET, None) };
    proof.seek_succeeded = Some(seek.is_ok());
    if seek.is_err() {
        reject_entry(ctx, entry, "http401Resource", true);
    }
    drop(seek);
    current &= owner
        .as_ref()
        .is_some_and(|o| disposal_current(ctx, o, entry, row));
    if proof.seek_succeeded != Some(true) {
        drop(stream);
        return;
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
    proof.fixed_response_created = Some(response.is_ok());
    if response.is_err() {
        reject_entry(ctx, entry, "http401Resource", true);
    }
    current &= owner
        .as_ref()
        .is_some_and(|o| disposal_current(ctx, o, entry, row));
    if let Ok(response) = response {
        let attempts = Cell::new(0);
        let readbacks = Cell::new(0);
        // The same production ordering helper is used for top, auth and resource:
        // the actual getter is unavoidable after an entered setter, including Err.
        let (set, read) = restrictive_pair(
            || {
                attempts.set(1);
                let result = unsafe { args.SetResponse(&response) };
                if result.is_err() {
                    reject_entry(ctx, entry, "http401Resource", true);
                }
                result
            },
            || {
                readbacks.set(1);
                let result = unsafe { args.Response() };
                if result.is_err() {
                    reject_entry(ctx, entry, "http401Resource", true);
                }
                result
            },
        );
        proof.response_set_attempts = attempts.get();
        proof.response_set_successes = u16::from(set.is_ok());
        proof.response_readbacks = readbacks.get();
        drop(set);
        current &= owner
            .as_ref()
            .is_some_and(|o| disposal_current(ctx, o, entry, row));
        if let Ok(read) = read {
            if current && proof.response_set_successes == 1 {
                let mut status = -1;
                let result = unsafe { read.StatusCode(&mut status) };
                proof.status_code = result.is_ok().then_some(status);
                if proof.status_code != Some(403) {
                    reject_entry(ctx, entry, "http401Resource", result.is_err());
                }
                drop(result);
                current &= owner
                    .as_ref()
                    .is_some_and(|o| disposal_current(ctx, o, entry, row));
                if current && proof.status_code == Some(403) {
                    let content = unsafe { read.Content() };
                    proof.content_query_succeeded = Some(content.is_ok());
                    if content.is_err() {
                        reject_entry(ctx, entry, "http401Resource", true);
                    }
                    current &= owner
                        .as_ref()
                        .is_some_and(|o| disposal_current(ctx, o, entry, row));
                    if let Ok(content) = content {
                        if current {
                            let mut stat = STATSTG::default();
                            let result = unsafe { content.Stat(&mut stat, STATFLAG_NONAME) };
                            proof.stat_succeeded = Some(result.is_ok());
                            proof.content_bytes = observed_stat_bytes(result.is_ok(), stat.cbSize);
                            let name = StatName(stat.pwcsName);
                            if result.is_err()
                                || !name.0.is_null()
                                || proof.content_bytes != Some(0)
                            {
                                reject_entry(
                                    ctx,
                                    entry,
                                    "http401Resource",
                                    result.is_err() || !name.0.is_null(),
                                );
                            }
                            drop(result);
                            drop(name);
                            current &= owner
                                .as_ref()
                                .is_some_and(|o| disposal_current(ctx, o, entry, row));
                        }
                        // No Content interface is alive across response QIs.
                        drop(content);
                        current &= owner
                            .as_ref()
                            .is_some_and(|o| disposal_current(ctx, o, entry, row));
                    }
                }
                if current && proof.stat_succeeded == Some(true) && proof.content_bytes == Some(0) {
                    let expected = response.cast::<IUnknown>();
                    if expected.is_err() {
                        reject_entry(ctx, entry, "http401Resource", true);
                    }
                    current &= owner
                        .as_ref()
                        .is_some_and(|o| disposal_current(ctx, o, entry, row));
                    let actual = if current {
                        read.cast::<IUnknown>()
                    } else {
                        Err(rejection())
                    };
                    if actual.is_err() {
                        reject_entry(ctx, entry, "http401Resource", true);
                    }
                    current &= owner
                        .as_ref()
                        .is_some_and(|o| disposal_current(ctx, o, entry, row));
                    proof.response_matched = expected
                        .as_ref()
                        .ok()
                        .zip(actual.as_ref().ok())
                        .map(|(a, b)| a.as_raw() == b.as_raw());
                    if proof.response_matched != Some(true) {
                        reject_entry(
                            ctx,
                            entry,
                            "http401Resource",
                            proof.response_matched.is_none(),
                        );
                    }
                    drop(actual);
                    current &= owner
                        .as_ref()
                        .is_some_and(|o| disposal_current(ctx, o, entry, row));
                    drop(expected);
                    current &= owner
                        .as_ref()
                        .is_some_and(|o| disposal_current(ctx, o, entry, row));
                }
                if current
                    && proof.response_matched == Some(true)
                    && let Some(owner) = owner.as_ref()
                {
                    proof.common.core_matched =
                        sender_after_refusal(ctx, owner, entry, row, sender, &mut current);
                    if proof.common.core_matched != Some(true) {
                        reject_entry(
                            ctx,
                            entry,
                            "http401Binding",
                            proof.common.core_matched.is_none(),
                        );
                    }
                }
            }
            drop(read);
            current &= owner
                .as_ref()
                .is_some_and(|o| disposal_current(ctx, o, entry, row));
        }
        drop(response);
        current &= owner
            .as_ref()
            .is_some_and(|o| disposal_current(ctx, o, entry, row));
    }
    drop(stream);
    current &= owner
        .as_ref()
        .is_some_and(|o| disposal_current(ctx, o, entry, row));
    if qualifying && !current {
        reject_entry(ctx, entry, "http401Resource", false);
    }
}
struct ResourceCandidate {
    entry: EventEntry,
    active_path: bool,
    observed: bool,
    eligible: bool,
    exact: Option<bool>,
    qualified: bool,
    proof: Option<ResourceDisposalProof>,
    reserved: bool,
    recorded: bool,
    environment: Option<Rc<ICoreWebView2Environment>>,
    url: Option<String>,
}
fn prepare_resource(
    ctx: &Context,
    entry: EventEntry,
    row: Option<&Rc<RegistrationBinding>>,
) -> ResourceCandidate {
    let mut candidate = ResourceCandidate {
        entry,
        active_path: false,
        observed: false,
        eligible: false,
        exact: None,
        qualified: false,
        proof: None,
        reserved: false,
        recorded: false,
        environment: None,
        url: None,
    };
    let owner = entry_owner(ctx, entry);
    let has_state = owner.as_ref().is_some_and(|o| {
        o.borrow()
            .instance
            .as_ref()
            .is_some_and(|i| i.http401.is_some())
    });
    let environment = owner.as_ref().filter(|_| has_state).and_then(|o| {
        o.borrow()
            .instance
            .as_ref()
            .and_then(|i| i.http401.as_ref())
            .and_then(|s| s.environment.as_ref())
            .map(Rc::clone)
    });
    if entry.ordinal == 2
        && let Some(owner) = owner.as_ref().filter(|_| has_state)
    {
        let live = ctx.qualified(owner, true, Instant::now());
        let o = owner.borrow();
        let i = o.instance.as_ref().unwrap();
        i.http401
            .as_ref()
            .unwrap()
            .diagnostic_extra(entry, [i.closing, i.failed], live);
    }
    if entry.ordinal != 1 || !has_state {
        let mut proof = ResourceDisposalProof::default();
        let mut qualifying = false;
        if let Some(owner) = owner.as_ref().filter(|_| has_state)
            && let Some(row) = row
        {
            let (common, ready) = disposal_snapshot(ctx, owner, entry, row, Instant::now());
            proof.common = common;
            if proof.common.target_established_at_entry == Some(true)
                && proof.common.closing_admitted_at_entry == Some(true)
            {
                candidate.reserved = ctx.entries.reserve_disposal(entry);
                candidate.recorded = {
                    let mut o = owner.borrow_mut();
                    let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
                    let first = s.facts.post_target.resource.record_candidate(proof.clone());
                    if !first {
                        s.facts.overflow = true;
                    }
                    first
                };
                qualifying = ready && candidate.reserved && candidate.recorded;
            }
        }
        if !qualifying {
            reject_entry(
                ctx,
                entry,
                disposal_failure_reason(&proof.common, ctx.entries.overflowed(), "http401Resource"),
                !has_state,
            );
        }
        candidate.environment = environment;
        candidate.eligible = qualifying;
        candidate.proof = Some(proof);
        return candidate;
    }
    let owner = owner.unwrap();
    candidate.active_path = true;
    let live = ctx.qualified(&owner, true, entry.at)
        && ctx.qualified(&owner, true, Instant::now())
        && ctx.entries.current(entry);
    let (eligible, url) = {
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        (s.top.committed && s.resource.reserve(live), s.url.clone())
    };
    candidate.eligible = eligible;
    candidate.environment = environment;
    candidate.url = url;
    if !eligible {
        reject_entry(ctx, entry, "http401Resource", false);
    }
    candidate
}
fn capture_resource(
    ctx: &Context,
    mut candidate: ResourceCandidate,
    row: Option<&Rc<RegistrationBinding>>,
    sender: Option<&ICoreWebView2>,
    args: Option<&ICoreWebView2WebResourceRequestedEventArgs>,
    fallback_environment: Option<&ICoreWebView2Environment>,
) -> ResourceCandidate {
    let entry = candidate.entry;
    candidate.observed = args.is_some();
    let environment = candidate.environment.take();
    let available = environment.as_deref().or(fallback_environment);
    if !candidate.active_path {
        let mut proof = candidate.proof.take().unwrap_or_default();
        if let Some(row) = row {
            capture_resource_refusal(
                ctx,
                entry,
                row,
                ResourceRefusalArgs {
                    sender,
                    args,
                    environment: available,
                },
                &mut proof,
                candidate.eligible,
            );
        } else {
            let _ = ctx.entries.resource_refused.fetch_update(
                Ordering::SeqCst,
                Ordering::SeqCst,
                |n| Some(n.saturating_add(1).min(5)),
            );
            if let Some((environment, args)) = available.zip(args) {
                let _ = empty_403(environment, args);
            }
        }
        drop(environment);
        candidate.proof = Some(proof);
        return candidate;
    }
    let Some(owner) = entry_owner(ctx, entry) else {
        if let Some((environment, args)) = available.zip(args) {
            let _ = empty_403(environment, args);
        }
        drop(environment);
        return candidate;
    };
    let url = candidate.url.take();
    let Some(args) = args else {
        reject_entry(ctx, entry, "http401Resource", true);
        return candidate;
    };
    if candidate.eligible
        && ctx.entries.current(entry)
        && ctx.qualified(&owner, true, Instant::now())
        && ctx.sender(sender)
        && ctx.qualified(&owner, true, Instant::now())
        && ctx.entries.current(entry)
    {
        let request = unsafe { args.Request() };
        if let Ok(request) = request {
            if ctx.qualified(&owner, true, Instant::now()) && ctx.entries.current(entry) {
                let uri = allocated_string(64, |raw| unsafe { request.Uri(raw) });
                let uri_available = uri.is_ok();
                let uri_good = uri
                    .as_ref()
                    .ok()
                    .zip(url.as_ref())
                    .is_some_and(|(a, b)| a == b);
                drop(uri);
                if ctx.qualified(&owner, true, Instant::now()) && ctx.entries.current(entry) {
                    let method = allocated_string(64, |raw| unsafe { request.Method(raw) });
                    let method_available = method.is_ok();
                    let method_good = method.as_ref().is_ok_and(|v| v == "GET");
                    drop(method);
                    if ctx.qualified(&owner, true, Instant::now()) && ctx.entries.current(entry) {
                        let mut context = COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL;
                        let r = unsafe { args.ResourceContext(&mut context) };
                        candidate.exact = (uri_available && method_available && r.is_ok())
                            .then_some(
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
        candidate.qualified = candidate.exact == Some(true)
            && ctx.qualified(&owner, true, Instant::now())
            && ctx.entries.current(entry);
    }
    if !candidate.qualified {
        reject_entry(ctx, entry, "http401Resource", false);
        let _ =
            ctx.entries
                .resource_refused
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
                    Some(n.saturating_add(1).min(5))
                });
        let denied = available.is_some_and(|environment| empty_403(environment, args));
        if !denied {
            reject_entry(ctx, entry, "http401Resource", true);
        }
    }
    drop(environment);
    drop(url);
    candidate
}
fn finish_resource(ctx: &Context, mut candidate: ResourceCandidate) {
    let entry = candidate.entry;
    let Some(owner) = entry_owner(ctx, entry) else {
        if candidate.reserved {
            let _ = ctx.entries.finish_disposal(entry);
        }
        ctx.entries.reject(entry);
        return;
    };
    let mut good = false;
    if candidate.active_path && ctx.same(&owner) {
        let current = candidate.eligible
            && candidate.qualified
            && ctx.qualified(&owner, true, Instant::now())
            && ctx.entries.current(entry);
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        record_first_resource_metadata(
            &mut s.facts.resource,
            entry.ordinal == 1,
            candidate.exact,
            candidate.observed.then_some(current),
        );
        if current && s.resource.commit(true) {
            s.facts.resource.passed += 1;
            s.facts.phase = "Passed";
            good = true
        } else {
            s.resource.revoke();
        }
    } else if let Some(proof) = candidate.proof.as_mut() {
        let row = {
            owner
                .borrow()
                .instance
                .as_ref()
                .and_then(|i| i.http401.as_ref())
                .map(|s| Rc::clone(&s.resource_binding))
        };
        if candidate.recorded
            && candidate.reserved
            && let Some(row) = row
        {
            good = settle_common(ctx, &owner, entry, &row, &mut proof.common) && proof.positive();
        }
        if candidate.recorded
            && let Some(s) = owner
                .borrow_mut()
                .instance
                .as_mut()
                .and_then(|i| i.http401.as_mut())
        {
            s.facts.post_target.resource.proof = Some(proof.clone());
        }
    }
    if candidate.reserved {
        good &= ctx.entries.finish_disposal(entry);
    }
    if good {
        good = ctx.entries.settle(
            entry,
            if candidate.active_path {
                EntryClass::Active
            } else {
                EntryClass::Disposal
            },
        );
    }
    if !good {
        ctx.entries.reject(entry);
        let reason = candidate.proof.as_ref().map_or("http401Resource", |proof| {
            disposal_failure_reason(&proof.common, ctx.entries.overflowed(), "http401Resource")
        });
        fail(&owner, reason, false);
        cleanup(&owner);
    }
    publish_close(&owner);
    emit(&owner);
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
pub(super) struct AuthCandidate {
    entry: EventEntry,
    origin: Option<Instant>,
    first: bool,
    eligible: bool,
    set: bool,
    read: Option<bool>,
    uri: Option<bool>,
    challenge: Option<bool>,
    sender: Option<bool>,
    before: bool,
    url: Option<String>,
}
pub(super) fn auth_callback(
    ctx: &Context,
    sender: Option<&ICoreWebView2>,
    args: Option<&ICoreWebView2BasicAuthenticationRequestedEventArgs>,
) -> AuthCandidate {
    auth_entered(ctx, ctx.entries.enter(EntryKind::Auth), sender, args)
}
fn auth_entered(
    ctx: &Context,
    entry: EventEntry,
    sender: Option<&ICoreWebView2>,
    args: Option<&ICoreWebView2BasicAuthenticationRequestedEventArgs>,
) -> AuthCandidate {
    capture_auth(ctx, prepare_auth(ctx, entry), sender, args)
}
fn prepare_auth(ctx: &Context, entry: EventEntry) -> AuthCandidate {
    let owner = ctx.owner.upgrade();
    let same = owner.as_ref().is_some_and(|o| ctx.same(o));
    let before = owner.as_ref().filter(|_| same).is_some_and(|o| {
        ctx.qualified(o, true, entry.at)
            && ctx.qualified(o, true, Instant::now())
            && ctx.entries.current(entry)
    });
    let mut url = None;
    let eligible = if let Some(owner) = owner.as_ref().filter(|_| same) {
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        s.facts.auth.callbacks = ctx.entries.auth_count.load(Ordering::SeqCst);
        let bounded = entry.ordinal == 1 && s.facts.auth.callbacks == 1;
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
    let first = entry.ordinal == 1;

    let candidate = AuthCandidate {
        entry,
        origin: None,
        first,
        eligible,
        set: false,
        read: None,
        uri: None,
        challenge: None,
        sender: None,
        before,
        url,
    };
    if !eligible {
        reject_entry(ctx, entry, "http401Auth", false);
    }
    candidate
}
fn capture_auth(
    ctx: &Context,
    mut candidate: AuthCandidate,
    sender: Option<&ICoreWebView2>,
    args: Option<&ICoreWebView2BasicAuthenticationRequestedEventArgs>,
) -> AuthCandidate {
    let entry = candidate.entry;
    let owner = ctx.owner.upgrade();
    let same = owner.as_ref().is_some_and(|o| ctx.same(o));
    let eligible = candidate.eligible;
    let url = candidate.url.take();
    let origin = Cell::new(None);
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
                origin.set(i.http401.as_ref().unwrap().timing.first_close);
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
    candidate.origin = origin.get();
    candidate.set = set.is_ok();
    candidate.read = get.is_ok().then_some(cancel.as_bool());
    drop(get);
    drop(set);
    if let Some(owner) = owner.as_ref().filter(|_| same)
        && eligible
        && candidate.set
        && candidate.read == Some(true)
        && ctx.qualified(owner, false, Instant::now())
        && ctx.entries.current(entry)
    {
        let uri = allocated_string(64, |raw| unsafe { args.Uri(raw) });
        candidate.uri = uri.as_ref().ok().map(|v| url.as_ref() == Some(v));
        drop(uri);
        if ctx.qualified(owner, false, Instant::now()) && ctx.entries.current(entry) {
            let challenge = allocated_string(64, |raw| unsafe { args.Challenge(raw) });
            candidate.challenge = challenge
                .as_ref()
                .ok()
                .map(|v| v == "Basic realm=\"code0-fixture\"");
            drop(challenge);
            if ctx.qualified(owner, false, Instant::now()) && ctx.entries.current(entry) {
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
    let current = ctx.qualified(&owner, false, now) && ctx.entries.current(candidate.entry);
    let mut good = candidate.eligible
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
        let close_matched = candidate
            .origin
            .map(|at| s.timing.first_close == Some(at) && at <= now);
        let history_ready = s.navigation_progress.sealable()
            && s.facts.navigation.attempts == 1
            && s.facts.navigation.grants == 1
            && s.facts.navigation.hresult_succeeded == Some(true)
            && s.facts.navigation.qualification_matched == Some(true)
            && s.facts.navigation.navigation_id_readable == Some(true)
            && s.facts.navigation.cancel_false == Some(true)
            && s.facts.resource.passed == 1
            && s.facts.resource.exact_document_get == Some(true)
            && s.facts.resource.qualification_matched == Some(true)
            && s.facts.server.request_valid == Some(true)
            && s.facts.server.response_complete == Some(true)
            && s.facts.server.sent_bytes == 139
            && s.facts.auth.callbacks == 1
            && s.facts.auth.cancel_attempts == 1
            && s.facts.auth.cancel_successes == 1
            && s.facts.auth.cancel_readbacks == 1
            && s.nav_id.is_some()
            && s.historical.is_none()
            && s.facts.receipts.target == 0
            && i.refusal_registry
                .http_rows()
                .iter()
                .all(|row| row.usable())
            && s.resource_binding.usable()
            && s.completed_binding.usable()
            && s.filter_binding.usable();
        good &=
            candidate.first && history_ready && close_matched == Some(true) && s.timing.before(now);
        let admission_once = s
            .facts
            .post_target
            .admission
            .record_attempt(Some(good), close_matched);
        if !admission_once {
            s.facts.overflow = true;
            good = false;
        }
        if good {
            // The seal and target are admitted together, here after all original
            // owned callback args/sender/native/error Drops, before publication.
            if let (Some(end), Some(first_close), Some(nav_id)) =
                (s.timing.end, s.timing.first_close, s.nav_id)
            {
                s.historical = Some(HistoricalAuthTarget {
                    ctx: ctx.clone(),
                    s2_rows: i.refusal_registry.http_rows(),
                    resource: Rc::clone(&s.resource_binding),
                    completed: Rc::clone(&s.completed_binding),
                    filter: Rc::clone(&s.filter_binding),
                    started: s.timing.started,
                    end,
                    first_close,
                    target: now,
                    guard: Arc::clone(&s.guard),
                    navigation: s.facts.navigation.clone(),
                    resource_history: s.facts.resource.clone(),
                    auth: s.facts.auth.clone(),
                    nav_id,
                    request_valid: s.facts.server.request_valid,
                    response_complete: s.facts.server.response_complete,
                    sent_bytes: s.facts.server.sent_bytes,
                });
                s.facts.receipts.target = 1;
                s.timing.target = Some(now);
            } else {
                good = false;
            }
        }
        if !good {
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
    diagnostic_web_error_status: Option<i32>,
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
            diagnostic_web_error_status: None,
            expected_id: None,
            first: false,
        };
    };
    if !ctx.same(&owner) {
        ctx.lost();
        return CompletedCandidate {
            profile: None,
            diagnostic_web_error_status: None,
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
            diagnostic_web_error_status: None,
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
    let mut diagnostic_web_error_status = None;
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
            if r.is_ok() {
                diagnostic_web_error_status = Some(status.0);
            }
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
        diagnostic_web_error_status,
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
    let now = Instant::now();
    profile.post_drop_current = Some(ctx.qualified(&owner, true, now));
    let good = profile.positive() && candidate.expected_id.is_some();
    let unique = {
        let mut o = owner.borrow_mut();
        let s = o.instance.as_mut().unwrap().http401.as_mut().unwrap();
        let unique = candidate.first && s.facts.early401.count == 1;
        if let Some(diagnostic) = &ctx.callbacks.diagnostics
            && diagnostic.http401_enabled()
        {
            // Observed here after native argument drops, never callback entry.
            // Reuse the existing post-drop clock read; diagnostic only.
            let at_ms = s.timing.ms(now);
            let flags = u64::from(candidate.diagnostic_web_error_status.unwrap_or(0) as u32)
                | (u64::from(candidate.diagnostic_web_error_status.is_some()) << 32)
                | (u64::from(at_ms.is_some()) << 33)
                | (u64::from(candidate.first) << 34)
                | (u64::from(profile.post_drop_current == Some(true)) << 35)
                | (u64::from(s.closing) << 36)
                | (u64::from(s.in_action) << 37);
            let counts = u64::from(s.setup)
                | (u64::from(s.accepted_setup) << 8)
                | (u64::from(s.facts.server.setup_calls) << 16)
                | (u64::from(s.facts.server.accepted_setup_calls) << 32)
                | (u64::from(s.facts.server.io_attempts) << 48);
            let wire = u64::from(s.facts.server.accepts)
                | (u64::from(s.facts.server.parsed_requests) << 16)
                | (u64::from(s.facts.server.request_bytes) << 32)
                | (u64::from(s.facts.server.sent_bytes) << 48);
            diagnostic.http401_completion_snapshot([flags, counts, wire, at_ms.unwrap_or(0)]);
        }
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
fn removal_current(ctx: &Context, owner: &Rc<RefCell<Owner>>) -> bool {
    ctx.same(owner) && {
        let o = owner.borrow();
        o.instance.as_ref().is_some_and(|i| {
            !o.unmatched
                && !i.released
                && !i.destroyed
                && !i.cleanup_blocked
                && !ctx.callbacks.affinity_failed.load(Ordering::SeqCst)
                && !ctx.installation.load(Ordering::SeqCst)
        })
    }
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
            let row = {
                let o = owner.borrow();
                let s = o.instance.as_ref().unwrap().http401.as_ref().unwrap();
                if resource {
                    Rc::clone(&s.resource_binding)
                } else {
                    Rc::clone(&s.completed_binding)
                }
            };
            let result = core.as_ref().map(|core| {
                row.remove_entered();
                if resource {
                    unsafe { core.remove_WebResourceRequested(token) }
                } else {
                    unsafe { core.remove_NavigationCompleted(token) }
                }
            });
            let good = result.as_ref().is_some_and(Result::is_ok);
            row.remove_returned(good);
            if !good {
                fail(owner, "http401Cleanup", true);
            }
            drop(result);
            row.remove_ack(good && removal_current(&context, owner));
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
        let row = {
            let o = owner.borrow();
            Rc::clone(
                &o.instance
                    .as_ref()
                    .unwrap()
                    .http401
                    .as_ref()
                    .unwrap()
                    .filter_binding,
            )
        };
        let result = core22.as_ref().map(|core| {
            row.remove_entered();
            unsafe {
                core.RemoveWebResourceRequestedFilterWithRequestSourceKinds(
                    windows::core::w!("*"),
                    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                    COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
                )
            }
        });
        let good = result.as_ref().is_some_and(Result::is_ok);
        row.remove_returned(good);
        if !good {
            fail(owner, "http401Cleanup", true);
        }
        drop(result);
        row.remove_ack(good && removal_current(&context, owner));
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
    (!i.http401_selected.load(Ordering::SeqCst) || !i.http401_entries.unsettled())
        && i.http401.as_ref().is_none_or(State::quiescent)
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

#[cfg(test)]
mod v2_tests {
    use super::*;

    // Mutation: a failed actual attempt is erased as no attempt, or a retry replaces it.
    #[test]
    fn v2_admission_retains_actual_failure_and_never_replaces_it() {
        let mut admission = Admission::default();
        assert_eq!(admission.count, 0);
        assert_eq!(admission.qualified, None);
        assert!(admission.record_attempt(Some(false), None));
        assert_eq!(admission.count, 1);
        assert_eq!(admission.qualified, Some(false));
        assert_eq!(admission.first_close_matched, None);
        assert!(!admission.record_attempt(Some(true), Some(true)));
        assert_eq!(admission.count, 2);
        assert_eq!(admission.qualified, Some(false));
        assert_eq!(admission.first_close_matched, None);
    }

    // Mutation: an actual entry is credited twice or disappears while its action is pending.
    #[test]
    fn v2_delivery_partition_counts_pending_active_disposal_and_invalid_exclusively() {
        let ledger = EntryLedger::default();
        let first = ledger.enter(EntryKind::Top);
        let row = ledger.partition(EntryKind::Top);
        assert_eq!(
            (row.deliveries, row.active, row.pending, row.invalid),
            (1, 0, 1, 0)
        );
        assert!(ledger.settle(first, EntryClass::Active));
        assert!(!ledger.settle(first, EntryClass::Active));
        let second = ledger.enter(EntryKind::Top);
        assert!(ledger.reserve_disposal(second));
        assert!(ledger.finish_disposal(second));
        assert!(ledger.settle(second, EntryClass::Disposal));
        let row = ledger.partition(EntryKind::Top);
        assert_eq!(
            (
                row.deliveries,
                row.active,
                row.qualified_disposal,
                row.invalid,
                row.pending
            ),
            (2, 1, 1, 0, 0)
        );
        let third = ledger.enter(EntryKind::Top);
        assert!(ledger.settle(third, EntryClass::Invalid));
        let row = ledger.partition(EntryKind::Top);
        assert_eq!((row.deliveries, row.invalid, row.pending), (3, 1, 0));
        assert!(ledger.overflowed());
    }

    // Mutation: the new positive cap truncates already observed resource3/4 failure facts.
    #[test]
    fn v2_resource_error_totals_remain_exact_three_four_and_lower_bound_five() {
        let ledger = EntryLedger::default();
        for count in 1..=4 {
            let entry = ledger.enter(EntryKind::Resource);
            assert!(ledger.settle(entry, EntryClass::Invalid));
            let row = ledger.partition(EntryKind::Resource);
            assert_eq!(row.deliveries, count);
            assert_eq!(row.invalid, count);
            assert_eq!(row.qualified_disposal, 0);
        }
        for _ in 0..3 {
            let entry = ledger.enter(EntryKind::Resource);
            ledger.settle(entry, EntryClass::Invalid);
        }
        assert_eq!(ledger.partition(EntryKind::Resource).deliveries, 5);
        assert!(ledger.overflowed());
    }

    // Mutation: a nested other-kind/guard callback can acquire a second candidate or heal serial.
    #[test]
    fn v2_one_global_pending_candidate_and_nested_entry_reject_the_outer_serial() {
        let ledger = EntryLedger::default();
        let active = ledger.enter(EntryKind::Top);
        ledger.settle(active, EntryClass::Active);
        let outer = ledger.enter(EntryKind::Top);
        assert!(ledger.reserve_disposal(outer));
        let nested = ledger.enter(EntryKind::Resource);
        assert!(!ledger.current(outer));
        assert!(!ledger.reserve_disposal(nested));
        assert!(ledger.finish_disposal(outer));
        assert!(!ledger.current(outer));
        assert!(ledger.interrupted());
    }

    // Mutation: off-UI/lost-owner return drops actual delivery accounting or creates proof.
    #[test]
    fn v2_rejected_entry_remains_an_actual_invalid_delivery_without_ui_access() {
        let ledger = EntryLedger::default();
        let entry = ledger.enter(EntryKind::Top);
        ledger.reject(entry);
        let row = ledger.partition(EntryKind::Top);
        assert_eq!((row.deliveries, row.invalid, row.pending), (1, 1, 0));
        assert!(ledger.uncertain());
        assert!(!ledger.current(entry));
        assert!(!ledger.reserve_disposal(entry));
    }

    // Mutation: checked serial exhaustion wraps and supplies a reusable current stamp.
    #[test]
    fn v2_serial_exhaustion_keeps_actual_entry_but_never_supplies_a_current_stamp() {
        let ledger = EntryLedger::default();
        ledger.serial.store(u64::MAX, Ordering::SeqCst);
        let entry = ledger.enter(EntryKind::Top);
        assert_eq!(entry.serial, None);
        ledger.reject(entry);
        assert_eq!(ledger.partition(EntryKind::Top).invalid, 1);
        assert!(!ledger.current(entry));
        assert!(ledger.overflowed());
    }

    // Mutation: a callback before the original Navigate return gets a seal later by replay.
    #[test]
    fn v2_history_requires_actual_navigation_return_and_post_drop_settlement() {
        let mut navigation = NavigationProgress::default();
        assert!(navigation.enter());
        assert!(!navigation.sealable());
        navigation.returned(true);
        assert!(!navigation.sealable());
        navigation.settle(true);
        assert!(navigation.sealable());
        assert!(!navigation.enter());
        assert!(!navigation.sealable());
        let mut failed = NavigationProgress::default();
        assert!(failed.enter());
        failed.returned(false);
        failed.settle(true);
        assert!(!failed.sealable());
    }

    // Mutation: close is rearmed, an equality/rollback/target-before-entry condition is ignored.
    #[test]
    fn v2_disposal_uses_both_original_precise_deadlines_and_target_order() {
        let start = Instant::now();
        let mut timing = Timing::new(start);
        timing.close(start + Duration::from_secs(40));
        timing.target = Some(start + Duration::from_secs(41));
        assert!(!timing.disposal_before(start + Duration::from_secs(40)));
        assert!(timing.disposal_before(start + Duration::from_secs(45) - Duration::from_nanos(1)));
        assert!(!timing.disposal_before(start + Duration::from_secs(45)));
        let mut early = Timing::new(start);
        early.close(start + Duration::from_secs(1));
        early.target = Some(start + Duration::from_secs(2));
        early.close(start + Duration::from_secs(8));
        assert!(early.disposal_before(start + Duration::from_secs(11) - Duration::from_nanos(1)));
        assert!(!early.disposal_before(start + Duration::from_secs(11)));
        assert!(!early.disposal_before(start - Duration::from_nanos(1)));
    }

    // Mutation: return HRESULT or token0 absence replaces genuine protected registration ACK.
    #[test]
    fn v2_registration_history_requires_actual_ack_and_preserves_zero_token() {
        let binding = RegistrationBinding::new(0);
        assert!(!binding.usable());
        binding.add_entered();
        binding.add_returned(true, Some(RegistrationToken::Event(0)));
        assert!(!binding.usable());
        binding.add_ack(true);
        assert!(binding.usable());
        binding.remove_entered();
        assert!(!binding.usable());
        binding.remove_returned(true);
        assert!(!binding.usable());
        binding.remove_ack(true);
        assert!(binding.usable());
        assert_eq!(binding.token.get(), Some(RegistrationToken::Event(0)));
        let failed = RegistrationBinding::new(1);
        failed.add_entered();
        failed.add_returned(false, None);
        failed.add_ack(true);
        assert!(!failed.usable());
    }

    // Mutation: the first failed candidate proof is overwritten with a later passing template.
    #[test]
    fn v2_candidate_partial_record_is_once_only_and_retains_actual_failure() {
        let mut row = DisposalRow::<TopDisposalProof>::default();
        let first = TopDisposalProof {
            common: DisposalCommon {
                ui_matched: Some(false),
                ..DisposalCommon::default()
            },
            ..TopDisposalProof::default()
        };
        assert!(row.record_candidate(first));
        let later = TopDisposalProof {
            common: DisposalCommon {
                ui_matched: Some(true),
                ..DisposalCommon::default()
            },
            ..TopDisposalProof::default()
        };
        assert!(!row.record_candidate(later));
        assert_eq!(row.candidates, 2);
        assert_eq!(row.proof.as_ref().unwrap().common.ui_matched, Some(false));
    }

    // Mutation: a disposal result overwrites the genuine active top's original metadata.
    #[test]
    fn v2_disposal_never_replaces_the_first_active_top_metadata() {
        let mut navigation = Navigation::default();
        record_first_top_metadata(&mut navigation, true, true, true, Some(true));
        record_first_top_metadata(&mut navigation, false, false, false, None);
        assert_eq!(navigation.qualification_matched, Some(true));
        assert_eq!(navigation.navigation_id_readable, Some(true));
        assert_eq!(navigation.cancel_false, Some(true));
    }

    // Mutation: zero-initialized STATSTG becomes a successful empty-content observation on error.
    #[test]
    fn v2_failed_stat_never_turns_default_zero_into_observed_content_bytes() {
        assert_eq!(observed_stat_bytes(false, 0), None);
        assert_eq!(observed_stat_bytes(true, 0), Some(0));
        assert_eq!(observed_stat_bytes(true, 7), Some(7));
    }

    // Mutation: a late actual invalid entry is hidden by an earlier copied native snapshot.
    #[test]
    fn v2_live_delivery_refresh_preserves_late_invalid_entry_and_failure() {
        let ledger = EntryLedger::default();
        let entry = ledger.enter(EntryKind::Top);
        ledger.settle(entry, EntryClass::Active);
        let mut facts = Facts::default();
        refresh_delivery_facts(&mut facts, &ledger);
        assert_eq!(facts.navigation.top_events, 1);
        let late = ledger.enter(EntryKind::Top);
        ledger.reject(late);
        refresh_delivery_facts(&mut facts, &ledger);
        assert_eq!(facts.navigation.top_events, 2);
        assert_eq!(facts.post_target.top.invalid, 1);
        assert!(facts.unknown);
        assert!(facts.failure.is_some());
    }
}

#[cfg(test)]
mod v2_proof_tests {
    use super::*;
    fn common() -> DisposalCommon {
        DisposalCommon {
            ui_matched: Some(true),
            target_established_at_entry: Some(true),
            closing_admitted_at_entry: Some(true),
            request_matched: Some(true),
            generation_matched: Some(true),
            core_matched: Some(true),
            s2_install_matched: Some(true),
            guard_install_matched: Some(true),
            registration_matched: Some(true),
            not_released: Some(true),
            not_destroyed: Some(true),
            no_prior_failure: Some(true),
            entry_order_current: Some(true),
            post_drop_current: Some(true),
            within_attempt_deadline: Some(true),
            within_close_deadline: Some(true),
            entry_at_ms: Some(100),
            settled_at_ms: Some(101),
        }
    }
    // Mutation: one available true/readback substitutes for the full current conjunction.
    #[test]
    fn v2_common_proof_rejects_each_missing_leaf_and_original_time_equality() {
        assert!(common().positive());
        for field in 0..16 {
            let mut proof = common();
            let value = match field {
                0 => &mut proof.ui_matched,
                1 => &mut proof.target_established_at_entry,
                2 => &mut proof.closing_admitted_at_entry,
                3 => &mut proof.request_matched,
                4 => &mut proof.generation_matched,
                5 => &mut proof.core_matched,
                6 => &mut proof.s2_install_matched,
                7 => &mut proof.guard_install_matched,
                8 => &mut proof.registration_matched,
                9 => &mut proof.not_released,
                10 => &mut proof.not_destroyed,
                11 => &mut proof.no_prior_failure,
                12 => &mut proof.entry_order_current,
                13 => &mut proof.post_drop_current,
                14 => &mut proof.within_attempt_deadline,
                _ => &mut proof.within_close_deadline,
            };
            *value = None;
            assert!(!proof.positive());
        }
        let mut proof = common();
        proof.settled_at_ms = Some(45_000);
        assert!(!proof.positive());
        proof.settled_at_ms = Some(99);
        assert!(!proof.positive());
    }
    // Mutation: successful SetResponse or zero-initialized bytes supplies missing Status/Stat.
    #[test]
    fn v2_empty_resource_requires_real_status_content_stat_and_actual_response_readback() {
        let proof = ResourceDisposalProof {
            common: common(),
            empty_stream_created: Some(true),
            seek_succeeded: Some(true),
            fixed_response_created: Some(true),
            response_set_attempts: 1,
            response_set_successes: 1,
            response_readbacks: 1,
            response_matched: Some(true),
            status_code: Some(403),
            content_query_succeeded: Some(true),
            stat_succeeded: Some(true),
            content_bytes: Some(0),
        };
        assert!(proof.positive());
        let mut missing = proof.clone();
        missing.content_bytes = None;
        assert!(!missing.positive());
        let mut nonempty = proof.clone();
        nonempty.content_bytes = Some(1);
        assert!(!nonempty.positive());
        let mut status = proof.clone();
        status.status_code = Some(200);
        assert!(!status.positive());
        let mut failed = proof.clone();
        failed.stat_succeeded = Some(false);
        assert!(!failed.positive());
        let mut wrong = proof;
        wrong.response_matched = Some(false);
        assert!(!wrong.positive());
    }
    // Mutation: proof-null/candidate pairing or active delivery is inferred from admission alone.
    #[test]
    fn v2_zero_disposal_branches_need_real_active_partitions_and_no_candidate_record() {
        let mut post = PostTarget::default();
        post.admission.record_attempt(Some(true), Some(true));
        post.top.deliveries = 1;
        post.top.active = 1;
        post.resource.deliveries = 1;
        post.resource.active = 1;
        assert!(post.positive());
        post.top.candidates = 1;
        assert!(!post.positive());
        post.top.candidates = 0;
        post.resource.invalid = 1;
        assert!(!post.positive());
    }
}

#[cfg(test)]
mod v2_continuation_tests {
    use super::*;

    // Regression: a failed setter short-circuits the required actual getter.
    #[test]
    fn v2_refusal_pair_always_reads_after_set_error_in_the_same_order() {
        let calls = RefCell::new(Vec::new());
        let (set, read) = restrictive_pair(
            || {
                calls.borrow_mut().push("set");
                Err::<(), ()>(())
            },
            || {
                calls.borrow_mut().push("read");
                false
            },
        );
        assert_eq!(set, Err(()));
        assert!(!read);
        assert_eq!(*calls.borrow(), ["set", "read"]);
    }

    // Regression: off-UI failure receives a later UI poll/receipt close origin.
    #[test]
    fn v2_atomic_failure_origin_preserves_first_actual_entry_time() {
        let start = Instant::now();
        let ledger = EntryLedger::with_start(start);
        ledger.note_failure(start + Duration::from_secs(2));
        ledger.note_failure(start + Duration::from_secs(8));
        assert_eq!(ledger.failure_at(), Some(start + Duration::from_secs(2)));
        let rollback = EntryLedger::with_start(start);
        rollback.note_failure(start - Duration::from_nanos(1));
        assert_eq!(rollback.failure_at(), None);
        assert!(rollback.uncertain());
    }

    // Regression: an actual off-UI/cold Basic callback disappears from callbacks.
    #[test]
    fn v2_atomic_auth_entries_are_counted_before_any_native_capture() {
        let ledger = EntryLedger::default();
        let first = ledger.enter(EntryKind::Auth);
        ledger.reject(first);
        let mut facts = Facts::default();
        refresh_delivery_facts(&mut facts, &ledger);
        assert_eq!(facts.auth.callbacks, 1);
        assert!(facts.unknown);
        let second = ledger.enter(EntryKind::Auth);
        ledger.reject(second);
        refresh_delivery_facts(&mut facts, &ledger);
        assert_eq!(facts.auth.callbacks, 2);
        assert!(facts.overflow);
    }

    // Regression: later matching snapshot can repair first observed false/readback.
    #[test]
    fn v2_snapshot_progress_never_repairs_false_or_changes_first_native_history() {
        let mut old = Facts {
            navigation: Navigation {
                qualification_matched: Some(false),
                ..Navigation::default()
            },
            ..Facts::default()
        };
        old.post_target.admission.record_attempt(Some(false), None);
        let mut next = old.clone();
        assert!(http401_snapshot_monotonic(&old, &next));
        next.navigation.qualification_matched = Some(true);
        assert!(!http401_snapshot_monotonic(&old, &next));
        next = old.clone();
        next.post_target.admission.qualified = Some(true);
        assert!(!http401_snapshot_monotonic(&old, &next));
        next = old.clone();
        next.navigation.top_events = 1;
        assert!(http401_snapshot_monotonic(&old, &next));
        assert!(!http401_snapshot_monotonic(&next, &old));
    }

    // Regression: receipt merge erases fixture-local uncertainty after native false.
    #[test]
    fn v2_merge_retains_local_unknown_and_first_failure() {
        let mut local = Facts {
            unknown: true,
            failure: Some("http401Session"),
            ..Facts::default()
        };
        let native = Facts::default();
        merge_http401_native_facts(&mut local, &native);
        assert!(local.unknown);
        assert_eq!(local.failure, Some("http401Session"));
    }

    // Regression: target count alone replaces the real immutable admission/action history.
    #[test]
    fn v2_target_history_requires_admission_and_survives_only_as_history_after_failure() {
        let mut facts = Facts {
            receipts: ReceiptCounts {
                target: 1,
                ..ReceiptCounts::default()
            },
            ..Facts::default()
        };
        assert!(!facts.target_history());
        facts
            .post_target
            .admission
            .record_attempt(Some(true), Some(true));
        assert!(!facts.target_history());
        facts.navigation = Navigation {
            attempts: 1,
            top_events: 1,
            grants: 1,
            hresult_succeeded: Some(true),
            qualification_matched: Some(true),
            navigation_id_readable: Some(true),
            cancel_false: Some(true),
        };
        facts.resource = Resource {
            total: 1,
            passed: 1,
            refused: 0,
            exact_document_get: Some(true),
            qualification_matched: Some(true),
        };
        facts.auth = Auth {
            callbacks: 1,
            cancel_attempts: 1,
            cancel_successes: 1,
            cancel_readbacks: 1,
            cancel_true: Some(true),
            uri_matched: Some(true),
            challenge_matched: Some(true),
            sender_matched: Some(true),
            post_drop_current: Some(true),
            within_attempt_deadline: Some(true),
        };
        facts.binding = Binding {
            request_matched: Some(true),
            generation_matched: Some(true),
            core_matched: Some(true),
            s2_install_matched: Some(true),
            guard_install_matched: Some(true),
            post_drop_current: Some(true),
        };
        facts.server.request_valid = Some(true);
        facts.server.response_complete = Some(true);
        facts.server.sent_bytes = 139;
        assert!(facts.target_history());
        facts.failure = Some("http401Navigation");
        facts.unknown = true;
        assert!(facts.target_history());
        assert!(!facts.native_positive());
        facts.post_target.admission.qualified = Some(false);
        assert!(!facts.target_history());
    }

    // Regression: refreshing actual delivery atomics erases independently observed final work.
    #[test]
    fn v2_merge_preserves_fixture_tempdir_and_final_clock_facts() {
        let mut local = Facts {
            cleanup: Cleanup {
                temp_dir_close_count: 1,
                first_close_adopted: Some(true),
                within_close_deadline: Some(true),
                target_at_ms: Some(100),
                first_close_at_ms: Some(99),
                final_at_ms: Some(105),
                cleanup_elapsed_ms: Some(6),
                ..Cleanup::default()
            },
            ..Facts::default()
        };
        let native = Facts::default();
        merge_http401_native_facts(&mut local, &native);
        assert_eq!(local.cleanup.temp_dir_close_count, 1);
        assert_eq!(local.cleanup.first_close_adopted, Some(true));
        assert_eq!(local.cleanup.target_at_ms, Some(100));
        assert_eq!(local.cleanup.first_close_at_ms, Some(99));
        assert_eq!(local.cleanup.final_at_ms, Some(105));
        assert_eq!(local.cleanup.cleanup_elapsed_ms, Some(6));
    }

    // Regression: a final clock below the actual target can still claim close timely.
    #[test]
    fn v2_final_clock_never_precedes_the_original_target_or_extends_close_equality() {
        let start = Instant::now();
        let mut clock = Timing::new(start);
        clock.close(start + Duration::from_secs(1));
        clock.target = Some(start + Duration::from_secs(2));
        assert!(
            !clock
                .final_clock(start + Duration::from_millis(1500))
                .within_close
        );
        assert!(
            clock
                .final_clock(start + Duration::from_secs(2))
                .within_close
        );
        assert!(
            clock
                .final_clock(start + Duration::from_secs(11) - Duration::from_nanos(1))
                .within_close
        );
        assert!(
            !clock
                .final_clock(start + Duration::from_secs(11))
                .within_close
        );
    }
    // Regression: a target arriving during AddRef is relabeled as present at entry.
    #[test]
    fn v2_target_establishment_is_checked_against_original_entry_instant() {
        let start = Instant::now();
        let mut clock = Timing::new(start);
        assert!(!clock.target_at_entry(start + Duration::from_secs(3)));
        clock.target = Some(start + Duration::from_secs(2));
        assert!(!clock.target_at_entry(start + Duration::from_secs(1)));
        assert!(clock.target_at_entry(start + Duration::from_secs(2)));
        assert!(clock.target_at_entry(start + Duration::from_secs(3)));
    }
}

// SOURCE definitions only. All six tests and native behavior UNRUN.
#[cfg(test)]
mod v2_native_fix1_tests {
    use super::*;

    fn clock_proof(timing: &Timing, entry: Instant, now: Instant) -> DisposalCommon {
        let (attempt, close) = timing.disposal_clock_facts(entry, now);
        DisposalCommon {
            target_established_at_entry: Some(true),
            closing_admitted_at_entry: Some(true),
            within_attempt_deadline: Some(attempt),
            within_close_deadline: Some(close),
            ..DisposalCommon::default()
        }
    }

    // Mutation: entry exactly at original end45 gets the generic kind reason.
    #[test]
    fn v2_fix1_attempt_equality_selects_deadline_without_extending_the_origin() {
        let start = Instant::now();
        let mut timing = Timing::new(start);
        timing.close(start + Duration::from_secs(40));
        timing.target = Some(start + Duration::from_secs(41));
        let timely = start + Duration::from_millis(44_999);
        let proof = clock_proof(&timing, timely, timely);
        assert_eq!(
            disposal_failure_reason(&proof, false, "http401Navigation"),
            "http401Navigation"
        );
        let expired = start + Duration::from_secs(45);
        let proof = clock_proof(&timing, expired, expired);
        assert_eq!(proof.within_attempt_deadline, Some(false));
        assert_eq!(
            disposal_failure_reason(&proof, false, "http401Navigation"),
            "http401Deadline"
        );
        assert_eq!(timing.first_close(), Some(start + Duration::from_secs(40)));
    }

    // Mutation: close equality or a clock before the original target is generic.
    #[test]
    fn v2_fix1_close_equality_and_backward_clock_select_deadline() {
        let start = Instant::now();
        let mut timing = Timing::new(start);
        timing.close(start + Duration::from_secs(1));
        timing.target = Some(start + Duration::from_secs(2));
        let entry = start + Duration::from_secs(3);
        let expired = start + Duration::from_secs(11);
        for (at, now) in [
            (entry, expired),
            (expired, expired),
            (entry, start + Duration::from_secs(1)),
        ] {
            let proof = clock_proof(&timing, at, now);
            assert_eq!(proof.within_attempt_deadline, Some(true));
            assert_eq!(proof.within_close_deadline, Some(false));
            assert_eq!(
                disposal_failure_reason(&proof, false, "http401Resource"),
                "http401Deadline"
            );
        }
        let backwards = start.checked_sub(Duration::from_nanos(1)).unwrap();
        let proof = clock_proof(&timing, entry, backwards);
        assert_eq!(proof.within_attempt_deadline, Some(false));
        assert_eq!(
            disposal_failure_reason(&proof, false, "http401Resource"),
            "http401Deadline"
        );
    }

    // Mutation: a timely entry is reused after args/sender Drop reaches close10.
    // This supplies controlled Instants to the production clock/reason helpers;
    // it does not execute COM Drop or attest native settlement.
    #[test]
    fn v2_fix1_timely_entry_then_post_drop_expiry_records_deadline() {
        let start = Instant::now();
        let mut timing = Timing::new(start);
        timing.close(start + Duration::from_secs(1));
        timing.target = Some(start + Duration::from_secs(2));
        let entry = start + Duration::from_secs(3);
        let at_entry = clock_proof(&timing, entry, entry);
        assert!(!disposal_clock_failed(&at_entry));
        let after_drop = clock_proof(&timing, entry, start + Duration::from_secs(11));
        assert!(disposal_clock_failed(&after_drop));
        let mut failure = None;
        assert!(retain_first_failure(
            &mut failure,
            disposal_failure_reason(&after_drop, false, "http401Navigation")
        ));
        assert!(!retain_first_failure(&mut failure, "http401Binding"));
        assert_eq!(failure, Some("http401Deadline"));
    }

    // Mutation: a late Deadline replaces an earlier actual non-deadline cause.
    #[test]
    fn v2_fix1_prior_non_deadline_failure_survives_clock_failure() {
        let proof = DisposalCommon {
            within_attempt_deadline: Some(false),
            ..DisposalCommon::default()
        };
        let mut failure = None;
        assert!(retain_first_failure(&mut failure, "http401Resource"));
        assert!(!retain_first_failure(
            &mut failure,
            disposal_failure_reason(&proof, false, "http401Navigation")
        ));
        assert_eq!(failure, Some("http401Resource"));
    }

    // Mutation: unavailable admission is mislabeled as an expired close clock,
    // or a cap failure already observed at entry is hidden by a later clock read.
    #[test]
    fn v2_fix1_unavailable_close_and_entry_overflow_keep_their_actual_category() {
        let proof = DisposalCommon {
            within_attempt_deadline: Some(true),
            within_close_deadline: Some(false),
            ..DisposalCommon::default()
        };
        assert!(!disposal_clock_failed(&proof));
        assert_eq!(
            disposal_failure_reason(&proof, false, "http401Navigation"),
            "http401Navigation"
        );
        let expired = DisposalCommon {
            within_attempt_deadline: Some(false),
            ..proof
        };
        assert_eq!(
            disposal_failure_reason(&expired, true, "http401Navigation"),
            "http401Overflow"
        );
    }

    // Mutation: first-extra diagnostics use stale receipt totals, include their
    // own already-counted entry, or relabel a third delivery as the first extra.
    #[test]
    fn v2_fix1_first_extra_diagnostics_use_actual_prior_entry_counts_once() {
        let ledger = EntryLedger::default();
        let navigation = Navigation {
            attempts: 1,
            grants: 1,
            ..Navigation::default()
        };
        let first_top = ledger.enter(EntryKind::Top);
        assert_eq!(
            diagnostic_extra_counts(&ledger, first_top, &navigation),
            None
        );
        ledger.enter(EntryKind::Resource);
        let extra_top = ledger.enter(EntryKind::Top);
        assert_eq!(
            diagnostic_extra_counts(&ledger, extra_top, &navigation),
            Some((0, 0x0001_0001_0001_0001))
        );
        let extra_resource = ledger.enter(EntryKind::Resource);
        assert_eq!(
            diagnostic_extra_counts(&ledger, extra_resource, &navigation),
            Some((1, 0x0001_0001_0001_0002))
        );
        let third = ledger.enter(EntryKind::Top);
        assert_eq!(diagnostic_extra_counts(&ledger, third, &navigation), None);
        let other = ledger.enter(EntryKind::Other);
        assert_eq!(diagnostic_extra_counts(&ledger, other, &navigation), None);
    }
}

// SOURCE definition only; actual test and native behavior UNRUN.
#[cfg(test)]
mod v2_native_fix2_tests {
    use super::*;

    // Mutation: checking only each absolute window and exported millisecond
    // floors admits a 1ns settlement rollback after the original target.
    #[test]
    fn v2_fix2_precise_entry_order_rejects_same_floor_rollback_and_keeps_first_cause() {
        let start = Instant::now();
        let mut timing = Timing::new(start);
        timing.close(start + Duration::from_secs(1));
        timing.target = Some(start + Duration::from_secs(2));
        let entry = start + Duration::from_secs(3) + Duration::from_nanos(500_000);
        let rollback = entry.checked_sub(Duration::from_nanos(1)).unwrap();
        for (now, expected) in [
            (rollback, false),
            (entry, true),
            (entry + Duration::from_nanos(1), true),
        ] {
            assert_eq!(timing.ms(entry), Some(3000));
            assert_eq!(timing.ms(now), Some(3000));
            let (attempt, close) = timing.disposal_clock_facts(entry, now);
            assert_eq!((attempt, close), (true, expected));
            let proof = DisposalCommon {
                target_established_at_entry: Some(true),
                closing_admitted_at_entry: Some(true),
                within_attempt_deadline: Some(attempt),
                within_close_deadline: Some(close),
                ..DisposalCommon::default()
            };
            assert_eq!(disposal_clock_failed(&proof), !expected);
            let reason = disposal_failure_reason(&proof, false, "http401Resource");
            assert_eq!(
                reason,
                if expected {
                    "http401Resource"
                } else {
                    "http401Deadline"
                }
            );
            if !expected {
                let mut first = None;
                assert!(retain_first_failure(&mut first, reason));
                assert!(!retain_first_failure(&mut first, "http401Binding"));
                assert_eq!(first, Some("http401Deadline"));
                let mut earlier = Some("http401Resource");
                assert!(!retain_first_failure(&mut earlier, reason));
                assert_eq!(earlier, Some("http401Resource"));
                assert_eq!(
                    disposal_failure_reason(&proof, true, "http401Resource"),
                    "http401Overflow"
                );
            }
        }
        assert_eq!(timing.first_close(), Some(start + Duration::from_secs(1)));
    }
}

#[cfg(test)]
mod bounded_transport_red_tests {
    use super::http401_transport::controlled::{Connection, exercise};

    // Reproduces the real old single-accept admission defect. The client does
    // not call a model, invent browser identity, send on idle A, or treat Pending
    // as refused. All real resource cleanup finishes before either assertion.
    #[test]
    fn http401_idle_first_request_second_uses_second_stream() {
        let run_id = std::env::var("GITHUB_RUN_ID").unwrap_or_else(|_| "local-unbound".to_owned());
        assert!(
            !run_id.is_empty()
                && run_id.len() <= 64
                && run_id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'),
            "HTTP401 transport run identity is unavailable or malformed"
        );
        let actual = exercise(run_id);
        let bytes = serde_json::to_vec(&actual.raw)
            .expect("HTTP401 controlled raw encoding failed after owned cleanup");
        assert!(
            bytes.len() <= 8192 && bytes.is_ascii(),
            "HTTP401 controlled raw record exceeds the frozen closed encoding envelope"
        );
        let raw = String::from_utf8(bytes)
            .expect("HTTP401 controlled raw encoding must be ASCII after owned cleanup");
        println!("Code0Http401Transport {raw}");
        assert!(
            actual.raw.complete,
            "HTTP401 transport setup, current completion, ordering or normal cleanup is unqualified: {}",
            actual.raw.failure
        );
        assert_eq!(
            actual.connection,
            Connection::Connected,
            "HTTP401-R idle first connection must not prevent the second owned connection from completing"
        );
    }
}
