//! UI-local direct-COM Code0 owner seam. This first checkpoint intentionally
//! denies native creation; it reports no policy, window or release evidence.
use std::path::PathBuf;
use std::rc::Rc;

use super::code0_profile_lease::{Code0LeaseError, Code0ProfileLease};
use super::code0_web_session::{
    Code0CleanupRequest, Code0SessionFailure, Code0WindowDriving, Code0WindowEvent,
    Code0WindowRequest,
};

pub type Code0NativeEventSink = Rc<dyn Fn(Code0WindowRequest, Code0WindowEvent)>;

/// Same constructor/create path for future production and the native fixture.
/// Rc deliberately keeps this owner !Send and !Sync; native COM must remain on
/// the UI/STA thread. A successful command is never an async acknowledgement.
pub struct Code0NativeDriver {
    _app: tauri::AppHandle,
    profile_root: PathBuf,
    lease: Option<Code0ProfileLease>,
    _events: Code0NativeEventSink,
    #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
    _fixture_observer: Option<fixture::Observer>,
}

impl Code0NativeDriver {
    pub fn new(app: tauri::AppHandle, profile_root: PathBuf, events: Code0NativeEventSink) -> Self {
        Self {
            _app: app,
            profile_root,
            lease: None,
            _events: events,
            #[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
            _fixture_observer: None,
        }
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
        // No native objects can exist in this deny-only checkpoint.
        if let Some(lease) = self.lease.take() {
            lease.release();
        }
    }

    fn create_unloaded(&mut self, _request: Code0WindowRequest) -> Result<(), Code0SessionFailure> {
        Err(Code0SessionFailure::PolicyRejected)
    }

    fn navigate(&mut self, _instance_id: &str) -> Result<(), Code0SessionFailure> {
        // Official navigation remains denied until all real policies and scripts
        // have acknowledged installation in a later scoped checkpoint.
        Err(Code0SessionFailure::PolicyRejected)
    }

    fn close(&mut self, _instance_id: &str) {
        // Nothing was constructed. Do not invent Released or Destroyed events.
    }

    fn clear_website_data(
        &mut self,
        _request: Code0CleanupRequest,
    ) -> Result<(), Code0SessionFailure> {
        Err(Code0SessionFailure::PolicyRejected)
    }
}

/// No release build exposes the inline-document or native observation seam,
/// even if the opt-in feature is accidentally enabled. No arbitrary URL/HTML,
/// host object, CDP, devtools or origin setting is accepted here.
#[cfg(all(debug_assertions, feature = "code0-native-fixture"))]
pub mod fixture {
    use std::path::PathBuf;
    use std::rc::Rc;

    use super::{Code0NativeDriver, Code0SessionFailure};

    /// Future GREEN must obtain these facts from the actual owned engine and
    /// callback, never from the requested path, synthetic counters or policies.
    pub enum NativeSmokeObservation {
        /// Synchronous receipt of the actual owned bare Window immediately after
        /// construction, before any hide/focus correction or asynchronous engine
        /// creation. The fixture derives identity and initial state from this
        /// handle and installs sticky native focus observation before returning.
        /// EngineReady may never stand in for or precede this receipt.
        WindowConstructed { window: Box<tauri::Window> },
        /// Only after actual non-null environment/controller/core completion.
        /// user_data_folder is queried from the environment, not copied input.
        EngineReady {
            window_label: String,
            user_data_folder: PathBuf,
            runtime_version: String,
        },
        /// Bounded fixed object JSON delivered by the real renderer message
        /// callback. This is not a production bridge or official-origin proof.
        RendererMessage { payload: Vec<u8> },
    }

    /// Invoked directly on the native callback thread, without a forwarding
    /// hop, so the fixture independently observes callback affinity and STA.
    pub type Observer = Rc<dyn Fn(NativeSmokeObservation)>;

    impl Code0NativeDriver {
        pub fn observe_fixture(&mut self, observer: Observer) {
            self._fixture_observer = Some(observer);
        }

        /// Later GREEN uses NavigateToString on this SAME owned controller with
        /// fully inline fixed HTML posting {kind:"code0NativeSmoke",value:7}.
        /// It may not call navigate(), install official policy acknowledgements,
        /// accept caller-provided content, or load any external resource.
        pub fn load_fixture_inline_document(
            &mut self,
            _instance_id: &str,
        ) -> Result<(), Code0SessionFailure> {
            Err(Code0SessionFailure::PolicyRejected)
        }
    }
}
