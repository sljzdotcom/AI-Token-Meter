use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use ai_token_meter_windows::collectors::code0_capture::Code0WebsiteCapture;
use ai_token_meter_windows::domain::{ProviderId, UsageSnapshot, UsageStatus};
use ai_token_meter_windows::platform::windows::code0_web_session::{
    Code0CleanupRequest, Code0SessionActionResult as Action, Code0SessionClock,
    Code0SessionFailure as Failure, Code0SessionState as State, Code0SnapshotStoring,
    Code0StoreError, Code0WebSession, Code0WindowDriving, Code0WindowEvent as Event,
    Code0WindowMode as Mode, Code0WindowRequest,
};
use serde_json::json;

const OLD_TIME: &str = "2026-10-27T00:00:00Z";
const NEW_TIME: &str = "2026-10-27T00:01:00Z";
const OTHER_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";

#[derive(Clone, Debug, PartialEq)]
enum Trace {
    Load,
    Acquire,
    Create(Mode),
    Navigate,
    Close,
    Remove,
    SaveAttempt,
    SaveSucceeded,
    Publish(Option<(f64, String)>),
    Clear,
    Release,
}
type Log = Rc<RefCell<Vec<Trace>>>;

#[derive(Clone)]
struct Clock(Rc<RefCell<(Duration, String)>>);
impl Code0SessionClock for Clock {
    fn now(&self) -> Duration {
        self.0.borrow().0
    }
    fn wall_now(&self) -> String {
        self.0.borrow().1.clone()
    }
}

struct Disk {
    load_failed: bool,
    save_failed: bool,
    remove_failed: bool,
    snapshot: Option<UsageSnapshot>,
}
struct Store {
    disk: Rc<RefCell<Disk>>,
    log: Log,
}
impl Code0SnapshotStoring for Store {
    fn load(&self) -> Result<Option<UsageSnapshot>, Code0StoreError> {
        self.log.borrow_mut().push(Trace::Load);
        let disk = self.disk.borrow();
        if disk.load_failed {
            Err(Code0StoreError::Unavailable)
        } else {
            Ok(disk.snapshot.clone())
        }
    }
    fn save_atomic(&mut self, snapshot: &UsageSnapshot) -> Result<(), Code0StoreError> {
        self.log.borrow_mut().push(Trace::SaveAttempt);
        let mut disk = self.disk.borrow_mut();
        if disk.save_failed {
            return Err(Code0StoreError::Unavailable);
        }
        disk.snapshot = Some(snapshot.clone());
        self.log.borrow_mut().push(Trace::SaveSucceeded);
        Ok(())
    }
    fn remove(&mut self) -> Result<(), Code0StoreError> {
        self.log.borrow_mut().push(Trace::Remove);
        let mut disk = self.disk.borrow_mut();
        if disk.remove_failed {
            return Err(Code0StoreError::Unavailable);
        }
        disk.snapshot = None;
        Ok(())
    }
}

#[derive(Default)]
struct Native {
    held: bool,
    acquire_failed: bool,
    create_failed: bool,
    requests: Vec<Code0WindowRequest>,
    cleanup: Vec<Code0CleanupRequest>,
}
struct Driver {
    native: Rc<RefCell<Native>>,
    log: Log,
}
impl Code0WindowDriving for Driver {
    fn acquire_profile_lease(&mut self) -> Result<(), Failure> {
        self.log.borrow_mut().push(Trace::Acquire);
        let mut native = self.native.borrow_mut();
        if native.acquire_failed {
            return Err(Failure::ProfileBusy);
        }
        native.held = true;
        Ok(())
    }
    fn release_profile_lease(&mut self) {
        self.native.borrow_mut().held = false;
        self.log.borrow_mut().push(Trace::Release);
    }
    fn create_unloaded(&mut self, request: Code0WindowRequest) -> Result<(), Failure> {
        self.log.borrow_mut().push(Trace::Create(request.mode));
        let mut native = self.native.borrow_mut();
        native.requests.push(request);
        if native.create_failed {
            Err(Failure::NavigationFailed)
        } else {
            Ok(())
        }
    }
    fn navigate(&mut self, _instance_id: &str) -> Result<(), Failure> {
        self.log.borrow_mut().push(Trace::Navigate);
        Ok(())
    }
    fn close(&mut self, _instance_id: &str) {
        self.log.borrow_mut().push(Trace::Close);
    }
    fn clear_website_data(&mut self, request: Code0CleanupRequest) -> Result<(), Failure> {
        self.log.borrow_mut().push(Trace::Clear);
        self.native.borrow_mut().cleanup.push(request);
        Ok(())
    }
}

