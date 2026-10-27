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
        #[cfg(windows)]
        if !native::object_free(&self.owner, None) {
            return;
        }
        if let Some(lease) = self.lease.take() {
            lease.release();
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
        CoTaskMemPWSTR, CreateCoreWebView2ControllerCompletedHandler,
        CreateCoreWebView2EnvironmentCompletedHandler, WebMessageReceivedEventHandler,
    };
    use windows::Win32::Foundation::{E_FAIL, HWND, RECT};
    use windows::Win32::System::Com::{
        APTTYPE, APTTYPE_MAINSTA, APTTYPE_STA, APTTYPEQUALIFIER, CoGetApartmentType,
    };
    use windows::core::{Interface, PCWSTR, PWSTR};

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
        polling: bool,
        ui_thread: ThreadId,
        events: Code0NativeEventSink,
        instance: Option<Instance>,
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        pub(super) diagnostics: Option<Arc<Diagnostics>>,
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
    }

    impl Owner {
        pub(super) fn new(events: Code0NativeEventSink) -> Self {
            Self {
                unmatched: false,
                unmatched_objects: Vec::new(),
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
        owner.borrow().ui_thread == thread::current().id() && is_sta()
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
            && instance.controller.is_none()
            && instance.core.is_none()
            && instance.message_token.is_none()
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
        matching(&mut owner.borrow_mut(), &request)
            .unwrap()
            .build_started = true;
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

    fn environment_completed(
        weak: &Weak<RefCell<Owner>>,
        request: &Code0WindowRequest,
        result: windows::core::Result<()>,
        environment: Option<ICoreWebView2Environment>,
    ) {
        let Some(owner) = weak.upgrade() else {
            return;
        };
        let (start_controller, ui, callbacks, hwnd) = {
            let mut state = owner.borrow_mut();
            let ui = state.ui_thread;
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
            (
                !instance.closing,
                ui,
                Arc::clone(&instance.callbacks),
                instance.hwnd,
            )
        };
        if !start_controller {
            return;
        }
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
                message_received(&owner, &owned_request, args.as_ref());
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
        args: Option<&ICoreWebView2WebMessageReceivedEventArgs>,
    ) {
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        {
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
        let _ = (owner, request, args);
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
                | (u32::from(instance.destroy_attempted) << 15);
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
            if !instance.closing || instance.callbacks.active.load(Ordering::SeqCst) != 0 {
                return;
            }
        }
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
        let references = {
            let mut state = owner.borrow_mut();
            let instance = state.instance.as_mut().unwrap();
            if !instance.cleanup_blocked
                && instance.message_token.is_none()
                && instance.callbacks.pending.load(Ordering::SeqCst) == 0
                && instance.callbacks.active.load(Ordering::SeqCst) == 0
                && instance.callbacks.handlers.load(Ordering::SeqCst) == 0
            {
                Some((
                    instance.core.take(),
                    instance.controller.take(),
                    instance.environment.take(),
                ))
            } else {
                None
            }
        };
        drop(references);
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
            let events = Rc::clone(&state.events);
            let instance = state.instance.as_mut().unwrap();
            if instance.failed && !instance.failure_sent {
                instance.failure_sent = true;
                Some((
                    events,
                    instance.request.clone(),
                    Code0WindowEvent::Failed(if instance.cleanup_blocked {
                        Code0SessionFailure::CleanupFailed
                    } else {
                        Code0SessionFailure::PolicyRejected
                    }),
                ))
            } else if !instance.released && inventory_empty(instance) {
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

/// Release builds expose neither inline document loading nor native observations.
#[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
pub mod fixture {
    use std::path::PathBuf;
    use std::rc::Rc;

    use super::{Code0NativeDriver, Code0SessionFailure};

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
    }

    #[derive(Default)]
    pub struct Diagnostics {
        enabled: bool,
        emitted: std::sync::atomic::AtomicBool,
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
