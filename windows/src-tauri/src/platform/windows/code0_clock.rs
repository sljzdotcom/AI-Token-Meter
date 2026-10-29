//! Conservative TEST-FIRST clock seam, deliberately not the production clock.
//! No native source is wired: Root must first observe a meaningful isolated
//! behavioral RED before replacing fixed elapsed output with tick conversion.

use std::rc::Rc;
use std::time::Duration;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::code0_web_session::Code0SessionClock;

/// UI-local shared source pair. Clones preserve one elapsed origin/source and
/// one independent wall source; no worker, wake, timer or managed Send state.
#[derive(Clone)]
pub(crate) struct Code0WindowsClock {
    elapsed_milliseconds: Rc<dyn Fn() -> u64>,
    wall_utc: Rc<dyn Fn() -> OffsetDateTime>,
}
impl Code0WindowsClock {
    /// Source injection only. Future native factory is Root-owned and absent.
    /// Trusted native construction must use GetTickCount64 and now_utc; neither
    /// read is implemented or executed by this conservative baseline.
    pub(crate) fn from_sources(
        elapsed_milliseconds: Rc<dyn Fn() -> u64>,
        wall_utc: Rc<dyn Fn() -> OffsetDateTime>,
    ) -> Self {
        Self {
            elapsed_milliseconds,
            wall_utc,
        }
    }
}
impl Code0SessionClock for Code0WindowsClock {
    fn now(&self) -> Duration {
        // Intentional, buildable behavioral baseline. Sampling the injected
        // source keeps the real future call seam present, but no elapsed time
        // is admitted until the separately authorized post-RED implementation.
        let _observed_milliseconds = (self.elapsed_milliseconds)();
        Duration::ZERO
    }
    fn wall_now(&self) -> String {
        // Infallible existing trait: empty invalid time is rejected by the real
        // session before save/publication. Never fabricate epoch/page time.
        (self.wall_utc)().format(&Rfc3339).unwrap_or_default()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::collectors::code0_capture::Code0WebsiteCapture;
    use crate::domain::{UsageSnapshot, UsageStatus};
    use crate::platform::windows::code0_web_session::{
        Code0CleanupRequest, Code0SessionActionResult as Action, Code0SessionFailure as Failure,
        Code0SessionState as State, Code0SnapshotStoring, Code0StoreError, Code0WebSession,
        Code0WindowDriving, Code0WindowEvent as Event, Code0WindowRequest,
    };
    use std::cell::{Cell, RefCell};
    use time::UtcOffset;

    const OLD_TIME: &str = "2026-10-29T00:00:00Z";
    const NEW_TIME: &str = "2026-10-29T00:01:00Z";

    fn wall(value: &str) -> OffsetDateTime {
        OffsetDateTime::parse(value, &Rfc3339).unwrap()
    }
    fn sources() -> (Code0WindowsClock, Rc<Cell<u64>>, Rc<Cell<OffsetDateTime>>) {
        let ticks = Rc::new(Cell::new(0));
        let wall = Rc::new(Cell::new(wall(NEW_TIME)));
        let tick_reader = ticks.clone();
        let wall_reader = wall.clone();
        let clock = Code0WindowsClock::from_sources(
            Rc::new(move || tick_reader.get()),
            Rc::new(move || wall_reader.get()),
        );
        (clock, ticks, wall)
    }
    fn snapshot() -> UsageSnapshot {
        let mut value = Code0WebsiteCapture::decode_filtered_json(
            br#"{"quota":49400,"usedQuota":182700,"divisor":400}"#,
        )
        .unwrap()
        .make_snapshot(OLD_TIME)
        .unwrap();
        value.status = UsageStatus::Cached;
        value
    }
    struct StoreState {
        saved: UsageSnapshot,
        saves: usize,
    }
    struct Store(Rc<RefCell<StoreState>>);
    impl Code0SnapshotStoring for Store {
        fn load(&self) -> Result<Option<UsageSnapshot>, Code0StoreError> {
            Ok(Some(self.0.borrow().saved.clone()))
        }
        fn save_atomic(&mut self, value: &UsageSnapshot) -> Result<(), Code0StoreError> {
            let mut state = self.0.borrow_mut();
            state.saved = value.clone();
            state.saves += 1;
            Ok(())
        }
        fn remove(&mut self) -> Result<(), Code0StoreError> {
            Err(Code0StoreError::Unavailable)
        }
    }
    struct Driver(Rc<RefCell<Vec<Code0WindowRequest>>>);
    impl Code0WindowDriving for Driver {
        fn acquire_profile_lease(&mut self) -> Result<(), Failure> {
            Ok(())
        }
        fn release_profile_lease(&mut self) {}
        fn create_unloaded(&mut self, request: Code0WindowRequest) -> Result<(), Failure> {
            self.0.borrow_mut().push(request);
            Ok(())
        }
        fn navigate(&mut self, _: &str) -> Result<(), Failure> {
            Ok(())
        }
        fn close(&mut self, _: &str) {}
        fn clear_website_data(&mut self, _: Code0CleanupRequest) -> Result<(), Failure> {
            Err(Failure::CleanupFailed)
        }
    }
    struct Fixture {
        session: Code0WebSession<Code0WindowsClock, Driver, Store>,
        clock: Code0WindowsClock,
        ticks: Rc<Cell<u64>>,
        wall: Rc<Cell<OffsetDateTime>>,
        requests: Rc<RefCell<Vec<Code0WindowRequest>>>,
        store: Rc<RefCell<StoreState>>,
        publications: Rc<RefCell<Vec<Option<UsageSnapshot>>>>,
    }
    impl Fixture {
        fn new() -> Self {
            let (clock, ticks, wall) = sources();
            let requests = Rc::new(RefCell::new(Vec::new()));
            let store = Rc::new(RefCell::new(StoreState {
                saved: snapshot(),
                saves: 0,
            }));
            let publications = Rc::new(RefCell::new(Vec::new()));
            let publish = publications.clone();
            let session = Code0WebSession::new(
                clock.clone(),
                Driver(requests.clone()),
                Store(store.clone()),
                Box::new(move |value| publish.borrow_mut().push(value)),
            );
            assert_eq!(session.state(), State::Paused);
            Self {
                session,
                clock,
                ticks,
                wall,
                requests,
                store,
                publications,
            }
        }
        fn begin(&mut self) -> Code0WindowRequest {
            assert_eq!(self.session.sync_now(), Action::Started);
            let request = self.requests.borrow().last().unwrap().clone();
            self.session
                .receive_window_event(&request, Event::PoliciesInstalled);
            request
        }
        fn capture(&mut self, request: &Code0WindowRequest) {
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
                self.session.receive_window_event(
                    request,
                    Event::Message {
                        payload: serde_json::to_vec(&value).unwrap(),
                        source_url: "https://console.code0.ai/console".into(),
                        is_main_frame: true,
                    },
                );
            }
        }
        fn activate(&mut self) {
            let request = self.begin();
            self.capture(&request);
            assert_eq!(self.session.state(), State::Closing);
            self.session.receive_window_event(&request, Event::Released);
            assert_eq!(self.session.state(), State::Active);
            assert!(self.session.is_automatically_syncing());
            assert_eq!(self.requests.borrow().len(), 1);
        }
    }

