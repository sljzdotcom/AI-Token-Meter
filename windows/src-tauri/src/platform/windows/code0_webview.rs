//! UI-local bare-Window/direct-COM Code0 owner. Official loading remains denied
//! until the separately verified production policy and bridge installation phase.
use std::path::PathBuf;
use std::rc::Rc;

use super::code0_profile_lease::{Code0LeaseError, Code0ProfileLease};
use super::code0_web_session::{
    Code0CleanupRequest, Code0SessionFailure, Code0WindowDriving, Code0WindowEvent,
    Code0WindowRequest,
};

pub type Code0NativeEventSink = Rc<dyn Fn(Code0WindowRequest, Code0WindowEvent)>;

/// Rc deliberately keeps the native owner !Send/!Sync. Its caller must retain it
/// and pump the UI STA through matching Released, including failed creation.
pub struct Code0NativeDriver {
    app: tauri::AppHandle,
    profile_root: PathBuf,
    lease: Option<Code0ProfileLease>,
    #[cfg(windows)]
    owner: Rc<std::cell::RefCell<native::Owner>>,
    #[cfg(not(windows))]
    _events: Code0NativeEventSink,
    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    fixture_observer: Option<fixture::Observer>,
}

impl Code0NativeDriver {
    pub fn new(app: tauri::AppHandle, profile_root: PathBuf, events: Code0NativeEventSink) -> Self {
        Self {
            app,
            profile_root,
            lease: None,
            #[cfg(windows)]
            owner: Rc::new(std::cell::RefCell::new(native::Owner::new(events))),
            #[cfg(not(windows))]
            _events: events,
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            fixture_observer: None,
        }
    }

    /// Feed only real events from this app's main UI RunEvent route. Tauri's
    /// Send-bound per-window listener must never capture this Rc/COM owner.
    pub fn on_window_event(&mut self, label: &str, event: &tauri::WindowEvent) {
        #[cfg(windows)]
        native::window_event(&self.owner, label, event);
        #[cfg(not(windows))]
        let _ = (label, event);
    }

    /// Called by the UI event pump after previous callbacks have returned.
    /// Failure is sticky; this never retries a failed native cleanup action.
    pub fn poll_cleanup(&mut self) {
        #[cfg(windows)]
        native::poll(&self.owner);
    }
}

impl Code0WindowDriving for Code0NativeDriver {
    fn acquire_profile_lease(&mut self) -> Result<(), Code0SessionFailure> {
        if self.lease.as_ref().is_some_and(Code0ProfileLease::is_held) {
            return Ok(());
        }
        self.lease = Some(
            Code0ProfileLease::acquire(&self.profile_root).map_err(|error| match error {
                Code0LeaseError::Busy => Code0SessionFailure::ProfileBusy,
                Code0LeaseError::Unavailable => Code0SessionFailure::StorageFailed,
            })?,
        );
        Ok(())
    }

    fn release_profile_lease(&mut self) {
        #[cfg(all(windows, debug_assertions, feature = "code0-native-fixture"))]
        let diagnostics = self.owner.borrow().diagnostics.clone();
        #[cfg(all(windows, debug_assertions, feature = "code0-native-fixture"))]
        if let Some(diagnostic) = &diagnostics {
            diagnostic.lease_fact(1 << 0);
        }
        #[cfg(windows)]
        {
            let object_free = native::object_free(&self.owner, None);
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            if let Some(diagnostic) = &diagnostics {
                diagnostic.lease_fact((1 << 1) | (u32::from(object_free) << 2));
            }
            if !object_free {
                return;
            }
        }
        #[cfg(all(windows, debug_assertions, feature = "code0-native-fixture"))]
        let mut lease_taken = false;
        if let Some(lease) = self.lease.take() {
            #[cfg(all(windows, debug_assertions, feature = "code0-native-fixture"))]
            {
                lease_taken = true;
                if let Some(diagnostic) = &diagnostics {
                    diagnostic.lease_fact(1 << 3);
                }
            }
            lease.release();
            #[cfg(all(windows, debug_assertions, feature = "code0-native-fixture"))]
            if let Some(diagnostic) = &diagnostics {
                diagnostic.lease_fact(1 << 4);
            }
        }
        #[cfg(all(windows, debug_assertions, feature = "code0-native-fixture"))]
        if let Some(diagnostic) = &diagnostics {
            // The original local lease scope has ended. This is not an OS
            // handle-close result or a browser-process exit observation.
            diagnostic
                .lease_fact((u32::from(lease_taken) << 5) | (u32::from(self.lease.is_none()) << 6));
        }
    }

    fn create_unloaded(&mut self, request: Code0WindowRequest) -> Result<(), Code0SessionFailure> {
        if !self.lease.as_ref().is_some_and(Code0ProfileLease::is_held) {
            return Err(Code0SessionFailure::ProfileBusy);
        }
        #[cfg(windows)]
        {
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            let observer = self.fixture_observer.clone();
            native::create(
                &self.owner,
                &self.app,
                &self.profile_root,
                request,
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                observer,
            )
        }
        #[cfg(not(windows))]
        {
            let _ = (&self.app, request);
            Err(Code0SessionFailure::PolicyRejected)
        }
    }

    fn navigate(&mut self, _instance_id: &str) -> Result<(), Code0SessionFailure> {
        Err(Code0SessionFailure::PolicyRejected)
    }

    fn close(&mut self, instance_id: &str) {
        #[cfg(windows)]
        native::close(&self.owner, instance_id);
        #[cfg(not(windows))]
        let _ = instance_id;
    }

    fn clear_website_data(
        &mut self,
        _request: Code0CleanupRequest,
    ) -> Result<(), Code0SessionFailure> {
        Err(Code0SessionFailure::PolicyRejected)
    }
}

#[cfg(windows)]
mod native {
    use std::cell::RefCell;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    use std::path::PathBuf;
    use std::rc::{Rc, Weak};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::thread::{self, ThreadId};

    use tauri::Manager;
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    use webview2_com::{
        BrowserProcessExitedEventHandler, CoTaskMemPWSTR,
        CreateCoreWebView2ControllerCompletedHandler,
        CreateCoreWebView2EnvironmentCompletedHandler, WebMessageReceivedEventHandler,
    };
    use windows::Win32::Foundation::{E_FAIL, HWND, RECT};
    use windows::Win32::System::Com::{
        APTTYPE, APTTYPE_MAINSTA, APTTYPE_STA, APTTYPEQUALIFIER, CoGetApartmentType,
    };
    use windows::core::{IUnknown, Interface, PCWSTR, PWSTR};

    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    use super::fixture::{
        DiagnosticMilestone as DM, DiagnosticPhase as DP, DiagnosticUncertainty as DU,
        DiagnosticUncertaintyReason as UR, DiagnosticUncertaintySite as US,
        DiagnosticUncertaintyStage as UT, Diagnostics, NativeSmokeObservation, Observer,
    };
    use super::{Code0NativeEventSink, Code0SessionFailure, Code0WindowEvent, Code0WindowRequest};

    macro_rules! phase {
        ($owner:expr, $phase:ident, $returned:expr) => {
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            if let Some(diagnostic) = $owner.borrow().diagnostics.as_ref() {
                diagnostic.phase(DP::$phase, $returned);
            }
        };
    }
    macro_rules! milestone {
        ($owner:expr, $mark:ident) => {
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            if let Some(diagnostic) = $owner.borrow().diagnostics.as_ref() {
                diagnostic.observed(DM::$mark);
            }
        };
    }
    macro_rules! observed_call {
        ($owner:expr, $phase:ident, $call:expr) => {{
            phase!($owner, $phase, false);
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            let diagnostic = $owner.borrow().diagnostics.clone();
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            let _boundary = diagnostic.as_ref().map(|d| d.boundary(DP::$phase));
            let result = $call;
            phase!($owner, $phase, true);
            result
        }};
    }
    macro_rules! native_call {
        ($owner:expr, $phase:ident, $call:expr) => {{
            let result = observed_call!($owner, $phase, $call);
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            if let Some(diagnostic) = $owner.borrow().diagnostics.as_ref()
                && let Err(error) = &result
            {
                diagnostic.failure(DP::$phase, error.code().0);
            }
            result
        }};
    }

    // Same cleanup Result and call order. The optional source descriptor is
    // consumed only at cleanup_result's actual sticky-failure assignment.
    macro_rules! cleanup_at {
        ($owner:expr, $site:ident, $result:expr, $native:expr) => {{
            let result: windows::core::Result<()> = $result;
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            let detail = DU::cleanup(
                US::$site,
                if $native {
                    UR::NativeError
                } else {
                    UR::Condition
                },
                result
                    .as_ref()
                    .err()
                    .filter(|_| $native)
                    .map(|error| error.code().0),
            );
            cleanup_result(
                $owner,
                result,
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                detail,
            );
        }};
    }

    #[derive(Default)]
    struct Callbacks {
        pending: AtomicUsize,
        active: AtomicUsize,
        handlers: AtomicUsize,
        affinity_failed: AtomicBool,
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        diagnostics: Option<Arc<Diagnostics>>,
    }

    // Only atomics in these guards: even a never-invoked completion's actual
    // closure destruction can discharge its ticket without touching UI-local Rc.
    struct Pending(Arc<Callbacks>);
    impl Pending {
        fn new(callbacks: &Arc<Callbacks>) -> Self {
            callbacks.pending.fetch_add(1, Ordering::SeqCst);
            Self(Arc::clone(callbacks))
        }
    }
    impl Drop for Pending {
        fn drop(&mut self) {
            self.0.pending.fetch_sub(1, Ordering::SeqCst);
        }
    }
    struct Active(Arc<Callbacks>);
    impl Active {
        fn enter(callbacks: &Arc<Callbacks>) -> Self {
            callbacks.active.fetch_add(1, Ordering::SeqCst);
            Self(Arc::clone(callbacks))
        }
    }
    impl Drop for Active {
        fn drop(&mut self) {
            self.0.active.fetch_sub(1, Ordering::SeqCst);
        }
    }

    struct HandlerLifetime(Arc<Callbacks>);
    impl HandlerLifetime {
        fn new(callbacks: &Arc<Callbacks>) -> Self {
            callbacks.handlers.fetch_add(1, Ordering::SeqCst);
            Self(Arc::clone(callbacks))
        }
    }
    impl Drop for HandlerLifetime {
        fn drop(&mut self) {
            self.0.handlers.fetch_sub(1, Ordering::SeqCst);
        }
    }

    // A COM/Window operation may reenter the app pump. A nested poll must not
    // certify release while the outer poll still owns action-local interfaces.
    struct PollGuard(Rc<RefCell<Owner>>);
    impl Drop for PollGuard {
        fn drop(&mut self) {
            self.0.borrow_mut().polling = false;
        }
    }

    enum UnmatchedObject {
        Environment(ICoreWebView2Environment),
        Controller {
            object: ICoreWebView2Controller,
            attempted: bool,
        },
    }

    pub(super) struct Owner {
        // A mismatched completion violates the serialized ownership protocol.
        // Retain and dispose its actual object on UI, but never certify release
        // of a different request or permit the profile lease to be released.
        unmatched: bool,
        unmatched_objects: Vec<UnmatchedObject>,
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        unmatched_script_ids: Vec<String>,
        polling: bool,
        ui_thread: ThreadId,
        events: Code0NativeEventSink,
        instance: Option<Instance>,
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        pub(super) diagnostics: Option<Arc<Diagnostics>>,
    }

    #[derive(Clone, Copy)]
    enum BrowserExitState {
        // Only the existing failed-before-build path can be object-free without
        // a browser witness. Starting native construction creates an obligation.
        NotStarted,
        Unarmed,
        Armed(u32),
        Observed,
        Unknown,
    }

    struct Instance {
        request: Code0WindowRequest,
        callbacks: Arc<Callbacks>,
        window: Option<tauri::Window>,
        hwnd: Option<HWND>,
        had_window: bool,
        build_started: bool,
        destroyed: bool,
        environment: Option<ICoreWebView2Environment>,
        environment5: Option<ICoreWebView2Environment5>,
        exit_token: Option<i64>,
        exit_remove_attempted: bool,
        browser_exit: BrowserExitState,
        controls_disposed: bool,
        controller: Option<ICoreWebView2Controller>,
        core: Option<ICoreWebView2>,
        message_token: Option<i64>,
        remove_attempted: bool,
        stop_attempted: bool,
        close_attempted: bool,
        destroy_attempted: bool,
        closing: bool,
        failed: bool,
        failure_sent: bool,
        cleanup_blocked: bool,
        released: bool,
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        observer: Option<Observer>,
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        inline_loaded: bool,
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        policy: Option<p1::Installation>,
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        policy_mailbox: Option<Arc<p1::Mailbox>>,
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        policy_fault: p1::FaultState,
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        refusal_registry: NativeRefusalRegistry,
    }

    impl Owner {
        pub(super) fn new(events: Code0NativeEventSink) -> Self {
            Self {
                unmatched: false,
                unmatched_objects: Vec::new(),
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                unmatched_script_ids: Vec::new(),
                polling: false,
                ui_thread: thread::current().id(),
                events,
                instance: None,
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                diagnostics: None,
            }
        }
    }

    fn is_sta() -> bool {
        let mut apartment = APTTYPE::default();
        let mut qualifier = APTTYPEQUALIFIER::default();
        unsafe { CoGetApartmentType(&mut apartment, &mut qualifier) }.is_ok()
            && (apartment == APTTYPE_STA || apartment == APTTYPE_MAINSTA)
    }

    fn on_ui(owner: &Rc<RefCell<Owner>>) -> bool {
        let ui_thread = owner.borrow().ui_thread;
        ui_thread == thread::current().id() && is_sta()
    }

    fn callback_on_ui(ui: ThreadId, callbacks: &Callbacks) -> bool {
        let valid = thread::current().id() == ui && is_sta();
        if !valid {
            callbacks.affinity_failed.store(true, Ordering::SeqCst);
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            if let Some(diagnostic) = callbacks.diagnostics.as_ref() {
                diagnostic.failure(DP::UiAffinity, 0);
            }
        }
        valid
    }

    fn matching<'a>(
        owner: &'a mut Owner,
        request: &Code0WindowRequest,
    ) -> Option<&'a mut Instance> {
        owner.instance.as_mut().filter(|v| &v.request == request)
    }

    fn failed(owner: &Rc<RefCell<Owner>>, request: &Code0WindowRequest) {
        if let Some(instance) = matching(&mut owner.borrow_mut(), request) {
            instance.failed = true;
            instance.closing = true;
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            let _ = s2::mark_popup_close(instance);
        }
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s2::publish_popup_close(owner);
    }

    fn inventory_empty(instance: &Instance) -> bool {
        instance.window.is_none()
            && (!instance.had_window || instance.destroyed)
            && instance.environment.is_none()
            && instance.environment5.is_none()
            && instance.exit_token.is_none()
            && (instance.controls_disposed || !instance.build_started)
            && (matches!(instance.browser_exit, BrowserExitState::Observed)
                && instance.exit_remove_attempted
                || !instance.build_started
                    && matches!(instance.browser_exit, BrowserExitState::NotStarted))
            && instance.controller.is_none()
            && instance.core.is_none()
            && instance.message_token.is_none()
            && {
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                {
                    instance.policy.is_none()
                        && p1::mailbox_quiescent(instance)
                        && instance.refusal_registry.references_empty()
                }
                #[cfg(not(all(debug_assertions, feature = "code0-native-fixture")))]
                {
                    true
                }
            }
            && instance.callbacks.pending.load(Ordering::SeqCst) == 0
            && instance.callbacks.active.load(Ordering::SeqCst) == 0
            && instance.callbacks.handlers.load(Ordering::SeqCst) == 0
            && !instance.cleanup_blocked
            && !instance.callbacks.affinity_failed.load(Ordering::SeqCst)
    }

    pub(super) fn object_free(owner: &Rc<RefCell<Owner>>, instance_id: Option<&str>) -> bool {
        let state = owner.borrow();
        if state.unmatched {
            return false;
        }
        match &state.instance {
            None => instance_id.is_none(),
            Some(instance) => {
                instance_id.is_none_or(|id| id == instance.request.instance_id)
                    && inventory_empty(instance)
                    && (instance.released || !instance.build_started)
            }
        }
    }

    // Always guard/free a non-null allocated output, including HRESULT failure.
    // Bound scanning/copying and reject malformed UTF-16 rather than normalizing.
    fn allocated_string(
        max_units: usize,
        get: impl FnOnce(*mut PWSTR) -> windows::core::Result<()>,
    ) -> windows::core::Result<String> {
        let mut raw = PWSTR::null();
        let result = get(&mut raw);
        let _allocation = CoTaskMemPWSTR::from(raw);
        result?;
        if raw.is_null() {
            return Err(E_FAIL.into());
        }
        for length in 0..=max_units {
            // COM getter guarantees a valid NUL-terminated allocated PWSTR.
            if unsafe { *raw.0.add(length) } == 0 {
                let units = unsafe { std::slice::from_raw_parts(raw.0, length) };
                return String::from_utf16(units).map_err(|_| E_FAIL.into());
            }
        }
        Err(E_FAIL.into())
    }

    pub(super) fn create(
        owner: &Rc<RefCell<Owner>>,
        app: &tauri::AppHandle,
        profile: &Path,
        request: Code0WindowRequest,
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))] observer: Option<Observer>,
    ) -> Result<(), Code0SessionFailure> {
        if !on_ui(owner) || !object_free(owner, None) {
            return Err(Code0SessionFailure::PolicyRejected);
        }
        if owner
            .borrow()
            .instance
            .as_ref()
            .is_some_and(|instance| !instance.released)
        {
            return Err(Code0SessionFailure::PolicyRejected);
        }
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        let callbacks = Arc::new(Callbacks {
            diagnostics: owner.borrow().diagnostics.clone(),
            ..Callbacks::default()
        });
        #[cfg(not(all(debug_assertions, feature = "code0-native-fixture")))]
        let callbacks = Arc::new(Callbacks::default());
        // Encompass the entire outer create action, not only its callbacks.
        // Declared before every Window/handler/interface local so all of those
        // drop first on success and every early return. A synchronous factory
        // callback may finish before the factory call itself returns.
        let _action = Active::enter(&callbacks);
        owner.borrow_mut().instance = Some(Instance {
            request: request.clone(),
            callbacks: Arc::clone(&callbacks),
            window: None,
            hwnd: None,
            had_window: false,
            build_started: false,
            destroyed: false,
            environment: None,
            environment5: None,
            exit_token: None,
            exit_remove_attempted: false,
            browser_exit: BrowserExitState::NotStarted,
            controls_disposed: false,
            controller: None,
            core: None,
            message_token: None,
            remove_attempted: false,
            stop_attempted: false,
            close_attempted: false,
            destroy_attempted: false,
            closing: false,
            failed: false,
            failure_sent: false,
            cleanup_blocked: false,
            released: false,
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            observer: observer.clone(),
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            inline_loaded: false,
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            policy: None,
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            policy_mailbox: None,
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            policy_fault: p1::FaultState::default(),
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            refusal_registry: NativeRefusalRegistry::not_installed(),
        });
        let label = format!("code0-owned-{}", request.instance_id);
        if label.len() > 128
            || !label
                .bytes()
                .all(|v| v.is_ascii_alphanumeric() || v == b'-' || v == b'_')
            || app.get_window(&label).is_some()
        {
            failed(owner, &request);
            return Err(Code0SessionFailure::PolicyRejected);
        }
        {
            let mut state = owner.borrow_mut();
            let instance = matching(&mut state, &request).unwrap();
            instance.build_started = true;
            instance.browser_exit = BrowserExitState::Unarmed;
        }
        let built = observed_call!(
            owner,
            WindowBuild,
            tauri::WindowBuilder::new(app, label)
                .visible(false)
                .focused(false)
                .focusable(false)
                .build()
        );
        let window = match built {
            Ok(window) => window,
            Err(_) => {
                // A build error supplies no actual handle/destruction receipt.
                // Keep the unknown partial-build obligation; do not certify zero.
                let mut state = owner.borrow_mut();
                let instance = matching(&mut state, &request).unwrap();
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                if let Some(diagnostic) = instance.callbacks.diagnostics.as_ref() {
                    diagnostic.uncertainty(DU::cleanup(US::WindowBuild, UR::Condition, None));
                }
                instance.cleanup_blocked = true;
                drop(state);
                failed(owner, &request);
                return Err(Code0SessionFailure::PolicyRejected);
            }
        };
        {
            let mut state = owner.borrow_mut();
            let instance = matching(&mut state, &request).expect("owned create identity");
            instance.had_window = true;
            instance.window = Some(window.clone());
        }
        milestone!(owner, WindowBuilt);
        phase!(owner, ConstructionReceipt, false);
        // Frozen synchronous receipt: no correction, getter or asynchronous
        // environment work precedes this observation of the actual bare Window.
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        if let Some(observer) = observer {
            observer(NativeSmokeObservation::WindowConstructed {
                window: Box::new(window.clone()),
            });
        }
        phase!(owner, ConstructionReceipt, true);
        let actual_hwnd = observed_call!(owner, WindowHwnd, window.hwnd());
        let hwnd = match actual_hwnd {
            Ok(hwnd) => hwnd,
            Err(_) => {
                failed(owner, &request);
                return Err(Code0SessionFailure::PolicyRejected);
            }
        };
        matching(&mut owner.borrow_mut(), &request).unwrap().hwnd = Some(hwnd);
        let mut profile_wide: Vec<u16> = profile.as_os_str().encode_wide().collect();
        if !profile.is_absolute()
            || profile_wide.is_empty()
            || profile_wide.len() > 32767
            || profile_wide.contains(&0)
        {
            failed(owner, &request);
            return Err(Code0SessionFailure::StorageFailed);
        }
        profile_wide.push(0);
        let ui = owner.borrow().ui_thread;
        let weak = Rc::downgrade(owner);
        let owned_request = request.clone();
        let ticket = Pending::new(&callbacks);
        let handler = CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(
            move |result, environment| {
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                if let Some(diagnostic) = callbacks.diagnostics.as_ref() {
                    diagnostic.phase(DP::EnvironmentCallback, false);
                    diagnostic.observed(DM::EnvironmentCallback);
                    if let Err(error) = &result {
                        diagnostic.failure(DP::EnvironmentCallback, error.code().0);
                    }
                    if result.is_ok() && environment.is_some() {
                        diagnostic.observed(DM::EnvironmentSucceeded);
                    }
                }
                let active = Active::enter(&callbacks);
                if callback_on_ui(ui, &callbacks) {
                    environment_completed(&weak, &owned_request, result, environment);
                } else {
                    drop(environment);
                }
                drop(active);
                drop(ticket);
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                if let Some(diagnostic) = callbacks.diagnostics.as_ref() {
                    diagnostic.phase(DP::EnvironmentCallback, true);
                }
                Ok(())
            },
        ));
        let accepted = native_call!(owner, EnvironmentFactory, unsafe {
            CreateCoreWebView2EnvironmentWithOptions(
                PCWSTR::null(),
                PCWSTR(profile_wide.as_ptr()),
                None::<&ICoreWebView2EnvironmentOptions>,
                &handler,
            )
        });
        // Do not retain this callback interface. If COM did not retain/invoke it,
        // actual closure destruction discharges the pending ticket. HRESULT alone
        // never supplies that fact, and a COM-held late callback keeps it pending.
        drop(handler);
        if accepted.is_err() {
            failed(owner, &request);
            return Err(Code0SessionFailure::PolicyRejected);
        }
        Ok(())
    }

    fn browser_exit_unknown(owner: &Rc<RefCell<Owner>>, request: &Code0WindowRequest) {
        let mut state = owner.borrow_mut();
        if let Some(instance) = matching(&mut state, request) {
            instance.browser_exit = BrowserExitState::Unknown;
            instance.failed = true;
            instance.closing = true;
        } else {
            state.unmatched = true;
        }
    }

    fn install_browser_exit_handler(
        owner: &Rc<RefCell<Owner>>,
        request: &Code0WindowRequest,
    ) -> windows::core::Result<()> {
        // The environment completion's Active guard covers every registration,
        // callback, interface and action local until this function returns.
        let (environment, ui, callbacks) = {
            let state = owner.borrow();
            let instance = state.instance.as_ref().ok_or(E_FAIL)?;
            if &instance.request != request || instance.environment5.is_some() {
                return Err(E_FAIL.into());
            }
            (
                instance.environment.clone().ok_or(E_FAIL)?,
                state.ui_thread,
                Arc::clone(&instance.callbacks),
            )
        };
        let environment5: ICoreWebView2Environment5 =
            native_call!(owner, ExitCast, environment.cast())?;
        {
            let mut state = owner.borrow_mut();
            let instance = matching(&mut state, request).ok_or(E_FAIL)?;
            instance.environment5 = Some(environment5.clone());
        }
        let weak = Rc::downgrade(owner);
        let owned_request = request.clone();
        let lifetime = HandlerLifetime::new(&callbacks);
        let handler = BrowserProcessExitedEventHandler::create(Box::new(move |sender, args| {
            let _lifetime = &lifetime;
            let active = Active::enter(&callbacks);
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            if let Some(diagnostic) = callbacks.diagnostics.as_ref() {
                diagnostic.phase(DP::ExitCallback, false);
            }
            if callback_on_ui(ui, &callbacks)
                && let Some(owner) = weak.upgrade()
                && browser_process_exited(&owner, &owned_request, sender.as_ref(), args.as_ref())
                    .is_err()
            {
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                if let Some(diagnostic) = callbacks.diagnostics.as_ref() {
                    diagnostic.failure(DP::ExitCallback, E_FAIL.0);
                }
                browser_exit_unknown(&owner, &owned_request);
            }
            // The wrapper supplies owned interface clones. They must end while
            // Active still prevents any reentrant poll from certifying release.
            drop(args);
            drop(sender);
            drop(active);
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            if let Some(diagnostic) = callbacks.diagnostics.as_ref() {
                diagnostic.phase(DP::ExitCallback, true);
            }
            Ok(())
        }));
        let mut token = 0;
        let registered = native_call!(owner, ExitAdd, unsafe {
            environment5.add_BrowserProcessExited(&handler, &mut token)
        });
        drop(handler);
        registered?;
        {
            let mut state = owner.borrow_mut();
            let instance = matching(&mut state, request).ok_or(E_FAIL)?;
            // Token zero is valid. Reentrant failure/closing remains sticky; the
            // successful registration must still be retained for exact removal.
            instance.exit_token = Some(token);
        }
        milestone!(owner, ExitRegistered);
        Ok(())
    }

    fn browser_process_exited(
        owner: &Rc<RefCell<Owner>>,
        request: &Code0WindowRequest,
        sender: Option<&ICoreWebView2Environment>,
        args: Option<&ICoreWebView2BrowserProcessExitedEventArgs>,
    ) -> windows::core::Result<()> {
        let environment = {
            let state = owner.borrow();
            let instance = state.instance.as_ref().ok_or(E_FAIL)?;
            if &instance.request != request {
                return Err(E_FAIL.into());
            }
            instance.environment.clone().ok_or(E_FAIL)?
        };
        let sender = sender.ok_or(E_FAIL)?;
        let args = args.ok_or(E_FAIL)?;
        // Compare only canonical IUnknown pointers after fallible QI. Do not use
        // IUnknown::eq: its cached implementation can perform QI with unwrap.
        let expected_identity: IUnknown = native_call!(owner, ExitIdentity, environment.cast())?;
        let sender_identity: IUnknown = native_call!(owner, ExitIdentity, sender.cast())?;
        if expected_identity.as_raw() != sender_identity.as_raw() {
            return Err(E_FAIL.into());
        }
        let mut pid = 0;
        let mut kind = COREWEBVIEW2_BROWSER_PROCESS_EXIT_KIND::default();
        native_call!(owner, ExitPidGetter, unsafe {
            args.BrowserProcessId(&mut pid)
        })?;
        native_call!(owner, ExitKindGetter, unsafe {
            args.BrowserProcessExitKind(&mut kind)
        })?;
        {
            let mut state = owner.borrow_mut();
            let instance = matching(&mut state, request).ok_or(E_FAIL)?;
            if !instance.closing
                || !matches!(instance.browser_exit, BrowserExitState::Armed(expected) if expected == pid && pid != 0)
                || kind != COREWEBVIEW2_BROWSER_PROCESS_EXIT_KIND_NORMAL
            {
                return Err(E_FAIL.into());
            }
            // This is only a candidate witnessed by the actual callback. Poll
            // must still dispose controls, unregister, and drop all references.
            instance.browser_exit = BrowserExitState::Observed;
        }
        milestone!(owner, ExitObserved);
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        {
            let observer = owner.borrow().instance.as_ref().unwrap().observer.clone();
            if let Some(observer) = observer {
                observer(NativeSmokeObservation::BrowserProcessExited);
            }
        }
        Ok(())
    }

    fn arm_browser_exit(owner: &Rc<RefCell<Owner>>) {
        let candidate = {
            let state = owner.borrow();
            let instance = state.instance.as_ref().unwrap();
            if !matches!(instance.browser_exit, BrowserExitState::Unarmed) {
                return;
            }
            (
                instance.request.clone(),
                instance.core.clone(),
                instance.environment5.is_some() && instance.exit_token.is_some(),
            )
        };
        let (request, core, registered) = candidate;
        let Some(core) = core.filter(|_| registered) else {
            browser_exit_unknown(owner, &request);
            return;
        };
        let mut pid = 0;
        let result = native_call!(owner, BrowserPid, unsafe {
            core.BrowserProcessId(&mut pid)
        });
        drop(core);
        if result.is_err() || pid == 0 {
            browser_exit_unknown(owner, &request);
            return;
        }
        let armed = {
            let mut state = owner.borrow_mut();
            let instance = matching(&mut state, &request).unwrap();
            if matches!(instance.browser_exit, BrowserExitState::Unarmed) {
                instance.browser_exit = BrowserExitState::Armed(pid);
                true
            } else {
                // A callback can reenter the PID getter before arming. Its
                // unknown disposition must never be overwritten by this result.
                false
            }
        };
        if armed {
            milestone!(owner, ExitArmed);
        } else {
            browser_exit_unknown(owner, &request);
        }
    }

    fn environment_completed(
        weak: &Weak<RefCell<Owner>>,
        request: &Code0WindowRequest,
        result: windows::core::Result<()>,
        environment: Option<ICoreWebView2Environment>,
    ) {
        let Some(owner) = weak.upgrade() else {
            return;
        };
        let has_environment = {
            let mut state = owner.borrow_mut();
            let Some(instance) = matching(&mut state, request) else {
                state.unmatched = true;
                if let Some(environment) = environment {
                    state
                        .unmatched_objects
                        .push(UnmatchedObject::Environment(environment));
                }
                return;
            };
            instance.environment = environment;
            if result.is_err() || instance.environment.is_none() {
                instance.failed = true;
                instance.closing = true;
            }
            instance.environment.is_some()
        };
        if !has_environment || install_browser_exit_handler(&owner, request).is_err() {
            browser_exit_unknown(&owner, request);
            return;
        }
        let (ui, callbacks, hwnd) = {
            let mut state = owner.borrow_mut();
            let ui = state.ui_thread;
            let Some(instance) = matching(&mut state, request) else {
                state.unmatched = true;
                return;
            };
            if instance.closing
                || instance.failed
                || instance.exit_token.is_none()
                || !matches!(instance.browser_exit, BrowserExitState::Unarmed)
            {
                return;
            }
            (ui, Arc::clone(&instance.callbacks), instance.hwnd)
        };
        let Some(hwnd) = hwnd else {
            failed(&owner, request);
            return;
        };
        let environment = owner
            .borrow()
            .instance
            .as_ref()
            .unwrap()
            .environment
            .clone()
            .unwrap();
        let ticket = Pending::new(&callbacks);
        let weak = Rc::downgrade(&owner);
        let owned_request = request.clone();
        let handler = CreateCoreWebView2ControllerCompletedHandler::create(Box::new(
            move |result, controller| {
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                if let Some(diagnostic) = callbacks.diagnostics.as_ref() {
                    diagnostic.phase(DP::ControllerCallback, false);
                    diagnostic.observed(DM::ControllerCallback);
                    if let Err(error) = &result {
                        diagnostic.failure(DP::ControllerCallback, error.code().0);
                    }
                    if result.is_ok() && controller.is_some() {
                        diagnostic.observed(DM::ControllerSucceeded);
                    }
                }
                let active = Active::enter(&callbacks);
                if callback_on_ui(ui, &callbacks) {
                    controller_completed(&weak, &owned_request, result, controller);
                } else {
                    drop(controller);
                }
                drop(active);
                drop(ticket);
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                if let Some(diagnostic) = callbacks.diagnostics.as_ref() {
                    diagnostic.phase(DP::ControllerCallback, true);
                }
                Ok(())
            },
        ));
        let accepted = native_call!(owner, ControllerFactory, unsafe {
            environment.CreateCoreWebView2Controller(hwnd, &handler)
        });
        drop(handler);
        drop(environment);
        if accepted.is_err() {
            failed(&owner, request);
        }
    }

    fn controller_completed(
        weak: &Weak<RefCell<Owner>>,
        request: &Code0WindowRequest,
        result: windows::core::Result<()>,
        controller: Option<ICoreWebView2Controller>,
    ) {
        let Some(owner) = weak.upgrade() else {
            // Caller's contract retains the owner through its pending tickets.
            // Even a violated caller lifetime must close an actual late object.
            if let Some(controller) = controller {
                let _ = unsafe { controller.Close() };
            }
            return;
        };
        let current = {
            let mut state = owner.borrow_mut();
            let Some(instance) = matching(&mut state, request) else {
                state.unmatched = true;
                if let Some(controller) = controller {
                    state.unmatched_objects.push(UnmatchedObject::Controller {
                        object: controller,
                        attempted: false,
                    });
                }
                return;
            };
            instance.controller = controller;
            if result.is_err() || instance.controller.is_none() {
                instance.failed = true;
                instance.closing = true;
            }
            !instance.closing
        };
        if !current {
            return;
        }
        phase!(owner, Configure, false);
        let configured = initialize_core(&owner, request);
        phase!(owner, Configure, true);
        if configured.is_err() {
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            if let Some(diagnostic) = owner.borrow().diagnostics.as_ref() {
                diagnostic.failure(DP::Configure, 0);
            }
            failed(&owner, request);
        }
    }

    fn initialize_core(
        owner: &Rc<RefCell<Owner>>,
        request: &Code0WindowRequest,
    ) -> windows::core::Result<()> {
        let (controller, environment, window, ui, callbacks) = {
            let state = owner.borrow();
            let instance = state.instance.as_ref().unwrap();
            (
                instance.controller.clone().unwrap(),
                instance.environment.clone().unwrap(),
                instance.window.clone().ok_or(E_FAIL)?,
                state.ui_thread,
                Arc::clone(&instance.callbacks),
            )
        };
        native_call!(owner, ControllerVisibility, unsafe {
            controller.SetIsVisible(false)
        })?;
        let core = native_call!(owner, CoreGetter, unsafe { controller.CoreWebView2() })?;
        // Retain the core before any subsequent operation can fail/reenter.
        matching(&mut owner.borrow_mut(), request).unwrap().core = Some(core.clone());
        let size = observed_call!(owner, BoundsGetter, window.inner_size());
        let size = size.map_err(|_| windows::core::Error::from(E_FAIL))?;
        let right = i32::try_from(size.width).map_err(|_| windows::core::Error::from(E_FAIL))?;
        let bottom = i32::try_from(size.height).map_err(|_| windows::core::Error::from(E_FAIL))?;
        native_call!(owner, BoundsSet, unsafe {
            controller.SetBounds(RECT {
                left: 0,
                top: 0,
                right,
                bottom,
            })
        })?;
        let version = allocated_string(128, |raw| {
            native_call!(owner, VersionGetter, unsafe {
                environment.BrowserVersionString(raw)
            })
        })?;
        if version.is_empty() || !version.bytes().all(|v| v.is_ascii_digit() || v == b'.') {
            return Err(E_FAIL.into());
        }
        let environment7: ICoreWebView2Environment7 =
            native_call!(owner, ProfileCast, environment.cast())?;
        let profile = allocated_string(32767, |raw| {
            native_call!(owner, ProfileGetter, unsafe {
                environment7.UserDataFolder(raw)
            })
        })?;
        if profile.is_empty() || !Path::new(&profile).is_absolute() {
            return Err(E_FAIL.into());
        }
        milestone!(owner, GettersSucceeded);
        let weak = Rc::downgrade(owner);
        let owned_request = request.clone();
        let lifetime = HandlerLifetime::new(&callbacks);
        let handler = WebMessageReceivedEventHandler::create(Box::new(move |sender, args| {
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            if let Some(diagnostic) = callbacks.diagnostics.as_ref() {
                diagnostic.phase(DP::MessageCallback, false);
                diagnostic.observed(DM::RendererCallback);
            }
            // Keep the registration lifetime in the FnMut closure until the real
            // COM handler releases it, including failed add/remove paths.
            let _lifetime = &lifetime;
            let active = Active::enter(&callbacks);
            if callback_on_ui(ui, &callbacks)
                && let Some(owner) = weak.upgrade()
            {
                message_received(&owner, &owned_request, sender.as_ref(), args.as_ref());
            }
            drop(args);
            drop(sender);
            drop(active);
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            if let Some(diagnostic) = callbacks.diagnostics.as_ref() {
                diagnostic.phase(DP::MessageCallback, true);
            }
            Ok(())
        }));
        let mut token = 0;
        let registered = native_call!(owner, MessageAdd, unsafe {
            core.add_WebMessageReceived(&handler, &mut token)
        });
        drop(handler);
        registered?;
        milestone!(owner, MessageRegistered);
        let ready = {
            let mut state = owner.borrow_mut();
            let instance = matching(&mut state, request).unwrap();
            instance.message_token = Some(token);
            !instance.closing
        };
        // No production PoliciesInstalled acknowledgement. This tiny smoke does
        // not establish refusal handlers, bridge registration or official origin.
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        if ready {
            milestone!(owner, EngineReady);
            let observer = owner.borrow().instance.as_ref().unwrap().observer.clone();
            if let Some(observer) = observer {
                observer(NativeSmokeObservation::EngineReady {
                    window_label: window.label().to_owned(),
                    user_data_folder: PathBuf::from(profile),
                    runtime_version: version,
                });
            }
        }
        #[cfg(not(all(debug_assertions, feature = "code0-native-fixture")))]
        let _ = (ready, version, profile);
        Ok(())
    }

    fn message_received(
        owner: &Rc<RefCell<Owner>>,
        request: &Code0WindowRequest,
        sender: Option<&ICoreWebView2>,
        args: Option<&ICoreWebView2WebMessageReceivedEventArgs>,
    ) {
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        {
            if owner
                .borrow()
                .instance
                .as_ref()
                .is_some_and(|i| i.policy.is_some())
            {
                if p1::message(owner, request, sender, args).is_err() {
                    failed(owner, request);
                }
                return;
            }
            let observer = {
                let mut state = owner.borrow_mut();
                let Some(instance) = matching(&mut state, request) else {
                    return;
                };
                if instance.closing || !instance.inline_loaded {
                    return;
                }
                instance.observer.clone()
            };
            let payload = args
                .ok_or_else(|| windows::core::Error::from(E_FAIL))
                .and_then(|args| {
                    allocated_string(128, |raw| {
                        native_call!(owner, MessageGetter, unsafe { args.WebMessageAsJson(raw) })
                    })
                });
            match payload {
                Ok(payload) if payload.len() <= 128 => {
                    if let Some(observer) = observer {
                        observer(NativeSmokeObservation::RendererMessage {
                            payload: payload.into_bytes(),
                        });
                    }
                }
                _ => failed(owner, request),
            }
        }
        #[cfg(not(all(debug_assertions, feature = "code0-native-fixture")))]
        let _ = (owner, request, sender, args);
    }

    pub(super) fn close(owner: &Rc<RefCell<Owner>>, instance_id: &str) {
        if !on_ui(owner) {
            return;
        }
        let mut state = owner.borrow_mut();
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        let unmatched = state.unmatched;
        if let Some(instance) = state.instance.as_mut()
            && instance.request.instance_id == instance_id
        {
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            p1::mark_stale_close(instance, unmatched);
            // Revoke before any external Stop/Close/removal can reenter.
            instance.closing = true;
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            let _ = s2::mark_child_close(instance);
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            let _ = s2::mark_dialog_close(instance);
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            let _ = s2::mark_popup_close(instance);
        }
        drop(state);
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s2::publish_popup_close(owner);
        // Only a later UI poll can certify actual inventory and release.
    }

    pub(super) fn window_event(
        owner: &Rc<RefCell<Owner>>,
        label: &str,
        event: &tauri::WindowEvent,
    ) {
        if !on_ui(owner) {
            return;
        }
        let window = {
            let mut state = owner.borrow_mut();
            let Some(instance) = state.instance.as_mut() else {
                return;
            };
            if instance.window.as_ref().is_none_or(|v| v.label() != label) {
                return;
            }
            if matches!(event, tauri::WindowEvent::Destroyed) {
                instance.destroyed = true;
                instance.closing = true;
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                let _ = s2::mark_popup_close(instance);
                instance.window.take()
            } else {
                None
            }
        };
        if window.is_some() {
            milestone!(owner, WindowDestroyed);
        }
        drop(window);
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s2::publish_popup_close(owner);
    }

    fn cleanup_result(
        owner: &Rc<RefCell<Owner>>,
        result: windows::core::Result<()>,
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))] detail: DU,
    ) {
        if result.is_err() {
            let mut state = owner.borrow_mut();
            let instance = state.instance.as_mut().unwrap();
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            if let Some(diagnostic) = instance.callbacks.diagnostics.as_ref() {
                diagnostic.uncertainty(detail);
            }
            instance.failed = true;
            instance.cleanup_blocked = true;
        }
    }

    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    fn publish_inventory(owner: &Rc<RefCell<Owner>>) {
        let state = owner.borrow();
        if let Some(diagnostic) = state.diagnostics.as_ref()
            && let Some(instance) = state.instance.as_ref()
        {
            let flags = u32::from(instance.window.is_some())
                | (u32::from(instance.environment.is_some()) << 1)
                | (u32::from(instance.controller.is_some()) << 2)
                | (u32::from(instance.core.is_some()) << 3)
                | (u32::from(instance.message_token.is_some()) << 4)
                | (u32::from(instance.closing) << 5)
                | (u32::from(instance.destroyed) << 6)
                | (u32::from(instance.cleanup_blocked) << 7)
                | (u32::from(instance.released) << 8)
                | (u32::from(instance.callbacks.affinity_failed.load(Ordering::SeqCst)) << 9)
                | (u32::from(state.unmatched) << 10)
                | (u32::from(state.polling) << 11)
                | (u32::from(instance.remove_attempted) << 12)
                | (u32::from(instance.stop_attempted) << 13)
                | (u32::from(instance.close_attempted) << 14)
                | (u32::from(instance.destroy_attempted) << 15)
                | (u32::from(instance.environment5.is_some()) << 16)
                | (u32::from(instance.exit_token.is_some()) << 17)
                | (u32::from(matches!(
                    instance.browser_exit,
                    BrowserExitState::Unarmed
                        | BrowserExitState::Armed(_)
                        | BrowserExitState::Unknown
                )) << 18)
                | (u32::from(matches!(instance.browser_exit, BrowserExitState::Observed)) << 19)
                | (u32::from(instance.controls_disposed) << 20)
                | (u32::from(instance.exit_remove_attempted) << 21)
                | (u32::from(matches!(instance.browser_exit, BrowserExitState::Unknown)) << 22);
            diagnostic.publish_inventory(
                instance.callbacks.pending.load(Ordering::SeqCst),
                instance.callbacks.active.load(Ordering::SeqCst),
                instance.callbacks.handlers.load(Ordering::SeqCst),
                flags,
            );
        }
    }

    pub(super) fn poll(owner: &Rc<RefCell<Owner>>) {
        if !on_ui(owner) {
            return;
        }
        {
            let mut state = owner.borrow_mut();
            if state.polling {
                return;
            }
            state.polling = true;
        }
        let _poll = PollGuard(Rc::clone(owner));
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        publish_inventory(owner);
        // Installation progress and receipt publication happen only on a later
        // UI poll, after all previous callbacks and outer actions have settled.
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        p1::advance(owner);
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s2::advance_child(owner);
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s2::advance_dialog(owner);
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s2::advance_popup(owner);
        let unmatched = std::mem::take(&mut owner.borrow_mut().unmatched_objects);
        for object in unmatched {
            match object {
                UnmatchedObject::Environment(environment) => drop(environment),
                UnmatchedObject::Controller { object, attempted } => {
                    if attempted || unsafe { object.Close() }.is_err() {
                        owner
                            .borrow_mut()
                            .unmatched_objects
                            .push(UnmatchedObject::Controller {
                                object,
                                attempted: true,
                            });
                    }
                }
            }
        }
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s2::synchronize_popup_close(owner);
        {
            let mut state = owner.borrow_mut();
            let unmatched = state.unmatched;
            let Some(instance) = state.instance.as_mut() else {
                return;
            };
            if unmatched || instance.callbacks.affinity_failed.load(Ordering::SeqCst) {
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                if let Some(diagnostic) = instance.callbacks.diagnostics.as_ref() {
                    diagnostic.uncertainty(DU::cleanup(
                        US::PollIdentity,
                        if unmatched {
                            UR::Unmatched
                        } else {
                            UR::Affinity
                        },
                        None,
                    ));
                }
                instance.failed = true;
                instance.closing = true;
                instance.cleanup_blocked = true;
            }
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            if instance.refusal_registry.unknown() {
                instance.failed = true;
                instance.closing = true;
                instance.cleanup_blocked = true;
            }
            if !instance.closing
                || instance.callbacks.active.load(Ordering::SeqCst) != 0
                || instance.callbacks.pending.load(Ordering::SeqCst) != 0
            {
                return;
            }
        }
        // Bind the actual core's process before Stop/Close can synchronously
        // deliver exit. PollGuard covers this getter's action-local interface.
        arm_browser_exit(owner);
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        p1::remove_registrations(owner);
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s2::remove(owner);
        let remove = {
            let mut state = owner.borrow_mut();
            let instance = state.instance.as_mut().unwrap();
            if !instance.remove_attempted && instance.message_token.is_some() {
                instance.remove_attempted = true;
                instance.core.clone().zip(instance.message_token)
            } else {
                None
            }
        };
        if let Some((core, token)) = remove {
            let result = native_call!(owner, MessageRemove, unsafe {
                core.remove_WebMessageReceived(token)
            });
            if result.is_ok() {
                owner.borrow_mut().instance.as_mut().unwrap().message_token = None;
            }
            cleanup_at!(owner, MessageRemove, result, true);
        }
        let stop = {
            let mut state = owner.borrow_mut();
            let instance = state.instance.as_mut().unwrap();
            if !instance.stop_attempted && instance.core.is_some() {
                instance.stop_attempted = true;
                instance.core.clone()
            } else {
                None
            }
        };
        if let Some(core) = stop {
            cleanup_at!(
                owner,
                Stop,
                native_call!(owner, Stop, unsafe { core.Stop() }),
                true
            );
        }
        let close = {
            let mut state = owner.borrow_mut();
            let instance = state.instance.as_mut().unwrap();
            if !instance.close_attempted && instance.controller.is_some() {
                instance.close_attempted = true;
                instance.controller.clone()
            } else {
                None
            }
        };
        if let Some(controller) = close {
            cleanup_at!(
                owner,
                ControllerClose,
                native_call!(owner, ControllerClose, unsafe { controller.Close() }),
                true
            );
        }
        let controls = {
            let mut state = owner.borrow_mut();
            let unmatched = state.unmatched;
            let instance = state.instance.as_mut().unwrap();
            if !unmatched
                && !instance.callbacks.affinity_failed.load(Ordering::SeqCst)
                && !instance.controls_disposed
                && !instance.cleanup_blocked
                && instance.message_token.is_none()
                && {
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    {
                        p1::registrations_removed(instance) && s2::registrations_removed(instance)
                    }
                    #[cfg(not(all(debug_assertions, feature = "code0-native-fixture")))]
                    {
                        true
                    }
                }
                && (instance.core.is_none() || instance.stop_attempted)
                && (instance.controller.is_none() || instance.close_attempted)
                && instance.callbacks.pending.load(Ordering::SeqCst) == 0
                && instance.callbacks.active.load(Ordering::SeqCst) == 0
            {
                // Intentionally do not require aggregate handlers0 here: the
                // registered exit handler/environment must survive control drop.
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                let policy = instance.policy.take();
                Some((
                    instance.core.take(),
                    instance.controller.take(),
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    policy,
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    s2::take_controls(instance),
                ))
            } else {
                None
            }
        };
        if let Some(controls) = controls {
            phase!(owner, ControlDrop, false);
            drop(controls);
            // Drop can reenter. Only after all actual control locals end may a
            // later stage consume a synchronously recorded exit observation.
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            let disposed = {
                let policy = p1::after_control_drop(owner);
                let refusal = s2::after_control_drop(owner);
                policy && refusal
            };
            #[cfg(not(all(debug_assertions, feature = "code0-native-fixture")))]
            let disposed = true;
            owner
                .borrow_mut()
                .instance
                .as_mut()
                .unwrap()
                .controls_disposed = disposed;
            phase!(owner, ControlDrop, true);
            if disposed {
                milestone!(owner, ControlsDisposed);
            }
        }
        let exit_registration = {
            let mut state = owner.borrow_mut();
            let unmatched = state.unmatched;
            let instance = state.instance.as_mut().unwrap();
            if !unmatched
                && !instance.callbacks.affinity_failed.load(Ordering::SeqCst)
                && instance.controls_disposed
                && matches!(instance.browser_exit, BrowserExitState::Observed)
                && !instance.cleanup_blocked
                && !instance.exit_remove_attempted
                && instance.callbacks.pending.load(Ordering::SeqCst) == 0
                && instance.callbacks.active.load(Ordering::SeqCst) == 0
            {
                instance.exit_remove_attempted = true;
                instance.environment5.clone().zip(instance.exit_token)
            } else {
                None
            }
        };
        if let Some((environment5, token)) = exit_registration {
            let result = native_call!(owner, ExitRemove, unsafe {
                environment5.remove_BrowserProcessExited(token)
            });
            if result.is_ok() {
                owner.borrow_mut().instance.as_mut().unwrap().exit_token = None;
                milestone!(owner, ExitRemoved);
            }
            cleanup_at!(owner, ExitRemove, result, true);
        }
        let environments = {
            let mut state = owner.borrow_mut();
            let unmatched = state.unmatched;
            let instance = state.instance.as_mut().unwrap();
            if !unmatched
                && !instance.callbacks.affinity_failed.load(Ordering::SeqCst)
                && instance.controls_disposed
                && instance.core.is_none()
                && instance.controller.is_none()
                && matches!(instance.browser_exit, BrowserExitState::Observed)
                && !instance.cleanup_blocked
                && instance.exit_remove_attempted
                && instance.exit_token.is_none()
                && instance.callbacks.pending.load(Ordering::SeqCst) == 0
                && instance.callbacks.active.load(Ordering::SeqCst) == 0
                && (instance.environment.is_some() || instance.environment5.is_some())
            {
                Some((instance.environment5.take(), instance.environment.take()))
            } else {
                None
            }
        };
        if let Some(environments) = environments {
            phase!(owner, EnvironmentDrop, false);
            // A removed handler may remain retained until its emitter drops.
            // Its real HandlerLifetime still gates final inventory_empty.
            drop(environments);
            phase!(owner, EnvironmentDrop, true);
            milestone!(owner, EnvironmentDropped);
        }
        let destroy = {
            let mut state = owner.borrow_mut();
            let instance = state.instance.as_mut().unwrap();
            if !instance.destroy_attempted && instance.callbacks.pending.load(Ordering::SeqCst) == 0
            {
                instance.destroy_attempted = true;
                instance.window.clone()
            } else {
                None
            }
        };
        if let Some(window) = destroy {
            let destroyed = observed_call!(owner, WindowDestroy, window.destroy());
            if destroyed.is_err() {
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                if let Some(diagnostic) = owner.borrow().diagnostics.as_ref() {
                    diagnostic.failure(DP::WindowDestroy, 0);
                }
                cleanup_at!(owner, WindowDestroy, Err(E_FAIL.into()), false);
            }
        }
        let notification = {
            let mut state = owner.borrow_mut();
            let unmatched = state.unmatched;
            let events = Rc::clone(&state.events);
            let instance = state.instance.as_mut().unwrap();
            if instance.failed && !instance.failure_sent {
                instance.failure_sent = true;
                Some((
                    events,
                    instance.request.clone(),
                    Code0WindowEvent::Failed(
                        if instance.cleanup_blocked
                            || matches!(instance.browser_exit, BrowserExitState::Unknown)
                        {
                            Code0SessionFailure::CleanupFailed
                        } else {
                            Code0SessionFailure::PolicyRejected
                        },
                    ),
                ))
            } else if !unmatched && !instance.released && inventory_empty(instance) {
                instance.released = true;
                Some((events, instance.request.clone(), Code0WindowEvent::Released))
            } else {
                None
            }
        };
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        publish_inventory(owner);
        if let Some((events, request, event)) = notification {
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            let released = matches!(event, Code0WindowEvent::Released);
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            if released {
                phase!(owner, ReleasedDispatch, false);
                p1::publish_final_fault(owner);
                p1::publish_final_coverage(owner);
                s2::publish_child_final(owner);
                s2::publish_dialog_final(owner);
                s2::publish_popup_final(owner);
            }
            events(request, event);
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            if released {
                phase!(owner, ReleasedDispatch, true);
            }
        }
    }

    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    pub(super) fn zero_creation(owner: &Rc<RefCell<Owner>>, instance_id: &str) -> bool {
        let state = owner.borrow();
        !state.unmatched
            && state.instance.as_ref().is_some_and(|instance| {
                instance.request.instance_id == instance_id
                    && !instance.build_started
                    && inventory_empty(instance)
            })
    }

    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    pub(super) fn begin_policy_install(
        owner: &Rc<RefCell<Owner>>,
        request: &Code0WindowRequest,
        case: super::fixture::PolicyFixtureCase,
    ) -> Result<(), Code0SessionFailure> {
        match case {
            super::fixture::PolicyFixtureCase::InstallPrematureLoadDenied => {
                Err(Code0SessionFailure::PolicyRejected)
            }
            super::fixture::PolicyFixtureCase::InstallAckBeforeFirstLoad
            | super::fixture::PolicyFixtureCase::InstallSettingInjectedFailure
            | super::fixture::PolicyFixtureCase::InstallAddInjectedFailure
            | super::fixture::PolicyFixtureCase::InstallFilterInjectedFailure
            | super::fixture::PolicyFixtureCase::InstallScriptInjectedFailure
            | super::fixture::PolicyFixtureCase::InstallCdpInjectedFailure
            | super::fixture::PolicyFixtureCase::InstallScriptStaleDisposition
            | super::fixture::PolicyFixtureCase::InstallCdpStaleDisposition
            | super::fixture::PolicyFixtureCase::InstallRealmsNoPassthrough => {
                p1::begin(owner, request, case)
            }
        }
    }

    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    pub(super) fn probe_premature_policy_load(
        owner: &Rc<RefCell<Owner>>,
        request: &Code0WindowRequest,
    ) -> Result<(), Code0SessionFailure> {
        p1::premature_load(owner, request)
    }

    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    pub(super) fn load_policy_document(
        owner: &Rc<RefCell<Owner>>,
        request: &Code0WindowRequest,
    ) -> Result<(), Code0SessionFailure> {
        p1::load(owner, request)
    }

    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    type NativeRefusalRegistry = s2::Registry;

    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    pub(super) fn require_native_refusal_install(
        owner: &Rc<RefCell<Owner>>,
        request: &Code0WindowRequest,
    ) -> Result<super::fixture::NativeRefusalInstallReceipt, Code0SessionFailure> {
        if !on_ui(owner) {
            return Err(Code0SessionFailure::PolicyRejected);
        }
        let state = owner.borrow();
        let i = state
            .instance
            .as_ref()
            .ok_or(Code0SessionFailure::PolicyRejected)?;
        if state.unmatched
            || &i.request != request
            || i.closing
            || i.failed
            || i.cleanup_blocked
            || i.released
            || i.destroyed
            || i.inline_loaded
            || i.policy.is_some()
            || i.policy_mailbox.is_some()
            || i.core.is_none()
            || i.controller.is_none()
            || i.message_token.is_none()
            || i.environment.is_none()
            || i.environment5.is_none()
            || i.exit_token.is_none()
            || !matches!(i.browser_exit, BrowserExitState::Unarmed)
            || i.callbacks.affinity_failed.load(Ordering::SeqCst)
        {
            return Err(Code0SessionFailure::PolicyRejected);
        }
        // Read only actual same-emitter token inventory; never install or load.
        let installed_mask = i.refusal_registry.installed_mask();
        Ok(super::fixture::NativeRefusalInstallReceipt {
            request: i.request.clone(),
            installed_mask,
            not_installed: super::fixture::NativeRefusalHook::ALL
                .into_iter()
                .filter(|hook| installed_mask & hook.bit() == 0)
                .collect(),
            load_rejected: !i.refusal_registry.complete(),
            installation: Arc::clone(&i.refusal_registry.unknown),
        })
    }

    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    pub(super) fn install_native_refusal_handlers(
        owner: &Rc<RefCell<Owner>>,
        request: &Code0WindowRequest,
    ) -> Result<(), Code0SessionFailure> {
        s2::begin(owner, request)
    }

    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    pub(super) fn navigate_top_owned_file(
        owner: &Rc<RefCell<Owner>>,
        request: &Code0WindowRequest,
        owned_root: &Path,
        deadline: super::fixture::NativeRefusalDeadline,
    ) -> Result<super::fixture::NativeTopSubmissionReceipt, Code0SessionFailure> {
        s2::navigate_top_owned_file(owner, request, owned_root, deadline)
    }

    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    pub(super) fn submit_child_initial_data(
        owner: &Rc<RefCell<Owner>>,
        request: &Code0WindowRequest,
        deadline: super::fixture::NativeRefusalDeadline,
    ) -> Result<super::fixture::NativeChildSubmissionReceipt, Code0SessionFailure> {
        s2::submit_child_initial_data(owner, request, deadline)
    }

    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    pub(super) fn s2_dialog_owned_empty_html(
        owner: &Rc<RefCell<Owner>>,
        request: &Code0WindowRequest,
        owned_root: &Path,
        deadline: super::fixture::NativeDialogDeadline,
    ) -> Result<(), Code0SessionFailure> {
        s2::navigate_dialog_owned_empty_html(owner, request, owned_root, deadline)
    }

    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    pub(super) fn submit_popup_initial_blank(
        owner: &Rc<RefCell<Owner>>,
        request: &Code0WindowRequest,
        deadline: super::fixture::NativeDialogDeadline,
    ) -> Result<(), Code0SessionFailure> {
        s2::submit_popup_initial_blank(owner, request, deadline)
    }

    /// Debug-only refusal registration and finite N1 audit; never production authority.
    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    mod s2 {
        use super::super::fixture::NativeRefusalHook as Hook;
        use super::*;
        use webview2_com::{
            BasicAuthenticationRequestedEventHandler, ClientCertificateRequestedEventHandler,
            DownloadStartingEventHandler, LaunchingExternalUriSchemeEventHandler,
            NavigationCompletedEventHandler, NavigationStartingEventHandler,
            NewWindowRequestedEventHandler, PermissionRequestedEventHandler,
            ScriptDialogOpeningEventHandler, ServerCertificateErrorDetectedEventHandler,
        };
        use windows::core::BOOL;

        use std::sync::Mutex;
        use std::time::Instant;

        const GENERATION: u64 = 1;
        use super::super::fixture::{
            LegacyChildClose, LegacyCompletionInput, LegacyRefusalEvent, NativeDialogDeadline,
            NativeDialogFacts, NativeDialogReceipt, NativeDialogStage, NativePopupFacts,
            NativePopupReceipt, NativePopupStage, NativeRefusalDeadline,
            legacy_completion_disposition_at, popup_result_units,
        };
        const DIALOG_LEAF: &str = "code0-s2-dialog-empty.html";
        const DIALOG_MESSAGE: &str = "code0-native-dialog-refusal-v1";
        const DIALOG_SCRIPT: &str = "(()=>{if(top!==self||location.protocol!=='file:'||!location.pathname.endsWith('/code0-s2-dialog-empty.html')||!document.documentElement||document.querySelector('iframe,frame')!==null){return 'code0-dialog-precondition-v1';}alert('code0-native-dialog-refusal-v1');return 'code0-dialog-returned-v1';})()";
        const DIALOG_RESULT: &str = "\"code0-dialog-returned-v1\"";
        const CHILD_TARGET: &str = "data:text/html,";
        const CHILD_SCRIPT: &str = "(()=>{if(top!==self||location.href!=='about:blank'||!document.documentElement||document.querySelector('iframe')!==null){throw new Error('code0-child-precondition');}const f=document.createElement('iframe');f.src='data:text/html,';document.documentElement.appendChild(f);return 'code0-child-submitted-v1';})()";
        const CHILD_RESULT: &str = "\"code0-child-submitted-v1\"";
        const POPUP_SCRIPT: &str = "(()=>{if(top!==self||location.href!=='about:blank'||!document.documentElement||document.documentElement.tagName!=='HTML'||document.querySelector('iframe,frame')!==null){return 'code0-popup-precondition-v1';}window.open('about:blank','_blank');return 'code0-popup-submitted-v1';})()";

        enum Emitter {
            Core(Rc<ICoreWebView2>),
            External(Rc<ICoreWebView2_18>),
            Download(Rc<ICoreWebView2_4>),
            Authentication(Rc<ICoreWebView2_10>),
            Certificate(Rc<ICoreWebView2_5>),
            Tls(Rc<ICoreWebView2_14>),
        }
        struct Registration {
            hook: Hook,
            emitter: Rc<Emitter>,
            token: Option<i64>,
            remove_attempted: bool,
        }
        pub(super) struct Registry {
            generation: u64,
            started: bool,
            settings_acknowledged: bool,
            completed: bool,
            pub(super) unknown: Arc<AtomicBool>,
            core: Option<Rc<ICoreWebView2>>,
            settings: Option<Rc<ICoreWebView2Settings>>,
            entries: Vec<Registration>,
            // Finite N1 reservation only; never a production load permission.
            top_target: Option<String>,
            legacy: Option<LegacyState>,
            child: Option<ChildState>,
            dialog: Option<DialogState>,
            dialog_completed: Option<DialogRegistration>,
            popup: Option<PopupState>,
        }
        impl Registry {
            pub(super) fn not_installed() -> Self {
                Self {
                    generation: 0,
                    started: false,
                    settings_acknowledged: false,
                    completed: false,
                    unknown: Arc::new(AtomicBool::new(false)),
                    core: None,
                    settings: None,
                    entries: Vec::new(),
                    top_target: None,
                    legacy: None,
                    child: None,
                    dialog: None,
                    dialog_completed: None,
                    popup: None,
                }
            }
            pub(super) fn installed_mask(&self) -> u16 {
                self.entries.iter().fold(0, |mask, entry| {
                    mask | if entry.token.is_some() {
                        entry.hook.bit()
                    } else {
                        0
                    }
                })
            }
            pub(super) fn complete(&self) -> bool {
                self.completed
                    && self.settings_acknowledged
                    && self.installed_mask() == 0x03ff
                    && !self.unknown.load(Ordering::SeqCst)
            }
            pub(super) fn unknown(&self) -> bool {
                self.unknown.load(Ordering::SeqCst)
            }
            pub(super) fn references_empty(&self) -> bool {
                self.core.is_none()
                    && self.settings.is_none()
                    && self.entries.is_empty()
                    && !self.unknown.load(Ordering::SeqCst)
                    && self.child.as_ref().is_none_or(ChildState::quiescent)
                    && self.dialog_completed.is_none()
                    && self.dialog.as_ref().is_none_or(DialogState::quiescent)
                    && self.popup.as_ref().is_none_or(PopupState::quiescent)
            }
        }
        pub(super) struct Controls {
            _core: Option<Rc<ICoreWebView2>>,
            _settings: Option<Rc<ICoreWebView2Settings>>,
            _entries: Vec<Registration>,
            _dialog_completed: Option<DialogRegistration>,
        }

        #[derive(Clone, Copy, PartialEq, Eq)]
        enum LegacyKind {
            Top,
            Child,
        }
        struct LegacyState {
            kind: LegacyKind,
            deadline: Rc<NativeRefusalDeadline>,
            core: Weak<ICoreWebView2>,
            top_closing: Option<Instant>,
            child_closing: LegacyChildClose,
        }
        fn bind_legacy(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            deadline: NativeRefusalDeadline,
            kind: LegacyKind,
        ) -> Result<Arc<AtomicBool>, Code0SessionFailure> {
            let denied = || Code0SessionFailure::PolicyRejected;
            super::require_native_refusal_install(owner, request)?;
            let deadline = Rc::new(deadline);
            let installation = {
                let mut state = owner.borrow_mut();
                if state.unmatched {
                    return Err(denied());
                }
                let i = matching(&mut state, request).ok_or_else(denied)?;
                if !i.refusal_registry.complete()
                    || i.refusal_registry.legacy.is_some()
                    || i.refusal_registry.top_target.is_some()
                    || i.refusal_registry.child.is_some()
                    || i.refusal_registry.dialog.is_some()
                    || i.refusal_registry.popup.is_some()
                {
                    return Err(denied());
                }
                let core = i.refusal_registry.core.as_ref().ok_or_else(denied)?;
                i.refusal_registry.legacy = Some(LegacyState {
                    kind,
                    deadline,
                    core: Rc::downgrade(core),
                    top_closing: None,
                    child_closing: LegacyChildClose::new(),
                });
                Arc::clone(&i.refusal_registry.unknown)
            };
            if legacy_authorized(owner, request, &installation, true).is_none() {
                return Err(denied());
            }
            Ok(installation)
        }
        fn legacy_deadline(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            installation: &Arc<AtomicBool>,
        ) -> Option<Rc<NativeRefusalDeadline>> {
            let state = owner.borrow();
            let i = state.instance.as_ref()?;
            if &i.request != request
                || i.refusal_registry.generation != GENERATION
                || !Arc::ptr_eq(&i.refusal_registry.unknown, installation)
            {
                return None;
            }
            let legacy = i.refusal_registry.legacy.as_ref()?;
            let core = i.refusal_registry.core.as_ref()?;
            if !Weak::ptr_eq(&legacy.core, &Rc::downgrade(core)) {
                return None;
            }
            Some(Rc::clone(&legacy.deadline))
        }
        fn mark_top_close(i: &mut Instance) -> Option<Instant> {
            let legacy = i.refusal_registry.legacy.as_mut()?;
            if legacy.kind != LegacyKind::Top || legacy.top_closing.is_some() {
                return None;
            }
            let at = Instant::now();
            legacy.top_closing = Some(at);
            Some(at)
        }
        fn fail_legacy(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            installation: &Arc<AtomicBool>,
        ) {
            let (top, child) = {
                let mut state = owner.borrow_mut();
                let Some(i) = state.instance.as_mut().filter(|i| {
                    &i.request == request
                        && i.refusal_registry.generation == GENERATION
                        && Arc::ptr_eq(&i.refusal_registry.unknown, installation)
                        && i.refusal_registry.legacy.is_some()
                }) else {
                    return;
                };
                i.failed = true;
                i.closing = true;
                (
                    mark_top_close(i).map(|at| (i.observer.clone(), at)),
                    mark_child_close(i).map(|at| (i.observer.clone(), at)),
                )
            };
            if let Some((Some(observer), at)) = top {
                observer(NativeSmokeObservation::TopClosing { at });
            }
            if let Some((Some(observer), at)) = child {
                observer(NativeSmokeObservation::ChildClosing { at });
            }
        }
        fn legacy_authorized(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            installation: &Arc<AtomicBool>,
            require_live: bool,
        ) -> Option<Rc<NativeRefusalDeadline>> {
            let deadline = legacy_deadline(owner, request, installation)?;
            let identity = {
                let state = owner.borrow();
                !state.unmatched
                    && state.instance.as_ref().is_some_and(|i| {
                        &i.request == request
                            && i.refusal_registry.generation == GENERATION
                            && Arc::ptr_eq(&i.refusal_registry.unknown, installation)
                            && !i.released
                            && !i.cleanup_blocked
                            && !i.callbacks.affinity_failed.load(Ordering::SeqCst)
                            && !i.refusal_registry.unknown()
                    })
            };
            if deadline
                .run_if_authorized_at(
                    Instant::now(),
                    identity && (!require_live || live(owner, request)),
                    || (),
                )
                .is_none()
            {
                fail_legacy(owner, request, installation);
                return None;
            }
            Some(deadline)
        }
        fn legacy_step<T>(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            installation: &Arc<AtomicBool>,
            action: impl FnOnce() -> T,
        ) -> Option<T> {
            let deadline = legacy_authorized(owner, request, installation, true)?;
            let value = deadline.run_if_authorized_at(Instant::now(), live(owner, request), action);
            if value.is_none() {
                fail_legacy(owner, request, installation);
            }
            value
        }
        fn legacy_setup_step<T, E>(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            installation: &Arc<AtomicBool>,
            action: impl FnOnce() -> Result<T, E>,
        ) -> Result<T, Code0SessionFailure> {
            let denied = || Code0SessionFailure::PolicyRejected;
            let result = legacy_step(owner, request, installation, action).ok_or_else(denied)?;
            let result = result.map_err(|error| {
                drop(error);
                denied()
            });
            if legacy_authorized(owner, request, installation, true).is_none() {
                return Err(denied());
            }
            result
        }

        struct ChildMailbox {
            disposition: Mutex<Option<ChildDisposition>>,
            invoked: AtomicBool,
            invalid: AtomicBool,
            // This is NOT Callbacks::pending: creation safety gates stay exact.
            // Only consuming a real matching completion discharges this ticket.
            obligation: AtomicUsize,
            handlers: AtomicUsize,
            closing_since: Mutex<Option<Instant>>,
        }
        struct ChildState {
            mailbox: Arc<ChildMailbox>,
            completion_seen: bool,
        }
        impl ChildState {
            fn quiescent(&self) -> bool {
                self.completion_seen
                    && self.mailbox.obligation.load(Ordering::SeqCst) == 0
                    && self.mailbox.handlers.load(Ordering::SeqCst) == 0
                    && !self.mailbox.invalid.load(Ordering::SeqCst)
                    && self.mailbox.disposition.lock().unwrap().is_none()
            }
        }
        struct ChildDisposition {
            request: Code0WindowRequest,
            generation: u64,
            installation: Arc<AtomicBool>,
            received_at: Instant,
            hresult_succeeded: bool,
            bounded_result_valid: bool,
            fixed_result_matched: bool,
        }
        struct ChildHandlerLifetime {
            mailbox: std::sync::Weak<ChildMailbox>,
            _aggregate: HandlerLifetime,
        }
        impl Drop for ChildHandlerLifetime {
            fn drop(&mut self) {
                if let Some(mailbox) = self.mailbox.upgrade() {
                    mailbox.handlers.fetch_sub(1, Ordering::SeqCst);
                }
                // A never-called handler never discharges completion obligation.
            }
        }
        #[windows::core::implement(ICoreWebView2ExecuteScriptCompletedHandler, Agile = false)]
        struct ChildCompletion {
            mailbox: std::sync::Weak<ChildMailbox>,
            callbacks: Arc<Callbacks>,
            installation: Arc<AtomicBool>,
            request: Code0WindowRequest,
            generation: u64,
            ui: ThreadId,
            _lifetime: ChildHandlerLifetime,
        }
        impl ICoreWebView2ExecuteScriptCompletedHandler_Impl for ChildCompletion_Impl {
            fn Invoke(
                &self,
                errorcode: windows::core::HRESULT,
                result: &PCWSTR,
            ) -> windows::core::Result<()> {
                let _active = Active::enter(&self.callbacks);
                // No UI Weak/Rc/RefCell, raw read or allocation before this gate.
                if !callback_on_ui(self.ui, &self.callbacks) {
                    self.installation.store(true, Ordering::SeqCst);
                    if let Some(mailbox) = self.mailbox.upgrade() {
                        mailbox.invalid.store(true, Ordering::SeqCst);
                    }
                    return Ok(());
                }
                let Some(mailbox) = self.mailbox.upgrade() else {
                    self.installation.store(true, Ordering::SeqCst);
                    return Ok(());
                };
                if mailbox.invoked.swap(true, Ordering::SeqCst) {
                    mailbox.invalid.store(true, Ordering::SeqCst);
                    self.installation.store(true, Ordering::SeqCst);
                    return Ok(());
                }
                let received_at = Instant::now();
                let (bounded_result_valid, fixed_result_matched) = child_result(*result);
                let disposition = ChildDisposition {
                    request: self.request.clone(),
                    generation: self.generation,
                    installation: Arc::clone(&self.installation),
                    received_at,
                    hresult_succeeded: errorcode.is_ok(),
                    bounded_result_valid,
                    fixed_result_matched,
                };
                let mut slot = mailbox.disposition.lock().unwrap();
                if slot.is_some() {
                    mailbox.invalid.store(true, Ordering::SeqCst);
                    self.installation.store(true, Ordering::SeqCst);
                } else {
                    *slot = Some(disposition);
                }
                // Transport success carries no permission or target evidence.
                Ok(())
            }
        }
        fn child_result(raw: PCWSTR) -> (bool, bool) {
            if raw.is_null() {
                return (false, false);
            }
            let mut copied = [0u16; 64];
            let mut length = 0;
            while length < copied.len() {
                // Borrowed completion PCWSTR: at most 64 units read, never freed.
                let unit = unsafe { *raw.0.add(length) };
                if unit == 0 {
                    let units = &copied[..length];
                    let valid = char::decode_utf16(units.iter().copied()).all(|v| v.is_ok());
                    return (
                        valid,
                        valid && units.iter().copied().eq(CHILD_RESULT.encode_utf16()),
                    );
                }
                copied[length] = unit;
                length += 1;
            }
            (false, false)
        }
        pub(super) fn mark_child_close(i: &mut Instance) -> Option<Instant> {
            if let Some(legacy) = i.refusal_registry.legacy.as_mut()
                && legacy.kind == LegacyKind::Child
            {
                let now = Instant::now();
                return if let Some(child) = i.refusal_registry.child.as_ref() {
                    let mut closing = child.mailbox.closing_since.lock().unwrap();
                    legacy.child_closing.mark_at(now, Some(&mut closing))
                } else {
                    legacy.child_closing.mark_at(now, None)
                };
            }
            let child = i.refusal_registry.child.as_ref()?;
            let mut closing = child.mailbox.closing_since.lock().unwrap();
            if closing.is_some() {
                return None;
            }
            let at = Instant::now();
            *closing = Some(at);
            Some(at)
        }
        pub(super) fn advance_child(owner: &Rc<RefCell<Owner>>) {
            let binding = owner.borrow().instance.as_ref().and_then(|i| {
                i.refusal_registry.child.as_ref().map(|c| {
                    (
                        Arc::clone(&c.mailbox),
                        i.request.clone(),
                        Arc::clone(&i.refusal_registry.unknown),
                    )
                })
            });
            let Some((mailbox, request, installation)) = binding else {
                return;
            };
            let disposition = mailbox.disposition.lock().unwrap().take();
            let mut observation = None;
            let mut failed = None;
            {
                let mut state = owner.borrow_mut();
                let unmatched = state.unmatched;
                let Some(i) = state.instance.as_mut().filter(|i| {
                    i.request == request
                        && i.refusal_registry.generation == GENERATION
                        && Arc::ptr_eq(&i.refusal_registry.unknown, &installation)
                        && i.refusal_registry
                            .child
                            .as_ref()
                            .is_some_and(|c| Arc::ptr_eq(&c.mailbox, &mailbox))
                }) else {
                    // Stale work marks only its captured installation/mailbox.
                    installation.store(true, Ordering::SeqCst);
                    mailbox.invalid.store(true, Ordering::SeqCst);
                    return;
                };
                if mailbox.invalid.load(Ordering::SeqCst) {
                    i.refusal_registry.unknown.store(true, Ordering::SeqCst);
                }
                if let Some(d) = disposition {
                    let Some(child) = i.refusal_registry.child.as_mut() else {
                        return;
                    };
                    let same = !unmatched
                        && i.request == d.request
                        && i.refusal_registry.generation == d.generation
                        && Arc::ptr_eq(&i.refusal_registry.unknown, &d.installation)
                        && Arc::ptr_eq(&child.mailbox, &mailbox)
                        && !child.completion_seen
                        && mailbox.obligation.load(Ordering::SeqCst) == 1;
                    let closing = *mailbox.closing_since.lock().unwrap();
                    let original = i.refusal_registry.legacy.as_ref().map(|l| {
                        legacy_completion_disposition_at(
                            &l.deadline,
                            d.received_at,
                            Instant::now(),
                            closing,
                            LegacyCompletionInput {
                                identity_valid: same
                                    && !i.released
                                    && !i.callbacks.affinity_failed.load(Ordering::SeqCst)
                                    && !mailbox.invalid.load(Ordering::SeqCst)
                                    && !i.refusal_registry.unknown.load(Ordering::SeqCst),
                                hresult_succeeded: d.hresult_succeeded,
                                bounded_result_valid: d.bounded_result_valid,
                                fixed_result_matched: d.fixed_result_matched,
                            },
                        )
                    });
                    let within_deadline = original.as_ref().is_some_and(|v| v.timely);
                    let disposable = original.as_ref().is_some_and(|v| v.disposable);
                    let successful = original.as_ref().is_some_and(|v| v.successful);
                    let facts = super::super::fixture::NativeChildCompletionFacts {
                        identity_matched: same,
                        hresult_succeeded: d.hresult_succeeded,
                        bounded_result_valid: d.bounded_result_valid,
                        fixed_result_matched: d.fixed_result_matched,
                        before_release: !i.released,
                        within_deadline,
                    };
                    // A real matching late/error callback may dispose under the
                    // original close10. Its success evidence still requires t0+45.
                    if disposable {
                        child.completion_seen = true;
                        mailbox.obligation.store(0, Ordering::SeqCst);
                    } else {
                        i.refusal_registry.unknown.store(true, Ordering::SeqCst);
                    }
                    if !successful {
                        failed = Some((d.request.clone(), Arc::clone(&d.installation)));
                    }
                    observation = Some((
                        i.observer.clone(),
                        super::super::fixture::NativeChildCompletionReceipt {
                            request: d.request,
                            installation: d.installation,
                            facts,
                        },
                    ));
                }
            }
            if let Some((request, installation)) = failed {
                fail_legacy(owner, &request, &installation);
            }
            latch_unknown(owner);
            if let Some((Some(observer), receipt)) = observation {
                observer(NativeSmokeObservation::ChildCompletion(receipt));
            }
        }

        pub(super) fn publish_child_final(owner: &Rc<RefCell<Owner>>) {
            let observation = {
                let state = owner.borrow();
                state.instance.as_ref().and_then(|i| {
                    i.refusal_registry.child.as_ref().map(|c| {
                        (
                            i.observer.clone(),
                            super::super::fixture::NativeChildFinalReceipt {
                                request: i.request.clone(),
                                installation: Arc::clone(&i.refusal_registry.unknown),
                                obligation_drained: c.completion_seen
                                    && c.mailbox.obligation.load(Ordering::SeqCst) == 0,
                                handlers_drained: c.mailbox.handlers.load(Ordering::SeqCst) == 0
                                    && i.callbacks.handlers.load(Ordering::SeqCst) == 0,
                            },
                        )
                    })
                })
            };
            if let Some((Some(observer), receipt)) = observation {
                observer(NativeSmokeObservation::ChildFinal(receipt));
            }
        }
        pub(super) fn submit_child_initial_data(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            deadline: NativeRefusalDeadline,
        ) -> Result<super::super::fixture::NativeChildSubmissionReceipt, Code0SessionFailure>
        {
            let denied = || Code0SessionFailure::PolicyRejected;
            let installation = bind_legacy(owner, request, deadline, LegacyKind::Child)?;
            let (callbacks, ui, core) = {
                let state = owner.borrow();
                let i = state.instance.as_ref().ok_or_else(denied)?;
                (
                    Arc::clone(&i.callbacks),
                    state.ui_thread,
                    Rc::clone(i.refusal_registry.core.as_ref().ok_or_else(denied)?),
                )
            };
            let active = Active::enter(&callbacks);
            let source = legacy_step(owner, request, &installation, || {
                allocated_string(64, |raw| unsafe { core.Source(raw) })
            });
            let source_matched = source
                .as_ref()
                .is_some_and(|r| r.as_ref().is_ok_and(|s| s == "about:blank"));
            drop(source);
            let source_timely = legacy_authorized(owner, request, &installation, true).is_some();
            if !source_matched || !source_timely {
                drop(core);
                drop(active);
                let _ = legacy_authorized(owner, request, &installation, false);
                return Err(denied());
            }
            let mailbox = Arc::new(ChildMailbox {
                disposition: Mutex::new(None),
                invoked: AtomicBool::new(false),
                invalid: AtomicBool::new(false),
                obligation: AtomicUsize::new(1),
                handlers: AtomicUsize::new(1),
                closing_since: Mutex::new(None),
            });
            let handler: ICoreWebView2ExecuteScriptCompletedHandler = ChildCompletion {
                mailbox: Arc::downgrade(&mailbox),
                callbacks: Arc::clone(&callbacks),
                installation: Arc::clone(&installation),
                request: request.clone(),
                generation: GENERATION,
                ui,
                _lifetime: ChildHandlerLifetime {
                    mailbox: Arc::downgrade(&mailbox),
                    _aggregate: HandlerLifetime::new(&callbacks),
                },
            }
            .into();
            let wide: Vec<u16> = CHILD_SCRIPT
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            // Prepared handler ownership is distinct from a submitted invocation.
            // Commit exactly at the last authorized call, before synchronous reentry.
            let result = legacy_step(owner, request, &installation, || {
                {
                    let mut state = owner.borrow_mut();
                    let i = matching(&mut state, request).unwrap();
                    i.refusal_registry.child = Some(ChildState {
                        mailbox: Arc::clone(&mailbox),
                        completion_seen: false,
                    });
                }
                unsafe { core.ExecuteScript(PCWSTR(wide.as_ptr()), &handler) }
            });
            let native_submit_ok = result.as_ref().is_some_and(|r| r.is_ok());
            let submitted = result.is_some();
            drop(result);
            let _ = legacy_authorized(owner, request, &installation, false);
            drop(handler);
            let _ = legacy_authorized(owner, request, &installation, false);
            drop(core);
            let _ = legacy_authorized(owner, request, &installation, false);
            drop(active);
            let original_timely = legacy_authorized(owner, request, &installation, false).is_some();
            if !submitted {
                return Err(denied());
            }
            Ok(super::super::fixture::NativeChildSubmissionReceipt {
                request: request.clone(),
                installation,
                native_submit_ok,
                initial_source_matched: source_matched,
                original_timely,
            })
        }

        // N3 owns a separate completion obligation. It never changes creation
        // Pending or prevents Stop/Close while ExecuteScript is outstanding.
        struct DialogMailbox {
            disposition: Mutex<Option<DialogDisposition>>,
            invoked: AtomicBool,
            invalid: AtomicBool,
            obligation: AtomicUsize,
            handlers: AtomicUsize,
            closing_since: Mutex<Option<Instant>>,
        }
        struct DialogDisposition {
            request: Code0WindowRequest,
            generation: u64,
            installation: Arc<AtomicBool>,
            received_at: Instant,
            hresult_succeeded: bool,
            bounded_result_valid: bool,
            fixed_result_matched: bool,
        }
        struct DialogState {
            target: String,
            deadline: Rc<NativeDialogDeadline>,
            grant_pending: bool,
            start_id: Option<u64>,
            document_published: bool,
            mailbox: Option<Arc<DialogMailbox>>,
            completion_seen: bool,
            closing_since: Option<Instant>,
            facts: NativeDialogFacts,
        }
        impl DialogState {
            fn quiescent(&self) -> bool {
                self.mailbox.as_ref().is_none_or(|m| {
                    self.completion_seen
                        && m.obligation.load(Ordering::SeqCst) == 0
                        && m.handlers.load(Ordering::SeqCst) == 0
                        && !m.invalid.load(Ordering::SeqCst)
                        && m.disposition.lock().unwrap().is_none()
                })
            }
        }
        struct DialogRegistration {
            core: Rc<ICoreWebView2>,
            token: Option<i64>,
            remove_attempted: bool,
        }
        struct DialogHandlerLifetime {
            mailbox: std::sync::Weak<DialogMailbox>,
            _aggregate: HandlerLifetime,
        }
        impl Drop for DialogHandlerLifetime {
            fn drop(&mut self) {
                if let Some(m) = self.mailbox.upgrade() {
                    m.handlers.fetch_sub(1, Ordering::SeqCst);
                }
                // Handler disposal cannot discharge an uninvoked operation.
            }
        }
        #[windows::core::implement(ICoreWebView2ExecuteScriptCompletedHandler, Agile = false)]
        struct DialogCompletion {
            mailbox: std::sync::Weak<DialogMailbox>,
            callbacks: Arc<Callbacks>,
            installation: Arc<AtomicBool>,
            request: Code0WindowRequest,
            generation: u64,
            ui: ThreadId,
            _lifetime: DialogHandlerLifetime,
        }
        impl ICoreWebView2ExecuteScriptCompletedHandler_Impl for DialogCompletion_Impl {
            fn Invoke(
                &self,
                errorcode: windows::core::HRESULT,
                result: &PCWSTR,
            ) -> windows::core::Result<()> {
                let _active = Active::enter(&self.callbacks);
                // No UI-local Weak/Rc/RefCell, raw read or allocation before STA.
                if !callback_on_ui(self.ui, &self.callbacks) {
                    self.installation.store(true, Ordering::SeqCst);
                    if let Some(m) = self.mailbox.upgrade() {
                        m.invalid.store(true, Ordering::SeqCst);
                    }
                    return Ok(());
                }
                let Some(m) = self.mailbox.upgrade() else {
                    self.installation.store(true, Ordering::SeqCst);
                    return Ok(());
                };
                if m.invoked.swap(true, Ordering::SeqCst) {
                    m.invalid.store(true, Ordering::SeqCst);
                    self.installation.store(true, Ordering::SeqCst);
                    return Ok(());
                }
                let received_at = Instant::now();
                // An unsuccessful HRESULT is disposal data only. Its nullable
                // result is not interpreted and can never satisfy strict S.
                let (bounded_result_valid, fixed_result_matched) = if errorcode.is_ok() {
                    dialog_result(*result)
                } else {
                    (false, false)
                };
                let d = DialogDisposition {
                    request: self.request.clone(),
                    generation: self.generation,
                    installation: Arc::clone(&self.installation),
                    received_at,
                    hresult_succeeded: errorcode.is_ok(),
                    bounded_result_valid,
                    fixed_result_matched,
                };
                let mut slot = m.disposition.lock().unwrap();
                if slot.is_some() {
                    m.invalid.store(true, Ordering::SeqCst);
                    self.installation.store(true, Ordering::SeqCst);
                } else {
                    *slot = Some(d);
                }
                Ok(())
            }
        }
        fn dialog_result(raw: PCWSTR) -> (bool, bool) {
            if raw.is_null() {
                return (false, false);
            }
            let mut copied = [0u16; 64];
            let mut length = 0;
            while length < copied.len() {
                // Borrowed completion buffer, bounded before conversion; never free.
                let unit = unsafe { *raw.0.add(length) };
                if unit == 0 {
                    let units = &copied[..length];
                    let valid = char::decode_utf16(units.iter().copied()).all(|v| v.is_ok());
                    return (
                        valid,
                        valid && units.iter().copied().eq(DIALOG_RESULT.encode_utf16()),
                    );
                }
                copied[length] = unit;
                length += 1;
            }
            (false, false)
        }
        fn emit_dialog(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            installation: &Arc<AtomicBool>,
            stage: NativeDialogStage,
        ) {
            let observation = {
                let state = owner.borrow();
                state
                    .instance
                    .as_ref()
                    .filter(|i| {
                        &i.request == request
                            && i.refusal_registry.generation == GENERATION
                            && Arc::ptr_eq(&i.refusal_registry.unknown, installation)
                    })
                    .and_then(|i| {
                        i.refusal_registry.dialog.as_ref().map(|d| {
                            (
                                i.observer.clone(),
                                NativeDialogReceipt {
                                    request: i.request.clone(),
                                    installation: Arc::clone(&i.refusal_registry.unknown),
                                    stage,
                                    facts: d.facts,
                                },
                            )
                        })
                    })
            };
            if let Some((Some(observer), receipt)) = observation {
                observer(NativeSmokeObservation::Dialog(receipt));
            }
        }
        pub(super) fn mark_dialog_close(i: &mut Instance) -> Option<Instant> {
            let d = i.refusal_registry.dialog.as_mut()?;
            if d.closing_since.is_some() {
                return None;
            }
            let at = Instant::now();
            d.closing_since = Some(at);
            if let Some(m) = &d.mailbox {
                *m.closing_since.lock().unwrap() = Some(at);
            }
            Some(at)
        }
        fn fail_dialog(owner: &Rc<RefCell<Owner>>, unknown: bool) {
            let notification = {
                let mut state = owner.borrow_mut();
                let Some(i) = state.instance.as_mut() else {
                    return;
                };
                if i.refusal_registry.dialog.is_none() {
                    return;
                }
                if unknown {
                    i.refusal_registry.unknown.store(true, Ordering::SeqCst);
                    i.cleanup_blocked = true;
                }
                i.failed = true;
                i.closing = true;
                mark_dialog_close(i).map(|at| (i.observer.clone(), at))
            };
            if let Some((Some(observer), at)) = notification {
                observer(NativeSmokeObservation::DialogClosing { at });
            }
        }
        fn dialog_deadline(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            installation: &Arc<AtomicBool>,
        ) -> Option<Rc<NativeDialogDeadline>> {
            let state = owner.borrow();
            state
                .instance
                .as_ref()
                .filter(|i| {
                    &i.request == request
                        && i.refusal_registry.generation == GENERATION
                        && Arc::ptr_eq(&i.refusal_registry.unknown, installation)
                })
                .and_then(|i| {
                    i.refusal_registry
                        .dialog
                        .as_ref()
                        .map(|d| Rc::clone(&d.deadline))
                })
        }
        fn dialog_authorized(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            installation: &Arc<AtomicBool>,
            require_live: bool,
        ) -> Option<Rc<NativeDialogDeadline>> {
            let deadline = dialog_deadline(owner, request, installation)?;
            let identity = {
                let state = owner.borrow();
                !state.unmatched
                    && state.instance.as_ref().is_some_and(|i| {
                        &i.request == request
                            && i.refusal_registry.generation == GENERATION
                            && Arc::ptr_eq(&i.refusal_registry.unknown, installation)
                            && !i.released
                            && !i.cleanup_blocked
                            && !i.callbacks.affinity_failed.load(Ordering::SeqCst)
                            && !i.refusal_registry.unknown()
                    })
            };
            let allowed = deadline
                .run_if_authorized_at(
                    Instant::now(),
                    identity && (!require_live || live(owner, request)),
                    || (),
                )
                .is_some();
            if !allowed {
                fail_dialog(owner, false);
                return None;
            }
            Some(deadline)
        }
        fn dialog_step<T>(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            installation: &Arc<AtomicBool>,
            action: impl FnOnce() -> T,
        ) -> Option<T> {
            let deadline = dialog_authorized(owner, request, installation, true)?;
            let result =
                deadline.run_if_authorized_at(Instant::now(), live(owner, request), action);
            if result.is_none() {
                fail_dialog(owner, false);
            }
            result
        }
        impl Context {
            fn dialog_selected(&self) -> bool {
                self.owner.upgrade().is_some_and(|owner| {
                    owner.borrow().instance.as_ref().is_some_and(|i| {
                        self.matches_instance(i) && i.refusal_registry.dialog.is_some()
                    })
                })
            }
            fn dialog_live(&self, owner: &Rc<RefCell<Owner>>) -> bool {
                let original_live = self.authorized(owner);
                dialog_authorized(owner, &self.request, &self.unknown, true).is_some()
                    && original_live
            }
            fn dialog_after_drop(&self) -> bool {
                self.owner.upgrade().is_some_and(|owner| {
                    dialog_authorized(&owner, &self.request, &self.unknown, false).is_some()
                })
            }
            fn dialog_navigation(
                &self,
                args: &ICoreWebView2NavigationStartingEventArgs,
                eligible: bool,
            ) -> windows::core::Result<bool> {
                let owner = self.owner.upgrade().ok_or_else(rejection)?;
                let original_live = self.dialog_live(&owner);
                // Consume the finite grant BEFORE any fallible/reentrant getter.
                let target = {
                    let mut state = owner.borrow_mut();
                    let i = matching(&mut state, &self.request).ok_or_else(rejection)?;
                    let d = i.refusal_registry.dialog.as_mut().ok_or_else(rejection)?;
                    d.facts.navigation_starts = d.facts.navigation_starts.saturating_add(1);
                    let allowed = eligible
                        && original_live
                        && d.grant_pending
                        && d.facts.navigation_starts == 1;
                    d.grant_pending = false;
                    d.facts.grant_consumed = allowed;
                    allowed.then(|| d.target.clone())
                };
                let Some(target) = target else {
                    unsafe { args.SetCancel(true) }?;
                    return Ok(true);
                };
                let uri = allocated_string(2048, |raw| unsafe { args.Uri(raw) });
                let mut id = 0;
                let id_result = unsafe { args.NavigationId(&mut id) };
                let mut redirected = BOOL::default();
                let redirect_result = unsafe { args.IsRedirected(&mut redirected) };
                let mut canceled = BOOL::default();
                let cancel_result = unsafe { args.Cancel(&mut canceled) };
                let live_after = self.dialog_live(&owner);
                let valid = uri.as_ref().is_ok_and(|u| u == &target)
                    && id_result.is_ok()
                    && redirect_result.is_ok()
                    && !redirected.as_bool()
                    && cancel_result.is_ok()
                    && !canceled.as_bool()
                    && live_after;
                {
                    let mut state = owner.borrow_mut();
                    let i = matching(&mut state, &self.request).ok_or_else(rejection)?;
                    let d = i.refusal_registry.dialog.as_mut().ok_or_else(rejection)?;
                    d.facts.start_uri_matched = uri.as_ref().is_ok_and(|u| u == &target);
                    d.facts.start_sender_matched = eligible;
                    d.facts.start_live_after_getters = live_after;
                    d.facts.start_id_getter_succeeded = id_result.is_ok();
                    d.facts.start_redirected_getter_succeeded = redirect_result.is_ok();
                    d.facts.start_not_redirected = redirect_result.is_ok() && !redirected.as_bool();
                    d.facts.start_cancel_readback_succeeded = cancel_result.is_ok();
                    d.facts.start_not_canceled = cancel_result.is_ok() && !canceled.as_bool();
                    if valid {
                        d.start_id = Some(id);
                    }
                }
                if valid {
                    return Ok(false);
                } // Never SetCancel(false).
                let restrictive = unsafe { args.SetCancel(true) };
                uri?;
                id_result?;
                redirect_result?;
                cancel_result?;
                restrictive?;
                Ok(true)
            }
            fn dialog_completed(
                &self,
                args: &ICoreWebView2NavigationCompletedEventArgs,
                eligible: bool,
            ) -> windows::core::Result<bool> {
                let owner = self.owner.upgrade().ok_or_else(rejection)?;
                let original_live = eligible && self.dialog_live(&owner);
                let mut id = 0;
                let id_result = unsafe { args.NavigationId(&mut id) };
                let mut success = BOOL::default();
                let success_result = unsafe { args.IsSuccess(&mut success) };
                let live_after = original_live && self.dialog_live(&owner);
                let valid = {
                    let mut state = owner.borrow_mut();
                    let i = matching(&mut state, &self.request).ok_or_else(rejection)?;
                    let d = i.refusal_registry.dialog.as_mut().ok_or_else(rejection)?;
                    d.facts.navigation_completions =
                        d.facts.navigation_completions.saturating_add(1);
                    d.facts.completed_sender_matched = eligible;
                    d.facts.completed_id_getter_succeeded = id_result.is_ok();
                    d.facts.completed_success_getter_succeeded = success_result.is_ok();
                    d.facts.completed_success = success_result.is_ok() && success.as_bool();
                    d.facts.navigation_id_matched = id_result.is_ok() && d.start_id == Some(id);
                    d.facts.completed_live_after_getters = live_after;
                    d.facts.navigation_completions == 1
                        && d.facts.navigation_starts == 1
                        && !d.document_published
                        && d.mailbox.is_none()
                        && d.facts.navigation_id_matched
                        && d.facts.completed_success
                        && live_after
                };
                id_result?;
                success_result?;
                if !live_after {
                    return Ok(true);
                } // Known expiry/revocation: cleanup, no new authority.
                if !valid {
                    return Err(rejection());
                }
                Ok(false)
            }
            fn dialog_opening(
                &self,
                args: &ICoreWebView2ScriptDialogOpeningEventArgs,
                eligible: bool,
            ) -> windows::core::Result<bool> {
                let owner = self.owner.upgrade().ok_or_else(rejection)?;
                let original_live = eligible && self.dialog_live(&owner);
                let target = {
                    let state = owner.borrow();
                    let i = state.instance.as_ref().ok_or_else(rejection)?;
                    i.refusal_registry
                        .dialog
                        .as_ref()
                        .ok_or_else(rejection)?
                        .target
                        .clone()
                };
                let mut kind = COREWEBVIEW2_SCRIPT_DIALOG_KIND::default();
                let kind_result = unsafe { args.Kind(&mut kind) };
                let message = allocated_string(128, |raw| unsafe { args.Message(raw) });
                let uri = allocated_string(2048, |raw| unsafe { args.Uri(raw) });
                let live_after = original_live && self.dialog_live(&owner);
                {
                    let mut state = owner.borrow_mut();
                    let i = matching(&mut state, &self.request).ok_or_else(rejection)?;
                    let d = i.refusal_registry.dialog.as_mut().ok_or_else(rejection)?;
                    d.facts.dialog_events = d.facts.dialog_events.saturating_add(1);
                    d.facts.dialog_kind_alert =
                        kind_result.is_ok() && kind == COREWEBVIEW2_SCRIPT_DIALOG_KIND_ALERT;
                    d.facts.dialog_message_matched =
                        message.as_ref().is_ok_and(|m| m == DIALOG_MESSAGE);
                    d.facts.dialog_uri_matched = uri.as_ref().is_ok_and(|u| u == &target);
                    d.facts.dialog_sender_matched = eligible;
                    d.facts.dialog_live_after_getters = live_after
                        && d.facts.fresh_document
                        && d.facts.script_submissions == 1
                        && d.mailbox.is_some();
                }
                // The restrictive action is the actual return without Accept,
                // ResultText, deferral or custom UI, even for invalid metadata.
                kind_result?;
                message?;
                uri?;
                Ok(true)
            }
        }
        fn add_script_dialog(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            index: usize,
            core: &ICoreWebView2,
        ) -> windows::core::Result<()> {
            let context = Context::new(owner, request);
            let handler = ScriptDialogOpeningEventHandler::create(Box::new(move |sender, args| {
                let active = Active::enter(&context.callbacks);
                let mut selected = false;
                let result = context.invoke(sender.as_ref(), |eligible| {
                    let args = args.as_ref().ok_or_else(rejection)?;
                    selected = context.dialog_selected();
                    if selected {
                        return context.dialog_opening(args, eligible);
                    }
                    let mut kind = COREWEBVIEW2_SCRIPT_DIALOG_KIND::default();
                    unsafe { args.Kind(&mut kind) }?;
                    // No Accept, result text or default-dialog fallback.
                    Ok(true)
                });
                drop(args);
                drop(sender);
                // selected can only be set inside invoke's STA-gated action.
                let timely_return = !selected || context.dialog_after_drop();
                if selected && let Some(owner) = context.owner.upgrade() {
                    {
                        let mut state = owner.borrow_mut();
                        if let Some(i) = matching(&mut state, &context.request)
                            && context.matches_instance(i)
                            && let Some(d) = i.refusal_registry.dialog.as_mut()
                        {
                            d.facts.non_accept_action_returned = result.is_ok() && timely_return;
                            d.facts.dialog_live_after_getters &= timely_return;
                        }
                    }
                    emit_dialog(
                        &owner,
                        &context.request,
                        &context.unknown,
                        NativeDialogStage::Refusal,
                    );
                }
                drop(active);
                result
            }));
            let mut token = 0;
            let result = unsafe { core.add_ScriptDialogOpening(&handler, &mut token) };
            {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, request).unwrap();
                if result.is_ok() {
                    i.refusal_registry.entries[index].token = Some(token);
                } else {
                    i.refusal_registry.unknown.store(true, Ordering::SeqCst);
                    i.failed = true;
                    i.closing = true;
                    i.cleanup_blocked = true;
                }
            }
            drop(handler);
            result?;
            require_live(owner, request)
        }
        fn add_dialog_completed(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            core: &Rc<ICoreWebView2>,
        ) -> windows::core::Result<()> {
            let context = Context::new(owner, request);
            let installation = Arc::clone(&context.unknown);
            let handler = NavigationCompletedEventHandler::create(Box::new(move |sender, args| {
                let active = Active::enter(&context.callbacks);
                let mut selected = false;
                let result = context.invoke(sender.as_ref(), |eligible| {
                    selected = true; // invoke has passed STA before this point.
                    context.dialog_completed(args.as_ref().ok_or_else(rejection)?, eligible)
                });
                drop(args);
                drop(sender);
                if selected {
                    let _ = context.dialog_after_drop();
                }
                drop(active);
                result
            }));
            {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, request).ok_or_else(rejection)?;
                i.refusal_registry.dialog_completed = Some(DialogRegistration {
                    core: Rc::clone(core),
                    token: None,
                    remove_attempted: false,
                });
            }
            if dialog_authorized(owner, request, &installation, true).is_none() {
                return Err(rejection());
            }
            let mut token = 0;
            let result = unsafe { core.add_NavigationCompleted(&handler, &mut token) };
            {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, request).ok_or_else(rejection)?;
                if result.is_ok() {
                    i.refusal_registry.dialog_completed.as_mut().unwrap().token = Some(token);
                    i.refusal_registry
                        .dialog
                        .as_mut()
                        .unwrap()
                        .facts
                        .completed_handler_added = true;
                } else {
                    i.refusal_registry.unknown.store(true, Ordering::SeqCst);
                }
            }
            drop(handler);
            result?;
            require_live(owner, request)
        }
        pub(super) fn navigate_dialog_owned_empty_html(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            owned_root: &Path,
            deadline: NativeDialogDeadline,
        ) -> Result<(), Code0SessionFailure> {
            use std::path::{Component, Prefix};
            super::require_native_refusal_install(owner, request)?;
            let denied = || Code0SessionFailure::PolicyRejected;
            let callbacks = {
                let state = owner.borrow();
                let i = state.instance.as_ref().ok_or_else(denied)?;
                Arc::clone(&i.callbacks)
            };
            let active = Active::enter(&callbacks);
            // Declared after Active: native locals end under its guard on errors.
            let core = {
                let state = owner.borrow();
                let i = state.instance.as_ref().ok_or_else(denied)?;
                if i.refusal_registry.legacy.is_some()
                    || i.refusal_registry.top_target.is_some()
                    || i.refusal_registry.child.is_some()
                    || i.refusal_registry.dialog.is_some()
                    || i.refusal_registry.dialog_completed.is_some()
                    || i.refusal_registry.popup.is_some()
                {
                    return Err(denied());
                }
                i.refusal_registry
                    .core
                    .as_ref()
                    .map(Rc::clone)
                    .ok_or_else(denied)?
            };
            // Bind the immutable original attempt end to this exact installation
            // before any N3 native query, observer registration or grant.
            let installation = {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, request).ok_or_else(denied)?;
                if !i.refusal_registry.complete()
                    || i.refusal_registry
                        .core
                        .as_ref()
                        .is_none_or(|actual| !Rc::ptr_eq(actual, &core))
                {
                    return Err(denied());
                }
                i.refusal_registry.dialog = Some(DialogState {
                    target: String::new(),
                    deadline: Rc::new(deadline),
                    grant_pending: false,
                    start_id: None,
                    document_published: false,
                    mailbox: None,
                    completion_seen: false,
                    closing_since: None,
                    facts: NativeDialogFacts {
                        settings_before_document: i.refusal_registry.settings_acknowledged,
                        ..NativeDialogFacts::default()
                    },
                });
                Arc::clone(&i.refusal_registry.unknown)
            };
            if dialog_authorized(owner, request, &installation, true).is_none() {
                return Err(denied());
            }
            let root = owned_root.canonicalize().map_err(|_| denied())?;
            let fixed = root.join(DIALOG_LEAF);
            let target = fixed.canonicalize().map_err(|_| denied())?;
            let local = |p: &Path| {
                p.is_absolute()
                    && matches!(p.components().next(),
                Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
            };
            if !local(&root)
                || !local(&target)
                || target != fixed
                || target.parent() != Some(root.as_path())
                || target.file_name() != Some(std::ffi::OsStr::new(DIALOG_LEAF))
            {
                return Err(denied());
            }
            let metadata = target.metadata().map_err(|_| denied())?;
            if !metadata.is_file() || metadata.len() != 0 {
                return Err(denied());
            }
            let url = reqwest::Url::from_file_path(&target).map_err(|_| denied())?;
            if url.scheme() != "file"
                || url.host_str().is_some()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
                || url.as_str().is_empty()
                || url.as_str().encode_utf16().count() > 2048
            {
                return Err(denied());
            }
            let target = url.to_string();
            let wide: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
            if dialog_authorized(owner, request, &installation, true).is_none() {
                return Err(denied());
            }
            {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, request).ok_or_else(denied)?;
                let d = i.refusal_registry.dialog.as_mut().ok_or_else(denied)?;
                d.target = target;
                d.facts.asset_binding_validated = true;
            }
            let source = dialog_step(owner, request, &installation, || {
                allocated_string(64, |raw| unsafe { core.Source(raw) })
            })
            .ok_or_else(denied)?;
            let initial = source.as_ref().is_ok_and(|s| s == "about:blank");
            drop(source);
            if !initial || dialog_authorized(owner, request, &installation, true).is_none() {
                fail_dialog(owner, false);
                return Err(denied());
            }
            matching(&mut owner.borrow_mut(), request)
                .unwrap()
                .refusal_registry
                .dialog
                .as_mut()
                .unwrap()
                .facts
                .initial_source_matched = true;
            let added = dialog_step(owner, request, &installation, || {
                add_dialog_completed(owner, request, &core)
            })
            .ok_or_else(denied)?;
            let added_ok = added.is_ok();
            drop(added);
            if !added_ok {
                // Actual failed adds already latch uncertainty inside the add
                // producer. A known pre-add expiry must not invent that failure.
                fail_dialog(owner, false);
                return Err(denied());
            }
            if dialog_authorized(owner, request, &installation, true).is_none() {
                return Err(denied());
            }
            {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, request).ok_or_else(denied)?;
                let d = i.refusal_registry.dialog.as_mut().ok_or_else(denied)?;
                if d.facts.navigation_submissions != 0
                    || d.facts.navigation_starts != 0
                    || d.facts.navigation_completions != 0
                {
                    return Err(denied());
                }
                d.grant_pending = true;
            }
            let result = dialog_step(owner, request, &installation, || {
                matching(&mut owner.borrow_mut(), request)
                    .unwrap()
                    .refusal_registry
                    .dialog
                    .as_mut()
                    .unwrap()
                    .facts
                    .navigation_submissions = 1;
                unsafe { core.Navigate(PCWSTR(wide.as_ptr())) }
            })
            .ok_or_else(denied)?;
            let ok = result.is_ok();
            drop(result);
            drop(core);
            drop(active);
            let timely = dialog_authorized(owner, request, &installation, true).is_some();
            {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, request).ok_or_else(denied)?;
                let d = i.refusal_registry.dialog.as_mut().ok_or_else(denied)?;
                d.facts.navigation_submission_returned = true;
                d.facts.navigation_submit_ok = ok;
            }
            // Truthful submission disposal, never a late new grant/readiness.
            emit_dialog(owner, request, &installation, NativeDialogStage::Submitted);
            if !ok || !timely {
                fail_dialog(owner, false);
                return Err(denied());
            }
            Ok(())
        }
        fn consume_dialog_completion(owner: &Rc<RefCell<Owner>>) {
            let mailbox = owner.borrow().instance.as_ref().and_then(|i| {
                i.refusal_registry
                    .dialog
                    .as_ref()
                    .and_then(|d| d.mailbox.as_ref().map(Arc::clone))
            });
            let Some(m) = mailbox else {
                return;
            };
            if m.invalid.load(Ordering::SeqCst) {
                fail_dialog(owner, true);
            }
            let disposition = m.disposition.lock().unwrap().take();
            let Some(d) = disposition else {
                return;
            };
            let mut uncertainty = false;
            let successful = {
                let mut state = owner.borrow_mut();
                let unmatched = state.unmatched;
                let Some(i) = state.instance.as_mut() else {
                    return;
                };
                let same_installation = i.request == d.request
                    && i.refusal_registry.generation == d.generation
                    && Arc::ptr_eq(&i.refusal_registry.unknown, &d.installation);
                let Some(dialog) = i.refusal_registry.dialog.as_mut() else {
                    return;
                };
                let same = !unmatched
                    && same_installation
                    && dialog
                        .mailbox
                        .as_ref()
                        .is_some_and(|actual| Arc::ptr_eq(actual, &m))
                    && !dialog.completion_seen
                    && m.obligation.load(Ordering::SeqCst) == 1;
                if !same {
                    d.installation.store(true, Ordering::SeqCst);
                    m.invalid.store(true, Ordering::SeqCst);
                    return;
                }
                let closing = *m.closing_since.lock().unwrap();
                let consumed_at = Instant::now();
                let timely =
                    dialog
                        .deadline
                        .completion_timely_at(d.received_at, consumed_at, closing);
                let disposal_timely = NativeDialogDeadline::disposal_within_close_at(
                    d.received_at,
                    consumed_at,
                    closing,
                );
                let valid = same
                    && !i.released
                    && disposal_timely
                    && !m.invalid.load(Ordering::SeqCst)
                    && !i.callbacks.affinity_failed.load(Ordering::SeqCst)
                    && (!d.hresult_succeeded || d.bounded_result_valid && d.fixed_result_matched);
                let f = &mut dialog.facts;
                f.script_completions = f.script_completions.saturating_add(1);
                f.completion_identity_matched = same;
                f.completion_hresult_succeeded = d.hresult_succeeded;
                f.bounded_result_valid = d.bounded_result_valid;
                f.fixed_result_matched = d.fixed_result_matched;
                f.completion_before_release = !i.released;
                f.completion_within_deadline = timely;
                if valid {
                    dialog.completion_seen = true;
                    m.obligation.store(0, Ordering::SeqCst);
                    f.completion_disposition_consumed = true;
                } else {
                    uncertainty = true;
                }
                valid
                    && timely
                    && d.hresult_succeeded
                    && d.bounded_result_valid
                    && d.fixed_result_matched
            };
            if !successful {
                fail_dialog(owner, uncertainty);
            }
            emit_dialog(
                owner,
                &d.request,
                &d.installation,
                NativeDialogStage::Completion,
            );
        }
        pub(super) fn advance_dialog(owner: &Rc<RefCell<Owner>>) {
            let unknown = owner.borrow().instance.as_ref().is_some_and(|i| {
                i.refusal_registry.dialog.is_some() && i.refusal_registry.unknown()
            });
            if unknown {
                fail_dialog(owner, true);
            }
            let binding = {
                let state = owner.borrow();
                state
                    .instance
                    .as_ref()
                    .filter(|i| i.refusal_registry.dialog.is_some())
                    .map(|i| (i.request.clone(), Arc::clone(&i.refusal_registry.unknown)))
            };
            if let Some((request, installation)) = binding {
                // This observes expiry even while readiness is still incomplete;
                // a later closing adoption cannot renew the original attempt.
                let _ = dialog_authorized(owner, &request, &installation, false);
            }
            consume_dialog_completion(owner);
            let action = {
                let state = owner.borrow();
                state.instance.as_ref().and_then(|i| {
                    let d = i.refusal_registry.dialog.as_ref()?;
                    if d.document_published
                        || i.callbacks.active.load(Ordering::SeqCst) != 0
                        || !d.facts.navigation_submission_returned
                        || !d.facts.navigation_submit_ok
                        || d.facts.navigation_starts != 1
                        || d.facts.navigation_completions != 1
                        || !d.facts.navigation_id_matched
                        || !d.facts.completed_success
                    {
                        return None;
                    }
                    i.refusal_registry.core.as_ref().map(|core| {
                        (
                            Rc::clone(core),
                            Arc::clone(&i.callbacks),
                            i.request.clone(),
                            d.target.clone(),
                            Arc::clone(&i.refusal_registry.unknown),
                        )
                    })
                })
            };
            let Some((core, callbacks, request, target, installation)) = action else {
                return;
            };
            let active = Active::enter(&callbacks);
            if dialog_authorized(owner, &request, &installation, true).is_none() {
                drop(core);
                drop(active);
                return;
            }
            let Some(source) = dialog_step(owner, &request, &installation, || {
                allocated_string(2048, |raw| unsafe { core.Source(raw) })
            }) else {
                drop(core);
                drop(active);
                return;
            };
            let matched = source.as_ref().is_ok_and(|s| s == &target);
            drop(source);
            let valid =
                matched && dialog_authorized(owner, &request, &installation, true).is_some();
            drop(core);
            drop(active);
            if !valid || dialog_authorized(owner, &request, &installation, true).is_none() {
                fail_dialog(owner, false);
                return;
            }
            if dialog_step(owner, &request, &installation, || {
                {
                    let mut state = owner.borrow_mut();
                    let i = matching(&mut state, &request).unwrap();
                    let d = i.refusal_registry.dialog.as_mut().unwrap();
                    d.document_published = true;
                    d.facts.source_after_complete_matched = true;
                    d.facts.fresh_document = true;
                }
                emit_dialog(
                    owner,
                    &request,
                    &installation,
                    NativeDialogStage::DocumentReady,
                );
            })
            .is_none()
            {
                return;
            }
            submit_dialog_alert(owner, &request, &installation);
        }
        fn submit_dialog_alert(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            expected_installation: &Arc<AtomicBool>,
        ) {
            let action = {
                let state = owner.borrow();
                state
                    .instance
                    .as_ref()
                    .filter(|i| {
                        &i.request == request
                            && i.refusal_registry.generation == GENERATION
                            && Arc::ptr_eq(&i.refusal_registry.unknown, expected_installation)
                    })
                    .and_then(|i| {
                        let d = i.refusal_registry.dialog.as_ref()?;
                        if !d.facts.fresh_document || d.mailbox.is_some() {
                            return None;
                        }
                        i.refusal_registry.core.as_ref().map(|core| {
                            (
                                Rc::clone(core),
                                Arc::clone(&i.callbacks),
                                state.ui_thread,
                                d.target.clone(),
                                Arc::clone(&i.refusal_registry.unknown),
                            )
                        })
                    })
            };
            let Some((core, callbacks, ui, target, installation)) = action else {
                expected_installation.store(true, Ordering::SeqCst);
                if dialog_deadline(owner, request, expected_installation).is_some() {
                    fail_dialog(owner, true);
                }
                return;
            };
            let active = Active::enter(&callbacks);
            let Some(source) = dialog_step(owner, request, &installation, || {
                allocated_string(2048, |raw| unsafe { core.Source(raw) })
            }) else {
                drop(core);
                drop(active);
                return;
            };
            let matched = source.as_ref().is_ok_and(|s| s == &target);
            drop(source);
            if !matched || dialog_authorized(owner, request, &installation, true).is_none() {
                drop(core);
                drop(active);
                fail_dialog(owner, false);
                return;
            }
            let m = Arc::new(DialogMailbox {
                disposition: Mutex::new(None),
                invoked: AtomicBool::new(false),
                invalid: AtomicBool::new(false),
                obligation: AtomicUsize::new(1),
                handlers: AtomicUsize::new(1),
                closing_since: Mutex::new(None),
            });
            if dialog_step(owner, request, &installation, || {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, request).unwrap();
                let d = i.refusal_registry.dialog.as_mut().unwrap();
                d.facts.source_before_script_matched = true;
            })
            .is_none()
            {
                drop(core);
                drop(active);
                return;
            }
            let handler: ICoreWebView2ExecuteScriptCompletedHandler = DialogCompletion {
                mailbox: Arc::downgrade(&m),
                callbacks: Arc::clone(&callbacks),
                installation: Arc::clone(&installation),
                request: request.clone(),
                generation: GENERATION,
                ui,
                _lifetime: DialogHandlerLifetime {
                    mailbox: Arc::downgrade(&m),
                    _aggregate: HandlerLifetime::new(&callbacks),
                },
            }
            .into();
            let wide: Vec<u16> = DIALOG_SCRIPT
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let result = dialog_step(owner, request, &installation, || {
                {
                    let mut state = owner.borrow_mut();
                    let d = matching(&mut state, request)
                        .unwrap()
                        .refusal_registry
                        .dialog
                        .as_mut()
                        .unwrap();
                    // Commit the separate invocation ticket only after the final
                    // original-deadline gate. Once committed, it is never erased
                    // without a real matching callback, including submit errors.
                    d.mailbox = Some(Arc::clone(&m));
                    d.facts.script_submissions = 1;
                }
                unsafe { core.ExecuteScript(PCWSTR(wide.as_ptr()), &handler) }
            });
            let returned = result.is_some();
            let ok = result.as_ref().is_some_and(|result| result.is_ok());
            drop(result);
            drop(handler);
            drop(core);
            drop(active);
            let timely_return = dialog_authorized(owner, request, &installation, false).is_some();
            {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, request).unwrap();
                let d = i.refusal_registry.dialog.as_mut().unwrap();
                d.facts.script_submission_returned = returned;
                d.facts.script_submit_ok = ok;
            }
            if returned {
                emit_dialog(
                    owner,
                    request,
                    &installation,
                    NativeDialogStage::ScriptSubmitted,
                );
            }
            if !ok || !timely_return {
                fail_dialog(owner, false);
            }
        }
        fn remove_dialog_completed(owner: &Rc<RefCell<Owner>>) {
            let action = {
                let mut state = owner.borrow_mut();
                let i = state.instance.as_mut().unwrap();
                let Some(e) = i.refusal_registry.dialog_completed.as_mut() else {
                    return;
                };
                if e.remove_attempted {
                    return;
                }
                let Some(token) = e.token else {
                    return;
                };
                e.remove_attempted = true;
                let same = i
                    .refusal_registry
                    .core
                    .as_ref()
                    .is_some_and(|core| Rc::ptr_eq(core, &e.core));
                let d = i.refusal_registry.dialog.as_mut().unwrap();
                d.facts.completed_remove_attempts =
                    d.facts.completed_remove_attempts.saturating_add(1);
                d.facts.completed_same_emitter = same;
                (Rc::clone(&e.core), token, same)
            };
            // PollGuard protects this local emitter and HRESULT/ErrorInfo drops.
            let (core, token, same) = action;
            let result = unsafe { core.remove_NavigationCompleted(token) };
            {
                let mut state = owner.borrow_mut();
                let i = state.instance.as_mut().unwrap();
                if result.is_ok() && same {
                    i.refusal_registry.dialog_completed.as_mut().unwrap().token = None;
                    i.refusal_registry
                        .dialog
                        .as_mut()
                        .unwrap()
                        .facts
                        .completed_remove_succeeded = true;
                } else {
                    i.refusal_registry.unknown.store(true, Ordering::SeqCst);
                }
            }
            drop(result);
            drop(core);
            if !same {
                fail_dialog(owner, true);
            }
        }
        pub(super) fn publish_dialog_final(owner: &Rc<RefCell<Owner>>) {
            let binding = {
                let mut state = owner.borrow_mut();
                if let Some(i) = state.instance.as_mut()
                    && let Some(d) = i.refusal_registry.dialog.as_mut()
                {
                    d.facts.completion_obligation_drained = d.completion_seen
                        && d.mailbox
                            .as_ref()
                            .is_some_and(|m| m.obligation.load(Ordering::SeqCst) == 0);
                    d.facts.completion_handlers_drained = d
                        .mailbox
                        .as_ref()
                        .is_some_and(|m| m.handlers.load(Ordering::SeqCst) == 0)
                        && i.callbacks.handlers.load(Ordering::SeqCst) == 0;
                    Some((i.request.clone(), Arc::clone(&i.refusal_registry.unknown)))
                } else {
                    None
                }
            };
            if let Some((request, installation)) = binding {
                emit_dialog(owner, &request, &installation, NativeDialogStage::Final);
            }
        }

        // N4 has no creation Pending and no additional emitter registration.
        // Its first revocation exists before Source/handler preparation; only an
        // actual final native invocation installs a completion obligation.
        struct PopupState {
            deadline: Rc<NativeDialogDeadline>,
            core: Weak<ICoreWebView2>,
            closing_since: Option<Instant>,
            first_close: LegacyChildClose,
            closing_published: bool,
            mailbox: Option<Arc<PopupMailbox>>,
            completion_seen: bool,
            facts: NativePopupFacts,
        }
        impl PopupState {
            fn quiescent(&self) -> bool {
                self.mailbox.as_ref().is_none_or(|m| {
                    self.completion_seen
                        && m.obligation.load(Ordering::SeqCst) == 0
                        && m.handlers.load(Ordering::SeqCst) == 0
                        && !m.invalid.load(Ordering::SeqCst)
                        && m.disposition.lock().unwrap().is_none()
                })
            }
        }
        struct PopupMailbox {
            disposition: Mutex<Option<PopupDisposition>>,
            invoked: AtomicBool,
            invalid: AtomicBool,
            obligation: AtomicUsize,
            handlers: AtomicUsize,
            closing_since: Mutex<Option<Instant>>,
        }
        struct PopupDisposition {
            request: Code0WindowRequest,
            generation: u64,
            installation: Arc<AtomicBool>,
            received_at: Instant,
            hresult_succeeded: bool,
            bounded_result_valid: bool,
            fixed_result_matched: bool,
        }
        struct PopupHandlerLifetime {
            mailbox: std::sync::Weak<PopupMailbox>,
            _aggregate: HandlerLifetime,
        }
        impl Drop for PopupHandlerLifetime {
            fn drop(&mut self) {
                if let Some(m) = self.mailbox.upgrade() {
                    m.handlers.fetch_sub(1, Ordering::SeqCst);
                }
                // Only a real matching invocation disposition settles its ticket.
            }
        }
        #[windows::core::implement(ICoreWebView2ExecuteScriptCompletedHandler, Agile = false)]
        struct PopupCompletion {
            mailbox: std::sync::Weak<PopupMailbox>,
            callbacks: Arc<Callbacks>,
            installation: Arc<AtomicBool>,
            request: Code0WindowRequest,
            generation: u64,
            ui: ThreadId,
            _lifetime: PopupHandlerLifetime,
        }
        impl ICoreWebView2ExecuteScriptCompletedHandler_Impl for PopupCompletion_Impl {
            fn Invoke(
                &self,
                errorcode: windows::core::HRESULT,
                result: &PCWSTR,
            ) -> windows::core::Result<()> {
                let _active = Active::enter(&self.callbacks);
                // UI/STA before raw copying, allocation, or UI-local references.
                if !callback_on_ui(self.ui, &self.callbacks) {
                    self.installation.store(true, Ordering::SeqCst);
                    if let Some(m) = self.mailbox.upgrade() {
                        m.invalid.store(true, Ordering::SeqCst);
                    }
                    return Ok(());
                }
                let Some(m) = self.mailbox.upgrade() else {
                    self.installation.store(true, Ordering::SeqCst);
                    return Ok(());
                };
                if m.invoked.swap(true, Ordering::SeqCst) {
                    m.invalid.store(true, Ordering::SeqCst);
                    self.installation.store(true, Ordering::SeqCst);
                    return Ok(());
                }
                let received_at = Instant::now();
                let (bounded_result_valid, fixed_result_matched) = popup_result(*result);
                let disposition = PopupDisposition {
                    request: self.request.clone(),
                    generation: self.generation,
                    installation: Arc::clone(&self.installation),
                    received_at,
                    hresult_succeeded: errorcode.is_ok(),
                    bounded_result_valid,
                    fixed_result_matched,
                };
                let mut slot = m.disposition.lock().unwrap();
                if slot.is_some() {
                    m.invalid.store(true, Ordering::SeqCst);
                    self.installation.store(true, Ordering::SeqCst);
                } else {
                    *slot = Some(disposition);
                }
                Ok(())
            }
        }
        fn popup_result(raw: PCWSTR) -> (bool, bool) {
            if raw.is_null() {
                return (false, false);
            }
            let mut copied = [0u16; 64];
            for length in 0..copied.len() {
                // Borrowed callback memory, never retained/freed or converted by
                // an unbounded wrapper. Including NUL: at most 64 UTF16 reads.
                let unit = unsafe { *raw.0.add(length) };
                if unit == 0 {
                    return popup_result_units(&copied[..length]);
                }
                copied[length] = unit;
            }
            (false, false)
        }
        fn emit_popup(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            installation: &Arc<AtomicBool>,
            stage: NativePopupStage,
        ) {
            let observation = {
                let state = owner.borrow();
                state
                    .instance
                    .as_ref()
                    .filter(|i| {
                        &i.request == request
                            && i.refusal_registry.generation == GENERATION
                            && Arc::ptr_eq(&i.refusal_registry.unknown, installation)
                    })
                    .and_then(|i| {
                        i.refusal_registry.popup.as_ref().map(|p| {
                            (
                                i.observer.clone(),
                                NativePopupReceipt {
                                    request: request.clone(),
                                    installation: Arc::clone(installation),
                                    stage,
                                    facts: p.facts,
                                },
                            )
                        })
                    })
            };
            if let Some((Some(observer), receipt)) = observation {
                observer(NativeSmokeObservation::Popup(receipt));
            }
        }
        pub(super) fn mark_popup_close(i: &mut Instance) -> Option<Instant> {
            let p = i.refusal_registry.popup.as_mut()?;
            let at = p
                .first_close
                .mark_at(Instant::now(), Some(&mut p.closing_since))?;
            p.facts.first_closing_observed = true;
            if let Some(m) = p.mailbox.as_ref() {
                *m.closing_since.lock().unwrap() = Some(at);
            }
            Some(at)
        }
        pub(super) fn publish_popup_close(owner: &Rc<RefCell<Owner>>) {
            let notification = {
                let mut state = owner.borrow_mut();
                state.instance.as_mut().and_then(|i| {
                    let p = i.refusal_registry.popup.as_mut()?;
                    let at = p.closing_since?;
                    if p.closing_published {
                        return None;
                    }
                    p.closing_published = true;
                    Some((i.observer.clone(), at))
                })
            };
            if let Some((Some(observer), at)) = notification {
                observer(NativeSmokeObservation::PopupClosing { at });
            }
        }
        pub(super) fn synchronize_popup_close(owner: &Rc<RefCell<Owner>>) {
            {
                let mut state = owner.borrow_mut();
                let unmatched = state.unmatched;
                if let Some(i) = state.instance.as_mut()
                    && i.refusal_registry.popup.is_some()
                {
                    let invalid = unmatched
                        || i.callbacks.affinity_failed.load(Ordering::SeqCst)
                        || i.refusal_registry.unknown();
                    if invalid {
                        i.failed = true;
                        i.closing = true;
                        i.cleanup_blocked = true;
                    }
                    if i.closing {
                        let _ = mark_popup_close(i);
                    }
                }
            }
            publish_popup_close(owner);
        }
        fn fail_popup(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            installation: &Arc<AtomicBool>,
        ) {
            {
                let mut state = owner.borrow_mut();
                let Some(i) = matching(&mut state, request).filter(|i| {
                    i.refusal_registry.generation == GENERATION
                        && Arc::ptr_eq(&i.refusal_registry.unknown, installation)
                        && i.refusal_registry.popup.is_some()
                }) else {
                    return;
                };
                i.failed = true;
                i.closing = true;
                let _ = mark_popup_close(i);
            }
            publish_popup_close(owner);
        }
        fn popup_deadline(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            installation: &Arc<AtomicBool>,
        ) -> Option<Rc<NativeDialogDeadline>> {
            let state = owner.borrow();
            let i = state.instance.as_ref()?;
            if &i.request != request
                || i.refusal_registry.generation != GENERATION
                || !Arc::ptr_eq(&i.refusal_registry.unknown, installation)
            {
                return None;
            }
            let p = i.refusal_registry.popup.as_ref()?;
            let core = i.refusal_registry.core.as_ref()?;
            if !Weak::ptr_eq(&p.core, &Rc::downgrade(core)) {
                return None;
            }
            Some(Rc::clone(&p.deadline))
        }
        fn popup_authorized(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            installation: &Arc<AtomicBool>,
            require_live: bool,
        ) -> Option<Rc<NativeDialogDeadline>> {
            let deadline = popup_deadline(owner, request, installation)?;
            let identity = {
                let state = owner.borrow();
                !state.unmatched
                    && state.instance.as_ref().is_some_and(|i| {
                        &i.request == request
                            && i.refusal_registry.generation == GENERATION
                            && Arc::ptr_eq(&i.refusal_registry.unknown, installation)
                            && !i.released
                            && !i.cleanup_blocked
                            && !i.callbacks.affinity_failed.load(Ordering::SeqCst)
                            && !i.refusal_registry.unknown()
                    })
            };
            if deadline
                .run_if_authorized_at(
                    Instant::now(),
                    identity && (!require_live || live(owner, request)),
                    || (),
                )
                .is_none()
            {
                fail_popup(owner, request, installation);
                return None;
            }
            Some(deadline)
        }
        fn popup_step<T>(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            installation: &Arc<AtomicBool>,
            action: impl FnOnce() -> T,
        ) -> Option<T> {
            let deadline = popup_authorized(owner, request, installation, true)?;
            let result =
                deadline.run_if_authorized_at(Instant::now(), live(owner, request), action);
            if result.is_none() {
                fail_popup(owner, request, installation);
            }
            result
        }
        pub(super) fn submit_popup_initial_blank(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            deadline: NativeDialogDeadline,
        ) -> Result<(), Code0SessionFailure> {
            let denied = || Code0SessionFailure::PolicyRejected;
            super::require_native_refusal_install(owner, request)?;
            let (callbacks, ui, core, installation) = {
                let mut state = owner.borrow_mut();
                let ui = state.ui_thread;
                if state.unmatched {
                    return Err(denied());
                }
                let i = matching(&mut state, request).ok_or_else(denied)?;
                if !i.refusal_registry.complete()
                    || i.refusal_registry.legacy.is_some()
                    || i.refusal_registry.top_target.is_some()
                    || i.refusal_registry.child.is_some()
                    || i.refusal_registry.dialog.is_some()
                    || i.refusal_registry.dialog_completed.is_some()
                    || i.refusal_registry.popup.is_some()
                {
                    return Err(denied());
                }
                let core = Rc::clone(i.refusal_registry.core.as_ref().ok_or_else(denied)?);
                i.refusal_registry.popup = Some(PopupState {
                    deadline: Rc::new(deadline),
                    core: Rc::downgrade(&core),
                    closing_since: None,
                    first_close: LegacyChildClose::new(),
                    closing_published: false,
                    mailbox: None,
                    completion_seen: false,
                    facts: NativePopupFacts {
                        settings_before_stimulus: i.refusal_registry.settings_acknowledged,
                        no_new_window_assignment: true,
                        ..NativePopupFacts::default()
                    },
                });
                (
                    Arc::clone(&i.callbacks),
                    ui,
                    core,
                    Arc::clone(&i.refusal_registry.unknown),
                )
            };
            let active = Active::enter(&callbacks);
            let source = popup_step(owner, request, &installation, || {
                allocated_string(2048, |raw| unsafe { core.Source(raw) })
            });
            let source_matched = source
                .as_ref()
                .is_some_and(|r| r.as_ref().is_ok_and(|s| s == "about:blank"));
            drop(source);
            let source_timely = popup_authorized(owner, request, &installation, true).is_some();
            {
                let mut state = owner.borrow_mut();
                let p = matching(&mut state, request)
                    .unwrap()
                    .refusal_registry
                    .popup
                    .as_mut()
                    .unwrap();
                p.facts.initial_source_matched = source_matched;
                p.facts.source_after_drop_timely = source_timely;
            }
            if !source_matched || !source_timely {
                fail_popup(owner, request, &installation);
                drop(core);
                drop(active);
                let _ = popup_authorized(owner, request, &installation, false);
                return Err(denied());
            }
            let mailbox = Arc::new(PopupMailbox {
                disposition: Mutex::new(None),
                invoked: AtomicBool::new(false),
                invalid: AtomicBool::new(false),
                obligation: AtomicUsize::new(1),
                handlers: AtomicUsize::new(1),
                closing_since: Mutex::new(None),
            });
            let handler: ICoreWebView2ExecuteScriptCompletedHandler = PopupCompletion {
                mailbox: Arc::downgrade(&mailbox),
                callbacks: Arc::clone(&callbacks),
                installation: Arc::clone(&installation),
                request: request.clone(),
                generation: GENERATION,
                ui,
                _lifetime: PopupHandlerLifetime {
                    mailbox: Arc::downgrade(&mailbox),
                    _aggregate: HandlerLifetime::new(&callbacks),
                },
            }
            .into();
            let wide: Vec<u16> = POPUP_SCRIPT
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let result = popup_step(owner, request, &installation, || {
                {
                    let mut state = owner.borrow_mut();
                    let p = matching(&mut state, request)
                        .unwrap()
                        .refusal_registry
                        .popup
                        .as_mut()
                        .unwrap();
                    // The final native-call commitment, before possible synchronous
                    // event/completion entry. No earlier preparation is a ticket.
                    p.mailbox = Some(Arc::clone(&mailbox));
                    p.facts.script_submissions = 1;
                }
                unsafe { core.ExecuteScript(PCWSTR(wide.as_ptr()), &handler) }
            });
            let returned = result.is_some();
            let native_ok = result.as_ref().is_some_and(|r| r.is_ok());
            drop(result);
            let _ = popup_authorized(owner, request, &installation, false);
            drop(handler);
            let _ = popup_authorized(owner, request, &installation, false);
            drop(core);
            let _ = popup_authorized(owner, request, &installation, false);
            drop(wide);
            drop(active);
            let timely = popup_authorized(owner, request, &installation, false).is_some();
            {
                let mut state = owner.borrow_mut();
                let p = matching(&mut state, request)
                    .unwrap()
                    .refusal_registry
                    .popup
                    .as_mut()
                    .unwrap();
                p.facts.script_submission_returned = returned;
                p.facts.script_submit_ok = native_ok;
                p.facts.script_return_timely = timely;
            }
            if returned {
                emit_popup(owner, request, &installation, NativePopupStage::Submitted);
            }
            if !native_ok || !timely {
                fail_popup(owner, request, &installation);
                return Err(denied());
            }
            Ok(())
        }
        pub(super) fn advance_popup(owner: &Rc<RefCell<Owner>>) {
            let binding = owner.borrow().instance.as_ref().and_then(|i| {
                i.refusal_registry.popup.as_ref().and_then(|p| {
                    p.mailbox.as_ref().map(|m| {
                        (
                            Arc::clone(m),
                            i.request.clone(),
                            Arc::clone(&i.refusal_registry.unknown),
                        )
                    })
                })
            });
            let Some((mailbox, request, installation)) = binding else {
                return;
            };
            let disposition = mailbox.disposition.lock().unwrap().take();
            let mut failed = false;
            let mut publish = false;
            {
                let mut state = owner.borrow_mut();
                let unmatched = state.unmatched;
                let Some(i) = matching(&mut state, &request).filter(|i| {
                    i.refusal_registry.generation == GENERATION
                        && Arc::ptr_eq(&i.refusal_registry.unknown, &installation)
                        && i.refusal_registry
                            .popup
                            .as_ref()
                            .and_then(|p| p.mailbox.as_ref())
                            .is_some_and(|m| Arc::ptr_eq(m, &mailbox))
                }) else {
                    installation.store(true, Ordering::SeqCst);
                    mailbox.invalid.store(true, Ordering::SeqCst);
                    return;
                };
                if mailbox.invalid.load(Ordering::SeqCst) {
                    installation.store(true, Ordering::SeqCst);
                }
                if let Some(d) = disposition {
                    let p = i.refusal_registry.popup.as_mut().unwrap();
                    p.facts.script_completions =
                        p.facts.script_completions.saturating_add(1).min(2);
                    let same = !unmatched
                        && d.request == i.request
                        && d.generation == i.refusal_registry.generation
                        && Arc::ptr_eq(&installation, &d.installation)
                        && !p.completion_seen
                        && p.facts.script_submissions == 1
                        && mailbox.obligation.load(Ordering::SeqCst) == 1;
                    let closing = *mailbox.closing_since.lock().unwrap();
                    let outcome = legacy_completion_disposition_at(
                        &p.deadline,
                        d.received_at,
                        Instant::now(),
                        closing,
                        LegacyCompletionInput {
                            identity_valid: same
                                && !i.released
                                && !i.cleanup_blocked
                                && !i.callbacks.affinity_failed.load(Ordering::SeqCst)
                                && !mailbox.invalid.load(Ordering::SeqCst)
                                && !installation.load(Ordering::SeqCst),
                            hresult_succeeded: d.hresult_succeeded,
                            bounded_result_valid: d.bounded_result_valid,
                            fixed_result_matched: d.fixed_result_matched,
                        },
                    );
                    p.facts.completion_identity_matched = same;
                    p.facts.completion_hresult_succeeded = d.hresult_succeeded;
                    p.facts.bounded_result_valid = d.bounded_result_valid;
                    p.facts.fixed_result_matched = d.fixed_result_matched;
                    p.facts.completion_before_release = !i.released;
                    p.facts.completion_within_deadline = outcome.timely;
                    if outcome.disposable {
                        p.completion_seen = true;
                        mailbox.obligation.store(0, Ordering::SeqCst);
                        p.facts.completion_disposition_consumed = true;
                    } else {
                        installation.store(true, Ordering::SeqCst);
                    }
                    failed = !outcome.successful;
                    publish = true;
                }
            }
            if failed {
                fail_popup(owner, &request, &installation);
            }
            latch_unknown(owner);
            if publish {
                emit_popup(owner, &request, &installation, NativePopupStage::Completion);
            }
        }
        pub(super) fn publish_popup_final(owner: &Rc<RefCell<Owner>>) {
            let binding = {
                let mut state = owner.borrow_mut();
                state.instance.as_mut().and_then(|i| {
                    i.refusal_registry.popup.as_mut().map(|p| {
                        p.facts.completion_obligation_drained = p.completion_seen
                            && p.mailbox
                                .as_ref()
                                .is_some_and(|m| m.obligation.load(Ordering::SeqCst) == 0);
                        p.facts.completion_handlers_drained = p
                            .mailbox
                            .as_ref()
                            .is_some_and(|m| m.handlers.load(Ordering::SeqCst) == 0)
                            && i.callbacks.handlers.load(Ordering::SeqCst) == 0;
                        (i.request.clone(), Arc::clone(&i.refusal_registry.unknown))
                    })
                })
            };
            if let Some((request, installation)) = binding {
                emit_popup(owner, &request, &installation, NativePopupStage::Final);
            }
        }
        impl Context {
            fn popup_selected(&self) -> bool {
                self.owner.upgrade().is_some_and(|owner| {
                    owner.borrow().instance.as_ref().is_some_and(|i| {
                        self.matches_instance(i) && i.refusal_registry.popup.is_some()
                    })
                })
            }
            fn reserve_popup_target(&self) -> bool {
                let Some(owner) = self.owner.upgrade() else {
                    return false;
                };
                let mut state = owner.borrow_mut();
                let Some(i) =
                    matching(&mut state, &self.request).filter(|i| self.matches_instance(i))
                else {
                    return false;
                };
                let Some(p) = i.refusal_registry.popup.as_mut() else {
                    return false;
                };
                p.facts.popup_events = p.facts.popup_events.saturating_add(1).min(2);
                let issued = p.facts.script_submissions == 1 && p.mailbox.is_some();
                p.facts.issued_before_event = issued;
                if p.facts.popup_events != 1 || !issued {
                    self.unknown.store(true, Ordering::SeqCst);
                    return false;
                }
                true
            }
            fn popup_observe(
                &self,
                event: &mut LegacyRefusalEvent,
                require_live: bool,
                eligible: bool,
            ) -> bool {
                let Some(owner) = self.owner.upgrade() else {
                    return false;
                };
                let Some(deadline) = popup_deadline(&owner, &self.request, &self.unknown) else {
                    return false;
                };
                let identity =
                    popup_authorized(&owner, &self.request, &self.unknown, require_live).is_some();
                event.observe_at(&deadline, Instant::now(), identity && eligible)
            }
            fn popup_after_drop(&self, event: &mut LegacyRefusalEvent) -> bool {
                let Some(owner) = self.owner.upgrade() else {
                    return false;
                };
                let Some(deadline) = popup_deadline(&owner, &self.request, &self.unknown) else {
                    return false;
                };
                let identity =
                    popup_authorized(&owner, &self.request, &self.unknown, false).is_some();
                event.qualify_at(&deadline, Instant::now(), identity)
            }
            fn popup_opening(
                &self,
                args: &ICoreWebView2NewWindowRequestedEventArgs,
                eligible: bool,
                event: &mut LegacyRefusalEvent,
                facts: &mut NativePopupFacts,
            ) -> windows::core::Result<bool> {
                // Restriction is unconditional even if eligibility/metadata fails.
                // No other native action is performed before this single setter.
                facts.popup_sender_matched = eligible;
                facts.request_generation_live_matched = eligible;
                let initial = self.popup_observe(event, true, eligible);
                facts.handled_attempts = 1;
                let action = event
                    .cancel_once(|| unsafe { args.SetHandled(true) })
                    .ok_or_else(rejection)?;
                facts.handled_returns = 1;
                facts.handled_successes = u8::from(action.is_ok());
                let action_ok = action.is_ok();
                drop(action);
                let mut timely = self.popup_observe(event, true, true) && initial;
                if action_ok && timely {
                    let mut handled = BOOL::from(false);
                    facts.handled_readbacks = 1;
                    let readback = unsafe { args.Handled(&mut handled) };
                    facts.handled_readback_succeeded = readback.is_ok();
                    facts.native_handled_true = readback.is_ok() && handled.as_bool();
                    drop(readback);
                    timely &= self.popup_observe(event, true, true);
                }
                if action_ok && timely && facts.native_handled_true {
                    let uri = allocated_string(2048, |raw| unsafe { args.Uri(raw) });
                    facts.target_uri_getter_succeeded = uri.is_ok();
                    facts.target_uri_matched = uri.as_ref().is_ok_and(|s| s == "about:blank");
                    drop(uri);
                    timely &= self.popup_observe(event, true, true);
                }
                if action_ok && timely && facts.target_uri_matched {
                    let mut user = BOOL::from(true);
                    let gesture = unsafe { args.IsUserInitiated(&mut user) };
                    facts.non_user_getter_succeeded = gesture.is_ok();
                    facts.non_user = gesture.is_ok() && !user.as_bool();
                    drop(gesture);
                    timely &= self.popup_observe(event, true, true);
                }
                facts.live_after_calls = timely;
                if facts.handled_readbacks == 1
                    && (!facts.handled_readback_succeeded || !facts.native_handled_true)
                {
                    self.unknown.store(true, Ordering::SeqCst);
                }
                if action_ok {
                    Ok(true)
                } else {
                    Err(rejection())
                }
            }
            fn publish_popup_target(&self, target: NativePopupFacts, timely: bool) {
                let Some(owner) = self.owner.upgrade() else {
                    return;
                };
                {
                    let mut state = owner.borrow_mut();
                    let Some(i) =
                        matching(&mut state, &self.request).filter(|i| self.matches_instance(i))
                    else {
                        self.unknown.store(true, Ordering::SeqCst);
                        return;
                    };
                    let Some(p) = i.refusal_registry.popup.as_mut() else {
                        return;
                    };
                    // Never overwrite invocation/completion or duplicate counters
                    // with the callback's earlier local snapshot.
                    p.facts.handled_attempts = p
                        .facts
                        .handled_attempts
                        .saturating_add(target.handled_attempts)
                        .min(2);
                    p.facts.handled_returns = p
                        .facts
                        .handled_returns
                        .saturating_add(target.handled_returns)
                        .min(2);
                    p.facts.handled_successes = p
                        .facts
                        .handled_successes
                        .saturating_add(target.handled_successes)
                        .min(2);
                    p.facts.handled_readbacks = p
                        .facts
                        .handled_readbacks
                        .saturating_add(target.handled_readbacks)
                        .min(2);
                    p.facts.popup_sender_matched = target.popup_sender_matched;
                    p.facts.request_generation_live_matched =
                        target.request_generation_live_matched;
                    p.facts.target_uri_getter_succeeded = target.target_uri_getter_succeeded;
                    p.facts.target_uri_matched = target.target_uri_matched;
                    p.facts.non_user_getter_succeeded = target.non_user_getter_succeeded;
                    p.facts.non_user = target.non_user;
                    p.facts.handled_readback_succeeded = target.handled_readback_succeeded;
                    p.facts.native_handled_true = target.native_handled_true;
                    p.facts.live_after_calls = target.live_after_calls;
                    p.facts.target_after_drop_timely = timely;
                }
                emit_popup(
                    &owner,
                    &self.request,
                    &self.unknown,
                    NativePopupStage::Refusal,
                );
            }
        }
        fn add_popup(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            index: usize,
            core: &ICoreWebView2,
        ) -> windows::core::Result<()> {
            let context = Context::new(owner, request);
            let handler = NewWindowRequestedEventHandler::create(Box::new(move |sender, args| {
                let active = Active::enter(&context.callbacks);
                if !callback_on_ui(context.ui, &context.callbacks) {
                    context.unknown.store(true, Ordering::SeqCst);
                    drop(args);
                    drop(sender);
                    drop(active);
                    return Err(rejection());
                }
                let selected = context.popup_selected();
                // Reserve before Context's fallible native IUnknown queries.
                let reserved = selected && context.reserve_popup_target();
                let mut event = LegacyRefusalEvent::new();
                let mut facts = NativePopupFacts::default();
                let result = context.invoke(sender.as_ref(), |eligible| {
                    let args = args.as_ref().ok_or_else(rejection)?;
                    if selected {
                        context.popup_opening(args, eligible && reserved, &mut event, &mut facts)
                    } else {
                        unsafe { args.SetHandled(true) }.map(|()| true)
                    }
                });
                let failed = result.is_err();
                drop(result);
                drop(args);
                drop(sender);
                let timely = !selected || context.popup_after_drop(&mut event);
                drop(active);
                let timely = timely && (!selected || context.popup_after_drop(&mut event));
                if selected {
                    context.publish_popup_target(facts, timely);
                }
                if failed { Err(rejection()) } else { Ok(()) }
            }));
            let mut token = 0;
            let result = unsafe { core.add_NewWindowRequested(&handler, &mut token) };
            {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, request).unwrap();
                if result.is_ok() {
                    i.refusal_registry.entries[index].token = Some(token);
                } else {
                    i.refusal_registry.unknown.store(true, Ordering::SeqCst);
                    i.failed = true;
                    i.closing = true;
                    i.cleanup_blocked = true;
                }
            }
            drop(handler);
            result?;
            require_live(owner, request)
        }

        fn rejection() -> windows::core::Error {
            // Empty ErrorInfo: construction and HRESULT conversion perform no
            // thread-error-info calls, including the off-STA callback path.
            windows::core::Error::from_hresult(E_FAIL)
        }
        fn live(owner: &Rc<RefCell<Owner>>, request: &Code0WindowRequest) -> bool {
            let state = owner.borrow();
            !state.unmatched
                && state.instance.as_ref().is_some_and(|i| {
                    &i.request == request
                        && !i.closing
                        && !i.failed
                        && !i.cleanup_blocked
                        && !i.released
                        && !i.inline_loaded
                        && i.policy.is_none()
                        && i.policy_mailbox.is_none()
                        && !i.callbacks.affinity_failed.load(Ordering::SeqCst)
                        && i.refusal_registry.started
                        && i.refusal_registry.generation == GENERATION
                        && !i.refusal_registry.unknown.load(Ordering::SeqCst)
                })
        }
        fn require_live(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
        ) -> windows::core::Result<()> {
            if live(owner, request) {
                Ok(())
            } else {
                Err(rejection())
            }
        }
        fn latch_unknown(owner: &Rc<RefCell<Owner>>) {
            let observation = {
                let mut state = owner.borrow_mut();
                if let Some(i) = state.instance.as_mut()
                    && i.refusal_registry.unknown.load(Ordering::SeqCst)
                {
                    i.failed = true;
                    i.closing = true;
                    i.cleanup_blocked = true;
                    (
                        mark_child_close(i).map(|at| (i.observer.clone(), at)),
                        mark_popup_close(i).map(|at| (i.observer.clone(), at)),
                    )
                } else {
                    (None, None)
                }
            };
            if let Some((Some(observer), at)) = observation.0 {
                observer(NativeSmokeObservation::ChildClosing { at });
            }
            if observation.1.is_some() {
                publish_popup_close(owner);
            }
        }

        struct Context {
            owner: Weak<RefCell<Owner>>,
            request: Code0WindowRequest,
            generation: u64,
            ui: ThreadId,
            callbacks: Arc<Callbacks>,
            unknown: Arc<AtomicBool>,
            _lifetime: HandlerLifetime,
        }
        impl Context {
            fn new(owner: &Rc<RefCell<Owner>>, request: &Code0WindowRequest) -> Self {
                let state = owner.borrow();
                let i = state.instance.as_ref().unwrap();
                let callbacks = Arc::clone(&i.callbacks);
                Self {
                    owner: Rc::downgrade(owner),
                    request: request.clone(),
                    generation: GENERATION,
                    ui: state.ui_thread,
                    unknown: Arc::clone(&i.refusal_registry.unknown),
                    _lifetime: HandlerLifetime::new(&callbacks),
                    callbacks,
                }
            }
            fn matches_instance(&self, i: &Instance) -> bool {
                i.request == self.request
                    && i.refusal_registry.generation == self.generation
                    && Arc::ptr_eq(&i.callbacks, &self.callbacks)
                    && Arc::ptr_eq(&i.refusal_registry.unknown, &self.unknown)
            }
            fn authorized(&self, owner: &Rc<RefCell<Owner>>) -> bool {
                let state = owner.borrow();
                !state.unmatched
                    && state.instance.as_ref().is_some_and(|i| {
                        self.matches_instance(i)
                            && !i.closing
                            && !i.failed
                            && !i.cleanup_blocked
                            && !i.released
                            && !i.inline_loaded
                            && i.policy.is_none()
                            && i.policy_mailbox.is_none()
                            && !i.callbacks.affinity_failed.load(Ordering::SeqCst)
                            && i.refusal_registry.complete()
                    })
            }
            fn legacy_selected(&self) -> bool {
                self.owner.upgrade().is_some_and(|owner| {
                    owner.borrow().instance.as_ref().is_some_and(|i| {
                        self.matches_instance(i) && i.refusal_registry.legacy.is_some()
                    })
                })
            }
            fn legacy_live(&self, owner: &Rc<RefCell<Owner>>, require_live: bool) -> bool {
                legacy_authorized(owner, &self.request, &self.unknown, require_live).is_some()
            }
            fn legacy_observe(&self, event: &mut LegacyRefusalEvent, require_live: bool) -> bool {
                let Some(owner) = self.owner.upgrade() else {
                    return false;
                };
                let Some(deadline) = legacy_deadline(&owner, &self.request, &self.unknown) else {
                    return false;
                };
                let identity = self.legacy_live(&owner, require_live);
                event.observe_at(&deadline, Instant::now(), identity)
            }
            fn legacy_after_drop(&self, event: &mut LegacyRefusalEvent) -> bool {
                let Some(owner) = self.owner.upgrade() else {
                    return false;
                };
                let Some(deadline) = legacy_deadline(&owner, &self.request, &self.unknown) else {
                    return false;
                };
                let identity = self.legacy_live(&owner, false);
                event.qualify_at(&deadline, Instant::now(), identity)
            }
            fn legacy_expired(&self) -> bool {
                self.owner
                    .upgrade()
                    .and_then(|owner| legacy_deadline(&owner, &self.request, &self.unknown))
                    .is_some_and(|deadline| !deadline.before_original_at(Instant::now()))
            }
            fn invoke(
                &self,
                sender: Option<&ICoreWebView2>,
                mut action: impl FnMut(bool) -> windows::core::Result<bool>,
            ) -> windows::core::Result<()> {
                if !callback_on_ui(self.ui, &self.callbacks) {
                    self.unknown.store(true, Ordering::SeqCst);
                    return Err(rejection());
                }
                let owner = self.owner.upgrade();
                let binding = owner.as_ref().and_then(|owner| {
                    let state = owner.borrow();
                    state
                        .instance
                        .as_ref()
                        .filter(|i| self.matches_instance(i))
                        .and_then(|i| i.refusal_registry.core.as_ref())
                        .cloned() // Clones only the Rc wrapper, never COM under this borrow.
                });
                // Native identity queries occur after the Owner borrow ends.
                let identity = binding.as_ref().zip(sender).is_some_and(|(core, sender)| {
                    let expected = core.cast::<IUnknown>();
                    let actual = sender.cast::<IUnknown>();
                    expected
                        .ok()
                        .zip(actual.ok())
                        .is_some_and(|(a, b)| a.as_raw() == b.as_raw())
                });
                // Both native identity queries may reenter. Authority is read
                // again only after those calls and their temporary drops end.
                let eligible =
                    identity && owner.as_ref().is_some_and(|owner| self.authorized(owner));
                // Keep the original sender/live-at-entry observation truthful.
                // Selected legacy time authority is a separate revocation/witness
                // gate; an expired safely-STA event still receives its refusal.
                if self.legacy_selected()
                    && let Some(owner) = owner.as_ref()
                {
                    let _ = self.legacy_live(owner, true);
                }
                // Invalid/stale senders still receive the restrictive native
                // action on STA. They never authorize official navigation.
                let mut result = action(eligible);
                if result.as_ref().is_ok_and(|refused| !*refused)
                    && !owner.as_ref().is_some_and(|owner| {
                        self.authorized(owner)
                            && (!self.dialog_selected() || self.dialog_live(owner))
                            && (!self.legacy_selected() || self.legacy_live(owner, true))
                    })
                {
                    // Only an allowed navigation can return false. If its URI
                    // getter reentered/revoked, issue its first cancel now.
                    // Already-denied events never repeat either native setter.
                    result = action(false);
                }
                let same_installation = owner.as_ref().is_some_and(|owner| {
                    owner
                        .borrow()
                        .instance
                        .as_ref()
                        .is_some_and(|i| self.matches_instance(i))
                });
                if !identity || !same_installation || result.is_err() {
                    self.unknown.store(true, Ordering::SeqCst);
                }
                let mut top_close = None;
                let mut child_close = None;
                let mut dialog_close = None;
                let mut popup_close = None;
                if let Some(owner) = owner.as_ref() {
                    let mut state = owner.borrow_mut();
                    if let Some(i) = matching(&mut state, &self.request)
                        && self.matches_instance(i)
                    {
                        if self.unknown.load(Ordering::SeqCst) {
                            i.failed = true;
                            i.closing = true;
                            i.cleanup_blocked = true;
                        } else if result.as_ref().is_ok_and(|refused| *refused) {
                            // Existing poll delivers Failed(PolicyRejected),
                            // outside this borrow and without session reentry.
                            i.failed = true;
                            i.closing = true;
                        }
                        if i.closing {
                            top_close = mark_top_close(i).map(|at| (i.observer.clone(), at));
                            child_close = mark_child_close(i).map(|at| (i.observer.clone(), at));
                            dialog_close = mark_dialog_close(i).map(|at| (i.observer.clone(), at));
                            popup_close = mark_popup_close(i).map(|at| (i.observer.clone(), at));
                        }
                    }
                }
                if let Some((Some(observer), at)) = top_close {
                    observer(NativeSmokeObservation::TopClosing { at });
                }
                if let Some((Some(observer), at)) = child_close {
                    observer(NativeSmokeObservation::ChildClosing { at });
                }
                if let Some((Some(observer), at)) = dialog_close {
                    observer(NativeSmokeObservation::DialogClosing { at });
                }
                if popup_close.is_some()
                    && let Some(owner) = owner.as_ref()
                {
                    publish_popup_close(owner);
                }
                if !identity || !same_installation {
                    return Err(rejection());
                }
                result.map(|_| ())
            }
        }

        fn official(source: &str) -> bool {
            if source.bytes().any(|b| b <= 32 || b == 127 || b == b'\\') {
                return false;
            }
            let Some((_, tail)) = source.split_once("://") else {
                return false;
            };
            let authority = tail.split(['/', '?', '#']).next().unwrap_or_default();
            if authority.is_empty() || authority.contains('@') {
                return false;
            }
            let Ok(url) = reqwest::Url::parse(source) else {
                return false;
            };
            url.scheme() == "https"
                && url.host_str() == Some("console.code0.ai")
                && url.port_or_known_default() == Some(443)
                && url.username().is_empty()
                && url.password().is_none()
        }
        fn navigation(
            args: &ICoreWebView2NavigationStartingEventArgs,
            eligible: bool,
        ) -> windows::core::Result<bool> {
            let uri = if eligible {
                allocated_string(2048, |raw| unsafe { args.Uri(raw) })
            } else {
                Ok(String::new())
            };
            if uri.as_ref().is_ok_and(|uri| official(uri)) {
                return Ok(false);
            }
            let canceled = unsafe { args.SetCancel(true) };
            // Getter failure must not bypass the actual cancel attempt.
            uri?;
            canceled?;
            Ok(true)
        }

        /// N1 accepts only this owned fixed empty leaf, after the independent
        /// complete native inventory receipt. No URL or page input is accepted.
        pub(super) fn navigate_top_owned_file(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            owned_root: &Path,
            deadline: NativeRefusalDeadline,
        ) -> Result<super::super::fixture::NativeTopSubmissionReceipt, Code0SessionFailure>
        {
            use std::path::{Component, Prefix};
            let denied = || Code0SessionFailure::PolicyRejected;
            let installation = bind_legacy(owner, request, deadline, LegacyKind::Top)?;
            let root =
                legacy_setup_step(owner, request, &installation, || owned_root.canonicalize())?;
            let fixed = root.join("code0-s2-top-denied.txt");
            let target = legacy_setup_step(owner, request, &installation, || fixed.canonicalize())?;
            let local = |path: &Path| {
                path.is_absolute()
                    && matches!(
                        path.components().next(),
                        Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
                    )
            };
            if !local(&root)
                || !local(&target)
                || target != fixed
                || target.parent() != Some(root.as_path())
                || target.file_name() != Some(std::ffi::OsStr::new("code0-s2-top-denied.txt"))
            {
                return Err(denied());
            }
            let metadata = legacy_setup_step(owner, request, &installation, || target.metadata())?;
            if !metadata.is_file() || metadata.len() != 0 {
                return Err(denied());
            }
            let url = legacy_setup_step(owner, request, &installation, || {
                reqwest::Url::from_file_path(&target)
            })?;
            if url.scheme() != "file"
                || url.host_str().is_some()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
                || url.as_str().is_empty()
                || url.as_str().encode_utf16().count() > 2048
            {
                return Err(denied());
            }
            let target = url.to_string();
            let wide: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
            let (core, callbacks) = {
                let mut state = owner.borrow_mut();
                if state.unmatched {
                    return Err(denied());
                }
                let i = matching(&mut state, request).ok_or(Code0SessionFailure::PolicyRejected)?;
                if i.closing
                    || i.failed
                    || i.cleanup_blocked
                    || i.released
                    || i.inline_loaded
                    || i.policy.is_some()
                    || i.policy_mailbox.is_some()
                    || i.callbacks.affinity_failed.load(Ordering::SeqCst)
                    || !i.refusal_registry.complete()
                    || i.refusal_registry.top_target.is_some()
                    || i.refusal_registry.child.is_some()
                    || i.refusal_registry.dialog.is_some()
                {
                    return Err(denied());
                }
                let core = i
                    .refusal_registry
                    .core
                    .as_ref()
                    .map(Rc::clone)
                    .ok_or(Code0SessionFailure::PolicyRejected)?;
                (core, Arc::clone(&i.callbacks))
            };
            let active = Active::enter(&callbacks);
            let result = legacy_step(owner, request, &installation, || {
                // Only this last original-clock grant reserves the one Navigate.
                // The Owner borrow ends before the native call or any reentry.
                owner
                    .borrow_mut()
                    .instance
                    .as_mut()
                    .unwrap()
                    .refusal_registry
                    .top_target = Some(target);
                unsafe { core.Navigate(PCWSTR(wide.as_ptr())) }
            });
            let native_submit_ok = result.as_ref().is_some_and(|r| r.is_ok());
            let submitted = result.is_some();
            // Any HRESULT ErrorInfo and the action-local interface wrapper end
            // while the callback inventory still protects this native action.
            drop(result);
            let _ = legacy_authorized(owner, request, &installation, false);
            drop(core);
            let _ = legacy_authorized(owner, request, &installation, false);
            drop(active);
            let original_timely = legacy_authorized(owner, request, &installation, false).is_some();
            if !submitted {
                return Err(denied());
            }
            Ok(super::super::fixture::NativeTopSubmissionReceipt {
                request: request.clone(),
                installation,
                native_submit_ok,
                original_timely,
            })
        }

        impl Context {
            fn top_navigation(
                &self,
                args: &ICoreWebView2NavigationStartingEventArgs,
                eligible: bool,
                event: &mut LegacyRefusalEvent,
                receipt: &mut Option<super::super::fixture::NativeTopRefusalReceipt>,
            ) -> windows::core::Result<bool> {
                if self.dialog_selected() {
                    return self.dialog_navigation(args, eligible);
                }
                let owner = self.owner.upgrade();
                let target = owner.as_ref().and_then(|owner| {
                    let state = owner.borrow();
                    state
                        .instance
                        .as_ref()
                        .filter(|i| self.matches_instance(i))
                        .and_then(|i| i.refusal_registry.top_target.clone())
                });
                let Some(target) = target else {
                    return navigation(args, eligible);
                };
                let _ = self.legacy_observe(event, true);
                // All metadata is fallible and may reenter. None of its errors
                // can bypass the original restrictive native SetCancel action.
                let uri = allocated_string(2048, |raw| unsafe { args.Uri(raw) });
                let _ = self.legacy_observe(event, true);
                let mut navigation_id = 0u64;
                let id_result = unsafe { args.NavigationId(&mut navigation_id) };
                let _ = self.legacy_observe(event, true);
                let mut redirected = BOOL::from(false);
                let redirected_result = unsafe { args.IsRedirected(&mut redirected) };
                let metadata_timely = self.legacy_observe(event, true);
                let live_after_metadata = metadata_timely
                    && eligible
                    && owner.as_ref().is_some_and(|owner| self.authorized(owner));
                let canceled = event
                    .cancel_once(|| unsafe { args.SetCancel(true) })
                    .ok_or_else(rejection)?;
                let _ = self.legacy_observe(event, false);
                let mut cancel = BOOL::from(false);
                let readback = unsafe { args.Cancel(&mut cancel) };
                let readback_timely = self.legacy_observe(event, false);
                let live_after_getters = readback_timely
                    && live_after_metadata
                    && owner.as_ref().is_some_and(|owner| self.authorized(owner));
                let facts = super::super::fixture::NativeTopRefusalFacts {
                    event_kind: "topNavigationStarting",
                    action_kind: "setCancelTrue",
                    target_uri_matched: uri.as_ref().is_ok_and(|uri| uri == &target),
                    native_sender_matched: eligible,
                    request_generation_live_matched: live_after_getters,
                    navigation_id_getter_succeeded: id_result.is_ok(),
                    redirected_getter_succeeded: redirected_result.is_ok(),
                    is_redirected: redirected.as_bool(),
                    issued_before_event: true,
                    cancel_attempts: 1,
                    cancel_returns: 1,
                    cancel_successes: u8::from(canceled.is_ok()),
                    cancel_readbacks: 1,
                    cancel_readback_succeeded: readback.is_ok(),
                    native_cancel_true: readback.is_ok() && cancel.as_bool(),
                    live_after_getters,
                    native_action_before_receipt: true,
                };
                *receipt = Some(super::super::fixture::NativeTopRefusalReceipt {
                    request: self.request.clone(),
                    installation: Arc::clone(&self.unknown),
                    facts,
                });
                uri?;
                id_result?;
                redirected_result?;
                canceled?;
                readback?;
                if !cancel.as_bool() || (!live_after_getters && !self.legacy_expired()) {
                    return Err(rejection());
                }
                Ok(true)
            }
        }

        impl Context {
            fn child_navigation(
                &self,
                args: &ICoreWebView2NavigationStartingEventArgs,
                eligible: bool,
                event: &mut LegacyRefusalEvent,
                receipt: &mut Option<super::super::fixture::NativeChildRefusalReceipt>,
            ) -> windows::core::Result<bool> {
                if self.dialog_selected() {
                    unsafe { args.SetCancel(true) }?;
                    return Ok(true);
                }
                let owner = self.owner.upgrade();
                let armed = owner.as_ref().is_some_and(|owner| {
                    owner.borrow().instance.as_ref().is_some_and(|i| {
                        self.matches_instance(i) && i.refusal_registry.child.is_some()
                    })
                });
                if !armed {
                    return navigation(args, eligible);
                }
                let target = CHILD_TARGET;
                let _ = self.legacy_observe(event, true);
                // All metadata is fallible and may reenter. None of its errors
                // can bypass the original restrictive native SetCancel action.
                let uri = allocated_string(2048, |raw| unsafe { args.Uri(raw) });
                let _ = self.legacy_observe(event, true);
                if uri.as_ref().is_ok_and(|uri| uri != target) {
                    // An earlier blank/other child keeps the original delegate.
                    // It cannot be relabeled as this target or delayed for it.
                    return navigation(args, eligible);
                }
                let mut navigation_id = 0u64;
                let id_result = unsafe { args.NavigationId(&mut navigation_id) };
                let _ = self.legacy_observe(event, true);
                let mut redirected = BOOL::from(false);
                let redirected_result = unsafe { args.IsRedirected(&mut redirected) };
                let metadata_timely = self.legacy_observe(event, true);
                let live_after_metadata = metadata_timely
                    && eligible
                    && owner.as_ref().is_some_and(|owner| self.authorized(owner));
                let canceled = event
                    .cancel_once(|| unsafe { args.SetCancel(true) })
                    .ok_or_else(rejection)?;
                let _ = self.legacy_observe(event, false);
                let mut cancel = BOOL::from(false);
                let readback = unsafe { args.Cancel(&mut cancel) };
                let readback_timely = self.legacy_observe(event, false);
                let live_after_getters = readback_timely
                    && live_after_metadata
                    && owner.as_ref().is_some_and(|owner| self.authorized(owner));
                let facts = super::super::fixture::NativeChildRefusalFacts {
                    event_kind: "frameNavigationStarting",
                    action_kind: "setCancelTrue",
                    target_uri_matched: uri.as_ref().is_ok_and(|uri| uri == target),
                    native_sender_matched: eligible,
                    request_generation_live_matched: live_after_getters,
                    navigation_id_getter_succeeded: id_result.is_ok(),
                    redirected_getter_succeeded: redirected_result.is_ok(),
                    is_redirected: redirected.as_bool(),
                    issued_before_event: true,
                    cancel_attempts: 1,
                    cancel_returns: 1,
                    cancel_successes: u8::from(canceled.is_ok()),
                    cancel_readbacks: 1,
                    cancel_readback_succeeded: readback.is_ok(),
                    native_cancel_true: readback.is_ok() && cancel.as_bool(),
                    live_after_getters,
                    native_action_before_receipt: true,
                };
                if facts.target_uri_matched {
                    *receipt = Some(super::super::fixture::NativeChildRefusalReceipt {
                        request: self.request.clone(),
                        installation: Arc::clone(&self.unknown),
                        facts,
                    });
                }
                uri?;
                id_result?;
                redirected_result?;
                canceled?;
                readback?;
                if !cancel.as_bool() || (!live_after_getters && !self.legacy_expired()) {
                    return Err(rejection());
                }
                Ok(true)
            }
        }

        fn add_top_navigation(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            index: usize,
            core: &ICoreWebView2,
        ) -> windows::core::Result<()> {
            let context = Context::new(owner, request);
            let handler = NavigationStartingEventHandler::create(Box::new(move |sender, args| {
                let active = Active::enter(&context.callbacks);
                let mut receipt = None;
                let mut event = LegacyRefusalEvent::new();
                let mut dialog_selected = false;
                let result = context.invoke(sender.as_ref(), |eligible| {
                    let args = args.as_ref().ok_or_else(rejection)?;
                    dialog_selected = context.dialog_selected();
                    context.top_navigation(args, eligible, &mut event, &mut receipt)
                });
                drop(args);
                drop(sender);
                if dialog_selected {
                    let _ = context.dialog_after_drop();
                }
                if let Some(mut receipt) = receipt {
                    let timely = context.legacy_after_drop(&mut event);
                    receipt.facts.live_after_getters &= timely;
                    receipt.facts.request_generation_live_matched &= timely;
                    let observer = context.owner.upgrade().and_then(|owner| {
                        let state = owner.borrow();
                        state
                            .instance
                            .as_ref()
                            .filter(|i| context.matches_instance(i))
                            .and_then(|i| i.observer.clone())
                    });
                    if let Some(observer) = observer {
                        observer(NativeSmokeObservation::TopRefusal(receipt));
                    }
                }
                drop(active);
                result
            }));
            let mut token = 0;
            let result = unsafe { core.add_NavigationStarting(&handler, &mut token) };
            {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, request).unwrap();
                if result.is_ok() {
                    i.refusal_registry.entries[index].token = Some(token);
                } else {
                    i.refusal_registry.unknown.store(true, Ordering::SeqCst);
                    i.failed = true;
                    i.closing = true;
                    i.cleanup_blocked = true;
                }
            }
            drop(handler);
            result?;
            require_live(owner, request)
        }

        fn add_child_navigation(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            index: usize,
            core: &ICoreWebView2,
        ) -> windows::core::Result<()> {
            let context = Context::new(owner, request);
            let handler = NavigationStartingEventHandler::create(Box::new(move |sender, args| {
                let active = Active::enter(&context.callbacks);
                let mut receipt = None;
                let mut event = LegacyRefusalEvent::new();
                let result = context.invoke(sender.as_ref(), |eligible| {
                    let args = args.as_ref().ok_or_else(rejection)?;
                    context.child_navigation(args, eligible, &mut event, &mut receipt)
                });
                drop(args);
                drop(sender);
                if let Some(mut receipt) = receipt {
                    let timely = context.legacy_after_drop(&mut event);
                    receipt.facts.live_after_getters &= timely;
                    receipt.facts.request_generation_live_matched &= timely;
                    let observer = context.owner.upgrade().and_then(|owner| {
                        let state = owner.borrow();
                        state
                            .instance
                            .as_ref()
                            .filter(|i| context.matches_instance(i))
                            .and_then(|i| i.observer.clone())
                    });
                    if let Some(observer) = observer {
                        observer(NativeSmokeObservation::ChildRefusal(receipt));
                    }
                }
                drop(active);
                result
            }));
            let mut token = 0;
            let result = unsafe { core.add_FrameNavigationStarting(&handler, &mut token) };
            {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, request).unwrap();
                if result.is_ok() {
                    i.refusal_registry.entries[index].token = Some(token);
                } else {
                    i.refusal_registry.unknown.store(true, Ordering::SeqCst);
                    i.failed = true;
                    i.closing = true;
                    i.cleanup_blocked = true;
                }
            }
            drop(handler);
            result?;
            require_live(owner, request)
        }

        macro_rules! add_handler {
            ($owner:expr, $request:expr, $index:expr, $emitter:expr, $wrapper:ident, $add:ident, $action:expr) => {{
                let context = Context::new($owner, $request);
                let action = $action;
                let handler = $wrapper::create(Box::new(move |sender, args| {
                    let active = Active::enter(&context.callbacks);
                    let result = context.invoke(sender.as_ref(), |eligible| {
                        let args = args.as_ref().ok_or_else(rejection)?;
                        action(args, eligible)
                    });
                    drop(args);
                    drop(sender);
                    drop(active);
                    result
                }));
                let mut token = 0;
                let result = unsafe { $emitter.$add(&handler, &mut token) };
                {
                    let mut state = $owner.borrow_mut();
                    let i = matching(&mut state, $request).unwrap();
                    if result.is_ok() {
                        // Preserve every actual success before reentry checks.
                        i.refusal_registry.entries[$index].token = Some(token);
                    } else {
                        // A failed add cannot prove no handler was retained.
                        i.refusal_registry.unknown.store(true, Ordering::SeqCst);
                        i.failed = true;
                        i.closing = true;
                        i.cleanup_blocked = true;
                    }
                }
                drop(handler);
                result?;
                require_live($owner, $request)
            }};
        }
        fn register(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            hook: Hook,
            emitter: Rc<Emitter>,
        ) -> windows::core::Result<()> {
            require_live(owner, request)?;
            let index = {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, request).unwrap();
                let index = i.refusal_registry.entries.len();
                if index >= Hook::ALL.len() {
                    return Err(rejection());
                }
                i.refusal_registry.entries.push(Registration {
                    hook,
                    emitter: Rc::clone(&emitter),
                    token: None,
                    remove_attempted: false,
                });
                index
            };
            match (hook, emitter.as_ref()) {
                (Hook::TopNavigation, Emitter::Core(core)) => {
                    add_top_navigation(owner, request, index, core)
                }
                (Hook::ChildNavigation, Emitter::Core(core)) => {
                    add_child_navigation(owner, request, index, core)
                }
                (Hook::ExternalProtocol, Emitter::External(core)) => add_handler!(
                    owner,
                    request,
                    index,
                    core,
                    LaunchingExternalUriSchemeEventHandler,
                    add_LaunchingExternalUriScheme,
                    |args: &ICoreWebView2LaunchingExternalUriSchemeEventArgs, _| unsafe {
                        args.SetCancel(true)
                    }
                    .map(|()| true)
                ),
                (Hook::NewWindow, Emitter::Core(core)) => add_popup(owner, request, index, core),
                (Hook::Download, Emitter::Download(core)) => add_handler!(
                    owner,
                    request,
                    index,
                    core,
                    DownloadStartingEventHandler,
                    add_DownloadStarting,
                    |args: &ICoreWebView2DownloadStartingEventArgs, _| {
                        let canceled = unsafe { args.SetCancel(true) };
                        let handled = unsafe { args.SetHandled(true) };
                        canceled?;
                        handled?;
                        Ok(true)
                    }
                ),
                (Hook::ScriptDialog, Emitter::Core(core)) => {
                    add_script_dialog(owner, request, index, core)
                }
                (Hook::Permission, Emitter::Core(core)) => add_handler!(
                    owner,
                    request,
                    index,
                    core,
                    PermissionRequestedEventHandler,
                    add_PermissionRequested,
                    |args: &ICoreWebView2PermissionRequestedEventArgs, _| unsafe {
                        args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY)
                    }
                    .map(|()| true)
                ),
                (Hook::BasicAuthentication, Emitter::Authentication(core)) => add_handler!(
                    owner,
                    request,
                    index,
                    core,
                    BasicAuthenticationRequestedEventHandler,
                    add_BasicAuthenticationRequested,
                    |args: &ICoreWebView2BasicAuthenticationRequestedEventArgs, _| unsafe {
                        args.SetCancel(true)
                    }
                    .map(|()| true)
                ),
                (Hook::ClientCertificate, Emitter::Certificate(core)) => add_handler!(
                    owner,
                    request,
                    index,
                    core,
                    ClientCertificateRequestedEventHandler,
                    add_ClientCertificateRequested,
                    |args: &ICoreWebView2ClientCertificateRequestedEventArgs, _| {
                        let canceled = unsafe { args.SetCancel(true) };
                        let handled = unsafe { args.SetHandled(true) };
                        canceled?;
                        handled?;
                        Ok(true)
                    }
                ),
                (Hook::TlsError, Emitter::Tls(core)) => add_handler!(
                    owner,
                    request,
                    index,
                    core,
                    ServerCertificateErrorDetectedEventHandler,
                    add_ServerCertificateErrorDetected,
                    |args: &ICoreWebView2ServerCertificateErrorDetectedEventArgs, _| unsafe {
                        args.SetAction(COREWEBVIEW2_SERVER_CERTIFICATE_ERROR_ACTION_CANCEL)
                    }
                    .map(|()| true)
                ),
                _ => Err(rejection()),
            }
        }

        pub(super) fn begin(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
        ) -> Result<(), Code0SessionFailure> {
            // Reuse the exact read-only unloaded startup validation first.
            super::require_native_refusal_install(owner, request)?;
            let callbacks = {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, request).unwrap();
                if i.refusal_registry.started {
                    return Err(Code0SessionFailure::PolicyRejected);
                }
                i.refusal_registry.started = true;
                i.refusal_registry.generation = GENERATION;
                Arc::clone(&i.callbacks)
            };
            let _action = Active::enter(&callbacks);
            // Native AddRef may reenter: move originals before cloning and
            // restore them under Active, exactly as the existing P1 installer.
            let original = matching(&mut owner.borrow_mut(), request)
                .unwrap()
                .core
                .take()
                .unwrap();
            let core = Rc::new(original.clone());
            {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, request).unwrap();
                i.core = Some(original);
                i.refusal_registry.core = Some(Rc::clone(&core));
            }
            let result = (|| -> windows::core::Result<()> {
                require_live(owner, request)?;
                let settings = Rc::new(unsafe { core.Settings() }?);
                matching(&mut owner.borrow_mut(), request)
                    .unwrap()
                    .refusal_registry
                    .settings = Some(Rc::clone(&settings));
                require_live(owner, request)?;
                unsafe { settings.SetAreDefaultScriptDialogsEnabled(false) }?;
                require_live(owner, request)?;
                let mut actual = BOOL::default();
                unsafe { settings.AreDefaultScriptDialogsEnabled(&mut actual) }?;
                require_live(owner, request)?;
                if actual.as_bool() {
                    return Err(rejection());
                }
                matching(&mut owner.borrow_mut(), request)
                    .unwrap()
                    .refusal_registry
                    .settings_acknowledged = true;
                for hook in Hook::ALL {
                    require_live(owner, request)?;
                    let emitter = match hook {
                        Hook::ExternalProtocol => {
                            Emitter::External(Rc::new(core.cast::<ICoreWebView2_18>()?))
                        }
                        Hook::Download => {
                            Emitter::Download(Rc::new(core.cast::<ICoreWebView2_4>()?))
                        }
                        Hook::BasicAuthentication => {
                            Emitter::Authentication(Rc::new(core.cast::<ICoreWebView2_10>()?))
                        }
                        Hook::ClientCertificate => {
                            Emitter::Certificate(Rc::new(core.cast::<ICoreWebView2_5>()?))
                        }
                        Hook::TlsError => Emitter::Tls(Rc::new(core.cast::<ICoreWebView2_14>()?)),
                        _ => Emitter::Core(Rc::clone(&core)),
                    };
                    register(owner, request, hook, Rc::new(emitter))?;
                }
                require_live(owner, request)?;
                matching(&mut owner.borrow_mut(), request)
                    .unwrap()
                    .refusal_registry
                    .completed = true;
                Ok(())
            })();
            result.map_err(|_| {
                failed(owner, request);
                Code0SessionFailure::PolicyRejected
            })
        }

        pub(super) fn registrations_removed(i: &Instance) -> bool {
            !i.refusal_registry.unknown.load(Ordering::SeqCst)
                && i.refusal_registry.entries.iter().all(|e| e.token.is_none())
                && i.refusal_registry
                    .dialog_completed
                    .as_ref()
                    .is_none_or(|e| e.token.is_none())
        }
        pub(super) fn remove(owner: &Rc<RefCell<Owner>>) {
            remove_dialog_completed(owner);
            loop {
                let action = {
                    let mut state = owner.borrow_mut();
                    let i = state.instance.as_mut().unwrap();
                    i.refusal_registry
                        .entries
                        .iter_mut()
                        .enumerate()
                        .find_map(|(index, e)| {
                            if e.remove_attempted {
                                return None;
                            }
                            let token = e.token?;
                            e.remove_attempted = true;
                            Some((index, e.hook, Rc::clone(&e.emitter), token))
                        })
                };
                let Some((index, hook, emitter, token)) = action else {
                    break;
                };
                let result = unsafe {
                    match (hook, emitter.as_ref()) {
                        (Hook::TopNavigation, Emitter::Core(core)) => {
                            core.remove_NavigationStarting(token)
                        }
                        (Hook::ChildNavigation, Emitter::Core(core)) => {
                            core.remove_FrameNavigationStarting(token)
                        }
                        (Hook::ExternalProtocol, Emitter::External(core)) => {
                            core.remove_LaunchingExternalUriScheme(token)
                        }
                        (Hook::NewWindow, Emitter::Core(core)) => {
                            core.remove_NewWindowRequested(token)
                        }
                        (Hook::Download, Emitter::Download(core)) => {
                            core.remove_DownloadStarting(token)
                        }
                        (Hook::ScriptDialog, Emitter::Core(core)) => {
                            core.remove_ScriptDialogOpening(token)
                        }
                        (Hook::Permission, Emitter::Core(core)) => {
                            core.remove_PermissionRequested(token)
                        }
                        (Hook::BasicAuthentication, Emitter::Authentication(core)) => {
                            core.remove_BasicAuthenticationRequested(token)
                        }
                        (Hook::ClientCertificate, Emitter::Certificate(core)) => {
                            core.remove_ClientCertificateRequested(token)
                        }
                        (Hook::TlsError, Emitter::Tls(core)) => {
                            core.remove_ServerCertificateErrorDetected(token)
                        }
                        _ => Err(rejection()),
                    }
                };
                {
                    let mut state = owner.borrow_mut();
                    let i = state.instance.as_mut().unwrap();
                    if result.is_ok() {
                        i.refusal_registry.entries[index].token = None;
                    } else {
                        i.refusal_registry.unknown.store(true, Ordering::SeqCst);
                    }
                }
                latch_unknown(owner);
            }
        }
        pub(super) fn take_controls(i: &mut Instance) -> Controls {
            Controls {
                _core: i.refusal_registry.core.take(),
                _settings: i.refusal_registry.settings.take(),
                _entries: std::mem::take(&mut i.refusal_registry.entries),
                _dialog_completed: i.refusal_registry.dialog_completed.take(),
            }
        }
        pub(super) fn after_control_drop(owner: &Rc<RefCell<Owner>>) -> bool {
            latch_unknown(owner);
            !owner
                .borrow()
                .instance
                .as_ref()
                .unwrap()
                .refusal_registry
                .unknown
                .load(Ordering::SeqCst)
        }
    }

    /// Finite debug-only P1. No production policy authority is derived here.
    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    mod p1 {
        use super::super::fixture::{
            ContainmentReceipt, ControlledDocumentReceipt, ControlledLoadReceipt,
            InstallBarrierReceipt, PolicyFaultReceipt, PolicyFaultTarget, PolicyFixtureCase,
            PolicyObservation, StaleDispositionReceipt, StaleDispositionStage,
        };
        use super::*;
        use crate::collectors::code0_bridge::{
            CODE0_BRIDGE_SOURCE, Code0BridgeContext, Code0BridgeEvent, Code0BridgeValidator,
        };
        use std::sync::Mutex;
        use webview2_com::{
            DevToolsProtocolEventReceivedEventHandler, WebResourceRequestedEventHandler,
        };
        use windows::Win32::Foundation::E_OUTOFMEMORY;
        use windows::Win32::System::Com::{IStream, STREAM_SEEK_SET};
        use windows::Win32::UI::Shell::SHCreateMemStream;
        use windows::core::{BOOL, w};

        const GENERATION: u64 = 1;
        const URI: &str = "https://console.code0.ai/__ai_meter_fixture__/install-p1";
        const MAX_REQUESTS: u32 = 16;
        const MAIN: &[u8] = br#"<!doctype html><meta charset="utf-8"><title>Code0 P1</title><script nonce="code0-p1-fixed">document.addEventListener('DOMContentLoaded',()=>{window.chrome.webview.postMessage({kind:'code0InstallDocumentComplete',value:1});},{once:true});</script>"#;
        const REFUSAL: &[u8] = b"Code0 P1 synthetic refusal";
        const HEADERS: &str = "Content-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'; script-src 'nonce-code0-p1-fixed'; connect-src 'none'; img-src 'none'; style-src 'none'; font-src 'none'; media-src 'none'; object-src 'none'; frame-src 'none'; worker-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'\r\n";

        const P8_MAIN_URI: &str = "https://console.code0.ai/__ai_meter_fixture__/containment-main";
        const P8_CHILD_URI: &str =
            "https://console.code0.ai/__ai_meter_fixture__/containment-child";
        const P8_WORKER_URI: &str =
            "https://console.code0.ai/__ai_meter_fixture__/containment-worker.js";
        const P8_PROBES: [&str; 3] = [
            "https://console.code0.ai/__ai_meter_fixture__/probe-main",
            "https://console.code0.ai/__ai_meter_fixture__/probe-child",
            "https://console.code0.ai/__ai_meter_fixture__/probe-worker",
        ];
        // Fixed execution-witness nonce, never the quota bridge configuration.
        const P8_NONCE: &str = "code0-p8-witness-v1";
        const P8_REFUSAL: &[u8] = b"Code0 P8 synthetic refusal";
        const P8_MAIN: &[u8] = br#"<!doctype html><meta charset="utf-8"><title>Code0 containment</title><body><script nonce="code0-p8-fixed">document.addEventListener('DOMContentLoaded',()=>{const nonce='code0-p8-witness-v1';let main=false,child=false,worker=false,done=false,invalid=false;const frame=document.createElement('iframe');const task=new Worker('/__ai_meter_fixture__/containment-worker.js');const emit=(detached,terminated)=>window.chrome.webview.postMessage({kind:'code0P8Complete',nonce,main,child,worker,childDetached:detached,workerTerminateReturned:terminated});const finish=()=>{if(!done&&main&&child&&worker){done=true;frame.remove();const detached=!frame.isConnected;task.terminate();emit(detached,true);}};const valid=(v,kind)=>v!==null&&typeof v==='object'&&!Array.isArray(v)&&Object.keys(v).length===4&&v.kind===kind&&v.nonce===nonce&&v.status===403&&v.body==='Code0 P8 synthetic refusal';const bad=()=>{if(!invalid){invalid=true;done=true;emit(false,false);}};window.addEventListener('message',e=>{if(e.source!==frame.contentWindow||e.origin!=='https://console.code0.ai'){bad();return;}if(child||!valid(e.data,'child')){bad();return;}child=true;finish();});task.addEventListener('message',e=>{if(e.target!==task||worker||!valid(e.data,'worker')){bad();return;}worker=true;finish();});task.addEventListener('error',bad);frame.src='/__ai_meter_fixture__/containment-child';document.body.appendChild(frame);fetch('/__ai_meter_fixture__/probe-main').then(async r=>{main=r.status===403&&(await r.text())==='Code0 P8 synthetic refusal';if(!main){bad();return;}finish();}).catch(bad);},{once:true});</script>"#;
        const P8_CHILD: &[u8] = br#"<!doctype html><meta charset="utf-8"><title>Code0 child</title><script nonce="code0-p8-fixed">document.addEventListener('DOMContentLoaded',()=>{fetch('/__ai_meter_fixture__/probe-child').then(async r=>parent.postMessage({kind:'child',nonce:'code0-p8-witness-v1',status:r.status,body:await r.text()},'https://console.code0.ai'));},{once:true});</script>"#;
        const P8_WORKER: &[u8] = br#"fetch('/__ai_meter_fixture__/probe-worker').then(async r=>postMessage({kind:'worker',nonce:'code0-p8-witness-v1',status:r.status,body:await r.text()}));"#;
        const P8_MAIN_HEADERS: &str = "Content-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'; script-src 'nonce-code0-p8-fixed'; frame-src 'self'; worker-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'\r\n";
        const P8_CHILD_HEADERS: &str = "Content-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'; script-src 'nonce-code0-p8-fixed'; connect-src 'self'; frame-ancestors 'self'; base-uri 'none'; form-action 'none'\r\n";
        const P8_WORKER_HEADERS: &str = "Content-Type: application/javascript; charset=utf-8\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'; connect-src 'self'\r\n";
        const P8_REFUSAL_HEADERS: &str = "Content-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'\r\n";

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct ContainmentWitness {
            kind: String,
            nonce: String,
            main: bool,
            child: bool,
            worker: bool,
            child_detached: bool,
            worker_terminate_returned: bool,
        }

        #[derive(Clone, Copy)]
        enum ResponseKind {
            Main,
            Refusal,
            P8Main,
            P8Child,
            P8Worker,
            P8Probe(usize),
            P8Other,
        }
        impl ResponseKind {
            fn allowed(self) -> bool {
                matches!(
                    self,
                    Self::Main | Self::P8Main | Self::P8Child | Self::P8Worker
                )
            }
            fn refusal(self) -> Self {
                if matches!(self, Self::Main | Self::Refusal) {
                    Self::Refusal
                } else {
                    Self::P8Other
                }
            }
            fn diagnostic_lane(self) -> u8 {
                match self {
                    Self::Main => 8,
                    Self::Refusal => 9,
                    _ => self.p8_index() as u8 + 1,
                }
            }
            fn p8_index(self) -> usize {
                match self {
                    Self::P8Main => 0,
                    Self::P8Child => 1,
                    Self::P8Worker => 2,
                    Self::P8Probe(index) => 3 + index,
                    _ => 6,
                }
            }
        }

        // Call-local primitives only. Publish at uncertain_response's actual
        // sticky assignment, never at a metadata observation that can still unwind.
        struct ResourceTrace {
            detail: DU,
        }
        impl ResourceTrace {
            fn new() -> Self {
                Self {
                    detail: DU::cleanup(US::ResourceResponse, UR::Condition, None),
                }
            }
            fn condition(&mut self, stage: UT, reason: UR) {
                self.detail.stage = stage;
                self.detail.reason = reason;
                self.detail.hresult = None;
            }
            fn observe_native<T>(&mut self, stage: UT, result: &windows::core::Result<T>) {
                if let Err(error) = result {
                    self.detail.stage = stage;
                    self.detail.reason = UR::NativeError;
                    self.detail.hresult = Some(error.code().0);
                }
            }
            fn native<T>(
                &mut self,
                stage: UT,
                result: windows::core::Result<T>,
            ) -> windows::core::Result<T> {
                self.observe_native(stage, &result);
                result
            }
            fn bounded(
                &mut self,
                stage: UT,
                result: windows::core::Result<String>,
            ) -> windows::core::Result<String> {
                if result.is_err() && self.detail.hresult.is_none() {
                    self.condition(stage, UR::InvalidString);
                }
                result
            }
        }

        fn diagnostic_lane(uri: &str) -> u8 {
            match uri {
                P8_MAIN_URI => 1,
                P8_CHILD_URI => 2,
                P8_WORKER_URI => 3,
                URI => 8,
                _ => P8_PROBES
                    .iter()
                    .position(|path| uri == *path)
                    .map_or(7, |index| 4 + index as u8),
            }
        }

        // Observation-only data remains on the existing Instance through native
        // teardown. No count participates in readiness, cleanup or release.
        #[derive(Default)]
        pub(super) struct FaultState {
            case: Option<PolicyFixtureCase>,
            // target, injection, premature denial, Navigate, script, CDP,
            // resource add, filter add, resource remove, filter remove, script remove.
            counts: [u32; 11],
            // Actual ID transfer, script ACK application, CDP ACK application.
            application: [bool; 3],
            final_sent: bool,
            stale_phase: StalePhase,
            stale_order: [u32; 4],
            stale_script: bool,
            containment: ContainmentState,
        }

        #[derive(Default, PartialEq, Eq)]
        enum StalePhase {
            #[default]
            Waiting,
            Held,
            CloseRevoked,
            Drained,
        }

        #[derive(Default)]
        struct ContainmentState {
            deliveries: [u32; 7],
            responses: [u32; 7],
            total: u32,
            renderer: [bool; 5],
            source_matched: bool,
            document: [bool; 3],
            final_sent: bool,
        }

        // Fixed successful native actions / actual call-attempt boundaries only.
        enum Count {
            PrematureDenial = 2,
            Navigate = 3,
            ScriptSubmission = 4,
            CdpSubmission = 5,
            ResourceAdd = 6,
            FilterAdd = 7,
            ResourceRemoval = 8,
            FilterRemoval = 9,
            ScriptRemoval = 10,
        }

        fn count(owner: &Rc<RefCell<Owner>>, operation: Count) {
            let mut state = owner.borrow_mut();
            let fault = &mut state.instance.as_mut().unwrap().policy_fault;
            let index = operation as usize;
            fault.counts[index] = fault.counts[index].saturating_add(1);
        }

        enum SynchronousFlow {
            Continue,
            Injected,
        }

        fn target_succeeded(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            target: PolicyFaultTarget,
        ) -> windows::core::Result<SynchronousFlow> {
            let notification = {
                let mut state = owner.borrow_mut();
                let Some(i) = matching(&mut state, request) else {
                    return Ok(SynchronousFlow::Continue);
                };
                let fault = &mut i.policy_fault;
                if fault.case.and_then(PolicyFixtureCase::fault_target) != Some(target) {
                    return Ok(SynchronousFlow::Continue);
                }
                fault.counts[0] = fault.counts[0].saturating_add(1);
                (
                    i.observer.clone(),
                    PolicyFaultReceipt {
                        request: request.clone(),
                        target,
                        counts: fault.counts,
                        application: fault.application,
                    },
                )
            };
            let (observer, receipt) = notification;
            let counts = receipt.counts;
            let application = receipt.application;
            if let Some(observer) = observer.as_ref() {
                observer(NativeSmokeObservation::Policy(
                    PolicyObservation::FaultTarget(receipt),
                ));
            }
            if !matches!(
                target,
                PolicyFaultTarget::HostObjectsSetting
                    | PolicyFaultTarget::ResourceAdd
                    | PolicyFaultTarget::AllSourceFilter
                    | PolicyFaultTarget::ScriptApplication
                    | PolicyFaultTarget::CdpApplication
            ) {
                return Ok(SynchronousFlow::Continue);
            }
            require_live(owner, request)?;
            let applied = {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, request).ok_or(E_FAIL)?;
                let fault = &mut i.policy_fault;
                if fault.case.and_then(PolicyFixtureCase::fault_target) != Some(target)
                    || fault.counts != counts
                    || fault.application != application
                    || counts[0] != 1
                    || counts[1] != 0
                {
                    return Err(E_FAIL.into());
                }
                // Reserve once after the genuine target receipt. This is a
                // local debug abort, never a fabricated native API failure.
                fault.counts[1] = fault.counts[1].saturating_add(1);
                PolicyFaultReceipt {
                    request: request.clone(),
                    target,
                    counts: fault.counts,
                    application: fault.application,
                }
            };
            if let Some(observer) = observer {
                observer(NativeSmokeObservation::Policy(
                    PolicyObservation::FaultApplied(applied),
                ));
            }
            require_live(owner, request)?;
            Ok(SynchronousFlow::Injected)
        }

        pub(super) fn publish_final_fault(owner: &Rc<RefCell<Owner>>) {
            let notification = {
                let mut state = owner.borrow_mut();
                let Some(i) = state.instance.as_mut() else {
                    return;
                };
                let fault = &mut i.policy_fault;
                let Some(target) = fault.case.and_then(PolicyFixtureCase::fault_target) else {
                    return;
                };
                if fault.final_sent {
                    return;
                }
                fault.final_sent = true;
                (
                    i.observer.clone(),
                    PolicyFaultReceipt {
                        request: i.request.clone(),
                        target,
                        counts: fault.counts,
                        application: fault.application,
                    },
                )
            };
            if let (Some(observer), receipt) = notification {
                observer(NativeSmokeObservation::Policy(
                    PolicyObservation::FaultFinal(receipt),
                ));
            }
        }

        pub(super) fn premature_load(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
        ) -> Result<(), Code0SessionFailure> {
            if !on_ui(owner) {
                return Err(Code0SessionFailure::PolicyRejected);
            }
            {
                let mut state = owner.borrow_mut();
                if state.unmatched {
                    return Err(Code0SessionFailure::PolicyRejected);
                }
                let i = matching(&mut state, request).ok_or(Code0SessionFailure::PolicyRejected)?;
                if i.closing
                    || i.failed
                    || i.inline_loaded
                    || i.policy.is_some()
                    || i.policy_mailbox.is_some()
                    || i.policy_fault.case.is_some()
                    || i.core.is_none()
                    || i.environment.is_none()
                    || i.controller.is_none()
                    || i.message_token.is_none()
                {
                    return Err(Code0SessionFailure::PolicyRejected);
                }
                i.policy_fault.case = Some(PolicyFixtureCase::InstallPrematureLoadDenied);
            }
            // This actually invokes the same finite controlled loader before
            // begin; its live/installed gate, not this helper, must deny it.
            let result = load(owner, request);
            if result == Err(Code0SessionFailure::PolicyRejected) {
                count(owner, Count::PrematureDenial);
                target_succeeded(owner, request, PolicyFaultTarget::PrematureLoad)
                    .map_err(|_| Code0SessionFailure::PolicyRejected)?;
            }
            result
        }

        // Rc snapshots do not call COM while Owner is borrowed. Every actual
        // interface is disposed with the R4 nonterminal control stage. Event
        // closures capture only weak Owner, immutable request/generation and the
        // existing counted atomic lifetime, never any of these emitters.
        pub(super) struct Installation {
            generation: u64,
            core: Rc<ICoreWebView2>,
            environment: Rc<ICoreWebView2Environment>,
            settings: Option<Rc<ICoreWebView2Settings>>,
            core22: Option<Rc<ICoreWebView2_22>>,
            receiver: Option<Rc<ICoreWebView2DevToolsProtocolEventReceiver>>,
            resource_token: Option<i64>,
            resource_remove_attempted: bool,
            chooser_token: Option<i64>,
            chooser_remove_attempted: bool,
            filter: bool,
            filter_remove_attempted: bool,
            scripts: Vec<(String, bool)>,
            sync_ack: bool,
            script_submitted: bool,
            script_completed: bool,
            script_ack: bool,
            cdp_submitted: bool,
            cdp_completed: bool,
            cdp_ack: bool,
            barrier: bool,
            load_attempted: bool,
            main_response: bool,
            ready_control: bool,
            document_complete: bool,
            document_receipt: bool,
            validator: Code0BridgeValidator,
            delivered_requests: u32,
        }

        const MAX_DISPOSITIONS: usize = 4;
        const MAX_SCRIPT_ID_UNITS: usize = 4096;

        pub(super) struct Mailbox {
            state: Mutex<MailboxState>,
            invalid: AtomicBool,
            unknown: AtomicBool,
        }

        #[derive(Default)]
        struct MailboxState {
            queue: Vec<Disposition>,
            script_ids: Vec<String>,
        }

        // Thread-safe weak mailbox is the callback's route to the same UI
        // owner's instance. It carries no native refs or independent authority.
        // In particular no Rc/RefCell/COM object is touched on an off-STA Invoke.
        struct Completion {
            mailbox: std::sync::Weak<Mailbox>,
            callbacks: Arc<Callbacks>,
            request: Code0WindowRequest,
            generation: u64,
            ui: ThreadId,
            invoked: AtomicBool,
            first_pending: Mutex<Option<Pending>>,
            _lifetime: HandlerLifetime,
        }

        #[windows::core::implement(
            ICoreWebView2AddScriptToExecuteOnDocumentCreatedCompletedHandler,
            Agile = false
        )]
        struct ScriptCompletion {
            completion: Completion,
        }

        impl ICoreWebView2AddScriptToExecuteOnDocumentCreatedCompletedHandler_Impl
            for ScriptCompletion_Impl
        {
            fn Invoke(
                &self,
                errorcode: windows::core::HRESULT,
                result: &PCWSTR,
            ) -> windows::core::Result<()> {
                self.completion.invoke(true, errorcode, *result)
            }
        }

        #[windows::core::implement(
            ICoreWebView2CallDevToolsProtocolMethodCompletedHandler,
            Agile = false
        )]
        struct CdpCompletion {
            completion: Completion,
        }

        impl ICoreWebView2CallDevToolsProtocolMethodCompletedHandler_Impl for CdpCompletion_Impl {
            fn Invoke(
                &self,
                errorcode: windows::core::HRESULT,
                result: &PCWSTR,
            ) -> windows::core::Result<()> {
                self.completion.invoke(false, errorcode, *result)
            }
        }

        enum Disposition {
            Script {
                request: Code0WindowRequest,
                generation: u64,
                succeeded: bool,
                valid_id: bool,
                first: bool,
                id: Option<String>,
                pending: Pending,
            },
            Cdp {
                request: Code0WindowRequest,
                generation: u64,
                expected: bool,
                first: bool,
                pending: Pending,
            },
        }

        // Callback PCWSTR is borrowed, not CoTaskMem output. Never free it or
        // retain its address. An overlength/malformed ID is unknown ownership;
        // it is never truncated and submitted to native removal.
        fn borrowed_string(raw: PCWSTR, max_units: usize) -> windows::core::Result<String> {
            if raw.is_null() {
                return Err(windows::core::Error::from_hresult(E_FAIL));
            }
            for length in 0..=max_units {
                if unsafe { *raw.0.add(length) } == 0 {
                    let units = unsafe { std::slice::from_raw_parts(raw.0, length) };
                    return String::from_utf16(units)
                        .map_err(|_| windows::core::Error::from_hresult(E_FAIL));
                }
            }
            Err(windows::core::Error::from_hresult(E_FAIL))
        }

        impl Completion {
            fn new(owner: &Rc<RefCell<Owner>>, request: &Code0WindowRequest) -> Self {
                let (callbacks, ui) = callback_state(owner);
                let mailbox = {
                    let state = owner.borrow();
                    Arc::downgrade(
                        state
                            .instance
                            .as_ref()
                            .unwrap()
                            .policy_mailbox
                            .as_ref()
                            .unwrap(),
                    )
                };
                Self {
                    mailbox,
                    request: request.clone(),
                    generation: GENERATION,
                    ui,
                    invoked: AtomicBool::new(false),
                    first_pending: Mutex::new(Some(Pending::new(&callbacks))),
                    _lifetime: HandlerLifetime::new(&callbacks),
                    callbacks,
                }
            }

            fn invoke(
                &self,
                script: bool,
                errorcode: windows::core::HRESULT,
                raw: PCWSTR,
            ) -> windows::core::Result<()> {
                let _active = Active::enter(&self.callbacks);
                // Off-thread must perform only owned-copy/atomics/mutex work.
                // The UI-thread path may additionally validate its apartment.
                if thread::current().id() != self.ui || !is_sta() {
                    self.callbacks.affinity_failed.store(true, Ordering::SeqCst);
                }
                let Some(mailbox) = self.mailbox.upgrade() else {
                    self.callbacks.affinity_failed.store(true, Ordering::SeqCst);
                    return Ok(());
                };
                let first = !self.invoked.swap(true, Ordering::SeqCst);
                if !first {
                    mailbox.invalid.store(true, Ordering::SeqCst);
                }
                let pending = if first {
                    self.first_pending.lock().unwrap().take().unwrap()
                } else {
                    Pending::new(&self.callbacks)
                };
                let copied = borrowed_string(raw, if script { MAX_SCRIPT_ID_UNITS } else { 2048 })
                    .and_then(|value| {
                        if !script && value.len() > 2048 {
                            Err(windows::core::Error::from_hresult(E_FAIL))
                        } else {
                            Ok(value)
                        }
                    });
                if copied.is_err() {
                    mailbox.unknown.store(true, Ordering::SeqCst);
                }
                let disposition = if script {
                    let valid_id = copied.as_ref().is_ok_and(|id| !id.is_empty());
                    // A successful registration without a usable exact ID is an
                    // unresolved removal obligation, not an empty inventory.
                    if errorcode.is_ok() && !valid_id {
                        mailbox.unknown.store(true, Ordering::SeqCst);
                    }
                    let mut id = copied.ok().filter(|id| !id.is_empty());
                    {
                        let mut state = mailbox.state.lock().unwrap();
                        if let Some(value) = id.as_ref() {
                            if state.script_ids.contains(value) {
                                id = None; // Same real ID is removed at most once.
                            } else if state.script_ids.len() < MAX_DISPOSITIONS {
                                state.script_ids.push(value.clone());
                            } else {
                                mailbox.unknown.store(true, Ordering::SeqCst);
                                id = None;
                            }
                        }
                    }
                    Disposition::Script {
                        request: self.request.clone(),
                        generation: self.generation,
                        succeeded: errorcode.is_ok(),
                        valid_id,
                        first,
                        id,
                        pending,
                    }
                } else {
                    let expected = errorcode.is_ok()
                        && copied.as_ref().is_ok_and(|json| {
                            json.len() <= 2048
                                && serde_json::from_str::<serde_json::Value>(json)
                                    .ok()
                                    .is_some_and(|v| v.as_object().is_some_and(|o| o.is_empty()))
                        });
                    Disposition::Cdp {
                        request: self.request.clone(),
                        generation: self.generation,
                        expected,
                        first,
                        pending,
                    }
                };
                let overflow = {
                    let mut state = mailbox.state.lock().unwrap();
                    if state.queue.len() < MAX_DISPOSITIONS {
                        state.queue.push(disposition);
                        None
                    } else {
                        mailbox.unknown.store(true, Ordering::SeqCst);
                        Some(disposition)
                    }
                };
                // Pending/owned payload drops happen after the queue lock ends.
                drop(overflow);
                // Invoke transport success is never an installation ACK. Do
                // not let generated Error->HRESULT conversion set thread COM
                // error info off STA; sticky flags/disposition carry failure.
                Ok(())
            }
        }

        fn live(owner: &Rc<RefCell<Owner>>, request: &Code0WindowRequest) -> bool {
            let state = owner.borrow();
            !state.unmatched
                && state.instance.as_ref().is_some_and(|i| {
                    &i.request == request
                        && i.policy_mailbox.as_ref().is_some_and(|m| {
                            !m.invalid.load(Ordering::SeqCst) && !m.unknown.load(Ordering::SeqCst)
                        })
                        && !i.closing
                        && !i.failed
                        && !i.inline_loaded
                        && !i.callbacks.affinity_failed.load(Ordering::SeqCst)
                        && i.policy
                            .as_ref()
                            .is_some_and(|p| p.generation == GENERATION)
                })
        }

        fn require_live(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
        ) -> windows::core::Result<()> {
            if live(owner, request) {
                Ok(())
            } else {
                Err(E_FAIL.into())
            }
        }

        fn uuid(value: &str) -> bool {
            value.len() == 36
                && value.bytes().enumerate().all(|(n, b)| {
                    if [8, 13, 18, 23].contains(&n) {
                        b == b'-'
                    } else {
                        b.is_ascii_hexdigit()
                    }
                })
        }

        fn core(owner: &Rc<RefCell<Owner>>) -> Rc<ICoreWebView2> {
            Rc::clone(
                &owner
                    .borrow()
                    .instance
                    .as_ref()
                    .unwrap()
                    .policy
                    .as_ref()
                    .unwrap()
                    .core,
            )
        }

        fn callback_state(owner: &Rc<RefCell<Owner>>) -> (Arc<Callbacks>, ThreadId) {
            let state = owner.borrow();
            (
                Arc::clone(&state.instance.as_ref().unwrap().callbacks),
                state.ui_thread,
            )
        }

        pub(super) fn begin(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            case: PolicyFixtureCase,
        ) -> Result<(), Code0SessionFailure> {
            if !on_ui(owner) {
                return Err(Code0SessionFailure::PolicyRejected);
            }
            let callbacks = {
                let state = owner.borrow();
                let i = state
                    .instance
                    .as_ref()
                    .ok_or(Code0SessionFailure::PolicyRejected)?;
                if &i.request != request
                    || i.closing
                    || i.failed
                    || i.inline_loaded
                    || i.policy.is_some()
                    || i.core.is_none()
                    || i.environment.is_none()
                    || i.controller.is_none()
                    || i.message_token.is_none()
                    || ![
                        &request.instance_id,
                        &request.connection_epoch,
                        &request.nonce,
                        &request.document_id,
                    ]
                    .into_iter()
                    .all(|s| uuid(s))
                {
                    return Err(Code0SessionFailure::PolicyRejected);
                }
                Arc::clone(&i.callbacks)
            };
            owner
                .borrow_mut()
                .instance
                .as_mut()
                .unwrap()
                .policy_fault
                .case = Some(case);
            let _action = Active::enter(&callbacks);
            // Move original refs out before AddRef, then restore them. Active
            // prevents a reentrant poll from treating the temporary move as free.
            let (original_core, original_environment) = {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, request).unwrap();
                (i.core.take().unwrap(), i.environment.take().unwrap())
            };
            let retained_core = Rc::new(original_core.clone());
            let retained_environment = Rc::new(original_environment.clone());
            {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, request).unwrap();
                i.core = Some(original_core);
                i.environment = Some(original_environment);
                i.policy_mailbox = Some(Arc::new(Mailbox {
                    state: Mutex::new(MailboxState::default()),
                    invalid: AtomicBool::new(false),
                    unknown: AtomicBool::new(false),
                }));
                i.policy = Some(Installation {
                    generation: GENERATION,
                    core: retained_core,
                    environment: retained_environment,
                    settings: None,
                    core22: None,
                    receiver: None,
                    resource_token: None,
                    resource_remove_attempted: false,
                    chooser_token: None,
                    chooser_remove_attempted: false,
                    filter: false,
                    filter_remove_attempted: false,
                    scripts: Vec::new(),
                    sync_ack: false,
                    script_submitted: false,
                    script_completed: false,
                    script_ack: false,
                    cdp_submitted: false,
                    cdp_completed: false,
                    cdp_ack: false,
                    barrier: false,
                    load_attempted: false,
                    main_response: false,
                    ready_control: false,
                    document_complete: false,
                    document_receipt: false,
                    validator: Code0BridgeValidator::new(
                        &request.connection_epoch,
                        &request.instance_id,
                        &request.nonce,
                        &request.document_id,
                    ),
                    delivered_requests: 0,
                });
            }
            let accepted = match synchronous(owner, request) {
                Ok(SynchronousFlow::Continue) => submit_script(owner, request).is_ok(),
                Ok(SynchronousFlow::Injected) | Err(_) => false,
            };
            if !accepted {
                failed(owner, request);
                return Err(Code0SessionFailure::PolicyRejected);
            }
            Ok(())
        }

        fn synchronous(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
        ) -> windows::core::Result<SynchronousFlow> {
            require_live(owner, request)?;
            let core = core(owner);
            let settings = Rc::new(unsafe { core.Settings() }?);
            owner
                .borrow_mut()
                .instance
                .as_mut()
                .unwrap()
                .policy
                .as_mut()
                .unwrap()
                .settings = Some(Rc::clone(&settings));
            require_live(owner, request)?;
            macro_rules! setting {
                ($set:ident, $get:ident, $value:expr) => {{
                    unsafe { settings.$set($value) }?;
                    require_live(owner, request)?;
                    let mut actual = BOOL::default();
                    unsafe { settings.$get(&mut actual) }?;
                    require_live(owner, request)?;
                    if actual.as_bool() != $value {
                        return Err(E_FAIL.into());
                    }
                }};
            }
            setting!(SetIsWebMessageEnabled, IsWebMessageEnabled, true);
            setting!(
                SetAreDefaultScriptDialogsEnabled,
                AreDefaultScriptDialogsEnabled,
                false
            );
            setting!(SetAreDevToolsEnabled, AreDevToolsEnabled, false);
            setting!(
                SetAreDefaultContextMenusEnabled,
                AreDefaultContextMenusEnabled,
                false
            );
            setting!(SetAreHostObjectsAllowed, AreHostObjectsAllowed, false);
            if let SynchronousFlow::Injected =
                target_succeeded(owner, request, PolicyFaultTarget::HostObjectsSetting)?
            {
                return Ok(SynchronousFlow::Injected);
            }
            require_live(owner, request)?;
            let core22 = Rc::new(core.cast::<ICoreWebView2_22>()?);
            owner
                .borrow_mut()
                .instance
                .as_mut()
                .unwrap()
                .policy
                .as_mut()
                .unwrap()
                .core22 = Some(Rc::clone(&core22));
            require_live(owner, request)?;
            let (callbacks, ui) = callback_state(owner);
            let lifetime = HandlerLifetime::new(&callbacks);
            let weak = Rc::downgrade(owner);
            let owned = request.clone();
            let generation = GENERATION;
            let handler =
                WebResourceRequestedEventHandler::create(Box::new(move |sender, args| {
                    let _lifetime = &lifetime;
                    let active = Active::enter(&callbacks);
                    let result = if callback_on_ui(ui, &callbacks) {
                        if let Some(owner) = weak.upgrade() {
                            let mut trace = ResourceTrace::new();
                            let result = resource(
                                &owner,
                                &owned,
                                generation,
                                sender.as_ref(),
                                args.as_ref(),
                                &mut trace,
                            );
                            if result.is_err() {
                                uncertain_response(&owner, &owned, trace.detail);
                            }
                            result
                        } else {
                            Err(E_FAIL.into())
                        }
                    } else {
                        // callback_on_ui already latched affinity failure. A
                        // transport error would access COM thread error info;
                        // transport success grants no response/install ACK.
                        Ok(())
                    };
                    drop(args);
                    drop(sender);
                    drop(active);
                    result
                }));
            let mut token = 0;
            let result = unsafe { core.add_WebResourceRequested(&handler, &mut token) };
            // Store every successful token before checking reentrant revocation.
            if result.is_ok() {
                owner
                    .borrow_mut()
                    .instance
                    .as_mut()
                    .unwrap()
                    .policy
                    .as_mut()
                    .unwrap()
                    .resource_token = Some(token);
                count(owner, Count::ResourceAdd);
            }
            drop(handler);
            result?;
            if let SynchronousFlow::Injected =
                target_succeeded(owner, request, PolicyFaultTarget::ResourceAdd)?
            {
                return Ok(SynchronousFlow::Injected);
            }
            require_live(owner, request)?;
            let filtered = unsafe {
                core22.AddWebResourceRequestedFilterWithRequestSourceKinds(
                    w!("*"),
                    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                    COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
                )
            };
            if filtered.is_ok() {
                owner
                    .borrow_mut()
                    .instance
                    .as_mut()
                    .unwrap()
                    .policy
                    .as_mut()
                    .unwrap()
                    .filter = true;
                count(owner, Count::FilterAdd);
            }
            filtered?;
            if let SynchronousFlow::Injected =
                target_succeeded(owner, request, PolicyFaultTarget::AllSourceFilter)?
            {
                return Ok(SynchronousFlow::Injected);
            }
            require_live(owner, request)?;
            let receiver = Rc::new(unsafe {
                core.GetDevToolsProtocolEventReceiver(w!("Page.fileChooserOpened"))
            }?);
            owner
                .borrow_mut()
                .instance
                .as_mut()
                .unwrap()
                .policy
                .as_mut()
                .unwrap()
                .receiver = Some(Rc::clone(&receiver));
            require_live(owner, request)?;
            let (callbacks, ui) = callback_state(owner);
            let lifetime = HandlerLifetime::new(&callbacks);
            let weak = Rc::downgrade(owner);
            let owned = request.clone();
            let generation = GENERATION;
            let handler =
                DevToolsProtocolEventReceivedEventHandler::create(Box::new(move |sender, args| {
                    let _lifetime = &lifetime;
                    let active = Active::enter(&callbacks);
                    if callback_on_ui(ui, &callbacks)
                        && let Some(owner) = weak.upgrade()
                    {
                        // No chooser stimulus belongs to this benign case. Even a
                        // missing/unknown event fails it; no file/parameter parsing.
                        let mut state = owner.borrow_mut();
                        if let Some(i) = matching(&mut state, &owned)
                            && i.policy
                                .as_ref()
                                .is_some_and(|p| p.generation == generation)
                        {
                            i.failed = true;
                            i.closing = true;
                        } else {
                            state.unmatched = true;
                        }
                    }
                    drop(args);
                    drop(sender);
                    drop(active);
                    Ok(())
                }));
            let mut token = 0;
            let registered =
                unsafe { receiver.add_DevToolsProtocolEventReceived(&handler, &mut token) };
            if registered.is_ok() {
                owner
                    .borrow_mut()
                    .instance
                    .as_mut()
                    .unwrap()
                    .policy
                    .as_mut()
                    .unwrap()
                    .chooser_token = Some(token);
            }
            drop(handler);
            registered?;
            require_live(owner, request)?;
            owner
                .borrow_mut()
                .instance
                .as_mut()
                .unwrap()
                .policy
                .as_mut()
                .unwrap()
                .sync_ack = true;
            Ok(SynchronousFlow::Continue)
        }

        fn submit_script(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
        ) -> windows::core::Result<()> {
            require_live(owner, request)?;
            let core = core(owner);
            let handler: ICoreWebView2AddScriptToExecuteOnDocumentCreatedCompletedHandler =
                ScriptCompletion {
                    completion: Completion::new(owner, request),
                }
                .into();
            // The canonical expression bytes are unchanged. Evaluation, config
            // and transport construction all occur inside the exact realm guard.
            let script = format!(
                "(()=>{{if(window!==top||location.origin!==\"https://console.code0.ai\")return;const install=({CODE0_BRIDGE_SOURCE});install({{version:1,nonce:{},document:{},send:(message)=>window.chrome.webview.postMessage(message)}});}})();",
                serde_json::to_string(&request.nonce).map_err(|_| E_FAIL)?,
                serde_json::to_string(&request.document_id).map_err(|_| E_FAIL)?
            );
            let script: Vec<u16> = script.encode_utf16().chain(Some(0)).collect();
            owner
                .borrow_mut()
                .instance
                .as_mut()
                .unwrap()
                .policy
                .as_mut()
                .unwrap()
                .script_submitted = true;
            count(owner, Count::ScriptSubmission);
            let submitted = unsafe {
                core.AddScriptToExecuteOnDocumentCreated(PCWSTR(script.as_ptr()), &handler)
            };
            drop(handler);
            submitted?;
            require_live(owner, request)
        }

        fn submit_cdp(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
        ) -> windows::core::Result<()> {
            require_live(owner, request)?;
            let (callbacks, _) = callback_state(owner);
            let _action = Active::enter(&callbacks);
            let core = core(owner);
            let handler: ICoreWebView2CallDevToolsProtocolMethodCompletedHandler = CdpCompletion {
                completion: Completion::new(owner, request),
            }
            .into();
            owner
                .borrow_mut()
                .instance
                .as_mut()
                .unwrap()
                .policy
                .as_mut()
                .unwrap()
                .cdp_submitted = true;
            count(owner, Count::CdpSubmission);
            let result = unsafe {
                core.CallDevToolsProtocolMethod(
                    w!("Page.setInterceptFileChooserDialog"),
                    w!("{\"enabled\":true,\"cancel\":true}"),
                    &handler,
                )
            };
            drop(handler);
            result?;
            require_live(owner, request)
        }

        fn hold_disposition(owner: &Rc<RefCell<Owner>>) -> bool {
            let request = {
                let state = owner.borrow();
                let Some(i) = state.instance.as_ref() else {
                    return false;
                };
                if !i
                    .policy_fault
                    .case
                    .is_some_and(PolicyFixtureCase::stale_disposition)
                {
                    return false;
                }
                i.request.clone()
            };
            if !live(owner, &request) {
                return false;
            }
            let notification = {
                let mut state = owner.borrow_mut();
                if state.unmatched {
                    return false;
                }
                let Some(i) = matching(&mut state, &request) else {
                    return false;
                };
                if i.policy_fault.stale_phase == StalePhase::Held {
                    return true;
                }
                if i.policy_fault.stale_phase != StalePhase::Waiting {
                    return false;
                }
                let Some(p) = i.policy.as_ref() else {
                    return false;
                };
                let Some(mailbox) = i.policy_mailbox.as_ref() else {
                    return false;
                };
                if mailbox.invalid.load(Ordering::SeqCst)
                    || mailbox.unknown.load(Ordering::SeqCst)
                    || i.callbacks.affinity_failed.load(Ordering::SeqCst)
                {
                    return false;
                }
                let queue = mailbox.state.lock().unwrap();
                let candidate = queue.queue.iter().find_map(|d| match d {
                    Disposition::Script {
                        request: r,
                        generation,
                        succeeded,
                        valid_id,
                        first,
                        id,
                        ..
                    } if i.policy_fault.case
                        == Some(PolicyFixtureCase::InstallScriptStaleDisposition)
                        && r == &request
                        && *generation == p.generation
                        && *first
                        && *succeeded
                        && *valid_id
                        && id.as_ref().is_some_and(|id| !id.is_empty())
                        && p.sync_ack
                        && p.script_submitted
                        && !p.script_completed
                        && !p.script_ack
                        && p.scripts.is_empty() =>
                    {
                        Some((true, true))
                    }
                    Disposition::Cdp {
                        request: r,
                        generation,
                        expected,
                        first,
                        ..
                    } if i.policy_fault.case
                        == Some(PolicyFixtureCase::InstallCdpStaleDisposition)
                        && r == &request
                        && *generation == p.generation
                        && *first
                        && *expected
                        && p.sync_ack
                        && p.script_ack
                        && p.cdp_submitted
                        && !p.cdp_completed
                        && !p.cdp_ack
                        && p.scripts.len() == 1 =>
                    {
                        Some((false, false))
                    }
                    _ => None,
                });
                let Some((script, queued_script_id)) = candidate else {
                    return false;
                };
                i.policy_fault.stale_phase = StalePhase::Held;
                i.policy_fault.stale_script = script;
                i.policy_fault.stale_order[0] = i.policy_fault.stale_order[0].saturating_add(1);
                (
                    i.observer.clone(),
                    StaleDispositionReceipt {
                        request: request.clone(),
                        generation: p.generation,
                        stage: StaleDispositionStage::Held,
                        script,
                        counts: i.policy_fault.counts,
                        application: i.policy_fault.application,
                        order: i.policy_fault.stale_order,
                        queued_script_id,
                        policy_script_ids: p.scripts.len() as u32,
                    },
                )
            };
            if !live(owner, &request) {
                return false;
            }
            if let (Some(observer), receipt) = notification {
                observer(NativeSmokeObservation::Policy(
                    PolicyObservation::StaleDisposition(receipt),
                ));
            }
            // Never drain this turn, including observer-driven close/reentry.
            true
        }

        pub(super) fn mark_stale_close(i: &mut Instance, unmatched: bool) {
            if !unmatched
                && !i.closing
                && !i.failed
                && i.policy_fault.stale_phase == StalePhase::Held
                && i.policy
                    .as_ref()
                    .is_some_and(|p| p.generation == GENERATION)
                && i.policy_mailbox.as_ref().is_some_and(|m| {
                    !m.invalid.load(Ordering::SeqCst) && !m.unknown.load(Ordering::SeqCst)
                })
                && !i.callbacks.affinity_failed.load(Ordering::SeqCst)
            {
                i.policy_fault.stale_phase = StalePhase::CloseRevoked;
                i.policy_fault.stale_order[1] = i.policy_fault.stale_order[1].saturating_add(1);
            }
        }

        fn stale_drained(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            generation: u64,
            script: bool,
            genuine: bool,
        ) {
            let notification = {
                let mut state = owner.borrow_mut();
                let Some(i) = matching(&mut state, request) else {
                    return;
                };
                let Some(p) = i.policy.as_ref() else {
                    return;
                };
                if !genuine
                    || p.generation != generation
                    || i.policy_fault.stale_script != script
                    || i.policy_fault.stale_phase != StalePhase::CloseRevoked
                {
                    return;
                }
                i.policy_fault.stale_phase = StalePhase::Drained;
                i.policy_fault.stale_order[2] = i.policy_fault.stale_order[2].saturating_add(1);
                (
                    i.observer.clone(),
                    StaleDispositionReceipt {
                        request: request.clone(),
                        generation,
                        stage: StaleDispositionStage::Drained,
                        script,
                        counts: i.policy_fault.counts,
                        application: i.policy_fault.application,
                        order: i.policy_fault.stale_order,
                        queued_script_id: false,
                        policy_script_ids: p.scripts.len() as u32,
                    },
                )
            };
            if let (Some(observer), receipt) = notification {
                observer(NativeSmokeObservation::Policy(
                    PolicyObservation::StaleDisposition(receipt),
                ));
            }
        }

        pub(super) fn publish_final_coverage(owner: &Rc<RefCell<Owner>>) {
            let notification = {
                let mut state = owner.borrow_mut();
                let Some(i) = state.instance.as_mut() else {
                    return;
                };
                if i.policy_fault
                    .case
                    .is_some_and(PolicyFixtureCase::stale_disposition)
                {
                    if i.policy_fault.stale_order[3] != 0 {
                        return;
                    }
                    i.policy_fault.stale_order[3] = i.policy_fault.stale_order[3].saturating_add(1);
                    Some((
                        i.observer.clone(),
                        PolicyObservation::StaleDisposition(StaleDispositionReceipt {
                            request: i.request.clone(),
                            generation: GENERATION,
                            stage: StaleDispositionStage::Final,
                            script: i.policy_fault.stale_script,
                            counts: i.policy_fault.counts,
                            application: i.policy_fault.application,
                            order: i.policy_fault.stale_order,
                            queued_script_id: false,
                            policy_script_ids: i
                                .policy
                                .as_ref()
                                .map_or(0, |p| p.scripts.len() as u32),
                        }),
                    ))
                } else if i.policy_fault.case == Some(PolicyFixtureCase::InstallRealmsNoPassthrough)
                {
                    let c = &mut i.policy_fault.containment;
                    if c.final_sent {
                        return;
                    }
                    c.final_sent = true;
                    Some((
                        i.observer.clone(),
                        PolicyObservation::Containment(ContainmentReceipt {
                            request: i.request.clone(),
                            generation: GENERATION,
                            final_snapshot: true,
                            counts: i.policy_fault.counts,
                            application: i.policy_fault.application,
                            deliveries: c.deliveries,
                            responses: c.responses,
                            total_delivered: c.total,
                            renderer: c.renderer,
                            document: c.document,
                        }),
                    ))
                } else {
                    None
                }
            };
            if let Some((Some(observer), receipt)) = notification {
                observer(NativeSmokeObservation::Policy(receipt));
            }
        }

        fn drain_dispositions(owner: &Rc<RefCell<Owner>>) {
            let mailbox = {
                let state = owner.borrow();
                state
                    .instance
                    .as_ref()
                    .and_then(|i| i.policy_mailbox.as_ref())
                    .map(Arc::clone)
            };
            let Some(mailbox) = mailbox else {
                return;
            };
            let dispositions = std::mem::take(&mut mailbox.state.lock().unwrap().queue);
            for disposition in dispositions {
                match disposition {
                    Disposition::Script {
                        request,
                        generation,
                        succeeded,
                        valid_id,
                        first,
                        id,
                        pending,
                    } => {
                        let asynchronous = owner.borrow().instance.as_ref().is_some_and(|i| {
                            i.policy_fault
                                .case
                                .is_some_and(PolicyFixtureCase::observes_application)
                        });
                        let current_live = !asynchronous || live(owner, &request);
                        let genuine = first && succeeded && valid_id && id.is_some();
                        let retained_id = if owner.borrow().instance.as_ref().is_some_and(|i| {
                            i.policy_fault
                                .case
                                .is_some_and(PolicyFixtureCase::async_application)
                        }) {
                            id.clone()
                        } else {
                            None
                        };
                        let target = {
                            let mut state = owner.borrow_mut();
                            if let Some(i) = state.instance.as_mut()
                                && let Some(p) = i.policy.as_mut()
                            {
                                let contributed_id = id.is_some();
                                if let Some(id) = id {
                                    p.scripts.push((id, false));
                                    if asynchronous {
                                        i.policy_fault.application[0] = true;
                                    }
                                }
                                let valid = i.request == request
                                    && p.generation == generation
                                    && first
                                    && p.script_submitted
                                    && !p.script_completed
                                    && p.sync_ack
                                    && !i.closing
                                    && !i.failed
                                    && succeeded
                                    && valid_id
                                    && (!asynchronous
                                        || (current_live
                                            && contributed_id
                                            && p.scripts.len() == 1
                                            && !p.script_ack))
                                    && !mailbox.invalid.load(Ordering::SeqCst)
                                    && !mailbox.unknown.load(Ordering::SeqCst)
                                    && !i.callbacks.affinity_failed.load(Ordering::SeqCst);
                                p.script_completed = true;
                                let target = valid
                                    && i.policy_fault.case
                                        == Some(PolicyFixtureCase::InstallScriptInjectedFailure);
                                if !target {
                                    p.script_ack = valid;
                                    if asynchronous && valid {
                                        i.policy_fault.application[1] = true;
                                    }
                                }
                                if !valid {
                                    i.failed = true;
                                    i.closing = true;
                                }
                                target
                            } else {
                                if let Some(id) = id {
                                    state.unmatched_script_ids.push(id);
                                }
                                state.unmatched = true;
                                false
                            }
                        };
                        if target {
                            // Pending and the exact inventoried ID survive the observer.
                            // Only Continue may apply the real ACK; injected or invalid work closes without it.
                            let continued = matches!(
                                target_succeeded(
                                    owner,
                                    &request,
                                    PolicyFaultTarget::ScriptApplication
                                ),
                                Ok(SynchronousFlow::Continue)
                            ) && live(owner, &request);
                            let mut state = owner.borrow_mut();
                            if let Some(i) = state.instance.as_mut()
                                && let Some(p) = i.policy.as_mut()
                            {
                                let valid = continued
                                    && i.request == request
                                    && p.generation == generation
                                    && p.sync_ack
                                    && p.script_submitted
                                    && p.script_completed
                                    && !p.script_ack
                                    && p.scripts.len() == 1
                                    && p.scripts.iter().any(|(id, attempted)| {
                                        Some(id) == retained_id.as_ref() && !attempted
                                    });
                                if valid {
                                    p.script_ack = true;
                                    i.policy_fault.application[1] = true;
                                } else {
                                    i.failed = true;
                                    i.closing = true;
                                }
                            } else {
                                state.unmatched = true;
                            }
                        }
                        stale_drained(owner, &request, generation, true, genuine);
                        drop(pending);
                    }
                    Disposition::Cdp {
                        request,
                        generation,
                        expected,
                        first,
                        pending,
                    } => {
                        let asynchronous = owner.borrow().instance.as_ref().is_some_and(|i| {
                            i.policy_fault
                                .case
                                .is_some_and(PolicyFixtureCase::observes_application)
                        });
                        let current_live = !asynchronous || live(owner, &request);
                        let target = {
                            let mut state = owner.borrow_mut();
                            if let Some(i) = state.instance.as_mut()
                                && let Some(p) = i.policy.as_mut()
                            {
                                let valid = i.request == request
                                    && p.generation == generation
                                    && first
                                    && p.sync_ack
                                    && p.script_ack
                                    && p.cdp_submitted
                                    && !p.cdp_completed
                                    && !i.closing
                                    && !i.failed
                                    && expected
                                    && (!asynchronous || (current_live && !p.cdp_ack))
                                    && !mailbox.invalid.load(Ordering::SeqCst)
                                    && !mailbox.unknown.load(Ordering::SeqCst)
                                    && !i.callbacks.affinity_failed.load(Ordering::SeqCst);
                                p.cdp_completed = true;
                                let target = valid
                                    && i.policy_fault.case
                                        == Some(PolicyFixtureCase::InstallCdpInjectedFailure);
                                if !target {
                                    p.cdp_ack = valid;
                                    if asynchronous && valid {
                                        i.policy_fault.application[2] = true;
                                    }
                                }
                                if !valid {
                                    i.failed = true;
                                    i.closing = true;
                                }
                                target
                            } else {
                                state.unmatched = true;
                                false
                            }
                        };
                        if target {
                            let continued = matches!(
                                target_succeeded(
                                    owner,
                                    &request,
                                    PolicyFaultTarget::CdpApplication
                                ),
                                Ok(SynchronousFlow::Continue)
                            ) && live(owner, &request);
                            let mut state = owner.borrow_mut();
                            if let Some(i) = state.instance.as_mut()
                                && let Some(p) = i.policy.as_mut()
                            {
                                let valid = continued
                                    && i.request == request
                                    && p.generation == generation
                                    && p.sync_ack
                                    && p.script_ack
                                    && p.cdp_submitted
                                    && p.cdp_completed
                                    && !p.cdp_ack
                                    && p.scripts.len() == 1
                                    && !p.scripts.iter().any(|(_, attempted)| *attempted);
                                if valid {
                                    p.cdp_ack = true;
                                    i.policy_fault.application[2] = true;
                                } else {
                                    i.failed = true;
                                    i.closing = true;
                                }
                            } else {
                                state.unmatched = true;
                            }
                        }
                        stale_drained(owner, &request, generation, false, first && expected);
                        drop(pending);
                    }
                }
            }
            if mailbox.invalid.load(Ordering::SeqCst) || mailbox.unknown.load(Ordering::SeqCst) {
                let mut state = owner.borrow_mut();
                if let Some(i) = state.instance.as_mut() {
                    i.failed = true;
                    i.closing = true;
                    if mailbox.unknown.load(Ordering::SeqCst) {
                        if let Some(diagnostic) = i.callbacks.diagnostics.as_ref() {
                            diagnostic.uncertainty(DU::cleanup(
                                US::MailboxUnknown,
                                UR::Unknown,
                                None,
                            ));
                        }
                        i.cleanup_blocked = true;
                    }
                }
            }
        }

        pub(super) fn mailbox_quiescent(i: &Instance) -> bool {
            i.policy_mailbox.as_ref().is_none_or(|m| {
                !m.unknown.load(Ordering::SeqCst) && m.state.lock().unwrap().queue.is_empty()
            })
        }

        pub(super) fn after_control_drop(owner: &Rc<RefCell<Owner>>) -> bool {
            let mut state = owner.borrow_mut();
            let unmatched = state.unmatched;
            let i = state.instance.as_mut().unwrap();
            let valid = !unmatched
                && !i.cleanup_blocked
                && mailbox_quiescent(i)
                && i.callbacks.pending.load(Ordering::SeqCst) == 0
                && i.callbacks.active.load(Ordering::SeqCst) == 0
                && !i.callbacks.affinity_failed.load(Ordering::SeqCst);
            if !valid {
                if let Some(diagnostic) = i.callbacks.diagnostics.as_ref() {
                    diagnostic.uncertainty(DU::cleanup(US::AfterControlDrop, UR::Condition, None));
                }
                if let Some(m) = &i.policy_mailbox {
                    m.unknown.store(true, Ordering::SeqCst);
                }
                i.cleanup_blocked = true;
                i.failed = true;
                i.closing = true;
            }
            valid
        }

        fn installed(p: &Installation) -> bool {
            p.sync_ack
                && p.script_submitted
                && p.script_completed
                && p.script_ack
                && p.scripts.len() == 1
                && p.cdp_submitted
                && p.cdp_completed
                && p.cdp_ack
                && p.settings.is_some()
                && p.core22.is_some()
                && p.receiver.is_some()
                && p.resource_token.is_some()
                && p.chooser_token.is_some()
                && p.filter
                && !p.resource_remove_attempted
                && !p.chooser_remove_attempted
                && !p.filter_remove_attempted
                && !p.scripts.iter().any(|(_, attempted)| *attempted)
        }

        pub(super) fn advance(owner: &Rc<RefCell<Owner>>) {
            // Selected async application waits for the current action to settle.
            // Never require Pending0: the queued real disposition owns Pending.
            if owner.borrow().instance.as_ref().is_some_and(|i| {
                i.policy_fault
                    .case
                    .is_some_and(PolicyFixtureCase::observes_application)
                    && i.callbacks.active.load(Ordering::SeqCst) != 0
            }) {
                return;
            }
            if hold_disposition(owner) {
                return;
            }
            drain_dispositions(owner);
            let request = {
                let state = owner.borrow();
                let Some(i) = state.instance.as_ref() else {
                    return;
                };
                if i.policy.is_none()
                    || i.closing
                    || i.failed
                    || i.callbacks.pending.load(Ordering::SeqCst) != 0
                    || i.callbacks.active.load(Ordering::SeqCst) != 0
                    || i.callbacks.affinity_failed.load(Ordering::SeqCst)
                {
                    return;
                }
                i.request.clone()
            };
            if !live(owner, &request) {
                failed(owner, &request);
                return;
            }
            let cdp = {
                let state = owner.borrow();
                let p = state.instance.as_ref().unwrap().policy.as_ref().unwrap();
                p.sync_ack && p.script_ack && !p.cdp_submitted
            };
            if cdp {
                if submit_cdp(owner, &request).is_err() {
                    failed(owner, &request);
                }
                // Even a synchronous completion must settle this action first.
                return;
            }
            let notification = {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, &request).unwrap();
                let p = i.policy.as_mut().unwrap();
                let event = if installed(p) && !p.barrier {
                    p.barrier = true;
                    Some(PolicyObservation::BarrierReady(InstallBarrierReceipt {
                        request: request.clone(),
                        synchronous_install_succeeded: true,
                        script_acknowledged: p.script_ack,
                        cdp_acknowledged: p.cdp_ack,
                    }))
                } else if installed(p)
                    && p.barrier
                    && p.load_attempted
                    && p.main_response
                    && p.ready_control
                    && p.document_complete
                    && !p.document_receipt
                {
                    if i.policy_fault.case == Some(PolicyFixtureCase::InstallRealmsNoPassthrough) {
                        let c = &mut i.policy_fault.containment;
                        if c.deliveries[..6] == [1; 6]
                            && c.responses[..6] == [1; 6]
                            && c.renderer == [true; 5]
                            && c.source_matched
                        {
                            p.document_receipt = true;
                            c.document = [c.source_matched, p.main_response, p.ready_control];
                            Some(PolicyObservation::Containment(ContainmentReceipt {
                                request: request.clone(),
                                generation: p.generation,
                                final_snapshot: false,
                                counts: i.policy_fault.counts,
                                application: i.policy_fault.application,
                                deliveries: c.deliveries,
                                responses: c.responses,
                                total_delivered: c.total,
                                renderer: c.renderer,
                                document: c.document,
                            }))
                        } else {
                            None
                        }
                    } else {
                        p.document_receipt = true;
                        Some(PolicyObservation::ControlledDocumentReady(
                            ControlledDocumentReceipt {
                                request: request.clone(),
                                source_matched: true,
                                synthetic_response_applied: p.main_response,
                                canonical_ready_control: p.ready_control,
                            },
                        ))
                    }
                } else {
                    None
                };
                event.map(|event| (i.observer.clone(), event))
            };
            if let Some((Some(observer), event)) = notification {
                observer(NativeSmokeObservation::Policy(event));
            }
        }

        pub(super) fn load(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
        ) -> Result<(), Code0SessionFailure> {
            if !on_ui(owner) || !live(owner, request) {
                return Err(Code0SessionFailure::PolicyRejected);
            }
            let (callbacks, _) = callback_state(owner);
            let _action = Active::enter(&callbacks);
            let (core, observer, containment) = {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, request).ok_or(Code0SessionFailure::PolicyRejected)?;
                let p = i
                    .policy
                    .as_mut()
                    .ok_or(Code0SessionFailure::PolicyRejected)?;
                if !installed(p)
                    || !p.barrier
                    || p.load_attempted
                    || i.callbacks.pending.load(Ordering::SeqCst) != 0
                    || i.callbacks.active.load(Ordering::SeqCst) != 1
                {
                    return Err(Code0SessionFailure::PolicyRejected);
                }
                p.load_attempted = true;
                (
                    Rc::clone(&p.core),
                    i.observer.clone(),
                    i.policy_fault.case == Some(PolicyFixtureCase::InstallRealmsNoPassthrough),
                )
            };
            require_live(owner, request).map_err(|_| Code0SessionFailure::PolicyRejected)?;
            // Counts only this compiled fixed-controlled call boundary; it
            // makes no claim about implicit/default browser navigation.
            count(owner, Count::Navigate);
            let result = unsafe {
                core.Navigate(if containment {
                    w!("https://console.code0.ai/__ai_meter_fixture__/containment-main")
                } else {
                    w!("https://console.code0.ai/__ai_meter_fixture__/install-p1")
                })
            };
            if result.is_err() || !live(owner, request) {
                failed(owner, request);
                return Err(Code0SessionFailure::NavigationFailed);
            }
            // A receipt requires the actual fixed Navigate to have returned
            // successfully; neither an observer nor a method entry invents it.
            if let Some(observer) = observer {
                observer(NativeSmokeObservation::Policy(
                    PolicyObservation::FirstControlledLoad(ControlledLoadReceipt {
                        request: request.clone(),
                    }),
                ));
            }
            Ok(())
        }

        fn sender_matches(
            owner: &Rc<RefCell<Owner>>,
            sender: Option<&ICoreWebView2>,
            mut trace: Option<&mut ResourceTrace>,
        ) -> windows::core::Result<()> {
            let core = core(owner);
            let expected = core.cast();
            if let Some(trace) = trace.as_deref_mut() {
                trace.observe_native(UT::Sender, &expected);
            }
            let expected: IUnknown = expected?;
            let sender = sender.ok_or_else(|| {
                if let Some(trace) = trace.as_deref_mut() {
                    trace.condition(UT::Sender, UR::Missing);
                }
                E_FAIL
            })?;
            let actual = sender.cast();
            if let Some(trace) = trace.as_deref_mut() {
                trace.observe_native(UT::Sender, &actual);
            }
            let actual: IUnknown = actual?;
            if expected.as_raw() == actual.as_raw() {
                Ok(())
            } else {
                if let Some(trace) = trace {
                    trace.condition(UT::Sender, UR::SenderMismatch);
                }
                Err(E_FAIL.into())
            }
        }

        // A failed/over-cap response is retained uncertainty, never a successful
        // unset response or a claimed transport cancellation. Stop is attempted
        // under this callback's Active guard, then ordinary R4 close is requested.
        fn uncertain_response(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            detail: DU,
        ) {
            let core = {
                let mut state = owner.borrow_mut();
                let Some(i) = matching(&mut state, request) else {
                    state.unmatched = true;
                    return;
                };
                i.failed = true;
                i.closing = true;
                if let Some(diagnostic) = i.callbacks.diagnostics.as_ref() {
                    diagnostic.uncertainty(detail);
                }
                i.cleanup_blocked = true;
                let Some(policy) = i.policy.as_ref().filter(|p| p.generation == GENERATION) else {
                    // Native Drop may reenter after the policy group was taken.
                    // Its missing emitter is sticky unknown, never an unwrap or
                    // an invented native reference/cleanup result.
                    if let Some(mailbox) = &i.policy_mailbox {
                        mailbox.unknown.store(true, Ordering::SeqCst);
                    }
                    state.unmatched = true;
                    return;
                };
                if i.stop_attempted {
                    None
                } else {
                    // Reserve once before any Stop reentry. Rc::clone does not
                    // call COM while the owner is borrowed; Active covers the
                    // actual native call and snapshot's later drop.
                    i.stop_attempted = true;
                    Some(Rc::clone(&policy.core))
                }
            };
            if let Some(core) = core {
                let _ = unsafe { core.Stop() };
            }
        }

        fn make_response(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            environment: &ICoreWebView2Environment,
            kind: ResponseKind,
            trace: &mut ResourceTrace,
        ) -> windows::core::Result<(IStream, ICoreWebView2WebResourceResponse, ResponseKind)>
        {
            let (body, headers) = match kind {
                ResponseKind::Main => (MAIN, HEADERS),
                ResponseKind::Refusal => (REFUSAL, HEADERS),
                ResponseKind::P8Main => (P8_MAIN, P8_MAIN_HEADERS),
                ResponseKind::P8Child => (P8_CHILD, P8_CHILD_HEADERS),
                ResponseKind::P8Worker => (P8_WORKER, P8_WORKER_HEADERS),
                ResponseKind::P8Probe(_) | ResponseKind::P8Other => {
                    (P8_REFUSAL, P8_REFUSAL_HEADERS)
                }
            };
            if body.len() > 65536 || u32::try_from(body.len()).is_err() {
                trace.condition(UT::Body, UR::BodyBound);
                return Err(E_FAIL.into());
            }
            let stream = unsafe { SHCreateMemStream(Some(body)) }.ok_or_else(|| {
                trace.condition(UT::Stream, UR::NullStream);
                windows::core::Error::from_hresult(E_OUTOFMEMORY)
            })?;
            if kind.allowed() && !live(owner, request) {
                trace.detail.downgraded = true;
                drop(stream);
                return make_response(owner, request, environment, kind.refusal(), trace);
            }
            trace.native(UT::Seek, unsafe { stream.Seek(0, STREAM_SEEK_SET, None) })?;
            if kind.allowed() && !live(owner, request) {
                trace.detail.downgraded = true;
                drop(stream);
                return make_response(owner, request, environment, kind.refusal(), trace);
            }
            let headers: Vec<u16> = headers.encode_utf16().chain(Some(0)).collect();
            let response = unsafe {
                environment.CreateWebResourceResponse(
                    &stream,
                    if kind.allowed() { 200 } else { 403 },
                    if kind.allowed() {
                        w!("OK")
                    } else {
                        w!("Forbidden")
                    },
                    PCWSTR(headers.as_ptr()),
                )
            };
            let response = trace.native(UT::CreateResponse, response)?;
            if kind.allowed() && !live(owner, request) {
                trace.detail.downgraded = true;
                drop(response);
                drop(stream);
                return make_response(owner, request, environment, kind.refusal(), trace);
            }
            Ok((stream, response, kind))
        }

        fn resource(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            generation: u64,
            sender: Option<&ICoreWebView2>,
            args: Option<&ICoreWebView2WebResourceRequestedEventArgs>,
            trace: &mut ResourceTrace,
        ) -> windows::core::Result<()> {
            let args = args.ok_or_else(|| {
                trace.condition(UT::Args, UR::Missing);
                E_FAIL
            })?;
            let (environment, within_bound, containment) = {
                let mut state = owner.borrow_mut();
                let i = state.instance.as_mut().ok_or_else(|| {
                    trace.condition(UT::Policy, UR::Missing);
                    E_FAIL
                })?;
                let p = i.policy.as_mut().ok_or_else(|| {
                    trace.condition(UT::Policy, UR::Missing);
                    E_FAIL
                })?;
                p.delivered_requests = p.delivered_requests.saturating_add(1);
                let containment =
                    i.policy_fault.case == Some(PolicyFixtureCase::InstallRealmsNoPassthrough);
                if containment {
                    i.policy_fault.containment.total = p.delivered_requests;
                }
                (
                    Rc::clone(&p.environment),
                    p.delivered_requests <= MAX_REQUESTS,
                    containment,
                )
            };
            let refused = if containment {
                ResponseKind::P8Other
            } else {
                ResponseKind::Refusal
            };
            let metadata = (|| -> windows::core::Result<ResponseKind> {
                sender_matches(owner, sender, Some(trace))?;
                let native_request = trace.native(UT::Request, unsafe { args.Request() })?;
                let uri = allocated_string(2048, |raw| {
                    trace.native(UT::Uri, unsafe { native_request.Uri(raw) })
                });
                let uri = trace.bounded(UT::Uri, uri)?;
                trace.detail.lane = diagnostic_lane(&uri);
                let method = allocated_string(16, |raw| {
                    trace.native(UT::Method, unsafe { native_request.Method(raw) })
                });
                let method = trace.bounded(UT::Method, method)?;
                trace.detail.method = if method == "GET" { 1 } else { 2 };
                let mut context = COREWEBVIEW2_WEB_RESOURCE_CONTEXT::default();
                trace.native(UT::Context, unsafe { args.ResourceContext(&mut context) })?;
                trace.detail.context_scalar = Some(context.0);
                trace.detail.context = if context == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT {
                    1
                } else if context == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_SCRIPT {
                    2
                } else if context == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FETCH {
                    3
                } else {
                    4
                };
                let state = owner.borrow();
                let i = state.instance.as_ref().ok_or_else(|| {
                    trace.condition(UT::Policy, UR::Missing);
                    E_FAIL
                })?;
                let p = i.policy.as_ref().ok_or_else(|| {
                    trace.condition(UT::Policy, UR::Missing);
                    E_FAIL
                })?;
                if !containment {
                    return Ok(
                        if &i.request == request
                            && p.generation == generation
                            && !i.closing
                            && !i.failed
                            && installed(p)
                            && p.barrier
                            && p.load_attempted
                            && !p.main_response
                            && within_bound
                            && uri == URI
                            && method == "GET"
                            && context == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT
                        {
                            ResponseKind::Main
                        } else {
                            ResponseKind::Refusal
                        },
                    );
                }
                if &i.request != request || p.generation != generation {
                    trace.condition(UT::Binding, UR::BindingMismatch);
                    return Err(E_FAIL.into());
                }
                // Classify actual fixed-path delivery before readiness downgrade:
                // a late duplicate must not become an incidental request.
                let (kind, expected_context) = if uri == P8_MAIN_URI {
                    (
                        ResponseKind::P8Main,
                        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT,
                    )
                } else if uri == P8_CHILD_URI {
                    (
                        ResponseKind::P8Child,
                        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT,
                    )
                } else if uri == P8_WORKER_URI {
                    (
                        ResponseKind::P8Worker,
                        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_SCRIPT,
                    )
                } else if let Some(index) = P8_PROBES.iter().position(|path| uri == *path) {
                    (
                        ResponseKind::P8Probe(index),
                        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FETCH,
                    )
                } else {
                    return Ok(ResponseKind::P8Other);
                };
                // This one fixed synthetic script asset may be delivered as
                // SCRIPT or OTHER; neither value proves a native worker realm.
                let context_matches = context == expected_context
                    || (uri == P8_WORKER_URI && context == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_OTHER)
                    || (matches!(kind, ResponseKind::P8Probe(_))
                        && context == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_XML_HTTP_REQUEST);
                if method != "GET" || !context_matches {
                    trace.condition(
                        UT::Binding,
                        if method != "GET" {
                            UR::MethodMismatch
                        } else {
                            UR::ContextMismatch
                        },
                    );
                    return Err(E_FAIL.into());
                }
                Ok(kind)
            })();
            trace.detail.metadata = if metadata.is_ok() {
                1
            } else if trace.detail.hresult.is_some() {
                3
            } else {
                2
            };
            let requested = metadata.as_ref().copied().unwrap_or(refused);
            let duplicate = if containment {
                let mut state = owner.borrow_mut();
                let i = state.instance.as_mut().ok_or_else(|| {
                    trace.condition(UT::Inventory, UR::Missing);
                    E_FAIL
                })?;
                let c = &mut i.policy_fault.containment;
                let index = requested.p8_index();
                let duplicate = index < 6 && c.deliveries[index] != 0;
                c.deliveries[index] = c.deliveries[index].saturating_add(1);
                duplicate
            } else {
                false
            };
            let ready = live(owner, request)
                && (!containment
                    || owner.borrow().instance.as_ref().is_some_and(|i| {
                        i.policy.as_ref().is_some_and(|p| {
                            installed(p) && p.barrier && p.load_attempted && within_bound
                        })
                    }));
            let kind = if duplicate || !ready {
                requested.refusal()
            } else {
                requested
            };
            trace.detail.downgraded |= requested.diagnostic_lane() != kind.diagnostic_lane();
            let pair = make_response(owner, request, &environment, kind, trace)?;
            let actual = pair.2;
            trace.detail.set_response = 1;
            let applied = unsafe { args.SetResponse(&pair.1) };
            trace.detail.set_response = if applied.is_ok() { 2 } else { 3 };
            // Drop caller aliases outside Owner borrows under callback Active.
            // Successful SetResponse is application, not consumption or cancellation.
            drop(pair);
            trace.native(UT::SetResponse, applied)?;
            {
                let mut state = owner.borrow_mut();
                let i = state.instance.as_mut().ok_or_else(|| {
                    trace.condition(UT::Inventory, UR::Missing);
                    E_FAIL
                })?;
                if containment {
                    let c = &mut i.policy_fault.containment;
                    let index = actual.p8_index();
                    c.responses[index] = c.responses[index].saturating_add(1);
                }
                let p = i.policy.as_mut().ok_or_else(|| {
                    trace.condition(UT::Inventory, UR::Missing);
                    E_FAIL
                })?;
                if matches!(actual, ResponseKind::Main | ResponseKind::P8Main)
                    && &i.request == request
                    && !i.closing
                    && !i.failed
                {
                    p.main_response = true;
                }
            }
            if metadata.is_err()
                || !within_bound
                || duplicate
                || actual.allowed() && !live(owner, request)
            {
                if metadata.is_ok() {
                    trace.condition(
                        UT::PostResponse,
                        if !within_bound {
                            UR::RequestLimit
                        } else if duplicate {
                            UR::Duplicate
                        } else {
                            UR::Revoked
                        },
                    );
                }
                return Err(E_FAIL.into());
            }
            Ok(())
        }

        pub(super) fn message(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            sender: Option<&ICoreWebView2>,
            args: Option<&ICoreWebView2WebMessageReceivedEventArgs>,
        ) -> windows::core::Result<()> {
            require_live(owner, request)?;
            sender_matches(owner, sender, None)?;
            require_live(owner, request)?;
            let args = args.ok_or(E_FAIL)?;
            let source = allocated_string(2048, |raw| unsafe { args.Source(raw) })?;
            require_live(owner, request)?;
            let payload = allocated_string(2048, |raw| unsafe { args.WebMessageAsJson(raw) })?;
            require_live(owner, request)?;
            let containment = owner.borrow().instance.as_ref().is_some_and(|i| {
                i.policy_fault.case == Some(PolicyFixtureCase::InstallRealmsNoPassthrough)
            });
            let expected_source = if containment { P8_MAIN_URI } else { URI };
            if source != expected_source || payload.len() > 2048 {
                return Err(E_FAIL.into());
            }
            if containment {
                let core = core(owner);
                let actual_source = allocated_string(2048, |raw| unsafe { core.Source(raw) })?;
                drop(core);
                require_live(owner, request)?;
                if actual_source != P8_MAIN_URI {
                    return Err(E_FAIL.into());
                }
            }
            let context = Code0BridgeContext {
                connection_epoch: request.connection_epoch.clone(),
                attempt_id: request.instance_id.clone(),
                source_url: source,
                is_main_frame: true, // Only actual core route, never Frame2.
            };
            let mut state = owner.borrow_mut();
            let i = matching(&mut state, request).ok_or(E_FAIL)?;
            let p = i.policy.as_mut().ok_or(E_FAIL)?;
            if !installed(p) || !p.barrier || !p.load_attempted || !p.main_response {
                return Err(E_FAIL.into());
            }
            if !p.ready_control {
                if p.validator.receive(payload.as_bytes(), &context) != Code0BridgeEvent::Control {
                    return Err(E_FAIL.into());
                }
                p.ready_control = true;
            } else if containment {
                let witness: ContainmentWitness =
                    serde_json::from_str(&payload).map_err(|_| E_FAIL)?;
                if p.document_complete
                    || witness.kind != "code0P8Complete"
                    || witness.nonce != P8_NONCE
                {
                    return Err(E_FAIL.into());
                }
                let page = [
                    witness.main,
                    witness.child,
                    witness.worker,
                    witness.child_detached,
                    witness.worker_terminate_returned,
                ];
                if page != [true; 5] {
                    return Err(E_FAIL.into());
                }
                i.policy_fault.containment.renderer = page;
                i.policy_fault.containment.source_matched = true;
                p.document_complete = true;
            } else {
                let expected = serde_json::json!({"kind":"code0InstallDocumentComplete","value":1});
                if p.document_complete
                    || serde_json::from_str::<serde_json::Value>(&payload).ok() != Some(expected)
                {
                    return Err(E_FAIL.into());
                }
                p.document_complete = true;
            }
            Ok(())
        }

        pub(super) fn registrations_removed(i: &Instance) -> bool {
            mailbox_quiescent(i)
                && i.policy.as_ref().is_none_or(|p| {
                    p.resource_token.is_none()
                        && p.chooser_token.is_none()
                        && !p.filter
                        && p.scripts.is_empty()
                })
        }

        pub(super) fn remove_registrations(owner: &Rc<RefCell<Owner>>) {
            // Caller already holds PollGuard and has drained pending/active0.
            // All native refs are Rc snapshots, dropped before controls_disposed.
            let has_policy = owner.borrow().instance.as_ref().unwrap().policy.is_some();
            if !has_policy {
                return;
            }
            let core = core(owner);
            let resource = {
                let mut state = owner.borrow_mut();
                let p = state.instance.as_mut().unwrap().policy.as_mut().unwrap();
                if !p.resource_remove_attempted && p.resource_token.is_some() {
                    p.resource_remove_attempted = true;
                    p.resource_token
                } else {
                    None
                }
            };
            if let Some(token) = resource {
                let result = unsafe { core.remove_WebResourceRequested(token) };
                if result.is_ok() {
                    owner
                        .borrow_mut()
                        .instance
                        .as_mut()
                        .unwrap()
                        .policy
                        .as_mut()
                        .unwrap()
                        .resource_token = None;
                    count(owner, Count::ResourceRemoval);
                }
                cleanup_at!(owner, ResourceRemove, result, true);
            }
            let chooser = {
                let mut state = owner.borrow_mut();
                let p = state.instance.as_mut().unwrap().policy.as_mut().unwrap();
                if !p.chooser_remove_attempted && p.chooser_token.is_some() {
                    p.chooser_remove_attempted = true;
                    p.receiver
                        .as_ref()
                        .map(|r| (Rc::clone(r), p.chooser_token.unwrap()))
                } else {
                    None
                }
            };
            if let Some((receiver, token)) = chooser {
                let result = unsafe { receiver.remove_DevToolsProtocolEventReceived(token) };
                if result.is_ok() {
                    owner
                        .borrow_mut()
                        .instance
                        .as_mut()
                        .unwrap()
                        .policy
                        .as_mut()
                        .unwrap()
                        .chooser_token = None;
                }
                cleanup_at!(owner, ChooserRemove, result, true);
            }
            let filter = {
                let mut state = owner.borrow_mut();
                let p = state.instance.as_mut().unwrap().policy.as_mut().unwrap();
                if p.filter && !p.filter_remove_attempted {
                    p.filter_remove_attempted = true;
                    p.core22.as_ref().map(Rc::clone)
                } else {
                    None
                }
            };
            if let Some(core22) = filter {
                let result = unsafe {
                    core22.RemoveWebResourceRequestedFilterWithRequestSourceKinds(
                        w!("*"),
                        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                        COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
                    )
                };
                if result.is_ok() {
                    owner
                        .borrow_mut()
                        .instance
                        .as_mut()
                        .unwrap()
                        .policy
                        .as_mut()
                        .unwrap()
                        .filter = false;
                    count(owner, Count::FilterRemoval);
                }
                cleanup_at!(owner, FilterRemove, result, true);
            }
            loop {
                let script = {
                    let mut state = owner.borrow_mut();
                    let p = state.instance.as_mut().unwrap().policy.as_mut().unwrap();
                    p.scripts.iter_mut().find(|(_, attempted)| !*attempted).map(
                        |(id, attempted)| {
                            *attempted = true;
                            id.clone()
                        },
                    )
                };
                let Some(id) = script else {
                    break;
                };
                let result = if id.contains('\0') {
                    Err(E_FAIL.into())
                } else {
                    let wide: Vec<u16> = id.encode_utf16().chain(Some(0)).collect();
                    unsafe { core.RemoveScriptToExecuteOnDocumentCreated(PCWSTR(wide.as_ptr())) }
                };
                if result.is_ok() {
                    owner
                        .borrow_mut()
                        .instance
                        .as_mut()
                        .unwrap()
                        .policy
                        .as_mut()
                        .unwrap()
                        .scripts
                        .retain(|(stored, _)| stored != &id);
                    count(owner, Count::ScriptRemoval);
                }
                cleanup_at!(owner, ScriptRemove, result, !id.contains('\0'));
            }
        }
    }

    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    pub(super) fn load_inline(
        owner: &Rc<RefCell<Owner>>,
        instance_id: &str,
    ) -> Result<(), Code0SessionFailure> {
        if !on_ui(owner) {
            return Err(Code0SessionFailure::PolicyRejected);
        }
        // Enter before cloning the COM action reference. This guard outlives
        // every action local and NavigateToString, including native-call reentry
        // after an individual renderer callback has already returned.
        let _action = {
            let state = owner.borrow();
            let instance = state
                .instance
                .as_ref()
                .ok_or(Code0SessionFailure::PolicyRejected)?;
            Active::enter(&instance.callbacks)
        };
        let (core, request) = {
            let mut state = owner.borrow_mut();
            let instance = state
                .instance
                .as_mut()
                .ok_or(Code0SessionFailure::PolicyRejected)?;
            if instance.request.instance_id != instance_id
                || instance.closing
                || instance.inline_loaded
                || instance.message_token.is_none()
            {
                return Err(Code0SessionFailure::PolicyRejected);
            }
            let core = instance
                .core
                .clone()
                .ok_or(Code0SessionFailure::PolicyRejected)?;
            instance.inline_loaded = true;
            (core, instance.request.clone())
        };
        const HTML: &str = "<!doctype html><meta charset=utf-8><title>Code0 native smoke</title><script>window.chrome.webview.postMessage({kind:'code0NativeSmoke',value:7});</script>";
        let html: Vec<u16> = HTML.encode_utf16().chain(std::iter::once(0)).collect();
        let result = native_call!(owner, Inline, unsafe {
            core.NavigateToString(PCWSTR(html.as_ptr()))
        });
        if result.is_ok() {
            milestone!(owner, InlineAccepted);
        }
        drop(core);
        result.map_err(|_| {
            failed(owner, &request);
            Code0SessionFailure::NavigationFailed
        })
    }
}

/// Release builds expose neither fixture loading nor native observations.
#[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
pub mod fixture {
    use std::path::PathBuf;
    use std::rc::Rc;

    use super::{Code0NativeDriver, Code0SessionFailure, Code0WindowRequest};

    /// Trusted debug-fixture handoff of the actual attempt's absolute end.
    /// No URL/script/root or duration/clock callback is accepted by this type.
    pub struct NativeDialogDeadline {
        expires_at: std::time::Instant,
        expired: std::cell::Cell<bool>,
    }
    impl NativeDialogDeadline {
        pub fn from_attempt_started(started: std::time::Instant) -> Option<Self> {
            Some(Self {
                expires_at: started.checked_add(std::time::Duration::from_secs(45))?,
                expired: std::cell::Cell::new(false),
            })
        }
        pub(super) fn before_original_at(&self, now: std::time::Instant) -> bool {
            if now >= self.expires_at {
                self.expired.set(true);
            }
            !self.expired.get()
        }
        pub(super) fn run_if_authorized_at<T>(
            &self,
            now: std::time::Instant,
            identity_live: bool,
            action: impl FnOnce() -> T,
        ) -> Option<T> {
            if self.before_original_at(now) && identity_live {
                Some(action())
            } else {
                None
            }
        }
        pub(super) fn disposal_within_close_at(
            received: std::time::Instant,
            consumed: std::time::Instant,
            closing: Option<std::time::Instant>,
        ) -> bool {
            closing.is_none_or(|at| {
                at.checked_add(std::time::Duration::from_secs(10))
                    .is_some_and(|end| received < end && consumed < end)
            })
        }
        pub(super) fn completion_timely_at(
            &self,
            received: std::time::Instant,
            consumed: std::time::Instant,
            closing: Option<std::time::Instant>,
        ) -> bool {
            self.before_original_at(received)
                && self.before_original_at(consumed)
                && closing.is_none_or(|at| self.before_original_at(at))
                && Self::disposal_within_close_at(received, consumed, closing)
        }
    }

    /// The same immutable 45-second handoff, restricted to selected N1/N2.
    pub type NativeRefusalDeadline = NativeDialogDeadline;

    // One native event owns one refusal action. Time/identity observations can
    // only remove its eventual positive witness; they never suppress refusal.
    pub(super) struct LegacyRefusalEvent {
        canceled: bool,
        timely: bool,
    }
    impl LegacyRefusalEvent {
        pub(super) fn new() -> Self {
            Self {
                canceled: false,
                timely: true,
            }
        }
        pub(super) fn cancel_once<T>(&mut self, action: impl FnOnce() -> T) -> Option<T> {
            if self.canceled {
                return None;
            }
            self.canceled = true;
            Some(action())
        }
        pub(super) fn observe_at(
            &mut self,
            deadline: &NativeDialogDeadline,
            now: std::time::Instant,
            identity: bool,
        ) -> bool {
            let original = deadline.before_original_at(now);
            self.timely &= original && identity;
            self.timely
        }
        pub(super) fn qualify_at(
            &mut self,
            deadline: &NativeDialogDeadline,
            now: std::time::Instant,
            identity: bool,
        ) -> bool {
            self.observe_at(deadline, now, identity) && self.canceled
        }
    }
    /// Debug N1/N2 cleanup adoption only; preserves the native first timestamp.
    /// It confers no target or completion success and never resets close10.
    pub fn adopt_legacy_native_close(
        closing: &mut Option<std::time::Instant>,
        native: Option<std::time::Instant>,
    ) -> Option<std::time::Instant> {
        if closing.is_some() {
            return None;
        }
        let at = native?;
        *closing = Some(at);
        Some(at)
    }
    // Selected Child's first revocation exists before an invocation/mailbox.
    // Optional actual submitted budget is synchronized, never manufactured.
    pub(super) struct LegacyChildClose {
        first: Option<std::time::Instant>,
    }
    impl LegacyChildClose {
        pub(super) fn new() -> Self {
            Self { first: None }
        }
        pub(super) fn mark_at(
            &mut self,
            now: std::time::Instant,
            submitted_closing: Option<&mut Option<std::time::Instant>>,
        ) -> Option<std::time::Instant> {
            let first_publication = self.first.is_none();
            let existing = submitted_closing.as_ref().and_then(|closing| **closing);
            let at = *self.first.get_or_insert(existing.unwrap_or(now));
            if let Some(closing) = submitted_closing {
                *closing = Some(at);
            }
            first_publication.then_some(at)
        }
    }
    pub(super) struct LegacyCompletionInput {
        pub(super) identity_valid: bool,
        pub(super) hresult_succeeded: bool,
        pub(super) bounded_result_valid: bool,
        pub(super) fixed_result_matched: bool,
    }
    pub(super) struct LegacyCompletionDisposition {
        pub(super) timely: bool,
        pub(super) disposable: bool,
        pub(super) successful: bool,
    }
    pub(super) fn legacy_completion_disposition_at(
        deadline: &NativeDialogDeadline,
        received: std::time::Instant,
        consumed: std::time::Instant,
        closing: Option<std::time::Instant>,
        input: LegacyCompletionInput,
    ) -> LegacyCompletionDisposition {
        let timely = deadline.completion_timely_at(received, consumed, closing);
        let disposable = input.identity_valid
            && NativeDialogDeadline::disposal_within_close_at(received, consumed, closing)
            && (!input.hresult_succeeded
                || input.bounded_result_valid && input.fixed_result_matched);
        LegacyCompletionDisposition {
            timely,
            disposable,
            successful: disposable
                && timely
                && input.hresult_succeeded
                && input.bounded_result_valid
                && input.fixed_result_matched,
        }
    }

    pub(super) fn popup_result_units(units: &[u16]) -> (bool, bool) {
        let valid =
            units.len() < 64 && char::decode_utf16(units.iter().copied()).all(|v| v.is_ok());
        (
            valid,
            valid
                && units
                    .iter()
                    .copied()
                    .eq("\"code0-popup-submitted-v1\"".encode_utf16()),
        )
    }

    #[cfg(test)]
    mod popup_contract_tests {
        use super::popup_result_units;

        // A permissive JSON/string parser would accept preflight failure,
        // a missing quoted token, malformed UTF16 or an oversized callback.
        #[test]
        fn popup_completion_accepts_only_bounded_exact_json_token() {
            let exact: Vec<u16> = "\"code0-popup-submitted-v1\"".encode_utf16().collect();
            assert_eq!(popup_result_units(&exact), (true, true));
            for text in [
                "code0-popup-submitted-v1",
                "null",
                "\"code0-popup-precondition-v1\"",
                "\"code0-popup-submitted-v1\" ",
                "",
            ] {
                assert_eq!(
                    popup_result_units(&text.encode_utf16().collect::<Vec<_>>()),
                    (true, false)
                );
            }
            assert_eq!(popup_result_units(&[0xd800]), (false, false));
            assert_eq!(popup_result_units(&[65; 63]), (true, false));
            assert_eq!(popup_result_units(&[65; 64]), (false, false));
        }
    }

    #[cfg(test)]
    mod legacy_original_deadline_tests {
        use super::{LegacyRefusalEvent, NativeDialogDeadline};
        use std::cell::Cell;
        use std::time::{Duration, Instant};

        fn at(start: Instant, millis: u64) -> Instant {
            start.checked_add(Duration::from_millis(millis)).unwrap()
        }

        #[test]
        fn child_no_submission_first_close_survives_delayed_return_and_adoption() {
            for revoked_at_ms in [44999, 45001] {
                let start = Instant::now();
                let deadline = NativeDialogDeadline::from_attempt_started(start).unwrap();
                let mut native_close = super::LegacyChildClose::new();
                // Primitive optional submitted budget only: no ChildState,
                // handler/mailbox, invocation or completion ticket exists.
                let mut submitted_closing = None::<Option<Instant>>;
                let t_c = at(start, revoked_at_ms);
                let publication = native_close.mark_at(t_c, submitted_closing.as_mut());
                assert_eq!(publication, Some(t_c));
                assert_eq!(submitted_closing, None);
                let returned_at = Cell::new(t_c);
                struct ReturnLocal<'a>(&'a Cell<Instant>, Instant);
                impl Drop for ReturnLocal<'_> {
                    fn drop(&mut self) {
                        self.0.set(self.1);
                    }
                }
                drop(ReturnLocal(&returned_at, at(start, revoked_at_ms + 12000)));
                let mut adopted = None;
                assert_eq!(
                    super::adopt_legacy_native_close(&mut adopted, publication),
                    Some(t_c)
                );
                assert_ne!(adopted, Some(returned_at.get()));
                let repeated_publication =
                    native_close.mark_at(returned_at.get(), submitted_closing.as_mut());
                assert!(repeated_publication.is_none());
                assert!(
                    super::adopt_legacy_native_close(&mut adopted, repeated_publication).is_none()
                );
                assert_eq!(adopted, Some(t_c));
                assert_eq!(submitted_closing, None);
                let invocations = Cell::new(0);
                assert!(
                    deadline
                        .run_if_authorized_at(returned_at.get(), true, || invocations
                            .set(invocations.get() + 1))
                        .is_none()
                );
                assert_eq!(invocations.get(), 0);
                assert!(NativeDialogDeadline::disposal_within_close_at(
                    t_c,
                    at(start, revoked_at_ms + 9999),
                    adopted
                ));
                assert!(!NativeDialogDeadline::disposal_within_close_at(
                    t_c,
                    at(start, revoked_at_ms + 10000),
                    adopted
                ));
            }
        }

        #[test]
        fn child_submitted_budget_and_independent_first_close_stay_the_same() {
            let start = Instant::now();
            let mut first = super::LegacyChildClose::new();
            let mut submitted = None;
            assert_eq!(
                first.mark_at(at(start, 44000), Some(&mut submitted)),
                Some(at(start, 44000))
            );
            assert_eq!(submitted, Some(at(start, 44000)));
            assert!(
                first
                    .mark_at(at(start, 48000), Some(&mut submitted))
                    .is_none()
            );
            assert_eq!(submitted, Some(at(start, 44000)));
            // An already recorded actual submitted budget cannot be rebased.
            let mut adopted_existing = super::LegacyChildClose::new();
            assert_eq!(
                adopted_existing.mark_at(at(start, 50000), Some(&mut submitted)),
                Some(at(start, 44000))
            );
            assert_eq!(submitted, Some(at(start, 44000)));
        }

        #[test]
        fn delayed_ready_uses_original_end_for_both_selected_actions() {
            for _ in ["N1", "N2"] {
                let start = Instant::now();
                let deadline = NativeDialogDeadline::from_attempt_started(start).unwrap();
                let calls = Cell::new(0);
                assert!(
                    deadline
                        .run_if_authorized_at(at(start, 45000), true, || calls.set(calls.get() + 1))
                        .is_none()
                );
                assert!(
                    deadline
                        .run_if_authorized_at(at(start, 44999), true, || calls.set(calls.get() + 1))
                        .is_none()
                );
                assert_eq!(calls.get(), 0);
            }
        }

        #[test]
        fn reentrant_source_and_native_local_drop_deny_the_next_privileged_step() {
            let start = Instant::now();
            let deadline = NativeDialogDeadline::from_attempt_started(start).unwrap();
            let source_calls = Cell::new(0);
            let submit_calls = Cell::new(0);
            let clock = Cell::new(at(start, 44999));
            struct NativeLocal<'a>(&'a Cell<Instant>, Instant);
            impl Drop for NativeLocal<'_> {
                fn drop(&mut self) {
                    self.0.set(self.1);
                }
            }
            let local = deadline
                .run_if_authorized_at(clock.get(), true, || {
                    source_calls.set(source_calls.get() + 1);
                    NativeLocal(&clock, at(start, 45001))
                })
                .unwrap();
            drop(local);
            assert!(
                deadline
                    .run_if_authorized_at(clock.get(), true, || submit_calls
                        .set(submit_calls.get() + 1))
                    .is_none()
            );
            assert_eq!(source_calls.get(), 1);
            assert_eq!(submit_calls.get(), 0);
        }

        #[test]
        fn getters_native_errors_and_local_drops_cannot_publish_late_witnesses() {
            for boundary in [
                "identity",
                "metadata",
                "nativeReadback",
                "errorDrop",
                "argsDrop",
                "senderDrop",
            ] {
                let start = Instant::now();
                let deadline = NativeDialogDeadline::from_attempt_started(start).unwrap();
                let mut event = LegacyRefusalEvent::new();
                let cancels = Cell::new(0);
                assert!(event.observe_at(&deadline, at(start, 44999), true));
                let drop_time = Cell::new(at(start, 44999));
                struct Local<'a>(&'a Cell<Instant>, Instant);
                impl Drop for Local<'_> {
                    fn drop(&mut self) {
                        self.0.set(self.1);
                    }
                }
                let local = Local(&drop_time, at(start, 45001));
                let native_result: Result<Local<'_>, Local<'_>> = event
                    .cancel_once(|| {
                        cancels.set(cancels.get() + 1);
                        if boundary == "errorDrop" {
                            Err(local)
                        } else {
                            Ok(local)
                        }
                    })
                    .unwrap();
                drop(native_result);
                let publishes = Cell::new(0);
                if event.qualify_at(&deadline, drop_time.get(), true) {
                    publishes.set(publishes.get() + 1);
                }
                assert_eq!(cancels.get(), 1, "{boundary}");
                assert_eq!(publishes.get(), 0, "{boundary}");
            }
        }

        #[test]
        fn refusal_runs_once_at_before_exact_and_after_end_but_witness_is_strict() {
            for millis in [44999, 45000, 45001] {
                let start = Instant::now();
                let deadline = NativeDialogDeadline::from_attempt_started(start).unwrap();
                let mut event = LegacyRefusalEvent::new();
                let cancels = Cell::new(0);
                event.observe_at(&deadline, at(start, millis), true);
                assert!(
                    event
                        .cancel_once(|| cancels.set(cancels.get() + 1))
                        .is_some()
                );
                assert!(
                    event
                        .cancel_once(|| cancels.set(cancels.get() + 1))
                        .is_none()
                );
                assert_eq!(cancels.get(), 1);
                assert_eq!(
                    event.qualify_at(&deadline, at(start, millis), true),
                    millis < 45000
                );
            }
        }

        #[test]
        fn closing_adoption_cannot_revive_pre_adoption_or_same_tick_expiry() {
            for close_already_visible in [true, false] {
                let start = Instant::now();
                let deadline = NativeDialogDeadline::from_attempt_started(start).unwrap();
                let mut event = LegacyRefusalEvent::new();
                let first_close = Cell::new(None);
                assert!(
                    event
                        .cancel_once(|| first_close.set(Some(at(start, 44999))))
                        .is_some()
                );
                let mut adopted = None;
                let adopts = Cell::new(0);
                if close_already_visible
                    && super::adopt_legacy_native_close(&mut adopted, first_close.get()).is_some()
                {
                    adopts.set(adopts.get() + 1);
                }
                assert!(!event.qualify_at(&deadline, at(start, 45001), true));
                if super::adopt_legacy_native_close(&mut adopted, first_close.get()).is_some() {
                    adopts.set(adopts.get() + 1);
                }
                assert_eq!(adopts.get(), 1);
                assert_eq!(adopted, Some(at(start, 44999)));
                assert!(!event.qualify_at(&deadline, at(start, 44999), true));
            }
        }

        #[test]
        fn completion_original_received_consumed_and_close_bounds_are_independent() {
            for (received, consumed, timely) in [
                (44998, 44999, true),
                (45000, 45001, false),
                (44999, 45000, false),
            ] {
                let start = Instant::now();
                let deadline = NativeDialogDeadline::from_attempt_started(start).unwrap();
                let closing = Some(at(start, 44000));
                assert_eq!(
                    deadline.completion_timely_at(
                        at(start, received),
                        at(start, consumed),
                        closing
                    ),
                    timely
                );
                let disposed = Cell::new(0);
                if NativeDialogDeadline::disposal_within_close_at(
                    at(start, received),
                    at(start, consumed),
                    closing,
                ) {
                    disposed.set(disposed.get() + 1);
                }
                assert_eq!(disposed.get(), 1);
            }
            let start = Instant::now();
            assert!(!NativeDialogDeadline::disposal_within_close_at(
                at(start, 45001),
                at(start, 54000),
                Some(at(start, 44000))
            ));
        }

        #[test]
        fn timely_completion_allows_cleanup_after45_but_never_resets_first_close10() {
            let start = Instant::now();
            let deadline = NativeDialogDeadline::from_attempt_started(start).unwrap();
            let mut event = LegacyRefusalEvent::new();
            let native_close = Some(at(start, 44000));
            assert!(event.cancel_once(|| ()).is_some());
            assert!(event.qualify_at(&deadline, at(start, 44001), true));
            let completion = super::legacy_completion_disposition_at(
                &deadline,
                at(start, 44998),
                at(start, 44999),
                native_close,
                super::LegacyCompletionInput {
                    identity_valid: true,
                    hresult_succeeded: true,
                    bounded_result_valid: true,
                    fixed_result_matched: true,
                },
            );
            assert!(completion.successful && completion.disposable);
            let mut adopted = None;
            assert_eq!(
                super::adopt_legacy_native_close(&mut adopted, native_close),
                native_close
            );
            assert!(
                super::adopt_legacy_native_close(&mut adopted, Some(at(start, 50000))).is_none()
            );
            assert_eq!(adopted, native_close);
            assert!(NativeDialogDeadline::disposal_within_close_at(
                at(start, 44998),
                at(start, 53999),
                adopted
            ));
            assert!(!NativeDialogDeadline::disposal_within_close_at(
                at(start, 44998),
                at(start, 54000),
                adopted
            ));
        }

        #[test]
        fn actual_failed_or_late_completion_disposes_without_success_or_revival() {
            for hresult_ok in [true, false] {
                let start = Instant::now();
                let deadline = NativeDialogDeadline::from_attempt_started(start).unwrap();
                assert!(!deadline.before_original_at(at(start, 45000)));
                let disposition = super::legacy_completion_disposition_at(
                    &deadline,
                    at(start, 45001),
                    at(start, 45002),
                    Some(at(start, 44000)),
                    super::LegacyCompletionInput {
                        identity_valid: true,
                        hresult_succeeded: hresult_ok,
                        bounded_result_valid: hresult_ok,
                        fixed_result_matched: hresult_ok,
                    },
                );
                assert!(disposition.disposable);
                assert!(!disposition.timely);
                assert!(!disposition.successful);
                for _later in ["token", "SessionFailure", "Released"] {
                    assert!(
                        deadline
                            .run_if_authorized_at(at(start, 44999), true, || ())
                            .is_none()
                    );
                }
            }
        }
    }

    #[cfg(test)]
    mod dialog_original_deadline_tests {
        use super::NativeDialogDeadline;
        use std::cell::Cell;
        use std::time::{Duration, Instant};

        fn at(started: Instant, millis: u64) -> Instant {
            started.checked_add(Duration::from_millis(millis)).unwrap()
        }

        #[test]
        fn late_engine_ready_cannot_renew_the_original_attempt() {
            let started = Instant::now();
            let deadline = NativeDialogDeadline::from_attempt_started(started).unwrap();
            let navigations = Cell::new(0);
            // The same path that issues native actions receives original t0,
            // even though engine readiness itself arrives after the budget.
            assert!(
                deadline
                    .run_if_authorized_at(at(started, 46000), true, || {
                        navigations.set(navigations.get() + 1);
                    })
                    .is_none()
            );
            assert_eq!(navigations.get(), 0);
        }

        #[test]
        fn reentrant_getter_crossing_deadline_prevents_the_next_action() {
            let started = Instant::now();
            let deadline = NativeDialogDeadline::from_attempt_started(started).unwrap();
            let now = Cell::new(at(started, 44990));
            let getters = Cell::new(0);
            let alerts = Cell::new(0);
            assert!(
                deadline
                    .run_if_authorized_at(now.get(), true, || {
                        getters.set(getters.get() + 1);
                        now.set(at(started, 45010));
                    })
                    .is_some()
            );
            // Identity stayed live. The shared grant path still denies alert.
            assert!(
                deadline
                    .run_if_authorized_at(now.get(), true, || {
                        alerts.set(alerts.get() + 1);
                    })
                    .is_none()
            );
            assert_eq!((getters.get(), alerts.get()), (1, 0));
        }

        #[test]
        fn action_local_drop_crossing_deadline_prevents_success_publication() {
            struct Local<'a> {
                now: &'a Cell<Instant>,
                after_drop: Instant,
                drops: &'a Cell<u8>,
            }
            impl Drop for Local<'_> {
                fn drop(&mut self) {
                    self.now.set(self.after_drop);
                    self.drops.set(self.drops.get() + 1);
                }
            }
            let started = Instant::now();
            let deadline = NativeDialogDeadline::from_attempt_started(started).unwrap();
            let now = Cell::new(at(started, 44990));
            let drops = Cell::new(0);
            let publications = Cell::new(0);
            let local = deadline
                .run_if_authorized_at(now.get(), true, || Local {
                    now: &now,
                    after_drop: at(started, 45000),
                    drops: &drops,
                })
                .unwrap();
            drop(local);
            assert!(
                deadline
                    .run_if_authorized_at(now.get(), true, || {
                        publications.set(publications.get() + 1);
                    })
                    .is_none()
            );
            assert_eq!((drops.get(), publications.get()), (1, 0));
        }

        #[test]
        fn exact_deadline_denies_and_before_deadline_control_executes_once() {
            let started = Instant::now();
            let deadline = NativeDialogDeadline::from_attempt_started(started).unwrap();
            let actions = Cell::new(0);
            assert!(
                deadline
                    .run_if_authorized_at(at(started, 44999), true, || {
                        actions.set(actions.get() + 1);
                    })
                    .is_some()
            );
            assert!(
                deadline
                    .run_if_authorized_at(at(started, 45000), true, || {
                        actions.set(actions.get() + 1);
                    })
                    .is_none()
            );
            assert_eq!(actions.get(), 1);
        }

        #[test]
        fn later_closing_or_completion_cannot_revive_expired_authority() {
            let started = Instant::now();
            let deadline = NativeDialogDeadline::from_attempt_started(started).unwrap();
            let actions = Cell::new(0);
            assert!(
                deadline
                    .run_if_authorized_at(at(started, 45000), true, || {
                        actions.set(actions.get() + 1);
                    })
                    .is_none()
            );
            assert!(!deadline.completion_timely_at(
                at(started, 45100),
                at(started, 45200),
                Some(at(started, 45050))
            ));
            // Even a historically earlier timestamp cannot reset a latched expiry.
            assert!(
                deadline
                    .run_if_authorized_at(at(started, 44000), true, || {
                        actions.set(actions.get() + 1);
                    })
                    .is_none()
            );
            assert_eq!(actions.get(), 0);
        }

        #[test]
        fn completion_checks_original_invocation_consumption_and_close_budget() {
            let started = Instant::now();
            let before = NativeDialogDeadline::from_attempt_started(started).unwrap();
            assert!(before.completion_timely_at(
                at(started, 44000),
                at(started, 44500),
                Some(at(started, 44250))
            ));
            assert!(!before.completion_timely_at(
                at(started, 44000),
                at(started, 45000),
                Some(at(started, 44250))
            ));
            let close = NativeDialogDeadline::from_attempt_started(started).unwrap();
            assert!(close.completion_timely_at(
                at(started, 29000),
                at(started, 29999),
                Some(at(started, 20000))
            ));
            assert!(!close.completion_timely_at(
                at(started, 29000),
                at(started, 30000),
                Some(at(started, 20000))
            ));
        }
    }

    /// Fixed required S2 registrations. Redirects reuse both navigation hooks.
    #[derive(Clone, Copy)]
    #[repr(u8)]
    pub enum NativeRefusalHook {
        TopNavigation,
        ChildNavigation,
        ExternalProtocol,
        NewWindow,
        Download,
        ScriptDialog,
        Permission,
        BasicAuthentication,
        ClientCertificate,
        TlsError,
    }
    impl NativeRefusalHook {
        pub(super) const ALL: [Self; 10] = [
            Self::TopNavigation,
            Self::ChildNavigation,
            Self::ExternalProtocol,
            Self::NewWindow,
            Self::Download,
            Self::ScriptDialog,
            Self::Permission,
            Self::BasicAuthentication,
            Self::ClientCertificate,
            Self::TlsError,
        ];
        pub(super) fn bit(self) -> u16 {
            1u16 << self as u8
        }
        pub fn name(self) -> &'static str {
            match self {
                Self::TopNavigation => "topNavigation",
                Self::ChildNavigation => "childNavigation",
                Self::ExternalProtocol => "externalProtocol",
                Self::NewWindow => "newWindow",
                Self::Download => "download",
                Self::ScriptDialog => "scriptDialog",
                Self::Permission => "permission",
                Self::BasicAuthentication => "basicAuthentication",
                Self::ClientCertificate => "clientCertificate",
                Self::TlsError => "tlsError",
            }
        }
    }

    /// Read-only token inventory, constructed only after live native owner
    /// validation. No public constructor, installer or installed-counter setter.
    pub struct NativeRefusalInstallReceipt {
        pub(super) request: Code0WindowRequest,
        pub(super) installed_mask: u16,
        pub(super) not_installed: Vec<NativeRefusalHook>,
        pub(super) load_rejected: bool,
        pub(super) installation: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }
    impl NativeRefusalInstallReceipt {
        pub fn matches(&self, request: &Code0WindowRequest) -> bool {
            &self.request == request
        }
        pub fn installed_mask(&self) -> u16 {
            self.installed_mask
        }
        pub fn load_rejected(&self) -> bool {
            self.load_rejected
        }
        pub fn not_installed(&self) -> &[NativeRefusalHook] {
            &self.not_installed
        }
    }

    /// Primitive native callback audit only; no native references or identifiers.
    #[derive(Clone, Copy, Default, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct NativeTopRefusalFacts {
        pub event_kind: &'static str,
        pub action_kind: &'static str,
        pub target_uri_matched: bool,
        pub native_sender_matched: bool,
        pub request_generation_live_matched: bool,
        pub navigation_id_getter_succeeded: bool,
        pub redirected_getter_succeeded: bool,
        pub is_redirected: bool,
        pub issued_before_event: bool,
        pub cancel_attempts: u8,
        pub cancel_returns: u8,
        pub cancel_successes: u8,
        pub cancel_readbacks: u8,
        pub cancel_readback_succeeded: bool,
        pub native_cancel_true: bool,
        pub live_after_getters: bool,
        pub native_action_before_receipt: bool,
    }
    pub struct NativeTopRefusalReceipt {
        pub(super) request: Code0WindowRequest,
        pub(super) installation: std::sync::Arc<std::sync::atomic::AtomicBool>,
        pub(super) facts: NativeTopRefusalFacts,
    }
    impl NativeTopRefusalReceipt {
        pub fn matches(
            &self,
            request: &Code0WindowRequest,
            install: &NativeRefusalInstallReceipt,
        ) -> bool {
            &self.request == request
                && install.matches(request)
                && std::sync::Arc::ptr_eq(&self.installation, &install.installation)
        }
        pub fn facts(&self) -> NativeTopRefusalFacts {
            self.facts
        }
    }
    pub struct NativeTopSubmissionReceipt {
        pub(super) request: Code0WindowRequest,
        pub(super) installation: std::sync::Arc<std::sync::atomic::AtomicBool>,
        pub(super) native_submit_ok: bool,
        pub(super) original_timely: bool,
    }
    impl NativeTopSubmissionReceipt {
        pub fn matches(
            &self,
            request: &Code0WindowRequest,
            install: &NativeRefusalInstallReceipt,
        ) -> bool {
            &self.request == request
                && install.matches(request)
                && std::sync::Arc::ptr_eq(&self.installation, &install.installation)
        }
        /// Internal attempt qualification, never serialized as a native return.
        pub fn original_attempt_timely(&self) -> bool {
            self.original_timely
        }
        pub fn native_submit_ok(&self) -> bool {
            self.native_submit_ok
        }
    }

    #[derive(Clone, Copy, Default, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct NativeChildRefusalFacts {
        pub event_kind: &'static str,
        pub action_kind: &'static str,
        pub target_uri_matched: bool,
        pub native_sender_matched: bool,
        pub request_generation_live_matched: bool,
        pub navigation_id_getter_succeeded: bool,
        pub redirected_getter_succeeded: bool,
        pub is_redirected: bool,
        pub issued_before_event: bool,
        pub cancel_attempts: u8,
        pub cancel_returns: u8,
        pub cancel_successes: u8,
        pub cancel_readbacks: u8,
        pub cancel_readback_succeeded: bool,
        pub native_cancel_true: bool,
        pub live_after_getters: bool,
        pub native_action_before_receipt: bool,
    }
    pub struct NativeChildRefusalReceipt {
        pub(super) request: Code0WindowRequest,
        pub(super) installation: std::sync::Arc<std::sync::atomic::AtomicBool>,
        pub(super) facts: NativeChildRefusalFacts,
    }
    pub struct NativeChildSubmissionReceipt {
        pub(super) request: Code0WindowRequest,
        pub(super) installation: std::sync::Arc<std::sync::atomic::AtomicBool>,
        pub(super) native_submit_ok: bool,
        pub(super) initial_source_matched: bool,
        pub(super) original_timely: bool,
    }
    #[derive(Clone, Copy)]
    pub struct NativeChildCompletionFacts {
        pub identity_matched: bool,
        pub hresult_succeeded: bool,
        pub bounded_result_valid: bool,
        pub fixed_result_matched: bool,
        pub before_release: bool,
        pub within_deadline: bool,
    }
    pub struct NativeChildCompletionReceipt {
        pub(super) request: Code0WindowRequest,
        pub(super) installation: std::sync::Arc<std::sync::atomic::AtomicBool>,
        pub(super) facts: NativeChildCompletionFacts,
    }
    pub struct NativeChildFinalReceipt {
        pub(super) request: Code0WindowRequest,
        pub(super) installation: std::sync::Arc<std::sync::atomic::AtomicBool>,
        pub(super) obligation_drained: bool,
        pub(super) handlers_drained: bool,
    }
    macro_rules! child_receipt_matches {
        ($receipt:ty) => {
            impl $receipt {
                pub fn matches(
                    &self,
                    request: &Code0WindowRequest,
                    install: &NativeRefusalInstallReceipt,
                ) -> bool {
                    &self.request == request
                        && install.matches(request)
                        && std::sync::Arc::ptr_eq(&self.installation, &install.installation)
                }
            }
        };
    }
    child_receipt_matches!(NativeChildRefusalReceipt);
    child_receipt_matches!(NativeChildSubmissionReceipt);
    child_receipt_matches!(NativeChildCompletionReceipt);
    child_receipt_matches!(NativeChildFinalReceipt);
    impl NativeChildRefusalReceipt {
        pub fn facts(&self) -> NativeChildRefusalFacts {
            self.facts
        }
    }
    impl NativeChildSubmissionReceipt {
        /// Internal attempt qualification, never serialized as a native return.
        pub fn original_attempt_timely(&self) -> bool {
            self.original_timely
        }
        pub fn native_submit_ok(&self) -> bool {
            self.native_submit_ok
        }
        pub fn initial_source_matched(&self) -> bool {
            self.initial_source_matched
        }
    }
    impl NativeChildCompletionReceipt {
        pub fn facts(&self) -> NativeChildCompletionFacts {
            self.facts
        }
    }
    impl NativeChildFinalReceipt {
        pub fn obligation_drained(&self) -> bool {
            self.obligation_drained
        }
        pub fn handlers_drained(&self) -> bool {
            self.handlers_drained
        }
    }

    /// N3 primitive facts only. No path, URI, native ID, message or token escapes.
    #[derive(Clone, Copy, Default, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct NativeDialogFacts {
        pub navigation_submissions: u8,
        pub navigation_starts: u8,
        pub navigation_completions: u8,
        pub script_submissions: u8,
        pub dialog_events: u8,
        pub script_completions: u8,
        pub completed_remove_attempts: u8,
        pub asset_binding_validated: bool,
        pub initial_source_matched: bool,
        pub completed_handler_added: bool,
        pub settings_before_document: bool,
        pub navigation_submission_returned: bool,
        pub navigation_submit_ok: bool,
        pub start_uri_matched: bool,
        pub start_sender_matched: bool,
        pub start_live_after_getters: bool,
        pub start_id_getter_succeeded: bool,
        pub start_redirected_getter_succeeded: bool,
        pub start_not_redirected: bool,
        pub start_cancel_readback_succeeded: bool,
        pub start_not_canceled: bool,
        pub grant_consumed: bool,
        pub completed_sender_matched: bool,
        pub completed_id_getter_succeeded: bool,
        pub completed_success_getter_succeeded: bool,
        pub completed_success: bool,
        pub navigation_id_matched: bool,
        pub completed_live_after_getters: bool,
        pub source_after_complete_matched: bool,
        pub fresh_document: bool,
        pub source_before_script_matched: bool,
        pub script_submission_returned: bool,
        pub script_submit_ok: bool,
        pub dialog_kind_alert: bool,
        pub dialog_message_matched: bool,
        pub dialog_uri_matched: bool,
        pub dialog_sender_matched: bool,
        pub dialog_live_after_getters: bool,
        pub non_accept_action_returned: bool,
        pub completion_identity_matched: bool,
        pub completion_hresult_succeeded: bool,
        pub bounded_result_valid: bool,
        pub fixed_result_matched: bool,
        pub completion_before_release: bool,
        pub completion_within_deadline: bool,
        pub completion_disposition_consumed: bool,
        pub completion_obligation_drained: bool,
        pub completion_handlers_drained: bool,
        pub completed_remove_succeeded: bool,
        pub completed_same_emitter: bool,
    }
    impl NativeDialogFacts {
        pub fn positive(&self) -> bool {
            self.navigation_submissions == 1
                && self.navigation_starts == 1
                && self.navigation_completions == 1
                && self.script_submissions == 1
                && self.dialog_events == 1
                && self.script_completions == 1
                && self.completed_remove_attempts == 1
                && self.asset_binding_validated
                && self.initial_source_matched
                && self.completed_handler_added
                && self.settings_before_document
                && self.navigation_submission_returned
                && self.navigation_submit_ok
                && self.start_uri_matched
                && self.start_sender_matched
                && self.start_live_after_getters
                && self.start_id_getter_succeeded
                && self.start_redirected_getter_succeeded
                && self.start_not_redirected
                && self.start_cancel_readback_succeeded
                && self.start_not_canceled
                && self.grant_consumed
                && self.completed_sender_matched
                && self.completed_id_getter_succeeded
                && self.completed_success_getter_succeeded
                && self.completed_success
                && self.navigation_id_matched
                && self.completed_live_after_getters
                && self.source_after_complete_matched
                && self.fresh_document
                && self.source_before_script_matched
                && self.script_submission_returned
                && self.script_submit_ok
                && self.dialog_kind_alert
                && self.dialog_message_matched
                && self.dialog_uri_matched
                && self.dialog_sender_matched
                && self.dialog_live_after_getters
                && self.non_accept_action_returned
                && self.completion_identity_matched
                && self.completion_hresult_succeeded
                && self.bounded_result_valid
                && self.fixed_result_matched
                && self.completion_before_release
                && self.completion_within_deadline
                && self.completion_disposition_consumed
                && self.completion_obligation_drained
                && self.completion_handlers_drained
                && self.completed_remove_succeeded
                && self.completed_same_emitter
        }
    }
    #[derive(Clone, Copy)]
    pub enum NativeDialogStage {
        Submitted,
        DocumentReady,
        ScriptSubmitted,
        Refusal,
        Completion,
        Final,
    }
    pub struct NativeDialogReceipt {
        pub(super) request: Code0WindowRequest,
        pub(super) installation: std::sync::Arc<std::sync::atomic::AtomicBool>,
        pub(super) stage: NativeDialogStage,
        pub(super) facts: NativeDialogFacts,
    }
    impl NativeDialogReceipt {
        pub fn matches(
            &self,
            request: &Code0WindowRequest,
            install: &NativeRefusalInstallReceipt,
        ) -> bool {
            &self.request == request
                && install.matches(request)
                && std::sync::Arc::ptr_eq(&self.installation, &install.installation)
        }
        pub fn stage(&self) -> NativeDialogStage {
            self.stage
        }
        pub fn facts(&self) -> NativeDialogFacts {
            self.facts
        }
    }

    /// N4 finite scalar facts. no_new_window_assignment is a reviewed source
    /// invariant; it is never a typed nullable NewWindow getter interpretation.
    #[derive(Clone, Copy, Default, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct NativePopupFacts {
        pub script_submissions: u8,
        pub popup_events: u8,
        pub handled_attempts: u8,
        pub handled_returns: u8,
        pub handled_successes: u8,
        pub handled_readbacks: u8,
        pub script_completions: u8,
        pub settings_before_stimulus: bool,
        pub initial_source_matched: bool,
        pub source_after_drop_timely: bool,
        pub script_submission_returned: bool,
        pub script_submit_ok: bool,
        pub script_return_timely: bool,
        pub popup_sender_matched: bool,
        pub request_generation_live_matched: bool,
        pub issued_before_event: bool,
        pub target_uri_getter_succeeded: bool,
        pub target_uri_matched: bool,
        pub non_user_getter_succeeded: bool,
        pub non_user: bool,
        pub handled_readback_succeeded: bool,
        pub native_handled_true: bool,
        pub live_after_calls: bool,
        pub target_after_drop_timely: bool,
        pub no_new_window_assignment: bool,
        pub first_closing_observed: bool,
        pub completion_identity_matched: bool,
        pub completion_hresult_succeeded: bool,
        pub bounded_result_valid: bool,
        pub fixed_result_matched: bool,
        pub completion_before_release: bool,
        pub completion_within_deadline: bool,
        pub completion_disposition_consumed: bool,
        pub completion_obligation_drained: bool,
        pub completion_handlers_drained: bool,
    }
    impl NativePopupFacts {
        pub fn target_positive(&self) -> bool {
            self.popup_events == 1
                && self.handled_attempts == 1
                && self.handled_returns == 1
                && self.handled_successes == 1
                && self.handled_readbacks == 1
                && self.popup_sender_matched
                && self.request_generation_live_matched
                && self.issued_before_event
                && self.target_uri_getter_succeeded
                && self.target_uri_matched
                && self.non_user_getter_succeeded
                && self.non_user
                && self.handled_readback_succeeded
                && self.native_handled_true
                && self.live_after_calls
                && self.target_after_drop_timely
                && self.no_new_window_assignment
                && self.first_closing_observed
        }
        pub fn positive(&self) -> bool {
            self.script_submissions == 1
                && self.script_completions == 1
                && self.settings_before_stimulus
                && self.initial_source_matched
                && self.source_after_drop_timely
                && self.script_submission_returned
                && self.script_submit_ok
                && self.script_return_timely
                && self.target_positive()
                && self.completion_identity_matched
                && self.completion_hresult_succeeded
                && self.bounded_result_valid
                && self.fixed_result_matched
                && self.completion_before_release
                && self.completion_within_deadline
                && self.completion_disposition_consumed
                && self.completion_obligation_drained
                && self.completion_handlers_drained
        }
    }
    #[derive(Clone, Copy)]
    pub enum NativePopupStage {
        Submitted,
        Refusal,
        Completion,
        Final,
    }
    pub struct NativePopupReceipt {
        pub(super) request: Code0WindowRequest,
        pub(super) installation: std::sync::Arc<std::sync::atomic::AtomicBool>,
        pub(super) stage: NativePopupStage,
        pub(super) facts: NativePopupFacts,
    }
    impl NativePopupReceipt {
        pub fn matches(
            &self,
            request: &Code0WindowRequest,
            install: &NativeRefusalInstallReceipt,
        ) -> bool {
            &self.request == request
                && install.matches(request)
                && std::sync::Arc::ptr_eq(&self.installation, &install.installation)
        }
        pub fn stage(&self) -> NativePopupStage {
            self.stage
        }
        pub fn facts(&self) -> NativePopupFacts {
            self.facts
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum PolicyFixtureCase {
        InstallAckBeforeFirstLoad,
        InstallPrematureLoadDenied,
        InstallSettingInjectedFailure,
        InstallAddInjectedFailure,
        InstallFilterInjectedFailure,
        InstallScriptInjectedFailure,
        InstallCdpInjectedFailure,
        InstallScriptStaleDisposition,
        InstallCdpStaleDisposition,
        InstallRealmsNoPassthrough,
    }

    impl PolicyFixtureCase {
        pub fn name(self) -> &'static str {
            match self {
                Self::InstallAckBeforeFirstLoad => "install-ack-before-first-load",
                Self::InstallPrematureLoadDenied => "install-premature-load-denied",
                Self::InstallSettingInjectedFailure => "install-setting-injected-failure",
                Self::InstallAddInjectedFailure => "install-add-injected-failure",
                Self::InstallFilterInjectedFailure => "install-filter-injected-failure",
                Self::InstallScriptInjectedFailure => "install-script-injected-failure",
                Self::InstallCdpInjectedFailure => "install-cdp-injected-failure",
                Self::InstallScriptStaleDisposition => "install-script-stale-disposition",
                Self::InstallCdpStaleDisposition => "install-cdp-stale-disposition",
                Self::InstallRealmsNoPassthrough => "install-realms-no-passthrough",
            }
        }
        pub fn async_application(self) -> bool {
            matches!(
                self,
                Self::InstallScriptInjectedFailure | Self::InstallCdpInjectedFailure
            )
        }
        pub fn stale_disposition(self) -> bool {
            matches!(
                self,
                Self::InstallScriptStaleDisposition | Self::InstallCdpStaleDisposition
            )
        }
        pub fn observes_application(self) -> bool {
            self.async_application()
                || self.stale_disposition()
                || self == Self::InstallRealmsNoPassthrough
        }
        pub fn fault_target(self) -> Option<PolicyFaultTarget> {
            match self {
                Self::InstallAckBeforeFirstLoad
                | Self::InstallScriptStaleDisposition
                | Self::InstallCdpStaleDisposition
                | Self::InstallRealmsNoPassthrough => None,
                Self::InstallPrematureLoadDenied => Some(PolicyFaultTarget::PrematureLoad),
                Self::InstallSettingInjectedFailure => Some(PolicyFaultTarget::HostObjectsSetting),
                Self::InstallAddInjectedFailure => Some(PolicyFaultTarget::ResourceAdd),
                Self::InstallFilterInjectedFailure => Some(PolicyFaultTarget::AllSourceFilter),
                Self::InstallScriptInjectedFailure => Some(PolicyFaultTarget::ScriptApplication),
                Self::InstallCdpInjectedFailure => Some(PolicyFaultTarget::CdpApplication),
            }
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum PolicyFaultTarget {
        PrematureLoad,
        HostObjectsSetting,
        ResourceAdd,
        AllSourceFilter,
        ScriptApplication,
        CdpApplication,
    }

    impl PolicyFaultTarget {
        pub fn name(self) -> &'static str {
            match self {
                Self::PrematureLoad => "premature-load",
                Self::HostObjectsSetting => "host-objects-setting",
                Self::ResourceAdd => "resource-add",
                Self::AllSourceFilter => "all-source-filter",
                Self::ScriptApplication => "script-application",
                Self::CdpApplication => "cdp-application",
            }
        }
    }

    /// Sealed observation-only snapshot: no Default, constructor, native refs,
    /// tokens, IDs, paths or raw results. No consumer can grant install authority.
    pub struct PolicyFaultReceipt {
        pub(super) request: Code0WindowRequest,
        pub(super) target: PolicyFaultTarget,
        pub(super) counts: [u32; 11],
        pub(super) application: [bool; 3],
    }

    impl PolicyFaultReceipt {
        pub fn matches(&self, request: &Code0WindowRequest) -> bool {
            &self.request == request
        }
        pub fn target(&self) -> PolicyFaultTarget {
            self.target
        }
        /// target, injection, premature denial, fixed Navigate, script submit,
        /// CDP submit, resource add, filter add, resource remove, filter remove,
        /// script remove. Every counter saturates at u32::MAX.
        pub fn counts(&self) -> [u32; 11] {
            self.counts
        }
        /// Actual ID inventoried, script ACK applied, CDP ACK applied.
        pub fn application(&self) -> [bool; 3] {
            self.application
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum StaleDispositionStage {
        Held,
        Drained,
        Final,
    }

    pub struct StaleDispositionReceipt {
        pub(super) request: Code0WindowRequest,
        pub(super) generation: u64,
        pub(super) stage: StaleDispositionStage,
        pub(super) script: bool,
        pub(super) counts: [u32; 11],
        pub(super) application: [bool; 3],
        pub(super) order: [u32; 4],
        pub(super) queued_script_id: bool,
        pub(super) policy_script_ids: u32,
    }
    impl StaleDispositionReceipt {
        pub fn matches(&self, request: &Code0WindowRequest) -> bool {
            &self.request == request && self.generation == 1
        }
        pub fn stage(&self) -> StaleDispositionStage {
            self.stage
        }
        pub fn script(&self) -> bool {
            self.script
        }
        pub fn counts(&self) -> [u32; 11] {
            self.counts
        }
        pub fn application(&self) -> [bool; 3] {
            self.application
        }
        pub fn order(&self) -> [u32; 4] {
            self.order
        }
        pub fn queued_script_id(&self) -> bool {
            self.queued_script_id
        }
        pub fn policy_script_ids(&self) -> u32 {
            self.policy_script_ids
        }
    }

    pub struct ContainmentReceipt {
        pub(super) request: Code0WindowRequest,
        pub(super) generation: u64,
        pub(super) final_snapshot: bool,
        pub(super) counts: [u32; 11],
        pub(super) application: [bool; 3],
        pub(super) deliveries: [u32; 7],
        pub(super) responses: [u32; 7],
        pub(super) total_delivered: u32,
        pub(super) renderer: [bool; 5],
        pub(super) document: [bool; 3],
    }
    impl ContainmentReceipt {
        pub fn matches(&self, request: &Code0WindowRequest) -> bool {
            &self.request == request && self.generation == 1
        }
        pub fn final_snapshot(&self) -> bool {
            self.final_snapshot
        }
        pub fn counts(&self) -> [u32; 11] {
            self.counts
        }
        pub fn application(&self) -> [bool; 3] {
            self.application
        }
        pub fn deliveries(&self) -> [u32; 7] {
            self.deliveries
        }
        pub fn responses(&self) -> [u32; 7] {
            self.responses
        }
        pub fn total_delivered(&self) -> u32 {
            self.total_delivered
        }
        pub fn renderer(&self) -> [bool; 5] {
            self.renderer
        }
        pub fn document(&self) -> [bool; 3] {
            self.document
        }
    }

    /// Opaque partial-debug receipts are produced only by the native owner at
    /// actual boundaries. No public constructor or callback-injection API.
    pub enum PolicyObservation {
        StaleDisposition(StaleDispositionReceipt),
        Containment(ContainmentReceipt),
        FaultTarget(PolicyFaultReceipt),
        FaultApplied(PolicyFaultReceipt),
        FaultFinal(PolicyFaultReceipt),
        BarrierReady(InstallBarrierReceipt),
        FirstControlledLoad(ControlledLoadReceipt),
        ControlledDocumentReady(ControlledDocumentReceipt),
    }

    pub struct InstallBarrierReceipt {
        pub(super) request: Code0WindowRequest,
        pub(super) synchronous_install_succeeded: bool,
        pub(super) script_acknowledged: bool,
        pub(super) cdp_acknowledged: bool,
    }

    impl InstallBarrierReceipt {
        pub fn matches(&self, request: &Code0WindowRequest) -> bool {
            &self.request == request && self.synchronous_install_succeeded
        }
        pub fn script_acknowledged(&self) -> bool {
            self.script_acknowledged
        }
        pub fn cdp_acknowledged(&self) -> bool {
            self.cdp_acknowledged
        }
    }

    pub struct ControlledLoadReceipt {
        pub(super) request: Code0WindowRequest,
    }

    impl ControlledLoadReceipt {
        pub fn matches(&self, request: &Code0WindowRequest) -> bool {
            &self.request == request
        }
    }

    pub struct ControlledDocumentReceipt {
        pub(super) request: Code0WindowRequest,
        pub(super) source_matched: bool,
        pub(super) synthetic_response_applied: bool,
        pub(super) canonical_ready_control: bool,
    }

    impl ControlledDocumentReceipt {
        pub fn matches(&self, request: &Code0WindowRequest) -> bool {
            &self.request == request
        }
        pub fn source_matched(&self) -> bool {
            self.source_matched
        }
        pub fn synthetic_response_applied(&self) -> bool {
            self.synthetic_response_applied
        }
        pub fn canonical_ready_control(&self) -> bool {
            self.canonical_ready_control
        }
    }

    pub enum NativeSmokeObservation {
        /// Actual bare Window, synchronously immediately after build, before any
        /// correction/getter or asynchronous environment/controller creation.
        WindowConstructed {
            window: Box<tauri::Window>,
        },
        /// Actual environment/controller/core completion and environment getters.
        EngineReady {
            window_label: String,
            user_data_folder: PathBuf,
            runtime_version: String,
        },
        /// Actual bounded renderer object JSON; not production bridge evidence.
        RendererMessage {
            payload: Vec<u8>,
        },
        /// Actual registered event, validated against the owned environment,
        /// armed core PID and normal exit kind. Contains no native references.
        BrowserProcessExited,
        /// Actual registered N1 top callback; opaque source-bound primitive audit.
        TopRefusal(NativeTopRefusalReceipt),
        /// First actual N1 revocation, before callback-local Drops or receipt delivery.
        TopClosing {
            at: std::time::Instant,
        },
        /// First actual N2 closing transition, timestamp only; no authority.
        ChildClosing {
            at: std::time::Instant,
        },
        ChildRefusal(NativeChildRefusalReceipt),
        ChildCompletion(NativeChildCompletionReceipt),
        ChildFinal(NativeChildFinalReceipt),
        PopupClosing {
            at: std::time::Instant,
        },
        Popup(NativePopupReceipt),
        DialogClosing {
            at: std::time::Instant,
        },
        Dialog(NativeDialogReceipt),
        /// Partial debug installation only; never a production policy event.
        Policy(PolicyObservation),
    }

    pub type Observer = Rc<dyn Fn(NativeSmokeObservation)>;

    /// Opt-in observations only. No diagnostic value participates in acceptance,
    /// native ownership, retries, authority or the fixed-schema smoke summary.
    #[derive(Clone, Copy)]
    #[repr(u32)]
    pub enum DiagnosticPhase {
        FreshSummary = 1,
        Sta,
        AppBuild,
        RunReturn,
        EventDispatch,
        UiAffinity,
        Lease,
        Create,
        WindowBuild,
        ConstructionReceipt,
        ConstructionQueries,
        ConstructionLock,
        WindowHwnd,
        EnvironmentFactory,
        EnvironmentCallback,
        ControllerFactory,
        ControllerCallback,
        Configure,
        ControllerVisibility,
        CoreGetter,
        BoundsGetter,
        BoundsSet,
        VersionGetter,
        ProfileCast,
        ProfileGetter,
        MessageAdd,
        Inline,
        MessageCallback,
        MessageGetter,
        Poll,
        MessageRemove,
        Stop,
        ControllerClose,
        WindowDestroy,
        WindowEventDispatch,
        ReleasedDispatch,
        SummaryLock,
        LiveWindowQueries,
        CloseRequest,
        ExitRequest,
        WakeJoin,
        TempClose,
        FinalSummary,
        WatchdogJoin,
        ExitCast,
        ExitAdd,
        BrowserPid,
        ExitCallback,
        ExitIdentity,
        ExitKindGetter,
        ExitPidGetter,
        ExitRemove,
        ControlDrop,
        EnvironmentDrop,
    }

    #[derive(Clone, Copy)]
    #[repr(u32)]
    pub enum DiagnosticMilestone {
        Sta = 0,
        NoOverrides,
        AppBuilt,
        Ready,
        Lease,
        WindowBuilt,
        ConstructionSafe,
        EnvironmentCallback,
        EnvironmentSucceeded,
        ControllerCallback,
        ControllerSucceeded,
        GettersSucceeded,
        MessageRegistered,
        EngineReady,
        InlineAccepted,
        RendererCallback,
        RendererMessage,
        CloseRequested,
        WindowDestroyed,
        Released,
        RunReturned,
        WakeJoined,
        TempClosed,
        ProfileMatched,
        ExitRegistered,
        ExitArmed,
        ExitObserved,
        ControlsDisposed,
        ExitRemoved,
        EnvironmentDropped,
    }

    #[repr(usize)]
    enum CloseTiming {
        BudgetBegin,
        CloseReturned,
        StopReturned,
        ControllerCloseReturned,
        ControlsDisposed,
        ExitObserved,
        Destroyed,
        ReleasedProduced,
        ReleasedConsumed,
        RunReturned,
    }

    /// Owned numeric watermarks only; no native or summary references.
    pub struct DiagnosticCloseSnapshot {
        kind: u8,
        terminal: bool,
        close_elapsed_ms1: u64,
        sample_ms1: u64,
        phase: u32,
        boundary: u32,
        depth: u32,
        heartbeat: u64,
        milestones: u64,
        inventory: u64,
        first_failure: u64,
        times: [u64; 10],
    }

    impl DiagnosticCloseSnapshot {
        pub fn emit(self) {
            use std::io::Write as _;
            // Fixed literals/numbers only, under1024bytes at maximum widths.
            let _ = writeln!(
                std::io::stderr(),
                "Code0NativeCloseTiming schema=1 kind={} terminal={} closeElapsedMs1={} sampleMs1={} phase={} boundary={} depth={} heartbeat={} milestones={:016x} inventory={:016x} firstFailure={:016x} budgetBeginMs1={} closeReturnedMs1={} stopReturnedMs1={} controllerCloseReturnedMs1={} controlsDisposedMs1={} exitObservedMs1={} destroyedMs1={} releasedProducedMs1={} releasedConsumedMs1={} runReturnedMs1={}",
                self.kind,
                u8::from(self.terminal),
                self.close_elapsed_ms1,
                self.sample_ms1,
                self.phase,
                self.boundary,
                self.depth,
                self.heartbeat,
                self.milestones,
                self.inventory,
                self.first_failure,
                self.times[0],
                self.times[1],
                self.times[2],
                self.times[3],
                self.times[4],
                self.times[5],
                self.times[6],
                self.times[7],
                self.times[8],
                self.times[9]
            );
        }
    }

    // A single word is published only at an actual cleanup_blocked boundary.
    // No raw metadata or dynamic string is retained. See the frozen encoding table.
    #[cfg(windows)]
    #[derive(Clone, Copy)]
    #[repr(u8)]
    pub(super) enum DiagnosticUncertaintySite {
        WindowBuild = 1,
        PollIdentity = 2,
        MailboxUnknown = 3,
        AfterControlDrop = 4,
        ResourceResponse = 5,
        MessageRemove = 6,
        Stop = 7,
        ControllerClose = 8,
        ExitRemove = 9,
        WindowDestroy = 10,
        ResourceRemove = 11,
        ChooserRemove = 12,
        FilterRemove = 13,
        ScriptRemove = 14,
    }

    #[cfg(windows)]
    #[derive(Clone, Copy, Default)]
    #[repr(u8)]
    pub(super) enum DiagnosticUncertaintyStage {
        #[default]
        None = 0,
        Args = 1,
        Policy = 2,
        Sender = 3,
        Request = 4,
        Uri = 5,
        Method = 6,
        Context = 7,
        Binding = 8,
        Body = 9,
        Stream = 10,
        Seek = 11,
        CreateResponse = 12,
        SetResponse = 13,
        Inventory = 14,
        PostResponse = 15,
    }

    #[cfg(windows)]
    #[derive(Clone, Copy, Default)]
    #[repr(u8)]
    pub(super) enum DiagnosticUncertaintyReason {
        #[default]
        Condition = 0,
        NativeError = 1,
        Missing = 2,
        SenderMismatch = 3,
        BindingMismatch = 4,
        MethodMismatch = 5,
        ContextMismatch = 6,
        InvalidString = 7,
        BodyBound = 8,
        NullStream = 9,
        RequestLimit = 10,
        Duplicate = 11,
        Revoked = 12,
        Unknown = 13,
        Unmatched = 14,
        Affinity = 15,
    }

    #[cfg(windows)]
    #[derive(Clone, Copy)]
    pub(super) struct DiagnosticUncertainty {
        pub(super) site: DiagnosticUncertaintySite,
        pub(super) stage: DiagnosticUncertaintyStage,
        pub(super) reason: DiagnosticUncertaintyReason,
        pub(super) lane: u8,
        pub(super) method: u8,
        pub(super) context: u8,
        pub(super) downgraded: bool,
        pub(super) set_response: u8,
        pub(super) metadata: u8,
        pub(super) hresult: Option<i32>,
        pub(super) context_scalar: Option<i32>,
    }

    #[cfg(windows)]
    impl DiagnosticUncertainty {
        pub(super) fn cleanup(
            site: DiagnosticUncertaintySite,
            reason: DiagnosticUncertaintyReason,
            hresult: Option<i32>,
        ) -> Self {
            Self {
                site,
                stage: DiagnosticUncertaintyStage::None,
                reason,
                lane: 0,
                method: 0,
                context: 0,
                downgraded: false,
                set_response: 0,
                metadata: 0,
                hresult,
                context_scalar: None,
            }
        }

        fn packed(self) -> u64 {
            // Schema2: native HRESULT wins; otherwise preserve the exact
            // successful getter scalar, including zero and unknown i32 values.
            let (payload, context_present) = match (self.hresult, self.context_scalar) {
                (Some(code), _) => (code, false),
                (None, Some(context)) => (context, true),
                (None, None) => (0, false),
            };
            (1 << 63)
                | u64::from(payload as u32)
                | ((self.site as u64) << 32)
                | ((self.stage as u64) << 37)
                | ((self.reason as u64) << 42)
                | (u64::from(self.lane.min(15)) << 47)
                | (u64::from(self.method.min(3)) << 51)
                | (u64::from(self.context.min(7)) << 53)
                | (u64::from(self.downgraded) << 56)
                | (u64::from(self.set_response.min(3)) << 57)
                | (u64::from(self.hresult.is_some()) << 59)
                | (u64::from(self.metadata.min(3)) << 60)
                | (u64::from(context_present) << 62)
        }
    }

    #[derive(Default)]
    pub struct Diagnostics {
        enabled: bool,
        emitted: std::sync::atomic::AtomicBool,
        temp_close_emitted: std::sync::atomic::AtomicBool,
        close_deadline_emitted: std::sync::atomic::AtomicBool,
        close_final_emitted: std::sync::atomic::AtomicBool,
        origin: Option<std::time::Instant>,
        close_times: [std::sync::atomic::AtomicU64; 10],
        lease_facts: std::sync::atomic::AtomicU32,
        phase: std::sync::atomic::AtomicU32,
        boundary: std::sync::atomic::AtomicU32,
        boundary_depth: std::sync::atomic::AtomicU32,
        heartbeat: std::sync::atomic::AtomicU64,
        milestones: std::sync::atomic::AtomicU64,
        inventory: std::sync::atomic::AtomicU64,
        first_failure: std::sync::atomic::AtomicU64,
        #[cfg(windows)]
        first_uncertainty: std::sync::atomic::AtomicU64,
    }

    /// The outermost outstanding native/getter call survives nested UI phase
    /// updates. It is not a call stack and never participates in native lifetime.
    pub struct DiagnosticBoundary {
        diagnostics: std::sync::Arc<Diagnostics>,
    }
    impl Drop for DiagnosticBoundary {
        fn drop(&mut self) {
            use std::sync::atomic::Ordering::Relaxed;
            if self.diagnostics.enabled
                && self.diagnostics.boundary_depth.fetch_sub(1, Relaxed) == 1
            {
                self.diagnostics.boundary.store(0, Relaxed);
            }
        }
    }

    impl Diagnostics {
        pub fn boundary(self: &std::sync::Arc<Self>, phase: DiagnosticPhase) -> DiagnosticBoundary {
            use std::sync::atomic::Ordering::Relaxed;
            if self.enabled && self.boundary_depth.fetch_add(1, Relaxed) == 0 {
                self.boundary.store((phase as u32) * 2, Relaxed);
            }
            DiagnosticBoundary {
                diagnostics: std::sync::Arc::clone(self),
            }
        }

        pub fn from_env() -> Self {
            static ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
            let enabled = std::env::var_os("AI_METER_CODE0_NATIVE_DIAGNOSTICS").as_deref()
                == Some(std::ffi::OsStr::new("1"));
            Self {
                enabled,
                origin: enabled.then(|| *ORIGIN.get_or_init(std::time::Instant::now)),
                ..Self::default()
            }
        }

        fn record_close_time(&self, timing: CloseTiming, instant: std::time::Instant) {
            use std::sync::atomic::Ordering::Relaxed;
            if let Some(origin) = self.origin {
                let value = instant
                    .saturating_duration_since(origin)
                    .as_millis()
                    .saturating_add(1)
                    .min(u128::from(u64::MAX)) as u64;
                let _ =
                    self.close_times[timing as usize].compare_exchange(0, value, Relaxed, Relaxed);
            }
        }

        pub fn close_budget_started(&self, instant: std::time::Instant) {
            if self.enabled {
                self.record_close_time(CloseTiming::BudgetBegin, instant);
            }
        }

        fn close_snapshot(
            &self,
            kind: u8,
            terminal: bool,
            close_elapsed_ms1: u64,
        ) -> DiagnosticCloseSnapshot {
            use std::sync::atomic::Ordering::Relaxed;
            DiagnosticCloseSnapshot {
                kind,
                terminal,
                close_elapsed_ms1,
                sample_ms1: self.origin.map_or(0, |origin| {
                    origin
                        .elapsed()
                        .as_millis()
                        .saturating_add(1)
                        .min(u128::from(u64::MAX)) as u64
                }),
                phase: self.phase.load(Relaxed),
                boundary: self.boundary.load(Relaxed),
                depth: self.boundary_depth.load(Relaxed),
                heartbeat: self.heartbeat.load(Relaxed),
                milestones: self.milestones.load(Relaxed),
                inventory: self.inventory.load(Relaxed),
                first_failure: self.first_failure.load(Relaxed),
                times: std::array::from_fn(|n| self.close_times[n].load(Relaxed)),
            }
        }

        pub fn close_deadline_snapshot_once(
            &self,
            closing: std::time::Instant,
        ) -> Option<DiagnosticCloseSnapshot> {
            use std::sync::atomic::Ordering::Relaxed;
            if !self.enabled || self.close_deadline_emitted.swap(true, Relaxed) {
                return None;
            }
            let elapsed = closing
                .elapsed()
                .as_millis()
                .saturating_add(1)
                .min(u128::from(u64::MAX)) as u64;
            Some(self.close_snapshot(1, false, elapsed))
        }

        pub fn phase(&self, phase: DiagnosticPhase, returned: bool) {
            if self.enabled {
                self.phase.store(
                    (phase as u32) * 2 + u32::from(returned),
                    std::sync::atomic::Ordering::Relaxed,
                );
                let timing = match (phase, returned) {
                    (DiagnosticPhase::CloseRequest, true) => Some(CloseTiming::CloseReturned),
                    (DiagnosticPhase::Stop, true) => Some(CloseTiming::StopReturned),
                    (DiagnosticPhase::ControllerClose, true) => {
                        Some(CloseTiming::ControllerCloseReturned)
                    }
                    (DiagnosticPhase::ReleasedDispatch, false) => {
                        Some(CloseTiming::ReleasedProduced)
                    }
                    _ => None,
                };
                if let Some(timing) = timing {
                    self.record_close_time(timing, std::time::Instant::now());
                }
            }
        }

        pub fn ui_heartbeat(&self) {
            if self.enabled {
                self.heartbeat
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }

        pub fn observed(&self, milestone: DiagnosticMilestone) {
            if self.enabled {
                self.milestones.fetch_or(
                    1 << (milestone as u32),
                    std::sync::atomic::Ordering::Relaxed,
                );
                let timing = match milestone {
                    DiagnosticMilestone::ControlsDisposed => Some(CloseTiming::ControlsDisposed),
                    DiagnosticMilestone::ExitObserved => Some(CloseTiming::ExitObserved),
                    DiagnosticMilestone::WindowDestroyed => Some(CloseTiming::Destroyed),
                    DiagnosticMilestone::Released => Some(CloseTiming::ReleasedConsumed),
                    DiagnosticMilestone::RunReturned => Some(CloseTiming::RunReturned),
                    _ => None,
                };
                if let Some(timing) = timing {
                    self.record_close_time(timing, std::time::Instant::now());
                }
            }
        }

        pub fn failure(&self, phase: DiagnosticPhase, hresult: i32) {
            if self.enabled {
                let code = ((phase as u64) << 32) | u64::from(hresult as u32);
                let _ = self.first_failure.compare_exchange(
                    0,
                    code,
                    std::sync::atomic::Ordering::Relaxed,
                    std::sync::atomic::Ordering::Relaxed,
                );
            }
        }

        #[cfg(windows)]
        pub(super) fn uncertainty(&self, detail: DiagnosticUncertainty) {
            if self.enabled {
                // One coherent first-latch record. Later errors cannot replace it.
                let _ = self.first_uncertainty.compare_exchange(
                    0,
                    detail.packed(),
                    std::sync::atomic::Ordering::Relaxed,
                    std::sync::atomic::Ordering::Relaxed,
                );
            }
        }

        pub fn publish_inventory(
            &self,
            pending: usize,
            active: usize,
            handlers: usize,
            flags: u32,
        ) {
            if self.enabled {
                let packed = (pending.min(255) as u64)
                    | ((active.min(255) as u64) << 8)
                    | ((handlers.min(255) as u64) << 16)
                    | (u64::from(flags) << 24);
                self.inventory
                    .store(packed, std::sync::atomic::Ordering::Relaxed);
            }
        }

        /// Monotonic facts from the existing release invocation, never a gate.
        /// Bits: reached/evaluated/object-free/taken/release-returned/local-drop/
        /// slot-empty. Local drop does not certify OS handle closure.
        #[cfg(windows)]
        pub(super) fn lease_fact(&self, facts: u32) {
            if self.enabled {
                self.lease_facts
                    .fetch_or(facts, std::sync::atomic::Ordering::Relaxed);
            }
        }

        /// One additional bounded line, borrowing the actual one-shot close
        /// result. Never format its path-bearing error or infer an OS code.
        pub fn emit_temp_close_once(&self, result: &std::io::Result<()>) {
            use std::io::{ErrorKind, Write as _};
            use std::sync::atomic::Ordering::Relaxed;
            if self.enabled && !self.temp_close_emitted.swap(true, Relaxed) {
                let (kind, raw) = result.as_ref().err().map_or((0u8, None), |error| {
                    let kind = match error.kind() {
                        ErrorKind::NotFound => 1,
                        ErrorKind::PermissionDenied => 2,
                        ErrorKind::AlreadyExists => 3,
                        ErrorKind::DirectoryNotEmpty => 4,
                        ErrorKind::InvalidInput => 5,
                        ErrorKind::Interrupted => 6,
                        ErrorKind::Unsupported => 7,
                        ErrorKind::Other => 8,
                        _ => 255,
                    };
                    (kind, error.raw_os_error())
                });
                // Under256bytes even for u8::MAX, i32::MIN and u32::MAX.
                // tempfile3.27 wraps OS errors: rawPresent0 is unavailable,
                // not native error0. The unchanged Result controls acceptance.
                let _ = writeln!(
                    std::io::stderr(),
                    "Code0NativeTempClose schema=1 ok={} kind={} rawPresent={} raw={} leaseFacts={:08x}",
                    u8::from(result.is_ok()),
                    kind,
                    u8::from(raw.is_some()),
                    raw.unwrap_or(0),
                    self.lease_facts.load(Relaxed)
                );
            }
        }

        /// Original one-shot diagnostic plus a separate final timing snapshot.
        /// Terminal1 is watchdog,0 normal.
        /// Snapshot reads touch only atomics, not UI/native/summary/file locks.
        /// Values are diagnostic watermarks, never a native stack or acceptance.
        pub fn emit_once(&self, watchdog: bool) {
            use std::io::Write as _;
            use std::sync::atomic::Ordering::Relaxed;
            if self.enabled && !self.emitted.swap(true, Relaxed) {
                // Fixed literals plus eight bounded integers; under512bytes even
                // at their maximum widths. Ignore diagnostic stderr I/O errors.
                let _ = writeln!(
                    std::io::stderr(),
                    "Code0NativeDiagnostics schema=1 terminal={} phase={} boundary={} depth={} heartbeat={} milestones={:016x} inventory={:016x} firstFailure={:016x}",
                    u8::from(watchdog),
                    self.phase.load(Relaxed),
                    self.boundary.load(Relaxed),
                    self.boundary_depth.load(Relaxed),
                    self.heartbeat.load(Relaxed),
                    self.milestones.load(Relaxed),
                    self.inventory.load(Relaxed),
                    self.first_failure.load(Relaxed)
                );
                if !self.close_final_emitted.swap(true, Relaxed) {
                    self.close_snapshot(2, watchdog, 0).emit();
                }
                #[cfg(windows)]
                let uncertainty = self.first_uncertainty.load(Relaxed);
                #[cfg(windows)]
                if uncertainty != 0 {
                    // Fixed56-byte line, only the one coherent atomic snapshot.
                    let _ = writeln!(
                        std::io::stderr(),
                        "Code0NativeUncertainty schema=2 packed={uncertainty:016x}"
                    );
                }
            }
        }
    }

    impl Code0NativeDriver {
        pub fn observe_fixture_diagnostics(&mut self, diagnostics: std::sync::Arc<Diagnostics>) {
            #[cfg(windows)]
            {
                self.owner.borrow_mut().diagnostics = Some(diagnostics);
            }
            #[cfg(not(windows))]
            let _ = diagnostics;
        }

        pub fn observe_fixture(&mut self, observer: Observer) {
            self.fixture_observer = Some(observer);
        }

        /// Explicit finite S2-1 registration action on the real unloaded owner.
        /// This does not authorize page loading or production PoliciesInstalled.
        pub fn install_fixture_native_refusal_handlers(
            &mut self,
            request: &Code0WindowRequest,
        ) -> Result<(), Code0SessionFailure> {
            if !self
                .lease
                .as_ref()
                .is_some_and(super::Code0ProfileLease::is_held)
            {
                return Err(Code0SessionFailure::ProfileBusy);
            }
            #[cfg(windows)]
            {
                super::native::install_native_refusal_handlers(&self.owner, request)
            }
            #[cfg(not(windows))]
            {
                let _ = request;
                Err(Code0SessionFailure::PolicyRejected)
            }
        }

        /// Read actual same-instance registration inventory. Never installs or loads.
        pub fn require_fixture_native_refusal_install(
            &mut self,
            request: &Code0WindowRequest,
        ) -> Result<NativeRefusalInstallReceipt, Code0SessionFailure> {
            if !self
                .lease
                .as_ref()
                .is_some_and(super::Code0ProfileLease::is_held)
            {
                return Err(Code0SessionFailure::ProfileBusy);
            }
            #[cfg(windows)]
            {
                super::native::require_native_refusal_install(&self.owner, request)
            }
            #[cfg(not(windows))]
            {
                let _ = request;
                Err(Code0SessionFailure::PolicyRejected)
            }
        }

        /// One debug-only N1 Navigate, derived from the owned fixed inert leaf.
        /// The production navigate and PoliciesInstalled paths remain denied.
        pub fn navigate_fixture_top_owned_file(
            &mut self,
            request: &Code0WindowRequest,
            deadline: NativeRefusalDeadline,
        ) -> Result<NativeTopSubmissionReceipt, Code0SessionFailure> {
            if !self
                .lease
                .as_ref()
                .is_some_and(super::Code0ProfileLease::is_held)
            {
                return Err(Code0SessionFailure::ProfileBusy);
            }
            // The root is derived from this driver's leased dedicated profile,
            // never from page/App URL or path input to the finite action.
            let owned_root = self
                .profile_root
                .parent()
                .ok_or(Code0SessionFailure::PolicyRejected)?;
            #[cfg(windows)]
            {
                super::native::navigate_top_owned_file(&self.owner, request, owned_root, deadline)
            }
            #[cfg(not(windows))]
            {
                let _ = (request, owned_root, deadline);
                Err(Code0SessionFailure::PolicyRejected)
            }
        }

        /// N3 finite owned-file document then fixed alert. No caller URL or script.
        pub fn navigate_fixture_dialog_owned_empty_html(
            &mut self,
            request: &Code0WindowRequest,
            deadline: NativeDialogDeadline,
        ) -> Result<(), Code0SessionFailure> {
            if !self
                .lease
                .as_ref()
                .is_some_and(super::Code0ProfileLease::is_held)
            {
                return Err(Code0SessionFailure::ProfileBusy);
            }
            let owned_root = self
                .profile_root
                .parent()
                .ok_or(Code0SessionFailure::PolicyRejected)?;
            #[cfg(windows)]
            {
                super::native::s2_dialog_owned_empty_html(
                    &self.owner,
                    request,
                    owned_root,
                    deadline,
                )
            }
            #[cfg(not(windows))]
            {
                let _ = (request, owned_root, deadline);
                Err(Code0SessionFailure::PolicyRejected)
            }
        }

        /// One internally fixed N2 operation on the existing initial blank DOM.
        pub fn submit_fixture_child_initial_data(
            &mut self,
            request: &Code0WindowRequest,
            deadline: NativeRefusalDeadline,
        ) -> Result<NativeChildSubmissionReceipt, Code0SessionFailure> {
            if !self
                .lease
                .as_ref()
                .is_some_and(super::Code0ProfileLease::is_held)
            {
                return Err(Code0SessionFailure::ProfileBusy);
            }
            #[cfg(windows)]
            {
                super::native::submit_child_initial_data(&self.owner, request, deadline)
            }
            #[cfg(not(windows))]
            {
                let _ = (request, deadline);
                Err(Code0SessionFailure::PolicyRejected)
            }
        }

        /// One fixed non-user N4 popup operation on the actual initial blank.
        pub fn submit_fixture_popup_initial_blank(
            &mut self,
            request: &Code0WindowRequest,
            deadline: NativeDialogDeadline,
        ) -> Result<(), Code0SessionFailure> {
            if !self
                .lease
                .as_ref()
                .is_some_and(super::Code0ProfileLease::is_held)
            {
                return Err(Code0SessionFailure::ProfileBusy);
            }
            #[cfg(windows)]
            {
                super::native::submit_popup_initial_blank(&self.owner, request, deadline)
            }
            #[cfg(not(windows))]
            {
                let _ = (request, deadline);
                Err(Code0SessionFailure::PolicyRejected)
            }
        }

        /// Start the finite partial debug barrier on this real native instance.
        /// A submission Ok would still require actual asynchronous receipts.
        pub fn begin_fixture_policy_install(
            &mut self,
            request: &Code0WindowRequest,
            case: PolicyFixtureCase,
        ) -> Result<(), Code0SessionFailure> {
            if !self
                .lease
                .as_ref()
                .is_some_and(super::Code0ProfileLease::is_held)
            {
                return Err(Code0SessionFailure::ProfileBusy);
            }
            #[cfg(windows)]
            {
                super::native::begin_policy_install(&self.owner, request, case)
            }
            #[cfg(not(windows))]
            {
                let _ = (request, case);
                Err(Code0SessionFailure::PolicyRejected)
            }
        }

        /// Invoke the finite loader on this real engine before installation.
        /// Only the loader's actual PolicyRejected result can produce a receipt.
        pub fn probe_fixture_premature_policy_load(
            &mut self,
            request: &Code0WindowRequest,
        ) -> Result<(), Code0SessionFailure> {
            if !self
                .lease
                .as_ref()
                .is_some_and(super::Code0ProfileLease::is_held)
            {
                return Err(Code0SessionFailure::ProfileBusy);
            }
            #[cfg(windows)]
            {
                super::native::probe_premature_policy_load(&self.owner, request)
            }
            #[cfg(not(windows))]
            {
                let _ = request;
                Err(Code0SessionFailure::PolicyRejected)
            }
        }

        /// One finite P1 document, only after this same owner's barrier receipt.
        pub fn load_fixture_policy_document(
            &mut self,
            request: &Code0WindowRequest,
        ) -> Result<(), Code0SessionFailure> {
            if !self
                .lease
                .as_ref()
                .is_some_and(super::Code0ProfileLease::is_held)
            {
                return Err(Code0SessionFailure::ProfileBusy);
            }
            #[cfg(windows)]
            {
                super::native::load_policy_document(&self.owner, request)
            }
            #[cfg(not(windows))]
            {
                let _ = request;
                Err(Code0SessionFailure::PolicyRejected)
            }
        }

        /// Only a separately observed true no-window/no-COM/no-pending state may
        /// fast-exit a create failure. It never supplies a Destroyed observation.
        pub fn fixture_creation_is_object_free(&self, instance_id: &str) -> bool {
            #[cfg(windows)]
            {
                super::native::zero_creation(&self.owner, instance_id)
            }
            #[cfg(not(windows))]
            {
                let _ = instance_id;
                true
            }
        }

        pub fn load_fixture_inline_document(
            &mut self,
            instance_id: &str,
        ) -> Result<(), Code0SessionFailure> {
            #[cfg(windows)]
            {
                super::native::load_inline(&self.owner, instance_id)
            }
            #[cfg(not(windows))]
            {
                let _ = instance_id;
                Err(Code0SessionFailure::PolicyRejected)
            }
        }
    }
}
