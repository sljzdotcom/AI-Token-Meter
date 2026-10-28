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
        DiagnosticMilestone as DM, DiagnosticPhase as DP, Diagnostics, NativeSmokeObservation,
        Observer,
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
        }
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
                    instance.policy.is_none() && p1::mailbox_quiescent(instance)
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
                matching(&mut owner.borrow_mut(), &request)
                    .unwrap()
                    .cleanup_blocked = true;
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
        if let Some(instance) = owner.borrow_mut().instance.as_mut()
            && instance.request.instance_id == instance_id
        {
            // Revoke before any external Stop/Close/removal can reenter.
            instance.closing = true;
        }
        // Do not emit Released here: callers may still have action-local COM
        // clones on their stack. Only a later UI poll can certify the inventory.
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
                instance.window.take()
            } else {
                None
            }
        };
        if window.is_some() {
            milestone!(owner, WindowDestroyed);
        }
        drop(window);
    }

    fn cleanup_result(owner: &Rc<RefCell<Owner>>, result: windows::core::Result<()>) {
        if result.is_err() {
            let mut state = owner.borrow_mut();
            let instance = state.instance.as_mut().unwrap();
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
        {
            let mut state = owner.borrow_mut();
            let unmatched = state.unmatched;
            let Some(instance) = state.instance.as_mut() else {
                return;
            };
            if unmatched || instance.callbacks.affinity_failed.load(Ordering::SeqCst) {
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
            cleanup_result(owner, result);
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
            cleanup_result(owner, native_call!(owner, Stop, unsafe { core.Stop() }));
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
            cleanup_result(
                owner,
                native_call!(owner, ControllerClose, unsafe { controller.Close() }),
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
                        p1::registrations_removed(instance)
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
            let disposed = p1::after_control_drop(owner);
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
            cleanup_result(owner, result);
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
                cleanup_result(owner, Err(E_FAIL.into()));
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
            super::fixture::PolicyFixtureCase::InstallAckBeforeFirstLoad => {
                p1::begin(owner, request)
            }
        }
    }

    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    pub(super) fn load_policy_document(
        owner: &Rc<RefCell<Owner>>,
        request: &Code0WindowRequest,
    ) -> Result<(), Code0SessionFailure> {
        p1::load(owner, request)
    }

    /// Finite debug-only P1. No production policy authority is derived here.
    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    mod p1 {
        use super::super::fixture::{
            ControlledDocumentReceipt, ControlledLoadReceipt, InstallBarrierReceipt,
            PolicyObservation,
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
            let result = synchronous(owner, request).and_then(|()| submit_script(owner, request));
            if result.is_err() {
                failed(owner, request);
                return Err(Code0SessionFailure::PolicyRejected);
            }
            Ok(())
        }

        fn synchronous(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
        ) -> windows::core::Result<()> {
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
                            let result = resource(
                                &owner,
                                &owned,
                                generation,
                                sender.as_ref(),
                                args.as_ref(),
                            );
                            if result.is_err() {
                                uncertain_response(&owner, &owned);
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
            }
            drop(handler);
            result?;
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
            }
            filtered?;
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
            Ok(())
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
                        {
                            let mut state = owner.borrow_mut();
                            if let Some(i) = state.instance.as_mut()
                                && let Some(p) = i.policy.as_mut()
                            {
                                if let Some(id) = id {
                                    p.scripts.push((id, false));
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
                                    && !mailbox.invalid.load(Ordering::SeqCst)
                                    && !mailbox.unknown.load(Ordering::SeqCst)
                                    && !i.callbacks.affinity_failed.load(Ordering::SeqCst);
                                p.script_completed = true;
                                p.script_ack = valid;
                                if !valid {
                                    i.failed = true;
                                    i.closing = true;
                                }
                            } else {
                                if let Some(id) = id {
                                    state.unmatched_script_ids.push(id);
                                }
                                state.unmatched = true;
                            }
                        }
                        drop(pending);
                    }
                    Disposition::Cdp {
                        request,
                        generation,
                        expected,
                        first,
                        pending,
                    } => {
                        {
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
                                    && !mailbox.invalid.load(Ordering::SeqCst)
                                    && !mailbox.unknown.load(Ordering::SeqCst)
                                    && !i.callbacks.affinity_failed.load(Ordering::SeqCst);
                                p.cdp_completed = true;
                                p.cdp_ack = valid;
                                if !valid {
                                    i.failed = true;
                                    i.closing = true;
                                }
                            } else {
                                state.unmatched = true;
                            }
                        }
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
                    p.document_receipt = true;
                    Some(PolicyObservation::ControlledDocumentReady(
                        ControlledDocumentReceipt {
                            request: request.clone(),
                            source_matched: true,
                            synthetic_response_applied: p.main_response,
                            canonical_ready_control: p.ready_control,
                        },
                    ))
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
            let (core, observer) = {
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
                (Rc::clone(&p.core), i.observer.clone())
            };
            require_live(owner, request).map_err(|_| Code0SessionFailure::PolicyRejected)?;
            let result = unsafe {
                core.Navigate(w!(
                    "https://console.code0.ai/__ai_meter_fixture__/install-p1"
                ))
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
        ) -> windows::core::Result<()> {
            let core = core(owner);
            let expected: IUnknown = core.cast()?;
            let actual: IUnknown = sender.ok_or(E_FAIL)?.cast()?;
            if expected.as_raw() == actual.as_raw() {
                Ok(())
            } else {
                Err(E_FAIL.into())
            }
        }

        // A failed/over-cap response is retained uncertainty, never a successful
        // unset response or a claimed transport cancellation. Stop is attempted
        // under this callback's Active guard, then ordinary R4 close is requested.
        fn uncertain_response(owner: &Rc<RefCell<Owner>>, request: &Code0WindowRequest) {
            let core = {
                let mut state = owner.borrow_mut();
                let Some(i) = matching(&mut state, request) else {
                    state.unmatched = true;
                    return;
                };
                i.failed = true;
                i.closing = true;
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
            allowed: bool,
        ) -> windows::core::Result<(IStream, ICoreWebView2WebResourceResponse, bool)> {
            let body = if allowed { MAIN } else { REFUSAL };
            if body.len() > 65536 || u32::try_from(body.len()).is_err() {
                return Err(E_FAIL.into());
            }
            // SHCreateMemStream returns Option, not HRESULT/GetLastError.
            // This is our fixed allocation-failure mapping, not a native code.
            let stream = unsafe { SHCreateMemStream(Some(body)) }
                .ok_or_else(|| windows::core::Error::from_hresult(E_OUTOFMEMORY))?;
            if allowed && !live(owner, request) {
                drop(stream);
                return make_response(owner, request, environment, false);
            }
            unsafe { stream.Seek(0, STREAM_SEEK_SET, None) }?;
            if allowed && !live(owner, request) {
                drop(stream);
                return make_response(owner, request, environment, false);
            }
            let headers: Vec<u16> = HEADERS.encode_utf16().chain(Some(0)).collect();
            let response = unsafe {
                environment.CreateWebResourceResponse(
                    &stream,
                    if allowed { 200 } else { 403 },
                    if allowed { w!("OK") } else { w!("Forbidden") },
                    PCWSTR(headers.as_ptr()),
                )
            }?;
            if allowed && !live(owner, request) {
                drop(response);
                drop(stream);
                return make_response(owner, request, environment, false);
            }
            Ok((stream, response, allowed))
        }

        fn resource(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            generation: u64,
            sender: Option<&ICoreWebView2>,
            args: Option<&ICoreWebView2WebResourceRequestedEventArgs>,
        ) -> windows::core::Result<()> {
            let args = args.ok_or(E_FAIL)?;
            let (environment, within_bound) = {
                let mut state = owner.borrow_mut();
                let i = state.instance.as_mut().ok_or(E_FAIL)?;
                let p = i.policy.as_mut().ok_or(E_FAIL)?;
                p.delivered_requests = p.delivered_requests.saturating_add(1);
                (
                    Rc::clone(&p.environment),
                    p.delivered_requests <= MAX_REQUESTS,
                )
            };
            // Metadata/identity errors, revocation and unexpected requests still
            // receive the same fixed synthetic refusal, never successful unset.
            let metadata = (|| -> windows::core::Result<bool> {
                sender_matches(owner, sender)?;
                let native_request = unsafe { args.Request() }?;
                let uri = allocated_string(2048, |raw| unsafe { native_request.Uri(raw) })?;
                let method = allocated_string(16, |raw| unsafe { native_request.Method(raw) })?;
                let mut context = COREWEBVIEW2_WEB_RESOURCE_CONTEXT::default();
                unsafe { args.ResourceContext(&mut context) }?;
                let state = owner.borrow();
                let i = state.instance.as_ref().ok_or(E_FAIL)?;
                let p = i.policy.as_ref().ok_or(E_FAIL)?;
                Ok(&i.request == request
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
                    && context == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT)
            })();
            let allowed = matches!(metadata, Ok(true)) && live(owner, request);
            let pair = make_response(owner, request, &environment, allowed)?;
            let allowed = pair.2;
            let applied = unsafe { args.SetResponse(&pair.1) };
            // Root-reviewed Microsoft native sample/COM ownership boundary:
            // native code retains its own refs on successful put_Response.
            // Drop our aliases outside Owner borrows, under the real callback
            // Active guard. This proves neither consumption nor cancellation.
            drop(pair);
            applied?;
            {
                let mut state = owner.borrow_mut();
                let i = state.instance.as_mut().ok_or(E_FAIL)?;
                let p = i.policy.as_mut().ok_or(E_FAIL)?;
                if allowed && &i.request == request && !i.closing && !i.failed {
                    p.main_response = true;
                }
            }
            if metadata.is_err() || !within_bound || allowed && !live(owner, request) {
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
            sender_matches(owner, sender)?;
            require_live(owner, request)?;
            let args = args.ok_or(E_FAIL)?;
            let source = allocated_string(2048, |raw| unsafe { args.Source(raw) })?;
            require_live(owner, request)?;
            let payload = allocated_string(2048, |raw| unsafe { args.WebMessageAsJson(raw) })?;
            require_live(owner, request)?;
            if source != URI || payload.len() > 2048 {
                return Err(E_FAIL.into());
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
                }
                cleanup_result(owner, result);
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
                cleanup_result(owner, result);
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
                }
                cleanup_result(owner, result);
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
                }
                cleanup_result(owner, result);
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

    #[derive(Clone, Copy)]
    pub enum PolicyFixtureCase {
        InstallAckBeforeFirstLoad,
    }

    /// Opaque partial-debug receipts are produced only by the native owner at
    /// actual boundaries. No public constructor or callback-injection API.
    pub enum PolicyObservation {
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
        WindowConstructed { window: Box<tauri::Window> },
        /// Actual environment/controller/core completion and environment getters.
        EngineReady {
            window_label: String,
            user_data_folder: PathBuf,
            runtime_version: String,
        },
        /// Actual bounded renderer object JSON; not production bridge evidence.
        RendererMessage { payload: Vec<u8> },
        /// Actual registered event, validated against the owned environment,
        /// armed core PID and normal exit kind. Contains no native references.
        BrowserProcessExited,
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

    #[derive(Default)]
    pub struct Diagnostics {
        enabled: bool,
        emitted: std::sync::atomic::AtomicBool,
        temp_close_emitted: std::sync::atomic::AtomicBool,
        lease_facts: std::sync::atomic::AtomicU32,
        phase: std::sync::atomic::AtomicU32,
        boundary: std::sync::atomic::AtomicU32,
        boundary_depth: std::sync::atomic::AtomicU32,
        heartbeat: std::sync::atomic::AtomicU64,
        milestones: std::sync::atomic::AtomicU64,
        inventory: std::sync::atomic::AtomicU64,
        first_failure: std::sync::atomic::AtomicU64,
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
            Self {
                enabled: std::env::var_os("AI_METER_CODE0_NATIVE_DIAGNOSTICS").as_deref()
                    == Some(std::ffi::OsStr::new("1")),
                ..Self::default()
            }
        }

        pub fn phase(&self, phase: DiagnosticPhase, returned: bool) {
            if self.enabled {
                self.phase.store(
                    (phase as u32) * 2 + u32::from(returned),
                    std::sync::atomic::Ordering::Relaxed,
                );
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

        /// At most one fixed diagnostic line: terminal1 is watchdog,0 normal.
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