    // Mutation: a constant, seconds conversion, truncation or overflow replaces elapsed milliseconds.
    #[test]
    fn elapsed_milliseconds_include_exact_299_300_and_large_u64_values() {
        let (clock, ticks, _) = sources();
        for (input, want) in [
            (0, Duration::ZERO),
            (299_000, Duration::from_secs(299)),
            (300_000, Duration::from_secs(300)),
            (u64::MAX, Duration::new(18_446_744_073_709_551, 615_000_000)),
        ] {
            ticks.set(input);
            assert_eq!(clock.now(), want);
        }
    }

    // Mutation: cloned clocks freeze a captured value or start a second elapsed origin.
    #[test]
    fn clones_observe_the_same_live_source_without_reset_or_snapshot() {
        let (clock, ticks, _) = sources();
        let clone = clock.clone();
        ticks.set(1_500);
        assert_eq!(clock.now(), Duration::from_millis(1_500));
        assert_eq!(clone.now(), Duration::from_millis(1_500));
        ticks.set(3_000);
        assert_eq!(clone.now(), Duration::from_secs(3));
        assert_eq!(clock.now(), Duration::from_secs(3));
    }

    // Mutation: wall time is derived from boot ticks or feeds the elapsed admission source.
    #[test]
    fn wall_jump_is_independent_and_has_native_rfc3339_values() {
        let (clock, ticks, wall_source) = sources();
        ticks.set(299_000);
        let before = clock.now();
        assert_eq!(clock.wall_now(), NEW_TIME);
        wall_source.set(wall("2027-10-29T03:04:05Z"));
        assert_eq!(clock.wall_now(), "2027-10-29T03:04:05Z");
        assert_eq!(clock.now(), before);
        wall_source.set(wall("2025-10-29T03:04:05Z"));
        assert_eq!(clock.wall_now(), "2025-10-29T03:04:05Z");
        assert_eq!(clock.now(), before);
    }