type Subject = Code0WebSession<Clock, Driver, Store>;
struct Fixture {
    session: Subject,
    clock: Clock,
    native: Rc<RefCell<Native>>,
    disk: Rc<RefCell<Disk>>,
    log: Log,
}
impl Fixture {
    fn new(prior: Option<UsageSnapshot>, load_failed: bool) -> Self {
        let clock = Clock(Rc::new(RefCell::new((Duration::ZERO, NEW_TIME.into()))));
        let native = Rc::new(RefCell::new(Native::default()));
        let disk = Rc::new(RefCell::new(Disk {
            load_failed,
            save_failed: false,
            remove_failed: false,
            snapshot: prior,
        }));
        let log = Rc::new(RefCell::new(Vec::new()));
        let publication = log.clone();
        let session = Code0WebSession::new(
            clock.clone(),
            Driver {
                native: native.clone(),
                log: log.clone(),
            },
            Store {
                disk: disk.clone(),
                log: log.clone(),
            },
            Box::new(move |value| {
                publication.borrow_mut().push(Trace::Publish(
                    value.map(|s| (s.primary_metric.unwrap().current, s.fetched_at)),
                ));
            }),
        );
        Self {
            session,
            clock,
            native,
            disk,
            log,
        }
    }
    fn cached() -> Self {
        Self::new(Some(prior()), false)
    }
    fn advance_to(&self, seconds: u64) {
        self.clock.0.borrow_mut().0 = Duration::from_secs(seconds);
    }
    fn request(&self) -> Code0WindowRequest {
        self.native.borrow().requests.last().unwrap().clone()
    }
    fn installed(&mut self, request: &Code0WindowRequest) {
        self.session
            .receive_window_event(request, Event::PoliciesInstalled);
    }
    fn released(&mut self, request: &Code0WindowRequest) {
        self.session.receive_window_event(request, Event::Released);
    }
    fn begin_cached_sync(&mut self) -> Code0WindowRequest {
        assert_eq!(self.session.sync_now(), Action::Started);
        let request = self.request();
        assert_eq!(request.mode, Mode::Sync);
        self.installed(&request);
        request
    }
    fn message(&mut self, request: &Code0WindowRequest, kind: &str, sequence: u64) {
        let mut value = json!({
            "version": 1, "nonce": request.nonce, "document": request.document_id,
            "kind": kind, "sequence": sequence,
        });
        if kind == "selfStart" || kind == "capture" {
            value["request"] = json!(1);
            value["revision"] = json!(0);
        }
        if kind == "capture" {
            value["quota"] = json!(2250000);
            value["usedQuota"] = json!(750000);
            value["divisor"] = json!(500000);
        }
        if kind == "invalidate" {
            value["reason"] = json!("authenticationChanged");
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
    fn capture(&mut self, request: &Code0WindowRequest) {
        self.message(request, "ready", 1);
        self.message(request, "selfStart", 2);
        self.message(request, "capture", 3);
    }
    fn activate(&mut self) -> Code0WindowRequest {
        let request = self.begin_cached_sync();
        self.capture(&request);
        self.released(&request);
        assert_eq!(self.session.state(), State::Active);
        assert!(self.session.is_automatically_syncing());
        request
    }
    fn count(&self, event: &Trace) -> usize {
        self.log.borrow().iter().filter(|v| *v == event).count()
    }
}

fn prior() -> UsageSnapshot {
    let mut value = Code0WebsiteCapture::decode_filtered_json(
        br#"{"quota":1000000,"usedQuota":500000,"divisor":500000}"#,
    )
    .unwrap()
    .make_snapshot(OLD_TIME)
    .unwrap();
    value.status = UsageStatus::Cached;
    value
}
fn balance(session: &Subject) -> Option<f64> {
    session
        .snapshot()
        .and_then(|s| s.primary_metric.as_ref())
        .map(|m| m.current)
}

// Mutation target: cache presence must not authorize a lease/window/automatic run.
#[test]
fn cold_cache_is_paused_without_network_lease_or_new_success_time() {
    for prior_status in [UsageStatus::Fresh, UsageStatus::Cached] {
        let mut cached = prior();
        cached.status = prior_status;
        let mut f = Fixture::new(Some(cached), false);
        assert_eq!(f.session.state(), State::Paused);
        assert_eq!(balance(&f.session), Some(2.0));
        assert_eq!(f.session.snapshot().unwrap().fetched_at, OLD_TIME);
        assert_eq!(f.session.snapshot().unwrap().status, UsageStatus::Cached);
        f.advance_to(3600);
        f.session.process_scheduled_work();
        assert!(!f.session.is_automatically_syncing());
        assert!(!f.native.borrow().held);
        assert!(f.native.borrow().requests.is_empty());
        assert_eq!(f.count(&Trace::SaveAttempt), 0);
    }
}

#[test]
fn explicit_connect_waits_for_policies_and_actual_login_release_before_hidden_sync() {
    let mut f = Fixture::new(None, false);
    assert_eq!(f.session.connect(), Action::Started);
    let login = f.request();
    assert_eq!(login.mode, Mode::Login);
    assert!(f.native.borrow().held);
    assert_eq!(f.count(&Trace::Navigate), 0);
    f.installed(&login);
    assert_eq!(f.count(&Trace::Navigate), 1);
    f.capture(&login); // Login documents never publish numeric values.
    assert_eq!(f.count(&Trace::SaveAttempt), 0);
    assert_eq!(f.session.login_completed(), Action::Started);
    assert_eq!(f.session.state(), State::Closing);
    assert_eq!(f.session.login_completed(), Action::Busy);
    assert_eq!(f.native.borrow().requests.len(), 1);
    assert_eq!(f.count(&Trace::Close), 1);
    f.released(&login);
    let sync = f.request();
    assert_eq!(sync.mode, Mode::Sync);
    assert_ne!(sync.instance_id, login.instance_id);
    assert_ne!(sync.nonce, login.nonce);
    assert_ne!(sync.document_id, login.document_id);
    assert_eq!(f.count(&Trace::Navigate), 1);
    f.installed(&sync);
    assert_eq!(f.count(&Trace::Navigate), 2);
}

#[test]
fn successful_capture_saves_before_publish_and_authorizes_only_after_release() {
    let mut f = Fixture::cached();
    let request = f.begin_cached_sync();
    f.capture(&request);
    assert_eq!(balance(&f.session), Some(4.5));
    assert_eq!(f.session.snapshot().unwrap().fetched_at, NEW_TIME);
    assert_eq!(
        f.session
            .snapshot()
            .unwrap()
            .secondary_metric
            .as_ref()
            .unwrap()
            .current,
        1.5
    );
    assert_eq!(f.session.state(), State::Closing);
    assert!(!f.session.is_automatically_syncing());
    let log = f.log.borrow();
    let saved = log.iter().position(|e| *e == Trace::SaveSucceeded).unwrap();
    let published = log
        .iter()
        .position(|e| *e == Trace::Publish(Some((4.5, NEW_TIME.into()))))
        .unwrap();
    let closed = log.iter().position(|e| *e == Trace::Close).unwrap();
    assert!(saved < published && published < closed);
    drop(log);
    f.released(&request);
    assert_eq!(f.session.state(), State::Active);
    assert!(f.session.is_automatically_syncing());
    f.capture(&request);
    assert_eq!(f.count(&Trace::SaveAttempt), 1);
}

#[test]
fn failed_save_preserves_prior_value_time_and_never_publishes_candidate() {
    let mut f = Fixture::cached();
    f.disk.borrow_mut().save_failed = true;
    let request = f.begin_cached_sync();
    f.capture(&request);
    assert_eq!(balance(&f.session), Some(2.0));
    assert_eq!(f.session.snapshot().unwrap().fetched_at, OLD_TIME);
    assert_eq!(
        f.disk.borrow().snapshot.as_ref().unwrap().fetched_at,
        OLD_TIME
    );
    assert_eq!(f.session.failure(), Some(Failure::StorageFailed));
    assert!(
        !f.log
            .borrow()
            .iter()
            .any(|e| matches!(e, Trace::Publish(Some((v, _))) if *v == 4.5))
    );
    f.released(&request);
    f.advance_to(900);
    f.session.process_scheduled_work();
    assert_eq!(f.session.state(), State::Paused);
    assert!(!f.session.is_automatically_syncing());
    assert_eq!(f.native.borrow().requests.len(), 1);
}

#[test]
fn manual_and_automatic_sync_share_299_and_300_second_start_boundary() {
    for manual in [false, true] {
        let mut f = Fixture::cached();
        f.activate();
        f.advance_to(299);
        if manual {
            assert_eq!(
                f.session.sync_now(),
                Action::Throttled {
                    seconds_remaining: 1
                }
            );
        } else {
            f.session.process_scheduled_work();
        }
        assert_eq!(f.native.borrow().requests.len(), 1);
        f.advance_to(300);
        if manual {
            assert_eq!(f.session.sync_now(), Action::Started);
        } else {
            f.session.process_scheduled_work();
        }
        assert_eq!(f.native.borrow().requests.len(), 2);
        assert_eq!(f.session.state(), State::Syncing);
        assert!(!f.session.is_automatically_syncing());
    }
}

#[test]
fn sleep_coalesces_one_due_attempt_and_busy_does_not_create_another() {
    let mut f = Fixture::cached();
    f.activate();
    f.advance_to(3600);
    for _ in 0..10 {
        f.session.process_scheduled_work();
    }
    assert_eq!(f.native.borrow().requests.len(), 2);
    assert_eq!(f.session.sync_now(), Action::Busy);
    assert_eq!(f.session.connect(), Action::Busy);
    assert_eq!(f.native.borrow().requests.len(), 2);
}

#[test]
fn login_600_and_sync_45_deadlines_pause_without_retry_or_new_time() {
    for login in [false, true] {
        let mut f = if login {
            Fixture::new(None, false)
        } else {
            Fixture::cached()
        };
        let request = if login {
            assert_eq!(f.session.connect(), Action::Started);
            let request = f.request();
            f.installed(&request);
            request
        } else {
            f.begin_cached_sync()
        };
        let limit = if login { 600 } else { 45 };
        f.advance_to(limit - 1);
        f.session.process_scheduled_work();
        assert_eq!(f.count(&Trace::Close), 0);
        f.advance_to(limit);
        f.session.process_scheduled_work();
        assert_eq!(f.session.state(), State::Closing);
        assert_eq!(
            f.session.failure(),
            Some(if login {
                Failure::LoginTimedOut
            } else {
                Failure::SyncTimedOut
            })
        );
        assert_eq!(f.count(&Trace::Close), 1);
        f.released(&request);
        f.advance_to(3600);
        f.session.process_scheduled_work();
        assert!(!f.session.is_automatically_syncing());
        assert_eq!(f.native.borrow().requests.len(), 1);
        assert_eq!(f.count(&Trace::SaveAttempt), 0);
    }
}

#[test]
fn delayed_capture_cannot_bypass_sync_deadline_without_a_timer_tick() {
    let mut f = Fixture::cached();
    let request = f.begin_cached_sync();
    f.advance_to(45);
    f.capture(&request);
    assert_eq!(f.count(&Trace::SaveAttempt), 0);
    assert_eq!(balance(&f.session), Some(2.0));
    assert_eq!(f.session.failure(), Some(Failure::SyncTimedOut));
    assert!(!f.session.is_automatically_syncing());
}

#[test]
fn unconfirmed_close_at_ten_seconds_holds_lease_and_blocks_replacement() {
    let mut f = Fixture::cached();
    let request = f.begin_cached_sync();
    f.capture(&request);
    f.advance_to(9);
    f.session.process_scheduled_work();
    assert_eq!(f.session.state(), State::Closing);
    f.advance_to(10);
    f.session.process_scheduled_work();
    assert_eq!(f.session.state(), State::CleanupPending);
    assert!(f.native.borrow().held);
    assert_ne!(f.session.connect(), Action::Started);
    assert_ne!(f.session.sync_now(), Action::Started);
    assert_eq!(f.native.borrow().requests.len(), 1);
    f.released(&request);
    assert!(!f.session.is_automatically_syncing());
    assert_ne!(f.session.state(), State::Active);
}

#[test]
fn stale_epoch_attempt_or_document_callback_cannot_pause_or_publish_current_attempt() {
    for field in ["epoch", "attempt", "document"] {
        let mut f = Fixture::cached();
        let current = f.begin_cached_sync();
        let mut stale = current.clone();
        match field {
            "epoch" => stale.connection_epoch = OTHER_ID.into(),
            "attempt" => stale.instance_id = OTHER_ID.into(),
            _ => stale.document_id = OTHER_ID.into(),
        }
        f.session
            .receive_window_event(&stale, Event::Failed(Failure::PolicyRejected));
        f.capture(&stale);
        assert_eq!(f.session.state(), State::Syncing);
        assert_eq!(f.count(&Trace::SaveAttempt), 0);
        f.capture(&current);
        assert_eq!(balance(&f.session), Some(4.5));
    }
}

#[test]
fn nonofficial_source_or_subframe_message_cannot_publish_a_candidate() {
    for (source, main_frame) in [
        ("https://console.code0.ai.evil.invalid/", true),
        ("https://console.code0.ai/console", false),
    ] {
        let mut f = Fixture::cached();
        let request = f.begin_cached_sync();
        let ready = json!({"version":1,"nonce":request.nonce,
            "document":request.document_id,"sequence":1,"kind":"ready"});
        f.session.receive_window_event(
            &request,
            Event::Message {
                payload: serde_json::to_vec(&ready).unwrap(),
                source_url: source.into(),
                is_main_frame: main_frame,
            },
        );
        assert_eq!(f.session.state(), State::Closing);
        assert_eq!(f.count(&Trace::SaveAttempt), 0);
        assert_eq!(balance(&f.session), Some(2.0));
        f.released(&request);
        assert_eq!(f.session.state(), State::Paused);
        assert!(!f.session.is_automatically_syncing());
    }
}

#[test]
fn stop_revokes_capture_but_waits_for_owned_release_and_never_reauthorizes() {
    let mut f = Fixture::cached();
    let request = f.begin_cached_sync();
    f.session.stop();
    assert_eq!(f.session.state(), State::Closing);
    assert!(f.native.borrow().held);
    f.capture(&request);
    assert_eq!(f.count(&Trace::SaveAttempt), 0);
    assert_eq!(balance(&f.session), Some(2.0));
    f.released(&request);
    assert!(!f.native.borrow().held);
    assert_eq!(f.session.sync_now(), Action::Denied);
    assert_eq!(f.session.connect(), Action::Denied);
    f.advance_to(3600);
    f.session.process_scheduled_work();
    assert_eq!(f.native.borrow().requests.len(), 1);
}

#[test]
fn authentication_change_clears_cache_while_ordinary_failure_preserves_it() {
    for authentication_changed in [false, true] {
        let mut f = Fixture::cached();
        let request = f.begin_cached_sync();
        if authentication_changed {
            f.message(&request, "ready", 1);
            f.message(&request, "invalidate", 2);
            assert_eq!(balance(&f.session), None);
            assert!(f.disk.borrow().snapshot.is_none());
            assert_eq!(f.session.failure(), Some(Failure::AuthenticationChanged));
        } else {
            f.session
                .receive_window_event(&request, Event::Failed(Failure::NavigationFailed));
            assert_eq!(balance(&f.session), Some(2.0));
            assert_eq!(f.session.snapshot().unwrap().fetched_at, OLD_TIME);
        }
        f.released(&request);
        f.advance_to(900);
        f.session.process_scheduled_work();
        assert!(!f.session.is_automatically_syncing());
        assert_eq!(f.native.borrow().requests.len(), 1);
    }
}

#[test]
fn failed_or_invalid_cache_is_not_proof_that_prior_account_data_is_absent() {
    for failed_load in [false, true] {
        let mut invalid = prior();
        invalid.provider_id = ProviderId::Claude;
        let mut f = Fixture::new(Some(invalid), failed_load);
        assert_eq!(f.session.state(), State::Paused);
        assert_eq!(balance(&f.session), None);
        assert_eq!(f.session.failure(), Some(Failure::StorageFailed));
        assert_eq!(f.session.connect(), Action::Started);
        let log = f.log.borrow();
        let acquired = log.iter().position(|e| *e == Trace::Acquire).unwrap();
        let removed = log.iter().position(|e| *e == Trace::Remove).unwrap();
        let created = log
            .iter()
            .position(|e| *e == Trace::Create(Mode::Login))
            .unwrap();
        assert!(acquired < removed && removed < created);
    }
}

#[test]
fn account_boundary_remove_failure_blocks_login_and_retains_cleanup_authority() {
    let mut f = Fixture::cached();
    f.disk.borrow_mut().remove_failed = true;
    assert_eq!(f.session.connect(), Action::Denied);
    assert_eq!(f.session.state(), State::CleanupPending);
    assert_eq!(balance(&f.session), None);
    assert!(f.native.borrow().held);
    assert!(f.native.borrow().requests.is_empty());
    assert!(!f.session.is_automatically_syncing());
}

#[test]
fn disconnect_invalidates_numbers_and_waits_for_cleanup_ack_before_release() {
    let mut f = Fixture::cached();
    let request = f.activate();
    assert_eq!(f.session.disconnect(), Action::Started);
    assert_eq!(balance(&f.session), None);
    assert_eq!(f.session.state(), State::Closing);
    assert!(f.native.borrow().held);
    assert_eq!(f.native.borrow().cleanup.len(), 1);
    let cleanup = f.native.borrow().cleanup[0].clone();
    f.capture(&request);
    assert_eq!(f.count(&Trace::SaveAttempt), 1);
    f.session.receive_cleanup_result(&cleanup, Ok(()));
    assert_eq!(f.session.state(), State::Disconnected);
    assert!(!f.native.borrow().held);
    assert!(f.disk.borrow().snapshot.is_none());
    assert!(!f.session.is_automatically_syncing());
    let log = f.log.borrow();
    let clear = log.iter().position(|e| *e == Trace::Clear).unwrap();
    let release = log.iter().position(|e| *e == Trace::Release).unwrap();
    assert!(clear < release);
}

#[test]
fn disconnect_waits_for_owned_release_before_website_cleanup() {
    let mut f = Fixture::cached();
    let request = f.begin_cached_sync();
    assert_eq!(f.session.disconnect(), Action::Started);
    assert_eq!(balance(&f.session), None);
    assert_eq!(f.session.state(), State::Closing);
    assert!(f.native.borrow().held);
    assert!(f.native.borrow().cleanup.is_empty());
    assert_eq!(f.count(&Trace::Remove), 0);
    f.capture(&request);
    assert_eq!(f.count(&Trace::SaveAttempt), 0);
    f.released(&request);
    assert_eq!(f.native.borrow().cleanup.len(), 1);
    let cleanup = f.native.borrow().cleanup[0].clone();
    f.session.receive_cleanup_result(&cleanup, Ok(()));
    assert_eq!(f.session.state(), State::Disconnected);
    assert!(!f.native.borrow().held);
}

#[test]
fn disconnect_cleanup_failure_or_wrong_ack_never_grants_a_new_connection() {
    for field in ["epoch", "cleanup"] {
        let mut f = Fixture::cached();
        f.activate();
        assert_eq!(f.session.disconnect(), Action::Started);
        let cleanup = f.native.borrow().cleanup[0].clone();
        let mut stale = cleanup.clone();
        if field == "epoch" {
            stale.connection_epoch = OTHER_ID.into();
        } else {
            stale.cleanup_id = OTHER_ID.into();
        }
        f.session.receive_cleanup_result(&stale, Ok(()));
        assert_eq!(f.session.state(), State::Closing);
        assert!(f.native.borrow().held);
        f.session
            .receive_cleanup_result(&cleanup, Err(Failure::CleanupFailed));
        assert_eq!(f.session.state(), State::CleanupPending);
        assert!(f.native.borrow().held);
        assert_ne!(f.session.connect(), Action::Started);
        assert_eq!(f.native.borrow().requests.len(), 1);
    }
}

#[test]
fn partially_created_window_error_still_requires_actual_release() {
    let mut f = Fixture::new(None, false);
    f.native.borrow_mut().create_failed = true;
    let _ = f.session.connect();
    assert_eq!(f.native.borrow().requests.len(), 1);
    assert_eq!(f.session.state(), State::Closing);
    assert!(f.native.borrow().held);
    assert_ne!(f.session.connect(), Action::Started);
    assert_eq!(f.native.borrow().requests.len(), 1);
    let request = f.request();
    f.released(&request);
    assert!(!f.session.is_automatically_syncing());
    assert_eq!(f.session.state(), State::Paused);
}

#[test]
fn busy_profile_refuses_all_window_creation_and_preserves_prior_data() {
    let mut f = Fixture::cached();
    f.native.borrow_mut().acquire_failed = true;
    assert_eq!(f.session.sync_now(), Action::Denied);
    assert_eq!(f.session.failure(), Some(Failure::ProfileBusy));
    assert_eq!(balance(&f.session), Some(2.0));
    assert!(f.native.borrow().requests.is_empty());
    assert!(!f.native.borrow().held);
    assert_eq!(f.count(&Trace::SaveAttempt), 0);
}
