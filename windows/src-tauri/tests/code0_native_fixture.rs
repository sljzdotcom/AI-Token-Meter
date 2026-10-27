//! One process-main native smoke case. A skipped/unsupported target is never PASS.
//! Catches a driver that accepts create without a real hidden bare engine, renderer
//! message, actual destruction and owned reference release. No mock engine is used.
#[cfg(all(windows, debug_assertions, feature = "code0-native-fixture"))]
mod smoke {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::fs::{File, OpenOptions};
    use std::io::{Seek, SeekFrom, Write};
    use std::path::PathBuf;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex, mpsc};
    use std::thread::{self, ThreadId};
    use std::time::{Duration, Instant};

    use ai_token_meter_windows::platform::windows::code0_web_session::{
        Code0WindowDriving, Code0WindowEvent, Code0WindowMode, Code0WindowRequest,
    };
    use ai_token_meter_windows::platform::windows::code0_webview::{
        Code0NativeDriver,
        fixture::{
            DiagnosticMilestone as DM, DiagnosticPhase as DP, Diagnostics, NativeSmokeObservation,
        },
    };
    use serde::Serialize;
    use tauri::{Manager, RunEvent, WindowEvent};
    use windows::Win32::System::Com::{
        APTTYPE, APTTYPE_MAINSTA, APTTYPE_STA, APTTYPEQUALIFIER, COINIT_APARTMENTTHREADED,
        CoGetApartmentType, CoInitializeEx, CoUninitialize,
    };

    const CASE: &str = "hidden_bare_inline_engine_release";
    const INSTANCE: &str = "code0-native-smoke";

    #[derive(Default, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Summary {
        schema: u8,
        run_id: String,
        required_case: &'static str,
        selected_cases: u8,
        completed_cases: u8,
        pass: bool,
        failure: &'static str,
        sta_main: bool,
        callback_affinity: bool,
        no_inherited_override: bool,
        engine_ready: bool,
        renderer_message: bool,
        bare_hidden_unfocused: bool,
        construction_observed: bool,
        bare_window_violation: bool,
        profile_isolated: bool,
        destroyed: bool,
        owned_references_released: bool,
        run_returned: bool,
        cleanup_succeeded: bool,
        runtime_version: String,
    }

    impl Summary {
        fn positive(&self) -> bool {
            self.sta_main
                && self.callback_affinity
                && self.no_inherited_override
                && self.engine_ready
                && self.renderer_message
                && self.bare_hidden_unfocused
                && self.construction_observed
                && !self.bare_window_violation
                && self.profile_isolated
                && self.destroyed
                && self.owned_references_released
        }
    }

    struct ConstructionEvidence {
        receipts: u8,
        label: Option<String>,
        identity_matches: bool,
        violation: bool,
        callback_affinity: bool,
    }

    impl ConstructionEvidence {
        fn apply(&self, summary: &mut Summary) {
            summary.construction_observed = self.receipts == 1 && self.identity_matches;
            summary.bare_window_violation |= self.violation;
            summary.callback_affinity &= self.callback_affinity;
            summary.bare_hidden_unfocused =
                summary.construction_observed && !summary.bare_window_violation;
        }
    }

    fn hidden_bare_window(handle: &tauri::AppHandle, window: &tauri::Window) -> bool {
        handle.windows().len() == 1
            && window.webviews().is_empty()
            && window.is_visible().ok() == Some(false)
            && window.is_focused().ok() == Some(false)
    }

    fn write_summary(file: &mut File, summary: &Summary) -> std::io::Result<()> {
        let bytes = serde_json::to_vec(summary)?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(&bytes)?;
        file.set_len(bytes.len() as u64)?;
        file.sync_all()
    }

    struct Sta;
    impl Sta {
        fn initialize() -> Result<Self, &'static str> {
            // S_FALSE also requires a matching CoUninitialize on this same thread.
            unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
                .ok()
                .map_err(|_| "staInitializeFailed")?;
            let guard = Self;
            if !is_sta() {
                return Err("staQueryFailed");
            }
            Ok(guard)
        }
    }
    impl Drop for Sta {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }
    fn is_sta() -> bool {
        let mut apartment = APTTYPE::default();
        let mut qualifier = APTTYPEQUALIFIER::default();
        unsafe { CoGetApartmentType(&mut apartment, &mut qualifier) }.is_ok()
            && (apartment == APTTYPE_STA || apartment == APTTYPE_MAINSTA)
    }

    enum Observation {
        Native(NativeSmokeObservation, ThreadId, bool),
        Session(Code0WindowRequest, Code0WindowEvent, ThreadId, bool),
    }

    fn next_observation(queue: &RefCell<VecDeque<Observation>>) -> Option<Observation> {
        queue.borrow_mut().pop_front()
    }

    pub fn run() -> Result<bool, Box<dyn std::error::Error>> {
        let run_id = std::env::var("AI_METER_CODE0_NATIVE_RUN_ID")?;
        if run_id.is_empty()
            || run_id.len() > 64
            || !run_id
                .bytes()
                .all(|v| v.is_ascii_alphanumeric() || v == b'-' || v == b'_')
        {
            return Err("invalid fixture run ID".into());
        }
        let output = PathBuf::from(
            std::env::var_os("AI_METER_CODE0_NATIVE_SUMMARY")
                .ok_or("missing fresh summary path")?,
        );
        let diagnostics = Arc::new(Diagnostics::from_env());
        // Never accept an old report as evidence of this run.
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)?;
        let summary = Arc::new(Mutex::new(Summary {
            schema: 2,
            run_id,
            required_case: CASE,
            selected_cases: 1,
            failure: "notCompleted",
            ..Summary::default()
        }));
        diagnostics.phase(DP::FreshSummary, false);
        write_summary(&mut file, &summary.lock().unwrap())?;
        diagnostics.phase(DP::FreshSummary, true);
        let evidence = Arc::new(Mutex::new(file));
        let (watchdog_done, watchdog_rx) = mpsc::channel();
        let hard_run_id = summary.lock().unwrap().run_id.clone();
        let hard_evidence = Arc::clone(&evidence);
        let watchdog_diagnostics = Arc::clone(&diagnostics);
        let watchdog = thread::spawn(move || {
            if watchdog_rx.recv_timeout(Duration::from_secs(75)).is_err() {
                watchdog_diagnostics.emit_once(true);
                let failed = Summary {
                    schema: 2,
                    run_id: hard_run_id,
                    required_case: CASE,
                    selected_cases: 1,
                    failure: "watchdogTimeout",
                    ..Summary::default()
                };
                if let Ok(mut file) = hard_evidence.try_lock() {
                    let _ = write_summary(&mut file, &failed);
                }
                // Only this fixture process. Never cleanup proof or browser-process control.
                std::process::exit(124);
            }
        });
        let outcome = run_case(Arc::clone(&summary), Arc::clone(&diagnostics));
        if outcome.is_err() {
            diagnostics.phase(DP::SummaryLock, false);
            summary.lock().unwrap().failure = "fixtureSetupFailed";
            diagnostics.phase(DP::SummaryLock, true);
        }
        diagnostics.phase(DP::SummaryLock, false);
        let passed = summary.lock().unwrap().pass;
        diagnostics.phase(DP::SummaryLock, true);
        diagnostics.phase(DP::FinalSummary, false);
        let write_result = write_summary(&mut evidence.lock().unwrap(), &summary.lock().unwrap());
        diagnostics.phase(DP::FinalSummary, true);
        let _ = watchdog_done.send(());
        diagnostics.phase(DP::WatchdogJoin, false);
        let _ = watchdog.join();
        diagnostics.phase(DP::WatchdogJoin, true);
        diagnostics.emit_once(false);
        write_result?;
        outcome?;
        Ok(passed)
    }

    fn run_case(
        summary: Arc<Mutex<Summary>>,
        diagnostics: Arc<Diagnostics>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        diagnostics.phase(DP::Sta, false);
        let _sta = Sta::initialize()?;
        diagnostics.phase(DP::Sta, true);
        diagnostics.observed(DM::Sta);
        let main_thread = thread::current().id();
        diagnostics.phase(DP::SummaryLock, false);
        summary.lock().unwrap().sta_main = true;
        diagnostics.phase(DP::SummaryLock, true);
        diagnostics.phase(DP::SummaryLock, false);
        summary.lock().unwrap().callback_affinity = true;
        diagnostics.phase(DP::SummaryLock, true);
        // Any inherited engine override fails closed; never log values.
        if std::env::vars_os().any(|(key, _)| {
            key.to_string_lossy()
                .to_ascii_uppercase()
                .starts_with("WEBVIEW2_")
        }) {
            diagnostics.phase(DP::SummaryLock, false);
            summary.lock().unwrap().failure = "inheritedWebview2Override";
            diagnostics.phase(DP::SummaryLock, true);
            return Ok(());
        }
        diagnostics.phase(DP::SummaryLock, false);
        summary.lock().unwrap().no_inherited_override = true;
        diagnostics.phase(DP::SummaryLock, true);
        diagnostics.observed(DM::NoOverrides);
        let temp = tempfile::tempdir()?;
        let temp_root = temp.path().canonicalize()?;
        let profile = temp.path().join("code0");
        let mut context = tauri::generate_context!();
        context.config_mut().app.windows.clear();
        // No product run/setup/RuntimeState, plugins, commands or managed windows.
        diagnostics.phase(DP::AppBuild, false);
        let app = {
            let _boundary = diagnostics.boundary(DP::AppBuild);
            tauri::Builder::default().build(context)?
        };
        diagnostics.phase(DP::AppBuild, true);
        diagnostics.observed(DM::AppBuilt);
        let queue = Rc::new(RefCell::new(VecDeque::new()));
        let session_queue = Rc::clone(&queue);
        let mut driver = Code0NativeDriver::new(
            app.handle().clone(),
            profile.clone(),
            Rc::new(move |request, event| {
                session_queue.borrow_mut().push_back(Observation::Session(
                    request,
                    event,
                    thread::current().id(),
                    is_sta(),
                ));
            }),
        );
        driver.observe_fixture_diagnostics(Arc::clone(&diagnostics));
        let construction = Arc::new(Mutex::new(ConstructionEvidence {
            receipts: 0,
            label: None,
            identity_matches: false,
            violation: false,
            callback_affinity: true,
        }));
        let construction_sink = Arc::clone(&construction);
        let owner_handle = app.handle().clone();
        let native_queue = Rc::clone(&queue);
        let observer_diagnostics = Arc::clone(&diagnostics);
        driver.observe_fixture(Rc::new(move |event| {
            if let NativeSmokeObservation::WindowConstructed { window } = event {
                // This receipt is synchronous at actual bare construction, before
                // any correction/hiding or asynchronous environment/controller work.
                // Query construction state now. Focus-event latching is already
                // active in the main RunEvent callback; it needs no queued listener.
                observer_diagnostics.phase(DP::ConstructionQueries, false);
                let boundary = observer_diagnostics.boundary(DP::ConstructionQueries);
                let actual_hwnd = window.hwnd().ok();
                let identity_matches = actual_hwnd.is_some()
                    && owner_handle
                        .get_window(window.label())
                        .is_some_and(|owned| owned.hwnd().ok() == actual_hwnd);
                let safe = identity_matches && hidden_bare_window(&owner_handle, &window);
                drop(boundary);
                observer_diagnostics.phase(DP::ConstructionQueries, true);
                if safe {
                    observer_diagnostics.observed(DM::ConstructionSafe);
                }
                observer_diagnostics.phase(DP::ConstructionLock, false);
                let mut evidence = construction_sink.lock().unwrap();
                observer_diagnostics.phase(DP::ConstructionLock, true);
                evidence.receipts = evidence.receipts.saturating_add(1);
                evidence.violation |= evidence.receipts != 1 || !safe;
                evidence.callback_affinity &= thread::current().id() == main_thread && is_sta();
                evidence.identity_matches = identity_matches;
                if evidence.label.is_none() {
                    evidence.label = Some(window.label().to_owned());
                }
                return;
            }
            if let NativeSmokeObservation::EngineReady { window_label, .. } = &event {
                observer_diagnostics.phase(DP::ConstructionLock, false);
                let mut evidence = construction_sink.lock().unwrap();
                observer_diagnostics.phase(DP::ConstructionLock, true);
                // A late construction receipt cannot retroactively validate engine readiness.
                evidence.violation |= evidence.receipts != 1
                    || !evidence.identity_matches
                    || evidence.label.as_ref() != Some(window_label);
            }
            native_queue.borrow_mut().push_back(Observation::Native(
                event,
                thread::current().id(),
                is_sta(),
            ));
        }));
        let request = Code0WindowRequest {
            instance_id: INSTANCE.into(),
            connection_epoch: "fixture-epoch".into(),
            nonce: "fixture-nonce".into(),
            document_id: "fixture-document".into(),
            mode: Code0WindowMode::Sync,
        };
        let wake_handle = app.handle().clone();
        let (wake_done, wake_rx) = mpsc::channel();
        let wake = thread::spawn(move || {
            while wake_rx.recv_timeout(Duration::from_millis(25)).is_err() {
                if wake_handle.run_on_main_thread(|| {}).is_err() {
                    break;
                }
            }
        });
        let observed = Arc::clone(&summary);
        let started = Instant::now();
        let mut closing_since = None;
        let mut started_create = false;
        let mut object_free_failure = false;
        let mut terminal_exit_requested = false;
        let mut ready_count = 0;
        let mut message_count = 0;
        let mut browser_exit_count = 0;
        let live_construction = Arc::clone(&construction);
        let ui_diagnostics = Arc::clone(&diagnostics);
        diagnostics.phase(DP::RunReturn, false);
        let exit_code = app.run_return(move |handle, event| {
            // These are actual events from this isolated app (initially zero windows,
            // only one hidden/unfocused window is permitted). Latch even before a
            // verified construction label exists, or for an unexpected second window.
            // This runs before any focus query and before queued per-window listeners.
            ui_diagnostics.phase(DP::EventDispatch, false);
            if matches!(
                &event,
                RunEvent::WindowEvent {
                    event: WindowEvent::Focused(true),
                    ..
                }
            ) {
                ui_diagnostics.phase(DP::ConstructionLock, false);
                live_construction.lock().unwrap().violation = true;
                ui_diagnostics.phase(DP::ConstructionLock, true);
            }
            ui_diagnostics.phase(DP::UiAffinity, false);
            if thread::current().id() != main_thread || !is_sta() {
                ui_diagnostics.failure(DP::UiAffinity, 0);
                ui_diagnostics.phase(DP::SummaryLock, false);
                observed.lock().unwrap().failure = "uiAffinityFailed";
                ui_diagnostics.phase(DP::SummaryLock, true);
                // With partial native ownership, losing the cleanup apartment
                // must fail via the watchdog rather than drop the owner early.
                if !terminal_exit_requested
                    && let RunEvent::ExitRequested {
                        code: None, api, ..
                    } = &event
                {
                    api.prevent_exit();
                }
                return;
            }
            ui_diagnostics.phase(DP::UiAffinity, true);
            ui_diagnostics.ui_heartbeat();
            // An authorized own exit is terminal. Keep the focus/affinity
            // observations above, then let ExitRequested/Exit finish without
            // re-posting or re-evaluating native work/deadlines/verdicts.
            if terminal_exit_requested {
                return;
            }
            let mut create_failure = None;
            if matches!(event, RunEvent::Ready) && !started_create {
                started_create = true;
                ui_diagnostics.observed(DM::Ready);
                ui_diagnostics.phase(DP::Lease, false);
                let leased = driver.acquire_profile_lease();
                ui_diagnostics.phase(DP::Lease, true);
                if leased.is_err() {
                    ui_diagnostics.failure(DP::Lease, 0);
                    create_failure = Some("profileLeaseDenied");
                    object_free_failure = true;
                } else {
                    ui_diagnostics.observed(DM::Lease);
                    ui_diagnostics.phase(DP::Create, false);
                    let created = driver.create_unloaded(request.clone());
                    ui_diagnostics.phase(DP::Create, true);
                    if created.is_err() {
                        create_failure = Some("nativeCreateDenied");
                        // A failed create can already own a bare Window, COM objects
                        // or an in-flight completion. Retain the owner and lease and
                        // keep pumping until actual close evidence arrives.
                        closing_since = Some(Instant::now());
                        ui_diagnostics.phase(DP::CloseRequest, false);
                        driver.close(INSTANCE);
                        ui_diagnostics.phase(DP::CloseRequest, true);
                        ui_diagnostics.observed(DM::CloseRequested);
                        object_free_failure = driver.fixture_creation_is_object_free(INSTANCE);
                    }
                }
                if object_free_failure {
                    // This separate inventory fact never invents Destroyed.
                    ui_diagnostics.phase(DP::SummaryLock, false);
                    observed.lock().unwrap().failure = create_failure.unwrap();
                    ui_diagnostics.phase(DP::SummaryLock, true);
                    terminal_exit_requested = true;
                    driver.release_profile_lease();
                    ui_diagnostics.phase(DP::ExitRequest, false);
                    handle.exit(1);
                    ui_diagnostics.phase(DP::ExitRequest, true);
                    return;
                }
            }
            if let RunEvent::WindowEvent { label, event, .. } = &event {
                ui_diagnostics.phase(DP::WindowEventDispatch, false);
                driver.on_window_event(label, event);
                ui_diagnostics.phase(DP::WindowEventDispatch, true);
            }
            // Previous callbacks have returned before this poll. No Send-bound
            // Tauri listener captures the UI-local Rc/COM owner.
            ui_diagnostics.phase(DP::Poll, false);
            driver.poll_cleanup();
            ui_diagnostics.phase(DP::Poll, true);
            ui_diagnostics.phase(DP::SummaryLock, false);
            let mut state = observed.lock().unwrap();
            ui_diagnostics.phase(DP::SummaryLock, true);
            if let Some(failure) = create_failure {
                state.failure = failure;
            }
            ui_diagnostics.phase(DP::ConstructionLock, false);
            let owned_label = live_construction.lock().unwrap().label.clone();
            ui_diagnostics.phase(DP::ConstructionLock, true);
            if let RunEvent::WindowEvent {
                label,
                event: WindowEvent::Destroyed,
                ..
            } = &event
                && owned_label.as_ref() == Some(label)
            {
                state.destroyed = true;
            }
            // Pop in a helper so RefMut is dropped before an observer can enqueue.
            while let Some(observation) = next_observation(&queue) {
                match observation {
                    Observation::Native(
                        NativeSmokeObservation::EngineReady {
                            window_label: _,
                            user_data_folder,
                            runtime_version,
                        },
                        callback_thread,
                        callback_sta,
                    ) => {
                        ready_count += 1;
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        state.engine_ready = ready_count == 1
                            && !runtime_version.is_empty()
                            && runtime_version.len() <= 128
                            && runtime_version
                                .bytes()
                                .all(|v| v.is_ascii_digit() || v == b'.');
                        if state.engine_ready {
                            state.runtime_version = runtime_version;
                        }
                        state.profile_isolated = user_data_folder
                            .canonicalize()
                            .ok()
                            .zip(profile.canonicalize().ok())
                            .is_some_and(|(actual, expected)| {
                                actual == expected
                                    && actual.starts_with(&temp_root)
                                    && actual != temp_root
                            });
                        if state.profile_isolated {
                            ui_diagnostics.observed(DM::ProfileMatched);
                        }
                        // No fixture state lock crosses a COM call, which may
                        // deliver native callbacks before returning.
                        drop(state);
                        ui_diagnostics.phase(DP::Inline, false);
                        let loaded = driver.load_fixture_inline_document(INSTANCE);
                        ui_diagnostics.phase(DP::Inline, true);
                        ui_diagnostics.phase(DP::SummaryLock, false);
                        state = observed.lock().unwrap();
                        ui_diagnostics.phase(DP::SummaryLock, true);
                        if loaded.is_err() {
                            state.failure = "inlineLoadDenied";
                        }
                    }
                    Observation::Native(NativeSmokeObservation::WindowConstructed { .. }, _, _) => {
                        // Construction receipts are handled synchronously at the sink, never queued.
                        ui_diagnostics.phase(DP::ConstructionLock, false);
                        live_construction.lock().unwrap().violation = true;
                        ui_diagnostics.phase(DP::ConstructionLock, true);
                    }
                    Observation::Native(
                        NativeSmokeObservation::RendererMessage { payload },
                        callback_thread,
                        callback_sta,
                    ) => {
                        message_count += 1;
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        state.renderer_message = message_count == 1
                            && payload.len() <= 128
                            && serde_json::from_slice::<serde_json::Value>(&payload).ok()
                                == Some(serde_json::json!({"kind":"code0NativeSmoke","value":7}));
                        if state.renderer_message {
                            ui_diagnostics.observed(DM::RendererMessage);
                        }
                    }
                    Observation::Native(
                        NativeSmokeObservation::BrowserProcessExited,
                        callback_thread,
                        callback_sta,
                    ) => {
                        browser_exit_count += 1;
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        if browser_exit_count != 1
                            || closing_since.is_none()
                            || state.owned_references_released
                        {
                            state.failure = "browserExitReceiptInvalid";
                        }
                    }
                    Observation::Session(
                        owned,
                        Code0WindowEvent::Released,
                        callback_thread,
                        callback_sta,
                    ) => {
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        state.owned_references_released =
                            owned == request && browser_exit_count == 1;
                        if state.owned_references_released {
                            ui_diagnostics.observed(DM::Released);
                        }
                    }
                    Observation::Session(_, _, _, _) => {
                        // This smoke never installs the production policy set or captures official data.
                        state.failure = "unexpectedProductionEvent";
                    }
                }
            }
            if let Some(label) = &owned_label
                && !state.destroyed
            {
                ui_diagnostics.phase(DP::LiveWindowQueries, false);
                let boundary = ui_diagnostics.boundary(DP::LiveWindowQueries);
                let safe = handle
                    .get_window(label)
                    .is_some_and(|window| hidden_bare_window(handle, &window));
                drop(boundary);
                ui_diagnostics.phase(DP::LiveWindowQueries, true);
                ui_diagnostics.phase(DP::ConstructionLock, false);
                live_construction.lock().unwrap().violation |= !safe;
                ui_diagnostics.phase(DP::ConstructionLock, true);
            }
            ui_diagnostics.phase(DP::ConstructionLock, false);
            live_construction.lock().unwrap().apply(&mut state);
            ui_diagnostics.phase(DP::ConstructionLock, true);
            if state.bare_window_violation {
                state.failure = "bareWindowViolation";
            }
            if !object_free_failure
                && closing_since.is_none()
                && (state.renderer_message
                    || started.elapsed() >= Duration::from_secs(45)
                    || state.failure != "notCompleted")
            {
                closing_since = Some(Instant::now());
                drop(state);
                ui_diagnostics.phase(DP::CloseRequest, false);
                driver.close(INSTANCE);
                ui_diagnostics.phase(DP::CloseRequest, true);
                ui_diagnostics.observed(DM::CloseRequested);
                ui_diagnostics.phase(DP::SummaryLock, false);
                state = observed.lock().unwrap();
                ui_diagnostics.phase(DP::SummaryLock, true);
            }
            if let Some(closing) = closing_since {
                if closing.elapsed() >= Duration::from_secs(10) {
                    // Check the deadline before newly arrived cleanup evidence:
                    // cleanup after 10s must still fail, even on this first tick.
                    // Retain/pump CleanupPending until real cleanup, or the
                    // original outer process watchdog fails.
                    state.failure = "closeEvidenceTimeout";
                }
                if state.destroyed && state.owned_references_released {
                    let code = if state.positive() && state.failure == "notCompleted" {
                        0
                    } else {
                        1
                    };
                    // Latch only after the original actual-cleanup/deadline/
                    // verdict gates, before release or the queued exit request.
                    terminal_exit_requested = true;
                    drop(state);
                    driver.release_profile_lease();
                    ui_diagnostics.phase(DP::ExitRequest, false);
                    handle.exit(code);
                    ui_diagnostics.phase(DP::ExitRequest, true);
                    return;
                }
            }
            drop(state);
            ui_diagnostics.phase(DP::EventDispatch, true);
            if let RunEvent::ExitRequested {
                code: None, api, ..
            } = event
            {
                // Last-window auto-exit cannot outrun release evidence and the explicit exit above.
                api.prevent_exit();
            }
        });
        diagnostics.phase(DP::RunReturn, true);
        diagnostics.observed(DM::RunReturned);
        let _ = wake_done.send(());
        diagnostics.phase(DP::WakeJoin, false);
        {
            let _boundary = diagnostics.boundary(DP::WakeJoin);
            let _ = wake.join();
        }
        diagnostics.phase(DP::WakeJoin, true);
        diagnostics.observed(DM::WakeJoined);
        // run_return has dropped its callback and UI-local driver before TempDir cleanup.
        diagnostics.phase(DP::TempClose, false);
        let cleanup_result = {
            let _boundary = diagnostics.boundary(DP::TempClose);
            temp.close()
        };
        diagnostics.phase(DP::TempClose, true);
        let cleanup_succeeded = cleanup_result.is_ok();
        diagnostics.emit_temp_close_once(&cleanup_result);
        drop(cleanup_result);
        if cleanup_succeeded {
            diagnostics.observed(DM::TempClosed);
        }
        diagnostics.phase(DP::SummaryLock, false);
        let mut state = summary.lock().unwrap();
        diagnostics.phase(DP::SummaryLock, true);
        diagnostics.phase(DP::ConstructionLock, false);
        construction.lock().unwrap().apply(&mut state);
        diagnostics.phase(DP::ConstructionLock, true);
        if state.bare_window_violation {
            state.failure = "bareWindowViolation";
        }
        state.run_returned = true;
        state.cleanup_succeeded = cleanup_succeeded;
        state.completed_cases = 1;
        state.pass = exit_code == 0
            && state.failure == "notCompleted"
            && state.positive()
            && cleanup_succeeded;
        if state.pass {
            state.failure = "none";
        } else if !cleanup_succeeded {
            state.failure = "profileCleanupFailed";
        }
        Ok(())
    }
}

fn main() {
    #[cfg(all(windows, debug_assertions, feature = "code0-native-fixture"))]
    match smoke::run() {
        Ok(true) => (),
        Ok(false) => std::process::exit(1),
        Err(_) => {
            eprintln!("Code0 native smoke failed; inspect fresh fixed-schema summary");
            std::process::exit(1);
        }
    }
    #[cfg(not(all(windows, debug_assertions, feature = "code0-native-fixture")))]
    {
        eprintln!(
            "Code0 native smoke requires Windows debug fixture build; unsupported is failure"
        );
        std::process::exit(2);
    }
}