    // Mutation: failed wall formatting invents epoch time, saves or publishes an invalid capture.
    #[test]
    fn formatting_failure_yields_invalid_time_and_real_session_rejects_before_save() {
        let mut f = Fixture::new();
        let invalid = wall(NEW_TIME).to_offset(UtcOffset::from_hms(0, 0, 1).unwrap());
        f.wall.set(invalid);
        assert_eq!(f.clock.wall_now(), "");
        let request = f.begin();
        f.capture(&request);
        assert_eq!(f.session.failure(), Some(Failure::StorageFailed));
        assert_eq!(f.store.borrow().saves, 0);
        assert_eq!(f.store.borrow().saved.fetched_at, OLD_TIME);
        assert!(f.publications.borrow().is_empty());
        assert!(!f.session.is_automatically_syncing());
    }

    // Mutation: a wall jump advances admission or fixed elapsed cannot reach the real300 boundary.
    #[test]
    fn manual_sync_has_one_second_remaining_at_299_and_starts_at_300() {
        let mut f = Fixture::new();
        f.activate();
        f.wall.set(wall("2027-10-29T03:04:05Z"));
        f.ticks.set(299_000);
        assert_eq!(
            f.session.sync_now(),
            Action::Throttled {
                seconds_remaining: 1
            }
        );
        assert_eq!(f.requests.borrow().len(), 1);
        f.wall.set(wall("2025-10-29T03:04:05Z"));
        f.ticks.set(300_000);
        assert_eq!(f.session.sync_now(), Action::Started);
        assert_eq!(f.requests.borrow().len(), 2);
        assert_eq!(f.session.state(), State::Syncing);
    }

    // Mutation: constant elapsed prevents scheduled admission or boundary299 admits too soon.
    #[test]
    fn automatic_sync_starts_at_300_only_through_the_real_session() {
        let mut f = Fixture::new();
        f.activate();
        f.ticks.set(299_000);
        f.session.process_scheduled_work();
        assert_eq!(f.requests.borrow().len(), 1);
        assert_eq!(f.session.state(), State::Active);
        f.ticks.set(300_000);
        f.session.process_scheduled_work();
        assert_eq!(f.requests.borrow().len(), 2);
        assert_eq!(f.session.state(), State::Syncing);
    }

    // Mutation: an elapsed suspend jump is discarded or repeated settled turns create repeated work.
    #[test]
    fn injected_suspend_jump_coalesces_one_due_attempt_without_a_real_sleep() {
        let mut f = Fixture::new();
        f.activate();
        f.ticks.set(3_600_000);
        for _ in 0..10 {
            f.session.process_scheduled_work();
        }
        assert_eq!(f.requests.borrow().len(), 2);
        assert_eq!(f.session.state(), State::Syncing);
        assert!(!f.session.is_automatically_syncing());
    }
}
