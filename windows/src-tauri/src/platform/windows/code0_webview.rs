//! UI-local bare-Window/direct-COM Code0 owner. Official loading remains denied
//! until the separately verified production policy and bridge installation phase.
use std::path::PathBuf;
use std::rc::Rc;

use super::code0_profile_lease::{Code0LeaseError, Code0ProfileLease};
use super::code0_web_session::{
    Code0CleanupRequest, Code0SessionFailure, Code0WindowDriving, Code0WindowEvent,
    Code0WindowRequest,
};

#[cfg(all(windows, debug_assertions, feature = "code0-native-fixture"))]
mod chooser_protocol;

#[cfg(all(test, windows, debug_assertions, feature = "code0-native-fixture"))]
mod http401_protocol;

#[cfg(all(test, windows, debug_assertions, feature = "code0-native-fixture"))]
#[doc(hidden)]
pub mod permission_protocol;

#[cfg(all(windows, debug_assertions, feature = "code0-native-fixture"))]
#[doc(hidden)]
pub mod s4_capability_protocol;

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
        DiagnosticUncertaintyStage as UT, Diagnostics, N3Block as N3B, N3CompletionDiagnostic,
        N3DialogReason as N3R, N3Writer as N3W, NativeSmokeObservation, Observer,
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

    // N4 diagnostic begin.
    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    fn n4_note(
        callbacks: &Callbacks,
        cause: crate::platform::windows::code0_webview::fixture::N4Cause,
        site: u16,
        bits: u16,
    ) {
        if let Some(diagnostic) = callbacks.diagnostics.as_ref() {
            diagnostic.n4_first(cause, site, bits);
        }
    }
    // N4 diagnostic end.

    // N3 diagnostic begin: callers hold their original Owner borrow; no native work.
    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    fn n3_writer(callbacks: &Callbacks, site: N3W, bits: u16) {
        if let Some(diagnostic) = callbacks.diagnostics.as_ref() {
            diagnostic.n3_writer(site, bits);
        }
    }
    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    fn n3_block(instance: &Instance, was_blocked: bool, unmatched: bool, site: N3B, reason: u8) {
        if let Some(diagnostic) = instance
            .callbacks
            .diagnostics
            .as_ref()
            .filter(|d| d.n3_enabled())
        {
            let bits = u16::from(instance.controls_disposed)
                | (u16::from(instance.closing) << 1)
                | (u16::from(instance.destroyed) << 2)
                | (u16::from(matches!(instance.browser_exit, BrowserExitState::Observed)) << 3)
                | (u16::from(instance.exit_remove_attempted) << 4)
                | (u16::from(instance.refusal_registry.unknown()) << 5)
                | (u16::from(instance.callbacks.affinity_failed.load(Ordering::SeqCst)) << 6)
                | (u16::from(unmatched) << 7)
                | (u16::from(reason) << 8);
            diagnostic.n3_block(was_blocked, site, bits);
        }
    }
    // N3 diagnostic end.

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
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        chooser: Option<s3::State>,
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        capability: Option<s4::State>,
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

    fn callback_on_ui(ui: ThreadId, callbacks: &Callbacks, _n4_site: u16) -> bool {
        let valid = thread::current().id() == ui && is_sta();
        if !valid {
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            n4_note(
                callbacks,
                crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                _n4_site,
                0,
            );
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
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            n4_note(
                &instance.callbacks,
                crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                447,
                0,
            );
            instance.closing = true;
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            let _ = s2::mark_popup_close(instance);
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            let _ = s2::mark_download_close(instance);
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            s3::revoke(instance);
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            s4::mark_close(instance);
        }
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s2::publish_popup_close(owner);
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s2::publish_download_close(owner);
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
                        && s3::quiescent(instance)
                        && s4::quiescent(instance)
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
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            chooser: None,
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            capability: None,
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
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                n4_note(
                    &instance.callbacks,
                    crate::platform::windows::code0_webview::fixture::N4Cause::CleanupBlock,
                    635,
                    0,
                );
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
                if callback_on_ui(ui, &callbacks, 695) {
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
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            n4_note(
                &instance.callbacks,
                crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                731,
                0,
            );
            instance.browser_exit = BrowserExitState::Unknown;
            instance.failed = true;
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            n4_note(
                &instance.callbacks,
                crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                733,
                0,
            );
            instance.closing = true;
        } else {
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            if let Some(diagnostic) = state.diagnostics.as_ref() {
                diagnostic.n4_first(
                    crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                    735,
                    0,
                );
            }
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
            if callback_on_ui(ui, &callbacks, 774)
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
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                if let Some(diagnostic) = state.diagnostics.as_ref() {
                    diagnostic.n4_first(
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        926,
                        0,
                    );
                }
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
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                n4_note(
                    &instance.callbacks,
                    crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                    937,
                    0,
                );
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
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                if let Some(diagnostic) = state.diagnostics.as_ref() {
                    diagnostic.n4_first(
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        949,
                        0,
                    );
                }
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
                if callback_on_ui(ui, &callbacks, 990) {
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
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                if let Some(diagnostic) = state.diagnostics.as_ref() {
                    diagnostic.n4_first(
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        1031,
                        0,
                    );
                }
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
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                n4_note(
                    &instance.callbacks,
                    crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                    1043,
                    0,
                );
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
            if callback_on_ui(ui, &callbacks, 1127)
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
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            n4_note(
                &instance.callbacks,
                crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                1237,
                0,
            );
            instance.closing = true;
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            let _ = s2::mark_child_close(instance);
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            let _ = s2::mark_dialog_close(instance);
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            let _ = s2::mark_popup_close(instance);
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            let _ = s2::mark_download_close(instance);
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            s3::mark_close(instance);
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            s4::mark_close(instance);
        }
        drop(state);
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s2::publish_popup_close(owner);
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s2::publish_download_close(owner);
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
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                n4_note(
                    &instance.callbacks,
                    crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                    1273,
                    0,
                );
                instance.closing = true;
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                let _ = s2::mark_popup_close(instance);
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                let _ = s2::mark_download_close(instance);
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                s3::mark_close(instance);
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                s4::mark_close(instance);
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
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s2::publish_download_close(owner);
    }

    fn cleanup_result(
        owner: &Rc<RefCell<Owner>>,
        result: windows::core::Result<()>,
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))] detail: DU,
    ) {
        if result.is_err() {
            let mut state = owner.borrow_mut();
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            let n3_unmatched = state.unmatched;
            let instance = state.instance.as_mut().unwrap();
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            if let Some(diagnostic) = instance.callbacks.diagnostics.as_ref() {
                diagnostic.uncertainty(detail);
            }
            instance.failed = true;
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            n4_note(
                &instance.callbacks,
                crate::platform::windows::code0_webview::fixture::N4Cause::CleanupBlock,
                1306,
                0,
            );
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            let n3_was_blocked = instance.cleanup_blocked;
            instance.cleanup_blocked = true;
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            n3_block(
                instance,
                n3_was_blocked,
                n3_unmatched,
                N3B::NativeCleanup,
                detail.site as u8,
            );
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
            s2::diagnose_popup(instance, state.unmatched);
            s2::diagnose_dialog(instance, state.unmatched);
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
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s2::advance_download(owner);
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s3::advance(owner);
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s4::advance(owner);
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
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s2::synchronize_download_close(owner);
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s3::synchronize(owner);
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s4::synchronize(owner);
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
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                n4_note(
                    &instance.callbacks,
                    crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                    1420,
                    0,
                );
                instance.closing = true;
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                n4_note(
                    &instance.callbacks,
                    crate::platform::windows::code0_webview::fixture::N4Cause::CleanupBlock,
                    1421,
                    u16::from(unmatched)
                        | (u16::from(instance.callbacks.affinity_failed.load(Ordering::SeqCst))
                            << 1),
                );
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                let n3_was_blocked = instance.cleanup_blocked;
                instance.cleanup_blocked = true;
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                n3_block(instance, n3_was_blocked, unmatched, N3B::PollIdentity, 0);
            }
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            if instance.refusal_registry.unknown() {
                instance.failed = true;
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                n4_note(
                    &instance.callbacks,
                    crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                    1426,
                    0,
                );
                instance.closing = true;
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                n4_note(
                    &instance.callbacks,
                    crate::platform::windows::code0_webview::fixture::N4Cause::CleanupBlock,
                    1427,
                    0,
                );
                let n3_was_blocked = instance.cleanup_blocked;
                instance.cleanup_blocked = true;
                n3_block(instance, n3_was_blocked, unmatched, N3B::PollUnknown, 0);
            }
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            s3::mark_close(instance);
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            s4::mark_close(instance);
            if !instance.closing
                || {
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    {
                        !s4::ready_to_stop(instance)
                    }
                    #[cfg(not(all(debug_assertions, feature = "code0-native-fixture")))]
                    {
                        false
                    }
                }
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
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s3::remove(owner);
        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
        s4::remove(owner);
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
                native_call!(owner, Stop, {
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    s3::stop_entry(owner);
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    s4::stop_entry(owner);
                    unsafe { core.Stop() }
                }),
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
                native_call!(owner, ControllerClose, {
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    s3::stop_entry(owner);
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    s4::stop_entry(owner);
                    unsafe { controller.Close() }
                }),
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
                        p1::registrations_removed(instance)
                            && s2::registrations_removed(instance)
                            && s3::removed(instance)
                            && s4::removed(instance)
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
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    s3::take_controls(instance),
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    s4::take_controls(instance),
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
                s2::publish_download_final(owner);
                s3::publish_final(owner);
                s4::publish_final(owner);
            }
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            if let Code0WindowEvent::Failed(reason) = &event
                && let Some(diagnostic) = owner.borrow().diagnostics.as_ref()
            {
                diagnostic.n4_session_stage(reason, false);
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
            || i.capability.is_some()
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
            n4_note(
                &i.callbacks,
                crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                1751,
                u16::from(state.unmatched)
                    | (u16::from(&i.request != request) << 1)
                    | (u16::from(i.closing) << 2)
                    | (u16::from(i.failed) << 3)
                    | (u16::from(i.cleanup_blocked) << 4)
                    | (u16::from(i.released) << 5)
                    | (u16::from(i.refusal_registry.complete()) << 6),
            );
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

    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    pub(super) fn submit_download_initial_data_empty(
        owner: &Rc<RefCell<Owner>>,
        request: &Code0WindowRequest,
        deadline: super::fixture::NativeDialogDeadline,
    ) -> Result<(), Code0SessionFailure> {
        s2::submit_download_initial_data_empty(owner, request, deadline)
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
            NativeDialogFacts, NativeDialogReceipt, NativeDialogStage, NativeDownloadFacts,
            NativeDownloadReceipt, NativeDownloadStage, NativePopupFacts, NativePopupReceipt,
            NativePopupStage, NativeRefusalDeadline, download_result_units,
            legacy_completion_disposition_at, popup_result_units,
        };
        const DIALOG_LEAF: &str = "code0-s2-dialog-empty.html";
        const DIALOG_MESSAGE: &str = "code0-native-dialog-refusal-v1";
        const DIALOG_SCRIPT: &str = "(()=>{if(top!==self||location.protocol!=='file:'||!location.pathname.endsWith('/code0-s2-dialog-empty.html')||!document.documentElement||document.querySelector('iframe,frame')!==null){return 'code0-dialog-precondition-v1';}alert('code0-native-dialog-refusal-v1');return 'code0-dialog-returned-v1';})()";
        const DIALOG_RESULT: &str = "\"code0-dialog-returned-v1\"";
        const CHILD_TARGET: &str = "data:text/html,";
        const CHILD_SCRIPT: &str = "(()=>{if(top!==self||location.href!=='about:blank'||!document.documentElement||document.querySelector('iframe')!==null){throw new Error('code0-child-precondition');}const f=document.createElement('iframe');f.src='data:text/html,';document.documentElement.appendChild(f);return 'code0-child-submitted-v1';})()";
        const CHILD_RESULT: &str = "\"code0-child-submitted-v1\"";
        const DOWNLOAD_TARGET: &str = "data:application/octet-stream,";
        const DOWNLOAD_SCRIPT: &str = "(()=>{if(top!==self||location.href!=='about:blank'||!document.documentElement||document.documentElement.tagName!=='HTML'||!document.body||document.querySelector('iframe,frame,a,base')!==null){return 'code0-download-precondition-v1';}const a=document.createElement('a');a.href='data:application/octet-stream,';a.setAttribute('download','');a.target='_self';document.body.appendChild(a);try{a.click();}finally{a.remove();}return 'code0-download-submitted-v1';})()";
        const POPUP_SCRIPT: &str = "(()=>{if(top!==self||location.href!=='about:blank'||!document.documentElement||document.documentElement.tagName!=='HTML'||document.querySelector('iframe,frame')!==null){return 'code0-popup-precondition-v2';}setTimeout(()=>{if(top!==self||location.href!=='about:blank'||!document.documentElement||document.documentElement.tagName!=='HTML'||document.querySelector('iframe,frame')!==null){return;}window.open('about:blank','_blank');},0);return 'code0-popup-scheduled-v2';})()";

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
            download: Option<DownloadState>,
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
                    download: None,
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
                    && self.download.as_ref().is_none_or(DownloadState::quiescent)
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
                    || i.chooser.is_some()
                    || i.capability.is_some()
                    || i.refusal_registry.legacy.is_some()
                    || i.refusal_registry.top_target.is_some()
                    || i.refusal_registry.child.is_some()
                    || i.refusal_registry.dialog.is_some()
                    || i.refusal_registry.popup.is_some()
                    || i.refusal_registry.download.is_some()
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
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                n4_note(
                    &i.callbacks,
                    crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                    2071,
                    0,
                );
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
                if !callback_on_ui(self.ui, &self.callbacks, 2210) {
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
                if !callback_on_ui(self.ui, &self.callbacks, 2589) {
                    self.installation.store(true, Ordering::SeqCst);
                    n3_writer(&self.callbacks, N3W::CompletionAffinity, 0);
                    if let Some(m) = self.mailbox.upgrade() {
                        m.invalid.store(true, Ordering::SeqCst);
                    }
                    return Ok(());
                }
                let Some(m) = self.mailbox.upgrade() else {
                    self.installation.store(true, Ordering::SeqCst);
                    n3_writer(&self.callbacks, N3W::MissingMailbox, 0);
                    return Ok(());
                };
                if m.invoked.swap(true, Ordering::SeqCst) {
                    m.invalid.store(true, Ordering::SeqCst);
                    self.installation.store(true, Ordering::SeqCst);
                    n3_writer(&self.callbacks, N3W::DuplicateInvocation, 0);
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
                    n3_writer(&self.callbacks, N3W::OccupiedDisposition, 0);
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
            fail_dialog_diagnostic(owner, unknown, N3R::Unspecified, 0);
        }
        fn fail_dialog_diagnostic(
            owner: &Rc<RefCell<Owner>>,
            unknown: bool,
            reason: N3R,
            bits: u16,
        ) {
            let notification = {
                let mut state = owner.borrow_mut();
                let n3_unmatched = state.unmatched;
                let Some(i) = state.instance.as_mut() else {
                    return;
                };
                if i.refusal_registry.dialog.is_none() {
                    return;
                }
                if unknown {
                    i.refusal_registry.unknown.store(true, Ordering::SeqCst);
                    n3_writer(&i.callbacks, N3W::DialogUncertainty, bits);
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::CleanupBlock,
                        2711,
                        0,
                    );
                    let n3_was_blocked = i.cleanup_blocked;
                    i.cleanup_blocked = true;
                    n3_block(i, n3_was_blocked, n3_unmatched, N3B::Dialog, reason as u8);
                }
                i.failed = true;
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                n4_note(
                    &i.callbacks,
                    crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                    2714,
                    0,
                );
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
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                        3016,
                        0,
                    );
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        3016,
                        0,
                    );
                    i.refusal_registry.unknown.store(true, Ordering::SeqCst);
                    i.failed = true;
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                        3018,
                        0,
                    );
                    i.closing = true;
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::CleanupBlock,
                        3019,
                        0,
                    );
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
            let mut context = Context::new(owner, request);
            context.n3_event = 10;
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
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        3074,
                        0,
                    );
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
                if i.chooser.is_some()
                    || i.refusal_registry.legacy.is_some()
                    || i.refusal_registry.top_target.is_some()
                    || i.refusal_registry.child.is_some()
                    || i.refusal_registry.dialog.is_some()
                    || i.refusal_registry.dialog_completed.is_some()
                    || i.refusal_registry.popup.is_some()
                    || i.refusal_registry.download.is_some()
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
                fail_dialog_diagnostic(owner, true, N3R::InvalidMailbox, 0);
            }
            let disposition = m.disposition.lock().unwrap().take();
            let Some(d) = disposition else {
                return;
            };
            let mut uncertainty = false;
            let n3_completion_bits;
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
                    n3_writer(
                        &i.callbacks,
                        N3W::CompletionIdentity,
                        u16::from(unmatched)
                            | (u16::from(i.request == d.request) << 1)
                            | (u16::from(i.refusal_registry.generation == d.generation) << 2)
                            | (u16::from(Arc::ptr_eq(
                                &i.refusal_registry.unknown,
                                &d.installation,
                            )) << 3)
                            | (u16::from(
                                dialog
                                    .mailbox
                                    .as_ref()
                                    .is_some_and(|actual| Arc::ptr_eq(actual, &m)),
                            ) << 4)
                            | (u16::from(dialog.completion_seen) << 5)
                            | (u16::from(m.obligation.load(Ordering::SeqCst) == 1) << 6),
                    );
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
                let mut n3_invalid = None;
                let mut n3_affinity = None;
                let valid = same
                    && !i.released
                    && disposal_timely
                    && !{
                        let value = m.invalid.load(Ordering::SeqCst);
                        n3_invalid = Some(value);
                        value
                    }
                    && !{
                        let value = i.callbacks.affinity_failed.load(Ordering::SeqCst);
                        n3_affinity = Some(value);
                        value
                    }
                    && (!d.hresult_succeeded || d.bounded_result_valid && d.fixed_result_matched);
                n3_completion_bits = N3CompletionDiagnostic {
                    same,
                    not_released: !i.released,
                    disposal_timely,
                    invalid: n3_invalid,
                    affinity: n3_affinity,
                    hresult_succeeded: d.hresult_succeeded,
                    bounded: d.bounded_result_valid,
                    fixed: d.fixed_result_matched,
                    original_timely: timely,
                    valid,
                }
                .bits();
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
                fail_dialog_diagnostic(
                    owner,
                    uncertainty,
                    N3R::RejectedCompletion,
                    n3_completion_bits,
                );
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
                fail_dialog_diagnostic(owner, true, N3R::RegistryUnknown, 0);
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
            let n3_submission_diagnostic;
            let action = {
                let state = owner.borrow();
                n3_submission_diagnostic = state
                    .diagnostics
                    .as_ref()
                    .filter(|d| d.n3_enabled())
                    .map(|d| {
                        let bits = state.instance.as_ref().map_or(0, |i| {
                            u16::from(Arc::ptr_eq(
                                &i.refusal_registry.unknown,
                                expected_installation,
                            )) | (u16::from(&i.request == request) << 1)
                                | (u16::from(i.refusal_registry.generation == GENERATION) << 2)
                        });
                        (Arc::clone(d), bits)
                    });
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
                if let Some((diagnostic, bits)) = n3_submission_diagnostic {
                    diagnostic.n3_writer(N3W::SubmissionBinding, bits);
                }
                if dialog_deadline(owner, request, expected_installation).is_some() {
                    fail_dialog_diagnostic(owner, true, N3R::MissingBinding, 0);
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
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        3629,
                        0,
                    );
                    i.refusal_registry.unknown.store(true, Ordering::SeqCst);
                    n3_writer(
                        &i.callbacks,
                        N3W::CompletedRemoval,
                        u16::from(result.is_err()) | (u16::from(!same) << 1),
                    );
                }
            }
            drop(result);
            drop(core);
            if !same {
                fail_dialog_diagnostic(owner, true, N3R::RemovalMismatch, 0);
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

        // N3 diagnostic begin: called only under the existing inventory borrow.
        pub(super) fn diagnose_dialog(i: &Instance, unmatched: bool) {
            let Some(diagnostic) = i.callbacks.diagnostics.as_ref().filter(|d| d.n3_enabled())
            else {
                return;
            };
            let dialog = i.refusal_registry.dialog.as_ref();
            let mailbox = dialog.and_then(|d| d.mailbox.as_ref());
            let state = u64::from(dialog.is_some())
                | (u64::from(mailbox.is_some()) << 1)
                | (u64::from(mailbox.is_some_and(|m| m.invoked.load(Ordering::SeqCst))) << 2)
                | (u64::from(mailbox.is_some_and(|m| m.invalid.load(Ordering::SeqCst))) << 3)
                | (u64::from(dialog.is_some_and(|d| d.completion_seen)) << 4)
                | (u64::from(i.refusal_registry.unknown()) << 5)
                | (u64::from(i.cleanup_blocked) << 6)
                | (u64::from(i.controls_disposed) << 7)
                | (u64::from(i.closing) << 8)
                | (u64::from(i.released) << 9)
                | (u64::from(i.environment.is_some()) << 10)
                | (u64::from(i.environment5.is_some()) << 11)
                | (u64::from(i.exit_token.is_some()) << 12)
                | (u64::from(matches!(i.browser_exit, BrowserExitState::Observed)) << 13)
                | (u64::from(i.exit_remove_attempted) << 14)
                | (u64::from(i.callbacks.affinity_failed.load(Ordering::SeqCst)) << 15)
                | (u64::from(unmatched) << 16)
                | (u64::from(i.policy.is_some()) << 17)
                | (u64::from(i.chooser.is_some()) << 18)
                | (u64::from(i.capability.is_some()) << 19)
                | (u64::from(i.refusal_registry.legacy.is_some()) << 20)
                | (u64::from(i.refusal_registry.child.is_some()) << 21)
                | (u64::from(i.refusal_registry.popup.is_some()) << 22)
                | (u64::from(i.refusal_registry.download.is_some()) << 23)
                | (u64::from(i.destroyed) << 29);
            diagnostic.n3_publish_inventory(
                state,
                [
                    mailbox.map_or(0, |m| m.handlers.load(Ordering::SeqCst)),
                    mailbox.map_or(0, |m| m.obligation.load(Ordering::SeqCst)),
                    i.callbacks.handlers.load(Ordering::SeqCst),
                    i.callbacks.active.load(Ordering::SeqCst),
                    i.callbacks.pending.load(Ordering::SeqCst),
                ],
            );
        }
        // N3 diagnostic end.

        // N4 diagnostic begin: called under existing UI inventory borrow only.
        pub(super) fn diagnose_popup(i: &Instance, unmatched: bool) {
            let Some(diagnostic) = i.callbacks.diagnostics.as_ref().filter(|d| d.n4_enabled())
            else {
                return;
            };
            let mut words = [0u64; 12];
            words[0] = (u64::from(i.refusal_registry.unknown()) << 9)
                | (u64::from(i.failed) << 10)
                | (u64::from(i.failure_sent) << 11)
                | (u64::from(i.cleanup_blocked) << 12)
                | (u64::from(i.environment.is_some()) << 13)
                | (u64::from(i.environment5.is_some()) << 14)
                | (u64::from(i.exit_token.is_some()) << 15)
                | (u64::from(i.exit_remove_attempted) << 16)
                | (u64::from(i.controls_disposed) << 17)
                | (u64::from(i.closing) << 18)
                | (u64::from(i.released) << 19)
                | (u64::from(i.callbacks.affinity_failed.load(Ordering::SeqCst)) << 20)
                | (u64::from(unmatched) << 21);
            words[7] = i.callbacks.handlers.load(Ordering::SeqCst) as u64;
            let pending = i.callbacks.pending.load(Ordering::SeqCst);
            let active = i.callbacks.active.load(Ordering::SeqCst);
            words[8] = pending.min(u32::MAX as usize) as u64
                | ((active.min(u32::MAX as usize) as u64) << 32);
            words[0] |= (u64::from(pending > u32::MAX as usize) << 24)
                | (u64::from(active > u32::MAX as usize) << 25);
            words[9] = diagnostic.n4_time(Instant::now());
            words[11] = match i.browser_exit {
                BrowserExitState::NotStarted => 0,
                BrowserExitState::Unarmed => 1,
                BrowserExitState::Armed(_) => 2,
                BrowserExitState::Observed => 3,
                BrowserExitState::Unknown => 4,
            };
            if let Some(p) = i.refusal_registry.popup.as_ref() {
                words[0] |= 1
                    | (u64::from(p.completion_seen) << 2)
                    | (u64::from(p.closing_published) << 3)
                    | (u64::from(p.closing_since.is_some()) << 4);
                words[1] = u64::from(p.facts.script_submissions)
                    | (u64::from(p.facts.popup_events) << 8)
                    | (u64::from(p.facts.handled_attempts) << 16)
                    | (u64::from(p.facts.handled_returns) << 24)
                    | (u64::from(p.facts.handled_successes) << 32)
                    | (u64::from(p.facts.handled_readbacks) << 40)
                    | (u64::from(p.facts.script_completions) << 48);
                words[2] = u64::from(p.facts.settings_before_stimulus)
                    | (u64::from(p.facts.initial_source_matched) << 1)
                    | (u64::from(p.facts.source_after_drop_timely) << 2)
                    | (u64::from(p.facts.script_submission_returned) << 3)
                    | (u64::from(p.facts.script_submit_ok) << 4)
                    | (u64::from(p.facts.script_return_timely) << 5)
                    | (u64::from(p.facts.popup_sender_matched) << 6)
                    | (u64::from(p.facts.request_generation_live_matched) << 7)
                    | (u64::from(p.facts.issued_before_event) << 8)
                    | (u64::from(p.facts.target_uri_getter_succeeded) << 9)
                    | (u64::from(p.facts.target_uri_matched) << 10)
                    | (u64::from(p.facts.non_user_getter_succeeded) << 11)
                    | (u64::from(p.facts.non_user) << 12)
                    | (u64::from(p.facts.handled_readback_succeeded) << 13)
                    | (u64::from(p.facts.native_handled_true) << 14)
                    | (u64::from(p.facts.live_after_calls) << 15)
                    | (u64::from(p.facts.target_after_drop_timely) << 16)
                    | (u64::from(p.facts.no_new_window_assignment) << 17)
                    | (u64::from(p.facts.first_closing_observed) << 18)
                    | (u64::from(p.facts.completion_identity_matched) << 19)
                    | (u64::from(p.facts.completion_hresult_succeeded) << 20)
                    | (u64::from(p.facts.bounded_result_valid) << 21)
                    | (u64::from(p.facts.fixed_result_matched) << 22)
                    | (u64::from(p.facts.completion_before_release) << 23)
                    | (u64::from(p.facts.completion_within_deadline) << 24)
                    | (u64::from(p.facts.completion_disposition_consumed) << 25)
                    | (u64::from(p.facts.completion_obligation_drained) << 26)
                    | (u64::from(p.facts.completion_handlers_drained) << 27);
                words[3] = p.deadline.n4_deadline_time(diagnostic);
                words[4] = p
                    .first_close
                    .n4_first_close()
                    .map_or(0, |at| diagnostic.n4_time(at));
                words[0] |= (u64::from(p.first_close.n4_first_close().is_some()) << 26)
                    | (u64::from(p.deadline.n4_expired()) << 27);
                if let Some(m) = p.mailbox.as_ref() {
                    words[0] |= (1 << 1)
                        | (u64::from(m.invoked.load(Ordering::SeqCst)) << 5)
                        | (u64::from(m.invalid.load(Ordering::SeqCst)) << 6);
                    words[5] = m.obligation.load(Ordering::SeqCst) as u64;
                    words[6] = m.handlers.load(Ordering::SeqCst) as u64;
                    // Never block even on UI. Unknown/poisoned/busy stays explicit.
                    match m.disposition.try_lock() {
                        Ok(slot) => words[0] |= u64::from(slot.is_some()) << 7,
                        Err(_) => words[0] |= 1 << 22,
                    }
                    match m.closing_since.try_lock() {
                        Ok(at) => {
                            words[0] |= u64::from(at.is_some()) << 8;
                            words[10] = at.map_or(0, |at| diagnostic.n4_time(at));
                        }
                        Err(_) => words[0] |= 1 << 23,
                    }
                }
            }
            diagnostic.n4_publish_popup(words);
        }
        // N4 diagnostic end.

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
            diagnostic_result_class: u8,
            diagnostic_hresult: i32,
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
        #[windows::core::implement(
            ICoreWebView2CallDevToolsProtocolMethodCompletedHandler,
            Agile = false
        )]
        struct PopupCompletion {
            mailbox: std::sync::Weak<PopupMailbox>,
            callbacks: Arc<Callbacks>,
            installation: Arc<AtomicBool>,
            request: Code0WindowRequest,
            generation: u64,
            ui: ThreadId,
            _lifetime: PopupHandlerLifetime,
        }
        impl ICoreWebView2CallDevToolsProtocolMethodCompletedHandler_Impl for PopupCompletion_Impl {
            fn Invoke(
                &self,
                errorcode: windows::core::HRESULT,
                result: &PCWSTR,
            ) -> windows::core::Result<()> {
                let _active = Active::enter(&self.callbacks);
                // UI/STA before raw copying, allocation, or UI-local references.
                if !callback_on_ui(self.ui, &self.callbacks, 3734) {
                    n4_note(
                        &self.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        3735,
                        1,
                    );
                    self.installation.store(true, Ordering::SeqCst);
                    if let Some(m) = self.mailbox.upgrade() {
                        n4_note(
                            &self.callbacks,
                            crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                            3737,
                            1,
                        );
                        m.invalid.store(true, Ordering::SeqCst);
                    }
                    return Ok(());
                }
                let Some(m) = self.mailbox.upgrade() else {
                    n4_note(
                        &self.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        3742,
                        0,
                    );
                    self.installation.store(true, Ordering::SeqCst);
                    return Ok(());
                };
                if m.invoked.swap(true, Ordering::SeqCst) {
                    n4_note(
                        &self.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        3746,
                        2,
                    );
                    m.invalid.store(true, Ordering::SeqCst);
                    self.installation.store(true, Ordering::SeqCst);
                    return Ok(());
                }
                let received_at = Instant::now();
                let (bounded_result_valid, fixed_result_matched, diagnostic_result_class) =
                    popup_result(*result, self.callbacks.diagnostics.as_deref());
                let disposition = PopupDisposition {
                    request: self.request.clone(),
                    generation: self.generation,
                    installation: Arc::clone(&self.installation),
                    received_at,
                    hresult_succeeded: errorcode.is_ok(),
                    bounded_result_valid,
                    fixed_result_matched,
                    diagnostic_result_class,
                    diagnostic_hresult: errorcode.0,
                };
                let mut slot = m.disposition.lock().unwrap();
                if slot.is_some() {
                    n4_note(
                        &self.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        3763,
                        3,
                    );
                    m.invalid.store(true, Ordering::SeqCst);
                    self.installation.store(true, Ordering::SeqCst);
                } else {
                    *slot = Some(disposition);
                }
                Ok(())
            }
        }
        fn popup_result(raw: PCWSTR, diagnostics: Option<&Diagnostics>) -> (bool, bool, u8) {
            if raw.is_null() {
                return (false, false, 0);
            }
            let mut copied = [0u16; 2048];
            for length in 0..copied.len() {
                // Borrowed callback memory, never retained/freed or converted by
                // an unbounded wrapper. Including NUL: at most 2048 UTF16 reads.
                let unit = unsafe { *raw.0.add(length) };
                if unit == 0 {
                    let real = popup_result_units(&copied[..length]);
                    let class = if diagnostics.is_some_and(Diagnostics::n4_enabled) {
                        super::super::fixture::n4_popup_result_class(&copied[..length], real)
                    } else {
                        0
                    };
                    return (real.0, real.1, class);
                }
                copied[length] = unit;
            }
            (false, false, 5)
        }
        #[cfg(test)]
        mod popup_raw_result_tests {
            use super::*;

            fn mailbox() -> Arc<PopupMailbox> {
                Arc::new(PopupMailbox {
                    disposition: Mutex::new(None),
                    invoked: AtomicBool::new(false),
                    invalid: AtomicBool::new(false),
                    obligation: AtomicUsize::new(1),
                    handlers: AtomicUsize::new(1),
                    closing_since: Mutex::new(None),
                })
            }

            #[test]
            fn borrowed_popup_result_requires_nul_inside_the_2048_read_cap() {
                assert_eq!(popup_result(PCWSTR::null(), None), (false, false, 0));
                let exact = r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2"}}"#;
                let padded = format!("{exact}{}", " ".repeat(2047 - exact.len()));
                let mut units: Vec<u16> = padded.encode_utf16().chain(Some(0)).collect();
                assert_eq!(units.len(), 2048);
                assert_eq!(popup_result(PCWSTR(units.as_ptr()), None), (true, true, 0));
                units[2047] = 32;
                assert_eq!(
                    popup_result(PCWSTR(units.as_ptr()), None),
                    (false, false, 5)
                );
                let malformed = [0xd800, 0];
                assert_eq!(
                    popup_result(PCWSTR(malformed.as_ptr()), None),
                    (false, false, 0)
                );
            }

            #[test]
            fn actual_popup_handler_drop_does_not_settle_an_uninvoked_ticket() {
                let callbacks = Arc::new(Callbacks::default());
                let mailbox = mailbox();
                let lifetime = PopupHandlerLifetime {
                    mailbox: Arc::downgrade(&mailbox),
                    _aggregate: HandlerLifetime::new(&callbacks),
                };
                assert_eq!(callbacks.handlers.load(Ordering::SeqCst), 1);
                drop(lifetime);
                assert_eq!(callbacks.handlers.load(Ordering::SeqCst), 0);
                assert_eq!(mailbox.handlers.load(Ordering::SeqCst), 0);
                assert_eq!(mailbox.obligation.load(Ordering::SeqCst), 1);
                assert!(mailbox.disposition.lock().unwrap().is_none());
                assert!(!mailbox.invoked.load(Ordering::SeqCst));
            }

            #[test]
            fn actual_cdp_popup_callback_rejects_wrong_ui_before_invocation_or_disposition() {
                let callbacks = Arc::new(Callbacks::default());
                let mailbox = mailbox();
                let installation = Arc::new(AtomicBool::new(false));
                let other_ui = std::thread::spawn(|| std::thread::current().id())
                    .join()
                    .unwrap();
                let handler: ICoreWebView2CallDevToolsProtocolMethodCompletedHandler =
                    PopupCompletion {
                        mailbox: Arc::downgrade(&mailbox),
                        callbacks: Arc::clone(&callbacks),
                        installation: Arc::clone(&installation),
                        request: Code0WindowRequest {
                            instance_id: "n4-test-instance".into(),
                            connection_epoch: "n4-test-epoch".into(),
                            nonce: "n4-test-nonce".into(),
                            document_id: "n4-test-document".into(),
                            mode:
                                crate::platform::windows::code0_web_session::Code0WindowMode::Sync,
                        },
                        generation: GENERATION,
                        ui: other_ui,
                        _lifetime: PopupHandlerLifetime {
                            mailbox: Arc::downgrade(&mailbox),
                            _aggregate: HandlerLifetime::new(&callbacks),
                        },
                    }
                    .into();
                unsafe { handler.Invoke(windows::core::HRESULT(0), PCWSTR::null()) }.unwrap();
                assert!(callbacks.affinity_failed.load(Ordering::SeqCst));
                assert!(installation.load(Ordering::SeqCst));
                assert!(mailbox.invalid.load(Ordering::SeqCst));
                assert!(!mailbox.invoked.load(Ordering::SeqCst));
                assert!(mailbox.disposition.lock().unwrap().is_none());
                assert_eq!(callbacks.active.load(Ordering::SeqCst), 0);
                drop(handler);
                assert_eq!(mailbox.handlers.load(Ordering::SeqCst), 0);
                assert_eq!(callbacks.handlers.load(Ordering::SeqCst), 0);
                assert_eq!(mailbox.obligation.load(Ordering::SeqCst), 1);
            }
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
                        diagnose_popup(i, state.unmatched);
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
                if let Some(diagnostic) = owner.borrow().diagnostics.as_ref() {
                    diagnostic.n4_popup_stage(receipt.stage, false);
                }
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
                        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                        n4_note(
                            &i.callbacks,
                            crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                            3861,
                            0,
                        );
                        i.closing = true;
                        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                        n4_note(
                            &i.callbacks,
                            crate::platform::windows::code0_webview::fixture::N4Cause::CleanupBlock,
                            3862,
                            u16::from(unmatched)
                                | (u16::from(i.callbacks.affinity_failed.load(Ordering::SeqCst))
                                    << 1)
                                | (u16::from(i.refusal_registry.unknown()) << 2),
                        );
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
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                n4_note(
                    &i.callbacks,
                    crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                    3886,
                    0,
                );
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
                if let Some(callbacks) = owner.borrow().instance.as_ref().map(|i| &i.callbacks) {
                    n4_note(
                        callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                        3939,
                        u16::from(identity)
                            | (u16::from(require_live) << 1)
                            | (u16::from(deadline.n4_expired()) << 2),
                    );
                }
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
                if let Some(callbacks) = owner.borrow().instance.as_ref().map(|i| &i.callbacks) {
                    n4_note(
                        callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                        3954,
                        u16::from(deadline.n4_expired()),
                    );
                }
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
                    || i.chooser.is_some()
                    || i.capability.is_some()
                    || i.refusal_registry.legacy.is_some()
                    || i.refusal_registry.top_target.is_some()
                    || i.refusal_registry.child.is_some()
                    || i.refusal_registry.dialog.is_some()
                    || i.refusal_registry.dialog_completed.is_some()
                    || i.refusal_registry.popup.is_some()
                    || i.refusal_registry.download.is_some()
                {
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                        3980,
                        u16::from(i.refusal_registry.complete())
                            | (u16::from(i.refusal_registry.legacy.is_some()) << 1)
                            | (u16::from(i.refusal_registry.top_target.is_some()) << 2)
                            | (u16::from(i.refusal_registry.child.is_some()) << 3)
                            | (u16::from(i.refusal_registry.dialog.is_some()) << 4)
                            | (u16::from(i.refusal_registry.dialog_completed.is_some()) << 5)
                            | (u16::from(i.refusal_registry.popup.is_some()) << 6)
                            | (u16::from(i.refusal_registry.download.is_some()) << 7),
                    );
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
            let source_diagnostic = u16::from(source.is_some())
                | (u16::from(source.as_ref().is_some_and(|r| r.is_err())) << 1);
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
                n4_note(
                    &callbacks,
                    crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                    4026,
                    source_diagnostic
                        | (u16::from(source_matched) << 2)
                        | (u16::from(source_timely) << 3),
                );
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
            let handler: ICoreWebView2CallDevToolsProtocolMethodCompletedHandler =
                PopupCompletion {
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
            // Only this compiled ASCII program enters the fixed five-key request;
            // it contains no JSON quote/backslash and accepts no page/caller data.
            let parameters = format!(
                r#"{{"expression":"{POPUP_SCRIPT}","userGesture":false,"returnByValue":true,"includeCommandLineAPI":false,"allowUnsafeEvalBlockedByCSP":false}}"#
            );
            let wide: Vec<u16> = parameters
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
                    if let Some(i) = state.instance.as_ref() {
                        diagnose_popup(i, state.unmatched);
                    }
                }
                unsafe {
                    core.CallDevToolsProtocolMethod(
                        windows::core::w!("Runtime.evaluate"),
                        PCWSTR(wide.as_ptr()),
                        &handler,
                    )
                }
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
            drop(parameters);
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
                n4_note(
                    &callbacks,
                    crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                    4100,
                    u16::from(returned) | (u16::from(native_ok) << 1) | (u16::from(timely) << 2),
                );
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
                            i.callbacks.diagnostics.clone(),
                        )
                    })
                })
            });
            let Some((mailbox, request, installation, diagnostics)) = binding else {
                return;
            };
            let disposition = mailbox.disposition.lock().unwrap().take();
            let mut failed = false;
            let mut publish = false;
            {
                let mut state = owner.borrow_mut();
                let unmatched = state.unmatched;
                let diagnostic_binding = state.instance.as_ref().map_or(0, |i| {
                    u16::from(i.request == request)
                        | (u16::from(i.refusal_registry.generation == GENERATION) << 1)
                        | (u16::from(Arc::ptr_eq(&i.refusal_registry.unknown, &installation)) << 2)
                        | (u16::from(
                            i.refusal_registry
                                .popup
                                .as_ref()
                                .and_then(|p| p.mailbox.as_ref())
                                .is_some_and(|m| Arc::ptr_eq(m, &mailbox)),
                        ) << 3)
                });
                let Some(i) = matching(&mut state, &request).filter(|i| {
                    i.refusal_registry.generation == GENERATION
                        && Arc::ptr_eq(&i.refusal_registry.unknown, &installation)
                        && i.refusal_registry
                            .popup
                            .as_ref()
                            .and_then(|p| p.mailbox.as_ref())
                            .is_some_and(|m| Arc::ptr_eq(m, &mailbox))
                }) else {
                    if let Some(diagnostic) = diagnostics.as_ref() {
                        diagnostic.n4_first(
                            crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                            4135,
                            diagnostic_binding,
                        );
                    }
                    installation.store(true, Ordering::SeqCst);
                    mailbox.invalid.store(true, Ordering::SeqCst);
                    return;
                };
                if mailbox.invalid.load(Ordering::SeqCst) {
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        4140,
                        0,
                    );
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
                    if let Some(diagnostic) = i.callbacks.diagnostics.as_ref()
                        && diagnostic.n4_enabled()
                    {
                        let components = u64::from(same)
                            | (u64::from(!unmatched) << 1)
                            | (u64::from(d.request == i.request) << 2)
                            | (u64::from(d.generation == i.refusal_registry.generation) << 3)
                            | (u64::from(Arc::ptr_eq(&installation, &d.installation)) << 4)
                            | (u64::from(!p.completion_seen) << 5)
                            | (u64::from(p.facts.script_submissions == 1) << 6)
                            | (u64::from(mailbox.obligation.load(Ordering::SeqCst) == 1) << 7)
                            | (u64::from(!i.released) << 8)
                            | (u64::from(!i.cleanup_blocked) << 9)
                            | (u64::from(!i.callbacks.affinity_failed.load(Ordering::SeqCst))
                                << 10)
                            | (u64::from(!mailbox.invalid.load(Ordering::SeqCst)) << 11)
                            | (u64::from(!installation.load(Ordering::SeqCst)) << 12)
                            | (u64::from(d.hresult_succeeded) << 13)
                            | (u64::from(d.bounded_result_valid) << 14)
                            | (u64::from(d.fixed_result_matched) << 15)
                            | (u64::from(d.diagnostic_result_class) << 16)
                            | (u64::from(d.diagnostic_hresult as u32) << 32);
                        diagnostic.n4_publish_disposition([
                            components,
                            diagnostic.n4_time(d.received_at),
                            diagnostic.n4_time(Instant::now()),
                            u64::from(outcome.timely)
                                | (u64::from(outcome.disposable) << 1)
                                | (u64::from(outcome.successful) << 2),
                        ]);
                        if !outcome.disposable {
                            diagnostic.n4_first(
                                crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                                4182,
                                components as u16,
                            );
                        }
                        if !outcome.successful {
                            diagnostic.n4_first(crate::platform::windows::code0_webview::fixture::N4Cause::Revocation, 4189, components as u16);
                        }
                    }
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
                if let Some(diagnostic) = diagnostics.as_ref() {
                    diagnostic.n4_first(
                        crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                        4189,
                        0,
                    );
                }
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
                    n4_note(
                        &self.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        4243,
                        u16::from(p.facts.popup_events != 1) | (u16::from(!issued) << 1),
                    );
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
                    n4_note(
                        &self.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        4324,
                        u16::from(!facts.handled_readback_succeeded)
                            | (u16::from(!facts.native_handled_true) << 1),
                    );
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
                        n4_note(
                            &self.callbacks,
                            crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                            4341,
                            0,
                        );
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
                if !callback_on_ui(context.ui, &context.callbacks, 4398) {
                    n4_note(
                        &context.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        4399,
                        context.diagnostic_hook_bits(),
                    );
                    context.unknown.store(true, Ordering::SeqCst);
                    n3_writer(
                        &context.callbacks,
                        N3W::PopupAffinity,
                        u16::from(context.n3_event) << 8,
                    );
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
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                        4438,
                        index as u16,
                    );
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        4438,
                        index as u16,
                    );
                    i.refusal_registry.unknown.store(true, Ordering::SeqCst);
                    i.failed = true;
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                        4440,
                        0,
                    );
                    i.closing = true;
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::CleanupBlock,
                        4441,
                        0,
                    );
                    i.cleanup_blocked = true;
                }
            }
            drop(handler);
            result?;
            require_live(owner, request)
        }

        // N5 has no creation Pending and no additional emitter registration.
        // Its first revocation exists before Source/handler preparation; only an
        // actual final native invocation installs a completion obligation.
        struct DownloadState {
            deadline: Rc<NativeDialogDeadline>,
            core: Weak<ICoreWebView2>,
            closing_since: Option<Instant>,
            first_close: LegacyChildClose,
            closing_published: bool,
            mailbox: Option<Arc<DownloadMailbox>>,
            completion_seen: bool,
            facts: NativeDownloadFacts,
        }
        impl DownloadState {
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
        struct DownloadMailbox {
            disposition: Mutex<Option<DownloadDisposition>>,
            invoked: AtomicBool,
            invalid: AtomicBool,
            obligation: AtomicUsize,
            handlers: AtomicUsize,
            closing_since: Mutex<Option<Instant>>,
        }
        struct DownloadDisposition {
            request: Code0WindowRequest,
            generation: u64,
            installation: Arc<AtomicBool>,
            received_at: Instant,
            hresult_succeeded: bool,
            bounded_result_valid: bool,
            fixed_result_matched: bool,
        }
        struct DownloadHandlerLifetime {
            mailbox: std::sync::Weak<DownloadMailbox>,
            _aggregate: HandlerLifetime,
        }
        impl Drop for DownloadHandlerLifetime {
            fn drop(&mut self) {
                if let Some(m) = self.mailbox.upgrade() {
                    m.handlers.fetch_sub(1, Ordering::SeqCst);
                }
                // Only a real matching invocation disposition settles its ticket.
            }
        }
        #[windows::core::implement(ICoreWebView2ExecuteScriptCompletedHandler, Agile = false)]
        struct DownloadCompletion {
            mailbox: std::sync::Weak<DownloadMailbox>,
            callbacks: Arc<Callbacks>,
            installation: Arc<AtomicBool>,
            request: Code0WindowRequest,
            generation: u64,
            ui: ThreadId,
            _lifetime: DownloadHandlerLifetime,
        }
        impl ICoreWebView2ExecuteScriptCompletedHandler_Impl for DownloadCompletion_Impl {
            fn Invoke(
                &self,
                errorcode: windows::core::HRESULT,
                result: &PCWSTR,
            ) -> windows::core::Result<()> {
                let _active = Active::enter(&self.callbacks);
                // UI/STA before raw copying, allocation, or UI-local references.
                if !callback_on_ui(self.ui, &self.callbacks, 4520) {
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
                let (bounded_result_valid, fixed_result_matched) = download_result(*result);
                let disposition = DownloadDisposition {
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
        fn download_result(raw: PCWSTR) -> (bool, bool) {
            if raw.is_null() {
                return (false, false);
            }
            let mut copied = [0u16; 64];
            for length in 0..copied.len() {
                // Borrowed callback memory, never retained/freed or converted by
                // an unbounded wrapper. Including NUL: at most 64 UTF16 reads.
                let unit = unsafe { *raw.0.add(length) };
                if unit == 0 {
                    return download_result_units(&copied[..length]);
                }
                copied[length] = unit;
            }
            (false, false)
        }
        fn emit_download(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            installation: &Arc<AtomicBool>,
            stage: NativeDownloadStage,
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
                        i.refusal_registry.download.as_ref().map(|p| {
                            (
                                i.observer.clone(),
                                NativeDownloadReceipt {
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
                observer(NativeSmokeObservation::Download(receipt));
            }
        }
        pub(super) fn mark_download_close(i: &mut Instance) -> Option<Instant> {
            let p = i.refusal_registry.download.as_mut()?;
            let at = p
                .first_close
                .mark_at(Instant::now(), Some(&mut p.closing_since))?;
            p.facts.first_closing_observed = true;
            if let Some(m) = p.mailbox.as_ref() {
                *m.closing_since.lock().unwrap() = Some(at);
            }
            Some(at)
        }
        pub(super) fn publish_download_close(owner: &Rc<RefCell<Owner>>) {
            let notification = {
                let mut state = owner.borrow_mut();
                state.instance.as_mut().and_then(|i| {
                    let p = i.refusal_registry.download.as_mut()?;
                    let at = p.closing_since?;
                    if p.closing_published {
                        return None;
                    }
                    p.closing_published = true;
                    Some((i.observer.clone(), at))
                })
            };
            if let Some((Some(observer), at)) = notification {
                observer(NativeSmokeObservation::DownloadClosing { at });
            }
        }
        pub(super) fn synchronize_download_close(owner: &Rc<RefCell<Owner>>) {
            {
                let mut state = owner.borrow_mut();
                let unmatched = state.unmatched;
                if let Some(i) = state.instance.as_mut()
                    && i.refusal_registry.download.is_some()
                {
                    let invalid = unmatched
                        || i.callbacks.affinity_failed.load(Ordering::SeqCst)
                        || i.refusal_registry.unknown();
                    if invalid {
                        i.failed = true;
                        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                        n4_note(
                            &i.callbacks,
                            crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                            4647,
                            0,
                        );
                        i.closing = true;
                        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                        n4_note(
                            &i.callbacks,
                            crate::platform::windows::code0_webview::fixture::N4Cause::CleanupBlock,
                            4648,
                            0,
                        );
                        i.cleanup_blocked = true;
                    }
                    if i.closing {
                        let _ = mark_download_close(i);
                    }
                }
            }
            publish_download_close(owner);
        }
        fn fail_download(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            installation: &Arc<AtomicBool>,
        ) {
            {
                let mut state = owner.borrow_mut();
                let Some(i) = matching(&mut state, request).filter(|i| {
                    i.refusal_registry.generation == GENERATION
                        && Arc::ptr_eq(&i.refusal_registry.unknown, installation)
                        && i.refusal_registry.download.is_some()
                }) else {
                    return;
                };
                i.failed = true;
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                n4_note(
                    &i.callbacks,
                    crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                    4672,
                    0,
                );
                i.closing = true;
                let _ = mark_download_close(i);
            }
            publish_download_close(owner);
        }
        fn download_deadline(
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
            let p = i.refusal_registry.download.as_ref()?;
            let core = i.refusal_registry.core.as_ref()?;
            if !Weak::ptr_eq(&p.core, &Rc::downgrade(core)) {
                return None;
            }
            Some(Rc::clone(&p.deadline))
        }
        fn download_authorized(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            installation: &Arc<AtomicBool>,
            require_live: bool,
        ) -> Option<Rc<NativeDialogDeadline>> {
            let deadline = download_deadline(owner, request, installation)?;
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
                fail_download(owner, request, installation);
                return None;
            }
            Some(deadline)
        }
        fn download_step<T>(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            installation: &Arc<AtomicBool>,
            action: impl FnOnce() -> T,
        ) -> Option<T> {
            let deadline = download_authorized(owner, request, installation, true)?;
            let result =
                deadline.run_if_authorized_at(Instant::now(), live(owner, request), action);
            if result.is_none() {
                fail_download(owner, request, installation);
            }
            result
        }
        pub(super) fn submit_download_initial_data_empty(
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
                    || i.chooser.is_some()
                    || i.capability.is_some()
                    || i.refusal_registry.legacy.is_some()
                    || i.refusal_registry.top_target.is_some()
                    || i.refusal_registry.child.is_some()
                    || i.refusal_registry.dialog.is_some()
                    || i.refusal_registry.dialog_completed.is_some()
                    || i.refusal_registry.popup.is_some()
                    || i.refusal_registry.download.is_some()
                {
                    return Err(denied());
                }
                let core = Rc::clone(i.refusal_registry.core.as_ref().ok_or_else(denied)?);
                i.refusal_registry.download = Some(DownloadState {
                    deadline: Rc::new(deadline),
                    core: Rc::downgrade(&core),
                    closing_since: None,
                    first_close: LegacyChildClose::new(),
                    closing_published: false,
                    mailbox: None,
                    completion_seen: false,
                    facts: NativeDownloadFacts {
                        settings_before_stimulus: i.refusal_registry.settings_acknowledged,
                        ..NativeDownloadFacts::default()
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
            let source = download_step(owner, request, &installation, || {
                allocated_string(2048, |raw| unsafe { core.Source(raw) })
            });
            let source_matched = source
                .as_ref()
                .is_some_and(|r| r.as_ref().is_ok_and(|s| s == "about:blank"));
            drop(source);
            let source_timely = download_authorized(owner, request, &installation, true).is_some();
            {
                let mut state = owner.borrow_mut();
                let p = matching(&mut state, request)
                    .unwrap()
                    .refusal_registry
                    .download
                    .as_mut()
                    .unwrap();
                p.facts.initial_source_matched = source_matched;
                p.facts.source_after_drop_timely = source_timely;
            }
            if !source_matched || !source_timely {
                fail_download(owner, request, &installation);
                drop(core);
                drop(active);
                let _ = download_authorized(owner, request, &installation, false);
                return Err(denied());
            }
            let mailbox = Arc::new(DownloadMailbox {
                disposition: Mutex::new(None),
                invoked: AtomicBool::new(false),
                invalid: AtomicBool::new(false),
                obligation: AtomicUsize::new(1),
                handlers: AtomicUsize::new(1),
                closing_since: Mutex::new(None),
            });
            let handler: ICoreWebView2ExecuteScriptCompletedHandler = DownloadCompletion {
                mailbox: Arc::downgrade(&mailbox),
                callbacks: Arc::clone(&callbacks),
                installation: Arc::clone(&installation),
                request: request.clone(),
                generation: GENERATION,
                ui,
                _lifetime: DownloadHandlerLifetime {
                    mailbox: Arc::downgrade(&mailbox),
                    _aggregate: HandlerLifetime::new(&callbacks),
                },
            }
            .into();
            let wide: Vec<u16> = DOWNLOAD_SCRIPT
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let result = download_step(owner, request, &installation, || {
                {
                    let mut state = owner.borrow_mut();
                    let p = matching(&mut state, request)
                        .unwrap()
                        .refusal_registry
                        .download
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
            let _ = download_authorized(owner, request, &installation, false);
            drop(handler);
            let _ = download_authorized(owner, request, &installation, false);
            drop(core);
            let _ = download_authorized(owner, request, &installation, false);
            drop(wide);
            drop(active);
            let timely = download_authorized(owner, request, &installation, false).is_some();
            {
                let mut state = owner.borrow_mut();
                let p = matching(&mut state, request)
                    .unwrap()
                    .refusal_registry
                    .download
                    .as_mut()
                    .unwrap();
                p.facts.script_submission_returned = returned;
                p.facts.script_submit_ok = native_ok;
                p.facts.script_return_timely = timely;
            }
            if returned {
                emit_download(
                    owner,
                    request,
                    &installation,
                    NativeDownloadStage::Submitted,
                );
            }
            if !native_ok || !timely {
                fail_download(owner, request, &installation);
                return Err(denied());
            }
            Ok(())
        }
        pub(super) fn advance_download(owner: &Rc<RefCell<Owner>>) {
            let binding = owner.borrow().instance.as_ref().and_then(|i| {
                i.refusal_registry.download.as_ref().and_then(|p| {
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
                            .download
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
                    let p = i.refusal_registry.download.as_mut().unwrap();
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
                fail_download(owner, &request, &installation);
            }
            latch_unknown(owner);
            if publish {
                emit_download(
                    owner,
                    &request,
                    &installation,
                    NativeDownloadStage::Completion,
                );
            }
        }
        pub(super) fn publish_download_final(owner: &Rc<RefCell<Owner>>) {
            let binding = {
                let mut state = owner.borrow_mut();
                state.instance.as_mut().and_then(|i| {
                    i.refusal_registry.download.as_mut().map(|p| {
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
                emit_download(owner, &request, &installation, NativeDownloadStage::Final);
            }
        }
        impl Context {
            fn download_selected(&self) -> bool {
                self.owner.upgrade().is_some_and(|owner| {
                    owner.borrow().instance.as_ref().is_some_and(|i| {
                        self.matches_instance(i) && i.refusal_registry.download.is_some()
                    })
                })
            }
            fn reserve_download_target(&self) -> bool {
                let Some(owner) = self.owner.upgrade() else {
                    return false;
                };
                let mut state = owner.borrow_mut();
                let Some(i) =
                    matching(&mut state, &self.request).filter(|i| self.matches_instance(i))
                else {
                    return false;
                };
                let Some(p) = i.refusal_registry.download.as_mut() else {
                    return false;
                };
                p.facts.download_events = p.facts.download_events.saturating_add(1).min(2);
                let issued = p.facts.script_submissions == 1 && p.mailbox.is_some();
                p.facts.issued_before_event = issued;
                if p.facts.download_events != 1 || !issued {
                    self.unknown.store(true, Ordering::SeqCst);
                    return false;
                }
                true
            }
            fn download_observe(
                &self,
                event: &mut LegacyRefusalEvent,
                require_live: bool,
                eligible: bool,
            ) -> bool {
                let Some(owner) = self.owner.upgrade() else {
                    return false;
                };
                let Some(deadline) = download_deadline(&owner, &self.request, &self.unknown) else {
                    return false;
                };
                let identity =
                    download_authorized(&owner, &self.request, &self.unknown, require_live)
                        .is_some();
                event.observe_at(&deadline, Instant::now(), identity && eligible)
            }
            fn download_after_drop(&self, event: &mut LegacyRefusalEvent) -> bool {
                let Some(owner) = self.owner.upgrade() else {
                    return false;
                };
                let Some(deadline) = download_deadline(&owner, &self.request, &self.unknown) else {
                    return false;
                };
                let identity =
                    download_authorized(&owner, &self.request, &self.unknown, false).is_some();
                event.qualify_at(&deadline, Instant::now(), identity)
            }
            fn download_starting(
                &self,
                args: &ICoreWebView2DownloadStartingEventArgs,
                eligible: bool,
                event: &mut LegacyRefusalEvent,
                facts: &mut NativeDownloadFacts,
            ) -> windows::core::Result<bool> {
                facts.download_sender_matched = eligible;
                facts.request_generation_live_matched = eligible;
                let mut timely = self.download_observe(event, true, eligible);
                // Both restrictive setters run once before any optional metadata.
                // No Result propagation or qualification can skip either action.
                facts.cancel_attempts = 1;
                let canceled = event.cancel_once(|| unsafe { args.SetCancel(true) });
                facts.cancel_returns = u8::from(canceled.is_some());
                facts.cancel_successes = u8::from(canceled.as_ref().is_some_and(|r| r.is_ok()));
                facts.handled_attempts = 1;
                let handled = unsafe { args.SetHandled(true) };
                facts.handled_returns = 1;
                facts.handled_successes = u8::from(handled.is_ok());
                let action_ok = facts.cancel_successes == 1 && handled.is_ok();
                drop(canceled);
                drop(handled);
                timely &= self.download_observe(event, true, true);
                // Real readbacks are independent too, including setter failure.
                let mut cancel = BOOL::from(false);
                facts.cancel_readbacks = 1;
                let cancel_readback = unsafe { args.Cancel(&mut cancel) };
                facts.cancel_readback_succeeded = cancel_readback.is_ok();
                facts.native_cancel_true = cancel_readback.is_ok() && cancel.as_bool();
                drop(cancel_readback);
                timely &= self.download_observe(event, true, true);
                let mut handled = BOOL::from(false);
                facts.handled_readbacks = 1;
                let handled_readback = unsafe { args.Handled(&mut handled) };
                facts.handled_readback_succeeded = handled_readback.is_ok();
                facts.native_handled_true = handled_readback.is_ok() && handled.as_bool();
                drop(handled_readback);
                timely &= self.download_observe(event, true, true);
                if action_ok && timely && facts.native_cancel_true && facts.native_handled_true {
                    // This args-derived operation is a distinct COM object, never
                    // compared to the core's IUnknown or retained past Active.
                    let operation = unsafe { args.DownloadOperation() };
                    facts.operation_getter_succeeded = operation.is_ok();
                    timely &= self.download_observe(event, true, true);
                    if timely && let Ok(operation) = operation.as_ref() {
                        let uri = allocated_string(2048, |raw| unsafe { operation.Uri(raw) });
                        facts.target_uri_getter_succeeded = uri.is_ok();
                        facts.target_uri_matched = uri.as_ref().is_ok_and(|s| s == DOWNLOAD_TARGET);
                        drop(uri);
                        timely &= self.download_observe(event, true, true);
                    }
                    drop(operation);
                    timely &= self.download_observe(event, true, true);
                }
                facts.live_after_calls = timely;
                if !action_ok
                    || !facts.cancel_readback_succeeded
                    || !facts.native_cancel_true
                    || !facts.handled_readback_succeeded
                    || !facts.native_handled_true
                    || !facts.operation_getter_succeeded
                    || !facts.target_uri_getter_succeeded
                    || !facts.target_uri_matched
                    || !timely
                {
                    self.unknown.store(true, Ordering::SeqCst);
                }
                if action_ok {
                    Ok(true)
                } else {
                    Err(rejection())
                }
            }
            fn publish_download_target(&self, target: NativeDownloadFacts, timely: bool) {
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
                    let Some(p) = i.refusal_registry.download.as_mut() else {
                        return;
                    };
                    // Never overwrite invocation/completion or duplicate counters
                    // with the callback's earlier local snapshot.
                    p.facts.cancel_attempts = p
                        .facts
                        .cancel_attempts
                        .saturating_add(target.cancel_attempts)
                        .min(2);
                    p.facts.cancel_returns = p
                        .facts
                        .cancel_returns
                        .saturating_add(target.cancel_returns)
                        .min(2);
                    p.facts.cancel_successes = p
                        .facts
                        .cancel_successes
                        .saturating_add(target.cancel_successes)
                        .min(2);
                    p.facts.cancel_readbacks = p
                        .facts
                        .cancel_readbacks
                        .saturating_add(target.cancel_readbacks)
                        .min(2);
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
                    p.facts.download_sender_matched = target.download_sender_matched;
                    p.facts.request_generation_live_matched =
                        target.request_generation_live_matched;
                    p.facts.target_uri_getter_succeeded = target.target_uri_getter_succeeded;
                    p.facts.target_uri_matched = target.target_uri_matched;
                    p.facts.operation_getter_succeeded = target.operation_getter_succeeded;
                    p.facts.cancel_readback_succeeded = target.cancel_readback_succeeded;
                    p.facts.native_cancel_true = target.native_cancel_true;
                    p.facts.handled_readback_succeeded = target.handled_readback_succeeded;
                    p.facts.native_handled_true = target.native_handled_true;
                    p.facts.live_after_calls = target.live_after_calls;
                    p.facts.target_after_drop_timely = timely;
                }
                emit_download(
                    &owner,
                    &self.request,
                    &self.unknown,
                    NativeDownloadStage::Refusal,
                );
            }
        }
        fn add_download(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            index: usize,
            core: &ICoreWebView2_4,
        ) -> windows::core::Result<()> {
            let context = Context::new(owner, request);
            let handler = DownloadStartingEventHandler::create(Box::new(move |sender, args| {
                let active = Active::enter(&context.callbacks);
                if !callback_on_ui(context.ui, &context.callbacks, 5232) {
                    n4_note(
                        &context.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        5233,
                        context.diagnostic_hook_bits(),
                    );
                    context.unknown.store(true, Ordering::SeqCst);
                    n3_writer(
                        &context.callbacks,
                        N3W::DownloadAffinity,
                        u16::from(context.n3_event) << 8,
                    );
                    drop(args);
                    drop(sender);
                    drop(active);
                    return Err(rejection());
                }
                let selected = context.download_selected();
                // Reserve before Context's fallible native IUnknown queries.
                let reserved = selected && context.reserve_download_target();
                let mut event = LegacyRefusalEvent::new();
                let mut facts = NativeDownloadFacts::default();
                let result = context.invoke(sender.as_ref(), |eligible| {
                    let args = args.as_ref().ok_or_else(rejection)?;
                    if selected {
                        context.download_starting(
                            args,
                            eligible && reserved,
                            &mut event,
                            &mut facts,
                        )
                    } else {
                        let canceled = unsafe { args.SetCancel(true) };
                        let handled = unsafe { args.SetHandled(true) };
                        canceled?;
                        handled?;
                        Ok(true)
                    }
                });
                let failed = result.is_err();
                drop(result);
                drop(args);
                drop(sender);
                let timely = !selected || context.download_after_drop(&mut event);
                drop(active);
                let timely = timely && (!selected || context.download_after_drop(&mut event));
                if selected {
                    context.publish_download_target(facts, timely);
                }
                if failed { Err(rejection()) } else { Ok(()) }
            }));
            let mut token = 0;
            let result = unsafe { core.add_DownloadStarting(&handler, &mut token) };
            {
                let mut state = owner.borrow_mut();
                let i = matching(&mut state, request).unwrap();
                if result.is_ok() {
                    i.refusal_registry.entries[index].token = Some(token);
                } else {
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                        5281,
                        index as u16,
                    );
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        5281,
                        index as u16,
                    );
                    i.refusal_registry.unknown.store(true, Ordering::SeqCst);
                    i.failed = true;
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                        5283,
                        0,
                    );
                    i.closing = true;
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::CleanupBlock,
                        5284,
                        0,
                    );
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
                let n3_unmatched = state.unmatched;
                if let Some(i) = state.instance.as_mut()
                    && i.refusal_registry.unknown.load(Ordering::SeqCst)
                {
                    i.failed = true;
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                        5332,
                        0,
                    );
                    i.closing = true;
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::CleanupBlock,
                        5333,
                        0,
                    );
                    let n3_was_blocked = i.cleanup_blocked;
                    i.cleanup_blocked = true;
                    n3_block(i, n3_was_blocked, n3_unmatched, N3B::RegistryPropagation, 0);
                    (
                        mark_child_close(i).map(|at| (i.observer.clone(), at)),
                        mark_popup_close(i).map(|at| (i.observer.clone(), at)),
                        mark_download_close(i).map(|at| (i.observer.clone(), at)),
                    )
                } else {
                    (None, None, None)
                }
            };
            if let Some((Some(observer), at)) = observation.0 {
                observer(NativeSmokeObservation::ChildClosing { at });
            }
            if observation.1.is_some() {
                publish_popup_close(owner);
            }
            if observation.2.is_some() {
                publish_download_close(owner);
            }
        }

        struct Context {
            owner: Weak<RefCell<Owner>>,
            request: Code0WindowRequest,
            generation: u64,
            ui: ThreadId,
            callbacks: Arc<Callbacks>,
            unknown: Arc<AtomicBool>,
            diagnostic_hook: u8,
            n3_event: u8,
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
                    diagnostic_hook: i
                        .refusal_registry
                        .entries
                        .last()
                        .map_or(255, |e| e.hook as u8),
                    n3_event: i
                        .refusal_registry
                        .entries
                        .last()
                        .map_or(255, |e| e.hook as u8),
                    unknown: Arc::clone(&i.refusal_registry.unknown),
                    _lifetime: HandlerLifetime::new(&callbacks),
                    callbacks,
                }
            }
            fn diagnostic_hook_bits(&self) -> u16 {
                u16::from(self.diagnostic_hook) << 8
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
                if !callback_on_ui(self.ui, &self.callbacks, 5441) {
                    n4_note(
                        &self.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        5442,
                        self.diagnostic_hook_bits(),
                    );
                    self.unknown.store(true, Ordering::SeqCst);
                    n3_writer(
                        &self.callbacks,
                        N3W::ContextAffinity,
                        u16::from(self.n3_event) << 8,
                    );
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
                let mut identity_diagnostic =
                    u16::from(binding.is_some()) | (u16::from(sender.is_some()) << 1);
                let identity = binding.as_ref().zip(sender).is_some_and(|(core, sender)| {
                    let expected = core.cast::<IUnknown>();
                    let actual = sender.cast::<IUnknown>();
                    identity_diagnostic |=
                        (u16::from(expected.is_ok()) << 2) | (u16::from(actual.is_ok()) << 3);
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
                            && (!s3::selected(owner) || s3::authorized(owner, true))
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
                    n4_note(
                        &self.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        5499,
                        identity_diagnostic
                            | (u16::from(!same_installation) << 4)
                            | (u16::from(result.is_err()) << 5)
                            | (u16::from(identity) << 6)
                            | (u16::from(eligible) << 7)
                            | self.diagnostic_hook_bits(),
                    );
                    self.unknown.store(true, Ordering::SeqCst);
                    n3_writer(
                        &self.callbacks,
                        N3W::ContextIdentity,
                        identity_diagnostic
                            | (u16::from(!same_installation) << 4)
                            | (u16::from(result.is_err()) << 5)
                            | (u16::from(identity) << 6)
                            | (u16::from(eligible) << 7)
                            | (u16::from(self.n3_event) << 8),
                    );
                }
                let mut top_close = None;
                let mut child_close = None;
                let mut dialog_close = None;
                let mut popup_close = None;
                let mut download_close = None;
                if let Some(owner) = owner.as_ref() {
                    let mut state = owner.borrow_mut();
                    let n3_unmatched = state.unmatched;
                    if let Some(i) = matching(&mut state, &self.request)
                        && self.matches_instance(i)
                    {
                        if self.unknown.load(Ordering::SeqCst) {
                            i.failed = true;
                            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                            n4_note(&i.callbacks, crate::platform::windows::code0_webview::fixture::N4Cause::Revocation, 5513, 0);
                            i.closing = true;
                            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                            n4_note(&i.callbacks, crate::platform::windows::code0_webview::fixture::N4Cause::CleanupBlock, 5514, 0);
                            let n3_was_blocked = i.cleanup_blocked;
                            i.cleanup_blocked = true;
                            n3_block(
                                i,
                                n3_was_blocked,
                                n3_unmatched,
                                N3B::RefusalCallback,
                                self.n3_event,
                            );
                        } else if result.as_ref().is_ok_and(|refused| *refused) {
                            n4_note(&i.callbacks, crate::platform::windows::code0_webview::fixture::N4Cause::Revocation, 5519, self.diagnostic_hook_bits() | u16::from(identity) | (u16::from(same_installation) << 1) | (u16::from(eligible) << 2));
                            // Existing poll delivers Failed(PolicyRejected),
                            // outside this borrow and without session reentry.
                            i.failed = true;
                            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                            n4_note(&i.callbacks, crate::platform::windows::code0_webview::fixture::N4Cause::Revocation, 5519, 0);
                            i.closing = true;
                        }
                        if i.closing {
                            s3::revoke(i);
                            s4::mark_close(i);
                            top_close = mark_top_close(i).map(|at| (i.observer.clone(), at));
                            child_close = mark_child_close(i).map(|at| (i.observer.clone(), at));
                            dialog_close = mark_dialog_close(i).map(|at| (i.observer.clone(), at));
                            popup_close = mark_popup_close(i).map(|at| (i.observer.clone(), at));
                            download_close =
                                mark_download_close(i).map(|at| (i.observer.clone(), at));
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
                if download_close.is_some()
                    && let Some(owner) = owner.as_ref()
                {
                    publish_download_close(owner);
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
                    || i.chooser.is_some()
                    || i.capability.is_some()
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
                if let Some(owner) = self.owner.upgrade()
                    && s3::selected(&owner)
                {
                    return s3::navigation(&owner, args, eligible);
                }
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
                let mut chooser_selected = false;
                let result = context.invoke(sender.as_ref(), |eligible| {
                    let args = args.as_ref().ok_or_else(rejection)?;
                    dialog_selected = context.dialog_selected();
                    chooser_selected = context.owner.upgrade().is_some_and(|o| s3::selected(&o));
                    context.top_navigation(args, eligible, &mut event, &mut receipt)
                });
                drop(args);
                drop(sender);
                if dialog_selected {
                    let _ = context.dialog_after_drop();
                }
                if chooser_selected && let Some(owner) = context.owner.upgrade() {
                    s3::qualify_navigation(&owner);
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
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                        5931,
                        index as u16,
                    );
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        5931,
                        index as u16,
                    );
                    i.refusal_registry.unknown.store(true, Ordering::SeqCst);
                    i.failed = true;
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                        5933,
                        0,
                    );
                    i.closing = true;
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::CleanupBlock,
                        5934,
                        0,
                    );
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
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                        5986,
                        index as u16,
                    );
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                        5986,
                        index as u16,
                    );
                    i.refusal_registry.unknown.store(true, Ordering::SeqCst);
                    i.failed = true;
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                        5988,
                        0,
                    );
                    i.closing = true;
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::CleanupBlock,
                        5989,
                        0,
                    );
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
                        n4_note(
                            &i.callbacks,
                            crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                            6022,
                            $index as u16,
                        );
                        n4_note(
                            &i.callbacks,
                            crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                            6022,
                            $index as u16,
                        );
                        i.refusal_registry.unknown.store(true, Ordering::SeqCst);
                        i.failed = true;
                        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                        n4_note(
                            &i.callbacks,
                            crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                            6024,
                            0,
                        );
                        i.closing = true;
                        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                        n4_note(
                            &i.callbacks,
                            crate::platform::windows::code0_webview::fixture::N4Cause::CleanupBlock,
                            6025,
                            0,
                        );
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
                (Hook::Download, Emitter::Download(core)) => {
                    add_download(owner, request, index, core)
                }
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
                if let Some(i) = owner.borrow().instance.as_ref() {
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                        6212,
                        i.refusal_registry.installed_mask()
                            | (u16::from(i.refusal_registry.settings_acknowledged) << 10),
                    );
                }
                failed(owner, request);
                Code0SessionFailure::PolicyRejected
            })
        }

        // Read-only generation of the actual ten-hook installation. No grant.
        pub(super) fn finite_generation(i: &Instance) -> u64 {
            i.refusal_registry.generation
        }

        pub(super) fn chooser_core(i: &Instance) -> Option<Rc<ICoreWebView2>> {
            if i.refusal_registry.legacy.is_some()
                || i.refusal_registry.top_target.is_some()
                || i.refusal_registry.child.is_some()
                || i.refusal_registry.dialog.is_some()
                || i.refusal_registry.dialog_completed.is_some()
                || i.refusal_registry.popup.is_some()
                || i.refusal_registry.download.is_some()
                || !i.refusal_registry.complete()
            {
                return None;
            }
            i.refusal_registry.core.as_ref().map(Rc::clone)
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
                        n4_note(
                            &i.callbacks,
                            crate::platform::windows::code0_webview::fixture::N4Cause::Unknown,
                            6291,
                            hook as u16
                                | ((match emitter.as_ref() {
                                    Emitter::Core(_) => 0u16,
                                    Emitter::External(_) => 1,
                                    Emitter::Download(_) => 2,
                                    Emitter::Authentication(_) => 3,
                                    Emitter::Certificate(_) => 4,
                                    Emitter::Tls(_) => 5,
                                }) << 8),
                        );
                        i.refusal_registry.unknown.store(true, Ordering::SeqCst);
                        n3_writer(
                            &i.callbacks,
                            N3W::HookRemoval,
                            hook as u16
                                | ((match emitter.as_ref() {
                                    Emitter::Core(_) => 0u16,
                                    Emitter::External(_) => 1,
                                    Emitter::Download(_) => 2,
                                    Emitter::Authentication(_) => 3,
                                    Emitter::Certificate(_) => 4,
                                    Emitter::Tls(_) => 5,
                                }) << 8),
                        );
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

    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    pub(super) fn begin_chooser(
        owner: &Rc<RefCell<Owner>>,
        request: &Code0WindowRequest,
        root: &Path,
        deadline: super::fixture::NativeDialogDeadline,
    ) -> Result<(), Code0SessionFailure> {
        s3::begin(owner, request, root, deadline)
    }

    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    pub(super) fn s4_begin(
        owner: &Rc<RefCell<Owner>>,
        request: &Code0WindowRequest,
        selector: super::fixture::NativeCapabilitySelector,
        deadline: super::fixture::NativeDialogDeadline,
    ) -> Result<(), Code0SessionFailure> {
        s4::begin(owner, request, selector, deadline)
    }

    /// Selected S3 fixture only; no public/general CDP executor.
    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    mod s3 {
        use super::super::chooser_protocol::{
            self as wire, Binding, Context as World, Epoch, Event, Frame, Id, Reply, ReplyKind,
        };
        use super::super::fixture::{
            NativeChooserFacts, NativeChooserReceipt, NativeChooserStage, NativeDialogDeadline,
        };
        use super::*;
        use std::time::Instant;
        use webview2_com::{
            DevToolsProtocolEventReceivedEventHandler, NavigationCompletedEventHandler,
        };
        use windows::core::BOOL;

        struct Inventory {
            handlers: AtomicUsize,
            issued: AtomicUsize,
            invoked: AtomicUsize,
            consumed: AtomicUsize,
            text: AtomicUsize,
            invalid: AtomicBool,
        }
        impl Inventory {
            fn new() -> Self {
                Self {
                    handlers: AtomicUsize::new(0),
                    issued: AtomicUsize::new(0),
                    invoked: AtomicUsize::new(0),
                    consumed: AtomicUsize::new(0),
                    text: AtomicUsize::new(0),
                    invalid: AtomicBool::new(false),
                }
            }
            fn once(&self, index: usize, slot: &AtomicUsize, required: usize) -> bool {
                if index >= 11 {
                    self.invalid.store(true, Ordering::SeqCst);
                    return false;
                }
                let bit = 1usize << index;
                let good = required & bit != 0 && slot.fetch_or(bit, Ordering::SeqCst) & bit == 0;
                if !good {
                    self.invalid.store(true, Ordering::SeqCst);
                }
                good
            }
            fn issue_once(&self, index: usize) -> bool {
                self.once(index, &self.issued, 0x07ff)
            }
            fn invoke_once(&self, index: usize) -> bool {
                self.once(index, &self.invoked, self.issued.load(Ordering::SeqCst))
            }
            fn consume_once(&self, index: usize) -> bool {
                self.once(index, &self.consumed, self.invoked.load(Ordering::SeqCst))
            }
            fn drained(&self) -> bool {
                let issued = self.issued.load(Ordering::SeqCst);
                issued == self.invoked.load(Ordering::SeqCst)
                    && issued == self.consumed.load(Ordering::SeqCst)
            }
        }
        struct Text {
            inventory: Arc<Inventory>,
            bytes: usize,
        }
        impl Text {
            fn reserve(inventory: &Arc<Inventory>, bytes: usize) -> Option<Self> {
                if inventory
                    .text
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
                        n.checked_add(bytes).filter(|n| wire::text_fits(0, 0, *n))
                    })
                    .is_err()
                {
                    inventory.invalid.store(true, Ordering::SeqCst);
                    return None;
                }
                Some(Self {
                    inventory: Arc::clone(inventory),
                    bytes,
                })
            }
            fn shrink(&mut self, bytes: usize) {
                self.inventory
                    .text
                    .fetch_sub(self.bytes - bytes, Ordering::SeqCst);
                self.bytes = bytes;
            }
        }
        impl Drop for Text {
            fn drop(&mut self) {
                self.inventory.text.fetch_sub(self.bytes, Ordering::SeqCst);
            }
        }
        struct Lifetime {
            inventory: Arc<Inventory>,
            _aggregate: HandlerLifetime,
        }
        impl Lifetime {
            fn new(callbacks: &Arc<Callbacks>, inventory: &Arc<Inventory>) -> Self {
                inventory.handlers.fetch_add(1, Ordering::SeqCst);
                Self {
                    inventory: Arc::clone(inventory),
                    _aggregate: HandlerLifetime::new(callbacks),
                }
            }
        }
        impl Drop for Lifetime {
            fn drop(&mut self) {
                self.inventory.handlers.fetch_sub(1, Ordering::SeqCst);
            }
        }
        struct Receiver {
            receiver: Rc<ICoreWebView2DevToolsProtocolEventReceiver>,
            token: Option<i64>,
            attempted: bool,
        }
        struct Completed {
            core: Rc<ICoreWebView2>,
            token: Option<i64>,
            attempted: bool,
        }
        pub(super) struct Controls {
            _core: Option<Rc<ICoreWebView2>>,
            _receivers: Vec<Receiver>,
            _completed: Option<Completed>,
        }
        struct Response {
            index: usize,
            received: Instant,
            valid: Result<Reply, ()>,
        }
        pub(super) struct State {
            request: Code0WindowRequest,
            installation: Arc<AtomicBool>,
            core: Option<Rc<ICoreWebView2>>,
            inventory: Arc<Inventory>,
            target: String,
            _target_charge: Option<Text>,
            _id_charge: Option<Text>,
            epoch: Epoch,
            closing: Option<Instant>,
            closing_published: bool,
            receivers: Vec<Receiver>,
            completed: Option<Completed>,
            ready: bool,
            next: usize,
            in_flight: bool,
            response: Option<Response>,
            candidate: Option<World>,
            chooser: bool,
            initial: Option<u32>,
            file: Option<u32>,
            destroyed: u8,
            bootstrap_cleared: bool,
            cleared: bool,
            updated: bool,
            detached: bool,
            blank_frame: bool,
            file_frame: bool,
            world_ack: Option<u32>,
            context: Option<u32>,
            unique: Option<Id>,
            frame: Option<Id>,
            loader: Option<Id>,
            root: u32,
            node: u32,
            backend: u32,
            grant: bool,
            start: Option<u64>,
            completed_good: bool,
            source_checked: bool,
            facts: NativeChooserFacts,
        }
        struct Context {
            owner: Weak<RefCell<Owner>>,
            request: Code0WindowRequest,
            installation: Arc<AtomicBool>,
            inventory: Arc<Inventory>,
            callbacks: Arc<Callbacks>,
            ui: ThreadId,
        }
        impl Context {
            fn new(owner: &Rc<RefCell<Owner>>) -> Self {
                let o = owner.borrow();
                let i = o.instance.as_ref().unwrap();
                let s = i.chooser.as_ref().unwrap();
                Self {
                    owner: Rc::downgrade(owner),
                    request: i.request.clone(),
                    installation: Arc::clone(&s.installation),
                    inventory: Arc::clone(&s.inventory),
                    callbacks: Arc::clone(&i.callbacks),
                    ui: o.ui_thread,
                }
            }
            fn same(&self, owner: &Rc<RefCell<Owner>>) -> bool {
                let o = owner.borrow();
                !o.unmatched
                    && o.instance.as_ref().is_some_and(|i| {
                        i.request == self.request
                            && Arc::ptr_eq(&i.callbacks, &self.callbacks)
                            && i.chooser.as_ref().is_some_and(|s| {
                                Arc::ptr_eq(&s.inventory, &self.inventory)
                                    && Arc::ptr_eq(&s.installation, &self.installation)
                            })
                    })
            }
            fn sender(&self, owner: &Rc<RefCell<Owner>>, sender: Option<&ICoreWebView2>) -> bool {
                if !callback_window(owner, self) {
                    return false;
                }
                let core = {
                    let o = owner.borrow();
                    o.instance
                        .as_ref()
                        .and_then(|i| i.chooser.as_ref())
                        .and_then(|s| s.core.as_ref())
                        .map(Rc::clone)
                };
                let good = core.as_ref().zip(sender).is_some_and(|(core, sender)| {
                    let expected = core.cast::<IUnknown>();
                    if !callback_window(owner, self) {
                        drop(expected);
                        return false;
                    }
                    let actual = sender.cast::<IUnknown>();
                    let same = callback_window(owner, self)
                        && expected
                            .as_ref()
                            .ok()
                            .zip(actual.as_ref().ok())
                            .is_some_and(|(a, b)| a.as_raw() == b.as_raw());
                    drop(actual);
                    drop(expected);
                    same && callback_window(owner, self)
                });
                drop(core);
                good && callback_window(owner, self)
            }
        }
        fn failure() -> Code0SessionFailure {
            Code0SessionFailure::PolicyRejected
        }
        pub(super) fn selected(owner: &Rc<RefCell<Owner>>) -> bool {
            owner
                .borrow()
                .instance
                .as_ref()
                .is_some_and(|i| i.chooser.is_some())
        }
        fn valid_identity(i: &Instance) -> bool {
            !i.released
                && !i.destroyed
                && !i.cleanup_blocked
                && !i.refusal_registry.unknown()
                && !i.callbacks.affinity_failed.load(Ordering::SeqCst)
        }
        pub(super) fn authorized(owner: &Rc<RefCell<Owner>>, live: bool) -> bool {
            let good = {
                let o = owner.borrow();
                !o.unmatched
                    && o.instance.as_ref().is_some_and(|i| {
                        valid_identity(i)
                            && (!live || (!i.closing && !i.failed))
                            && i.chooser.as_ref().is_some_and(|s| {
                                s.request == i.request
                                    && s.epoch.timely(Instant::now())
                                    && !s.inventory.invalid.load(Ordering::SeqCst)
                            })
                    })
            };
            if !good {
                fail(owner, false);
            }
            good
        }
        fn fail(owner: &Rc<RefCell<Owner>>, unknown: bool) {
            {
                let mut o = owner.borrow_mut();
                if let Some(i) = o.instance.as_mut()
                    && let Some(s) = i.chooser.as_mut()
                {
                    s.epoch.invalidate(10);
                    i.failed = true;
                    i.closing = true;
                    if unknown {
                        i.cleanup_blocked = true;
                        s.installation.store(true, Ordering::SeqCst);
                    }
                    mark_close(i);
                }
            }
            synchronize(owner);
        }
        pub(super) fn revoke(i: &mut Instance) {
            if let Some(s) = i.chooser.as_mut() {
                s.epoch.observe_revocation(true, false);
            }
            mark_close(i);
        }
        pub(super) fn mark_close(i: &mut Instance) {
            let identity = valid_identity(i);
            if i.closing
                && let Some(s) = i.chooser.as_mut()
            {
                let chooser_caused = s.chooser && (identity || s.epoch.stop.is_some());
                s.epoch.observe_revocation(true, chooser_caused);
                if s.closing.is_none() {
                    s.closing = Some(Instant::now());
                    s.facts.first_closing_observed = true;
                }
            }
        }
        pub(super) fn synchronize(owner: &Rc<RefCell<Owner>>) {
            let notification = {
                let mut o = owner.borrow_mut();
                o.instance.as_mut().and_then(|i| {
                    mark_close(i);
                    let s = i.chooser.as_mut()?;
                    if s.closing_published {
                        return None;
                    }
                    let at = s.closing?;
                    s.closing_published = true;
                    Some((i.observer.clone(), at))
                })
            };
            if let Some((Some(f), at)) = notification {
                f(NativeSmokeObservation::ChooserClosing { at });
            }
        }
        pub(super) fn stop_entry(owner: &Rc<RefCell<Owner>>) {
            cleanup_window(owner);
            let mut o = owner.borrow_mut();
            if let Some(s) = o.instance.as_mut().and_then(|i| i.chooser.as_mut()) {
                s.epoch.stop_at(Instant::now());
                s.facts.actual_stop_entry_observed = true;
            }
        }
        fn emit(owner: &Rc<RefCell<Owner>>, stage: NativeChooserStage) {
            let event = {
                let o = owner.borrow();
                o.instance.as_ref().and_then(|i| {
                    i.chooser.as_ref().map(|s| {
                        (
                            i.observer.clone(),
                            NativeChooserReceipt {
                                request: i.request.clone(),
                                installation: Arc::clone(&s.installation),
                                stage,
                                facts: s.facts,
                            },
                        )
                    })
                })
            };
            if let Some((Some(f), receipt)) = event {
                f(NativeSmokeObservation::Chooser(receipt));
            }
        }
        fn step<T>(owner: &Rc<RefCell<Owner>>, action: impl FnOnce() -> T) -> Option<T> {
            if !authorized(owner, true) {
                return None;
            }
            let result = action();
            if authorized(owner, true) {
                Some(result)
            } else {
                drop(result);
                None
            }
        }
        // Before dereferencing native callback text, and after every native getter
        // and temporary Drop, rejoin the same installation and callback window.
        // Known post-Stop cleanup gets only the original first-close ten seconds.
        fn callback_window(owner: &Rc<RefCell<Owner>>, context: &Context) -> bool {
            if !context.same(owner) {
                return false;
            }
            let mut o = owner.borrow_mut();
            o.instance.as_mut().is_some_and(|i| {
                let identity = valid_identity(i);
                let revoked = i.failed || i.closing;
                i.chooser.as_mut().is_some_and(|s| {
                    s.epoch
                        .observe_revocation(revoked || !identity, s.chooser && identity);
                    let now = Instant::now();
                    identity
                        && s.core.is_some()
                        && if s.epoch.stop.is_some() {
                            NativeDialogDeadline::disposal_within_close_at(now, now, s.closing)
                        } else {
                            s.epoch.timely(now) && !s.inventory.invalid.load(Ordering::SeqCst)
                        }
                })
            })
        }
        fn cleanup_window(owner: &Rc<RefCell<Owner>>) {
            let mut o = owner.borrow_mut();
            if let Some(s) = o.instance.as_mut().and_then(|i| i.chooser.as_mut()) {
                let now = Instant::now();
                if (s.epoch.stop.is_none() && now >= s.epoch.end)
                    || !NativeDialogDeadline::disposal_within_close_at(now, now, s.closing)
                {
                    s.epoch.invalidate(10);
                }
            }
        }
        // The charge is reserved before reading/copying. COM allocation is freed
        // even on HRESULT/null/size/encoding failure, outside any Owner borrow.
        fn copy_units(raw: PCWSTR, cap: usize) -> Result<String, ()> {
            if raw.is_null() {
                return Err(());
            }
            for len in 0..=2048 {
                if unsafe { *raw.0.add(len) } == 0 {
                    let units = unsafe { std::slice::from_raw_parts(raw.0, len) };
                    let mut bytes = 0;
                    for c in char::decode_utf16(units.iter().copied()) {
                        bytes += c.map_err(|_| ())?.len_utf8();
                        if bytes > cap {
                            return Err(());
                        }
                    }
                    return String::from_utf16(units).map_err(|_| ());
                }
            }
            Err(())
        }
        fn allocated(
            cap: usize,
            get: impl FnOnce(*mut PWSTR) -> windows::core::Result<()>,
            qualified: impl Fn() -> bool,
        ) -> Result<String, ()> {
            let mut raw = PWSTR::null();
            let result = get(&mut raw);
            let allocation = CoTaskMemPWSTR::from(raw);
            let out = if result.is_ok() && qualified() {
                copy_units(PCWSTR(raw.0), cap)
            } else {
                Err(())
            };
            drop(result);
            drop(allocation);
            if qualified() {
                out
            } else {
                drop(out);
                Err(())
            }
        }
        fn source(owner: &Rc<RefCell<Owner>>, core: &ICoreWebView2, initial: bool) -> bool {
            let inventory = {
                let o = owner.borrow();
                Arc::clone(
                    &o.instance
                        .as_ref()
                        .unwrap()
                        .chooser
                        .as_ref()
                        .unwrap()
                        .inventory,
                )
            };
            let Some(_charge) = Text::reserve(&inventory, 1024) else {
                fail(owner, false);
                return false;
            };
            let Some(value) = step(owner, || {
                allocated(
                    1024,
                    |p| unsafe { core.Source(p) },
                    || authorized(owner, true),
                )
            }) else {
                return false;
            };
            let matched = {
                let o = owner.borrow();
                let s = o.instance.as_ref().unwrap().chooser.as_ref().unwrap();
                value
                    .as_ref()
                    .is_ok_and(|v| v == if initial { "about:blank" } else { &s.target })
            };
            drop(value);
            drop(_charge);
            matched && authorized(owner, true)
        }
        pub(super) fn begin(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            root: &Path,
            deadline: NativeDialogDeadline,
        ) -> Result<(), Code0SessionFailure> {
            super::require_native_refusal_install(owner, request)?;
            let (core, callbacks) = {
                let o = owner.borrow();
                let i = o.instance.as_ref().ok_or_else(failure)?;
                if i.chooser.is_some() {
                    return Err(failure());
                }
                (
                    s2::chooser_core(i).ok_or_else(failure)?,
                    Arc::clone(&i.callbacks),
                )
            };
            let active = Active::enter(&callbacks);
            // Identical strict canonical/local Disk-or-VerbatimDisk predicate as
            // N3; this selected URI is an additional <=1024 UTF8 precondition.
            let inventory = Arc::new(Inventory::new());
            let mut charge = Text::reserve(&inventory, 2048).ok_or_else(failure)?;
            let target = owned_uri(root)?;
            charge.shrink(target.len());
            let ids = Text::reserve(&inventory, 384).ok_or_else(failure)?;
            {
                let mut o = owner.borrow_mut();
                let i = matching(&mut o, request).ok_or_else(failure)?;
                i.chooser = Some(State {
                    request: request.clone(),
                    installation: Arc::clone(&i.refusal_registry.unknown),
                    core: Some(Rc::clone(&core)),
                    inventory,
                    target,
                    _target_charge: Some(charge),
                    _id_charge: Some(ids),
                    epoch: Epoch::new(deadline.original_end()),
                    closing: None,
                    closing_published: false,
                    receivers: Vec::with_capacity(10),
                    completed: None,
                    ready: false,
                    next: 0,
                    in_flight: false,
                    response: None,
                    candidate: None,
                    chooser: false,
                    initial: None,
                    file: None,
                    destroyed: 0,
                    bootstrap_cleared: false,
                    cleared: false,
                    updated: false,
                    detached: false,
                    blank_frame: false,
                    file_frame: false,
                    world_ack: None,
                    context: None,
                    unique: None,
                    frame: None,
                    loader: None,
                    root: 0,
                    node: 0,
                    backend: 0,
                    grant: false,
                    start: None,
                    completed_good: false,
                    source_checked: false,
                    facts: NativeChooserFacts {
                        asset_bound: true,
                        settings_and_hooks: i.refusal_registry.complete(),
                        ..NativeChooserFacts::default()
                    },
                });
            }
            if !source(owner, &core, true) {
                fail(owner, false);
                return Err(failure());
            }
            owner
                .borrow_mut()
                .instance
                .as_mut()
                .unwrap()
                .chooser
                .as_mut()
                .unwrap()
                .facts
                .initial_source = true;
            let registered = register(owner, &core);
            drop(core);
            drop(active);
            if !registered || !authorized(owner, true) {
                fail(owner, false);
                return Err(failure());
            }
            owner
                .borrow_mut()
                .instance
                .as_mut()
                .unwrap()
                .chooser
                .as_mut()
                .unwrap()
                .ready = true;
            emit(owner, NativeChooserStage::Started);
            Ok(())
        }
        fn owned_uri(root: &Path) -> Result<String, Code0SessionFailure> {
            use std::path::{Component, Prefix};
            let root = root.canonicalize().map_err(|_| failure())?;
            let fixed = root.join(wire::LEAF);
            let target = fixed.canonicalize().map_err(|_| failure())?;
            let local = |p: &Path| {
                p.is_absolute()
                    && matches!(p.components().next(),Some(Component::Prefix(v))if matches!(v.kind(),Prefix::Disk(_)|Prefix::VerbatimDisk(_)))
            };
            if !local(&root)
                || !local(&target)
                || target != fixed
                || target.parent() != Some(root.as_path())
                || target.file_name() != Some(std::ffi::OsStr::new(wire::LEAF))
            {
                return Err(failure());
            }
            let meta = target.metadata().map_err(|_| failure())?;
            if !meta.is_file() || meta.len() != 0 {
                return Err(failure());
            }
            let path_bytes = target.to_str().ok_or_else(failure)?.len();
            if !wire::uri_construction_fits(path_bytes) {
                return Err(failure());
            }
            let url = reqwest::Url::from_file_path(target).map_err(|_| failure())?;
            if url.scheme() != "file"
                || url.host_str().is_some()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
                || url.as_str().is_empty()
                || url.as_str().len() > 1024
            {
                return Err(failure());
            }
            Ok(url.into())
        }
        fn register(owner: &Rc<RefCell<Owner>>, core: &Rc<ICoreWebView2>) -> bool {
            let context = Context::new(owner);
            let registration_inventory = Arc::clone(&context.inventory);
            let lifetime = Lifetime::new(&context.callbacks, &context.inventory);
            let handler = NavigationCompletedEventHandler::create(Box::new(move |sender, args| {
                let _lifetime = &lifetime;
                let active = Active::enter(&context.callbacks);
                if !callback_on_ui(context.ui, &context.callbacks, 7301) {
                    context.inventory.invalid.store(true, Ordering::SeqCst);
                    drop(args);
                    drop(sender);
                    drop(active);
                    return Ok(());
                }
                if let Some(owner) = context.owner.upgrade() {
                    let identity = context.sender(&owner, sender.as_ref());
                    let good = if identity && authorized(&owner, true) {
                        args.as_ref().is_some_and(|args| completed(&owner, args))
                    } else {
                        false
                    };
                    drop(args);
                    drop(sender);
                    if !context.same(&owner) {
                        context.inventory.invalid.store(true, Ordering::SeqCst);
                    } else if !good || !authorized(&owner, true) {
                        fail(&owner, false);
                    }
                } else {
                    context.inventory.invalid.store(true, Ordering::SeqCst);
                    drop(args);
                    drop(sender);
                }
                drop(active);
                Ok(())
            }));
            {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().chooser.as_mut().unwrap();
                s.completed = Some(Completed {
                    core: Rc::clone(core),
                    token: None,
                    attempted: false,
                });
            }
            let mut token = 0;
            let add = step(owner, || unsafe {
                core.add_NavigationCompleted(&handler, &mut token)
            });
            let ok = add.as_ref().is_some_and(Result::is_ok);
            {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().chooser.as_mut().unwrap();
                if ok {
                    s.completed.as_mut().unwrap().token = Some(token);
                    s.facts.completed_added = true;
                }
            }
            drop(add);
            drop(handler);
            if !ok {
                fail(owner, true);
                return false;
            }
            if !authorized(owner, true) {
                return false;
            }
            for (index, name) in wire::EVENTS.iter().enumerate() {
                let Some(name_charge) = Text::reserve(&registration_inventory, name.len()) else {
                    fail(owner, false);
                    return false;
                };
                let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
                let Some(receiver) = step(owner, || unsafe {
                    core.GetDevToolsProtocolEventReceiver(PCWSTR(wide.as_ptr()))
                }) else {
                    return false;
                };
                let receiver = match receiver {
                    Ok(v) => Rc::new(v),
                    Err(e) => {
                        drop(e);
                        fail(owner, true);
                        return false;
                    }
                };
                if !authorized(owner, true) {
                    drop(receiver);
                    return false;
                }
                {
                    let mut o = owner.borrow_mut();
                    o.instance
                        .as_mut()
                        .unwrap()
                        .chooser
                        .as_mut()
                        .unwrap()
                        .receivers
                        .push(Receiver {
                            receiver: Rc::clone(&receiver),
                            token: None,
                            attempted: false,
                        });
                }
                let context = Context::new(owner);
                let lifetime = Lifetime::new(&context.callbacks, &context.inventory);
                let handler = DevToolsProtocolEventReceivedEventHandler::create(Box::new(
                    move |sender, args| {
                        let _lifetime = &lifetime;
                        let active = Active::enter(&context.callbacks);
                        if !callback_on_ui(context.ui, &context.callbacks, 7321) {
                            context.inventory.invalid.store(true, Ordering::SeqCst);
                            drop(args);
                            drop(sender);
                            drop(active);
                            return Ok(());
                        }
                        if let Some(owner) = context.owner.upgrade() {
                            let identity = context.sender(&owner, sender.as_ref());
                            let parser_charge = Text::reserve(&context.inventory, 256);
                            let event =
                                if identity && context.same(&owner) && parser_charge.is_some() {
                                    args.as_ref()
                                        .and_then(|a| receive(&owner, &context, index, a).ok())
                                } else {
                                    None
                                };
                            // Revoke and start first-close time as soon as the
                            // matched target is known, before opaque args Drop.
                            let target = if context.same(&owner) {
                                apply_event(&owner, index, event)
                            } else {
                                context.inventory.invalid.store(true, Ordering::SeqCst);
                                false
                            };
                            drop(args);
                            drop(sender);
                            drop(parser_charge);
                            if !context.same(&owner) {
                                context.inventory.invalid.store(true, Ordering::SeqCst);
                            } else if !callback_window(&owner, &context) {
                                fail(&owner, false);
                            } else if target {
                                qualify_target(&owner);
                            }
                        } else {
                            context.inventory.invalid.store(true, Ordering::SeqCst);
                            drop(args);
                            drop(sender);
                        }
                        drop(active);
                        Ok(())
                    },
                ));
                let mut token = 0;
                let added = step(owner, || unsafe {
                    receiver.add_DevToolsProtocolEventReceived(&handler, &mut token)
                });
                let ok = added.as_ref().is_some_and(Result::is_ok);
                {
                    let mut o = owner.borrow_mut();
                    let s = o.instance.as_mut().unwrap().chooser.as_mut().unwrap();
                    if ok {
                        s.receivers[index].token = Some(token);
                        s.facts.receiver_adds[index] = 1;
                    }
                }
                drop(added);
                drop(handler);
                drop(receiver);
                drop(wide);
                drop(name_charge);
                if !ok {
                    fail(owner, true);
                    return false;
                }
                if !authorized(owner, true) {
                    return false;
                }
            }
            true
        }
        fn receive(
            owner: &Rc<RefCell<Owner>>,
            context: &Context,
            index: usize,
            args: &ICoreWebView2DevToolsProtocolEventReceivedEventArgs,
        ) -> Result<Event, ()> {
            if !callback_window(owner, context) {
                return Err(());
            }
            let args2 = args
                .cast::<ICoreWebView2DevToolsProtocolEventReceivedEventArgs2>()
                .map_err(|_| ())?;
            if !callback_window(owner, context) {
                drop(args2);
                return Err(());
            }
            let mut raw = PWSTR::null();
            let got = unsafe { args2.SessionId(&mut raw) };
            let allocation = CoTaskMemPWSTR::from(raw);
            let empty = callback_window(owner, context)
                && got.is_ok()
                && !raw.is_null()
                && unsafe { *raw.0 } == 0;
            drop(got);
            drop(allocation);
            drop(args2);
            if !empty || !callback_window(owner, context) {
                return Err(());
            }
            let cap = [512, 1024, 256, 64, 64, 512, 512, 2048, 2048, 2048][index];
            let _charge = Text::reserve(&context.inventory, cap).ok_or(())?;
            let raw = allocated(
                cap,
                |p| unsafe { args.ParameterObjectAsJson(p) },
                || callback_window(owner, context),
            )?;
            if !callback_window(owner, context) {
                drop(raw);
                drop(_charge);
                return Err(());
            }
            let out = {
                let o = owner.borrow();
                let s = o.instance.as_ref().unwrap().chooser.as_ref().unwrap();
                let binding = s.frame.map(|frame| Binding {
                    frame,
                    backend: s.backend,
                });
                let uri = if index == 7 && !s.facts.navigation_issued {
                    "about:blank"
                } else {
                    &s.target
                };
                wire::parse_event(index, &raw, uri, binding.as_ref())
            };
            drop(raw);
            drop(_charge);
            if !callback_window(owner, context) {
                return Err(());
            }
            out
        }
        fn apply_event(owner: &Rc<RefCell<Owner>>, index: usize, event: Option<Event>) -> bool {
            let mut target = false;
            let mut rejected = false;
            {
                let mut o = owner.borrow_mut();
                let i = o.instance.as_mut().unwrap();
                let identity = valid_identity(i);
                let revoked = i.failed || i.closing;
                let s = i.chooser.as_mut().unwrap();
                s.epoch
                    .observe_revocation(revoked || !identity, s.chooser && identity);
                let post = s.epoch.stop.is_some();
                let live = identity
                    && s.epoch.timely(Instant::now())
                    && !s.inventory.invalid.load(Ordering::SeqCst);
                let bounded = s.ready && (s.epoch.armed || post || s.epoch.bootstrap_delivery());
                s.facts.bootstrap_delivered = s.epoch.bootstrap;
                if index == 0 {
                    s.facts.target_events = s.facts.target_events.saturating_add(1);
                }
                let valid = bounded
                    && match event {
                        Some(Event::Chooser) => {
                            target = live && s.epoch.target_at(Instant::now()) && !s.chooser;
                            if target {
                                s.chooser = true;
                            }
                            target
                        }
                        Some(Event::Created(c)) => {
                            if post || !live {
                                false
                            } else if s.epoch.armed {
                                if s.facts.commands_issued[4] == 1
                                    && c.named
                                    && s.candidate.is_none()
                                    && s.context.is_none()
                                    && s.initial != Some(c.id)
                                    && s.file != Some(c.id)
                                {
                                    s.candidate = Some(c);
                                    true
                                } else {
                                    false
                                }
                            } else if !c.named
                                && s.initial.is_none()
                                && s.facts.commands_issued[1] == 1
                                && !s.facts.navigation_issued
                            {
                                s.initial = Some(c.id);
                                s.facts.initial_default_session_observed = true;
                                true
                            } else if !c.named
                                && s.facts.navigation_issued
                                && s.file.is_none()
                                && s.initial != Some(c.id)
                            {
                                s.file = Some(c.id);
                                true
                            } else {
                                false
                            }
                        }
                        Some(Event::Destroyed(id, unique)) => {
                            let position = if s.initial == Some(id) {
                                Some(0)
                            } else if s.file == Some(id) {
                                Some(1)
                            } else if s.context == Some(id) {
                                Some(2)
                            } else {
                                None
                            };
                            position.is_some_and(|p| {
                                let known_unique =
                                    p != 2 || unique.is_none_or(|u| s.unique == Some(u));
                                let allowed = known_unique
                                    && (post
                                        || (!s.epoch.armed && s.facts.navigation_issued && p == 0))
                                    && s.destroyed & (1 << p) == 0;
                                if allowed {
                                    s.destroyed |= 1 << p;
                                }
                                allowed
                            })
                        }
                        Some(Event::Cleared) => {
                            if post {
                                let allowed = !s.cleared;
                                if allowed {
                                    s.cleared = true;
                                }
                                allowed
                            } else {
                                let allowed = live
                                    && !s.epoch.armed
                                    && s.facts.navigation_issued
                                    && !s.bootstrap_cleared;
                                if allowed {
                                    s.bootstrap_cleared = true;
                                }
                                allowed
                            }
                        }
                        Some(Event::Updated) => {
                            let allowed = post && !s.updated;
                            if allowed {
                                s.updated = true;
                            }
                            allowed
                        }
                        Some(Event::Detached(id, remove)) => {
                            let allowed = post && remove && s.frame == Some(id) && !s.detached;
                            if allowed {
                                s.detached = true;
                            }
                            allowed
                        }
                        Some(Event::Navigated(regular)) => {
                            if !live || post || s.epoch.armed || !regular {
                                false
                            } else if !s.facts.navigation_issued && !s.blank_frame {
                                s.blank_frame = true;
                                true
                            } else if s.facts.navigation_issued && !s.file_frame {
                                s.file_frame = true;
                                true
                            } else {
                                false
                            }
                        }
                        _ => false,
                    };
                if !valid {
                    s.epoch.invalidate(index);
                    rejected = true;
                }
                if target || rejected {
                    i.failed = true;
                    i.closing = true;
                    mark_close(i);
                }
            }
            synchronize(owner);
            if rejected {
                fail(owner, false);
            }
            target
        }
        fn qualify_target(owner: &Rc<RefCell<Owner>>) {
            let good = {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().chooser.as_mut().unwrap();
                let good = s.chooser && s.epoch.timely(Instant::now()) && s.epoch.stop.is_none();
                if good {
                    s.facts.target_session_identity = true;
                    s.facts.target_pre_stop = true;
                }
                good
            };
            if good {
                emit(owner, NativeChooserStage::Target);
            } else {
                fail(owner, false);
            }
        }
        pub(super) fn navigation(
            owner: &Rc<RefCell<Owner>>,
            args: &ICoreWebView2NavigationStartingEventArgs,
            eligible: bool,
        ) -> windows::core::Result<bool> {
            let allowed = eligible && authorized(owner, true);
            let granted = {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().chooser.as_mut().unwrap();
                s.facts.native_starts = s.facts.native_starts.saturating_add(1);
                let a =
                    allowed && s.grant && s.facts.navigation_issued && s.facts.native_starts == 1;
                s.grant = false;
                a
            };
            if !granted {
                fail(owner, false);
                unsafe { args.SetCancel(true) }?;
                return Ok(true);
            }
            let inventory = {
                let o = owner.borrow();
                Arc::clone(
                    &o.instance
                        .as_ref()
                        .unwrap()
                        .chooser
                        .as_ref()
                        .unwrap()
                        .inventory,
                )
            };
            let charge = Text::reserve(&inventory, 1024);
            let uri = if charge.is_some() {
                step(owner, || {
                    allocated(1024, |p| unsafe { args.Uri(p) }, || authorized(owner, true))
                })
            } else {
                None
            };
            let mut id = 0;
            let a = step(owner, || unsafe { args.NavigationId(&mut id) });
            let mut redirected = BOOL::default();
            let b = step(owner, || unsafe { args.IsRedirected(&mut redirected) });
            let mut canceled = BOOL::default();
            let c = step(owner, || unsafe { args.Cancel(&mut canceled) });
            let matched = {
                let o = owner.borrow();
                let s = o.instance.as_ref().unwrap().chooser.as_ref().unwrap();
                uri.as_ref()
                    .is_some_and(|v| v.as_ref().is_ok_and(|v| v == &s.target))
            };
            let good = matched
                && a.as_ref().is_some_and(Result::is_ok)
                && b.as_ref().is_some_and(Result::is_ok)
                && !redirected.as_bool()
                && c.as_ref().is_some_and(Result::is_ok)
                && !canceled.as_bool();
            drop(uri);
            drop(a);
            drop(b);
            drop(c);
            drop(charge);
            if good && authorized(owner, true) {
                owner
                    .borrow_mut()
                    .instance
                    .as_mut()
                    .unwrap()
                    .chooser
                    .as_mut()
                    .unwrap()
                    .start = Some(id);
                Ok(false)
            } else {
                fail(owner, false);
                let canceled = unsafe { args.SetCancel(true) };
                canceled.map(|_| true)
            }
        }
        pub(super) fn qualify_navigation(owner: &Rc<RefCell<Owner>>) {
            if authorized(owner, true) {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().chooser.as_mut().unwrap();
                s.facts.navigation_qualified = s.start.is_some() && s.facts.native_starts == 1;
            }
        }
        fn completed(
            owner: &Rc<RefCell<Owner>>,
            args: &ICoreWebView2NavigationCompletedEventArgs,
        ) -> bool {
            let mut id = 0;
            let a = step(owner, || unsafe { args.NavigationId(&mut id) });
            let mut success = BOOL::default();
            let b = step(owner, || unsafe { args.IsSuccess(&mut success) });
            let good = a.as_ref().is_some_and(Result::is_ok)
                && b.as_ref().is_some_and(Result::is_ok)
                && success.as_bool();
            drop(a);
            drop(b);
            let timely = authorized(owner, true);
            let mut o = owner.borrow_mut();
            let s = o.instance.as_mut().unwrap().chooser.as_mut().unwrap();
            s.facts.native_completions = s.facts.native_completions.saturating_add(1);
            let matched = good
                && timely
                && s.facts.navigation_issued
                && s.start == Some(id)
                && s.facts.native_completions == 1;
            s.completed_good = matched;
            matched
        }
        #[windows::core::implement(
            ICoreWebView2CallDevToolsProtocolMethodCompletedHandler,
            Agile = false
        )]
        struct Completion {
            context: Context,
            index: usize,
            _lifetime: Lifetime,
        }
        impl ICoreWebView2CallDevToolsProtocolMethodCompletedHandler_Impl for Completion_Impl {
            fn Invoke(
                &self,
                hresult: windows::core::HRESULT,
                raw: &PCWSTR,
            ) -> windows::core::Result<()> {
                let c = &self.context;
                let active = Active::enter(&c.callbacks);
                if !callback_on_ui(c.ui, &c.callbacks, 7401) {
                    c.inventory.invalid.store(true, Ordering::SeqCst);
                    return Ok(());
                }
                let Some(owner) = c.owner.upgrade() else {
                    c.inventory.invalid.store(true, Ordering::SeqCst);
                    return Ok(());
                };
                let received = Instant::now();
                let invoked = c.inventory.invoke_once(self.index);
                if !c.same(&owner) {
                    c.inventory.invalid.store(true, Ordering::SeqCst);
                    return Ok(());
                }
                if !invoked {
                    fail(&owner, true);
                    return Ok(());
                }
                let cap = [64, 64, 64, 2048, 128, 256, 2048, 128, 2048, 2048, 512][self.index];
                let charge = Text::reserve(&c.inventory, cap);
                let parser_charge = Text::reserve(&c.inventory, 256);
                let text = if hresult.is_ok()
                    && charge.is_some()
                    && parser_charge.is_some()
                    && callback_window(&owner, c)
                {
                    copy_units(*raw, cap)
                } else {
                    Err(())
                };
                let parsed = {
                    let o = owner.borrow();
                    let s = o.instance.as_ref().unwrap().chooser.as_ref().unwrap();
                    text.as_ref()
                        .map_err(|_| ())
                        .and_then(|v| wire::parse_reply(self.index, v, &s.target, s.frame, s.node))
                };
                // Final-tree IDs are compared while the parser workspace is
                // charged, then discarded; the response retains only its ACK.
                let parsed = match parsed {
                    Ok(Reply {
                        kind: ReplyKind::Tree,
                        frame: Some(frame),
                        scalar: 0,
                    }) if self.index == 9 => {
                        let o = owner.borrow();
                        let s = o.instance.as_ref().unwrap().chooser.as_ref().unwrap();
                        if s.frame == Some(frame.id) && s.loader == Some(frame.loader) {
                            Ok(Reply {
                                kind: ReplyKind::Ack,
                                frame: None,
                                scalar: 0,
                            })
                        } else {
                            Err(())
                        }
                    }
                    result => result,
                };
                drop(text);
                drop(charge);
                drop(parser_charge);
                let identity = callback_window(&owner, c);
                let mut effect = false;
                let mut rejected = false;
                {
                    let mut o = owner.borrow_mut();
                    let i = o.instance.as_mut().unwrap();
                    let same = identity && valid_identity(i);
                    let s = i.chooser.as_mut().unwrap();
                    s.facts.completions[self.index] =
                        s.facts.completions[self.index].saturating_add(1);
                    let valid_slot = s.next == self.index && s.response.is_none();
                    if self.index == 10
                        && matches!(
                            parsed,
                            Ok(Reply {
                                kind: ReplyKind::Canceled,
                                frame: None,
                                scalar: 0
                            })
                        )
                        && same
                        && received < s.epoch.end
                        && !c.inventory.invalid.load(Ordering::SeqCst)
                    {
                        effect = s.epoch.effect_at(Instant::now());
                        if effect {
                            s.facts.effect_pre_stop = true;
                            s.facts.trusted_cancel_snapshot = true;
                            s.facts.snapshot_files_zero = true;
                            s.facts.snapshot_value_empty = true;
                            s.facts.snapshot_same_input_document = true;
                        }
                    }
                    if !same
                        || received >= s.epoch.end
                        || Instant::now() >= s.epoch.end
                        || s.epoch.stop.is_some()
                        || !valid_slot
                        || parsed.is_err()
                        || (self.index == 10 && !effect)
                    {
                        s.epoch.invalidate(10);
                        rejected = true;
                    }
                    if valid_slot {
                        s.response = Some(Response {
                            index: self.index,
                            received,
                            valid: parsed,
                        });
                    }
                }
                if rejected {
                    fail(&owner, false);
                }
                if effect {
                    emit(&owner, NativeChooserStage::Effect);
                }
                drop(active);
                Ok(())
            }
        }
        fn issue(owner: &Rc<RefCell<Owner>>, index: usize, core: &Rc<ICoreWebView2>) {
            let context = Context::new(owner);
            let Some(mut charge) = Text::reserve(&context.inventory, 2048) else {
                fail(owner, false);
                return;
            };
            let parameters = {
                let o = owner.borrow();
                let s = o.instance.as_ref().unwrap().chooser.as_ref().unwrap();
                wire::params(
                    index,
                    s.frame,
                    s.unique,
                    if index == 7 { s.root } else { s.node },
                )
            };
            let (wide, bytes) = match parameters {
                Ok(p) => p,
                Err(()) => {
                    drop(charge);
                    fail(owner, false);
                    return;
                }
            };
            charge.shrink(bytes);
            let Some(method_charge) = Text::reserve(&context.inventory, wire::METHODS[index].len())
            else {
                drop(wide);
                drop(charge);
                fail(owner, false);
                return;
            };
            let method: Vec<u16> = wire::METHODS[index].encode_utf16().chain(Some(0)).collect();
            let handler: ICoreWebView2CallDevToolsProtocolMethodCompletedHandler = Completion {
                _lifetime: Lifetime::new(&context.callbacks, &context.inventory),
                context,
                index,
            }
            .into();
            if !authorized(owner, true) {
                drop(handler);
                drop(wide);
                drop(charge);
                return;
            }
            // The final real invocation ticket is created only here, after all
            // fallible setup and actual handler creation, not at preparation.
            {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().chooser.as_mut().unwrap();
                if s.in_flight
                    || s.response.is_some()
                    || s.next != index
                    || s.facts.commands_issued[index] != 0
                {
                    drop(o);
                    drop(handler);
                    fail(owner, false);
                    return;
                }
                s.in_flight = true;
                s.facts.commands_issued[index] = 1;
                if !s.inventory.issue_once(index) {
                    s.epoch.invalidate(10);
                }
                if index == 10 {
                    s.epoch.phase2 = true;
                }
            }
            let result = unsafe {
                core.CallDevToolsProtocolMethod(
                    PCWSTR(method.as_ptr()),
                    PCWSTR(wide.as_ptr()),
                    &handler,
                )
            };
            let ok = result.is_ok();
            drop(result);
            drop(handler);
            drop(wide);
            drop(charge);
            drop(method);
            drop(method_charge);
            {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().chooser.as_mut().unwrap();
                s.facts.commands_returned[index] = 1;
                s.facts.commands_succeeded[index] = u8::from(ok);
            }
            // Native return/handler/parameter Drop can reenter; it never grants
            // another command after target. The submitted obligation remains.
            if !ok {
                fail(owner, false);
            } else if !owner.borrow().instance.as_ref().unwrap().closing {
                let _ = authorized(owner, true);
            }
        }
        fn consume(owner: &Rc<RefCell<Owner>>) {
            let response = {
                owner
                    .borrow_mut()
                    .instance
                    .as_mut()
                    .and_then(|i| i.chooser.as_mut())
                    .and_then(|s| {
                        if s.facts.commands_returned[s.next.min(10)] == 0 {
                            return None;
                        }
                        s.response.take()
                    })
            };
            let Some(response) = response else {
                return;
            };
            let mut bad = false;
            {
                let mut o = owner.borrow_mut();
                let i = o.instance.as_mut().unwrap();
                let s = i.chooser.as_mut().unwrap();
                s.in_flight = false;
                if !s.inventory.consume_once(response.index) {
                    bad = true;
                }
                s.facts.consumed[response.index] =
                    s.facts.consumed[response.index].saturating_add(1);
                let timely = NativeDialogDeadline::disposal_within_close_at(
                    response.received,
                    Instant::now(),
                    s.closing,
                );
                if !timely || response.index != s.next || response.received >= s.epoch.end {
                    bad = true;
                } else {
                    match response.valid {
                        Ok(Reply {
                            kind: ReplyKind::Ack,
                            frame: None,
                            scalar: 0,
                        }) => {}
                        Ok(Reply {
                            kind: ReplyKind::Tree,
                            frame: Some(Frame { id, loader }),
                            scalar: 0,
                        }) => {
                            if response.index == 3 {
                                s.frame = Some(id);
                                s.loader = Some(loader);
                            } else if s.frame != Some(id) || s.loader != Some(loader) {
                                bad = true;
                            }
                        }
                        Ok(Reply {
                            kind: ReplyKind::World,
                            frame: None,
                            scalar: id @ 1..=2147483647,
                        }) => s.world_ack = Some(id),
                        Ok(Reply {
                            kind: ReplyKind::Prepared,
                            frame: None,
                            scalar: 0,
                        }) => s.facts.phase1_prepared = true,
                        Ok(Reply {
                            kind: ReplyKind::Root,
                            frame: None,
                            scalar: id @ 1..=2147483647,
                        }) => {
                            s.root = id;
                            s.facts.root_bound = true;
                        }
                        Ok(Reply {
                            kind: ReplyKind::Node,
                            frame: None,
                            scalar: id @ 1..=2147483647,
                        }) => {
                            s.node = id;
                            s.facts.sole_input_bound = true;
                        }
                        Ok(Reply {
                            kind: ReplyKind::Input,
                            frame: None,
                            scalar: id @ 1..=2147483647,
                        }) => {
                            s.backend = id;
                            s.facts.backend_bound = true;
                        }
                        Ok(Reply {
                            kind: ReplyKind::Canceled,
                            frame: None,
                            scalar: 0,
                        }) => {}
                        _ => bad = true,
                    }
                }
                if bad {
                    s.epoch.invalidate(10);
                } else {
                    s.next += 1;
                }
            }
            if bad {
                fail(owner, false);
            }
        }
        fn navigate(owner: &Rc<RefCell<Owner>>, core: &Rc<ICoreWebView2>) {
            let inventory = {
                let o = owner.borrow();
                Arc::clone(
                    &o.instance
                        .as_ref()
                        .unwrap()
                        .chooser
                        .as_ref()
                        .unwrap()
                        .inventory,
                )
            };
            let charge = Text::reserve(&inventory, 1024);
            if charge.is_none() {
                fail(owner, false);
                return;
            }
            let wide: Vec<u16> = {
                let o = owner.borrow();
                o.instance
                    .as_ref()
                    .unwrap()
                    .chooser
                    .as_ref()
                    .unwrap()
                    .target
                    .encode_utf16()
                    .chain(Some(0))
                    .collect()
            };
            if !authorized(owner, true) {
                drop(wide);
                drop(charge);
                return;
            }
            {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().chooser.as_mut().unwrap();
                if s.facts.navigation_issued {
                    drop(o);
                    fail(owner, false);
                    return;
                }
                s.grant = true;
                s.facts.navigation_issued = true;
            }
            let result = unsafe { core.Navigate(PCWSTR(wide.as_ptr())) };
            let ok = result.is_ok();
            drop(result);
            drop(wide);
            drop(charge);
            {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().chooser.as_mut().unwrap();
                s.facts.navigation_returned = true;
                s.facts.navigation_succeeded = ok;
            }
            if !ok || !authorized(owner, true) {
                fail(owner, false);
            }
        }
        pub(super) fn advance(owner: &Rc<RefCell<Owner>>) {
            if !selected(owner) {
                return;
            }
            consume(owner);
            synchronize(owner);
            let closing = owner.borrow().instance.as_ref().unwrap().closing;
            if closing {
                return;
            }
            if !authorized(owner, true) {
                return;
            }
            let (core, index, waiting) = {
                let o = owner.borrow();
                let s = o.instance.as_ref().unwrap().chooser.as_ref().unwrap();
                (
                    s.core.as_ref().map(Rc::clone),
                    s.next,
                    !s.ready || s.in_flight || s.response.is_some(),
                )
            };
            let Some(core) = core else {
                fail(owner, false);
                return;
            };
            let active = {
                let o = owner.borrow();
                Active::enter(&o.instance.as_ref().unwrap().callbacks)
            };
            if waiting {
                drop(core);
                drop(active);
                return;
            }
            if index == 3 {
                let should_navigate = {
                    let o = owner.borrow();
                    let s = o.instance.as_ref().unwrap().chooser.as_ref().unwrap();
                    !s.facts.navigation_issued && s.initial.is_some()
                };
                if should_navigate {
                    navigate(owner, &core);
                    drop(core);
                    drop(active);
                    return;
                }
                let ready = {
                    let o = owner.borrow();
                    let s = o.instance.as_ref().unwrap().chooser.as_ref().unwrap();
                    s.facts.navigation_returned
                        && s.facts.navigation_succeeded
                        && s.facts.navigation_qualified
                        && s.completed_good
                };
                if !ready {
                    drop(core);
                    drop(active);
                    return;
                }
                if !source(owner, &core, false) {
                    drop(core);
                    drop(active);
                    fail(owner, false);
                    return;
                }
                {
                    let mut o = owner.borrow_mut();
                    let s = o.instance.as_mut().unwrap().chooser.as_mut().unwrap();
                    s.facts.fresh_document = true;
                    s.epoch.arm();
                    s.facts.armed_once = s.epoch.armed && s.epoch.invalid == 0;
                }
            }
            if index == 5 {
                let joined = {
                    let mut o = owner.borrow_mut();
                    let s = o.instance.as_mut().unwrap().chooser.as_mut().unwrap();
                    if let Some(c) = s.candidate.take() {
                        if Some(c.id) != s.world_ack
                            || s.initial == Some(c.id)
                            || s.file == Some(c.id)
                        {
                            s.epoch.invalidate(1);
                            false
                        } else {
                            s.context = Some(c.id);
                            s.unique = Some(c.unique);
                            s.facts.world_joined = true;
                            true
                        }
                    } else {
                        s.facts.world_joined
                    }
                };
                if !joined {
                    let invalid = owner
                        .borrow()
                        .instance
                        .as_ref()
                        .unwrap()
                        .chooser
                        .as_ref()
                        .unwrap()
                        .epoch
                        .invalid
                        != 0;
                    drop(core);
                    drop(active);
                    if invalid {
                        fail(owner, false);
                    }
                    return;
                }
            }
            if index == 10 {
                if !source(owner, &core, false) {
                    drop(core);
                    drop(active);
                    fail(owner, false);
                    return;
                }
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().chooser.as_mut().unwrap();
                s.facts.final_tree_and_source = s.frame.is_some()
                    && s.loader.is_some()
                    && s.facts.phase1_prepared
                    && s.facts.root_bound
                    && s.facts.sole_input_bound
                    && s.facts.backend_bound
                    && s.facts.world_joined;
                s.source_checked = s.facts.final_tree_and_source;
                if !s.source_checked {
                    drop(o);
                    drop(core);
                    drop(active);
                    fail(owner, false);
                    return;
                }
            }
            if index < 11 {
                issue(owner, index, &core);
            }
            drop(core);
            drop(active);
            if !owner.borrow().instance.as_ref().unwrap().closing {
                let _ = authorized(owner, true);
            }
        }
        pub(super) fn removed(i: &Instance) -> bool {
            i.chooser.as_ref().is_none_or(|s| {
                s.receivers.iter().all(|r| r.token.is_none())
                    && s.completed.as_ref().is_none_or(|r| r.token.is_none())
                    && !s.installation.load(Ordering::SeqCst)
            })
        }
        pub(super) fn remove(owner: &Rc<RefCell<Owner>>) {
            if !selected(owner) {
                return;
            }
            loop {
                let action = {
                    let mut o = owner.borrow_mut();
                    let s = o.instance.as_mut().unwrap().chooser.as_mut().unwrap();
                    s.receivers.iter_mut().enumerate().find_map(|(n, r)| {
                        if r.attempted {
                            return None;
                        }
                        let token = r.token?;
                        r.attempted = true;
                        Some((n, Rc::clone(&r.receiver), token))
                    })
                };
                let Some((index, receiver, token)) = action else {
                    break;
                };
                cleanup_window(owner);
                let result = unsafe { receiver.remove_DevToolsProtocolEventReceived(token) };
                let ok = result.is_ok();
                drop(result);
                drop(receiver);
                cleanup_window(owner);
                {
                    let mut o = owner.borrow_mut();
                    let s = o.instance.as_mut().unwrap().chooser.as_mut().unwrap();
                    if ok {
                        s.receivers[index].token = None;
                        s.facts.receiver_removes[index] = 1;
                    }
                }
                if !ok {
                    fail(owner, true);
                }
            }
            let action = {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().chooser.as_mut().unwrap();
                s.completed.as_mut().and_then(|r| {
                    if r.attempted {
                        return None;
                    }
                    let token = r.token?;
                    r.attempted = true;
                    Some((Rc::clone(&r.core), token))
                })
            };
            if let Some((core, token)) = action {
                cleanup_window(owner);
                let result = unsafe { core.remove_NavigationCompleted(token) };
                let ok = result.is_ok();
                drop(result);
                drop(core);
                cleanup_window(owner);
                if ok {
                    let mut o = owner.borrow_mut();
                    let s = o.instance.as_mut().unwrap().chooser.as_mut().unwrap();
                    s.completed.as_mut().unwrap().token = None;
                    s.facts.completed_removed = true;
                    s.facts.completed_same_emitter = true;
                } else {
                    fail(owner, true);
                }
            }
        }
        pub(super) fn take_controls(i: &mut Instance) -> Controls {
            if let Some(s) = i.chooser.as_mut() {
                Controls {
                    _core: s.core.take(),
                    _receivers: std::mem::take(&mut s.receivers),
                    _completed: s.completed.take(),
                }
            } else {
                Controls {
                    _core: None,
                    _receivers: Vec::new(),
                    _completed: None,
                }
            }
        }
        pub(super) fn quiescent(i: &Instance) -> bool {
            i.chooser.as_ref().is_none_or(|s| {
                s.core.is_none()
                    && s.receivers.is_empty()
                    && s.completed.is_none()
                    && s.response.is_none()
                    && s.inventory.handlers.load(Ordering::SeqCst) == 0
                    && s.inventory.drained()
            })
        }
        pub(super) fn publish_final(owner: &Rc<RefCell<Owner>>) {
            if !selected(owner) {
                return;
            }
            let text = {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().chooser.as_mut().unwrap();
                s.facts.observation_epoch_valid =
                    s.epoch.positive() && !s.inventory.invalid.load(Ordering::SeqCst);
                s.facts.obligations_drained = s.inventory.drained();
                s.facts.handlers_drained = s.inventory.handlers.load(Ordering::SeqCst) == 0;
                s.facts.receivers_drained = s.receivers.is_empty() && s.completed.is_none();
                s.unique = None;
                s.frame = None;
                s.loader = None;
                s.candidate = None;
                (
                    std::mem::take(&mut s.target),
                    s._target_charge.take(),
                    s._id_charge.take(),
                )
            };
            drop(text);
            {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().chooser.as_mut().unwrap();
                s.facts.protocol_text_drained = s.inventory.text.load(Ordering::SeqCst) == 0;
            }
            emit(owner, NativeChooserStage::Final);
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn real_invocation_tickets_require_callback_and_consumption_not_handler_drop() {
                let inventory = Inventory::new();
                assert!(inventory.issue_once(0));
                assert!(!inventory.drained());
                // Merely creating/dropping an unrelated handler lifetime cannot
                // consume the outstanding real command obligation.
                let callbacks = Arc::new(Callbacks::default());
                let inventory = Arc::new(inventory);
                drop(Lifetime::new(&callbacks, &inventory));
                assert!(!inventory.drained());
                assert!(inventory.invoke_once(0));
                assert!(!inventory.drained());
                assert!(inventory.consume_once(0));
                assert!(inventory.drained());
                assert!(!inventory.invoke_once(0));
                assert!(inventory.invalid.load(Ordering::SeqCst));
                let unknown = Inventory::new();
                assert!(!unknown.invoke_once(1));
                assert!(!unknown.issue_once(11));
            }

            #[test]
            fn text_charges_overlap_transfer_and_drop_without_exceeding_4096() {
                let inventory = Arc::new(Inventory::new());
                let pinned = Text::reserve(&inventory, 384).unwrap();
                let mut outgoing = Text::reserve(&inventory, 2048).unwrap();
                outgoing.shrink(1710);
                let response = Text::reserve(&inventory, 512).unwrap();
                assert_eq!(inventory.text.load(Ordering::SeqCst), 2606);
                assert!(Text::reserve(&inventory, 1491).is_none());
                drop(outgoing);
                assert_eq!(inventory.text.load(Ordering::SeqCst), 896);
                drop(response);
                drop(pinned);
                assert_eq!(inventory.text.load(Ordering::SeqCst), 0);
            }
        }
    }

    /// Finite, selected-only observation transport. Production navigation stays denied.
    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    mod s4 {
        use super::super::fixture::{
            NativeCapabilityDelivery as Delivery, NativeCapabilityFacts as Facts,
            NativeCapabilityFailure as Failure, NativeCapabilityReceipt,
            NativeCapabilityResult as ResultKind, NativeCapabilitySelector as Selector,
            NativeCapabilityStage as Stage, NativeDialogDeadline,
        };
        use super::super::s4_capability_protocol::{
            self as wire, ControlKind, Event, Method, PagePayload, ProbeKind,
        };
        use super::*;
        use std::cell::Cell;
        use std::time::{Duration, Instant};
        use webview2_com::{
            ContentLoadingEventHandler, DevToolsProtocolEventReceivedEventHandler,
            NavigationStartingEventHandler, ProcessFailedEventHandler,
            WebResourceRequestedEventHandler,
        };
        use windows::Win32::System::Com::{IStream, STREAM_SEEK_SET};
        use windows::Win32::UI::Shell::SHCreateMemStream;
        use windows::core::BOOL;

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum Phase {
            Installing,
            AwaitInitialJoin,
            AwaitParsed,
            AwaitCandidate,
            AwaitMutationTrace,
            ClosingAwaitDispose,
            RemoveProtocols,
            StopClose,
            AwaitReleased,
            Final,
        }
        #[derive(Clone, PartialEq, Eq)]
        struct PermitBinding {
            request: Code0WindowRequest,
            root: wire::Identity,
            context: wire::ContextId,
            unique: wire::Identity,
            loader: wire::Identity,
            end: Instant,
        }
        struct Permit {
            generation: u64,
            revoked: Cell<bool>,
            applied: Cell<u8>,
            binding: Option<PermitBinding>,
        }
        impl Permit {
            #[cfg(test)]
            fn new(generation: u64) -> Self {
                Self {
                    generation,
                    revoked: Cell::new(false),
                    applied: Cell::new(0),
                    binding: None,
                }
            }
        }
        struct Held {
            probe: wire::Probe,
            permit: Rc<Permit>,
            generation: u64,
        }
        fn candidate_matches(
            held: &Held,
            current: &Rc<Permit>,
            generation: u64,
            current_binding: Option<&PermitBinding>,
        ) -> bool {
            Rc::ptr_eq(&held.permit, current)
                && held.generation == generation
                && current.generation == generation
                && held.permit.binding.as_ref() == current_binding
                && held.probe.kind() == ProbeKind::Candidate
                && held.probe.flags() == 0
        }
        fn consume_candidate(
            held: Held,
            current: &Rc<Permit>,
            generation: u64,
            current_binding: Option<&PermitBinding>,
        ) -> Delivery {
            if !candidate_matches(&held, current, generation, current_binding) {
                return Delivery::Failed;
            }
            if held.permit.revoked.get() {
                Delivery::RejectedRevokedPermit
            } else if current.applied.get() == 0 {
                // Actual harmless application, separate from reporting its outcome.
                current.applied.set(1);
                Delivery::Applied
            } else {
                Delivery::Failed
            }
        }
        fn receipt_timely(now: Instant, close: Instant) -> bool {
            now < close + Duration::from_secs(2) && now < close + Duration::from_secs(10)
        }
        struct Inventory {
            issued: AtomicUsize,
            invoked: AtomicUsize,
            consumed: AtomicUsize,
            aborted: AtomicUsize,
            handlers: AtomicUsize,
            raw: AtomicUsize,
            invalid: AtomicBool,
        }
        impl Inventory {
            fn new() -> Self {
                Self {
                    issued: AtomicUsize::new(0),
                    invoked: AtomicUsize::new(0),
                    consumed: AtomicUsize::new(0),
                    aborted: AtomicUsize::new(0),
                    handlers: AtomicUsize::new(0),
                    raw: AtomicUsize::new(0),
                    invalid: AtomicBool::new(false),
                }
            }
            fn once(&self, index: usize, slot: &AtomicUsize, required: usize) -> bool {
                let ok = index < 8
                    && required & (1 << index) != 0
                    && slot.fetch_or(1 << index, Ordering::SeqCst) & (1 << index) == 0;
                if !ok {
                    self.invalid.store(true, Ordering::SeqCst);
                }
                ok
            }
            fn issue(&self, index: usize) -> bool {
                self.once(index, &self.issued, 255)
            }
            fn invoke(&self, index: usize) -> bool {
                self.once(
                    index,
                    &self.invoked,
                    self.issued.load(Ordering::SeqCst) & !self.aborted.load(Ordering::SeqCst),
                )
            }
            fn consume(&self, index: usize) -> bool {
                self.once(index, &self.consumed, self.invoked.load(Ordering::SeqCst))
            }
            fn abort(&self, index: usize) {
                let bit = 1 << index;
                if self.invoked.load(Ordering::SeqCst) & bit == 0 {
                    self.aborted.fetch_or(bit, Ordering::SeqCst);
                }
            }
            fn drained(&self) -> bool {
                let n = self.issued.load(Ordering::SeqCst);
                let invoked = self.invoked.load(Ordering::SeqCst);
                let aborted = self.aborted.load(Ordering::SeqCst);
                n == (invoked | aborted)
                    && invoked == self.consumed.load(Ordering::SeqCst)
                    && invoked & aborted == 0
            }
        }
        struct Lifetime {
            inventory: Arc<Inventory>,
            _aggregate: HandlerLifetime,
        }
        impl Lifetime {
            fn new(context: &Context) -> Self {
                context.inventory.handlers.fetch_add(1, Ordering::SeqCst);
                Self {
                    inventory: Arc::clone(&context.inventory),
                    _aggregate: HandlerLifetime::new(&context.callbacks),
                }
            }
        }
        impl Drop for Lifetime {
            fn drop(&mut self) {
                self.inventory.handlers.fetch_sub(1, Ordering::SeqCst);
            }
        }
        struct RawLease(Arc<Inventory>);
        impl RawLease {
            fn acquire(i: &Arc<Inventory>) -> Result<Self, ()> {
                i.raw
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
                        if n < 2 { Some(n + 1) } else { None }
                    })
                    .map_err(|_| ())?;
                Ok(Self(Arc::clone(i)))
            }
        }
        impl Drop for RawLease {
            fn drop(&mut self) {
                self.0.raw.fetch_sub(1, Ordering::SeqCst);
            }
        }
        struct Receiver {
            receiver: Rc<ICoreWebView2DevToolsProtocolEventReceiver>,
            token: Option<i64>,
            attempted: bool,
        }
        struct Response {
            index: usize,
            received: Instant,
            result: Result<wire::MethodResult, ()>,
        }
        #[derive(Clone, Copy)]
        struct Trace {
            _kind: u8,
            _ordinal: u8,
            _revoked: bool,
        }
        pub(super) struct State {
            request: Code0WindowRequest,
            generation: u64,
            installation: Arc<AtomicBool>,
            inventory: Arc<Inventory>,
            callbacks: Arc<Callbacks>,
            core: Option<Rc<ICoreWebView2>>,
            environment: Option<Rc<ICoreWebView2Environment>>,
            core22: Option<Rc<ICoreWebView2_22>>,
            selector: Selector,
            identity: wire::ProtocolIdentity,
            deadline: NativeDialogDeadline,
            phase: Phase,
            terminal: bool,
            closing: Option<Instant>,
            close_published: bool,
            disposition_published: bool,
            receivers: Vec<Receiver>,
            native_tokens: [Option<i64>; 4],
            native_attempted: [bool; 4],
            filter: bool,
            filter_attempted: bool,
            script_id: Option<String>,
            script_attempted: bool,
            script_submitted: bool,
            script_response: Option<(Instant, Result<String, ()>)>,
            next: usize,
            in_flight: bool,
            response: Option<Response>,
            acknowledged: [bool; 7],
            submitted: [bool; 7],
            setup_ready: bool,
            navigation_issued: bool,
            navigation_returned: bool,
            grant: bool,
            navigation_id: Option<u64>,
            starts: u8,
            loading: u8,
            root: Option<wire::Identity>,
            loader: Option<wire::Identity>,
            root_context: Option<wire::ContextId>,
            unique: Option<wire::Identity>,
            contexts: wire::ContextTable,
            staged: wire::StagedMessages,
            staged_entries: std::collections::VecDeque<(u8, bool)>,
            sequence: wire::ProbeSequence,
            binding_count: u8,
            trace: [Option<Trace>; 64],
            ordinal: u8,
            permit: Option<Rc<Permit>>,
            held: Option<Held>,
            consumer_due: bool,
            controls: u8,
            prepare_sent: bool,
            mutate_sent: bool,
            detach_sent: bool,
            receipt_consumed: bool,
            missing_disposition: bool,
            disposed_at: Option<Instant>,
            witness: bool,
            request_count: u8,
            asset_bytes: usize,
            main_response: bool,
            child_response: bool,
            cleanup_unknown: bool,
            facts: Facts,
        }
        #[derive(Clone, Copy, Default)]
        struct CurrentAuthority {
            identity_matches: bool,
            failed: bool,
            closing: bool,
            destroyed: bool,
            released: bool,
            cleanup_blocked: bool,
            affinity_failed: bool,
            refusal_unknown: bool,
            inventory_invalid: bool,
            cleanup_unknown: bool,
            core_present: bool,
            installation_live: bool,
        }
        impl CurrentAuthority {
            fn usable(self) -> bool {
                self.identity_matches
                    && self.core_present
                    && self.installation_live
                    && !self.destroyed
                    && !self.released
                    && !self.cleanup_blocked
                    && !self.affinity_failed
                    && !self.refusal_unknown
                    && !self.inventory_invalid
                    && !self.cleanup_unknown
            }
        }
        fn active_phase(phase: Phase) -> bool {
            matches!(
                phase,
                Phase::Installing
                    | Phase::AwaitInitialJoin
                    | Phase::AwaitParsed
                    | Phase::AwaitCandidate
                    | Phase::AwaitMutationTrace
            )
        }
        impl State {
            fn active_at(&self, current: CurrentAuthority, expected: Phase, now: Instant) -> bool {
                current.usable()
                    && !current.failed
                    && !current.closing
                    && !self.terminal
                    && self.closing.is_none()
                    && self.phase == expected
                    && active_phase(expected)
                    && matches!(
                        self.facts.failure,
                        None | Some(Failure::CapabilityApprovalMissing)
                    )
                    && self.deadline.before_original_at(now)
            }
            fn live_at(&self, current: CurrentAuthority, expected: Phase, now: Instant) -> bool {
                self.active_at(current, expected, now)
                    && self.permit.as_ref().is_none_or(|p| !p.revoked.get())
            }
        }
        fn terminal_failure(s: &mut State, failure: Failure, unknown: bool) {
            if s.facts.failure.is_none()
                || s.facts.failure == Some(Failure::CapabilityApprovalMissing)
            {
                s.facts.failure = Some(failure);
            }
            s.facts.result = ResultKind::NotEstablished;
            s.facts.failed_close_no_activation = true;
            s.terminal = true;
            s.grant = false;
            revoke(s);
            s.cleanup_unknown |= unknown;
            if active_phase(s.phase) {
                s.phase = Phase::ClosingAwaitDispose;
            }
            if !s.consumer_due {
                s.held = None;
            }
        }
        fn current_binding(s: &State) -> Option<PermitBinding> {
            s.root
                .clone()
                .zip(s.root_context)
                .zip(s.unique.clone())
                .zip(s.loader.clone())
                .map(|(((root, context), unique), loader)| PermitBinding {
                    request: s.request.clone(),
                    root,
                    context,
                    unique,
                    loader,
                    end: s.deadline.original_end(),
                })
        }
        enum DeliveryWindow {
            Attempt(Instant),
            Closing(Instant),
            Denied(Failure),
        }
        struct Handoff {
            held: Held,
            permit: Rc<Permit>,
            generation: u64,
            binding: Option<PermitBinding>,
            window: DeliveryWindow,
            callbacks: Arc<Callbacks>,
            inventory: Arc<Inventory>,
            installation: Arc<AtomicBool>,
        }
        struct Consumed {
            outcome: Delivery,
            applied: u8,
            failure: Option<Failure>,
        }
        fn take_handoff_at(
            s: &mut State,
            current: CurrentAuthority,
            now: Instant,
        ) -> Result<Option<Handoff>, ()> {
            if !s.consumer_due {
                return Ok(None);
            }
            if s.facts.candidate_delivery_attempts != 0 {
                return Err(());
            }
            let attempt = s.active_at(current, Phase::AwaitMutationTrace, now);
            let closing = current.usable()
                && s.phase == Phase::ClosingAwaitDispose
                && s.selector == Selector::DisposeMissing
                && s.missing_disposition
                && s.facts.receipt_deadline_observed
                && s.closing.is_some_and(|at| {
                    now >= at + Duration::from_secs(2) && now < at + Duration::from_secs(10)
                })
                && s.permit.as_ref().is_some_and(|p| p.revoked.get());
            let window = if attempt {
                DeliveryWindow::Attempt(s.deadline.original_end())
            } else if closing {
                DeliveryWindow::Closing(s.closing.unwrap())
            } else {
                DeliveryWindow::Denied(
                    if s.selector == Selector::DisposeMissing
                        && s.missing_disposition
                        && s.facts.receipt_deadline_observed
                        && s.closing
                            .is_some_and(|at| now >= at + Duration::from_secs(10))
                    {
                        Failure::NativeCleanup
                    } else if now >= s.deadline.original_end() && s.closing.is_none() {
                        Failure::StimulusNotObserved
                    } else {
                        Failure::Protocol
                    },
                )
            };
            let held = s.held.take().ok_or(())?;
            let permit = s
                .permit
                .as_ref()
                .map(Rc::clone)
                .unwrap_or_else(|| Rc::clone(&held.permit));
            let binding = current_binding(s);
            s.consumer_due = false;
            Ok(Some(Handoff {
                held,
                permit,
                generation: s.generation,
                binding,
                window,
                callbacks: Arc::clone(&s.callbacks),
                inventory: Arc::clone(&s.inventory),
                installation: Arc::clone(&s.installation),
            }))
        }
        impl Handoff {
            fn consume_at(self, now: Instant) -> Consumed {
                let invalid_identity = self.binding.is_none()
                    || !candidate_matches(
                        &self.held,
                        &self.permit,
                        self.generation,
                        self.binding.as_ref(),
                    );
                let atomic_invalid = self.installation.load(Ordering::SeqCst)
                    || self.inventory.invalid.load(Ordering::SeqCst)
                    || self.callbacks.affinity_failed.load(Ordering::SeqCst);
                let failure = if invalid_identity || atomic_invalid {
                    Some(Failure::Protocol)
                } else {
                    match self.window {
                        DeliveryWindow::Attempt(end) if now < end => None,
                        DeliveryWindow::Attempt(_) => Some(Failure::StimulusNotObserved),
                        DeliveryWindow::Closing(at)
                            if self.permit.revoked.get()
                                && now >= at + Duration::from_secs(2)
                                && now < at + Duration::from_secs(10) =>
                        {
                            None
                        }
                        DeliveryWindow::Closing(_) => Some(Failure::NativeCleanup),
                        DeliveryWindow::Denied(failure) => Some(failure),
                    }
                };
                if let Some(failure) = failure {
                    // This is an actual invocation with the original decoded item.
                    // Failed current authority forbids its application, even when
                    // the earlier snapshot was timely and the clock crossed later.
                    self.held.permit.revoked.set(true);
                    self.permit.revoked.set(true);
                    return Consumed {
                        outcome: Delivery::Failed,
                        applied: self.permit.applied.get(),
                        failure: Some(failure),
                    };
                }
                let outcome = consume_candidate(
                    self.held,
                    &self.permit,
                    self.generation,
                    self.binding.as_ref(),
                );
                Consumed {
                    outcome,
                    applied: self.permit.applied.get(),
                    failure: (outcome == Delivery::Failed).then_some(Failure::Protocol),
                }
            }
        }
        fn record_consumption(s: &mut State, consumed: &Consumed) {
            s.facts.candidate_delivery_attempts = 1;
            s.facts.candidate_applied = consumed.applied;
            s.facts.candidate_delivery_outcome = consumed.outcome;
            if let Some(failure) = consumed.failure {
                if failure == Failure::NativeCleanup
                    && s.selector == Selector::DisposeMissing
                    && s.missing_disposition
                    && s.facts.receipt_deadline_observed
                    && s.facts.failure == Some(Failure::DetachReceiptMissing)
                {
                    // The expected receipt2 disposition remains causal evidence,
                    // but missing close10 is now the current terminal failure.
                    s.facts.failure = Some(Failure::NativeCleanup);
                }
                terminal_failure(s, failure, false);
            }
        }
        fn commit_join_at(
            s: &mut State,
            current: CurrentAuthority,
            expected: Phase,
            now: Instant,
        ) -> bool {
            if !s.active_at(current, expected, now) {
                return false;
            }
            if !(s.navigation_returned
                && s.starts == 1
                && s.loading == 1
                && s.main_response
                && s.navigation_id.is_some()
                && s.root.is_some()
                && s.loader.is_some()
                && s.root_context.is_some()
                && s.unique.is_some())
            {
                return false;
            }
            if s.permit.is_none() {
                if !s.live_at(current, expected, now) {
                    return false;
                }
                let Some(binding) = current_binding(s) else {
                    return false;
                };
                s.permit = Some(Rc::new(Permit {
                    generation: s.generation,
                    revoked: Cell::new(false),
                    applied: Cell::new(0),
                    binding: Some(binding),
                }));
                s.phase = Phase::AwaitParsed;
            }
            s.facts.initial_native_join = true;
            s.facts.root_metadata_joined = true;
            while let Some(message) = s.staged.pop_front() {
                let Some(entry) = s.staged_entries.pop_front() else {
                    return false;
                };
                if !apply_binding(s, message, entry) {
                    return false;
                }
            }
            true
        }
        fn reserve_navigation_at(s: &mut State, current: CurrentAuthority, now: Instant) -> bool {
            if !s.live_at(current, Phase::Installing, now)
                || s.navigation_issued
                || !s.facts.script_ack
                || s.next != 4
                || s.root.is_none()
            {
                return false;
            }
            s.navigation_issued = true;
            s.grant = true;
            s.phase = Phase::AwaitInitialJoin;
            true
        }
        fn reserve_control_at(
            s: &mut State,
            current: CurrentAuthority,
            kind: ControlKind,
            now: Instant,
        ) -> Option<wire::BoundedText<256>> {
            let allowed = match kind {
                ControlKind::Prepare => {
                    s.live_at(current, Phase::AwaitParsed, now) && !s.prepare_sent
                }
                ControlKind::Mutate => {
                    s.live_at(current, Phase::AwaitCandidate, now)
                        && !s.mutate_sent
                        && s.facts.candidate_held
                }
                ControlKind::Detach => {
                    current.usable()
                        && s.phase == Phase::ClosingAwaitDispose
                        && !s.detach_sent
                        && s.closing
                            .is_some_and(|at| now >= at && now < at + Duration::from_secs(10))
                }
            };
            let limit = if s.selector == Selector::DisposeMissing {
                2
            } else {
                3
            };
            if !allowed || s.controls >= limit {
                return None;
            }
            let encoded = wire::encode_control(&s.identity, kind).ok()?;
            s.controls += 1;
            match kind {
                ControlKind::Prepare => {
                    s.prepare_sent = true;
                    s.phase = Phase::AwaitCandidate;
                }
                ControlKind::Mutate => {
                    s.mutate_sent = true;
                    s.phase = Phase::AwaitMutationTrace;
                }
                ControlKind::Detach => s.detach_sent = true,
            };
            Some(encoded)
        }
        fn observe_context_created_at(
            s: &mut State,
            context: wire::ExecutionContext,
            current: CurrentAuthority,
            now: Instant,
        ) -> bool {
            let eligibility = s.root.as_ref().map(|r| context.root_eligibility(r));
            let id = context.id;
            let unique = context.unique_id.clone();
            if s.contexts.insert(context).is_err() {
                return false;
            }
            if eligibility == Some(wire::RootEligibility::EligibleMetadata) && s.navigation_issued {
                if s.root_context.is_some() || !s.live_at(current, Phase::AwaitInitialJoin, now) {
                    return false;
                }
                s.root_context = Some(id);
                s.unique = unique;
            }
            true
        }
        fn observe_contexts_cleared(s: &mut State) -> bool {
            if s.root_context.is_some()
                || s.unique.is_some()
                || s.permit.is_some()
                || s.terminal
                || s.closing.is_some()
                || !matches!(s.phase, Phase::Installing | Phase::AwaitInitialJoin)
            {
                terminal_failure(s, Failure::ContextChanged, false);
                false
            } else {
                true
            }
        }

        struct Context {
            owner: Weak<RefCell<Owner>>,
            request: Code0WindowRequest,
            generation: u64,
            installation: Arc<AtomicBool>,
            inventory: Arc<Inventory>,
            callbacks: Arc<Callbacks>,
            ui: ThreadId,
        }
        impl Context {
            fn new(owner: &Rc<RefCell<Owner>>) -> Self {
                let o = owner.borrow();
                let i = o.instance.as_ref().unwrap();
                let s = i.capability.as_ref().unwrap();
                Self {
                    owner: Rc::downgrade(owner),
                    request: i.request.clone(),
                    generation: s.generation,
                    installation: Arc::clone(&s.installation),
                    inventory: Arc::clone(&s.inventory),
                    callbacks: Arc::clone(&i.callbacks),
                    ui: o.ui_thread,
                }
            }
            fn same(&self, owner: &Rc<RefCell<Owner>>) -> bool {
                let o = owner.borrow();
                !o.unmatched
                    && o.instance.as_ref().is_some_and(|i| {
                        i.request == self.request
                            && s2::finite_generation(i) == self.generation
                            && Arc::ptr_eq(&i.callbacks, &self.callbacks)
                            && i.capability.as_ref().is_some_and(|s| {
                                Arc::ptr_eq(&s.callbacks, &i.callbacks)
                                    && s.generation == self.generation
                                    && Arc::ptr_eq(&s.inventory, &self.inventory)
                                    && Arc::ptr_eq(&s.installation, &self.installation)
                            })
                    })
            }
            fn authority(&self, owner: &Rc<RefCell<Owner>>) -> CurrentAuthority {
                let same = self.same(owner);
                let o = owner.borrow();
                let Some(i) = o.instance.as_ref() else {
                    return CurrentAuthority::default();
                };
                let Some(s) = i.capability.as_ref() else {
                    return CurrentAuthority::default();
                };
                CurrentAuthority {
                    identity_matches: same
                        && s.request == i.request
                        && Arc::ptr_eq(&s.installation, &i.refusal_registry.unknown),
                    failed: i.failed,
                    closing: i.closing,
                    destroyed: i.destroyed,
                    released: i.released,
                    cleanup_blocked: i.cleanup_blocked,
                    affinity_failed: i.callbacks.affinity_failed.load(Ordering::SeqCst),
                    refusal_unknown: i.refusal_registry.unknown(),
                    inventory_invalid: s.inventory.invalid.load(Ordering::SeqCst),
                    cleanup_unknown: s.cleanup_unknown,
                    core_present: s.core.is_some(),
                    installation_live: i.refusal_registry.complete(),
                }
            }
            fn live_at(&self, owner: &Rc<RefCell<Owner>>, expected: Phase, now: Instant) -> bool {
                let current = self.authority(owner);
                let o = owner.borrow();
                o.instance
                    .as_ref()
                    .and_then(|i| i.capability.as_ref())
                    .is_some_and(|s| s.live_at(current, expected, now))
            }
            fn window(&self, owner: &Rc<RefCell<Owner>>) -> bool {
                self.same(owner) && {
                    let o = owner.borrow();
                    o.instance.as_ref().is_some_and(|i| {
                        !i.released
                            && !i.destroyed
                            && !i.cleanup_blocked
                            && !i.refusal_registry.unknown()
                            && !i.callbacks.affinity_failed.load(Ordering::SeqCst)
                            && i.capability.as_ref().is_some_and(|s| {
                                s.core.is_some()
                                    && if let Some(close) = s.closing {
                                        Instant::now() < close + Duration::from_secs(10)
                                    } else {
                                        s.deadline.before_original_at(Instant::now())
                                    }
                            })
                    })
                }
            }
            fn sender(&self, owner: &Rc<RefCell<Owner>>, sender: Option<&ICoreWebView2>) -> bool {
                if !self.window(owner) {
                    return false;
                }
                let core = {
                    let o = owner.borrow();
                    o.instance
                        .as_ref()
                        .and_then(|i| i.capability.as_ref())
                        .and_then(|s| s.core.as_ref())
                        .map(Rc::clone)
                };
                let good = core.as_ref().zip(sender).is_some_and(|(core, sender)| {
                    let expected = core.cast::<IUnknown>();
                    if !self.window(owner) {
                        drop(expected);
                        return false;
                    }
                    let actual = sender.cast::<IUnknown>();
                    let good = self.window(owner)
                        && expected
                            .as_ref()
                            .ok()
                            .zip(actual.as_ref().ok())
                            .is_some_and(|(a, b)| a.as_raw() == b.as_raw());
                    drop(actual);
                    drop(expected);
                    good && self.window(owner)
                });
                drop(core);
                good && self.window(owner)
            }
            fn entry(&self, owner: &Rc<RefCell<Owner>>, kind: u8) -> Option<(u8, bool)> {
                if !self.window(owner) {
                    return None;
                }
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut()?.capability.as_mut()?;
                if s.ordinal >= 64 {
                    s.inventory.invalid.store(true, Ordering::SeqCst);
                    return None;
                }
                s.ordinal += 1;
                let revoked = s.permit.as_ref().is_some_and(|p| p.revoked.get());
                s.trace[usize::from(s.ordinal - 1)] = Some(Trace {
                    _kind: kind,
                    _ordinal: s.ordinal,
                    _revoked: revoked,
                });
                Some((s.ordinal, revoked))
            }
            fn finish(&self, owner: &Rc<RefCell<Owner>>, good: bool, failure: Failure) {
                if !self.same(owner) {
                    self.inventory.invalid.store(true, Ordering::SeqCst);
                } else if !good || !self.window(owner) {
                    reject(owner, failure, false);
                }
            }
        }
        fn copy_units(raw: PCWSTR, cap: usize) -> Result<String, ()> {
            if raw.is_null() {
                return Err(());
            }
            for n in 0..=wire::MAX_RAW_UTF16 {
                if unsafe { *raw.0.add(n) } == 0 {
                    let units = unsafe { std::slice::from_raw_parts(raw.0, n) };
                    let mut bytes = 0;
                    for c in char::decode_utf16(units.iter().copied()) {
                        bytes += c.map_err(|_| ())?.len_utf8();
                        if bytes > cap || bytes > wire::MAX_RAW_UTF8 {
                            return Err(());
                        }
                    }
                    return String::from_utf16(units).map_err(|_| ());
                }
            }
            Err(())
        }
        fn allocated(
            context: &Context,
            owner: &Rc<RefCell<Owner>>,
            cap: usize,
            get: impl FnOnce(*mut PWSTR) -> windows::core::Result<()>,
        ) -> Result<String, ()> {
            if !context.window(owner) {
                return Err(());
            }
            let mut p = PWSTR::null();
            let hr = get(&mut p);
            let allocation = CoTaskMemPWSTR::from(p);
            let out = if hr.is_ok() && context.window(owner) {
                copy_units(PCWSTR(p.0), cap)
            } else {
                Err(())
            };
            drop(hr);
            drop(allocation);
            if context.window(owner) {
                out
            } else {
                drop(out);
                Err(())
            }
        }
        fn unit(
            c: &Context,
            owner: &Rc<RefCell<Owner>>,
            action: impl FnOnce() -> windows::core::Result<()>,
        ) -> bool {
            if !c.window(owner) {
                return false;
            }
            let result = action();
            let good = result.is_ok() && c.window(owner);
            drop(result);
            good && c.window(owner)
        }
        fn source(owner: &Rc<RefCell<Owner>>, core: &ICoreWebView2, blank: bool) -> bool {
            let c = Context::new(owner);
            let Ok(_raw) = RawLease::acquire(&c.inventory) else {
                return false;
            };
            let source = allocated(&c, owner, 2048, |p| unsafe { core.Source(p) });
            let good = source
                .as_ref()
                .is_ok_and(|s| s == if blank { "about:blank" } else { wire::MAIN_URL });
            drop(source);
            drop(_raw);
            good && c.window(owner)
        }
        pub(super) fn selected(owner: &Rc<RefCell<Owner>>) -> bool {
            owner
                .borrow()
                .instance
                .as_ref()
                .is_some_and(|i| i.capability.is_some())
        }
        fn revoke(s: &State) {
            if let Some(p) = &s.permit {
                p.revoked.set(true);
            }
        }
        pub(super) fn mark_close(i: &mut Instance) {
            if i.closing
                && let Some(s) = i.capability.as_mut()
            {
                s.terminal = true;
                s.grant = false;
                revoke(s);
                if s.closing.is_none() {
                    s.closing = Some(Instant::now());
                    if !matches!(
                        s.phase,
                        Phase::RemoveProtocols
                            | Phase::StopClose
                            | Phase::AwaitReleased
                            | Phase::Final
                    ) {
                        s.phase = Phase::ClosingAwaitDispose;
                    }
                }
            }
        }
        fn reject(owner: &Rc<RefCell<Owner>>, failure: Failure, unknown: bool) {
            let mut o = owner.borrow_mut();
            if let Some(i) = o.instance.as_mut()
                && let Some(s) = i.capability.as_mut()
            {
                terminal_failure(s, failure, unknown);
                i.failed = true;
                i.closing = true;
                if unknown {
                    i.cleanup_blocked = true;
                }
                mark_close(i);
            }
        }
        pub(super) fn synchronize(owner: &Rc<RefCell<Owner>>) {
            let note = {
                let mut o = owner.borrow_mut();
                o.instance.as_mut().and_then(|i| {
                    mark_close(i);
                    let s = i.capability.as_mut()?;
                    if s.close_published {
                        return None;
                    }
                    let at = s.closing?;
                    s.close_published = true;
                    Some((i.observer.clone(), at))
                })
            };
            if let Some((Some(f), at)) = note {
                f(NativeSmokeObservation::CapabilityClosing { at });
            }
        }
        fn emit(owner: &Rc<RefCell<Owner>>, stage: Stage) {
            let note = {
                let o = owner.borrow();
                o.instance.as_ref().and_then(|i| {
                    i.capability.as_ref().map(|s| {
                        (
                            i.observer.clone(),
                            NativeCapabilityReceipt {
                                request: i.request.clone(),
                                installation: Arc::clone(&s.installation),
                                stage,
                                facts: s.facts.clone(),
                            },
                        )
                    })
                })
            };
            if let Some((Some(f), receipt)) = note {
                f(NativeSmokeObservation::Capability(receipt));
            }
        }
        fn close_selected(owner: &Rc<RefCell<Owner>>) {
            let mut o = owner.borrow_mut();
            if let Some(i) = o.instance.as_mut() {
                i.closing = true;
                mark_close(i);
            }
        }
        pub(super) fn begin(
            owner: &Rc<RefCell<Owner>>,
            request: &Code0WindowRequest,
            selector: Selector,
            deadline: NativeDialogDeadline,
        ) -> Result<(), Code0SessionFailure> {
            super::require_native_refusal_install(owner, request)?;
            let (core, callbacks) = {
                let o = owner.borrow();
                let i = o
                    .instance
                    .as_ref()
                    .ok_or(Code0SessionFailure::PolicyRejected)?;
                if i.capability.is_some()
                    || i.chooser.is_some()
                    || !deadline.before_original_at(Instant::now())
                {
                    return Err(Code0SessionFailure::PolicyRejected);
                }
                (
                    s2::chooser_core(i).ok_or(Code0SessionFailure::PolicyRejected)?,
                    Arc::clone(&i.callbacks),
                )
            };
            let identity = wire::ProtocolIdentity::new(
                &request.nonce,
                &request.document_id,
                wire::Selector::parse(selector.name()).unwrap(),
            )
            .map_err(|_| Code0SessionFailure::PolicyRejected)?;
            let active = Active::enter(&callbacks);
            let original = {
                let mut o = owner.borrow_mut();
                matching(&mut o, request)
                    .unwrap()
                    .environment
                    .take()
                    .unwrap()
            };
            let environment = Rc::new(original.clone());
            {
                let mut o = owner.borrow_mut();
                let i = matching(&mut o, request).unwrap();
                i.environment = Some(original);
                i.capability = Some(State {
                    request: request.clone(),
                    generation: s2::finite_generation(i),
                    installation: Arc::clone(&i.refusal_registry.unknown),
                    inventory: Arc::new(Inventory::new()),
                    callbacks: Arc::clone(&i.callbacks),
                    core: Some(Rc::clone(&core)),
                    environment: Some(environment),
                    core22: None,
                    selector,
                    identity,
                    deadline,
                    phase: Phase::Installing,
                    terminal: false,
                    closing: None,
                    close_published: false,
                    disposition_published: false,
                    receivers: Vec::with_capacity(7),
                    native_tokens: [None; 4],
                    native_attempted: [false; 4],
                    filter: false,
                    filter_attempted: false,
                    script_id: None,
                    script_attempted: false,
                    script_submitted: false,
                    script_response: None,
                    next: 0,
                    in_flight: false,
                    response: None,
                    acknowledged: [false; 7],
                    submitted: [false; 7],
                    setup_ready: false,
                    navigation_issued: false,
                    navigation_returned: false,
                    grant: false,
                    navigation_id: None,
                    starts: 0,
                    loading: 0,
                    root: None,
                    loader: None,
                    root_context: None,
                    unique: None,
                    contexts: wire::ContextTable::new(),
                    staged: wire::StagedMessages::new(),
                    staged_entries: std::collections::VecDeque::with_capacity(10),
                    sequence: wire::ProbeSequence::new(),
                    binding_count: 0,
                    trace: [None; 64],
                    ordinal: 0,
                    permit: None,
                    held: None,
                    consumer_due: false,
                    controls: 0,
                    prepare_sent: false,
                    mutate_sent: false,
                    detach_sent: false,
                    receipt_consumed: false,
                    missing_disposition: false,
                    disposed_at: None,
                    witness: false,
                    request_count: 0,
                    asset_bytes: 0,
                    main_response: false,
                    child_response: false,
                    cleanup_unknown: false,
                    facts: Facts::new(selector),
                });
            }
            let result = source(owner, &core, true) && register(owner, &core);
            drop(core);
            drop(active);
            if !result || !Context::new(owner).live_at(owner, Phase::Installing, Instant::now()) {
                reject(owner, Failure::Setup, false);
                return Err(Code0SessionFailure::PolicyRejected);
            }
            owner
                .borrow_mut()
                .instance
                .as_mut()
                .unwrap()
                .capability
                .as_mut()
                .unwrap()
                .setup_ready = true;
            emit(owner, Stage::Started);
            Ok(())
        }
        fn register(owner: &Rc<RefCell<Owner>>, core: &Rc<ICoreWebView2>) -> bool {
            let c = Context::new(owner);
            if !c.live_at(owner, Phase::Installing, Instant::now()) {
                return false;
            }
            let settings = unsafe { core.Settings() };
            let ok = if let Ok(settings) = settings.as_ref() {
                let a = unit(&c, owner, || unsafe {
                    settings.SetIsWebMessageEnabled(true)
                });
                let mut actual = BOOL::default();
                let b = unit(&c, owner, || unsafe {
                    settings.IsWebMessageEnabled(&mut actual)
                });
                a && b && actual.as_bool() && c.live_at(owner, Phase::Installing, Instant::now())
            } else {
                false
            };
            drop(settings);
            if !ok || !c.live_at(owner, Phase::Installing, Instant::now()) {
                return false;
            }
            let core22 = core.cast::<ICoreWebView2_22>();
            let core22 = match core22 {
                Ok(v) => Rc::new(v),
                Err(_) => return false,
            };
            if !c.live_at(owner, Phase::Installing, Instant::now()) {
                drop(core22);
                return false;
            }
            owner
                .borrow_mut()
                .instance
                .as_mut()
                .unwrap()
                .capability
                .as_mut()
                .unwrap()
                .core22 = Some(Rc::clone(&core22));
            macro_rules! native_handler {
                ($index:expr,$handler:ident,$add:ident,$action:expr,$fallback:expr) => {{
                    let c = Context::new(owner);
                    let lifetime = Lifetime::new(&c);
                    let handler = $handler::create(Box::new(move |sender, args| {
                        let _lifetime = &lifetime;
                        let active = Active::enter(&c.callbacks);
                        if !callback_on_ui(c.ui, &c.callbacks, 8000 + $index) {
                            c.inventory.invalid.store(true, Ordering::SeqCst);
                            drop(args);
                            drop(sender);
                            drop(active);
                            return Ok(());
                        }
                        if let Some(owner) = c.owner.upgrade() {
                            let entry = c.entry(&owner, 10 + $index);
                            let good = c.sender(&owner, sender.as_ref())
                                && entry.is_some()
                                && args
                                    .as_ref()
                                    .is_some_and(|a| ($action)(&owner, a, entry.unwrap()));
                            if !good
                                && let Some(args) = args.as_ref()
                                && !($fallback)(&owner, args)
                            {
                                reject(&owner, Failure::NativeCleanup, true);
                            }
                            drop(args);
                            drop(sender);
                            c.finish(&owner, good, Failure::Protocol);
                        } else {
                            c.inventory.invalid.store(true, Ordering::SeqCst);
                            drop(args);
                            drop(sender);
                        }
                        drop(active);
                        Ok(())
                    }));
                    let mut token = 0;
                    let result = unsafe { core.$add(&handler, &mut token) };
                    let good = result.is_ok();
                    if good {
                        owner
                            .borrow_mut()
                            .instance
                            .as_mut()
                            .unwrap()
                            .capability
                            .as_mut()
                            .unwrap()
                            .native_tokens[$index] = Some(token);
                    }
                    drop(result);
                    drop(handler);
                    if !good
                        || !Context::new(owner).live_at(owner, Phase::Installing, Instant::now())
                    {
                        return false;
                    }
                }};
            }
            native_handler!(
                0,
                NavigationStartingEventHandler,
                add_NavigationStarting,
                navigation,
                deny_navigation
            );
            native_handler!(
                1,
                ContentLoadingEventHandler,
                add_ContentLoading,
                content_loading,
                deny_content
            );
            native_handler!(
                2,
                ProcessFailedEventHandler,
                add_ProcessFailed,
                process_failed,
                deny_process
            );
            native_handler!(
                3,
                WebResourceRequestedEventHandler,
                add_WebResourceRequested,
                resource,
                deny_resource
            );
            let filter = unsafe {
                core22.AddWebResourceRequestedFilterWithRequestSourceKinds(
                    windows::core::w!("*"),
                    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                    COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
                )
            };
            if filter.is_ok() {
                owner
                    .borrow_mut()
                    .instance
                    .as_mut()
                    .unwrap()
                    .capability
                    .as_mut()
                    .unwrap()
                    .filter = true;
            }
            let ok = filter.is_ok();
            drop(filter);
            drop(core22);
            if !ok || !c.live_at(owner, Phase::Installing, Instant::now()) {
                return false;
            }
            for (index, event) in Event::ALL.into_iter().enumerate() {
                let name: Vec<u16> = event.name().encode_utf16().chain(Some(0)).collect();
                let receiver =
                    unsafe { core.GetDevToolsProtocolEventReceiver(PCWSTR(name.as_ptr())) };
                drop(name);
                let receiver = match receiver {
                    Ok(v) => Rc::new(v),
                    Err(_) => return false,
                };
                if !c.live_at(owner, Phase::Installing, Instant::now()) {
                    drop(receiver);
                    return false;
                }
                owner
                    .borrow_mut()
                    .instance
                    .as_mut()
                    .unwrap()
                    .capability
                    .as_mut()
                    .unwrap()
                    .receivers
                    .push(Receiver {
                        receiver: Rc::clone(&receiver),
                        token: None,
                        attempted: false,
                    });
                let context = Context::new(owner);
                let lifetime = Lifetime::new(&context);
                let handler = DevToolsProtocolEventReceivedEventHandler::create(Box::new(
                    move |sender, args| {
                        let _lifetime = &lifetime;
                        let active = Active::enter(&context.callbacks);
                        if !callback_on_ui(context.ui, &context.callbacks, 8020) {
                            context.inventory.invalid.store(true, Ordering::SeqCst);
                            drop(args);
                            drop(sender);
                            drop(active);
                            return Ok(());
                        }
                        if let Some(owner) = context.owner.upgrade() {
                            // Entry facts precede sender QueryInterface, Args2 and every native getter.
                            let entry = context.entry(&owner, index as u8);
                            let within_binding_bound = if event == Event::BindingCalled {
                                let mut o = owner.borrow_mut();
                                let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                                s.binding_count = s.binding_count.saturating_add(1);
                                s.binding_count <= 10
                            } else {
                                true
                            };
                            if !within_binding_bound {
                                reject(&owner, Failure::BoundExceeded, false);
                            }
                            let valid =
                                within_binding_bound && context.sender(&owner, sender.as_ref());
                            let good = valid
                                && entry.is_some()
                                && args.as_ref().is_some_and(|args| {
                                    receive(&owner, &context, event, args, entry.unwrap())
                                });
                            drop(args);
                            drop(sender);
                            context.finish(&owner, good, Failure::Metadata);
                        } else {
                            context.inventory.invalid.store(true, Ordering::SeqCst);
                            drop(args);
                            drop(sender);
                        }
                        drop(active);
                        Ok(())
                    },
                ));
                let mut token = 0;
                let result =
                    unsafe { receiver.add_DevToolsProtocolEventReceived(&handler, &mut token) };
                let ok = result.is_ok();
                if ok {
                    let mut o = owner.borrow_mut();
                    let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                    s.receivers[index].token = Some(token);
                    s.facts.receiver_adds += 1;
                }
                drop(result);
                drop(handler);
                drop(receiver);
                if !ok || !c.live_at(owner, Phase::Installing, Instant::now()) {
                    return false;
                }
            }
            true
        }
        fn deny_navigation(
            _owner: &Rc<RefCell<Owner>>,
            args: &ICoreWebView2NavigationStartingEventArgs,
        ) -> bool {
            unsafe { args.SetCancel(true) }.is_ok()
        }
        fn deny_content(
            _owner: &Rc<RefCell<Owner>>,
            _args: &ICoreWebView2ContentLoadingEventArgs,
        ) -> bool {
            true
        }
        fn deny_process(
            _owner: &Rc<RefCell<Owner>>,
            _args: &ICoreWebView2ProcessFailedEventArgs,
        ) -> bool {
            true
        }
        fn deny_resource(
            owner: &Rc<RefCell<Owner>>,
            args: &ICoreWebView2WebResourceRequestedEventArgs,
        ) -> bool {
            // Even invalid metadata/overflow gets an actual empty fixed403. This
            // restrictive action grants no page or protocol authority.
            let c = Context::new(owner);
            if !c.same(owner) {
                return false;
            }
            let environment = {
                let o = owner.borrow();
                o.instance
                    .as_ref()
                    .and_then(|i| i.capability.as_ref())
                    .and_then(|s| s.environment.as_ref())
                    .map(Rc::clone)
            };
            let Some(environment) = environment else {
                return false;
            };
            let stream = unsafe { SHCreateMemStream(Some(&[])) };
            let response = stream.as_ref().and_then(|stream| {
                unsafe {
                    environment.CreateWebResourceResponse(
                        stream,
                        403,
                        windows::core::w!("Forbidden"),
                        windows::core::w!("Content-Length: 0\r\nCache-Control: no-store\r\n"),
                    )
                }
                .ok()
            });
            let applied = c.same(owner)
                && response
                    .as_ref()
                    .is_some_and(|response| unsafe { args.SetResponse(response) }.is_ok());
            drop(response);
            drop(stream);
            drop(environment);
            applied && c.same(owner)
        }
        fn navigation(
            owner: &Rc<RefCell<Owner>>,
            args: &ICoreWebView2NavigationStartingEventArgs,
            _entry: (u8, bool),
        ) -> bool {
            let c = Context::new(owner);
            let Ok(_raw) = RawLease::acquire(&c.inventory) else {
                let _ = unsafe { args.SetCancel(true) };
                return false;
            };
            let uri = allocated(&c, owner, 2048, |p| unsafe { args.Uri(p) });
            let mut id = 0;
            let a = unit(&c, owner, || unsafe { args.NavigationId(&mut id) });
            let mut redirect = BOOL::default();
            let b = unit(&c, owner, || unsafe { args.IsRedirected(&mut redirect) });
            let mut cancel = BOOL::default();
            let d = unit(&c, owner, || unsafe { args.Cancel(&mut cancel) });
            let valid = uri.as_ref().is_ok_and(|u| u == wire::MAIN_URL)
                && a
                && b
                && d
                && !redirect.as_bool()
                && !cancel.as_bool()
                && c.window(owner);
            drop(uri);
            drop(_raw);
            let allowed = {
                let mut o = owner.borrow_mut();
                let i = o.instance.as_mut().unwrap();
                let live = !i.closing && !i.failed;
                let s = i.capability.as_mut().unwrap();
                s.starts = s.starts.saturating_add(1);
                let good = valid
                    && live
                    && s.grant
                    && s.navigation_issued
                    && s.starts == 1
                    && s.deadline.before_original_at(Instant::now());
                s.grant = false;
                if good {
                    s.navigation_id = Some(id);
                }
                good
            };
            if !allowed {
                let result = unsafe { args.SetCancel(true) };
                let ok = result.is_ok();
                drop(result);
                reject(owner, Failure::InitialNavigation, !ok);
                return ok;
            }
            true
        }
        fn content_loading(
            owner: &Rc<RefCell<Owner>>,
            args: &ICoreWebView2ContentLoadingEventArgs,
            _entry: (u8, bool),
        ) -> bool {
            let c = Context::new(owner);
            let mut id = 0;
            let a = unit(&c, owner, || unsafe { args.NavigationId(&mut id) });
            let mut error = BOOL::default();
            let b = unit(&c, owner, || unsafe { args.IsErrorPage(&mut error) });
            let good = a && b && !error.as_bool() && c.window(owner);
            let mut o = owner.borrow_mut();
            let i = o.instance.as_mut().unwrap();
            let live = !i.closing && !i.failed;
            let s = i.capability.as_mut().unwrap();
            s.loading = s.loading.saturating_add(1);
            good && live
                && s.loading == 1
                && s.navigation_id == Some(id)
                && s.navigation_issued
                && s.deadline.before_original_at(Instant::now())
        }
        fn process_failed(
            owner: &Rc<RefCell<Owner>>,
            args: &ICoreWebView2ProcessFailedEventArgs,
            _entry: (u8, bool),
        ) -> bool {
            // The actual renderer/process-loss event immediately terminates this slice.
            let mut kind = COREWEBVIEW2_PROCESS_FAILED_KIND::default();
            let result = unsafe { args.ProcessFailedKind(&mut kind) };
            let good = result.is_ok();
            drop(result);
            reject(owner, Failure::NativeCleanup, false);
            good
        }
        fn main_html() -> &'static [u8] {
            b"<!doctype html><title>s4-main</title><body></body>"
        }
        fn child_html(request: &Code0WindowRequest, selector: Selector) -> Result<String, ()> {
            let nonce = serde_json::to_string(&request.nonce).map_err(|_| ())?;
            let case = serde_json::to_string(selector.name()).map_err(|_| ())?;
            Ok(format!(
                r#"<!doctype html><title>s4-child</title><body><script>"use strict";globalThis.__aiMeterS4Capability(JSON.stringify({{p:"s4c1",n:{nonce},c:{case},s:1,k:"childProbe",f:0}}));</script></body>"#
            ))
        }
        fn injection(request: &Code0WindowRequest, selector: Selector) -> Result<String, ()> {
            let nonce = serde_json::to_string(&request.nonce).map_err(|_| ())?;
            let document = serde_json::to_string(&request.document_id).map_err(|_| ())?;
            let case = serde_json::to_string(selector.name()).map_err(|_| ())?;
            let canonical = crate::collectors::code0_bridge::CODE0_BRIDGE_SOURCE;
            // Only native constants are interpolated through the JSON serializer.
            let source = format!(
                r#"(()=>{{"use strict";if(window!==top||location.origin!=="https://console.code0.ai")return;
const n={nonce},c={case},d={document},stringify=JSON.stringify.bind(JSON),send=globalThis.__aiMeterS4Capability.bind(globalThis),wv=chrome.webview,add=wv.addEventListener.bind(wv),remove=wv.removeEventListener.bind(wv),originalFetch=window.fetch,proto=XMLHttpRequest.prototype,originalOpen=proto.open,originalSend=proto.send,doc=document,apply=Reflect.apply,timer=setTimeout.bind(window);
let seq=0,phase=0,oldRoot=null,oldURL=null,cleanOpen=null,frame=null;
const emit=(k,f=0)=>send(stringify({{p:"s4c1",n,c,s:++seq,k,f}}));
const controller=({canonical})({{version:1,nonce:n,document:d,send:message=>send(stringify(message))}}),ownedFetch=window.fetch,ownedOpen=proto.open,ownedSend=proto.send;
const owned=()=>window.fetch===ownedFetch&&proto.open===ownedOpen&&proto.send===ownedSend;
const candidate=()=>{{if(!owned())return;oldRoot=document.documentElement;oldURL=location.href;phase=2;emit("candidate");}};
const listener=event=>{{const m=event.data;if(!m||typeof m!=="object"||Array.isArray(m)||Object.keys(m).sort().join(",")!=="c,k,n,p"||m.p!=="s4c1"||m.n!==n||m.c!==c)return;
if(m.k==="prepare"&&phase===1){{if(c==="s4-capability-open-child-prototype"){{phase=10;frame=document.createElement("iframe");frame.src="https://console.code0.ai/__ai_meter_fixture__/s4-capability-child";frame.onload=()=>{{cleanOpen=frame.contentWindow.Document.prototype.open;candidate();}};document.body.appendChild(frame);}}else candidate();return;}}
if(m.k==="mutate"&&phase===2&&c!=="s4-capability-dispose-missing"){{phase=3;emit("beforeOpen");const MARKUP='<!doctype html><title>s4-reset</title><body id="s4-reset"></body>';
if(c==="s4-capability-open-main"){{document.open();document.write(MARKUP);document.close();}}else if(c==="s4-capability-write-main"){{document.write(MARKUP);document.close();}}else{{apply(cleanOpen,document,[]);document.write(MARKUP);document.close();}}
const f=(!oldRoot.isConnected?1:0)|(document.getElementById("s4-reset")?2:0)|(document===doc?4:0)|(location.href===oldURL?8:0)|(owned()?16:0);emit("afterOpen",f);timer(()=>{{phase=4;emit("later");}},0);return;}}
if(m.k==="detach"&&((phase===4&&c!=="s4-capability-dispose-missing")||(phase===2&&c==="s4-capability-dispose-missing"))){{phase=5;if(!owned())return;controller.dispose();const f=(window.fetch===originalFetch?1:0)|(proto.open===originalOpen?2:0)|(proto.send===originalSend?4:0);emit(c==="s4-capability-dispose-missing"?"disposeWitness":"disposed",f);remove("message",listener);}}
}};add("message",listener);window.addEventListener("load",()=>timer(()=>{{if(document.readyState!=="complete")return;phase=1;emit("parsed");}},0),{{once:true}});
}})();"#
            );
            if source.len() > 32768 {
                return Err(());
            }
            Ok(source)
        }
        fn response_pair(
            c: &Context,
            owner: &Rc<RefCell<Owner>>,
            environment: &ICoreWebView2Environment,
            body: &[u8],
            positive: bool,
        ) -> Option<(IStream, ICoreWebView2WebResourceResponse)> {
            if !c.window(owner) {
                return None;
            }
            let stream = unsafe { SHCreateMemStream(Some(body)) }?;
            if !c.window(owner)
                || !unit(c, owner, || unsafe {
                    stream.Seek(0, STREAM_SEEK_SET, None)
                })
            {
                drop(stream);
                return None;
            }
            let response = unsafe {
                environment.CreateWebResourceResponse(&stream,if positive{200}else{403},if positive{windows::core::w!("OK")}else{windows::core::w!("Forbidden")},windows::core::w!("Content-Type: text/html; charset=utf-8\r\nContent-Security-Policy: default-src 'none'; script-src 'unsafe-inline'; frame-src https://console.code0.ai; connect-src 'none'; object-src 'none'; media-src 'none'\r\nCache-Control: no-store\r\n"))
            };
            if !c.window(owner) {
                drop(response);
                drop(stream);
                return None;
            }
            match response {
                Ok(response) => Some((stream, response)),
                Err(error) => {
                    drop(error);
                    drop(stream);
                    None
                }
            }
        }
        fn asset_live(c: &Context, owner: &Rc<RefCell<Owner>>) -> bool {
            c.window(owner) && {
                let o = owner.borrow();
                o.instance.as_ref().is_some_and(|i| {
                    !i.closing
                        && !i.failed
                        && i.capability
                            .as_ref()
                            .is_some_and(|s| s.deadline.before_original_at(Instant::now()))
                })
            }
        }
        fn resource(
            owner: &Rc<RefCell<Owner>>,
            args: &ICoreWebView2WebResourceRequestedEventArgs,
            _entry: (u8, bool),
        ) -> bool {
            let c = Context::new(owner);
            let Ok(_raw) = RawLease::acquire(&c.inventory) else {
                return false;
            };
            if !c.window(owner) {
                return false;
            }
            let native = unsafe { args.Request() };
            let (uri, method) = if let Ok(native) = native.as_ref() {
                (
                    allocated(&c, owner, 2048, |p| unsafe { native.Uri(p) }),
                    allocated(&c, owner, 16, |p| unsafe { native.Method(p) }),
                )
            } else {
                (Err(()), Err(()))
            };
            let mut context = COREWEBVIEW2_WEB_RESOURCE_CONTEXT::default();
            let result = unit(&c, owner, || unsafe { args.ResourceContext(&mut context) });
            let (environment, body, positive, bounded) = {
                let mut o = owner.borrow_mut();
                let i = o.instance.as_mut().unwrap();
                let live = !i.closing && !i.failed;
                let s = i.capability.as_mut().unwrap();
                s.request_count = s.request_count.saturating_add(1);
                let bounded = s.request_count <= 8;
                let get = method.as_ref().is_ok_and(|m| m == "GET")
                    && result
                    && context == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT;
                let main = bounded
                    && live
                    && s.navigation_issued
                    && !s.main_response
                    && get
                    && uri.as_ref().is_ok_and(|u| u == wire::MAIN_URL);
                let child = bounded
                    && live
                    && s.selector == Selector::OpenChildPrototype
                    && s.prepare_sent
                    && !s.child_response
                    && get
                    && uri.as_ref().is_ok_and(|u| u == wire::CHILD_URL);
                let body = if main {
                    main_html().to_vec()
                } else if child {
                    child_html(&s.request, s.selector)
                        .unwrap_or_default()
                        .into_bytes()
                } else {
                    Vec::new()
                };
                let fits = body.len() <= 32768 && s.asset_bytes + body.len() <= 49152;
                let positive = (main || child) && fits;
                (
                    s.environment.as_ref().map(Rc::clone),
                    if positive { body } else { Vec::new() },
                    if main && positive {
                        1
                    } else if child && positive {
                        2
                    } else {
                        0
                    },
                    bounded && fits,
                )
            };
            let metadata_good = native.is_ok() && uri.is_ok() && method.is_ok() && result;
            drop(native);
            drop(uri);
            drop(method);
            drop(_raw);
            let Some(environment) = environment else {
                return false;
            };
            let mut positive = positive;
            let mut pair = response_pair(&c, owner, &environment, &body, positive > 0);
            if positive > 0 && !asset_live(&c, owner) {
                drop(pair);
                positive = 0;
                pair = response_pair(&c, owner, &environment, &[], false);
            }
            let applied = pair.as_ref().is_some_and(|(_, response)| {
                unit(&c, owner, || unsafe { args.SetResponse(response) })
            });
            drop(pair);
            drop(environment);
            let good =
                applied && bounded && c.window(owner) && (positive == 0 || asset_live(&c, owner));
            if good && positive > 0 {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                s.asset_bytes += body.len();
                if positive == 1 {
                    s.main_response = true;
                } else {
                    s.child_response = true;
                }
            }
            drop(body);
            if applied && (!bounded || !metadata_good) {
                reject(
                    owner,
                    if !bounded {
                        Failure::BoundExceeded
                    } else {
                        Failure::Metadata
                    },
                    false,
                );
                return c.window(owner);
            }
            good && c.window(owner)
        }
        enum Decoded {
            Binding(wire::BindingMessage),
            Created(wire::ExecutionContext),
            Destroyed(wire::DestroyedContext),
            Cleared,
            Opened(wire::Frame),
            Navigated(wire::Frame),
            Detached(wire::DetachedFrame),
        }
        fn receive(
            owner: &Rc<RefCell<Owner>>,
            c: &Context,
            event: Event,
            args: &ICoreWebView2DevToolsProtocolEventReceivedEventArgs,
            entry: (u8, bool),
        ) -> bool {
            if !c.window(owner) {
                return false;
            }
            let args2 = match args.cast::<ICoreWebView2DevToolsProtocolEventReceivedEventArgs2>() {
                Ok(v) => v,
                Err(_) => return false,
            };
            if !c.window(owner) {
                drop(args2);
                return false;
            }
            let mut p = PWSTR::null();
            let result = unsafe { args2.SessionId(&mut p) };
            let allocation = CoTaskMemPWSTR::from(p);
            let empty = result.is_ok() && c.window(owner) && !p.is_null() && unsafe { *p.0 } == 0;
            drop(result);
            drop(allocation);
            drop(args2);
            if !empty || !c.window(owner) {
                return false;
            }
            let Ok(raw_lease) = RawLease::acquire(&c.inventory) else {
                return false;
            };
            let raw = allocated(c, owner, 8192, |p| unsafe { args.ParameterObjectAsJson(p) });
            let decoded = raw.as_ref().map_err(|_| ()).and_then(|raw| {
                let o = owner.borrow();
                let s = o.instance.as_ref().unwrap().capability.as_ref().unwrap();
                match event {
                    Event::BindingCalled => {
                        wire::parse_binding(raw.as_bytes(), &s.identity).map(Decoded::Binding)
                    }
                    Event::ContextCreated => {
                        wire::parse_context_created(raw.as_bytes()).map(Decoded::Created)
                    }
                    Event::ContextDestroyed => {
                        wire::parse_context_destroyed(raw.as_bytes()).map(Decoded::Destroyed)
                    }
                    Event::ContextsCleared => {
                        wire::parse_contexts_cleared(raw.as_bytes()).map(|()| Decoded::Cleared)
                    }
                    Event::DocumentOpened => wire::parse_frame(raw.as_bytes()).map(Decoded::Opened),
                    Event::FrameNavigated => {
                        wire::parse_frame(raw.as_bytes()).map(Decoded::Navigated)
                    }
                    Event::FrameDetached => {
                        wire::parse_frame_detached(raw.as_bytes()).map(Decoded::Detached)
                    }
                }
                .map_err(|_| ())
            });
            drop(raw);
            drop(raw_lease);
            if !c.window(owner) {
                return false;
            }
            let Ok(decoded) = decoded else {
                return false;
            };
            let current = c.authority(owner);
            let mut failure = None;
            {
                let mut o = owner.borrow_mut();
                let i = o.instance.as_mut().unwrap();
                let s = i.capability.as_mut().unwrap();
                if failure.is_none() {
                    match decoded {
                        Decoded::Binding(message) => {
                            if !s.facts.initial_native_join || !s.facts.root_metadata_joined {
                                if s.staged.push(message).is_err() || s.staged_entries.len() >= 10 {
                                    failure = Some(Failure::BoundExceeded);
                                } else {
                                    s.staged_entries.push_back(entry);
                                }
                            } else if !apply_binding(s, message, entry) {
                                failure = Some(Failure::Protocol);
                            }
                        }
                        Decoded::Created(context) => {
                            if !observe_context_created_at(s, context, current, Instant::now()) {
                                failure = Some(Failure::ContextChanged);
                            }
                        }
                        Decoded::Navigated(frame) => {
                            if s.root.as_ref() == Some(&frame.id) {
                                if s.navigation_issued
                                    && s.loader.is_none()
                                    && s.root.as_ref().is_some_and(|r| frame.matches_main_root(r))
                                    && !matches!(
                                        s.phase,
                                        Phase::ClosingAwaitDispose
                                            | Phase::RemoveProtocols
                                            | Phase::StopClose
                                            | Phase::AwaitReleased
                                            | Phase::Final
                                    )
                                {
                                    s.loader = Some(frame.loader_id);
                                } else if s.navigation_issued || frame.url != wire::FrameUrl::Blank
                                {
                                    failure = Some(Failure::ContextChanged);
                                }
                            } else if frame.parent_id.as_ref() != s.root.as_ref()
                                && s.navigation_issued
                            {
                                failure = Some(Failure::Metadata);
                            }
                        }
                        Decoded::Opened(frame) => {
                            failure = observe_document_opened(s, frame, entry.0);
                        }
                        Decoded::Destroyed(context) => {
                            if s.root_context == Some(context.id)
                                || (s.permit.is_some()
                                    && (s.contexts.get(context.id).is_none()
                                        || context.unique_id.as_ref().is_some_and(|u| {
                                            s.contexts
                                                .get(context.id)
                                                .and_then(|c| c.unique_id.as_ref())
                                                != Some(u)
                                        })))
                            {
                                failure = Some(Failure::ContextChanged);
                            }
                        }
                        Decoded::Cleared => {
                            if !observe_contexts_cleared(s) {
                                failure = Some(Failure::ContextChanged);
                            }
                        }
                        Decoded::Detached(frame) => {
                            if s.root.as_ref() == Some(&frame.id) {
                                failure = Some(Failure::ContextChanged);
                            }
                        }
                    }
                }
            }
            if let Some(failure) = failure {
                reject(owner, failure, false);
                return false;
            }
            true
        }
        fn observe_document_opened(
            s: &mut State,
            frame: wire::Frame,
            entry: u8,
        ) -> Option<Failure> {
            if s.root.as_ref() == Some(&frame.id) {
                if s.phase != Phase::AwaitMutationTrace
                    || s.facts.before_ordinal.is_none()
                    || s.facts.document_opened_ordinal.is_some()
                    || !s.root.as_ref().is_some_and(|r| frame.matches_main_root(r))
                {
                    return Some(Failure::StimulusNotObserved);
                }
                s.facts.document_opened_ordinal = Some(entry);
                s.facts.root_loader_matched = Some(s.loader.as_ref() == Some(&frame.loader_id));
                // Invalidate the same native permit before the later entry snapshot.
                // Observation stays nonterminal; the original held consumer still runs.
                revoke(s);
            } else if frame.parent_id.as_ref() != s.root.as_ref() {
                return Some(Failure::Metadata);
            }
            None
        }
        fn apply_binding(s: &mut State, message: wire::BindingMessage, entry: (u8, bool)) -> bool {
            let root = s.root_context == Some(message.context_id);
            if !root {
                return match message.payload {
                    PagePayload::Probe(probe)
                        if probe.kind() == ProbeKind::ChildProbe
                            && probe.sequence() == 1
                            && probe.flags() == 0
                            && s.selector == Selector::OpenChildPrototype
                            && s.prepare_sent
                            && s.facts.child_probe_refused == Some(false) =>
                    {
                        let child = s.contexts.get(message.context_id).is_some_and(|c| {
                            c.origin.as_str() == wire::OFFICIAL_ORIGIN
                                && c.unique_id.is_some()
                                && c.aux_data.as_ref().is_some_and(|a| {
                                    a.frame_id
                                        .as_ref()
                                        .is_some_and(|f| Some(f) != s.root.as_ref())
                                        && a.is_default == Some(true)
                                        && a.default_type == Some(true)
                                })
                        });
                        if child {
                            s.facts.child_probe_refused = Some(true);
                        }
                        child
                    }
                    _ => false,
                };
            }
            match message.payload {
                PagePayload::Ready(_) => {
                    let good = !s.facts.canonical_ready && !s.prepare_sent;
                    if good {
                        s.facts.canonical_ready = true;
                    }
                    good
                }
                PagePayload::Probe(probe) => {
                    if s.sequence.accept_root(&probe).is_err() {
                        return false;
                    }
                    match probe.kind() {
                        ProbeKind::Parsed => {
                            let good = !s.facts.parsed_after_complete
                                && !s.prepare_sent
                                && probe.flags() == 0;
                            if good {
                                s.facts.parsed_after_complete = true;
                            }
                            good
                        }
                        ProbeKind::Candidate => {
                            let good = s.phase == Phase::AwaitCandidate
                                && s.prepare_sent
                                && !s.facts.candidate_held
                                && probe.flags() == 0;
                            if !good {
                                return false;
                            }
                            let Some(permit) = s.permit.as_ref() else {
                                return false;
                            };
                            if permit.revoked.get() {
                                return false;
                            }
                            s.held = Some(Held {
                                probe,
                                permit: Rc::clone(permit),
                                generation: permit.generation,
                            });
                            s.facts.candidate_held = true;
                            true
                        }
                        ProbeKind::BeforeOpen => {
                            let good = s.phase == Phase::AwaitMutationTrace
                                && s.mutate_sent
                                && s.facts.candidate_held
                                && s.facts.before_ordinal.is_none()
                                && probe.flags() == 0;
                            if good {
                                s.facts.before_ordinal = Some(entry.0);
                            }
                            good
                        }
                        ProbeKind::AfterOpen => {
                            let good = s.phase == Phase::AwaitMutationTrace
                                && s.facts.before_ordinal.is_some()
                                && s.facts.after_ordinal.is_none()
                                && probe.flags() == 31;
                            if good {
                                s.facts.after_ordinal = Some(entry.0);
                                s.facts.permit_revoked_at_after_entry = Some(entry.1);
                                s.facts.logical_reset_flags = Some(probe.flags());
                                s.facts.root_context_retained = Some(
                                    s.root_context == Some(message.context_id)
                                        && s.contexts
                                            .get(message.context_id)
                                            .and_then(|c| c.unique_id.as_ref())
                                            == s.unique.as_ref(),
                                );
                                s.consumer_due = true;
                            }
                            good
                        }
                        ProbeKind::Later => {
                            let good = s.phase == Phase::AwaitMutationTrace
                                && s.facts.after_ordinal.is_some()
                                && s.facts.later_ordinal.is_none()
                                && probe.flags() == 0;
                            if good {
                                s.facts.later_ordinal = Some(entry.0);
                            }
                            good
                        }
                        ProbeKind::Disposed => {
                            let now = Instant::now();
                            let timely = s.closing.is_some_and(|close| receipt_timely(now, close));
                            let good = s.selector != Selector::DisposeMissing
                                && s.phase == Phase::ClosingAwaitDispose
                                && s.detach_sent
                                && s.facts.dispose_receipt_count == 0
                                && probe.flags() == 7
                                && timely;
                            if good {
                                s.facts.dispose_receipt_count = 1;
                                s.facts.restoration_flags = 7;
                                s.disposed_at = Some(now);
                            } else if !timely {
                                s.facts.failure = Some(Failure::DetachReceiptLate);
                            }
                            good
                        }
                        ProbeKind::DisposeWitness => {
                            let good = s.selector == Selector::DisposeMissing
                                && s.phase == Phase::ClosingAwaitDispose
                                && s.detach_sent
                                && !s.witness
                                && probe.flags() == 7
                                && s.closing
                                    .is_some_and(|close| receipt_timely(Instant::now(), close));
                            if good {
                                s.witness = true;
                                s.facts.dispose_witness_count = 1;
                                s.facts.restoration_flags = 7;
                            }
                            good
                        }
                        ProbeKind::ChildProbe => false,
                    }
                }
            }
        }
        #[windows::core::implement(
            ICoreWebView2CallDevToolsProtocolMethodCompletedHandler,
            Agile = false
        )]
        struct Completion {
            context: Context,
            index: usize,
            _lifetime: Lifetime,
        }
        impl ICoreWebView2CallDevToolsProtocolMethodCompletedHandler_Impl for Completion_Impl {
            fn Invoke(
                &self,
                hr: windows::core::HRESULT,
                raw: &PCWSTR,
            ) -> windows::core::Result<()> {
                let c = &self.context;
                let active = Active::enter(&c.callbacks);
                if !callback_on_ui(c.ui, &c.callbacks, 8040) {
                    c.inventory.invalid.store(true, Ordering::SeqCst);
                    return Ok(());
                }
                let Some(owner) = c.owner.upgrade() else {
                    c.inventory.invalid.store(true, Ordering::SeqCst);
                    return Ok(());
                };
                let _entry = c.entry(&owner, 20 + self.index as u8);
                if !c.inventory.invoke(self.index) || !c.same(&owner) {
                    c.inventory.invalid.store(true, Ordering::SeqCst);
                    return Ok(());
                }
                let lease = RawLease::acquire(&c.inventory);
                let copied = if hr.is_ok() && lease.is_ok() && c.same(&owner) {
                    copy_units(*raw, 8192)
                } else {
                    Err(())
                };
                let result = copied.as_ref().map_err(|_| ()).and_then(|v| {
                    wire::parse_method_result(Method::ALL[self.index], v.as_bytes()).map_err(|_| ())
                });
                drop(copied);
                drop(lease);
                let good = c.same(&owner);
                {
                    let mut o = owner.borrow_mut();
                    let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                    if s.response.is_some() || !s.in_flight || !s.submitted[self.index] {
                        s.inventory.invalid.store(true, Ordering::SeqCst);
                    } else {
                        s.response = Some(Response {
                            index: self.index,
                            received: Instant::now(),
                            result: if good { result } else { Err(()) },
                        });
                    }
                }
                drop(active);
                Ok(())
            }
        }
        #[windows::core::implement(
            ICoreWebView2AddScriptToExecuteOnDocumentCreatedCompletedHandler,
            Agile = false
        )]
        struct ScriptCompletion {
            context: Context,
            _lifetime: Lifetime,
        }
        impl ICoreWebView2AddScriptToExecuteOnDocumentCreatedCompletedHandler_Impl
            for ScriptCompletion_Impl
        {
            fn Invoke(
                &self,
                hr: windows::core::HRESULT,
                raw: &PCWSTR,
            ) -> windows::core::Result<()> {
                let c = &self.context;
                let active = Active::enter(&c.callbacks);
                if !callback_on_ui(c.ui, &c.callbacks, 8041) {
                    c.inventory.invalid.store(true, Ordering::SeqCst);
                    return Ok(());
                }
                let Some(owner) = c.owner.upgrade() else {
                    c.inventory.invalid.store(true, Ordering::SeqCst);
                    return Ok(());
                };
                let _entry = c.entry(&owner, 27);
                if !c.inventory.invoke(7) || !c.same(&owner) {
                    c.inventory.invalid.store(true, Ordering::SeqCst);
                    if hr.is_ok() && c.same(&owner) {
                        owner
                            .borrow_mut()
                            .instance
                            .as_mut()
                            .unwrap()
                            .capability
                            .as_mut()
                            .unwrap()
                            .cleanup_unknown = true;
                    }
                    return Ok(());
                }
                let lease = RawLease::acquire(&c.inventory);
                let result = if hr.is_ok() && lease.is_ok() && c.same(&owner) {
                    copy_units(*raw, 256)
                        .and_then(|id| if !id.is_empty() { Ok(id) } else { Err(()) })
                } else {
                    Err(())
                };
                drop(lease);
                {
                    let mut o = owner.borrow_mut();
                    let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                    if s.script_response.is_some() {
                        s.inventory.invalid.store(true, Ordering::SeqCst);
                    } else {
                        if hr.is_ok() && result.is_err() {
                            s.cleanup_unknown = true;
                        }
                        s.script_response = Some((Instant::now(), result));
                    }
                }
                drop(active);
                Ok(())
            }
        }
        fn issue(owner: &Rc<RefCell<Owner>>, index: usize, core: &ICoreWebView2) {
            let c = Context::new(owner);
            if !(if index < 4 {
                c.live_at(owner, Phase::Installing, Instant::now())
            } else {
                c.window(owner)
            }) {
                reject(owner, Failure::Setup, false);
                return;
            }
            {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                if s.in_flight || s.submitted[index] || !s.inventory.issue(index) {
                    return;
                }
                s.submitted[index] = true;
                s.in_flight = true;
                s.facts.method_submissions += 1;
            }
            let qualifier = Context::new(owner);
            let handler: ICoreWebView2CallDevToolsProtocolMethodCompletedHandler = Completion {
                _lifetime: Lifetime::new(&c),
                context: c,
                index,
            }
            .into();
            let method: Vec<u16> = Method::ALL[index]
                .name()
                .encode_utf16()
                .chain(Some(0))
                .collect();
            let params: Vec<u16> = Method::ALL[index]
                .parameters()
                .encode_utf16()
                .chain(Some(0))
                .collect();
            let allowed = if index < 4 {
                qualifier.live_at(owner, Phase::Installing, Instant::now())
            } else {
                qualifier.window(owner)
            };
            let result = if allowed {
                unsafe {
                    core.CallDevToolsProtocolMethod(
                        PCWSTR(method.as_ptr()),
                        PCWSTR(params.as_ptr()),
                        &handler,
                    )
                }
            } else {
                Err(E_FAIL.into())
            };
            let good = result.is_ok();
            drop(result);
            drop(handler);
            drop(method);
            drop(params);
            if !good {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                s.inventory.abort(index);
                s.in_flight = false;
                drop(o);
                reject(owner, Failure::Setup, false);
            } else if index < 4 && !qualifier.live_at(owner, Phase::Installing, Instant::now()) {
                reject(owner, Failure::Setup, false);
            }
        }
        fn submit_script(owner: &Rc<RefCell<Owner>>, core: &ICoreWebView2) {
            let c = Context::new(owner);
            if !c.live_at(owner, Phase::Installing, Instant::now()) {
                reject(owner, Failure::Setup, false);
                return;
            }
            let script = {
                let o = owner.borrow();
                let s = o.instance.as_ref().unwrap().capability.as_ref().unwrap();
                injection(&s.request, s.selector)
            };
            let Ok(script) = script else {
                reject(owner, Failure::BoundExceeded, false);
                return;
            };
            {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                if s.script_submitted || !s.inventory.issue(7) {
                    return;
                }
                s.script_submitted = true;
            }
            let qualifier = Context::new(owner);
            let handler: ICoreWebView2AddScriptToExecuteOnDocumentCreatedCompletedHandler =
                ScriptCompletion {
                    _lifetime: Lifetime::new(&c),
                    context: c,
                }
                .into();
            let wide: Vec<u16> = script.encode_utf16().chain(Some(0)).collect();
            let result = if qualifier.live_at(owner, Phase::Installing, Instant::now()) {
                unsafe { core.AddScriptToExecuteOnDocumentCreated(PCWSTR(wide.as_ptr()), &handler) }
            } else {
                Err(E_FAIL.into())
            };
            let good = result.is_ok();
            drop(result);
            drop(handler);
            drop(wide);
            drop(script);
            if !good {
                owner
                    .borrow()
                    .instance
                    .as_ref()
                    .unwrap()
                    .capability
                    .as_ref()
                    .unwrap()
                    .inventory
                    .abort(7);
                reject(owner, Failure::Setup, false);
            } else if !qualifier.live_at(owner, Phase::Installing, Instant::now()) {
                reject(owner, Failure::Setup, false);
            }
        }
        fn consume_responses(owner: &Rc<RefCell<Owner>>) {
            let c = Context::new(owner);
            let current = c.authority(owner);
            let (response, script) = {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                (s.response.take(), s.script_response.take())
            };
            if let Some(response) = response {
                let now = Instant::now();
                let mut failure = None;
                {
                    let mut o = owner.borrow_mut();
                    let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                    let consumed = s.inventory.consume(response.index);
                    s.in_flight = false;
                    let timely = if response.index < 4 {
                        s.active_at(current, Phase::Installing, now)
                            && s.deadline.before_original_at(response.received)
                            && s.deadline.before_original_at(now)
                            && s.closing.is_none()
                    } else {
                        NativeDialogDeadline::disposal_within_close_at(
                            response.received,
                            now,
                            s.closing,
                        )
                    };
                    if consumed && response.result.is_ok() {
                        s.acknowledged[response.index] = true;
                        s.facts.method_acks += 1;
                        if let Ok(wire::MethodResult::InitialRoot(root)) = response.result {
                            if s.root.is_some() {
                                failure = Some(Failure::Metadata);
                            } else {
                                s.root = Some(root);
                            }
                        }
                        if response.index < 4 && timely {
                            s.next = response.index + 1;
                        }
                        if response.index < 3 && s.closing.is_some() {
                            s.phase = Phase::RemoveProtocols;
                        }
                    }
                    if !consumed || !timely || !s.acknowledged[response.index] {
                        failure = Some(if response.index < 4 {
                            Failure::Setup
                        } else {
                            Failure::NativeCleanup
                        });
                    }
                }
                if let Some(failure) = failure {
                    reject(owner, failure, false);
                }
            }
            if let Some((received, result)) = script {
                let mut failure = None;
                {
                    let mut o = owner.borrow_mut();
                    let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                    let consumed = s.inventory.consume(7);
                    let timely = s.active_at(current, Phase::Installing, Instant::now())
                        && s.deadline.before_original_at(received)
                        && s.deadline.before_original_at(Instant::now())
                        && s.closing.is_none();
                    match result {
                        Ok(id) => {
                            s.script_id = Some(id);
                            if consumed && timely {
                                s.facts.script_ack = true;
                            } else {
                                failure = Some(Failure::Setup);
                            }
                        }
                        Err(()) => failure = Some(Failure::Setup),
                    }
                }
                if let Some(failure) = failure {
                    reject(owner, failure, false);
                }
            }
        }
        fn post(owner: &Rc<RefCell<Owner>>, core: &ICoreWebView2, kind: ControlKind) -> bool {
            let c = Context::new(owner);
            let current = c.authority(owner);
            let control = {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                reserve_control_at(s, current, kind, Instant::now())
            };
            let Some(control) = control else {
                return false;
            };
            let wide: Vec<u16> = std::str::from_utf8(control.as_bytes())
                .unwrap()
                .encode_utf16()
                .chain(Some(0))
                .collect();
            let expected = match kind {
                ControlKind::Prepare => Phase::AwaitCandidate,
                ControlKind::Mutate => Phase::AwaitMutationTrace,
                ControlKind::Detach => Phase::ClosingAwaitDispose,
            };
            let qualified = if kind == ControlKind::Detach {
                c.window(owner)
            } else {
                c.live_at(owner, expected, Instant::now())
            };
            let good = qualified
                && unit(&c, owner, || unsafe {
                    core.PostWebMessageAsJson(PCWSTR(wide.as_ptr()))
                });
            drop(wide);
            good && if kind == ControlKind::Detach {
                c.window(owner)
            } else {
                c.live_at(owner, expected, Instant::now())
            }
        }
        fn navigate(owner: &Rc<RefCell<Owner>>, core: &ICoreWebView2) -> bool {
            let c = Context::new(owner);
            if !c.live_at(owner, Phase::Installing, Instant::now()) || !source(owner, core, true) {
                return false;
            }
            let current = c.authority(owner);
            let reserved = {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                reserve_navigation_at(s, current, Instant::now())
            };
            if !reserved || !c.live_at(owner, Phase::AwaitInitialJoin, Instant::now()) {
                return false;
            }
            let result = unsafe {
                core.Navigate(windows::core::w!(
                    "https://console.code0.ai/__ai_meter_fixture__/s4-capability-main"
                ))
            };
            let good = result.is_ok();
            drop(result);
            let live = c.live_at(owner, Phase::AwaitInitialJoin, Instant::now());
            {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                s.navigation_returned = good && live;
            }
            good && live
        }
        fn join(owner: &Rc<RefCell<Owner>>, core: &ICoreWebView2) -> bool {
            let c = Context::new(owner);
            let current = c.authority(owner);
            let (possible, expected) = {
                let o = owner.borrow();
                let s = o.instance.as_ref().unwrap().capability.as_ref().unwrap();
                if !s.active_at(current, s.phase, Instant::now()) {
                    return false;
                }
                (
                    s.navigation_returned
                        && s.starts == 1
                        && s.loading == 1
                        && s.main_response
                        && s.navigation_id.is_some()
                        && s.loader.is_some()
                        && s.root_context.is_some()
                        && s.unique.is_some(),
                    s.phase,
                )
            };
            if !possible {
                return true;
            }
            if !source(owner, core, false) {
                return false;
            }
            // Source and every allocation/native local Drop have ended. Re-read
            // current owner identity/failure/phase before any join grant.
            let current = c.authority(owner);
            let joined = {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                commit_join_at(s, current, expected, Instant::now())
            };
            joined && {
                let current = c.authority(owner);
                let o = owner.borrow();
                let s = o.instance.as_ref().unwrap().capability.as_ref().unwrap();
                s.active_at(current, s.phase, Instant::now())
            }
        }
        fn consume_held(owner: &Rc<RefCell<Owner>>) -> bool {
            let c = Context::new(owner);
            let current = c.authority(owner);
            let handoff = {
                let mut o = owner.borrow_mut();
                let i = o.instance.as_mut().unwrap();
                if i.callbacks.active.load(Ordering::SeqCst) != 0 {
                    return true;
                }
                let s = i.capability.as_mut().unwrap();
                take_handoff_at(s, current, Instant::now())
            };
            let handoff = match handoff {
                Ok(Some(handoff)) => handoff,
                Ok(None) => return true,
                Err(()) => return false,
            };
            // The real decoded held item is consumed once, outside Owner/Active
            // and native actions. Time is checked again at this actual use.
            let consumed = handoff.consume_at(Instant::now());
            {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                record_consumption(s, &consumed);
            }
            if let Some(failure) = consumed.failure {
                reject(owner, failure, false);
            }
            true
        }
        fn mutation_trace(s: &State) -> bool {
            s.facts
                .before_ordinal
                .zip(s.facts.document_opened_ordinal)
                .zip(s.facts.after_ordinal)
                .zip(s.facts.later_ordinal)
                .is_some_and(|(((b, d), a), l)| b < d && d < a && a < l)
                && s.facts.root_loader_matched == Some(true)
                && s.facts.root_context_retained == Some(true)
                && s.facts.logical_reset_flags == Some(31)
                && s.facts.candidate_held
                && s.facts.candidate_delivery_attempts == 1
                && (s.selector != Selector::OpenChildPrototype
                    || s.facts.child_probe_refused == Some(true))
        }
        pub(super) fn advance(owner: &Rc<RefCell<Owner>>) {
            if !selected(owner) {
                return;
            }
            synchronize(owner);
            let eligible = {
                let o = owner.borrow();
                let i = o.instance.as_ref().unwrap();
                i.callbacks.active.load(Ordering::SeqCst) == 0
            };
            if !eligible {
                return;
            }
            consume_responses(owner);
            if !consume_held(owner) {
                reject(owner, Failure::Protocol, false);
            }
            let (invalid, unknown) = {
                let o = owner.borrow();
                let s = o.instance.as_ref().unwrap().capability.as_ref().unwrap();
                (
                    s.inventory.invalid.load(Ordering::SeqCst),
                    s.cleanup_unknown,
                )
            };
            if unknown {
                reject(owner, Failure::NativeCleanup, true);
            } else if invalid {
                reject(owner, Failure::BoundExceeded, false);
            }
            let expired = {
                let o = owner.borrow();
                let s = o.instance.as_ref().unwrap().capability.as_ref().unwrap();
                s.closing.is_none() && !s.deadline.before_original_at(Instant::now())
            };
            if expired {
                reject(owner, Failure::StimulusNotObserved, false);
            }
            let c = Context::new(owner);
            let (core, phase, waiting) = {
                let o = owner.borrow();
                let s = o.instance.as_ref().unwrap().capability.as_ref().unwrap();
                (s.core.as_ref().map(Rc::clone), s.phase, s.in_flight)
            };
            let Some(core) = core else {
                return;
            };
            if matches!(
                phase,
                Phase::StopClose | Phase::AwaitReleased | Phase::Final
            ) {
                drop(core);
                return;
            }
            // Consumer above precedes every native action scope on this poll.
            let active = Active::enter(&c.callbacks);
            if phase == Phase::Installing && !waiting {
                let (ready, next, script, ack) = {
                    let o = owner.borrow();
                    let s = o.instance.as_ref().unwrap().capability.as_ref().unwrap();
                    (
                        s.setup_ready,
                        s.next,
                        s.script_submitted,
                        s.facts.script_ack,
                    )
                };
                if ready && next < 4 {
                    issue(owner, next, &core);
                } else if ready && !script {
                    submit_script(owner, &core);
                } else if ready && ack && !navigate(owner, &core) {
                    reject(owner, Failure::InitialNavigation, false);
                }
            } else if matches!(
                phase,
                Phase::AwaitInitialJoin
                    | Phase::AwaitParsed
                    | Phase::AwaitCandidate
                    | Phase::AwaitMutationTrace
            ) {
                if !join(owner, &core) {
                    reject(owner, Failure::Metadata, false);
                } else {
                    let (phase, ready, held, child, later, attempts, missing) = {
                        let o = owner.borrow();
                        let s = o.instance.as_ref().unwrap().capability.as_ref().unwrap();
                        (
                            s.phase,
                            s.facts.canonical_ready && s.facts.parsed_after_complete,
                            s.facts.candidate_held,
                            s.selector != Selector::OpenChildPrototype
                                || s.facts.child_probe_refused == Some(true),
                            s.facts.later_ordinal.is_some(),
                            s.facts.candidate_delivery_attempts,
                            s.selector == Selector::DisposeMissing,
                        )
                    };
                    if phase == Phase::AwaitParsed && ready {
                        if !post(owner, &core, ControlKind::Prepare) {
                            reject(owner, Failure::Protocol, false);
                        }
                    } else if phase == Phase::AwaitCandidate && held && child {
                        if missing {
                            close_selected(owner);
                        } else if !post(owner, &core, ControlKind::Mutate) {
                            reject(owner, Failure::Protocol, false);
                        }
                    } else if phase == Phase::AwaitMutationTrace && later && attempts == 1 {
                        let trace = {
                            let o = owner.borrow();
                            mutation_trace(
                                o.instance.as_ref().unwrap().capability.as_ref().unwrap(),
                            )
                        };
                        let unchanged = trace && source(owner, &core, false);
                        {
                            let mut o = owner.borrow_mut();
                            let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                            s.facts.native_source_unchanged = Some(unchanged);
                        }
                        if unchanged {
                            close_selected(owner);
                        } else {
                            reject(owner, Failure::Ordering, false);
                        }
                    }
                }
            } else if phase == Phase::ClosingAwaitDispose {
                let (close, detach, can_detach, received, selector, disposition, attempts) = {
                    let o = owner.borrow();
                    let s = o.instance.as_ref().unwrap().capability.as_ref().unwrap();
                    (
                        s.closing.unwrap(),
                        s.detach_sent,
                        s.facts.script_ack
                            && s.facts.candidate_held
                            && matches!(
                                s.facts.failure,
                                None | Some(Failure::CapabilityApprovalMissing)
                            )
                            && (s.selector == Selector::DisposeMissing
                                || s.facts.later_ordinal.is_some()),
                        s.disposed_at,
                        s.selector,
                        s.missing_disposition,
                        s.facts.candidate_delivery_attempts,
                    )
                };
                if !can_detach {
                    owner
                        .borrow_mut()
                        .instance
                        .as_mut()
                        .unwrap()
                        .capability
                        .as_mut()
                        .unwrap()
                        .phase = Phase::RemoveProtocols;
                } else if !detach {
                    if !post(owner, &core, ControlKind::Detach) {
                        reject(owner, Failure::Protocol, false);
                        owner
                            .borrow_mut()
                            .instance
                            .as_mut()
                            .unwrap()
                            .capability
                            .as_mut()
                            .unwrap()
                            .phase = Phase::RemoveProtocols;
                    }
                } else if let Some(received) = received {
                    if receipt_timely(received, close) && receipt_timely(Instant::now(), close) {
                        let mut o = owner.borrow_mut();
                        let i = o.instance.as_mut().unwrap();
                        let s = i.capability.as_mut().unwrap();
                        s.receipt_consumed = true;
                        s.phase = Phase::RemoveProtocols;
                        i.failed = s.facts.failure.is_some();
                    } else {
                        reject(owner, Failure::DetachReceiptLate, false);
                        owner
                            .borrow_mut()
                            .instance
                            .as_mut()
                            .unwrap()
                            .capability
                            .as_mut()
                            .unwrap()
                            .phase = Phase::RemoveProtocols;
                    }
                } else if selector == Selector::DisposeMissing && disposition && attempts == 1 {
                    owner
                        .borrow_mut()
                        .instance
                        .as_mut()
                        .unwrap()
                        .capability
                        .as_mut()
                        .unwrap()
                        .phase = Phase::RemoveProtocols;
                } else if !disposition && !receipt_timely(Instant::now(), close) {
                    let mut o = owner.borrow_mut();
                    let i = o.instance.as_mut().unwrap();
                    let s = i.capability.as_mut().unwrap();
                    s.facts.receipt_deadline_observed = true;
                    s.facts.failed_close_no_activation = true;
                    i.failed = true;
                    if selector == Selector::DisposeMissing
                        && s.witness
                        && s.facts.candidate_held
                        && s.facts.candidate_delivery_attempts == 0
                    {
                        s.facts.failure = Some(Failure::DetachReceiptMissing);
                        s.missing_disposition = true;
                        s.consumer_due = true;
                    } else {
                        s.facts.failure = Some(Failure::DetachReceiptMissing);
                        s.facts.result = ResultKind::NotEstablished;
                        s.phase = Phase::RemoveProtocols;
                    }
                }
            } else if phase == Phase::RemoveProtocols && !waiting {
                let next = {
                    let o = owner.borrow();
                    let s = o.instance.as_ref().unwrap().capability.as_ref().unwrap();
                    [(4, 2), (5, 1), (6, 0)]
                        .into_iter()
                        .find(|(cleanup, enabled)| {
                            s.acknowledged[*enabled] && !s.submitted[*cleanup]
                        })
                        .map(|(cleanup, _)| cleanup)
                };
                if let Some(index) = next {
                    issue(owner, index, &core);
                } else {
                    owner
                        .borrow_mut()
                        .instance
                        .as_mut()
                        .unwrap()
                        .capability
                        .as_mut()
                        .unwrap()
                        .phase = Phase::StopClose;
                }
            }
            drop(core);
            drop(active);
            synchronize(owner);
            let disposition = {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                let ready = s.missing_disposition
                    || matches!(s.phase, Phase::RemoveProtocols | Phase::StopClose);
                if ready && !s.disposition_published {
                    s.disposition_published = true;
                    true
                } else {
                    false
                }
            };
            if disposition {
                emit(owner, Stage::Disposition);
            }
        }
        pub(super) fn ready_to_stop(i: &Instance) -> bool {
            i.capability.as_ref().is_none_or(|s| {
                matches!(
                    s.phase,
                    Phase::StopClose | Phase::AwaitReleased | Phase::Final
                ) && !s.in_flight
                    && s.response.is_none()
                    && s.script_response.is_none()
                    && s.inventory.drained()
            })
        }
        pub(super) fn stop_entry(owner: &Rc<RefCell<Owner>>) {
            if let Some(s) = owner
                .borrow_mut()
                .instance
                .as_mut()
                .and_then(|i| i.capability.as_mut())
            {
                if s.phase == Phase::StopClose {
                    s.facts.stop_after_dispose_receipt =
                        s.receipt_consumed && s.facts.dispose_receipt_count == 1;
                    s.phase = Phase::AwaitReleased;
                }
            }
        }
        pub(super) fn removed(i: &Instance) -> bool {
            i.capability.as_ref().is_none_or(|s| {
                s.receivers.iter().all(|r| r.token.is_none())
                    && s.native_tokens.iter().all(Option::is_none)
                    && !s.filter
                    && s.script_id.is_none()
                    && !s.cleanup_unknown
            })
        }
        pub(super) fn remove(owner: &Rc<RefCell<Owner>>) {
            if !selected(owner) {
                return;
            }
            let c = Context::new(owner);
            let core = {
                let o = owner.borrow();
                o.instance
                    .as_ref()
                    .unwrap()
                    .capability
                    .as_ref()
                    .unwrap()
                    .core
                    .as_ref()
                    .map(Rc::clone)
            };
            let Some(core) = core else {
                return;
            };
            let active = Active::enter(&c.callbacks);
            for index in 0..7 {
                let action = {
                    let mut o = owner.borrow_mut();
                    let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                    s.receivers.get_mut(index).and_then(|r| {
                        if r.attempted {
                            return None;
                        }
                        let token = r.token?;
                        r.attempted = true;
                        Some((Rc::clone(&r.receiver), token))
                    })
                };
                if let Some((receiver, token)) = action {
                    let result = unsafe { receiver.remove_DevToolsProtocolEventReceived(token) };
                    let good = result.is_ok();
                    drop(result);
                    drop(receiver);
                    if good {
                        let mut o = owner.borrow_mut();
                        let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                        s.receivers[index].token = None;
                        s.facts.receiver_removes += 1;
                    } else {
                        reject(owner, Failure::NativeCleanup, true);
                    }
                }
            }
            for index in 0..4 {
                let token = {
                    let mut o = owner.borrow_mut();
                    let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                    if s.native_attempted[index] {
                        None
                    } else {
                        s.native_attempted[index] = true;
                        s.native_tokens[index]
                    }
                };
                if let Some(token) = token {
                    let result = match index {
                        0 => unsafe { core.remove_NavigationStarting(token) },
                        1 => unsafe { core.remove_ContentLoading(token) },
                        2 => unsafe { core.remove_ProcessFailed(token) },
                        _ => unsafe { core.remove_WebResourceRequested(token) },
                    };
                    let good = result.is_ok();
                    drop(result);
                    if good {
                        owner
                            .borrow_mut()
                            .instance
                            .as_mut()
                            .unwrap()
                            .capability
                            .as_mut()
                            .unwrap()
                            .native_tokens[index] = None;
                    } else {
                        reject(owner, Failure::NativeCleanup, true);
                    }
                }
            }
            let filter = {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                if s.filter && !s.filter_attempted {
                    s.filter_attempted = true;
                    s.core22.as_ref().map(Rc::clone)
                } else {
                    None
                }
            };
            if let Some(core22) = filter {
                let result = unsafe {
                    core22.RemoveWebResourceRequestedFilterWithRequestSourceKinds(
                        windows::core::w!("*"),
                        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                        COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
                    )
                };
                let good = result.is_ok();
                drop(result);
                drop(core22);
                if good {
                    owner
                        .borrow_mut()
                        .instance
                        .as_mut()
                        .unwrap()
                        .capability
                        .as_mut()
                        .unwrap()
                        .filter = false;
                } else {
                    reject(owner, Failure::NativeCleanup, true);
                }
            }
            let script = {
                let mut o = owner.borrow_mut();
                let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                if s.script_attempted {
                    None
                } else {
                    s.script_attempted = true;
                    s.script_id.clone()
                }
            };
            if let Some(script) = script {
                let wide: Vec<u16> = script.encode_utf16().chain(Some(0)).collect();
                let result =
                    unsafe { core.RemoveScriptToExecuteOnDocumentCreated(PCWSTR(wide.as_ptr())) };
                let good = result.is_ok();
                drop(result);
                drop(wide);
                drop(script);
                if good {
                    let mut o = owner.borrow_mut();
                    let s = o.instance.as_mut().unwrap().capability.as_mut().unwrap();
                    s.script_id = None;
                    s.facts.script_removed = true;
                } else {
                    reject(owner, Failure::NativeCleanup, true);
                }
            }
            drop(core);
            drop(active);
            if !c.same(owner) {
                c.inventory.invalid.store(true, Ordering::SeqCst);
            }
        }
        pub(super) struct Controls {
            _core: Option<Rc<ICoreWebView2>>,
            _environment: Option<Rc<ICoreWebView2Environment>>,
            _core22: Option<Rc<ICoreWebView2_22>>,
            _receivers: Vec<Receiver>,
        }
        pub(super) fn take_controls(i: &mut Instance) -> Controls {
            if let Some(s) = i.capability.as_mut() {
                Controls {
                    _core: s.core.take(),
                    _environment: s.environment.take(),
                    _core22: s.core22.take(),
                    _receivers: std::mem::take(&mut s.receivers),
                }
            } else {
                Controls {
                    _core: None,
                    _environment: None,
                    _core22: None,
                    _receivers: Vec::new(),
                }
            }
        }
        pub(super) fn quiescent(i: &Instance) -> bool {
            i.capability.as_ref().is_none_or(|s| {
                s.core.is_none()
                    && s.environment.is_none()
                    && s.core22.is_none()
                    && s.receivers.is_empty()
                    && s.response.is_none()
                    && s.script_response.is_none()
                    && s.inventory.drained()
                    && s.inventory.handlers.load(Ordering::SeqCst) == 0
                    && s.inventory.raw.load(Ordering::SeqCst) == 0
                    && s.held.is_none()
                    && !s.cleanup_unknown
            })
        }
        pub(super) fn publish_final(owner: &Rc<RefCell<Owner>>) {
            if !selected(owner) {
                return;
            }
            {
                let mut o = owner.borrow_mut();
                let i = o.instance.as_mut().unwrap();
                let released = i.released && inventory_empty(i);
                let s = i.capability.as_mut().unwrap();
                revoke(s);
                s.phase = Phase::Final;
                s.facts.normal_released = released;
                let healthy = s.facts.method_submissions == 7
                    && s.facts.method_acks == 7
                    && s.facts.receiver_adds == 7
                    && s.facts.receiver_removes == 7
                    && s.facts.script_ack
                    && s.facts.script_removed
                    && s.facts.initial_native_join
                    && s.facts.root_metadata_joined
                    && s.facts.canonical_ready
                    && s.facts.parsed_after_complete
                    && s.facts.candidate_held
                    && s.facts.candidate_delivery_attempts == 1
                    && s.facts.restoration_flags == 7
                    && released;
                if healthy
                    && s.selector == Selector::DisposeMissing
                    && s.missing_disposition
                    && s.facts.failure == Some(Failure::DetachReceiptMissing)
                    && s.facts.candidate_delivery_outcome == Delivery::RejectedRevokedPermit
                    && s.facts.candidate_applied == 0
                    && s.facts.dispose_witness_count == 1
                    && s.facts.dispose_receipt_count == 0
                    && s.facts.receipt_deadline_observed
                    && s.facts.failed_close_no_activation
                    && !s.facts.stop_after_dispose_receipt
                {
                    s.facts.result = ResultKind::ExpectedMissingReceipt;
                } else if healthy
                    && s.selector != Selector::DisposeMissing
                    && mutation_trace(s)
                    && s.facts.native_source_unchanged == Some(true)
                    && s.facts.dispose_receipt_count == 1
                    && s.facts.dispose_witness_count == 0
                    && s.facts.stop_after_dispose_receipt
                    && s.facts.failure == Some(Failure::CapabilityApprovalMissing)
                    && s.facts.permit_revoked_at_after_entry == Some(false)
                    && s.facts.candidate_delivery_outcome == Delivery::Applied
                    && s.facts.candidate_applied == 1
                {
                    s.facts.result = ResultKind::BaselineRefused;
                } else if healthy
                    && s.selector != Selector::DisposeMissing
                    && mutation_trace(s)
                    && s.facts.native_source_unchanged == Some(true)
                    && s.facts.dispose_receipt_count == 1
                    && s.facts.dispose_witness_count == 0
                    && s.facts.stop_after_dispose_receipt
                    && s.facts.failure.is_none()
                    && s.facts.permit_revoked_at_after_entry == Some(true)
                    && s.facts.candidate_delivery_outcome == Delivery::RejectedRevokedPermit
                    && s.facts.candidate_applied == 0
                {
                    s.facts.result = ResultKind::FiniteObserved;
                } else {
                    s.facts.result = ResultKind::NotEstablished;
                    if s.facts.failure.is_none() {
                        s.facts.failure = Some(Failure::NativeCleanup);
                    }
                }
            }
            emit(owner, Stage::Final);
        }
        #[cfg(test)]
        mod tests {
            use super::*;
            // A consumer that drops an item or reconstructs a fresh permit must fail.
            #[test]
            fn same_held_candidate_is_applied_live_and_rejected_after_sticky_revoke() {
                let identity =
                    wire::ProtocolIdentity::new("nonce", "document", wire::Selector::OpenMain)
                        .unwrap();
                let probe = wire::parse_probe(br#"{"p":"s4c1","n":"nonce","c":"s4-capability-open-main","s":2,"k":"candidate","f":0}"#, &identity).unwrap();
                let permit = Rc::new(Permit::new(17));
                let held = Held {
                    probe: probe.clone(),
                    permit: Rc::clone(&permit),
                    generation: 17,
                };
                assert_eq!(permit.applied.get(), 0);
                assert_eq!(
                    consume_candidate(held, &permit, 17, None),
                    Delivery::Applied
                );
                assert_eq!(permit.applied.get(), 1);
                permit.revoked.set(true);
                let held = Held {
                    probe,
                    permit: Rc::clone(&permit),
                    generation: 17,
                };
                assert_eq!(
                    consume_candidate(held, &permit, 17, None),
                    Delivery::RejectedRevokedPermit
                );
                assert!(permit.revoked.get());
            }
            // A stale generation or alternate Rc may never count as revoked rejection.
            #[test]
            fn consumer_checks_immutable_identity_before_applying_or_rejecting() {
                let identity =
                    wire::ProtocolIdentity::new("nonce", "document", wire::Selector::OpenMain)
                        .unwrap();
                let probe = wire::parse_probe(br#"{"p":"s4c1","n":"nonce","c":"s4-capability-open-main","s":2,"k":"candidate","f":0}"#, &identity).unwrap();
                let permit = Rc::new(Permit::new(17));
                let other = Rc::new(Permit::new(17));
                let held = Held {
                    probe,
                    permit: Rc::clone(&permit),
                    generation: 16,
                };
                assert_eq!(consume_candidate(held, &other, 17, None), Delivery::Failed);
            }
            // Witness is deliberately insufficient and the exact 2s boundary is late.
            #[test]
            fn receipt_deadline_is_strict_and_does_not_reset_original_close() {
                let close = Instant::now();
                assert!(receipt_timely(close + Duration::from_millis(1999), close));
                assert!(!receipt_timely(close + Duration::from_secs(2), close));
                assert!(!receipt_timely(close + Duration::from_secs(10), close));
            }
            // Handler destruction cannot replace a genuine method completion.
            #[test]
            fn completion_obligations_require_issue_invoke_and_consume() {
                let inventory = Inventory::new();
                assert!(inventory.issue(3));
                assert!(!inventory.drained());
                assert!(inventory.invoke(3));
                assert!(!inventory.drained());
                assert!(inventory.consume(3));
                assert!(inventory.drained());
                assert!(!inventory.invoke(3));
                assert!(inventory.invalid.load(Ordering::SeqCst));
            }
        }
        #[cfg(test)]
        mod revision1_tests {
            use super::super::super::Code0WindowRequest;
            use super::*;
            use crate::platform::windows::code0_web_session::Code0WindowMode;

            fn authority() -> CurrentAuthority {
                CurrentAuthority {
                    identity_matches: true,
                    failed: false,
                    closing: false,
                    destroyed: false,
                    released: false,
                    cleanup_blocked: false,
                    affinity_failed: false,
                    refusal_unknown: false,
                    inventory_invalid: false,
                    cleanup_unknown: false,
                    core_present: true,
                    installation_live: true,
                }
            }
            // Replacing the original-end use check with stored-key equality must fail.
            #[test]
            fn held_handoff_uses_original_clock_before_at_and_after_deadline() {
                let started = Instant::now();
                for (offset, want) in [
                    (Duration::from_millis(44999), Delivery::Applied),
                    (Duration::from_secs(45), Delivery::Failed),
                    (Duration::from_secs(46), Delivery::Failed),
                ] {
                    let mut s = state(started);
                    let original = Rc::clone(s.permit.as_ref().unwrap());
                    let held = s.held.as_ref().unwrap();
                    assert!(held.permit.binding.is_some());
                    assert!(Rc::ptr_eq(&held.permit, &original));
                    let handoff = take_handoff_at(&mut s, authority(), started + offset)
                        .unwrap()
                        .unwrap();
                    let consumed = handoff.consume_at(started + offset);
                    assert_eq!(consumed.outcome, want);
                    record_consumption(&mut s, &consumed);
                    assert_eq!(s.facts.candidate_delivery_attempts, 1);
                    assert_eq!(original.applied.get(), u8::from(want == Delivery::Applied));
                    assert!(s.held.is_none());
                    assert!(
                        take_handoff_at(&mut s, authority(), started + offset)
                            .unwrap()
                            .is_none()
                    );
                    if want == Delivery::Failed {
                        assert!(s.terminal);
                        assert!(original.revoked.get());
                        assert_eq!(s.facts.result, ResultKind::NotEstablished);
                        assert!(s.facts.failure.is_some());
                    } else {
                        assert!(!s.terminal);
                        assert_eq!(s.facts.failure, Some(Failure::CapabilityApprovalMissing));
                    }
                }
            }
            // A valid capture may cross the deadline before the actual outside-borrow use.
            #[test]
            fn handoff_rechecks_clock_at_actual_consumer_application() {
                let started = Instant::now();
                let mut s = state(started);
                let permit = Rc::clone(s.permit.as_ref().unwrap());
                let handoff =
                    take_handoff_at(&mut s, authority(), started + Duration::from_millis(44999))
                        .unwrap()
                        .unwrap();
                let consumed = handoff.consume_at(started + Duration::from_secs(45));
                record_consumption(&mut s, &consumed);
                assert_eq!(consumed.outcome, Delivery::Failed);
                assert_eq!(permit.applied.get(), 0);
                assert!(permit.revoked.get());
                assert_eq!(s.facts.candidate_delivery_attempts, 1);
                assert!(s.terminal);
            }
            // Current identity/invalid/unknown/failed state must qualify the real held item.
            #[test]
            fn due_handoff_consumes_once_without_application_under_invalid_authority() {
                let started = Instant::now();
                for index in 0..15 {
                    let mut s = state(started);
                    let mut current = authority();
                    match index {
                        0 => current.identity_matches = false,
                        1 => current.inventory_invalid = true,
                        2 => current.refusal_unknown = true,
                        3 => current.cleanup_unknown = true,
                        4 => current.failed = true,
                        5 => current.closing = true,
                        6 => current.affinity_failed = true,
                        7 => current.cleanup_blocked = true,
                        8 => current.destroyed = true,
                        9 => current.core_present = false,
                        10 => current.installation_live = false,
                        11 => current.released = true,
                        12 => s.generation = 18,
                        13 => s.request.document_id = "different".into(),
                        _ => s.loader = Some(wire::Identity::new("different-loader").unwrap()),
                    }
                    let permit = Rc::clone(s.permit.as_ref().unwrap());
                    let handoff =
                        take_handoff_at(&mut s, current, started + Duration::from_secs(1))
                            .unwrap()
                            .unwrap();
                    let consumed = handoff.consume_at(started + Duration::from_secs(1));
                    record_consumption(&mut s, &consumed);
                    assert_eq!(consumed.outcome, Delivery::Failed);
                    assert_eq!(permit.applied.get(), 0);
                    assert!(permit.revoked.get());
                    assert_eq!(s.facts.candidate_delivery_attempts, 1);
                    assert!(s.terminal);
                    assert!(s.held.is_none());
                }
            }
            // Off-STA/native-invalid atomics can change after the UI snapshot.
            #[test]
            fn handoff_rechecks_actual_atomic_invalidation_at_outside_borrow_use() {
                let started = Instant::now();
                for index in 0..3 {
                    let mut s = state(started);
                    let permit = Rc::clone(s.permit.as_ref().unwrap());
                    let handoff =
                        take_handoff_at(&mut s, authority(), started + Duration::from_secs(1))
                            .unwrap()
                            .unwrap();
                    match index {
                        0 => s.inventory.invalid.store(true, Ordering::SeqCst),
                        1 => s.installation.store(true, Ordering::SeqCst),
                        _ => s.callbacks.affinity_failed.store(true, Ordering::SeqCst),
                    }
                    let consumed = handoff.consume_at(started + Duration::from_secs(1));
                    record_consumption(&mut s, &consumed);
                    assert_eq!(consumed.outcome, Delivery::Failed);
                    assert_eq!(permit.applied.get(), 0);
                    assert!(permit.revoked.get());
                    assert_eq!(s.facts.candidate_delivery_attempts, 1);
                    assert!(s.terminal);
                }
            }
            // The missing-receipt same-held negative uses close10, independently of45.
            #[test]
            fn revoked_close_handoff_after_original_end_keeps_same_item_and_close_bound() {
                let started = Instant::now();
                let mut detach = state(started);
                detach.selector = Selector::DisposeMissing;
                detach.phase = Phase::ClosingAwaitDispose;
                detach.closing = Some(started + Duration::from_millis(44500));
                detach.terminal = true;
                detach.permit.as_ref().unwrap().revoked.set(true);
                let mut closing_authority = authority();
                closing_authority.closing = true;
                assert!(
                    reserve_control_at(
                        &mut detach,
                        closing_authority,
                        ControlKind::Detach,
                        started + Duration::from_millis(45500)
                    )
                    .is_some()
                );
                assert!(
                    reserve_control_at(
                        &mut detach,
                        closing_authority,
                        ControlKind::Mutate,
                        started + Duration::from_millis(45500)
                    )
                    .is_none()
                );
                // Retaining the earlier missing stage at close10 must fail these assertions.
                for (take_ms, use_ms, want, consumed_failure, current_failure) in [
                    (
                        46600,
                        46600,
                        Delivery::RejectedRevokedPermit,
                        None,
                        Failure::DetachReceiptMissing,
                    ),
                    (
                        54500,
                        54500,
                        Delivery::Failed,
                        Some(Failure::NativeCleanup),
                        Failure::NativeCleanup,
                    ),
                    (
                        54600,
                        54600,
                        Delivery::Failed,
                        Some(Failure::NativeCleanup),
                        Failure::NativeCleanup,
                    ),
                    (
                        54499,
                        54500,
                        Delivery::Failed,
                        Some(Failure::NativeCleanup),
                        Failure::NativeCleanup,
                    ),
                ] {
                    let mut s = state(started);
                    s.selector = Selector::DisposeMissing;
                    s.phase = Phase::ClosingAwaitDispose;
                    s.closing = Some(started + Duration::from_millis(44500));
                    s.terminal = true;
                    s.missing_disposition = true;
                    s.facts.receipt_deadline_observed = true;
                    s.facts.failure = Some(Failure::DetachReceiptMissing);
                    let permit = Rc::clone(s.permit.as_ref().unwrap());
                    permit.revoked.set(true);
                    let binding = permit.binding.as_ref().unwrap();
                    assert_eq!(binding.end, started + Duration::from_secs(45));
                    assert!(Rc::ptr_eq(&s.held.as_ref().unwrap().permit, &permit));
                    let mut current = authority();
                    current.failed = true;
                    current.closing = true;
                    let take_at = started + Duration::from_millis(take_ms);
                    let use_at = started + Duration::from_millis(use_ms);
                    let handoff = take_handoff_at(&mut s, current, take_at).unwrap().unwrap();
                    assert!(Rc::ptr_eq(&handoff.permit, &permit));
                    assert!(handoff.binding.is_some());
                    let consumed = handoff.consume_at(use_at);
                    record_consumption(&mut s, &consumed);
                    assert_eq!(consumed.outcome, want);
                    assert_eq!(consumed.failure, consumed_failure);
                    assert_eq!(s.facts.failure, Some(current_failure));
                    assert_eq!(s.facts.result, ResultKind::NotEstablished);
                    assert_eq!(permit.applied.get(), 0);
                    assert_eq!(s.facts.candidate_delivery_attempts, 1);
                    assert_eq!(s.facts.candidate_delivery_outcome, want);
                    assert_eq!(s.facts.candidate_applied, 0);
                    assert!(permit.revoked.get());
                    assert!(s.missing_disposition);
                    assert!(s.facts.receipt_deadline_observed);
                    assert_eq!(s.closing, Some(started + Duration::from_millis(44500)));
                    assert!(s.held.is_none());
                    assert!(take_handoff_at(&mut s, current, use_at).unwrap().is_none());
                }
            }
            // The grant seam must reread terminal state after Source-return/local Drop reentry.
            #[test]
            fn source_return_and_drop_terminal_reentry_cannot_regrant_or_post_live_control() {
                struct NativeDrop {
                    state: Rc<RefCell<State>>,
                    current: Rc<Cell<CurrentAuthority>>,
                }
                impl Drop for NativeDrop {
                    fn drop(&mut self) {
                        let mut current = self.current.get();
                        current.failed = true;
                        current.closing = true;
                        self.current.set(current);
                        let mut s = self.state.borrow_mut();
                        terminal_failure(&mut s, Failure::NativeCleanup, false);
                        s.closing = Some(s.deadline.original_end() - Duration::from_secs(44));
                    }
                }
                let started = Instant::now();
                for on_drop in [false, true] {
                    let mut initial = state(started);
                    initial.permit = None;
                    initial.held = None;
                    initial.consumer_due = false;
                    initial.phase = Phase::AwaitInitialJoin;
                    initial.terminal = false;
                    initial.facts.failure = None;
                    initial.facts.initial_native_join = false;
                    initial.facts.root_metadata_joined = false;
                    initial.facts.candidate_held = false;
                    let shared = Rc::new(RefCell::new(initial));
                    let current = Rc::new(Cell::new(authority()));
                    let source_result = if on_drop {
                        Some(NativeDrop {
                            state: Rc::clone(&shared),
                            current: Rc::clone(&current),
                        })
                    } else {
                        let mut a = current.get();
                        a.failed = true;
                        a.closing = true;
                        current.set(a);
                        let mut s = shared.borrow_mut();
                        terminal_failure(&mut s, Failure::NativeCleanup, false);
                        s.closing = Some(started + Duration::from_secs(1));
                        None
                    };
                    drop(source_result);
                    let mut s = shared.borrow_mut();
                    let phase = s.phase;
                    assert!(!commit_join_at(
                        &mut s,
                        current.get(),
                        Phase::AwaitInitialJoin,
                        started + Duration::from_secs(1)
                    ));
                    assert!(s.permit.is_none());
                    assert_eq!(s.phase, phase);
                    assert!(!reserve_navigation_at(
                        &mut s,
                        current.get(),
                        started + Duration::from_secs(1)
                    ));
                    assert!(
                        reserve_control_at(
                            &mut s,
                            current.get(),
                            ControlKind::Prepare,
                            started + Duration::from_secs(1)
                        )
                        .is_none()
                    );
                    assert!(
                        reserve_control_at(
                            &mut s,
                            current.get(),
                            ControlKind::Mutate,
                            started + Duration::from_secs(1)
                        )
                        .is_none()
                    );
                    assert_eq!(s.controls, 0);
                    assert!(!s.grant);
                    assert!(!s.facts.initial_native_join);
                    assert!(!s.facts.root_metadata_joined);
                    // Failed pre-grant source does not fabricate a detach receipt.
                    // Its identity/close transport can still perform emergency cleanup.
                    assert!(current.get().usable());
                    assert!(NativeDialogDeadline::disposal_within_close_at(
                        started + Duration::from_secs(1),
                        started + Duration::from_secs(1),
                        s.closing
                    ));
                    assert_eq!(s.phase, Phase::ClosingAwaitDispose);
                }
            }
            // Clear must invalidate official metadata before a permit, without evict/rearm.
            #[test]
            fn adopted_root_clear_is_terminal_even_before_permit_and_blank_clear_is_distinct() {
                let started = Instant::now();
                let mut s = state(started);
                s.permit = None;
                s.held = None;
                s.consumer_due = false;
                s.phase = Phase::AwaitInitialJoin;
                s.facts.failure = None;
                assert!(s.root_context.is_some());
                assert!(s.unique.is_some());
                assert!(!observe_contexts_cleared(&mut s));
                assert!(s.terminal);
                assert!(!commit_join_at(
                    &mut s,
                    authority(),
                    Phase::AwaitInitialJoin,
                    started + Duration::from_secs(1)
                ));
                assert!(s.permit.is_none());
                assert_ne!(s.phase, Phase::AwaitParsed);
                let mut blank = state(started);
                blank.permit = None;
                blank.held = None;
                blank.consumer_due = false;
                blank.root_context = None;
                blank.unique = None;
                blank.phase = Phase::AwaitInitialJoin;
                blank.facts.failure = None;
                blank.facts.initial_native_join = false;
                blank.facts.root_metadata_joined = false;
                blank.contexts = wire::ContextTable::new();
                blank.contexts.insert(wire::parse_context_created(br#"{"context":{"id":1,"uniqueId":"blank-context","origin":"://","name":"","auxData":{"frameId":"root","isDefault":true,"type":"default"}}}"#).unwrap()).unwrap();
                assert!(observe_contexts_cleared(&mut blank));
                assert!(!blank.terminal);
                let created=wire::parse_context_created(br#"{"context":{"id":4,"uniqueId":"unique-root","origin":"https://console.code0.ai","name":"","auxData":{"frameId":"root","isDefault":true,"type":"default"}}}"#).unwrap();
                assert!(observe_context_created_at(
                    &mut blank,
                    created,
                    authority(),
                    started + Duration::from_secs(1)
                ));
                assert!(commit_join_at(
                    &mut blank,
                    authority(),
                    Phase::AwaitInitialJoin,
                    started + Duration::from_secs(1)
                ));
                assert!(blank.permit.is_some());
                assert_eq!(blank.phase, Phase::AwaitParsed);
            }
            // Timely RED marker is not a terminal native failure or pre-revoke shortcut.
            #[test]
            fn timely_missing_hook_marker_preserves_real_application_and_live_control() {
                let started = Instant::now();
                let mut s = state(started);
                let handoff =
                    take_handoff_at(&mut s, authority(), started + Duration::from_secs(1))
                        .unwrap()
                        .unwrap();
                let consumed = handoff.consume_at(started + Duration::from_secs(1));
                record_consumption(&mut s, &consumed);
                assert_eq!(consumed.outcome, Delivery::Applied);
                assert_eq!(s.facts.candidate_applied, 1);
                assert!(!s.terminal);
                assert_eq!(s.facts.failure, Some(Failure::CapabilityApprovalMissing));
                s.phase = Phase::AwaitCandidate;
                s.mutate_sent = false;
                s.controls = 1;
                assert!(
                    reserve_control_at(
                        &mut s,
                        authority(),
                        ControlKind::Mutate,
                        started + Duration::from_secs(1)
                    )
                    .is_some()
                );
                assert_eq!(s.phase, Phase::AwaitMutationTrace);
            }
            fn green_message(
                s: &State,
                context: u32,
                kind: &str,
                sequence: u8,
                flags: u8,
            ) -> wire::BindingMessage {
                let payload = if kind == "ready" {
                    serde_json::json!({"version":1,"nonce":"nonce","document":"document","sequence":sequence,"kind":"ready"})
                } else {
                    serde_json::json!({"p":"s4c1","n":"nonce","c":s.selector.name(),"s":sequence,"k":kind,"f":flags})
                }
                .to_string();
                let bytes = serde_json::to_vec(&serde_json::json!({"name":"__aiMeterS4Capability","executionContextId":context,"payload":payload})).unwrap();
                wire::parse_binding(&bytes, &s.identity).unwrap()
            }
            fn green_frame(child: bool) -> wire::Frame {
                let bytes: &[u8] = if child {
                    br#"{"frame":{"id":"child","parentId":"root","loaderId":"child-loader","url":"https://console.code0.ai/__ai_meter_fixture__/s4-capability-child","securityOrigin":"https://console.code0.ai","mimeType":"text/html"}}"#
                } else {
                    br#"{"frame":{"id":"root","loaderId":"loader","url":"https://console.code0.ai/__ai_meter_fixture__/s4-capability-main","securityOrigin":"https://console.code0.ai","mimeType":"text/html"}}"#
                };
                wire::parse_frame(bytes).unwrap()
            }
            fn green_armed_state(started: Instant, selector: Selector) -> State {
                let mut s = state(started);
                s.selector = selector;
                s.identity = wire::ProtocolIdentity::new(
                    "nonce",
                    "document",
                    wire::Selector::parse(selector.name()).unwrap(),
                )
                .unwrap();
                s.phase = Phase::AwaitParsed;
                s.prepare_sent = false;
                s.mutate_sent = false;
                s.held = None;
                s.consumer_due = false;
                s.facts = Facts::new(selector);
                s.facts.initial_native_join = true;
                s.facts.root_metadata_joined = true;
                let ready = green_message(&s, 4, "ready", 1, 0);
                assert!(apply_binding(&mut s, ready, (9, false)));
                let parsed = green_message(&s, 4, "parsed", 1, 0);
                assert!(apply_binding(&mut s, parsed, (10, false)));
                assert!(
                    reserve_control_at(
                        &mut s,
                        authority(),
                        ControlKind::Prepare,
                        started + Duration::from_secs(1)
                    )
                    .is_some()
                );
                if selector == Selector::OpenChildPrototype {
                    s.contexts.insert(wire::parse_context_created(br#"{"context":{"id":8,"uniqueId":"unique-child","origin":"https://console.code0.ai","name":"","auxData":{"frameId":"child","isDefault":true,"type":"default"}}}"#).unwrap()).unwrap();
                    let child = green_message(&s, 8, "childProbe", 1, 0);
                    assert!(apply_binding(&mut s, child, (11, false)));
                    assert_eq!(s.facts.child_probe_refused, Some(true));
                }
                let candidate = green_message(&s, 4, "candidate", 2, 0);
                assert!(apply_binding(&mut s, candidate, (12, false)));
                assert!(
                    reserve_control_at(
                        &mut s,
                        authority(),
                        ControlKind::Mutate,
                        started + Duration::from_secs(1)
                    )
                    .is_some()
                );
                let before = green_message(&s, 4, "beforeOpen", 3, 0);
                assert!(apply_binding(&mut s, before, (16, false)));
                s
            }
            fn green_consume_after_later(s: &mut State, now: Instant) -> Consumed {
                let permit = Rc::clone(s.permit.as_ref().unwrap());
                let entry_revoked = permit.revoked.get();
                let after = green_message(s, 4, "afterOpen", 4, 31);
                assert!(apply_binding(s, after, (18, entry_revoked)));
                let later = green_message(s, 4, "later", 5, 0);
                assert!(apply_binding(s, later, (19, permit.revoked.get())));
                let handoff = take_handoff_at(s, authority(), now).unwrap().unwrap();
                assert!(Rc::ptr_eq(&handoff.permit, &permit));
                assert!(handoff.binding.is_some());
                let consumed = handoff.consume_at(now);
                record_consumption(s, &consumed);
                assert!(s.held.is_none());
                assert!(take_handoff_at(s, authority(), now).unwrap().is_none());
                consumed
            }
            // Removing the actual root observer's revoke must change this to Applied1.
            #[test]
            fn parsed_root_document_open_revokes_same_held_candidate_before_after_entry() {
                let started = Instant::now();
                for selector in [
                    Selector::OpenMain,
                    Selector::WriteMain,
                    Selector::OpenChildPrototype,
                ] {
                    let mut s = green_armed_state(started, selector);
                    let permit = Rc::clone(s.permit.as_ref().unwrap());
                    let binding = permit.binding.clone();
                    let generation = s.generation;
                    assert!(binding.is_some());
                    assert_eq!(
                        binding.as_ref().unwrap().end,
                        started + Duration::from_secs(45)
                    );
                    assert!(Rc::ptr_eq(&s.held.as_ref().unwrap().permit, &permit));
                    assert_eq!(
                        observe_document_opened(&mut s, green_frame(false), 17),
                        None
                    );
                    assert!(permit.revoked.get());
                    assert!(Rc::ptr_eq(s.permit.as_ref().unwrap(), &permit));
                    assert!(Rc::ptr_eq(&s.held.as_ref().unwrap().permit, &permit));
                    assert_eq!(s.generation, generation);
                    assert_eq!(s.held.as_ref().unwrap().generation, generation);
                    assert!(permit.binding == binding);
                    assert_eq!(s.phase, Phase::AwaitMutationTrace);
                    assert!(!s.terminal);
                    assert!(s.closing.is_none());
                    assert_eq!(s.facts.failure, None);
                    assert_eq!(s.facts.document_opened_ordinal, Some(17));
                    assert_eq!(s.facts.root_loader_matched, Some(true));
                    let consumed =
                        green_consume_after_later(&mut s, started + Duration::from_secs(2));
                    assert_eq!(consumed.outcome, Delivery::RejectedRevokedPermit);
                    assert_eq!(consumed.failure, None);
                    assert_eq!(consumed.applied, 0);
                    assert_eq!(permit.applied.get(), 0);
                    assert!(permit.revoked.get());
                    assert_eq!(s.facts.permit_revoked_at_after_entry, Some(true));
                    assert_eq!(s.facts.candidate_delivery_attempts, 1);
                    assert_eq!(
                        s.facts.candidate_delivery_outcome,
                        Delivery::RejectedRevokedPermit
                    );
                    assert_eq!(s.facts.candidate_applied, 0);
                    assert!(mutation_trace(&s));
                    assert!(!s.terminal);
                    assert!(s.closing.is_none());
                    assert_eq!(s.facts.result, ResultKind::NotEstablished);
                }
            }
            // A child-only event or omitting the root hook cannot fake a revoked rejection.
            #[test]
            fn no_root_hook_and_child_only_document_open_leave_actual_application_live() {
                let started = Instant::now();
                for child_only in [false, true] {
                    let mut s = green_armed_state(started, Selector::OpenChildPrototype);
                    let permit = Rc::clone(s.permit.as_ref().unwrap());
                    if child_only {
                        assert_eq!(observe_document_opened(&mut s, green_frame(true), 17), None);
                    }
                    assert!(!permit.revoked.get());
                    assert_eq!(s.facts.document_opened_ordinal, None);
                    let consumed =
                        green_consume_after_later(&mut s, started + Duration::from_secs(2));
                    assert_eq!(consumed.outcome, Delivery::Applied);
                    assert_eq!(consumed.failure, None);
                    assert_eq!(permit.applied.get(), 1);
                    assert_eq!(s.facts.candidate_delivery_attempts, 1);
                    assert_eq!(s.facts.candidate_applied, 1);
                    assert_eq!(s.facts.permit_revoked_at_after_entry, Some(false));
                    assert_eq!(s.facts.failure, None);
                    assert!(!mutation_trace(&s));
                    assert!(!s.terminal);
                    assert!(s.closing.is_none());
                }
            }
            // Removing phase/before/root-shape/duplicate rejection must fail closed here.
            #[test]
            fn invalid_and_duplicate_root_document_open_cannot_regrant_or_apply() {
                let started = Instant::now();
                for invalid in 0..4 {
                    let mut s = green_armed_state(started, Selector::OpenMain);
                    let permit = Rc::clone(s.permit.as_ref().unwrap());
                    let mut frame = green_frame(false);
                    match invalid {
                        0 => s.phase = Phase::AwaitCandidate,
                        1 => s.facts.before_ordinal = None,
                        2 => frame.url = wire::FrameUrl::Other,
                        3 => assert_eq!(
                            observe_document_opened(&mut s, green_frame(false), 17),
                            None
                        ),
                        _ => unreachable!(),
                    }
                    let failure = observe_document_opened(&mut s, frame, 20).unwrap();
                    assert_eq!(failure, Failure::StimulusNotObserved);
                    // Native receive routes this returned failure through reject/terminal_failure.
                    terminal_failure(&mut s, failure, false);
                    assert!(s.terminal);
                    assert_eq!(s.phase, Phase::ClosingAwaitDispose);
                    assert!(permit.revoked.get());
                    assert_eq!(permit.applied.get(), 0);
                    assert_eq!(s.facts.failure, Some(Failure::StimulusNotObserved));
                    assert!(s.held.is_none());
                    assert!(!s.grant);
                    assert!(!commit_join_at(
                        &mut s,
                        authority(),
                        Phase::AwaitInitialJoin,
                        started + Duration::from_secs(2)
                    ));
                    assert!(Rc::ptr_eq(s.permit.as_ref().unwrap(), &permit));
                }
            }
            fn state(started: Instant) -> State {
                let request = Code0WindowRequest {
                    instance_id: "instance".into(),
                    connection_epoch: "epoch".into(),
                    nonce: "nonce".into(),
                    document_id: "document".into(),
                    mode: Code0WindowMode::Sync,
                };
                let selector = Selector::OpenMain;
                let identity =
                    wire::ProtocolIdentity::new("nonce", "document", wire::Selector::OpenMain)
                        .unwrap();
                let deadline = NativeDialogDeadline::from_attempt_started(started).unwrap();
                let mut s = State {
                    request: request.clone(),
                    generation: 17,
                    installation: Arc::new(AtomicBool::new(false)),
                    inventory: Arc::new(Inventory::new()),
                    callbacks: Arc::new(Callbacks::default()),
                    core: None,
                    environment: None,
                    core22: None,
                    selector,
                    identity,
                    deadline,
                    phase: Phase::Installing,
                    terminal: false,
                    closing: None,
                    close_published: false,
                    disposition_published: false,
                    receivers: Vec::with_capacity(7),
                    native_tokens: [None; 4],
                    native_attempted: [false; 4],
                    filter: false,
                    filter_attempted: false,
                    script_id: None,
                    script_attempted: false,
                    script_submitted: false,
                    script_response: None,
                    next: 0,
                    in_flight: false,
                    response: None,
                    acknowledged: [false; 7],
                    submitted: [false; 7],
                    setup_ready: false,
                    navigation_issued: false,
                    navigation_returned: false,
                    grant: false,
                    navigation_id: None,
                    starts: 0,
                    loading: 0,
                    root: None,
                    loader: None,
                    root_context: None,
                    unique: None,
                    contexts: wire::ContextTable::new(),
                    staged: wire::StagedMessages::new(),
                    staged_entries: std::collections::VecDeque::with_capacity(10),
                    sequence: wire::ProbeSequence::new(),
                    binding_count: 0,
                    trace: [None; 64],
                    ordinal: 0,
                    permit: None,
                    held: None,
                    consumer_due: false,
                    controls: 0,
                    prepare_sent: false,
                    mutate_sent: false,
                    detach_sent: false,
                    receipt_consumed: false,
                    missing_disposition: false,
                    disposed_at: None,
                    witness: false,
                    request_count: 0,
                    asset_bytes: 0,
                    main_response: false,
                    child_response: false,
                    cleanup_unknown: false,
                    facts: Facts::new(selector),
                };
                s.phase = Phase::AwaitMutationTrace;
                s.navigation_issued = true;
                s.navigation_returned = true;
                s.starts = 1;
                s.loading = 1;
                s.main_response = true;
                s.navigation_id = Some(8);
                s.root = Some(wire::Identity::new("root").unwrap());
                s.loader = Some(wire::Identity::new("loader").unwrap());
                s.root_context = Some(wire::ContextId::new(4).unwrap());
                s.unique = Some(wire::Identity::new("unique-root").unwrap());
                s.contexts.insert(wire::parse_context_created(br#"{"context":{"id":4,"uniqueId":"unique-root","origin":"https://console.code0.ai","name":"","auxData":{"frameId":"root","isDefault":true,"type":"default"}}}"#).unwrap()).unwrap();
                let key = current_binding(&s);
                let permit = Rc::new(Permit {
                    generation: 17,
                    revoked: Cell::new(false),
                    applied: Cell::new(0),
                    binding: key,
                });
                let probe=wire::parse_probe(br#"{"p":"s4c1","n":"nonce","c":"s4-capability-open-main","s":2,"k":"candidate","f":0}"#,&s.identity).unwrap();
                s.held = Some(Held {
                    probe,
                    permit: Rc::clone(&permit),
                    generation: 17,
                });
                s.permit = Some(permit);
                s.consumer_due = true;
                s.facts.candidate_held = true;
                s.facts.failure = Some(Failure::CapabilityApprovalMissing);
                s.facts.initial_native_join = true;
                s.facts.root_metadata_joined = true;
                s.prepare_sent = true;
                s.mutate_sent = true;
                s
            }
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
                    || i.chooser.is_some()
                    || i.capability.is_some()
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
                    || i.chooser.is_some()
                    || i.capability.is_some()
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
                    let result = if callback_on_ui(ui, &callbacks, 7190) {
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
            let handler = DevToolsProtocolEventReceivedEventHandler::create(Box::new(
                move |sender, args| {
                    let _lifetime = &lifetime;
                    let active = Active::enter(&callbacks);
                    if callback_on_ui(ui, &callbacks, 7290)
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
                            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                            n4_note(&i.callbacks, crate::platform::windows::code0_webview::fixture::N4Cause::Revocation, 7302, 0);
                            i.closing = true;
                        } else {
                            state.unmatched = true;
                        }
                    }
                    drop(args);
                    drop(sender);
                    drop(active);
                    Ok(())
                },
            ));
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
                                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                                    n4_note(&i.callbacks, crate::platform::windows::code0_webview::fixture::N4Cause::Revocation, 7746, 0);
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
                                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                                    n4_note(&i.callbacks, crate::platform::windows::code0_webview::fixture::N4Cause::Revocation, 7788, 0);
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
                                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                                    n4_note(&i.callbacks, crate::platform::windows::code0_webview::fixture::N4Cause::Revocation, 7841, 0);
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
                                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                                    n4_note(&i.callbacks, crate::platform::windows::code0_webview::fixture::N4Cause::Revocation, 7877, 0);
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
                    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                    n4_note(
                        &i.callbacks,
                        crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                        7892,
                        0,
                    );
                    i.closing = true;
                    if mailbox.unknown.load(Ordering::SeqCst) {
                        if let Some(diagnostic) = i.callbacks.diagnostics.as_ref() {
                            diagnostic.uncertainty(DU::cleanup(
                                US::MailboxUnknown,
                                UR::Unknown,
                                None,
                            ));
                        }
                        #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                        n4_note(
                            &i.callbacks,
                            crate::platform::windows::code0_webview::fixture::N4Cause::CleanupBlock,
                            7901,
                            0,
                        );
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
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                n4_note(
                    &i.callbacks,
                    crate::platform::windows::code0_webview::fixture::N4Cause::CleanupBlock,
                    7930,
                    0,
                );
                i.cleanup_blocked = true;
                i.failed = true;
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                n4_note(
                    &i.callbacks,
                    crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                    7932,
                    0,
                );
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
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                n4_note(
                    &i.callbacks,
                    crate::platform::windows::code0_webview::fixture::N4Cause::Revocation,
                    8175,
                    0,
                );
                i.closing = true;
                if let Some(diagnostic) = i.callbacks.diagnostics.as_ref() {
                    diagnostic.uncertainty(detail);
                }
                #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
                n4_note(
                    &i.callbacks,
                    crate::platform::windows::code0_webview::fixture::N4Cause::CleanupBlock,
                    8179,
                    0,
                );
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
                || instance.capability.is_some()
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
        // N4 diagnostic begin: read-only absolute deadline, never calls expiry gate.
        pub(super) fn n4_deadline_time(&self, diagnostics: &Diagnostics) -> u64 {
            diagnostics.n4_time(self.expires_at)
        }
        pub(super) fn n4_expired(&self) -> bool {
            self.expired.get()
        }
        // N4 diagnostic end.
        pub fn from_attempt_started(started: std::time::Instant) -> Option<Self> {
            Some(Self {
                expires_at: started.checked_add(std::time::Duration::from_secs(45))?,
                expired: std::cell::Cell::new(false),
            })
        }
        pub(super) fn original_end(&self) -> std::time::Instant {
            self.expires_at
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
        // N4 scalar read only; never installs or resets a close timestamp.
        pub(super) fn n4_first_close(&self) -> Option<std::time::Instant> {
            self.first
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

    pub(super) fn download_result_units(units: &[u16]) -> (bool, bool) {
        let valid =
            units.len() < 64 && char::decode_utf16(units.iter().copied()).all(|v| v.is_ok());
        (
            valid,
            valid
                && units
                    .iter()
                    .copied()
                    .eq("\"code0-download-submitted-v1\"".encode_utf16()),
        )
    }

    pub(super) fn popup_result_units(units: &[u16]) -> (bool, bool) {
        let kind = popup_cdp_result_kind(units);
        // Bounded-valid now means a complete closed primitive method-result
        // envelope. A precondition envelope is valid but never scheduling success.
        (matches!(kind, 1 | 2), kind == 1)
    }

    // This finite grammar accepts no arbitrary JSON tree, string escapes or
    // remote object handle. Inspect raw keys before any map can erase duplicates.
    fn popup_cdp_result_kind(units: &[u16]) -> u8 {
        if units.len() >= 2048 {
            return 5;
        }
        let mut text = String::with_capacity(2048);
        for decoded in char::decode_utf16(units.iter().copied()) {
            let Ok(character) = decoded else {
                return 4;
            };
            if text.len() + character.len_utf8() > 2048 {
                return 5;
            }
            text.push(character);
        }
        fn take(bytes: &[u8], at: &mut usize, literal: &[u8]) -> bool {
            while bytes
                .get(*at)
                .is_some_and(|&b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
            {
                *at += 1;
            }
            if bytes[*at..].starts_with(literal) {
                *at += literal.len();
                true
            } else {
                false
            }
        }
        fn quoted<'a>(bytes: &'a [u8], at: &mut usize) -> Option<&'a [u8]> {
            if !take(bytes, at, b"\"") {
                return None;
            }
            let start = *at;
            while let Some(&byte) = bytes.get(*at) {
                match byte {
                    b'"' => {
                        let value = &bytes[start..*at];
                        *at += 1;
                        return Some(value);
                    }
                    b'\\' | 0..=31 => return None,
                    _ => *at += 1,
                }
            }
            None
        }
        fn envelope(bytes: &[u8]) -> Option<bool> {
            let mut at = 0;
            if !take(bytes, &mut at, b"{")
                || quoted(bytes, &mut at)? != b"result"
                || !take(bytes, &mut at, b":")
                || !take(bytes, &mut at, b"{")
            {
                return None;
            }
            let mut seen = 0u8;
            let mut value = None;
            let mut description = None;
            loop {
                let key = quoted(bytes, &mut at)?;
                let bit = match key {
                    b"type" => 1,
                    b"value" => 2,
                    b"description" => 4,
                    _ => return None,
                };
                if seen & bit != 0 || !take(bytes, &mut at, b":") {
                    return None;
                }
                seen |= bit;
                let field = quoted(bytes, &mut at)?;
                match bit {
                    1 if field != b"string" => return None,
                    2 => value = Some(field),
                    4 => description = Some(field),
                    _ => (),
                }
                if take(bytes, &mut at, b",") {
                    continue;
                }
                if !take(bytes, &mut at, b"}") {
                    return None;
                }
                break;
            }
            if seen & 3 != 3
                || !take(bytes, &mut at, b"}")
                || !take(bytes, &mut at, b"")
                || at != bytes.len()
            {
                return None;
            }
            let value = value?;
            if description.is_some_and(|d| d != value) {
                return None;
            }
            match value {
                b"code0-popup-scheduled-v2" => Some(true),
                b"code0-popup-precondition-v2" => Some(false),
                _ => None,
            }
        }
        match envelope(text.as_bytes()) {
            Some(true) => 1,
            Some(false) => 2,
            None if text == "null" => 3,
            None => 4,
        }
    }

    #[cfg(test)]
    mod download_contract_tests {
        use super::download_result_units;

        // A permissive JSON/string parser would accept preflight failure,
        // a missing quoted token, malformed UTF16 or an oversized callback.
        #[test]
        fn download_completion_accepts_only_bounded_exact_json_token() {
            let exact: Vec<u16> = "\"code0-download-submitted-v1\"".encode_utf16().collect();
            assert_eq!(download_result_units(&exact), (true, true));
            for text in [
                "code0-download-submitted-v1",
                "null",
                "\"code0-download-precondition-v1\"",
                "\"code0-download-submitted-v1\" ",
                "",
            ] {
                assert_eq!(
                    download_result_units(&text.encode_utf16().collect::<Vec<_>>()),
                    (true, false)
                );
            }
            assert_eq!(download_result_units(&[0xd800]), (false, false));
            assert_eq!(download_result_units(&[65; 63]), (true, false));
            assert_eq!(download_result_units(&[65; 64]), (false, false));
        }
    }

    #[cfg(test)]
    mod popup_contract_tests {
        use super::{
            LegacyCompletionInput, LegacyRefusalEvent, NativePopupFacts, NativeRefusalDeadline,
            legacy_completion_disposition_at, popup_result_units,
        };
        use std::time::{Duration, Instant};

        // Missing closure, duplicate rejection or raw-string strictness would
        // let unrelated/error protocol results settle a real N4 completion.
        #[test]
        fn popup_completion_accepts_only_closed_primitive_cdp_envelopes() {
            for text in [
                r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2"}}"#,
                r#"{"result":{"value":"code0-popup-scheduled-v2","type":"string"}}"#,
                r#"{"result":{"description":"code0-popup-scheduled-v2","value":"code0-popup-scheduled-v2","type":"string"}}"#,
                " \r\n{ \t\"result\" : { \"type\" : \"string\", \"value\" : \"code0-popup-scheduled-v2\" } } \n",
            ] {
                assert_eq!(
                    popup_result_units(&text.encode_utf16().collect::<Vec<_>>()),
                    (true, true),
                    "{text}"
                );
            }
        }

        #[test]
        fn popup_completion_rejects_duplicate_extra_escaped_and_nonstring_results() {
            for text in [
                r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2"},"result":{"type":"string","value":"code0-popup-scheduled-v2"}}"#,
                r#"{"result":{"type":"string","type":"string","value":"code0-popup-scheduled-v2"}}"#,
                r#"{"result":{"type":"string","value":"wrong","value":"code0-popup-scheduled-v2"}}"#,
                r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2","description":"code0-popup-scheduled-v2","description":"code0-popup-scheduled-v2"}}"#,
                r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2"},"exceptionDetails":null}"#,
                r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2"},"exceptionDetails":{}}"#,
                r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2"},"error":{}}"#,
                r#"{"id":1,"result":{"type":"string","value":"code0-popup-scheduled-v2"}}"#,
                r#"{"sessionId":"x","result":{"type":"string","value":"code0-popup-scheduled-v2"}}"#,
                r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2","objectId":"x"}}"#,
                r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2","subtype":"null"}}"#,
                r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2","className":"String"}}"#,
                r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2","preview":{}}}"#,
                r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2","deepSerializedValue":{}}}"#,
                r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2","unserializableValue":"x"}}"#,
                r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2","description":"wrong"}}"#,
                r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2","description":null}}"#,
                r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2","unknown":true}}"#,
                r#"{"res\u0075lt":{"type":"string","value":"code0-popup-scheduled-v2"}}"#,
                r#"{"result":{"ty\u0070e":"string","value":"code0-popup-scheduled-v2"}}"#,
                r#"{"result":{"type":"str\u0069ng","value":"code0-popup-scheduled-v2"}}"#,
                r#"{"result":{"type":"string","value":"code0-popup-\u0073cheduled-v2"}}"#,
                r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2","description":"code0-popup-\u0073cheduled-v2"}}"#,
                r#"{"result":{"type":"string","value":"code0-popup-submitted-v1"}}"#,
                r#"{"result":{"type":"string","value":"prefix-code0-popup-scheduled-v2"}}"#,
                r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2-extra"}}"#,
                r#"{"result":{"type":"object","value":"code0-popup-scheduled-v2"}}"#,
                r#"{"result":{"type":true,"value":"code0-popup-scheduled-v2"}}"#,
                r#"{"result":{"type":"string","value":true}}"#,
                r#"{"result":{"type":"string","value":null}}"#,
                r#"{"result":{"type":"string","value":["code0-popup-scheduled-v2"]}}"#,
                r#"{"result":{"type":"string","value":{"value":"code0-popup-scheduled-v2"}}}"#,
                r#"{"result":{"value":"code0-popup-scheduled-v2"}}"#,
                r#"{"result":{"type":"string"}}"#,
                r#"{"result":null}"#,
                r#"{"result":[]}"#,
                r#"{"result":"code0-popup-scheduled-v2"}"#,
                r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2",}}"#,
                r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2"}}null"#,
                r#""code0-popup-scheduled-v2""#,
                r#""code0-popup-submitted-v1""#,
                "code0-popup-scheduled-v2",
                "code0-popup-submitted-v1",
                "\"code0-popup-submitted-v1\" ",
                "\"code0-popup-precondition-v1\"",
                "\"code0-popup-precondition-v2\"",
                "\"code0-popup-scheduled-v2\" ",
                " \"code0-popup-scheduled-v2\"",
                "\"code0-popup-scheduled-v2\"\n",
                "\"code0-popup-scheduled-v2-extra\"",
                "\"prefix-code0-popup-scheduled-v2\"",
                r#""code0-popup-\u0073cheduled-v2""#,
                "\"other\"",
                "null",
                "{}",
                "[]",
                "broken",
                "",
            ] {
                assert_eq!(
                    popup_result_units(&text.encode_utf16().collect::<Vec<_>>()),
                    (false, false),
                    "{text}"
                );
            }
            let precondition =
                r#"{"result":{"type":"string","value":"code0-popup-precondition-v2"}}"#;
            assert_eq!(
                popup_result_units(&precondition.encode_utf16().collect::<Vec<_>>()),
                (true, false)
            );
        }

        #[test]
        fn popup_completion_preserves_utf16_nul_capacity_and_utf8_byte_caps() {
            let exact = r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2"}}"#;
            let padded = format!("{exact}{}", " ".repeat(2047 - exact.len()));
            assert_eq!(
                popup_result_units(&padded.encode_utf16().collect::<Vec<_>>()),
                (true, true)
            );
            let over = format!("{padded} ");
            assert_eq!(
                popup_result_units(&over.encode_utf16().collect::<Vec<_>>()),
                (false, false)
            );
            assert_eq!(popup_result_units(&[0xd800]), (false, false));
            assert_eq!(popup_result_units(&[0xdc00]), (false, false));
            let utf8_over = vec![0x00e9; 1025];
            let real = popup_result_units(&utf8_over);
            assert_eq!(real, (false, false));
            assert_eq!(super::n4_popup_result_class(&utf8_over, real), 5);
        }

        #[test]
        fn cdp_envelope_cannot_replace_identity_hresult_or_strict_disposition() {
            let success = r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2"}}"#;
            let precondition =
                r#"{"result":{"type":"string","value":"code0-popup-precondition-v2"}}"#;
            for (identity, hresult, envelope, disposable, successful) in [
                (true, true, success, true, true),
                (false, true, success, false, false),
                (true, false, "broken", true, false),
                (true, true, "broken", false, false),
                (true, true, precondition, false, false),
            ] {
                let start = Instant::now();
                let deadline = NativeRefusalDeadline::from_attempt_started(start).unwrap();
                let (bounded, matched) =
                    popup_result_units(&envelope.encode_utf16().collect::<Vec<_>>());
                let outcome = legacy_completion_disposition_at(
                    &deadline,
                    at(start, 100),
                    at(start, 101),
                    None,
                    LegacyCompletionInput {
                        identity_valid: identity,
                        hresult_succeeded: hresult,
                        bounded_result_valid: bounded,
                        fixed_result_matched: matched,
                    },
                );
                assert!(outcome.timely);
                assert_eq!(outcome.disposable, disposable);
                assert_eq!(outcome.successful, successful);
            }
        }

        fn at(start: Instant, millis: u64) -> Instant {
            start.checked_add(Duration::from_millis(millis)).unwrap()
        }

        fn submitted() -> NativePopupFacts {
            NativePopupFacts {
                script_submissions: 1,
                settings_before_stimulus: true,
                initial_source_matched: true,
                source_after_drop_timely: true,
                script_submission_returned: true,
                script_submit_ok: true,
                script_return_timely: true,
                ..NativePopupFacts::default()
            }
        }

        // Test inputs for the real facts predicate, not a substitute event loop.
        // The actual refusal helper supplies the target's original-time gate.
        fn with_target(
            facts: NativePopupFacts,
            deadline: &NativeRefusalDeadline,
            observed: Instant,
        ) -> NativePopupFacts {
            let mut event = LegacyRefusalEvent::new();
            assert!(event.cancel_once(|| ()).is_some());
            NativePopupFacts {
                popup_events: 1,
                handled_attempts: 1,
                handled_returns: 1,
                handled_successes: 1,
                handled_readbacks: 1,
                popup_sender_matched: true,
                request_generation_live_matched: true,
                issued_before_event: true,
                target_uri_getter_succeeded: true,
                target_uri_matched: true,
                non_user_getter_succeeded: true,
                non_user: true,
                handled_readback_succeeded: true,
                native_handled_true: true,
                live_after_calls: true,
                target_after_drop_timely: event.qualify_at(deadline, observed, true),
                no_new_window_assignment: true,
                first_closing_observed: true,
                ..facts
            }
        }

        // Parse the actual CDP envelope and use the same disposition helper as the
        // native consumer; only the independent identity/HRESULT inputs are fixed.
        fn with_completion(
            facts: NativePopupFacts,
            deadline: &NativeRefusalDeadline,
            received: Instant,
            consumed: Instant,
            closing: Option<Instant>,
            token: &str,
        ) -> NativePopupFacts {
            let (bounded, matched) = popup_result_units(&token.encode_utf16().collect::<Vec<_>>());
            let outcome = legacy_completion_disposition_at(
                deadline,
                received,
                consumed,
                closing,
                LegacyCompletionInput {
                    identity_valid: true,
                    hresult_succeeded: true,
                    bounded_result_valid: bounded,
                    fixed_result_matched: matched,
                },
            );
            NativePopupFacts {
                script_completions: 1,
                completion_identity_matched: true,
                completion_hresult_succeeded: true,
                bounded_result_valid: bounded,
                fixed_result_matched: matched,
                completion_before_release: true,
                completion_within_deadline: outcome.timely,
                completion_disposition_consumed: outcome.disposable,
                completion_obligation_drained: outcome.disposable,
                completion_handlers_drained: true,
                ..facts
            }
        }

        // A scheduling receipt must never manufacture a native target. Missing
        // completion, native user activation, or a failed getter must still fail.
        #[test]
        fn scheduling_alone_missing_completion_and_user_popup_are_not_positive() {
            let start = Instant::now();
            let deadline = NativeRefusalDeadline::from_attempt_started(start).unwrap();
            let scheduling = with_completion(
                submitted(),
                &deadline,
                at(start, 100),
                at(start, 101),
                None,
                r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2"}}"#,
            );
            assert!(scheduling.fixed_result_matched && scheduling.completion_obligation_drained);
            assert!(!scheduling.target_positive());
            assert!(!scheduling.positive());
            let target = with_target(submitted(), &deadline, at(start, 200));
            assert!(target.target_positive());
            assert!(!target.positive());
            let complete = with_target(scheduling, &deadline, at(start, 200));
            assert!(complete.positive());
            let user_popup = NativePopupFacts {
                non_user: false,
                ..complete
            };
            assert!(!user_popup.target_positive());
            assert!(!user_popup.positive());
            let missing_getter = NativePopupFacts {
                non_user_getter_succeeded: false,
                ..complete
            };
            assert!(!missing_getter.target_positive());
            assert!(!missing_getter.positive());
        }

        // Requiring a target at completion consumption would break the first
        // order; accepting a target without strict completion breaks the second.
        #[test]
        fn scheduling_completion_and_target_keep_independent_evidence_in_both_orders() {
            for completion_first in [true, false] {
                let start = Instant::now();
                let deadline = NativeRefusalDeadline::from_attempt_started(start).unwrap();
                let facts = if completion_first {
                    let early = with_completion(
                        submitted(),
                        &deadline,
                        at(start, 100),
                        at(start, 101),
                        None,
                        r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2"}}"#,
                    );
                    assert!(early.completion_within_deadline && early.fixed_result_matched);
                    assert!(!early.target_positive() && !early.positive());
                    with_target(early, &deadline, at(start, 200))
                } else {
                    let early = with_target(submitted(), &deadline, at(start, 200));
                    assert!(early.target_positive());
                    assert!(!early.positive());
                    with_completion(
                        early,
                        &deadline,
                        at(start, 300),
                        at(start, 301),
                        Some(at(start, 200)),
                        r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2"}}"#,
                    )
                };
                assert!(facts.target_positive() && facts.positive());
                assert_eq!((facts.popup_events, facts.script_completions), (1, 1));
            }
        }

        // Neither a timely scheduling receipt nor a real target renews the
        // immutable attempt. Exact45 and first-close+10 are failure boundaries.
        #[test]
        fn popup_target_and_completion_reject_original45_and_close10_boundaries() {
            for target_ms in [44999, 45000, 45001] {
                let start = Instant::now();
                let deadline = NativeRefusalDeadline::from_attempt_started(start).unwrap();
                let scheduling = with_completion(
                    submitted(),
                    &deadline,
                    at(start, 100),
                    at(start, 101),
                    None,
                    r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2"}}"#,
                );
                let facts = with_target(scheduling, &deadline, at(start, target_ms));
                assert_eq!(facts.target_positive(), target_ms < 45000);
                assert_eq!(facts.positive(), target_ms < 45000);
            }
            for (close_ms, received_ms, consumed_ms, positive) in [
                (44000, 44998, 44999, true),
                (44000, 45000, 45001, false),
                (44000, 44999, 45000, false),
                (1000, 10998, 10999, true),
                (1000, 11000, 11001, false),
                (1000, 10999, 11000, false),
            ] {
                let start = Instant::now();
                let deadline = NativeRefusalDeadline::from_attempt_started(start).unwrap();
                let target = with_target(submitted(), &deadline, at(start, close_ms));
                assert!(target.target_positive());
                let facts = with_completion(
                    target,
                    &deadline,
                    at(start, received_ms),
                    at(start, consumed_ms),
                    Some(at(start, close_ms)),
                    r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2"}}"#,
                );
                assert_eq!(facts.completion_within_deadline, positive);
                assert_eq!(facts.positive(), positive);
            }
        }

        #[test]
        fn qualified_popup_cannot_replace_strict_scheduling_completion() {
            for token in [
                r#"{"result":{"type":"string","value":"code0-popup-precondition-v2"}}"#,
                "\"code0-popup-submitted-v1\"",
                "\"code0-popup-precondition-v2\"",
                "null",
                "",
            ] {
                let start = Instant::now();
                let deadline = NativeRefusalDeadline::from_attempt_started(start).unwrap();
                let target = with_target(submitted(), &deadline, at(start, 200));
                let facts = with_completion(
                    target,
                    &deadline,
                    at(start, 300),
                    at(start, 301),
                    Some(at(start, 200)),
                    token,
                );
                assert!(facts.target_positive());
                assert!(!facts.fixed_result_matched && !facts.positive());
            }
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

    /// N5 finite facts from this real download action and strict completion.
    #[derive(Clone, Copy, Default, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct NativeDownloadFacts {
        pub script_submissions: u8,
        pub download_events: u8,
        pub cancel_attempts: u8,
        pub cancel_returns: u8,
        pub cancel_successes: u8,
        pub cancel_readbacks: u8,
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
        pub download_sender_matched: bool,
        pub request_generation_live_matched: bool,
        pub issued_before_event: bool,
        pub target_uri_getter_succeeded: bool,
        pub target_uri_matched: bool,
        pub operation_getter_succeeded: bool,
        pub cancel_readback_succeeded: bool,
        pub native_cancel_true: bool,
        pub handled_readback_succeeded: bool,
        pub native_handled_true: bool,
        pub live_after_calls: bool,
        pub target_after_drop_timely: bool,
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
    impl NativeDownloadFacts {
        pub fn target_positive(&self) -> bool {
            self.download_events == 1
                && self.cancel_attempts == 1
                && self.cancel_returns == 1
                && self.cancel_successes == 1
                && self.cancel_readbacks == 1
                && self.handled_attempts == 1
                && self.handled_returns == 1
                && self.handled_successes == 1
                && self.handled_readbacks == 1
                && self.download_sender_matched
                && self.request_generation_live_matched
                && self.issued_before_event
                && self.target_uri_getter_succeeded
                && self.target_uri_matched
                && self.operation_getter_succeeded
                && self.cancel_readback_succeeded
                && self.native_cancel_true
                && self.handled_readback_succeeded
                && self.native_handled_true
                && self.live_after_calls
                && self.target_after_drop_timely
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
    pub enum NativeDownloadStage {
        Submitted,
        Refusal,
        Completion,
        Final,
    }
    pub struct NativeDownloadReceipt {
        pub(super) request: Code0WindowRequest,
        pub(super) installation: std::sync::Arc<std::sync::atomic::AtomicBool>,
        pub(super) stage: NativeDownloadStage,
        pub(super) facts: NativeDownloadFacts,
    }
    impl NativeDownloadReceipt {
        pub fn matches(
            &self,
            request: &Code0WindowRequest,
            install: &NativeRefusalInstallReceipt,
        ) -> bool {
            &self.request == request
                && install.matches(request)
                && std::sync::Arc::ptr_eq(&self.installation, &install.installation)
        }
        pub fn stage(&self) -> NativeDownloadStage {
            self.stage
        }
        pub fn facts(&self) -> NativeDownloadFacts {
            self.facts
        }
    }

    /// Four fixed source-only capability cases; never a general script executor.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum NativeCapabilitySelector {
        OpenMain,
        WriteMain,
        OpenChildPrototype,
        DisposeMissing,
    }
    impl NativeCapabilitySelector {
        pub const ALL: [Self; 4] = [
            Self::OpenMain,
            Self::WriteMain,
            Self::OpenChildPrototype,
            Self::DisposeMissing,
        ];
        pub fn name(self) -> &'static str {
            match self {
                Self::OpenMain => "s4-capability-open-main",
                Self::WriteMain => "s4-capability-write-main",
                Self::OpenChildPrototype => "s4-capability-open-child-prototype",
                Self::DisposeMissing => "s4-capability-dispose-missing",
            }
        }
        pub fn parse(name: &str) -> Option<Self> {
            Self::ALL.into_iter().find(|s| s.name() == name)
        }
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
    pub enum NativeCapabilityFailure {
        Setup,
        Metadata,
        InitialNavigation,
        Protocol,
        StimulusNotObserved,
        Ordering,
        ContextChanged,
        BoundExceeded,
        DetachReceiptMissing,
        DetachReceiptLate,
        NativeCleanup,
        CapabilityApprovalMissing,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    pub enum NativeCapabilityResult {
        FiniteObserved,
        ExpectedMissingReceipt,
        BaselineRefused,
        NotEstablished,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    pub enum NativeCapabilityDelivery {
        NotAttempted,
        Applied,
        RejectedRevokedPermit,
        Failed,
    }
    /// The exact private v1 37-key wire object. No raw identities are serialized.
    #[derive(Clone, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct NativeCapabilityFacts {
        pub version: u8,
        pub selector: &'static str,
        pub runtime_version: String,
        pub result: NativeCapabilityResult,
        pub failure: Option<NativeCapabilityFailure>,
        pub method_submissions: u8,
        pub method_acks: u8,
        pub receiver_adds: u8,
        pub receiver_removes: u8,
        pub script_ack: bool,
        pub script_removed: bool,
        pub initial_native_join: bool,
        pub root_metadata_joined: bool,
        pub canonical_ready: bool,
        pub parsed_after_complete: bool,
        pub child_probe_refused: Option<bool>,
        pub candidate_held: bool,
        pub candidate_delivery_attempts: u8,
        pub candidate_applied: u8,
        pub candidate_delivery_outcome: NativeCapabilityDelivery,
        pub before_ordinal: Option<u8>,
        pub document_opened_ordinal: Option<u8>,
        pub after_ordinal: Option<u8>,
        pub later_ordinal: Option<u8>,
        pub permit_revoked_at_after_entry: Option<bool>,
        pub root_context_retained: Option<bool>,
        pub root_loader_matched: Option<bool>,
        pub native_source_unchanged: Option<bool>,
        pub logical_reset_flags: Option<u8>,
        pub dispose_receipt_count: u8,
        pub dispose_witness_count: u8,
        pub restoration_flags: u8,
        pub receipt_deadline_observed: bool,
        pub stop_after_dispose_receipt: bool,
        pub failed_close_no_activation: bool,
        pub normal_released: bool,
        pub temp_dir_closed: bool,
    }
    impl NativeCapabilityFacts {
        pub fn new(selector: NativeCapabilitySelector) -> Self {
            let mutation = selector != NativeCapabilitySelector::DisposeMissing;
            Self {
                version: 1,
                selector: selector.name(),
                runtime_version: String::new(),
                result: NativeCapabilityResult::NotEstablished,
                failure: None,
                method_submissions: 0,
                method_acks: 0,
                receiver_adds: 0,
                receiver_removes: 0,
                script_ack: false,
                script_removed: false,
                initial_native_join: false,
                root_metadata_joined: false,
                canonical_ready: false,
                parsed_after_complete: false,
                child_probe_refused: (selector == NativeCapabilitySelector::OpenChildPrototype)
                    .then_some(false),
                candidate_held: false,
                candidate_delivery_attempts: 0,
                candidate_applied: 0,
                candidate_delivery_outcome: NativeCapabilityDelivery::NotAttempted,
                before_ordinal: None,
                document_opened_ordinal: None,
                after_ordinal: None,
                later_ordinal: None,
                permit_revoked_at_after_entry: mutation.then_some(false),
                root_context_retained: mutation.then_some(false),
                root_loader_matched: mutation.then_some(false),
                native_source_unchanged: mutation.then_some(false),
                logical_reset_flags: mutation.then_some(0),
                dispose_receipt_count: 0,
                dispose_witness_count: 0,
                restoration_flags: 0,
                receipt_deadline_observed: false,
                stop_after_dispose_receipt: false,
                failed_close_no_activation: false,
                normal_released: false,
                temp_dir_closed: false,
            }
        }
        pub fn positive_before_release(&self) -> bool {
            let common = self.version == 1
                && self.method_submissions == 7
                && self.method_acks == 7
                && self.receiver_adds == 7
                && self.receiver_removes == 7
                && self.script_ack
                && self.script_removed
                && self.initial_native_join
                && self.root_metadata_joined
                && self.canonical_ready
                && self.parsed_after_complete
                && self.candidate_held
                && self.candidate_delivery_attempts == 1
                && self.candidate_applied == 0
                && self.candidate_delivery_outcome
                    == NativeCapabilityDelivery::RejectedRevokedPermit
                && self.restoration_flags == 7;
            if self.selector == NativeCapabilitySelector::DisposeMissing.name() {
                common
                    && self.result == NativeCapabilityResult::ExpectedMissingReceipt
                    && self.failure == Some(NativeCapabilityFailure::DetachReceiptMissing)
                    && self.dispose_receipt_count == 0
                    && self.dispose_witness_count == 1
                    && self.receipt_deadline_observed
                    && self.failed_close_no_activation
                    && !self.stop_after_dispose_receipt
                    && self.before_ordinal.is_none()
                    && self.document_opened_ordinal.is_none()
                    && self.after_ordinal.is_none()
                    && self.later_ordinal.is_none()
                    && self.permit_revoked_at_after_entry.is_none()
                    && self.root_context_retained.is_none()
                    && self.root_loader_matched.is_none()
                    && self.native_source_unchanged.is_none()
                    && self.logical_reset_flags.is_none()
                    && self.child_probe_refused.is_none()
            } else {
                common
                    && self.result == NativeCapabilityResult::FiniteObserved
                    && self.failure.is_none()
                    && self
                        .before_ordinal
                        .zip(self.document_opened_ordinal)
                        .zip(self.after_ordinal)
                        .zip(self.later_ordinal)
                        .is_some_and(|(((b, d), a), l)| 0 < b && b < d && d < a && a < l && l <= 64)
                    && self.permit_revoked_at_after_entry == Some(true)
                    && self.root_context_retained == Some(true)
                    && self.root_loader_matched == Some(true)
                    && self.native_source_unchanged == Some(true)
                    && self.logical_reset_flags == Some(31)
                    && self.dispose_receipt_count == 1
                    && self.dispose_witness_count == 0
                    && !self.receipt_deadline_observed
                    && !self.failed_close_no_activation
                    && self.stop_after_dispose_receipt
                    && self.child_probe_refused
                        == if self.selector == NativeCapabilitySelector::OpenChildPrototype.name() {
                            Some(true)
                        } else {
                            None
                        }
            }
        }
    }
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum NativeCapabilityStage {
        Started,
        Disposition,
        Final,
    }
    pub struct NativeCapabilityReceipt {
        pub(super) request: Code0WindowRequest,
        pub(super) installation: std::sync::Arc<std::sync::atomic::AtomicBool>,
        pub(super) stage: NativeCapabilityStage,
        pub(super) facts: NativeCapabilityFacts,
    }
    impl NativeCapabilityReceipt {
        pub fn matches(
            &self,
            request: &Code0WindowRequest,
            install: &NativeRefusalInstallReceipt,
        ) -> bool {
            &self.request == request
                && install.matches(request)
                && std::sync::Arc::ptr_eq(&self.installation, &install.installation)
        }
        pub fn stage(&self) -> NativeCapabilityStage {
            self.stage
        }
        pub fn facts(&self) -> NativeCapabilityFacts {
            self.facts.clone()
        }
    }

    /// S3 facts are real native producer observations; scalar snapshot counts end
    /// at the trusted cancel, and do not assert event absence through Close.
    #[derive(Clone, Copy, Default, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct NativeChooserFacts {
        pub receiver_adds: [u8; 10],
        pub receiver_removes: [u8; 10],
        pub commands_issued: [u8; 11],
        pub commands_returned: [u8; 11],
        pub commands_succeeded: [u8; 11],
        pub completions: [u8; 11],
        pub consumed: [u8; 11],
        pub native_starts: u8,
        pub native_completions: u8,
        pub target_events: u8,
        pub bootstrap_delivered: u8,
        pub asset_bound: bool,
        pub initial_source: bool,
        pub settings_and_hooks: bool,
        pub navigation_issued: bool,
        pub navigation_returned: bool,
        pub navigation_succeeded: bool,
        pub navigation_qualified: bool,
        pub completed_added: bool,
        pub completed_removed: bool,
        pub completed_same_emitter: bool,
        pub fresh_document: bool,
        pub initial_default_session_observed: bool,
        pub armed_once: bool,
        pub world_joined: bool,
        pub phase1_prepared: bool,
        pub root_bound: bool,
        pub sole_input_bound: bool,
        pub backend_bound: bool,
        pub final_tree_and_source: bool,
        pub target_session_identity: bool,
        pub target_pre_stop: bool,
        pub effect_pre_stop: bool,
        pub trusted_cancel_snapshot: bool,
        pub snapshot_files_zero: bool,
        pub snapshot_value_empty: bool,
        pub snapshot_same_input_document: bool,
        pub first_closing_observed: bool,
        pub actual_stop_entry_observed: bool,
        pub observation_epoch_valid: bool,
        pub obligations_drained: bool,
        pub handlers_drained: bool,
        pub protocol_text_drained: bool,
        pub receivers_drained: bool,
    }
    impl NativeChooserFacts {
        pub fn target_positive(&self) -> bool {
            self.target_events == 1
                && self.target_session_identity
                && self.target_pre_stop
                && self.fresh_document
                && self.armed_once
                && self.world_joined
                && self.phase1_prepared
                && self.root_bound
                && self.sole_input_bound
                && self.backend_bound
                && self.final_tree_and_source
                && self.first_closing_observed
        }
        pub fn positive(&self) -> bool {
            self.receiver_adds == [1; 10]
                && self.receiver_removes == [1; 10]
                && self.commands_issued == [1; 11]
                && self.commands_returned == [1; 11]
                && self.commands_succeeded == [1; 11]
                && self.completions == [1; 11]
                && self.consumed == [1; 11]
                && self.native_starts == 1
                && self.native_completions == 1
                && self.asset_bound
                && self.initial_source
                && self.settings_and_hooks
                && self.navigation_issued
                && self.navigation_returned
                && self.navigation_succeeded
                && self.navigation_qualified
                && self.completed_added
                && self.completed_removed
                && self.completed_same_emitter
                && self.initial_default_session_observed
                && self.target_positive()
                && self.effect_pre_stop
                && self.trusted_cancel_snapshot
                && self.snapshot_files_zero
                && self.snapshot_value_empty
                && self.snapshot_same_input_document
                && self.actual_stop_entry_observed
                && self.observation_epoch_valid
                && self.obligations_drained
                && self.handlers_drained
                && self.protocol_text_drained
                && self.receivers_drained
                && (1..=16).contains(&self.bootstrap_delivered)
        }
    }
    #[cfg(test)]
    mod chooser_contract_tests {
        use super::NativeChooserFacts;

        #[test]
        fn default_or_missing_independent_chooser_producers_never_pass() {
            assert!(!NativeChooserFacts::default().positive());
            // Inert unit-test input only; runtime facts have native producers.
            let good = NativeChooserFacts {
                receiver_adds: [1; 10],
                receiver_removes: [1; 10],
                commands_issued: [1; 11],
                commands_returned: [1; 11],
                commands_succeeded: [1; 11],
                completions: [1; 11],
                consumed: [1; 11],
                native_starts: 1,
                native_completions: 1,
                target_events: 1,
                bootstrap_delivered: 1,
                asset_bound: true,
                initial_source: true,
                settings_and_hooks: true,
                navigation_issued: true,
                navigation_returned: true,
                navigation_succeeded: true,
                navigation_qualified: true,
                completed_added: true,
                completed_removed: true,
                completed_same_emitter: true,
                fresh_document: true,
                initial_default_session_observed: true,
                armed_once: true,
                world_joined: true,
                phase1_prepared: true,
                root_bound: true,
                sole_input_bound: true,
                backend_bound: true,
                final_tree_and_source: true,
                target_session_identity: true,
                target_pre_stop: true,
                effect_pre_stop: true,
                trusted_cancel_snapshot: true,
                snapshot_files_zero: true,
                snapshot_value_empty: true,
                snapshot_same_input_document: true,
                first_closing_observed: true,
                actual_stop_entry_observed: true,
                observation_epoch_valid: true,
                obligations_drained: true,
                handlers_drained: true,
                protocol_text_drained: true,
                receivers_drained: true,
            };
            assert!(good.positive());
            for bootstrap in [0, 17] {
                let mut missing = good;
                missing.bootstrap_delivered = bootstrap;
                assert!(!missing.positive());
            }
            for target_events in [0, 2] {
                let mut missing = good;
                missing.target_events = target_events;
                assert!(!missing.positive());
            }
            let mut missing = good;
            missing.effect_pre_stop = false;
            assert!(!missing.positive());
            missing = good;
            missing.snapshot_files_zero = false;
            assert!(!missing.positive());
            missing = good;
            missing.obligations_drained = false;
            assert!(!missing.positive());
            missing = good;
            missing.receiver_removes[0] = 0;
            assert!(!missing.positive());
            missing = good;
            missing.completions[10] = 0;
            assert!(!missing.positive());
        }
    }

    #[derive(Clone, Copy)]
    pub enum NativeChooserStage {
        Started,
        Target,
        Effect,
        Final,
    }
    pub struct NativeChooserReceipt {
        pub(super) request: Code0WindowRequest,
        pub(super) installation: std::sync::Arc<std::sync::atomic::AtomicBool>,
        pub(super) stage: NativeChooserStage,
        pub(super) facts: NativeChooserFacts,
    }
    impl NativeChooserReceipt {
        pub fn matches(
            &self,
            request: &Code0WindowRequest,
            install: &NativeRefusalInstallReceipt,
        ) -> bool {
            &self.request == request
                && install.matches(request)
                && std::sync::Arc::ptr_eq(&self.installation, &install.installation)
        }
        pub fn stage(&self) -> NativeChooserStage {
            self.stage
        }
        pub fn facts(&self) -> NativeChooserFacts {
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
        DownloadClosing {
            at: std::time::Instant,
        },
        Download(NativeDownloadReceipt),
        ChooserClosing {
            at: std::time::Instant,
        },
        Chooser(NativeChooserReceipt),
        CapabilityClosing {
            at: std::time::Instant,
        },
        Capability(NativeCapabilityReceipt),
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
        n3: Option<N3DiagnosticSnapshot>,
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
            if let Some(n3) = self.n3 {
                n3.emit(self.kind);
            }
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

    // N3 diagnostic begin: source-first focused tests; execution is separately owned.
    #[cfg(test)]
    mod n3_diagnostic_tests {
        use super::*;
        use std::sync::atomic::Ordering::Acquire;

        // Catches accidental cross-domain arming or records while diagnostics are off.
        #[test]
        fn n3_arming_is_opt_in_and_does_not_arm_n4() {
            let disabled = Diagnostics::default();
            disabled.arm_dialog_owned_empty_html();
            disabled.n3_writer(N3Writer::DuplicateInvocation, 0);
            assert_eq!(disabled.n3_writer_sites.load(Acquire), 0);
            let n4 = Diagnostics {
                enabled: true,
                ..Diagnostics::default()
            };
            n4.arm_popup_initial_blank();
            n4.n3_writer(N3Writer::DuplicateInvocation, 0);
            assert_eq!(n4.n3_writer_sites.load(Acquire), 0);
            let n3 = Diagnostics {
                enabled: true,
                ..Diagnostics::default()
            };
            n3.arm_dialog_owned_empty_html();
            n3.n3_writer(N3Writer::DuplicateInvocation, 0);
            assert_eq!(n3.n3_writer_sites.load(Acquire), 1 << 3);
            assert!(!n3.n4_enabled());
            assert!(n3.n4_causes.iter().all(|v| v.load(Acquire) == 0));
        }

        // Catches a repeated poll replacing the true first UI transition.
        #[test]
        fn n3_first_block_ignores_already_blocked_and_preserves_first_word() {
            let d = Diagnostics {
                enabled: true,
                ..Diagnostics::default()
            };
            d.arm_dialog_owned_empty_html();
            d.n3_block(true, N3Block::PollUnknown, 7);
            assert_eq!(d.n3_first_block.load(Acquire), 0);
            d.n3_block(false, N3Block::Dialog, 0x0201);
            d.n3_block(false, N3Block::PollUnknown, 0xffff);
            assert_eq!(d.n3_first_block.load(Acquire), 0x02010004);
            for site in [
                N3Block::NativeCleanup,
                N3Block::PollIdentity,
                N3Block::PollUnknown,
                N3Block::Dialog,
                N3Block::RegistryPropagation,
                N3Block::RefusalCallback,
            ] {
                assert!((1..=6).contains(&(site as u16)));
            }
        }

        // Catches lost contender bits; no assertion claims a global atomic winner.
        #[test]
        fn n3_writer_mask_keeps_all_closed_sites_and_first_complete_record() {
            let d = Diagnostics {
                enabled: true,
                ..Diagnostics::default()
            };
            d.arm_dialog_owned_empty_html();
            d.n3_writer(N3Writer::ContextIdentity, 0x0aff);
            let first = d.n3_first_writer.load(Acquire);
            std::thread::scope(|scope| {
                for writer in N3Writer::ALL {
                    let d = &d;
                    scope.spawn(move || d.n3_writer(writer, 0xffff));
                }
            });
            assert_eq!(d.n3_first_writer.load(Acquire), first);
            assert_eq!(first, 0x0aff0008);
            assert_ne!(first, 0x09ff0008);
            assert_eq!(d.n3_writer_sites.load(Acquire), 0x3ffe);
            assert_eq!(n4_first_word(1, 0, u64::MAX), 0xffffffff00000001);
        }

        // Catches counter truncation masquerading as empty/available inventory.
        #[test]
        fn n3_inventory_preserves_absence_and_marks_each_overflow() {
            assert_eq!(n3_inventory_words(0, [0; 5]), [0, 0]);
            assert_eq!(
                n3_inventory_words(3, [65536, 65535, 1, 256, 255]),
                [0x09000003, 0xffff0001ffffffff]
            );
            assert_eq!(
                n3_inventory_words(3, [usize::MAX; 5]),
                [0x1f000003, u64::MAX]
            );
            let copy = N4DiagnosticCopy::<2>::default();
            assert_eq!(copy.snapshot(), (1, [0, 0]));
            copy.publish(n3_inventory_words(3, [1, 1, 1, 0, 0]));
            assert_eq!(copy.snapshot(), (0, [3, 0x0000000100010001]));
        }

        // Catches loss of short-circuit availability and disposal/success distinctions.
        #[test]
        fn n3_completion_masks_distinguish_disposal_success_and_unknown_inputs() {
            let strict = N3CompletionDiagnostic {
                same: true,
                not_released: true,
                disposal_timely: true,
                invalid: Some(false),
                affinity: Some(false),
                hresult_succeeded: true,
                bounded: true,
                fixed: true,
                original_timely: true,
                valid: true,
            };
            assert_eq!(strict.bits(), 0x0faf);
            assert_eq!(
                N3CompletionDiagnostic {
                    hresult_succeeded: false,
                    bounded: false,
                    fixed: false,
                    ..strict
                }
                .bits(),
                0x0c2f
            );
            assert_eq!(
                N3CompletionDiagnostic {
                    original_timely: false,
                    ..strict
                }
                .bits(),
                0x0baf
            );
            assert_eq!(
                N3CompletionDiagnostic {
                    fixed: false,
                    valid: false,
                    ..strict
                }
                .bits(),
                0x05af
            );
            assert_eq!(
                N3CompletionDiagnostic {
                    same: false,
                    invalid: None,
                    affinity: None,
                    valid: false,
                    ..strict
                }
                .bits(),
                0x0786
            );
            assert_eq!(
                N3CompletionDiagnostic {
                    invalid: Some(true),
                    affinity: None,
                    valid: false,
                    ..strict
                }
                .bits(),
                0x079f
            );
        }
    }
    // N3 diagnostic end.

    // N4 diagnostic begin: focused helper regressions; execution is separately owned.
    #[cfg(test)]
    mod n4_diagnostic_tests {
        use super::*;
        #[test]
        fn n4_snapshot_never_accepts_mixed_concurrent_publications() {
            let copy = std::sync::Arc::new(N4DiagnosticCopy::<12>::default());
            let a = std::sync::Arc::clone(&copy);
            let b = std::sync::Arc::clone(&copy);
            let first = std::thread::spawn(move || {
                for _ in 0..10000 {
                    a.publish([0x1111111111111111; 12]);
                }
            });
            let second = std::thread::spawn(move || {
                for _ in 0..10000 {
                    b.publish([0xeeeeeeeeeeeeeeee; 12]);
                }
            });
            for _ in 0..10000 {
                let (status, values) = copy.snapshot();
                if status == 0 {
                    assert!(
                        values == [0x1111111111111111; 12] || values == [0xeeeeeeeeeeeeeeee; 12]
                    );
                } else {
                    assert!(status <= 3);
                }
            }
            first.join().unwrap();
            second.join().unwrap();
            let (status, values) = copy.snapshot();
            assert_eq!(status, 0);
            assert!(values == [0x1111111111111111; 12] || values == [0xeeeeeeeeeeeeeeee; 12]);
        }
        #[test]
        fn n4_first_cause_survives_later_writers_and_time_saturation_is_explicit() {
            let diagnostics = Diagnostics {
                enabled: true,
                origin: Some(std::time::Instant::now()),
                ..Diagnostics::default()
            };
            diagnostics.arm_popup_initial_blank();
            diagnostics.n4_first(N4Cause::Unknown, 3746, 2);
            let first = diagnostics.n4_causes[1].load(std::sync::atomic::Ordering::Acquire);
            std::thread::scope(|scope| {
                for _ in 0..8 {
                    scope.spawn(|| diagnostics.n4_first(N4Cause::Unknown, 5499, 255));
                }
            });
            assert_eq!(
                diagnostics.n4_causes[1].load(std::sync::atomic::Ordering::Acquire),
                first
            );
            assert_eq!(first & 0xffff, 3746);
            assert_ne!(n4_first_word(1, 0, u64::MAX) & (1 << 63), 0);
        }
        #[test]
        fn n4_result_classes_do_not_promote_precondition_null_or_other_tokens() {
            for (text, class) in [
                (
                    r#"{"result":{"type":"string","value":"code0-popup-scheduled-v2"}}"#,
                    1,
                ),
                (
                    r#"{"result":{"type":"string","value":"code0-popup-precondition-v2"}}"#,
                    2,
                ),
                (
                    r#"{"result":{"type":"string","value":"code0-popup-precondition-v2","description":"wrong"}}"#,
                    4,
                ),
                (r#""code0-popup-scheduled-v2""#, 4),
                (r#""code0-popup-precondition-v2""#, 4),
                (r#""code0-popup-submitted-v1""#, 4),
                (r#""code0-popup-precondition-v1""#, 4),
                ("\"code0-popup-scheduled-v2\" ", 4),
                (r#""code0-popup-\u0073cheduled-v2""#, 4),
                ("\"other\"", 4),
                (
                    r#"{"result":{"type":"string","value":"code0-popup-\u0073cheduled-v2"}}"#,
                    4,
                ),
                ("null", 3),
                ("broken", 4),
            ] {
                let units: Vec<u16> = text.encode_utf16().collect();
                let real = popup_result_units(&units);
                assert_eq!(n4_popup_result_class(&units, real), class);
                assert_eq!(real.1, class == 1);
            }
        }
    }
    // N4 diagnostic end.

    // N3 diagnostic begin: closed scalar domains, never lifecycle authority.
    #[derive(Clone, Copy)]
    #[repr(u16)]
    pub(super) enum N3Block {
        NativeCleanup = 1,
        PollIdentity = 2,
        PollUnknown = 3,
        Dialog = 4,
        RegistryPropagation = 5,
        RefusalCallback = 6,
    }
    #[derive(Clone, Copy)]
    #[repr(u16)]
    pub(super) enum N3Writer {
        CompletionAffinity = 1,
        MissingMailbox = 2,
        DuplicateInvocation = 3,
        OccupiedDisposition = 4,
        CompletionIdentity = 5,
        DialogUncertainty = 6,
        ContextAffinity = 7,
        ContextIdentity = 8,
        PopupAffinity = 9,
        DownloadAffinity = 10,
        SubmissionBinding = 11,
        CompletedRemoval = 12,
        HookRemoval = 13,
    }
    impl N3Writer {
        #[cfg(test)]
        const ALL: [Self; 13] = [
            Self::CompletionAffinity,
            Self::MissingMailbox,
            Self::DuplicateInvocation,
            Self::OccupiedDisposition,
            Self::CompletionIdentity,
            Self::DialogUncertainty,
            Self::ContextAffinity,
            Self::ContextIdentity,
            Self::PopupAffinity,
            Self::DownloadAffinity,
            Self::SubmissionBinding,
            Self::CompletedRemoval,
            Self::HookRemoval,
        ];
    }
    #[derive(Clone, Copy)]
    #[repr(u8)]
    pub(super) enum N3DialogReason {
        Unspecified = 0,
        InvalidMailbox = 1,
        RejectedCompletion = 2,
        RegistryUnknown = 3,
        MissingBinding = 4,
        RemovalMismatch = 5,
    }
    #[derive(Clone, Copy)]
    pub(super) struct N3CompletionDiagnostic {
        pub(super) same: bool,
        pub(super) not_released: bool,
        pub(super) disposal_timely: bool,
        pub(super) invalid: Option<bool>,
        pub(super) affinity: Option<bool>,
        pub(super) hresult_succeeded: bool,
        pub(super) bounded: bool,
        pub(super) fixed: bool,
        pub(super) original_timely: bool,
        pub(super) valid: bool,
    }
    impl N3CompletionDiagnostic {
        pub(super) fn bits(self) -> u16 {
            u16::from(self.same)
                | (u16::from(self.not_released) << 1)
                | (u16::from(self.disposal_timely) << 2)
                | (u16::from(self.invalid.is_some()) << 3)
                | (u16::from(self.invalid == Some(true)) << 4)
                | (u16::from(self.affinity.is_some()) << 5)
                | (u16::from(self.affinity == Some(true)) << 6)
                | (u16::from(self.hresult_succeeded) << 7)
                | (u16::from(self.bounded) << 8)
                | (u16::from(self.fixed) << 9)
                | (u16::from(self.original_timely) << 10)
                | (u16::from(self.valid) << 11)
        }
    }
    pub(super) fn n3_inventory_words(state: u64, counts: [usize; 5]) -> [u64; 2] {
        let mut flags = state & !(0x1f << 24);
        let mut packed = 0;
        for (index, (value, cap, shift)) in counts
            .into_iter()
            .zip([65535, 65535, 65535, 255, 255])
            .zip([0, 16, 32, 48, 56])
            .map(|((value, cap), shift)| (value, cap, shift))
            .enumerate()
        {
            flags |= u64::from(value > cap) << (24 + index);
            packed |= (value.min(cap) as u64) << shift;
        }
        [flags, packed]
    }
    struct N3DiagnosticSnapshot {
        block: u64,
        writer: u64,
        sites: u64,
        status: u8,
        missed: bool,
        words: [u64; 2],
    }
    impl N3DiagnosticSnapshot {
        fn emit(self, kind: u8) {
            use std::io::Write as _;
            let _ = writeln!(
                std::io::stderr(),
                "Code0NativeN3Causes schema=1 kind={} block={:016x} writer={:016x} sites={:016x}",
                kind,
                self.block,
                self.writer,
                self.sites
            );
            let _ = writeln!(
                std::io::stderr(),
                "Code0NativeN3Snapshot schema=1 kind={} status={} missed={} state={:016x} counts={:016x}",
                kind,
                self.status,
                u8::from(self.missed),
                self.words[0],
                self.words[1]
            );
        }
    }
    // N3 diagnostic end.

    // N4 diagnostic begin: fixed scalar copies, no owner/native references.
    #[derive(Clone, Copy)]
    #[repr(usize)]
    pub(super) enum N4Cause {
        Revocation,
        Unknown,
        CleanupBlock,
    }

    // One atomic word is the complete first record: site16, bits16, ms1(31),
    // bit63=time saturation. CAS is the commit: no pending/torn metadata state.
    fn n4_first_word(site: u16, bits: u16, ms1: u64) -> u64 {
        u64::from(site)
            | (u64::from(bits) << 16)
            | (ms1.min(0x7fffffff) << 32)
            | (u64::from(ms1 > 0x7fffffff) << 63)
    }

    struct N4DiagnosticCopy<const N: usize> {
        sequence: std::sync::atomic::AtomicU64,
        missed: std::sync::atomic::AtomicBool,
        words: [std::sync::atomic::AtomicU64; N],
    }
    impl<const N: usize> Default for N4DiagnosticCopy<N> {
        fn default() -> Self {
            Self {
                sequence: Default::default(),
                missed: Default::default(),
                words: std::array::from_fn(|_| Default::default()),
            }
        }
    }
    impl<const N: usize> N4DiagnosticCopy<N> {
        fn publish(&self, words: [u64; N]) {
            use std::sync::atomic::Ordering::{AcqRel, Acquire, Release};
            let sequence = self.sequence.load(Acquire);
            if sequence & 1 != 0
                || sequence >= u64::MAX - 1
                || self
                    .sequence
                    .compare_exchange(sequence, sequence + 1, AcqRel, Acquire)
                    .is_err()
            {
                self.missed.store(true, Release);
                return;
            }
            for (slot, value) in self.words.iter().zip(words) {
                slot.store(value, Release);
            }
            self.sequence.store(sequence + 2, Release);
        }
        // status0 coherent;1 never published;2 in progress;3 changed during copy.
        // No spin, retry, mailbox lock, Owner access, or native call on readers.
        fn snapshot(&self) -> (u8, [u64; N]) {
            use std::sync::atomic::Ordering::Acquire;
            let before = self.sequence.load(Acquire);
            if before == 0 {
                return (1, [0; N]);
            }
            if before & 1 != 0 {
                return (2, [0; N]);
            }
            let words = std::array::from_fn(|n| self.words[n].load(Acquire));
            std::sync::atomic::fence(Acquire);
            if self.sequence.load(Acquire) != before {
                (3, [0; N])
            } else {
                (0, words)
            }
        }
    }

    // Classification only: class1 is the closed scheduling envelope, class2 the
    // closed precondition envelope, class3 raw null, class4 rejected/malformed,
    // class5 a UTF16/UTF8 cap failure. No class supplies native target authority.
    pub(super) fn n4_popup_result_class(units: &[u16], real: (bool, bool)) -> u8 {
        if real.1 {
            1
        } else {
            popup_cdp_result_kind(units)
        }
    }
    // N4 diagnostic end.

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
        // N3 diagnostic begin.
        n3_armed: std::sync::atomic::AtomicBool,
        n3_first_block: std::sync::atomic::AtomicU64,
        n3_first_writer: std::sync::atomic::AtomicU64,
        n3_writer_sites: std::sync::atomic::AtomicU64,
        n3_snapshot: N4DiagnosticCopy<2>,
        // N3 diagnostic end.
        // N4 diagnostic begin.
        n4_armed: std::sync::atomic::AtomicBool,
        n4_deadline_emitted: std::sync::atomic::AtomicBool,
        n4_causes: [std::sync::atomic::AtomicU64; 3],
        n4_snapshot: N4DiagnosticCopy<12>,
        n4_disposition: N4DiagnosticCopy<4>,
        n4_stages: [std::sync::atomic::AtomicU64; 14],
        // N4 diagnostic end.
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
        // N3 diagnostic begin. No N4 arming or popup-specific work.
        pub fn arm_dialog_owned_empty_html(&self) {
            if self.enabled {
                self.n3_armed
                    .store(true, std::sync::atomic::Ordering::Release);
            }
        }
        pub(super) fn n3_enabled(&self) -> bool {
            self.enabled && self.n3_armed.load(std::sync::atomic::Ordering::Acquire)
        }
        pub(super) fn n3_block(&self, was_blocked: bool, site: N3Block, bits: u16) {
            if !was_blocked && self.n3_enabled() {
                let _ = self.n3_first_block.compare_exchange(
                    0,
                    n4_first_word(site as u16, bits, self.n4_time(std::time::Instant::now())),
                    std::sync::atomic::Ordering::AcqRel,
                    std::sync::atomic::Ordering::Acquire,
                );
            }
        }
        pub(super) fn n3_writer(&self, site: N3Writer, bits: u16) {
            if self.n3_enabled() {
                self.n3_writer_sites
                    .fetch_or(1 << (site as u16), std::sync::atomic::Ordering::AcqRel);
                // First recorded writer only: original business stores remain unchanged.
                let _ = self.n3_first_writer.compare_exchange(
                    0,
                    n4_first_word(site as u16, bits, self.n4_time(std::time::Instant::now())),
                    std::sync::atomic::Ordering::AcqRel,
                    std::sync::atomic::Ordering::Acquire,
                );
            }
        }
        fn n3_close_snapshot(&self) -> Option<N3DiagnosticSnapshot> {
            use std::sync::atomic::Ordering::Acquire;
            if !self.n3_enabled() {
                return None;
            }
            let (status, words) = self.n3_snapshot.snapshot();
            Some(N3DiagnosticSnapshot {
                block: self.n3_first_block.load(Acquire),
                writer: self.n3_first_writer.load(Acquire),
                sites: self.n3_writer_sites.load(Acquire),
                status,
                missed: self.n3_snapshot.missed.load(Acquire),
                words,
            })
        }
        pub(super) fn n3_publish_inventory(&self, state: u64, counts: [usize; 5]) {
            if self.n3_enabled() {
                self.n3_snapshot.publish(n3_inventory_words(state, counts));
            }
        }
        // N3 diagnostic end.

        // N4 diagnostic begin. Only the fixed selected N4 fixture arms this.
        pub fn arm_popup_initial_blank(&self) {
            if self.enabled {
                self.n4_armed
                    .store(true, std::sync::atomic::Ordering::Release);
            }
        }
        pub(super) fn n4_enabled(&self) -> bool {
            self.enabled && self.n4_armed.load(std::sync::atomic::Ordering::Acquire)
        }
        pub(super) fn n4_time(&self, at: std::time::Instant) -> u64 {
            self.origin.map_or(0, |origin| {
                let value = at
                    .saturating_duration_since(origin)
                    .as_millis()
                    .saturating_add(1);
                (value.min(u128::from(u64::MAX >> 1)) as u64)
                    | (u64::from(value > u128::from(u64::MAX >> 1)) << 63)
            })
        }
        pub(super) fn n4_publish_popup(&self, words: [u64; 12]) {
            if self.n4_enabled() {
                self.n4_snapshot.publish(words);
            }
        }
        pub(super) fn n4_publish_disposition(&self, words: [u64; 4]) {
            if self.n4_enabled() {
                self.n4_disposition.publish(words);
            }
        }
        pub(super) fn n4_first(&self, cause: N4Cause, site: u16, bits: u16) {
            if self.n4_enabled() {
                let word = n4_first_word(site, bits, self.n4_time(std::time::Instant::now()));
                let _ = self.n4_causes[cause as usize].compare_exchange(
                    0,
                    word,
                    std::sync::atomic::Ordering::AcqRel,
                    std::sync::atomic::Ordering::Acquire,
                );
            }
        }
        // Fixed indices: native Popup0..3, native FailedPolicy4/FailedCleanup5/
        // other6; fixture Popup7..10, FailedPolicy11/FailedCleanup12/other13.
        // Values are first producer/consumer time+1, not receipt totals or proof.
        pub fn n4_popup_stage(&self, stage: NativePopupStage, consumed: bool) {
            let index = match stage {
                NativePopupStage::Submitted => 0,
                NativePopupStage::Refusal => 1,
                NativePopupStage::Completion => 2,
                NativePopupStage::Final => 3,
            } + if consumed { 7 } else { 0 };
            self.n4_stage(index);
        }
        pub fn n4_session_stage(&self, reason: &Code0SessionFailure, consumed: bool) {
            let index = match reason {
                Code0SessionFailure::PolicyRejected => 4,
                Code0SessionFailure::CleanupFailed => 5,
                _ => 6,
            } + if consumed { 7 } else { 0 };
            self.n4_stage(index);
        }
        fn n4_stage(&self, index: usize) {
            if self.n4_enabled() {
                let _ = self.n4_stages[index].compare_exchange(
                    0,
                    self.n4_time(std::time::Instant::now()),
                    std::sync::atomic::Ordering::AcqRel,
                    std::sync::atomic::Ordering::Acquire,
                );
            }
        }
        pub fn emit_popup_close_deadline(&self) {
            if !self
                .n4_deadline_emitted
                .swap(true, std::sync::atomic::Ordering::Relaxed)
            {
                self.emit_n4(1);
            }
        }
        fn emit_n4(&self, kind: u8) {
            use std::io::Write as _;
            use std::sync::atomic::Ordering::Acquire;
            if !self.n4_enabled() {
                return;
            }
            let _ = writeln!(
                std::io::stderr(),
                "Code0NativeN4Causes schema=1 kind={} revocation={:016x} unknown={:016x} cleanupBlock={:016x}",
                kind,
                self.n4_causes[0].load(Acquire),
                self.n4_causes[1].load(Acquire),
                self.n4_causes[2].load(Acquire)
            );
            let (status, w) = self.n4_snapshot.snapshot();
            let _ = writeln!(
                std::io::stderr(),
                "Code0NativeN4Snapshot schema=1 kind={} status={} missed={} state={:016x} counts={:016x} facts={:016x} deadlineMs1={:016x} firstCloseMs1={:016x} obligation={:016x} popupHandlers={:016x} aggregateHandlers={:016x} pendingActive={:016x} sampleMs1={:016x} mailboxCloseMs1={:016x} exitState={:016x}",
                kind,
                status,
                u8::from(self.n4_snapshot.missed.load(Acquire)),
                w[0],
                w[1],
                w[2],
                w[3],
                w[4],
                w[5],
                w[6],
                w[7],
                w[8],
                w[9],
                w[10],
                w[11]
            );
            let (status, w) = self.n4_disposition.snapshot();
            let _ = writeln!(
                std::io::stderr(),
                "Code0NativeN4Disposition schema=1 kind={} status={} missed={} components={:016x} receivedMs1={:016x} consumedSampleMs1={:016x} outcome={:016x}",
                kind,
                status,
                u8::from(self.n4_disposition.missed.load(Acquire)),
                w[0],
                w[1],
                w[2],
                w[3]
            );
            let s: [u64; 14] = std::array::from_fn(|n| self.n4_stages[n].load(Acquire));
            let _ = writeln!(
                std::io::stderr(),
                "Code0NativeN4Stages schema=1 kind={} n0={:016x} n1={:016x} n2={:016x} n3={:016x} n4={:016x} n5={:016x} n6={:016x} c0={:016x} c1={:016x} c2={:016x} c3={:016x} c4={:016x} c5={:016x} c6={:016x}",
                kind,
                s[0],
                s[1],
                s[2],
                s[3],
                s[4],
                s[5],
                s[6],
                s[7],
                s[8],
                s[9],
                s[10],
                s[11],
                s[12],
                s[13]
            );
        }
        // N4 diagnostic end.

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
                n3: self.n3_close_snapshot(),
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
                self.emit_n4(2);
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

        /// One fixed non-user N5 download operation on the actual initial blank.
        pub fn submit_fixture_download_initial_data_empty(
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
                super::native::submit_download_initial_data_empty(&self.owner, request, deadline)
            }
            #[cfg(not(windows))]
            {
                let _ = (request, deadline);
                Err(Code0SessionFailure::PolicyRejected)
            }
        }

        /// Finite S4 transport only. The caller chooses one of four fixed cases.
        pub fn begin_fixture_capability(
            &mut self,
            request: &Code0WindowRequest,
            selector: NativeCapabilitySelector,
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
                super::native::s4_begin(&self.owner, request, selector, deadline)
            }
            #[cfg(not(windows))]
            {
                let _ = (request, selector, deadline);
                Err(Code0SessionFailure::PolicyRejected)
            }
        }

        /// S3 sole fixed input fixture; no caller script, URL or path.
        pub fn begin_fixture_prebound_chooser(
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
            let root = self
                .profile_root
                .parent()
                .ok_or(Code0SessionFailure::PolicyRejected)?;
            #[cfg(windows)]
            {
                super::native::begin_chooser(&self.owner, request, root, deadline)
            }
            #[cfg(not(windows))]
            {
                let _ = (request, root, deadline);
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
