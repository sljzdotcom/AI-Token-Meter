//! Finite cold binding only. No native clear, Profile2, physical completion or terminal permission.
use std::cell::Cell;
use std::time::Duration;

use serde::Serialize;

use super::super::code0_clock::Code0WindowsClock;
use super::super::code0_web_session::{Code0CleanupBudgetHandle, Code0SessionClock};

#[derive(Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ColdEntries {
    pub(crate) lease_acquire_entries: u32,
    pub(crate) create_entries: u32,
    pub(crate) native_create_entries: u32,
    pub(crate) clear_entries: u32,
}
impl ColdEntries {
    pub(crate) fn untouched(self) -> bool {
        self.lease_acquire_entries == 0
            && self.create_entries == 0
            && self.native_create_entries == 0
            && self.clear_entries == 0
    }
}
#[derive(Clone, Copy)]
pub(crate) enum ColdEntry {
    Acquire,
    Create,
    NativeCreate,
    Clear,
}
#[derive(Default)]
pub(crate) struct ColdHistory {
    entries: Cell<ColdEntries>,
}
impl ColdHistory {
    pub(crate) fn enter(&self, entry: ColdEntry) {
        // Entry precedes acquisition, validation, native construction or clear.
        // Failure/release has no operation that can undo this monotonic history.
        let mut entries = self.entries.get();
        let counter = match entry {
            ColdEntry::Acquire => &mut entries.lease_acquire_entries,
            ColdEntry::Create => &mut entries.create_entries,
            ColdEntry::NativeCreate => &mut entries.native_create_entries,
            ColdEntry::Clear => &mut entries.clear_entries,
        };
        *counter = counter.saturating_add(1);
        self.entries.set(entries);
    }
    pub(crate) fn eligible(&self) -> bool {
        self.entries.get().untouched()
    }
    pub(crate) fn facts(&self) -> ColdEntries {
        self.entries.get()
    }
}

pub(crate) struct CleanupContext {
    clock: Code0WindowsClock,
    // This is the actual new Session's read-only Weak budget. The cold route
    // has no admitted request; it must neither invent one nor interpret None.
    _budget: Code0CleanupBudgetHandle,
}
impl CleanupContext {
    pub(crate) fn new(clock: Code0WindowsClock, budget: Code0CleanupBudgetHandle) -> Self {
        Self {
            clock,
            _budget: budget,
        }
    }
    pub(crate) fn sample(&self) -> NativeColdClockSample {
        NativeColdClockSample {
            elapsed_milliseconds: self.clock.now().as_millis().try_into().unwrap_or(u64::MAX),
            wall_utc: self.clock.wall_now(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeColdClockSample {
    elapsed_milliseconds: u64,
    wall_utc: String,
}
impl NativeColdClockSample {
    pub fn elapsed(&self) -> Duration {
        Duration::from_millis(self.elapsed_milliseconds)
    }
    pub fn wall_utc(&self) -> &str {
        &self.wall_utc
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeColdInstallReceipt {
    version: u8,
    binding: u64,
    context_bound: bool,
    registry_published: bool,
    #[serde(flatten)]
    entries: ColdEntries,
}
impl NativeColdInstallReceipt {
    pub(crate) fn new(binding: u64, context_bound: bool, entries: ColdEntries) -> Self {
        Self {
            version: 1,
            binding,
            context_bound,
            registry_published: true,
            entries,
        }
    }
    pub fn binding(&self) -> u64 {
        self.binding
    }
    pub fn positive(&self) -> bool {
        self.version == 1
            && self.binding != 0
            && self.context_bound
            && self.registry_published
            && self.entries.untouched()
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeColdRetirementReceipt {
    version: u8,
    binding: u64,
    stopped_reply_consumed: bool,
    stopped_idle: bool,
    matching_record_taken: bool,
    owned_handles_dropped: bool,
    current_record_absent: bool,
    ui_sta: bool,
    #[serde(flatten)]
    entries: ColdEntries,
}
impl NativeColdRetirementReceipt {
    pub(crate) fn after_owned_drops(binding: u64, entries: ColdEntries, absent: bool) -> Self {
        // Only the consuming runtime route calls this after every owned Rc and
        // proxy has been explicitly dropped outside registry/backend borrows.
        Self {
            version: 1,
            binding,
            stopped_reply_consumed: true,
            stopped_idle: true,
            matching_record_taken: true,
            owned_handles_dropped: true,
            current_record_absent: absent,
            ui_sta: true,
            entries,
        }
    }
    pub fn binding(&self) -> u64 {
        self.binding
    }
    pub fn positive(&self) -> bool {
        self.version == 1
            && self.binding != 0
            && self.stopped_reply_consumed
            && self.stopped_idle
            && self.matching_record_taken
            && self.owned_handles_dropped
            && self.current_record_absent
            && self.ui_sta
            && self.entries.untouched()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Mutation: considering an empty slot after a failed/released acquisition cold again.
    #[test]
    fn cold_history_never_recovers_after_any_native_entry() {
        for entry in [
            ColdEntry::Acquire,
            ColdEntry::Create,
            ColdEntry::NativeCreate,
            ColdEntry::Clear,
        ] {
            let history = ColdHistory::default();
            assert!(history.eligible());
            history.enter(entry);
            assert!(!history.eligible());
            // A later observation cannot reset any entry, even after ordinary release.
            let _ = history.facts();
            assert!(!history.eligible());
        }
    }
    // Mutation: counter wraparound makes previously active native state look untouched.
    #[test]
    fn cold_history_saturation_cannot_restore_eligibility() {
        let history = ColdHistory::default();
        history.entries.set(ColdEntries {
            lease_acquire_entries: u32::MAX,
            ..ColdEntries::default()
        });
        history.enter(ColdEntry::Acquire);
        assert!(!history.eligible());
        assert_eq!(history.facts().lease_acquire_entries, u32::MAX);
    }
}
