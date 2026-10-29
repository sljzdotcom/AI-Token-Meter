//! One process-main native smoke case. A skipped/unsupported target is never PASS.
//! Catches a driver that accepts create without a real hidden bare engine, renderer
//! message, actual destruction and owned reference release. No mock engine is used.
#[cfg(all(windows, debug_assertions, feature = "code0-native-fixture"))]
mod smoke {
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;
    use std::fs::{File, OpenOptions};
    use std::io::{Seek, SeekFrom, Write};
    use std::path::PathBuf;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex, mpsc};
    use std::thread::{self, ThreadId};
    use std::time::{Duration, Instant};

    use ai_token_meter_windows::platform::windows::code0_web_session::{
        Code0SessionFailure, Code0WindowDriving, Code0WindowEvent, Code0WindowMode,
        Code0WindowRequest,
    };
    use ai_token_meter_windows::platform::windows::code0_webview::fixture::{
        NativeCapabilityFacts, NativeCapabilityFailure, NativeCapabilityReceipt,
        NativeCapabilityResult, NativeCapabilitySelector, NativeCapabilityStage,
        NativeChooserFacts, NativeChooserReceipt, NativeChooserStage,
    };
    use ai_token_meter_windows::platform::windows::code0_webview::{
        Code0NativeDriver,
        fixture::{
            ContainmentReceipt, DiagnosticMilestone as DM, DiagnosticPhase as DP, Diagnostics,
            NativeChildCompletionReceipt, NativeChildFinalReceipt, NativeChildRefusalFacts,
            NativeChildRefusalReceipt, NativeChildSubmissionReceipt, NativeDialogDeadline,
            NativeDialogFacts, NativeDialogReceipt, NativeDialogStage, NativeDownloadFacts,
            NativeDownloadReceipt, NativeDownloadStage, NativePopupFacts, NativePopupReceipt,
            NativePopupStage, NativeRefusalDeadline, NativeRefusalInstallReceipt,
            NativeSmokeObservation, NativeTopRefusalFacts, NativeTopRefusalReceipt,
            NativeTopSubmissionReceipt, PolicyFaultReceipt, PolicyFixtureCase, PolicyObservation,
            StaleDispositionReceipt, StaleDispositionStage, adopt_legacy_native_close,
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
    const POLICY_CASE: &str = "install-ack-before-first-load";
    const POLICY_INSTANCE: &str = "22222222-2222-4222-8222-222222222222";
    const NATIVE_REFUSAL_CASE: &str = "install-native-refusal-hooks-before-load";
    const CHILD_REFUSAL_CASE: &str = "refusal-child-initial-data";
    const TOP_REFUSAL_CASE: &str = "refusal-top-owned-file";
    const DIALOG_REFUSAL_CASE: &str = "refusal-dialog-owned-empty-html";
    const DOWNLOAD_REFUSAL_CASE: &str = "refusal-download-initial-data-empty";
    const POPUP_REFUSAL_CASE: &str = "refusal-popup-initial-blank";
    const CHOOSER_CASE: &str = "chooser-prebound-owned-empty-file";
    const CHOOSER_LEAF: &str = "code0-s3-chooser-empty.html";
    const DIALOG_REFUSAL_LEAF: &str = "code0-s2-dialog-empty.html";
    const TOP_REFUSAL_LEAF: &str = "code0-s2-top-denied.txt";
    const NATIVE_REFUSAL_HOOKS: [&str; 10] = [
        "topNavigation",
        "childNavigation",
        "externalProtocol",
        "newWindow",
        "download",
        "scriptDialog",
        "permission",
        "basicAuthentication",
        "clientCertificate",
        "tlsError",
    ];

    #[derive(Default, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct NativeRefusalEvidence {
        receipts: u8,
        native_instance_validated: bool,
        source_bound_registry: bool,
        required_mask: u16,
        installed_mask: u16,
        not_installed_hooks: Vec<&'static str>,
        navigation_not_issued: bool,
        load_rejected: bool,
    }
    impl NativeRefusalEvidence {
        fn observe(
            &mut self,
            receipt: &NativeRefusalInstallReceipt,
            request: &Code0WindowRequest,
        ) -> bool {
            self.receipts = self.receipts.saturating_add(1);
            if self.receipts != 1 || !receipt.matches(request) {
                return false;
            }
            self.native_instance_validated = true;
            // Opaque same-instance native add/token inventory, not interface
            // availability, a caller-set count or native-event coverage.
            self.source_bound_registry = true;
            self.required_mask = 0x03ff;
            self.installed_mask = receipt.installed_mask();
            self.not_installed_hooks = receipt
                .not_installed()
                .iter()
                .map(|hook| hook.name())
                .collect();
            self.navigation_not_issued = true;
            self.load_rejected = receipt.load_rejected();
            let expected_missing: Vec<_> = NATIVE_REFUSAL_HOOKS
                .into_iter()
                .enumerate()
                .filter(|(index, _)| self.installed_mask & (1u16 << *index) == 0)
                .map(|(_, name)| name)
                .collect();
            self.installed_mask & !self.required_mask == 0
                && self.not_installed_hooks == expected_missing
                && self.load_rejected == (self.installed_mask != self.required_mask)
        }
        fn positive(&self, lifecycle: &Summary) -> bool {
            self.receipts == 1
                && self.native_instance_validated
                && self.source_bound_registry
                && self.required_mask == 0x03ff
                && self.installed_mask == self.required_mask
                && self.not_installed_hooks.is_empty()
                && self.navigation_not_issued
                && !self.load_rejected
                && lifecycle.sta_main
                && lifecycle.callback_affinity
                && lifecycle.no_inherited_override
                && lifecycle.engine_ready
                && lifecycle.bare_hidden_unfocused
                && lifecycle.construction_observed
                && !lifecycle.bare_window_violation
                && lifecycle.profile_isolated
                && lifecycle.destroyed
                && lifecycle.owned_references_released
        }
    }

    // Finite N5: event/action, actual Session failure and strict completion have
    // independent producers. No installation or script flag supplies another.
    #[derive(Default, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct DownloadRefusalEvidence {
        submission_receipts: u8,
        target_receipts: u8,
        completion_receipts: u8,
        final_receipts: u8,
        receipt_installation_matched: bool,
        receipt_consumption_timely: bool,
        policy_failures: u8,
        action_before_policy_failure: bool,
        browser_process_exited_once: bool,
        #[serde(flatten)]
        facts: NativeDownloadFacts,
        #[serde(skip)]
        installation: Option<NativeRefusalInstallReceipt>,
    }
    impl DownloadRefusalEvidence {
        fn observe(
            &mut self,
            receipt: NativeDownloadReceipt,
            request: &Code0WindowRequest,
            attempt_started: Instant,
        ) -> bool {
            let before = attempt_started.elapsed() < Duration::from_secs(45);
            let success_stage = !matches!(receipt.stage(), NativeDownloadStage::Final);
            let first_success = self.submission_receipts == 0
                && self.target_receipts == 0
                && self.completion_receipts == 0;
            let matched = self
                .installation
                .as_ref()
                .is_some_and(|i| receipt.matches(request, i));
            let first = self.submission_receipts == 0
                && self.target_receipts == 0
                && self.completion_receipts == 0
                && self.final_receipts == 0;
            self.receipt_installation_matched = if first {
                matched
            } else {
                self.receipt_installation_matched && matched
            };
            let count = match receipt.stage() {
                NativeDownloadStage::Submitted => &mut self.submission_receipts,
                NativeDownloadStage::Refusal => &mut self.target_receipts,
                NativeDownloadStage::Completion => &mut self.completion_receipts,
                NativeDownloadStage::Final => &mut self.final_receipts,
            };
            *count = count.saturating_add(1).min(2);
            let once = *count == 1;
            self.facts = receipt.facts();
            drop(receipt);
            let timely = before && attempt_started.elapsed() < Duration::from_secs(45);
            if success_stage {
                self.receipt_consumption_timely = if first_success {
                    timely
                } else {
                    self.receipt_consumption_timely && timely
                };
            }
            matched && once && (!success_stage || timely)
        }
        fn target_positive(&self) -> bool {
            self.target_receipts == 1
                && self.receipt_installation_matched
                && self.receipt_consumption_timely
                && self.facts.target_positive()
        }
        fn positive(&self, native: &NativeRefusalEvidence, lifecycle: &Summary) -> bool {
            self.submission_receipts == 1
                && self.target_positive()
                && self.completion_receipts == 1
                && self.final_receipts == 1
                && self.policy_failures == 1
                && self.action_before_policy_failure
                && self.browser_process_exited_once
                && self.facts.positive()
                && native.positive(lifecycle)
        }
        fn failure(&self) -> &'static str {
            if !self.facts.initial_source_matched || !self.facts.source_after_drop_timely {
                "downloadInitialBlankPreconditionFailed"
            } else if !self.target_positive() {
                "downloadTargetUnobserved"
            } else if self.completion_receipts != 1
                || !self.facts.completion_hresult_succeeded
                || !self.facts.fixed_result_matched
            {
                "downloadStrictCompletionFailed"
            } else if !self.facts.completion_obligation_drained
                || !self.facts.completion_handlers_drained
            {
                "downloadCompletionNotDrained"
            } else {
                "downloadPositiveOracleMismatch"
            }
        }
    }

    struct CapabilityEvidence {
        facts: NativeCapabilityFacts,
        installation: Option<NativeRefusalInstallReceipt>,
        receipts: [u8; 3],
        matched: bool,
        policy_failures: u8,
        failure_after_disposition: bool,
        browser_process_exited_once: bool,
    }
    impl CapabilityEvidence {
        fn new(selector: NativeCapabilitySelector) -> Self {
            Self {
                facts: NativeCapabilityFacts::new(selector),
                installation: None,
                receipts: [0; 3],
                matched: true,
                policy_failures: 0,
                failure_after_disposition: false,
                browser_process_exited_once: false,
            }
        }
        fn observe(
            &mut self,
            receipt: NativeCapabilityReceipt,
            request: &Code0WindowRequest,
            started: Instant,
            close: Option<Instant>,
            runtime: &str,
        ) -> bool {
            let matched = self
                .installation
                .as_ref()
                .is_some_and(|i| receipt.matches(request, i));
            let index = match receipt.stage() {
                NativeCapabilityStage::Started => 0,
                NativeCapabilityStage::Disposition => 1,
                NativeCapabilityStage::Final => 2,
            };
            self.receipts[index] = self.receipts[index].saturating_add(1).min(2);
            let timely = if index == 0 {
                started.elapsed() < Duration::from_secs(45)
            } else {
                close.is_some_and(|at| Instant::now() < at + Duration::from_secs(10))
            };
            let facts = receipt.facts();
            drop(receipt);
            let valid = matched
                && timely
                && self.receipts[index] == 1
                && facts.selector == self.facts.selector
                && runtime.len() <= 128
                && !runtime.is_empty()
                && runtime.bytes().all(|v| v.is_ascii_digit() || v == b'.');
            self.matched &= valid;
            self.facts = facts;
            self.facts.runtime_version = runtime.to_owned();
            valid
        }
        fn positive(&self, native: &NativeRefusalEvidence, lifecycle: &Summary) -> bool {
            self.receipts == [1, 1, 1]
                && self.matched
                && self.facts.positive_before_release()
                && self.facts.normal_released
                && self.facts.runtime_version == lifecycle.runtime_version
                && self.browser_process_exited_once
                && if self.facts.failure.is_some() {
                    self.policy_failures == 1 && self.failure_after_disposition
                } else {
                    self.policy_failures == 0
                }
                && native.receipts == 1
                && native.native_instance_validated
                && native.source_bound_registry
                && native.required_mask == 1023
                && native.installed_mask == 1023
                && native.not_installed_hooks.is_empty()
                && !native.navigation_not_issued
                && !native.load_rejected
                && lifecycle.sta_main
                && lifecycle.callback_affinity
                && lifecycle.no_inherited_override
                && lifecycle.engine_ready
                && lifecycle.bare_hidden_unfocused
                && lifecycle.construction_observed
                && !lifecycle.bare_window_violation
                && lifecycle.profile_isolated
                && lifecycle.destroyed
                && lifecycle.owned_references_released
        }
        fn failure(&self) -> &'static str {
            if self.facts.result == NativeCapabilityResult::BaselineRefused
                && self.facts.failure == Some(NativeCapabilityFailure::CapabilityApprovalMissing)
            {
                "capabilityApprovalMissing"
            } else {
                "capabilityNotEstablished"
            }
        }
    }

    // S3 counts end at the real trusted cancel snapshot. Native target/effect,
    // actual Session failure and normal lifecycle have independent producers.
    #[derive(Default, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ChooserEvidence {
        owned_file_prepared: bool,
        started_receipts: u8,
        target_receipts: u8,
        effect_receipts: u8,
        final_receipts: u8,
        receipt_installation_matched: bool,
        receipt_consumption_timely: bool,
        policy_failures: u8,
        action_before_policy_failure: bool,
        browser_process_exited_once: bool,
        #[serde(flatten)]
        facts: NativeChooserFacts,
        #[serde(skip)]
        installation: Option<NativeRefusalInstallReceipt>,
    }
    impl ChooserEvidence {
        fn observe(
            &mut self,
            receipt: NativeChooserReceipt,
            request: &Code0WindowRequest,
            started: Instant,
        ) -> bool {
            let before = started.elapsed() < Duration::from_secs(45);
            let matched = self
                .installation
                .as_ref()
                .is_some_and(|i| receipt.matches(request, i));
            let first = self.started_receipts == 0
                && self.target_receipts == 0
                && self.effect_receipts == 0
                && self.final_receipts == 0;
            self.receipt_installation_matched = if first {
                matched
            } else {
                self.receipt_installation_matched && matched
            };
            let success_stage = !matches!(receipt.stage(), NativeChooserStage::Final);
            let first_success = self.started_receipts == 0
                && self.target_receipts == 0
                && self.effect_receipts == 0;
            let count = match receipt.stage() {
                NativeChooserStage::Started => &mut self.started_receipts,
                NativeChooserStage::Target => &mut self.target_receipts,
                NativeChooserStage::Effect => &mut self.effect_receipts,
                NativeChooserStage::Final => &mut self.final_receipts,
            };
            *count = count.saturating_add(1).min(2);
            let once = *count == 1;
            self.facts = receipt.facts();
            drop(receipt);
            let timely = before && started.elapsed() < Duration::from_secs(45);
            if success_stage {
                self.receipt_consumption_timely = if first_success {
                    timely
                } else {
                    self.receipt_consumption_timely && timely
                };
            }
            matched && once && (!success_stage || timely)
        }
        fn target_positive(&self) -> bool {
            self.target_receipts == 1
                && self.receipt_installation_matched
                && self.receipt_consumption_timely
                && self.facts.target_positive()
        }
        fn positive(&self, native: &NativeRefusalEvidence, lifecycle: &Summary) -> bool {
            self.owned_file_prepared
                && self.started_receipts == 1
                && self.target_positive()
                && self.effect_receipts == 1
                && self.final_receipts == 1
                && self.policy_failures == 1
                && self.action_before_policy_failure
                && self.browser_process_exited_once
                && self.facts.positive()
                && native.receipts == 1
                && native.native_instance_validated
                && native.source_bound_registry
                && native.required_mask == 1023
                && native.installed_mask == 1023
                && native.not_installed_hooks.is_empty()
                && !native.navigation_not_issued
                && !native.load_rejected
                && lifecycle.sta_main
                && lifecycle.callback_affinity
                && lifecycle.no_inherited_override
                && lifecycle.engine_ready
                && lifecycle.bare_hidden_unfocused
                && lifecycle.construction_observed
                && !lifecycle.bare_window_violation
                && lifecycle.profile_isolated
                && lifecycle.destroyed
                && lifecycle.owned_references_released
        }
        fn failure(&self) -> &'static str {
            if !self.owned_file_prepared || !self.facts.fresh_document {
                "chooserFreshDocumentUnproved"
            } else if !self.target_positive() {
                "chooserTargetUnobserved"
            } else if self.effect_receipts != 1
                || !self.facts.effect_pre_stop
                || !self.facts.trusted_cancel_snapshot
            {
                "chooserPreStopCancelUnproved"
            } else if self.policy_failures != 1 || !self.action_before_policy_failure {
                "chooserSessionFailureUnproved"
            } else if !self.facts.obligations_drained
                || !self.facts.handlers_drained
                || !self.facts.receivers_drained
                || !self.facts.protocol_text_drained
            {
                "chooserDrainUnproved"
            } else {
                "chooserPositiveOracleMismatch"
            }
        }
    }

    // Finite N4: event/action, actual Session failure and strict completion have
    // independent producers. No installation or script flag supplies another.
    #[derive(Default, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct PopupRefusalEvidence {
        submission_receipts: u8,
        target_receipts: u8,
        completion_receipts: u8,
        final_receipts: u8,
        receipt_installation_matched: bool,
        receipt_consumption_timely: bool,
        policy_failures: u8,
        action_before_policy_failure: bool,
        browser_process_exited_once: bool,
        #[serde(flatten)]
        facts: NativePopupFacts,
        #[serde(skip)]
        installation: Option<NativeRefusalInstallReceipt>,
    }
    impl PopupRefusalEvidence {
        fn observe(
            &mut self,
            receipt: NativePopupReceipt,
            request: &Code0WindowRequest,
            attempt_started: Instant,
        ) -> bool {
            let before = attempt_started.elapsed() < Duration::from_secs(45);
            let success_stage = !matches!(receipt.stage(), NativePopupStage::Final);
            let first_success = self.submission_receipts == 0
                && self.target_receipts == 0
                && self.completion_receipts == 0;
            let matched = self
                .installation
                .as_ref()
                .is_some_and(|i| receipt.matches(request, i));
            let first = self.submission_receipts == 0
                && self.target_receipts == 0
                && self.completion_receipts == 0
                && self.final_receipts == 0;
            self.receipt_installation_matched = if first {
                matched
            } else {
                self.receipt_installation_matched && matched
            };
            let count = match receipt.stage() {
                NativePopupStage::Submitted => &mut self.submission_receipts,
                NativePopupStage::Refusal => &mut self.target_receipts,
                NativePopupStage::Completion => &mut self.completion_receipts,
                NativePopupStage::Final => &mut self.final_receipts,
            };
            *count = count.saturating_add(1).min(2);
            let once = *count == 1;
            self.facts = receipt.facts();
            drop(receipt);
            let timely = before && attempt_started.elapsed() < Duration::from_secs(45);
            if success_stage {
                self.receipt_consumption_timely = if first_success {
                    timely
                } else {
                    self.receipt_consumption_timely && timely
                };
            }
            matched && once && (!success_stage || timely)
        }
        fn target_positive(&self) -> bool {
            self.target_receipts == 1
                && self.receipt_installation_matched
                && self.receipt_consumption_timely
                && self.facts.target_positive()
        }
        fn positive(&self, native: &NativeRefusalEvidence, lifecycle: &Summary) -> bool {
            self.submission_receipts == 1
                && self.target_positive()
                && self.completion_receipts == 1
                && self.final_receipts == 1
                && self.policy_failures == 1
                && self.action_before_policy_failure
                && self.browser_process_exited_once
                && self.facts.positive()
                && native.positive(lifecycle)
        }
        fn failure(&self) -> &'static str {
            if !self.facts.initial_source_matched || !self.facts.source_after_drop_timely {
                "popupInitialBlankPreconditionFailed"
            } else if !self.target_positive() {
                "popupTargetUnobserved"
            } else if self.completion_receipts != 1
                || !self.facts.completion_hresult_succeeded
                || !self.facts.fixed_result_matched
            {
                "popupStrictCompletionFailed"
            } else if !self.facts.completion_obligation_drained
                || !self.facts.completion_handlers_drained
            {
                "popupCompletionNotDrained"
            } else {
                "popupPositiveOracleMismatch"
            }
        }
    }

    // N3's observable contract: missing the native alert, post-alert completion,
    // exact new-file readiness, or actual cleanup must fail independently.
    #[derive(Default, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct DialogRefusalEvidence {
        owned_file_prepared: bool,
        submission_receipts: u8,
        document_receipts: u8,
        script_submission_receipts: u8,
        target_receipts: u8,
        completion_receipts: u8,
        final_receipts: u8,
        receipt_installation_matched: bool,
        policy_failures: u8,
        action_before_policy_failure: bool,
        browser_process_exited_once: bool,
        #[serde(flatten)]
        facts: NativeDialogFacts,
        #[serde(skip)]
        installation: Option<NativeRefusalInstallReceipt>,
    }
    impl DialogRefusalEvidence {
        fn observe(&mut self, receipt: NativeDialogReceipt, request: &Code0WindowRequest) -> bool {
            let matched = self
                .installation
                .as_ref()
                .is_some_and(|i| receipt.matches(request, i));
            self.receipt_installation_matched = if self.submission_receipts == 0
                && self.document_receipts == 0
                && self.script_submission_receipts == 0
                && self.target_receipts == 0
                && self.completion_receipts == 0
                && self.final_receipts == 0
            {
                matched
            } else {
                self.receipt_installation_matched && matched
            };
            let count = match receipt.stage() {
                NativeDialogStage::Submitted => &mut self.submission_receipts,
                NativeDialogStage::DocumentReady => &mut self.document_receipts,
                NativeDialogStage::ScriptSubmitted => &mut self.script_submission_receipts,
                NativeDialogStage::Refusal => &mut self.target_receipts,
                NativeDialogStage::Completion => &mut self.completion_receipts,
                NativeDialogStage::Final => &mut self.final_receipts,
            };
            *count = count.saturating_add(1);
            let once = *count == 1;
            self.facts = receipt.facts();
            matched && once
        }
        fn target_positive(&self) -> bool {
            let f = &self.facts;
            self.target_receipts == 1
                && self.receipt_installation_matched
                && f.dialog_events == 1
                && f.dialog_kind_alert
                && f.dialog_message_matched
                && f.dialog_uri_matched
                && f.dialog_sender_matched
                && f.dialog_live_after_getters
                && f.non_accept_action_returned
        }
        fn failure(&self) -> &'static str {
            if !self.owned_file_prepared
                || !self.facts.asset_binding_validated
                || !self.facts.navigation_submit_ok
            {
                "dialogOwnedDocumentSetupFailed"
            } else if self.document_receipts != 1 || !self.facts.fresh_document {
                "dialogDocumentNotReady"
            } else if !self.target_positive() {
                "dialogTargetUnobserved"
            } else if self.completion_receipts != 1
                || !self.facts.completion_hresult_succeeded
                || !self.facts.fixed_result_matched
            {
                "dialogStrictCompletionFailed"
            } else if !self.facts.completion_obligation_drained
                || !self.facts.completion_handlers_drained
                || !self.facts.completed_remove_succeeded
            {
                "dialogCleanupEvidenceMissing"
            } else {
                "dialogPositiveOracleMismatch"
            }
        }
        fn positive(&self, native: &NativeRefusalEvidence, lifecycle: &Summary) -> bool {
            self.owned_file_prepared
                && self.submission_receipts == 1
                && self.document_receipts == 1
                && self.script_submission_receipts == 1
                && self.target_positive()
                && self.completion_receipts == 1
                && self.final_receipts == 1
                && self.policy_failures == 1
                && self.action_before_policy_failure
                && self.browser_process_exited_once
                && self.facts.positive()
                && native.receipts == 1
                && native.native_instance_validated
                && native.source_bound_registry
                && native.required_mask == 1023
                && native.installed_mask == 1023
                && native.not_installed_hooks.is_empty()
                && !native.navigation_not_issued
                && !native.load_rejected
                && lifecycle.sta_main
                && lifecycle.callback_affinity
                && lifecycle.no_inherited_override
                && lifecycle.engine_ready
                && lifecycle.bare_hidden_unfocused
                && lifecycle.construction_observed
                && !lifecycle.bare_window_violation
                && lifecycle.profile_isolated
                && lifecycle.destroyed
                && lifecycle.owned_references_released
        }
    }

    // This oracle must reject a real native event without a successful Cancel
    // readback, a missing actual Session PolicyRejected, or abnormal cleanup.
    // Submission success and the installation inventory cannot supply those facts.
    #[derive(Default, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct TopRefusalEvidence {
        owned_file_prepared: bool,
        submissions: u8,
        submission_returned: bool,
        native_submit_ok: bool,
        asset_binding_validated: bool,
        installation_before_stimulus: bool,
        stimulus_kind: &'static str,
        target_receipts: u8,
        receipt_installation_matched: bool,
        policy_failures: u8,
        action_before_policy_failure: bool,
        browser_process_exited_once: bool,
        #[serde(flatten)]
        facts: NativeTopRefusalFacts,
        #[serde(skip)]
        installation: Option<NativeRefusalInstallReceipt>,
    }
    impl TopRefusalEvidence {
        fn facts_positive(facts: &NativeTopRefusalFacts) -> bool {
            facts.event_kind == "topNavigationStarting"
                && facts.action_kind == "setCancelTrue"
                && facts.target_uri_matched
                && facts.native_sender_matched
                && facts.request_generation_live_matched
                && facts.navigation_id_getter_succeeded
                && facts.redirected_getter_succeeded
                && !facts.is_redirected
                && facts.issued_before_event
                && facts.cancel_attempts == 1
                && facts.cancel_returns == 1
                && facts.cancel_successes == 1
                && facts.cancel_readbacks == 1
                && facts.cancel_readback_succeeded
                && facts.native_cancel_true
                && facts.live_after_getters
                && facts.native_action_before_receipt
        }
        fn submission(
            &mut self,
            receipt: NativeTopSubmissionReceipt,
            request: &Code0WindowRequest,
        ) -> bool {
            self.submissions = self.submissions.saturating_add(1);
            self.submission_returned = true;
            let matched = self
                .installation
                .as_ref()
                .is_some_and(|install| receipt.matches(request, install));
            self.asset_binding_validated = matched;
            self.installation_before_stimulus = matched
                && self.installation.as_ref().is_some_and(|install| {
                    install.installed_mask() == 1023 && !install.load_rejected()
                });
            self.stimulus_kind = "ownedFileTopNavigation";
            self.native_submit_ok = matched && receipt.native_submit_ok();
            self.submissions == 1 && self.native_submit_ok && receipt.original_attempt_timely()
        }
        fn observe(
            &mut self,
            receipt: NativeTopRefusalReceipt,
            request: &Code0WindowRequest,
        ) -> bool {
            self.target_receipts = self.target_receipts.saturating_add(1);
            self.receipt_installation_matched = self
                .installation
                .as_ref()
                .is_some_and(|install| receipt.matches(request, install));
            self.facts = receipt.facts();
            self.target_receipts == 1
                && self.receipt_installation_matched
                && Self::facts_positive(&self.facts)
        }
        fn positive(&self, native: &NativeRefusalEvidence, lifecycle: &Summary) -> bool {
            self.owned_file_prepared
                && self.submissions == 1
                && self.submission_returned
                && self.native_submit_ok
                && self.asset_binding_validated
                && self.installation_before_stimulus
                && self.stimulus_kind == "ownedFileTopNavigation"
                && self.target_receipts == 1
                && self.receipt_installation_matched
                && self.policy_failures == 1
                && self.action_before_policy_failure
                && self.browser_process_exited_once
                && Self::facts_positive(&self.facts)
                && native.receipts == 1
                && native.native_instance_validated
                && native.source_bound_registry
                && native.required_mask == 1023
                && native.installed_mask == 1023
                && native.not_installed_hooks.is_empty()
                && !native.navigation_not_issued
                && !native.load_rejected
                && lifecycle.sta_main
                && lifecycle.callback_affinity
                && lifecycle.no_inherited_override
                && lifecycle.engine_ready
                && lifecycle.bare_hidden_unfocused
                && lifecycle.construction_observed
                && !lifecycle.bare_window_violation
                && lifecycle.profile_isolated
                && lifecycle.destroyed
                && lifecycle.owned_references_released
        }
    }

    // N2 rejects a successful submit without the actual registered child action,
    // the real ExecuteScript completion, genuine Session failure and ref0 audit.
    #[derive(Default, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ChildRefusalEvidence {
        submissions: u8,
        submission_returned: bool,
        native_submit_ok: bool,
        initial_source_matched: bool,
        installation_before_stimulus: bool,
        stimulus_kind: &'static str,
        target_receipts: u8,
        receipt_installation_matched: bool,
        completions: u8,
        completion_identity_matched: bool,
        completion_hresult_succeeded: bool,
        bounded_result_valid: bool,
        fixed_result_matched: bool,
        completion_before_release: bool,
        completion_within_deadline: bool,
        final_receipts: u8,
        completion_obligation_drained: bool,
        completion_handlers_drained: bool,
        policy_failures: u8,
        action_before_policy_failure: bool,
        browser_process_exited_once: bool,
        #[serde(flatten)]
        facts: NativeChildRefusalFacts,
        #[serde(skip)]
        installation: Option<NativeRefusalInstallReceipt>,
    }
    impl ChildRefusalEvidence {
        fn facts_positive(f: &NativeChildRefusalFacts) -> bool {
            f.event_kind == "frameNavigationStarting"
                && f.action_kind == "setCancelTrue"
                && f.target_uri_matched
                && f.native_sender_matched
                && f.request_generation_live_matched
                && f.navigation_id_getter_succeeded
                && f.redirected_getter_succeeded
                && !f.is_redirected
                && f.issued_before_event
                && f.cancel_attempts == 1
                && f.cancel_returns == 1
                && f.cancel_successes == 1
                && f.cancel_readbacks == 1
                && f.cancel_readback_succeeded
                && f.native_cancel_true
                && f.live_after_getters
                && f.native_action_before_receipt
        }
        fn submission(
            &mut self,
            r: NativeChildSubmissionReceipt,
            request: &Code0WindowRequest,
        ) -> bool {
            self.submissions = self.submissions.saturating_add(1);
            self.submission_returned = true;
            let matched = self
                .installation
                .as_ref()
                .is_some_and(|i| r.matches(request, i));
            self.initial_source_matched = matched && r.initial_source_matched();
            self.installation_before_stimulus = matched
                && self
                    .installation
                    .as_ref()
                    .is_some_and(|i| i.installed_mask() == 1023 && !i.load_rejected());
            self.stimulus_kind = "initialBlankFixedDataChild";
            self.native_submit_ok = matched && r.native_submit_ok();
            self.submissions == 1
                && self.initial_source_matched
                && self.native_submit_ok
                && r.original_attempt_timely()
        }
        fn observe(&mut self, r: NativeChildRefusalReceipt, request: &Code0WindowRequest) -> bool {
            self.target_receipts = self.target_receipts.saturating_add(1);
            self.receipt_installation_matched = self
                .installation
                .as_ref()
                .is_some_and(|i| r.matches(request, i));
            self.facts = r.facts();
            self.target_receipts == 1
                && self.receipt_installation_matched
                && Self::facts_positive(&self.facts)
        }
        fn completion(
            &mut self,
            r: NativeChildCompletionReceipt,
            request: &Code0WindowRequest,
        ) -> bool {
            self.completions = self.completions.saturating_add(1);
            self.completion_identity_matched = self
                .installation
                .as_ref()
                .is_some_and(|i| r.matches(request, i));
            let f = r.facts();
            self.completion_identity_matched &= f.identity_matched;
            self.completion_hresult_succeeded = f.hresult_succeeded;
            self.bounded_result_valid = f.bounded_result_valid;
            self.fixed_result_matched = f.fixed_result_matched;
            self.completion_before_release = f.before_release;
            self.completion_within_deadline = f.within_deadline;
            self.completions == 1
                && self.completion_identity_matched
                && self.completion_hresult_succeeded
                && self.bounded_result_valid
                && self.fixed_result_matched
                && self.completion_before_release
                && self.completion_within_deadline
        }
        fn final_receipt(
            &mut self,
            r: NativeChildFinalReceipt,
            request: &Code0WindowRequest,
        ) -> bool {
            self.final_receipts = self.final_receipts.saturating_add(1);
            let matched = self
                .installation
                .as_ref()
                .is_some_and(|i| r.matches(request, i));
            self.completion_obligation_drained = matched && r.obligation_drained();
            self.completion_handlers_drained = matched && r.handlers_drained();
            self.final_receipts == 1
                && self.completion_obligation_drained
                && self.completion_handlers_drained
        }
        fn positive(&self, n: &NativeRefusalEvidence, lifecycle: &Summary) -> bool {
            self.submissions == 1
                && self.submission_returned
                && self.native_submit_ok
                && self.initial_source_matched
                && self.installation_before_stimulus
                && self.stimulus_kind == "initialBlankFixedDataChild"
                && self.target_receipts == 1
                && self.receipt_installation_matched
                && self.completions == 1
                && self.completion_identity_matched
                && self.completion_hresult_succeeded
                && self.bounded_result_valid
                && self.fixed_result_matched
                && self.completion_before_release
                && self.completion_within_deadline
                && self.final_receipts == 1
                && self.completion_obligation_drained
                && self.completion_handlers_drained
                && self.policy_failures == 1
                && self.action_before_policy_failure
                && self.browser_process_exited_once
                && Self::facts_positive(&self.facts)
                && n.receipts == 1
                && n.native_instance_validated
                && n.source_bound_registry
                && n.required_mask == 1023
                && n.installed_mask == 1023
                && n.not_installed_hooks.is_empty()
                && !n.navigation_not_issued
                && !n.load_rejected
                && lifecycle.sta_main
                && lifecycle.callback_affinity
                && lifecycle.no_inherited_override
                && lifecycle.engine_ready
                && lifecycle.bare_hidden_unfocused
                && lifecycle.construction_observed
                && !lifecycle.bare_window_violation
                && lifecycle.profile_isolated
                && lifecycle.destroyed
                && lifecycle.owned_references_released
        }
    }

    // This positive oracle catches premature installation readiness, a first
    // load before actual async ACKs, and a document without native Source/bridge
    // evidence. Submission Ok and EngineReady cannot satisfy any receipt.
    #[derive(Default, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct PolicyEvidence {
        submissions: u8,
        submission_accepted: bool,
        install_rejected: bool,
        barrier_receipts: u8,
        script_acknowledged: bool,
        cdp_acknowledged: bool,
        first_load_receipts: u8,
        document_receipts: u8,
        source_matched: bool,
        synthetic_response_applied: bool,
        canonical_ready_control: bool,
        ack_before_first_load: bool,
        #[serde(skip)]
        native_refusal: Option<NativeRefusalEvidence>,
        #[serde(skip)]
        top_refusal: Option<TopRefusalEvidence>,
        #[serde(skip)]
        child_refusal: Option<ChildRefusalEvidence>,
        #[serde(skip)]
        dialog_refusal: Option<DialogRefusalEvidence>,
        #[serde(skip)]
        popup_refusal: Option<PopupRefusalEvidence>,
        #[serde(skip)]
        download_refusal: Option<DownloadRefusalEvidence>,
        #[serde(skip)]
        chooser: Option<ChooserEvidence>,
        #[serde(skip)]
        capability: Option<CapabilityEvidence>,
    }

    impl PolicyEvidence {
        fn positive(&self, lifecycle: &Summary) -> bool {
            if let Some(capability) = &self.capability {
                return self
                    .native_refusal
                    .as_ref()
                    .is_some_and(|native| capability.positive(native, lifecycle));
            }
            if let Some(chooser) = &self.chooser {
                return self
                    .native_refusal
                    .as_ref()
                    .is_some_and(|native| chooser.positive(native, lifecycle));
            }
            if let Some(download) = &self.download_refusal {
                return self
                    .native_refusal
                    .as_ref()
                    .is_some_and(|native| download.positive(native, lifecycle));
            }
            if let Some(popup) = &self.popup_refusal {
                return self
                    .native_refusal
                    .as_ref()
                    .is_some_and(|native| popup.positive(native, lifecycle));
            }
            if let Some(dialog) = &self.dialog_refusal {
                return self
                    .native_refusal
                    .as_ref()
                    .is_some_and(|native| dialog.positive(native, lifecycle));
            }
            if let Some(child) = &self.child_refusal {
                return self
                    .native_refusal
                    .as_ref()
                    .is_some_and(|native| child.positive(native, lifecycle));
            }
            if let Some(top) = &self.top_refusal {
                return self
                    .native_refusal
                    .as_ref()
                    .is_some_and(|native| top.positive(native, lifecycle));
            }
            if let Some(native) = &self.native_refusal {
                return native.positive(lifecycle);
            }
            self.submissions == 1
                && self.submission_accepted
                && !self.install_rejected
                && self.barrier_receipts == 1
                && self.script_acknowledged
                && self.cdp_acknowledged
                && self.first_load_receipts == 1
                && self.document_receipts == 1
                && self.source_matched
                && self.synthetic_response_applied
                && self.canonical_ready_control
                && self.ack_before_first_load
                && lifecycle.sta_main
                && lifecycle.callback_affinity
                && lifecycle.no_inherited_override
                && lifecycle.engine_ready
                && lifecycle.bare_hidden_unfocused
                && lifecycle.construction_observed
                && !lifecycle.bare_window_violation
                && lifecycle.profile_isolated
                && lifecycle.destroyed
                && lifecycle.owned_references_released
        }

        fn observe(&mut self, event: PolicyObservation, request: &Code0WindowRequest) -> bool {
            match event {
                PolicyObservation::FaultTarget(_)
                | PolicyObservation::FaultApplied(_)
                | PolicyObservation::FaultFinal(_)
                | PolicyObservation::StaleDisposition(_)
                | PolicyObservation::Containment(_) => false,
                PolicyObservation::BarrierReady(receipt) => {
                    self.barrier_receipts = self.barrier_receipts.saturating_add(1);
                    let valid = receipt.matches(request)
                        && self.submissions == 1
                        && self.submission_accepted
                        && !self.install_rejected
                        && self.barrier_receipts == 1
                        && self.first_load_receipts == 0
                        && self.document_receipts == 0;
                    self.script_acknowledged = valid && receipt.script_acknowledged();
                    self.cdp_acknowledged = valid && receipt.cdp_acknowledged();
                    valid && self.script_acknowledged && self.cdp_acknowledged
                }
                PolicyObservation::FirstControlledLoad(receipt) => {
                    self.first_load_receipts = self.first_load_receipts.saturating_add(1);
                    self.ack_before_first_load = receipt.matches(request)
                        && self.barrier_receipts == 1
                        && self.script_acknowledged
                        && self.cdp_acknowledged
                        && self.first_load_receipts == 1
                        && self.document_receipts == 0;
                    self.ack_before_first_load
                }
                PolicyObservation::ControlledDocumentReady(receipt) => {
                    self.document_receipts = self.document_receipts.saturating_add(1);
                    let valid = receipt.matches(request)
                        && self.ack_before_first_load
                        && self.first_load_receipts == 1
                        && self.document_receipts == 1;
                    self.source_matched = valid && receipt.source_matched();
                    self.synthetic_response_applied = valid && receipt.synthetic_response_applied();
                    self.canonical_ready_control = valid && receipt.canonical_ready_control();
                    valid
                        && self.source_matched
                        && self.synthetic_response_applied
                        && self.canonical_ready_control
                }
            }
        }
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct PolicyLifecycle<'a> {
        sta_main: bool,
        callback_affinity: bool,
        no_inherited_override: bool,
        engine_ready: bool,
        bare_hidden_unfocused: bool,
        construction_observed: bool,
        bare_window_violation: bool,
        profile_isolated: bool,
        destroyed: bool,
        owned_references_released: bool,
        run_returned: bool,
        cleanup_succeeded: bool,
        runtime_version: &'a str,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct PolicySummary<'a> {
        schema: u8,
        run_id: &'a str,
        required_case: &'static str,
        selected_cases: u8,
        completed_cases: u8,
        pass: bool,
        failure: &'static str,
        install: &'a PolicyEvidence,
        lifecycle: PolicyLifecycle<'a>,
        #[serde(skip_serializing_if = "Option::is_none")]
        native_refusal: Option<&'a NativeRefusalEvidence>,
        #[serde(skip_serializing_if = "Option::is_none")]
        top_refusal: Option<&'a TopRefusalEvidence>,
        #[serde(skip_serializing_if = "Option::is_none")]
        child_refusal: Option<&'a ChildRefusalEvidence>,
        #[serde(skip_serializing_if = "Option::is_none")]
        dialog_refusal: Option<&'a DialogRefusalEvidence>,
        #[serde(skip_serializing_if = "Option::is_none")]
        popup_refusal: Option<&'a PopupRefusalEvidence>,
        #[serde(skip_serializing_if = "Option::is_none")]
        download_refusal: Option<&'a DownloadRefusalEvidence>,
        #[serde(skip_serializing_if = "Option::is_none")]
        chooser: Option<&'a ChooserEvidence>,
        #[serde(skip_serializing_if = "Option::is_none")]
        capability: Option<&'a NativeCapabilityFacts>,
    }

    impl<'a> PolicySummary<'a> {
        fn snapshot(state: &'a Summary, install: &'a PolicyEvidence) -> Self {
            Self {
                schema: if install.capability.is_some() {
                    13
                } else if install.chooser.is_some() {
                    11
                } else if install.download_refusal.is_some() {
                    12
                } else if install.popup_refusal.is_some() {
                    10
                } else if install.dialog_refusal.is_some() {
                    9
                } else if install.child_refusal.is_some() {
                    8
                } else if install.top_refusal.is_some() {
                    7
                } else if install.native_refusal.is_some() {
                    6
                } else {
                    1
                },
                run_id: &state.run_id,
                required_case: if let Some(capability) = &install.capability {
                    capability.facts.selector
                } else if install.chooser.is_some() {
                    CHOOSER_CASE
                } else if install.download_refusal.is_some() {
                    DOWNLOAD_REFUSAL_CASE
                } else if install.popup_refusal.is_some() {
                    POPUP_REFUSAL_CASE
                } else if install.dialog_refusal.is_some() {
                    DIALOG_REFUSAL_CASE
                } else if install.child_refusal.is_some() {
                    CHILD_REFUSAL_CASE
                } else if install.top_refusal.is_some() {
                    TOP_REFUSAL_CASE
                } else if install.native_refusal.is_some() {
                    NATIVE_REFUSAL_CASE
                } else {
                    POLICY_CASE
                },
                selected_cases: 1,
                completed_cases: state.completed_cases,
                pass: state.pass,
                failure: state.failure,
                install,
                native_refusal: install.native_refusal.as_ref(),
                top_refusal: install.top_refusal.as_ref(),
                child_refusal: install.child_refusal.as_ref(),
                dialog_refusal: install.dialog_refusal.as_ref(),
                popup_refusal: install.popup_refusal.as_ref(),
                download_refusal: install.download_refusal.as_ref(),
                chooser: install.chooser.as_ref(),
                capability: install.capability.as_ref().map(|c| &c.facts),
                lifecycle: PolicyLifecycle {
                    sta_main: state.sta_main,
                    callback_affinity: state.callback_affinity,
                    no_inherited_override: state.no_inherited_override,
                    engine_ready: state.engine_ready,
                    bare_hidden_unfocused: state.bare_hidden_unfocused,
                    construction_observed: state.construction_observed,
                    bare_window_violation: state.bare_window_violation,
                    profile_isolated: state.profile_isolated,
                    destroyed: state.destroyed,
                    owned_references_released: state.owned_references_released,
                    run_returned: state.run_returned,
                    cleanup_succeeded: state.cleanup_succeeded,
                    runtime_version: &state.runtime_version,
                },
            }
        }
    }

    // Future-positive oracle: removing the post-success abort, injecting before
    // the selected successful boundary, or accepting an early blanket denial
    // must fail. Target/final receipts come only from the real native owner.
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct FaultEvidence {
        target: &'static str,
        target_receipts: u32,
        injection_receipts: u32,
        premature_load_denials: u32,
        navigate_attempts: u32,
        script_submissions: u32,
        cdp_submissions: u32,
        resource_adds: u32,
        filter_adds: u32,
        resource_removals: u32,
        filter_removals: u32,
        script_removals: u32,
        policy_failures: u32,
        #[serde(skip)]
        case: PolicyFixtureCase,
        #[serde(skip)]
        target_events: u8,
        #[serde(skip)]
        final_events: u8,
        #[serde(skip)]
        application: [bool; 3],
        #[serde(skip)]
        target_application: Option<[bool; 3]>,
    }

    impl FaultEvidence {
        fn new(case: PolicyFixtureCase) -> Self {
            Self {
                target: case.fault_target().unwrap().name(),
                target_receipts: 0,
                injection_receipts: 0,
                premature_load_denials: 0,
                navigate_attempts: 0,
                script_submissions: 0,
                cdp_submissions: 0,
                resource_adds: 0,
                filter_adds: 0,
                resource_removals: 0,
                filter_removals: 0,
                script_removals: 0,
                policy_failures: 0,
                case,
                target_events: 0,
                final_events: 0,
                application: [false; 3],
                target_application: None,
            }
        }

        fn observe(
            &mut self,
            receipt: PolicyFaultReceipt,
            request: &Code0WindowRequest,
            final_snapshot: bool,
        ) -> bool {
            if !receipt.matches(request) || Some(receipt.target()) != self.case.fault_target() {
                return false;
            }
            let c = receipt.counts();
            let application = receipt.application();
            if application
                .iter()
                .zip(self.application)
                .any(|(current, prior)| prior && !*current)
                || (!final_snapshot
                    && application
                        != match self.case {
                            PolicyFixtureCase::InstallScriptInjectedFailure => [true, false, false],
                            PolicyFixtureCase::InstallCdpInjectedFailure => [true, true, false],
                            _ => [false; 3],
                        })
            {
                return false;
            }
            if final_snapshot {
                self.final_events = self.final_events.saturating_add(1);
                if self.final_events != 1 || c[0] != self.target_receipts {
                    return false;
                }
            } else {
                self.target_events = self.target_events.saturating_add(1);
                if self.target_events != 1 || self.final_events != 0 || c[0] != 1 {
                    return false;
                }
            }
            let previous = [
                self.target_receipts,
                self.injection_receipts,
                self.premature_load_denials,
                self.navigate_attempts,
                self.script_submissions,
                self.cdp_submissions,
                self.resource_adds,
                self.filter_adds,
                self.resource_removals,
                self.filter_removals,
                self.script_removals,
            ];
            if c.iter()
                .zip(previous)
                .any(|(current, prior)| *current < prior)
            {
                return false;
            }
            [
                self.target_receipts,
                self.injection_receipts,
                self.premature_load_denials,
                self.navigate_attempts,
                self.script_submissions,
                self.cdp_submissions,
                self.resource_adds,
                self.filter_adds,
                self.resource_removals,
                self.filter_removals,
                self.script_removals,
            ] = c;
            self.application = application;
            if !final_snapshot {
                self.target_application = Some(application);
            }
            true
        }

        fn observe_applied(
            &mut self,
            receipt: PolicyFaultReceipt,
            request: &Code0WindowRequest,
        ) -> bool {
            let c = receipt.counts();
            if !receipt.matches(request)
                || Some(receipt.target()) != self.case.fault_target()
                || self.case == PolicyFixtureCase::InstallPrematureLoadDenied
                || self.target_events != 1
                || self.target_receipts != 1
                || self.final_events != 0
                || self.injection_receipts != 0
                || Some(receipt.application()) != self.target_application
                || c != [
                    self.target_receipts,
                    1,
                    self.premature_load_denials,
                    self.navigate_attempts,
                    self.script_submissions,
                    self.cdp_submissions,
                    self.resource_adds,
                    self.filter_adds,
                    self.resource_removals,
                    self.filter_removals,
                    self.script_removals,
                ]
            {
                return false;
            }
            self.injection_receipts = c[1];
            true
        }

        fn positive(&self, install: &PolicyEvidence, state: &Summary) -> bool {
            let p0 = self.case == PolicyFixtureCase::InstallPrematureLoadDenied;
            let asynchronous = self.case.async_application();
            let p5 = self.case == PolicyFixtureCase::InstallCdpInjectedFailure;
            let (resources, filters) = match self.case {
                PolicyFixtureCase::InstallAddInjectedFailure => (1, 0),
                PolicyFixtureCase::InstallFilterInjectedFailure
                | PolicyFixtureCase::InstallScriptInjectedFailure
                | PolicyFixtureCase::InstallCdpInjectedFailure => (1, 1),
                _ => (0, 0),
            };
            self.target_events == 1
                && self.final_events == 1
                && self.target_receipts == 1
                && self.injection_receipts == u32::from(!p0)
                && self.premature_load_denials == u32::from(p0)
                && self.navigate_attempts == 0
                && self.script_submissions == u32::from(asynchronous)
                && self.cdp_submissions == u32::from(p5)
                && self.script_removals == u32::from(asynchronous)
                && self.application == [asynchronous, p5, false]
                && self.resource_adds == resources
                && self.resource_removals == resources
                && self.filter_adds == filters
                && self.filter_removals == filters
                && self.policy_failures == u32::from(!p0)
                && install.submissions == u8::from(!p0)
                && install.submission_accepted == asynchronous
                && install.install_rejected == (!p0 && !asynchronous)
                && install.barrier_receipts == 0
                && install.script_acknowledged == p5
                && !install.cdp_acknowledged
                && install.first_load_receipts == 0
                && install.document_receipts == 0
                && !install.source_matched
                && !install.synthetic_response_applied
                && !install.canonical_ready_control
                && !install.ack_before_first_load
                && state.sta_main
                && state.callback_affinity
                && state.no_inherited_override
                && state.engine_ready
                && state.bare_hidden_unfocused
                && state.construction_observed
                && !state.bare_window_violation
                && state.profile_isolated
                && state.destroyed
                && state.owned_references_released
        }
    }

    #[derive(Default, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ApplicationEvidence {
        script_id_inventoried: bool,
        script_acknowledged: bool,
        cdp_acknowledged: bool,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct FaultSummary<'a> {
        schema: u8,
        run_id: &'a str,
        required_case: &'static str,
        selected_cases: u8,
        completed_cases: u8,
        pass: bool,
        failure: &'static str,
        install: &'a PolicyEvidence,
        lifecycle: PolicyLifecycle<'a>,
        fault: &'a FaultEvidence,
        #[serde(skip_serializing_if = "Option::is_none")]
        application: Option<ApplicationEvidence>,
    }

    impl<'a> FaultSummary<'a> {
        fn snapshot(
            state: &'a Summary,
            install: &'a PolicyEvidence,
            fault: &'a FaultEvidence,
        ) -> Self {
            Self {
                schema: if fault.case.async_application() { 3 } else { 2 },
                run_id: &state.run_id,
                required_case: fault.case.name(),
                selected_cases: 1,
                completed_cases: state.completed_cases,
                pass: state.pass,
                failure: state.failure,
                install,
                lifecycle: PolicySummary::snapshot(state, install).lifecycle,
                fault,
                application: fault
                    .case
                    .async_application()
                    .then_some(ApplicationEvidence {
                        script_id_inventoried: fault.application[0],
                        script_acknowledged: fault.application[1],
                        cdp_acknowledged: fault.application[2],
                    }),
            }
        }
    }

    #[derive(Default, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct NativeOperations {
        target_receipts: u32,
        injection_receipts: u32,
        premature_load_denials: u32,
        navigate_attempts: u32,
        script_submissions: u32,
        cdp_submissions: u32,
        resource_adds: u32,
        filter_adds: u32,
        resource_removals: u32,
        filter_removals: u32,
        script_removals: u32,
    }

    #[derive(Default, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct StaleEvidence {
        held_receipts: u8,
        close_revocations: u32,
        drained_receipts: u8,
        final_receipts: u8,
        held_application: ApplicationEvidence,
        queued_script_id_at_held: bool,
        policy_script_ids_at_held: u32,
        policy_script_ids_at_drain: u32,
        policy_script_ids_at_final: u32,
        policy_failures: u32,
    }

    #[derive(Default, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ContainmentEvidence {
        asset_deliveries: [u32; 3],
        asset_responses: [u32; 3],
        probe_deliveries: [u32; 3],
        probe_responses: [u32; 3],
        other_deliveries: u32,
        other_responses: u32,
        total_delivered: u32,
        complete_receipts: u8,
        final_receipts: u8,
        policy_failures: u32,
    }

    #[derive(Default, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct RendererEvidence {
        main_refusal: bool,
        child_refusal: bool,
        worker_refusal: bool,
        child_detached: bool,
        worker_terminate_returned: bool,
    }

    struct CoverageEvidence {
        case: PolicyFixtureCase,
        counts: [u32; 11],
        facts: [bool; 3],
        native: NativeOperations,
        application: ApplicationEvidence,
        stale: StaleEvidence,
        containment: ContainmentEvidence,
        renderer: RendererEvidence,
    }

    impl CoverageEvidence {
        fn new(case: PolicyFixtureCase) -> Self {
            Self {
                case,
                counts: [0; 11],
                facts: [false; 3],
                native: NativeOperations::default(),
                application: ApplicationEvidence::default(),
                stale: StaleEvidence::default(),
                containment: ContainmentEvidence::default(),
                renderer: RendererEvidence::default(),
            }
        }

        fn snapshot(
            &mut self,
            counts: [u32; 11],
            facts: [bool; 3],
            install: &mut PolicyEvidence,
        ) -> bool {
            if counts
                .iter()
                .zip(self.counts)
                .any(|(now, prior)| *now < prior)
                || facts
                    .iter()
                    .zip(self.facts)
                    .any(|(now, prior)| prior && !*now)
            {
                return false;
            }
            self.counts = counts;
            self.facts = facts;
            [
                self.native.target_receipts,
                self.native.injection_receipts,
                self.native.premature_load_denials,
                self.native.navigate_attempts,
                self.native.script_submissions,
                self.native.cdp_submissions,
                self.native.resource_adds,
                self.native.filter_adds,
                self.native.resource_removals,
                self.native.filter_removals,
                self.native.script_removals,
            ] = counts;
            self.application = ApplicationEvidence {
                script_id_inventoried: facts[0],
                script_acknowledged: facts[1],
                cdp_acknowledged: facts[2],
            };
            install.script_acknowledged = facts[1];
            install.cdp_acknowledged = facts[2];
            true
        }

        fn observe_stale(
            &mut self,
            receipt: StaleDispositionReceipt,
            request: &Code0WindowRequest,
            install: &mut PolicyEvidence,
            closing: bool,
        ) -> bool {
            let script = self.case == PolicyFixtureCase::InstallScriptStaleDisposition;
            if !self.case.stale_disposition()
                || !receipt.matches(request)
                || receipt.script() != script
                || !self.snapshot(receipt.counts(), receipt.application(), install)
            {
                return false;
            }
            let order = receipt.order();
            match receipt.stage() {
                StaleDispositionStage::Held => {
                    if closing
                        || self.stale.held_receipts != 0
                        || order != [1, 0, 0, 0]
                        || receipt.application()
                            != if script {
                                [false; 3]
                            } else {
                                [true, true, false]
                            }
                        || receipt.queued_script_id() != script
                        || receipt.policy_script_ids() != u32::from(!script)
                    {
                        return false;
                    }
                    self.stale.held_receipts += 1;
                    let f = receipt.application();
                    self.stale.held_application = ApplicationEvidence {
                        script_id_inventoried: f[0],
                        script_acknowledged: f[1],
                        cdp_acknowledged: f[2],
                    };
                    self.stale.queued_script_id_at_held = receipt.queued_script_id();
                    self.stale.policy_script_ids_at_held = receipt.policy_script_ids();
                }
                StaleDispositionStage::Drained => {
                    if !closing
                        || self.stale.held_receipts != 1
                        || self.stale.drained_receipts != 0
                        || order != [1, 1, 1, 0]
                        || receipt.queued_script_id()
                        || receipt.policy_script_ids() != 1
                    {
                        return false;
                    }
                    self.stale.drained_receipts += 1;
                    self.stale.policy_script_ids_at_drain = receipt.policy_script_ids();
                }
                StaleDispositionStage::Final => {
                    if !closing
                        || self.stale.drained_receipts != 1
                        || self.stale.final_receipts != 0
                        || order != [1, 1, 1, 1]
                        || receipt.queued_script_id()
                        || receipt.policy_script_ids() != 0
                    {
                        return false;
                    }
                    self.stale.final_receipts += 1;
                    self.stale.policy_script_ids_at_final = receipt.policy_script_ids();
                }
            }
            self.stale.close_revocations = order[1];
            true
        }

        fn observe_containment(
            &mut self,
            receipt: ContainmentReceipt,
            request: &Code0WindowRequest,
            install: &mut PolicyEvidence,
            closing: bool,
        ) -> bool {
            if self.case != PolicyFixtureCase::InstallRealmsNoPassthrough
                || !receipt.matches(request)
                || !self.snapshot(receipt.counts(), receipt.application(), install)
            {
                return false;
            }
            let d = receipt.deliveries();
            let a = receipt.responses();
            let prior_d = [
                self.containment.asset_deliveries[0],
                self.containment.asset_deliveries[1],
                self.containment.asset_deliveries[2],
                self.containment.probe_deliveries[0],
                self.containment.probe_deliveries[1],
                self.containment.probe_deliveries[2],
                self.containment.other_deliveries,
            ];
            let prior_a = [
                self.containment.asset_responses[0],
                self.containment.asset_responses[1],
                self.containment.asset_responses[2],
                self.containment.probe_responses[0],
                self.containment.probe_responses[1],
                self.containment.probe_responses[2],
                self.containment.other_responses,
            ];
            if d.iter().zip(prior_d).any(|(now, prior)| *now < prior)
                || a.iter().zip(prior_a).any(|(now, prior)| *now < prior)
                || receipt.renderer() != [true; 5]
                || receipt.document() != [true; 3]
            {
                return false;
            }
            if receipt.final_snapshot() {
                if !closing
                    || self.containment.complete_receipts != 1
                    || self.containment.final_receipts != 0
                {
                    return false;
                }
                self.containment.final_receipts += 1;
            } else {
                if closing
                    || self.containment.complete_receipts != 0
                    || install.first_load_receipts != 1
                {
                    return false;
                }
                self.containment.complete_receipts += 1;
                install.document_receipts = install.document_receipts.saturating_add(1);
                let document = receipt.document();
                install.source_matched = document[0];
                install.synthetic_response_applied = document[1];
                install.canonical_ready_control = document[2];
            }
            self.containment.asset_deliveries.copy_from_slice(&d[..3]);
            self.containment.asset_responses.copy_from_slice(&a[..3]);
            self.containment.probe_deliveries.copy_from_slice(&d[3..6]);
            self.containment.probe_responses.copy_from_slice(&a[3..6]);
            self.containment.other_deliveries = d[6];
            self.containment.other_responses = a[6];
            self.containment.total_delivered = receipt.total_delivered();
            let page = receipt.renderer();
            self.renderer = RendererEvidence {
                main_refusal: page[0],
                child_refusal: page[1],
                worker_refusal: page[2],
                child_detached: page[3],
                worker_terminate_returned: page[4],
            };
            true
        }

        fn positive(&self, install: &PolicyEvidence, state: &Summary) -> bool {
            let stale = self.case.stale_disposition();
            let cdp = self.case == PolicyFixtureCase::InstallCdpStaleDisposition;
            let scenario = if stale {
                self.counts == [0, 0, 0, 0, 1, u32::from(cdp), 1, 1, 1, 1, 1]
                    && self.facts == [true, cdp, false]
                    && self.stale.held_receipts == 1
                    && self.stale.close_revocations == 1
                    && self.stale.drained_receipts == 1
                    && self.stale.final_receipts == 1
                    && self.stale.policy_failures == 1
                    && self.stale.queued_script_id_at_held != cdp
                    && self.stale.policy_script_ids_at_held == u32::from(cdp)
                    && self.stale.policy_script_ids_at_drain == 1
                    && self.stale.policy_script_ids_at_final == 0
                    && install.submissions == 1
                    && install.submission_accepted
                    && !install.install_rejected
                    && install.barrier_receipts == 0
                    && install.script_acknowledged == cdp
                    && !install.cdp_acknowledged
                    && install.first_load_receipts == 0
                    && install.document_receipts == 0
                    && !install.source_matched
                    && !install.synthetic_response_applied
                    && !install.canonical_ready_control
                    && !install.ack_before_first_load
            } else {
                self.counts == [0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1]
                    && self.facts == [true; 3]
                    && self.containment.asset_deliveries == [1; 3]
                    && self.containment.asset_responses == [1; 3]
                    && self.containment.probe_deliveries == [1; 3]
                    && self.containment.probe_responses == [1; 3]
                    && self.containment.other_deliveries == self.containment.other_responses
                    && self.containment.total_delivered == 6 + self.containment.other_deliveries
                    && self.containment.total_delivered <= 16
                    && self.containment.complete_receipts == 1
                    && self.containment.final_receipts == 1
                    && self.containment.policy_failures == 0
                    && self.renderer.main_refusal
                    && self.renderer.child_refusal
                    && self.renderer.worker_refusal
                    && self.renderer.child_detached
                    && self.renderer.worker_terminate_returned
                    && install.positive(state)
            };
            scenario
                && state.sta_main
                && state.callback_affinity
                && state.no_inherited_override
                && state.engine_ready
                && state.bare_hidden_unfocused
                && state.construction_observed
                && !state.bare_window_violation
                && state.profile_isolated
                && state.destroyed
                && state.owned_references_released
        }
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct CoverageSummary<'a> {
        schema: u8,
        run_id: &'a str,
        required_case: &'static str,
        selected_cases: u8,
        completed_cases: u8,
        pass: bool,
        failure: &'static str,
        install: &'a PolicyEvidence,
        lifecycle: PolicyLifecycle<'a>,
        native: &'a NativeOperations,
        application: &'a ApplicationEvidence,
        #[serde(skip_serializing_if = "Option::is_none")]
        stale: Option<&'a StaleEvidence>,
        #[serde(skip_serializing_if = "Option::is_none")]
        containment: Option<&'a ContainmentEvidence>,
        #[serde(skip_serializing_if = "Option::is_none")]
        renderer: Option<&'a RendererEvidence>,
    }
    impl<'a> CoverageSummary<'a> {
        fn snapshot(
            state: &'a Summary,
            install: &'a PolicyEvidence,
            evidence: &'a CoverageEvidence,
        ) -> Self {
            let stale = evidence.case.stale_disposition();
            Self {
                schema: if stale { 4 } else { 5 },
                run_id: &state.run_id,
                required_case: evidence.case.name(),
                selected_cases: 1,
                completed_cases: state.completed_cases,
                pass: state.pass,
                failure: state.failure,
                install,
                lifecycle: PolicySummary::snapshot(state, install).lifecycle,
                native: &evidence.native,
                application: &evidence.application,
                stale: stale.then_some(&evidence.stale),
                containment: (!stale).then_some(&evidence.containment),
                renderer: (!stale).then_some(&evidence.renderer),
            }
        }
    }

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

    fn write_summary(file: &mut File, summary: &impl Serialize) -> std::io::Result<()> {
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
        write_summary(&mut file, &*summary.lock().unwrap())?;
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
        let outcome = run_case(
            Arc::clone(&summary),
            Arc::clone(&diagnostics),
            None,
            None,
            None,
        );
        if outcome.is_err() {
            diagnostics.phase(DP::SummaryLock, false);
            summary.lock().unwrap().failure = "fixtureSetupFailed";
            diagnostics.phase(DP::SummaryLock, true);
        }
        diagnostics.phase(DP::SummaryLock, false);
        let passed = summary.lock().unwrap().pass;
        diagnostics.phase(DP::SummaryLock, true);
        diagnostics.phase(DP::FinalSummary, false);
        let write_result = write_summary(&mut evidence.lock().unwrap(), &*summary.lock().unwrap());
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

    fn write_policy_summary(file: &mut File, summary: &PolicySummary<'_>) -> std::io::Result<()> {
        if summary.schema == 13 && serde_json::to_vec(summary)?.len() > 8192 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "S4 summary exceeds fixed bound",
            ));
        }
        if summary.schema == 11 && serde_json::to_vec(summary)?.len() > 8192 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "S3 summary exceeds fixed bound",
            ));
        }
        if summary.schema == 12 && serde_json::to_vec(summary)?.len() > 8192 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "N5 summary exceeds fixed bound",
            ));
        }
        if summary.schema == 10 && serde_json::to_vec(summary)?.len() > 8192 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "N4 summary exceeds fixed bound",
            ));
        }
        if summary.schema == 9 && serde_json::to_vec(summary)?.len() > 8192 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "N3 summary exceeds fixed bound",
            ));
        }
        write_summary(file, summary)
    }

    pub fn run_policy(native_refusal: bool) -> Result<bool, Box<dyn std::error::Error>> {
        run_policy_selected(native_refusal, false, false, false, false, false, false)
    }

    pub fn run_top_refusal() -> Result<bool, Box<dyn std::error::Error>> {
        run_policy_selected(true, true, false, false, false, false, false)
    }

    pub fn run_child_refusal() -> Result<bool, Box<dyn std::error::Error>> {
        run_policy_selected(true, false, true, false, false, false, false)
    }

    pub fn run_dialog_refusal() -> Result<bool, Box<dyn std::error::Error>> {
        run_policy_selected(true, false, false, true, false, false, false)
    }

    pub fn run_popup_refusal() -> Result<bool, Box<dyn std::error::Error>> {
        run_policy_selected(true, false, false, false, true, false, false)
    }

    pub fn run_download_refusal() -> Result<bool, Box<dyn std::error::Error>> {
        run_policy_selected(true, false, false, false, false, true, false)
    }

    pub fn run_chooser() -> Result<bool, Box<dyn std::error::Error>> {
        run_policy_selected(true, false, false, false, false, false, true)
    }

    fn run_policy_selected(
        native_refusal: bool,
        top_refusal: bool,
        child_refusal: bool,
        dialog_refusal: bool,
        popup_refusal: bool,
        download_refusal: bool,
        chooser_selected: bool,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        run_policy_selected_with_capability(
            (
                native_refusal,
                top_refusal,
                child_refusal,
                dialog_refusal,
                popup_refusal,
                download_refusal,
                chooser_selected,
            ),
            None,
        )
    }
    pub fn run_capability(
        selector: NativeCapabilitySelector,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        run_policy_selected_with_capability(
            (true, false, false, false, false, false, false),
            Some(selector),
        )
    }
    fn run_policy_selected_with_capability(
        selection: (bool, bool, bool, bool, bool, bool, bool),
        capability_selector: Option<NativeCapabilitySelector>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let (
            native_refusal,
            top_refusal,
            child_refusal,
            dialog_refusal,
            popup_refusal,
            download_refusal,
            chooser_selected,
        ) = selection;
        let run_id = std::env::var("AI_METER_CODE0_POLICY_RUN_ID")?;
        if run_id.is_empty()
            || run_id.len() > 64
            || !run_id
                .bytes()
                .all(|v| v.is_ascii_alphanumeric() || v == b'-' || v == b'_')
        {
            return Err("invalid policy fixture run ID".into());
        }
        let output = PathBuf::from(
            std::env::var_os("AI_METER_CODE0_POLICY_SUMMARY")
                .ok_or("missing fresh policy summary path")?,
        );
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)?;
        let summary = Arc::new(Mutex::new(Summary {
            run_id,
            failure: "notCompleted",
            ..Summary::default()
        }));
        let install = Arc::new(Mutex::new(PolicyEvidence {
            native_refusal: native_refusal.then(NativeRefusalEvidence::default),
            top_refusal: top_refusal.then(TopRefusalEvidence::default),
            child_refusal: child_refusal.then(ChildRefusalEvidence::default),
            dialog_refusal: dialog_refusal.then(DialogRefusalEvidence::default),
            popup_refusal: popup_refusal.then(PopupRefusalEvidence::default),
            download_refusal: download_refusal.then(DownloadRefusalEvidence::default),
            chooser: chooser_selected.then(ChooserEvidence::default),
            capability: capability_selector.map(CapabilityEvidence::new),
            ..PolicyEvidence::default()
        }));
        write_policy_summary(
            &mut file,
            &PolicySummary::snapshot(&summary.lock().unwrap(), &install.lock().unwrap()),
        )?;
        let evidence = Arc::new(Mutex::new(file));
        let diagnostics = Arc::new(Diagnostics::from_env());
        if popup_refusal {
            diagnostics.arm_popup_initial_blank();
        }
        let (watchdog_done, watchdog_rx) = mpsc::channel();
        let hard_run_id = summary.lock().unwrap().run_id.clone();
        let hard_evidence = Arc::clone(&evidence);
        let watchdog_diagnostics = Arc::clone(&diagnostics);
        let watchdog = thread::spawn(move || {
            if watchdog_rx.recv_timeout(Duration::from_secs(75)).is_err() {
                watchdog_diagnostics.emit_once(true);
                let failed = Summary {
                    run_id: hard_run_id,
                    failure: "watchdogTimeout",
                    ..Summary::default()
                };
                if let Ok(mut file) = hard_evidence.try_lock() {
                    let _ = write_policy_summary(
                        &mut file,
                        &PolicySummary::snapshot(
                            &failed,
                            &PolicyEvidence {
                                native_refusal: native_refusal.then(NativeRefusalEvidence::default),
                                top_refusal: top_refusal.then(TopRefusalEvidence::default),
                                child_refusal: child_refusal.then(ChildRefusalEvidence::default),
                                dialog_refusal: dialog_refusal.then(DialogRefusalEvidence::default),
                                popup_refusal: popup_refusal.then(PopupRefusalEvidence::default),
                                download_refusal: download_refusal
                                    .then(DownloadRefusalEvidence::default),
                                chooser: chooser_selected.then(ChooserEvidence::default),
                                capability: capability_selector.map(CapabilityEvidence::new),
                                ..PolicyEvidence::default()
                            },
                        ),
                    );
                }
                // Only this fixture process; never native cleanup evidence.
                std::process::exit(124);
            }
        });
        let outcome = run_case(
            Arc::clone(&summary),
            Arc::clone(&diagnostics),
            Some(Arc::clone(&install)),
            None,
            None,
        );
        if outcome.is_err() {
            summary.lock().unwrap().failure = "fixtureSetupFailed";
        }
        let passed = summary.lock().unwrap().pass;
        let write_result = write_policy_summary(
            &mut evidence.lock().unwrap(),
            &PolicySummary::snapshot(&summary.lock().unwrap(), &install.lock().unwrap()),
        );
        let _ = watchdog_done.send(());
        let _ = watchdog.join();
        diagnostics.emit_once(false);
        write_result?;
        outcome?;
        Ok(passed)
    }

    pub fn run_fault(case: PolicyFixtureCase) -> Result<bool, Box<dyn std::error::Error>> {
        let run_id = std::env::var("AI_METER_CODE0_POLICY_RUN_ID")?;
        if run_id.is_empty()
            || run_id.len() > 64
            || !run_id
                .bytes()
                .all(|v| v.is_ascii_alphanumeric() || v == b'-' || v == b'_')
        {
            return Err("invalid policy fixture run ID".into());
        }
        let output = PathBuf::from(
            std::env::var_os("AI_METER_CODE0_POLICY_SUMMARY")
                .ok_or("missing fresh policy summary path")?,
        );
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)?;
        let summary = Arc::new(Mutex::new(Summary {
            run_id,
            failure: "notCompleted",
            ..Summary::default()
        }));
        let install = Arc::new(Mutex::new(PolicyEvidence::default()));
        let fault = Arc::new(Mutex::new(FaultEvidence::new(case)));
        write_summary(
            &mut file,
            &FaultSummary::snapshot(
                &summary.lock().unwrap(),
                &install.lock().unwrap(),
                &fault.lock().unwrap(),
            ),
        )?;
        let evidence = Arc::new(Mutex::new(file));
        let diagnostics = Arc::new(Diagnostics::from_env());
        let (watchdog_done, watchdog_rx) = mpsc::channel();
        let hard_run_id = summary.lock().unwrap().run_id.clone();
        let hard_evidence = Arc::clone(&evidence);
        let watchdog_diagnostics = Arc::clone(&diagnostics);
        let watchdog = thread::spawn(move || {
            if watchdog_rx.recv_timeout(Duration::from_secs(75)).is_err() {
                watchdog_diagnostics.emit_once(true);
                let failed = Summary {
                    run_id: hard_run_id,
                    failure: "watchdogTimeout",
                    ..Summary::default()
                };
                if let Ok(mut file) = hard_evidence.try_lock() {
                    let _ = write_summary(
                        &mut file,
                        &FaultSummary::snapshot(
                            &failed,
                            &PolicyEvidence::default(),
                            &FaultEvidence::new(case),
                        ),
                    );
                }
                std::process::exit(124);
            }
        });
        let outcome = run_case(
            Arc::clone(&summary),
            Arc::clone(&diagnostics),
            Some(Arc::clone(&install)),
            Some(Arc::clone(&fault)),
            None,
        );
        if outcome.is_err() {
            summary.lock().unwrap().failure = "fixtureSetupFailed";
        }
        let passed = summary.lock().unwrap().pass;
        let write_result = write_summary(
            &mut evidence.lock().unwrap(),
            &FaultSummary::snapshot(
                &summary.lock().unwrap(),
                &install.lock().unwrap(),
                &fault.lock().unwrap(),
            ),
        );
        let _ = watchdog_done.send(());
        let _ = watchdog.join();
        diagnostics.emit_once(false);
        write_result?;
        outcome?;
        Ok(passed)
    }

    pub fn run_coverage(case: PolicyFixtureCase) -> Result<bool, Box<dyn std::error::Error>> {
        let run_id = std::env::var("AI_METER_CODE0_POLICY_RUN_ID")?;
        if run_id.is_empty()
            || run_id.len() > 64
            || !run_id
                .bytes()
                .all(|v| v.is_ascii_alphanumeric() || v == b'-' || v == b'_')
        {
            return Err("invalid policy fixture run ID".into());
        }
        let output = PathBuf::from(
            std::env::var_os("AI_METER_CODE0_POLICY_SUMMARY")
                .ok_or("missing fresh policy summary path")?,
        );
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)?;
        let summary = Arc::new(Mutex::new(Summary {
            run_id,
            failure: "notCompleted",
            ..Summary::default()
        }));
        let install = Arc::new(Mutex::new(PolicyEvidence::default()));
        let coverage = Arc::new(Mutex::new(CoverageEvidence::new(case)));
        write_summary(
            &mut file,
            &CoverageSummary::snapshot(
                &summary.lock().unwrap(),
                &install.lock().unwrap(),
                &coverage.lock().unwrap(),
            ),
        )?;
        let evidence = Arc::new(Mutex::new(file));
        let diagnostics = Arc::new(Diagnostics::from_env());
        let (watchdog_done, watchdog_rx) = mpsc::channel();
        let hard_run_id = summary.lock().unwrap().run_id.clone();
        let hard_evidence = Arc::clone(&evidence);
        let watchdog_diagnostics = Arc::clone(&diagnostics);
        let watchdog = thread::spawn(move || {
            if watchdog_rx.recv_timeout(Duration::from_secs(75)).is_err() {
                watchdog_diagnostics.emit_once(true);
                let failed = Summary {
                    run_id: hard_run_id,
                    failure: "watchdogTimeout",
                    ..Summary::default()
                };
                if let Ok(mut file) = hard_evidence.try_lock() {
                    let _ = write_summary(
                        &mut file,
                        &CoverageSummary::snapshot(
                            &failed,
                            &PolicyEvidence::default(),
                            &CoverageEvidence::new(case),
                        ),
                    );
                }
                std::process::exit(124);
            }
        });
        let outcome = run_case(
            Arc::clone(&summary),
            Arc::clone(&diagnostics),
            Some(Arc::clone(&install)),
            None,
            Some(Arc::clone(&coverage)),
        );
        if outcome.is_err() {
            summary.lock().unwrap().failure = "fixtureSetupFailed";
        }
        let passed = summary.lock().unwrap().pass;
        let write_result = write_summary(
            &mut evidence.lock().unwrap(),
            &CoverageSummary::snapshot(
                &summary.lock().unwrap(),
                &install.lock().unwrap(),
                &coverage.lock().unwrap(),
            ),
        );
        let _ = watchdog_done.send(());
        let _ = watchdog.join();
        diagnostics.emit_once(false);
        write_result?;
        outcome?;
        Ok(passed)
    }

    // Native refusal can synchronously enter ordinary cleanup before Failed is
    // emitted at the bottom of poll. Preserve the first actual callback time;
    // adopting it never waits for an observation ACK or resets the 10s budget.
    fn adopt_top_refusal_close(
        driver: &mut Code0NativeDriver,
        instance: &str,
        closing_since: &mut Option<Instant>,
        at: Option<Instant>,
        diagnostics: &Diagnostics,
        legacy_selected: bool,
    ) {
        if legacy_selected {
            if let Some(closing) = adopt_legacy_native_close(closing_since, at) {
                diagnostics.close_budget_started(closing);
                diagnostics.phase(DP::CloseRequest, false);
                driver.close(instance);
                diagnostics.phase(DP::CloseRequest, true);
                diagnostics.observed(DM::CloseRequested);
            }
            return;
        }
        if closing_since.is_none()
            && let Some(closing) = at
        {
            *closing_since = Some(closing);
            diagnostics.close_budget_started(closing);
            diagnostics.phase(DP::CloseRequest, false);
            driver.close(instance);
            diagnostics.phase(DP::CloseRequest, true);
            diagnostics.observed(DM::CloseRequested);
        }
    }

    fn run_case(
        summary: Arc<Mutex<Summary>>,
        diagnostics: Arc<Diagnostics>,
        policy: Option<Arc<Mutex<PolicyEvidence>>>,
        fault: Option<Arc<Mutex<FaultEvidence>>>,
        coverage: Option<Arc<Mutex<CoverageEvidence>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fault_case = fault.as_ref().map(|fault| fault.lock().unwrap().case);
        let coverage_case = coverage
            .as_ref()
            .map(|coverage| coverage.lock().unwrap().case);
        let selected_case = fault_case.or(coverage_case);
        let child_selected = policy
            .as_ref()
            .is_some_and(|p| p.lock().unwrap().child_refusal.is_some());
        let dialog_selected = policy
            .as_ref()
            .is_some_and(|p| p.lock().unwrap().dialog_refusal.is_some());
        let download_selected = policy
            .as_ref()
            .is_some_and(|p| p.lock().unwrap().download_refusal.is_some());
        let popup_selected = policy
            .as_ref()
            .is_some_and(|p| p.lock().unwrap().popup_refusal.is_some());
        let chooser_selected = policy
            .as_ref()
            .is_some_and(|p| p.lock().unwrap().chooser.is_some());
        let capability_selector = policy.as_ref().and_then(|p| {
            p.lock()
                .unwrap()
                .capability
                .as_ref()
                .and_then(|c| NativeCapabilitySelector::parse(c.facts.selector))
        });
        let capability_selected = capability_selector.is_some();
        let top_selected = policy
            .as_ref()
            .is_some_and(|policy| policy.lock().unwrap().top_refusal.is_some());
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
        if top_selected {
            // Exclusive creation in this very TempDir; never reuse or read a body.
            let asset = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(temp_root.join(TOP_REFUSAL_LEAF))?;
            let metadata = asset.metadata()?;
            let empty = metadata.is_file() && metadata.len() == 0;
            drop(asset);
            if !empty {
                return Err("owned inert asset setup failed".into());
            }
            policy
                .as_ref()
                .unwrap()
                .lock()
                .unwrap()
                .top_refusal
                .as_mut()
                .unwrap()
                .owned_file_prepared = true;
        }
        if dialog_selected {
            // Controlled CI has one fixture writer. This is not atomic binding
            // against hostile same-privilege replacement after the handle closes.
            let asset = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(temp_root.join(DIALOG_REFUSAL_LEAF))?;
            let metadata = asset.metadata()?;
            let empty = metadata.is_file() && metadata.len() == 0;
            drop(asset);
            if !empty {
                return Err("owned empty HTML asset setup failed".into());
            }
            policy
                .as_ref()
                .unwrap()
                .lock()
                .unwrap()
                .dialog_refusal
                .as_mut()
                .unwrap()
                .owned_file_prepared = true;
        }
        if chooser_selected {
            let asset = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(temp_root.join(CHOOSER_LEAF))?;
            let metadata = asset.metadata()?;
            let empty = metadata.is_file() && metadata.len() == 0;
            drop(asset);
            if !empty {
                return Err("owned chooser empty HTML setup failed".into());
            }
            policy
                .as_ref()
                .unwrap()
                .lock()
                .unwrap()
                .chooser
                .as_mut()
                .unwrap()
                .owned_file_prepared = true;
        }
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
        let native_top_close = Rc::new(Cell::new(None::<Instant>));
        let observer_top_close = Rc::clone(&native_top_close);
        let native_dialog_close = Rc::new(Cell::new(None::<Instant>));
        let observer_dialog_close = Rc::clone(&native_dialog_close);
        let native_download_close = Rc::new(Cell::new(None::<Instant>));
        let observer_download_close = Rc::clone(&native_download_close);
        let native_popup_close = Rc::new(Cell::new(None::<Instant>));
        let observer_popup_close = Rc::clone(&native_popup_close);
        let native_capability_close = Rc::new(Cell::new(None::<Instant>));
        let observer_capability_close = Rc::clone(&native_capability_close);
        let native_chooser_close = Rc::new(Cell::new(None::<Instant>));
        let observer_chooser_close = Rc::clone(&native_chooser_close);
        let native_child_close = Rc::new(Cell::new(None::<Instant>));
        let observer_child_close = Rc::clone(&native_child_close);
        driver.observe_fixture(Rc::new(move |event| {
            if capability_selected
                && let NativeSmokeObservation::CapabilityClosing { at } = &event
                && observer_capability_close.get().is_none()
            {
                observer_capability_close.set(Some(*at));
            }
            if chooser_selected
                && let NativeSmokeObservation::ChooserClosing { at } = &event
                && observer_chooser_close.get().is_none()
            {
                observer_chooser_close.set(Some(*at));
            }
            if download_selected
                && let NativeSmokeObservation::DownloadClosing { at } = &event
                && observer_download_close.get().is_none()
            {
                observer_download_close.set(Some(*at));
            }
            if popup_selected
                && let NativeSmokeObservation::PopupClosing { at } = &event
                && observer_popup_close.get().is_none()
            {
                observer_popup_close.set(Some(*at));
            }
            if dialog_selected
                && let NativeSmokeObservation::DialogClosing { at } = &event
                && observer_dialog_close.get().is_none()
            {
                observer_dialog_close.set(Some(*at));
            }
            if child_selected
                && let NativeSmokeObservation::ChildClosing { at } = &event
                && observer_child_close.get().is_none()
            {
                observer_child_close.set(Some(*at));
            }
            if top_selected
                && let NativeSmokeObservation::TopClosing { at } = &event
                && observer_top_close.get().is_none()
            {
                // The real Context has already revoked this native instance.
                // UI-local timestamp only, outside Owner/summary/queue borrows.
                observer_top_close.set(Some(*at));
            }
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
        let instance = if policy.is_some() {
            POLICY_INSTANCE
        } else {
            INSTANCE
        };
        let request = Code0WindowRequest {
            instance_id: instance.into(),
            connection_epoch: if policy.is_some() {
                "11111111-1111-4111-8111-111111111111"
            } else {
                "fixture-epoch"
            }
            .into(),
            nonce: if policy.is_some() {
                "33333333-4444-4555-8666-777777777777"
            } else {
                "fixture-nonce"
            }
            .into(),
            document_id: if policy.is_some() {
                "88888888-9999-4000-8111-222222222222"
            } else {
                "fixture-document"
            }
            .into(),
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
        // N3 alone carries the original absolute end; late EngineReady cannot
        // construct a newer 45-second authorization window.
        let mut dialog_deadline = if dialog_selected {
            NativeDialogDeadline::from_attempt_started(started)
        } else {
            None
        };
        // N1/N2 move exactly this original absolute clock once at submission.
        let mut legacy_deadline = if top_selected || child_selected {
            NativeRefusalDeadline::from_attempt_started(started)
        } else {
            None
        };
        let mut download_deadline = if download_selected {
            NativeDialogDeadline::from_attempt_started(started)
        } else {
            None
        };
        let mut popup_deadline = if popup_selected {
            NativeDialogDeadline::from_attempt_started(started)
        } else {
            None
        };
        let mut capability_deadline = if capability_selected {
            NativeDialogDeadline::from_attempt_started(started)
        } else {
            None
        };
        let mut chooser_deadline = if chooser_selected {
            NativeDialogDeadline::from_attempt_started(started)
        } else {
            None
        };
        let mut closing_since = None;
        let mut started_create = false;
        let mut object_free_failure = false;
        let mut terminal_exit_requested = false;
        let mut ready_count = 0;
        let mut message_count = 0;
        let mut browser_exit_count = 0;
        let final_policy = policy.clone();
        let final_fault = fault.clone();
        let final_coverage = coverage.clone();
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
                        let closing = Instant::now();
                        closing_since = Some(closing);
                        ui_diagnostics.close_budget_started(closing);
                        ui_diagnostics.phase(DP::CloseRequest, false);
                        driver.close(instance);
                        ui_diagnostics.phase(DP::CloseRequest, true);
                        ui_diagnostics.observed(DM::CloseRequested);
                        object_free_failure = driver.fixture_creation_is_object_free(instance);
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
            adopt_top_refusal_close(
                &mut driver,
                instance,
                &mut closing_since,
                if capability_selected {
                    native_capability_close.get()
                } else if chooser_selected {
                    native_chooser_close.get()
                } else if download_selected {
                    native_download_close.get()
                } else if popup_selected {
                    native_popup_close.get()
                } else if dialog_selected {
                    native_dialog_close.get()
                } else if child_selected {
                    native_child_close.get()
                } else {
                    native_top_close.get()
                },
                &ui_diagnostics,
                top_selected || child_selected,
            );
            ui_diagnostics.phase(DP::Poll, false);
            driver.poll_cleanup();
            ui_diagnostics.phase(DP::Poll, true);
            // A target callback can also reenter a native query during poll.
            adopt_top_refusal_close(
                &mut driver,
                instance,
                &mut closing_since,
                if capability_selected {
                    native_capability_close.get()
                } else if chooser_selected {
                    native_chooser_close.get()
                } else if download_selected {
                    native_download_close.get()
                } else if popup_selected {
                    native_popup_close.get()
                } else if dialog_selected {
                    native_dialog_close.get()
                } else if child_selected {
                    native_child_close.get()
                } else {
                    native_top_close.get()
                },
                &ui_diagnostics,
                top_selected || child_selected,
            );
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
                        if let Some(policy) = &policy {
                            if ready_count != 1 || !state.engine_ready || !state.profile_isolated {
                                state.failure = "policyEnginePreconditionFailed";
                            } else {
                                // No summary/install lock crosses the actual driver
                                // entry. A submission result never supplies an ACK.
                                drop(state);
                                if fault_case == Some(PolicyFixtureCase::InstallPrematureLoadDenied)
                                {
                                    let denied =
                                        driver.probe_fixture_premature_policy_load(&request);
                                    state = observed.lock().unwrap();
                                    if denied != Err(Code0SessionFailure::PolicyRejected) {
                                        state.failure = "prematureLoadNotDenied";
                                    }
                                    continue;
                                }
                                let native_selected =
                                    { policy.lock().unwrap().native_refusal.is_some() };
                                if native_selected {
                                    // All summary/install guards ended before the
                                    // explicit native installer or readonly getter.
                                    let installed =
                                        driver.install_fixture_native_refusal_handlers(&request);
                                    let receipt =
                                        driver.require_fixture_native_refusal_install(&request);
                                    let mut install = policy.lock().unwrap();
                                    let native = install.native_refusal.as_mut().unwrap();
                                    let mut saved_receipt = None;
                                    let valid_inventory = receipt.is_ok_and(|receipt| {
                                        let valid = native.observe(&receipt, &request);
                                        if top_selected
                                            || child_selected
                                            || dialog_selected
                                            || popup_selected
                                            || download_selected
                                            || chooser_selected
                                            || capability_selected
                                        {
                                            saved_receipt = Some(receipt);
                                        }
                                        valid
                                    });
                                    let rejected = native.load_rejected;
                                    if capability_selected {
                                        install.capability.as_mut().unwrap().installation =
                                            saved_receipt;
                                    } else if chooser_selected {
                                        install.chooser.as_mut().unwrap().installation =
                                            saved_receipt;
                                    } else if download_selected {
                                        install.download_refusal.as_mut().unwrap().installation =
                                            saved_receipt;
                                    } else if popup_selected {
                                        install.popup_refusal.as_mut().unwrap().installation =
                                            saved_receipt;
                                    } else if dialog_selected {
                                        install.dialog_refusal.as_mut().unwrap().installation =
                                            saved_receipt;
                                    } else if child_selected {
                                        install.child_refusal.as_mut().unwrap().installation =
                                            saved_receipt;
                                    } else if top_selected {
                                        install.top_refusal.as_mut().unwrap().installation =
                                            saved_receipt;
                                    }
                                    drop(install);
                                    let capability_submission_failure = if capability_selected
                                        && installed.is_ok()
                                        && valid_inventory
                                        && !rejected
                                    {
                                        let result = match capability_deadline
                                            .take()
                                            .zip(capability_selector)
                                        {
                                            Some((deadline, selector)) => driver
                                                .begin_fixture_capability(
                                                    &request, selector, deadline,
                                                ),
                                            None => Err(Code0SessionFailure::PolicyRejected),
                                        };
                                        result.err().map(|_| "capabilityNativeSubmissionFailed")
                                    } else {
                                        None
                                    };
                                    let chooser_submission_failure = if chooser_selected
                                        && installed.is_ok()
                                        && valid_inventory
                                        && !rejected
                                    {
                                        let result = match chooser_deadline.take() {
                                            Some(deadline) => driver
                                                .begin_fixture_prebound_chooser(&request, deadline),
                                            None => Err(Code0SessionFailure::PolicyRejected),
                                        };
                                        result.err().map(|_| "chooserNativeSubmissionFailed")
                                    } else {
                                        None
                                    };
                                    let download_submission_failure = if download_selected
                                        && installed.is_ok()
                                        && valid_inventory
                                        && !rejected
                                    {
                                        let result = match download_deadline.take() {
                                            Some(deadline) => driver
                                                .submit_fixture_download_initial_data_empty(
                                                    &request, deadline,
                                                ),
                                            None => Err(Code0SessionFailure::PolicyRejected),
                                        };
                                        if result.is_err() {
                                            Some("downloadNativeSubmissionFailed")
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    };
                                    let popup_submission_failure = if popup_selected
                                        && installed.is_ok()
                                        && valid_inventory
                                        && !rejected
                                    {
                                        let result = match popup_deadline.take() {
                                            Some(deadline) => driver
                                                .submit_fixture_popup_initial_blank(
                                                    &request, deadline,
                                                ),
                                            None => Err(Code0SessionFailure::PolicyRejected),
                                        };
                                        if result.is_err() {
                                            Some("popupNativeSubmissionFailed")
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    };
                                    // No summary/install/queue guard crosses native Navigate.
                                    let dialog_submission = if dialog_selected
                                        && installed.is_ok()
                                        && valid_inventory
                                        && !rejected
                                    {
                                        Some(match dialog_deadline.take() {
                                            Some(deadline) => driver
                                                .navigate_fixture_dialog_owned_empty_html(
                                                    &request, deadline,
                                                ),
                                            None => Err(Code0SessionFailure::PolicyRejected),
                                        })
                                    } else {
                                        None
                                    };
                                    let dialog_submission_failure =
                                        if let Some(result) = dialog_submission {
                                            if result.is_ok() {
                                                policy
                                                    .lock()
                                                    .unwrap()
                                                    .native_refusal
                                                    .as_mut()
                                                    .unwrap()
                                                    .navigation_not_issued = false;
                                                None
                                            } else {
                                                Some("dialogOwnedDocumentSetupFailed")
                                            }
                                        } else {
                                            None
                                        };
                                    let top_submission = if top_selected
                                        && installed.is_ok()
                                        && valid_inventory
                                        && !rejected
                                    {
                                        Some(match legacy_deadline.take() {
                                            Some(deadline) => driver
                                                .navigate_fixture_top_owned_file(
                                                    &request, deadline,
                                                ),
                                            None => Err(Code0SessionFailure::PolicyRejected),
                                        })
                                    } else {
                                        None
                                    };
                                    let child_submission = if child_selected
                                        && installed.is_ok()
                                        && valid_inventory
                                        && !rejected
                                    {
                                        Some(match legacy_deadline.take() {
                                            Some(deadline) => driver
                                                .submit_fixture_child_initial_data(
                                                    &request, deadline,
                                                ),
                                            None => Err(Code0SessionFailure::PolicyRejected),
                                        })
                                    } else {
                                        None
                                    };
                                    let mut child_submission_failure = None;
                                    if let Some(result) = child_submission {
                                        let mut install = policy.lock().unwrap();
                                        match result {
                                            Ok(receipt) => {
                                                let original_timely =
                                                    receipt.original_attempt_timely();
                                                if !install
                                                    .child_refusal
                                                    .as_mut()
                                                    .unwrap()
                                                    .submission(receipt, &request)
                                                {
                                                    child_submission_failure =
                                                        Some(if !original_timely {
                                                            "childOriginalAttemptExpired"
                                                        } else {
                                                            "childNativeSubmissionFailed"
                                                        });
                                                }
                                                install
                                                    .native_refusal
                                                    .as_mut()
                                                    .unwrap()
                                                    .navigation_not_issued = false;
                                            }
                                            Err(_) => {
                                                child_submission_failure =
                                                    Some("childInitialBlankPreconditionFailed")
                                            }
                                        }
                                    }
                                    let mut top_submission_failure = None;
                                    if let Some(result) = top_submission {
                                        let mut install = policy.lock().unwrap();
                                        match result {
                                            Ok(receipt) => {
                                                let original_timely =
                                                    receipt.original_attempt_timely();
                                                let top = install.top_refusal.as_mut().unwrap();
                                                if !top.submission(receipt, &request) {
                                                    top_submission_failure =
                                                        Some(if !original_timely {
                                                            "topOriginalAttemptExpired"
                                                        } else if !top.asset_binding_validated {
                                                            "topSubmissionReceiptInvalid"
                                                        } else {
                                                            "topNativeSubmissionFailed"
                                                        });
                                                }
                                                install
                                                    .native_refusal
                                                    .as_mut()
                                                    .unwrap()
                                                    .navigation_not_issued = false;
                                            }
                                            Err(_) => {
                                                top_submission_failure =
                                                    Some("topAssetBindingPreconditionFailed")
                                            }
                                        }
                                    }
                                    state = observed.lock().unwrap();
                                    state.failure = if installed.is_err() {
                                        "nativeHookInstallFailed"
                                    } else if !valid_inventory {
                                        "nativeHookInventoryPreconditionFailed"
                                    } else if rejected {
                                        "nativeHooksMissing"
                                    } else if let Some(failure) = capability_submission_failure {
                                        failure
                                    } else if let Some(failure) = chooser_submission_failure {
                                        failure
                                    } else if let Some(failure) = download_submission_failure {
                                        failure
                                    } else if let Some(failure) = popup_submission_failure {
                                        failure
                                    } else if let Some(failure) = dialog_submission_failure {
                                        failure
                                    } else if let Some(failure) = child_submission_failure {
                                        failure
                                    } else if let Some(failure) = top_submission_failure {
                                        failure
                                    } else {
                                        "notCompleted"
                                    };
                                    continue;
                                }
                                policy.lock().unwrap().submissions += 1;
                                let installed = driver.begin_fixture_policy_install(
                                    &request,
                                    selected_case
                                        .unwrap_or(PolicyFixtureCase::InstallAckBeforeFirstLoad),
                                );
                                let mut install = policy.lock().unwrap();
                                install.submission_accepted = installed.is_ok();
                                install.install_rejected =
                                    installed == Err(Code0SessionFailure::PolicyRejected);
                                drop(install);
                                state = observed.lock().unwrap();
                                if let Err(error) = installed {
                                    state.failure = if fault.is_some()
                                        && error == Code0SessionFailure::PolicyRejected
                                    {
                                        "notCompleted"
                                    } else if fault.is_some() {
                                        "faultInstallPreconditionFailed"
                                    } else if error == Code0SessionFailure::PolicyRejected {
                                        "policyInstallRejected"
                                    } else {
                                        "policyInstallFailed"
                                    };
                                }
                            }
                        } else {
                            // No fixture state lock crosses a COM call, which may
                            // deliver native callbacks before returning.
                            drop(state);
                            ui_diagnostics.phase(DP::Inline, false);
                            let loaded = driver.load_fixture_inline_document(instance);
                            ui_diagnostics.phase(DP::Inline, true);
                            ui_diagnostics.phase(DP::SummaryLock, false);
                            state = observed.lock().unwrap();
                            ui_diagnostics.phase(DP::SummaryLock, true);
                            if loaded.is_err() {
                                state.failure = "inlineLoadDenied";
                            }
                        }
                    }
                    Observation::Native(
                        NativeSmokeObservation::CapabilityClosing { .. },
                        callback_thread,
                        callback_sta,
                    ) => {
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        if !capability_selected {
                            state.failure = "unexpectedCapabilityClosing";
                        }
                    }
                    Observation::Native(
                        NativeSmokeObservation::Capability(receipt),
                        callback_thread,
                        callback_sta,
                    ) => {
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        let valid = capability_selected
                            && policy.as_ref().is_some_and(|p| {
                                let mut install = p.lock().unwrap();
                                if receipt.facts().initial_native_join {
                                    install
                                        .native_refusal
                                        .as_mut()
                                        .unwrap()
                                        .navigation_not_issued = false;
                                }
                                install.capability.as_mut().is_some_and(|c| {
                                    c.observe(
                                        receipt,
                                        &request,
                                        started,
                                        native_capability_close.get(),
                                        &state.runtime_version,
                                    )
                                })
                            });
                        if !valid {
                            state.failure = "capabilityNativeReceiptInvalid";
                        }
                    }
                    Observation::Native(
                        NativeSmokeObservation::ChooserClosing { .. },
                        callback_thread,
                        callback_sta,
                    ) => {
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        if !chooser_selected {
                            state.failure = "unexpectedChooserClosing";
                        }
                    }
                    Observation::Native(
                        NativeSmokeObservation::Chooser(receipt),
                        callback_thread,
                        callback_sta,
                    ) => {
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        let valid = chooser_selected
                            && policy.as_ref().is_some_and(|p| {
                                let mut install = p.lock().unwrap();
                                if receipt.facts().navigation_issued {
                                    install
                                        .native_refusal
                                        .as_mut()
                                        .unwrap()
                                        .navigation_not_issued = false;
                                }
                                install
                                    .chooser
                                    .as_mut()
                                    .is_some_and(|c| c.observe(receipt, &request, started))
                            });
                        if !valid {
                            state.failure = "chooserNativeReceiptInvalid";
                        }
                    }
                    Observation::Native(
                        NativeSmokeObservation::DownloadClosing { .. },
                        callback_thread,
                        callback_sta,
                    ) => {
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        if !download_selected {
                            state.failure = "unexpectedDownloadClosing";
                        }
                    }
                    Observation::Native(
                        NativeSmokeObservation::Download(receipt),
                        callback_thread,
                        callback_sta,
                    ) => {
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        let valid = download_selected
                            && policy.as_ref().is_some_and(|p| {
                                p.lock().unwrap().download_refusal.as_mut().is_some_and(
                                    |download| download.observe(receipt, &request, started),
                                )
                            });
                        if !valid {
                            state.failure = "downloadNativeReceiptInvalid";
                        }
                    }
                    Observation::Native(
                        NativeSmokeObservation::PopupClosing { .. },
                        callback_thread,
                        callback_sta,
                    ) => {
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        if !popup_selected {
                            state.failure = "unexpectedPopupClosing";
                        }
                    }
                    Observation::Native(
                        NativeSmokeObservation::Popup(receipt),
                        callback_thread,
                        callback_sta,
                    ) => {
                        ui_diagnostics.n4_popup_stage(receipt.stage(), true);
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        let valid = popup_selected
                            && policy.as_ref().is_some_and(|p| {
                                p.lock()
                                    .unwrap()
                                    .popup_refusal
                                    .as_mut()
                                    .is_some_and(|popup| popup.observe(receipt, &request, started))
                            });
                        if !valid {
                            state.failure = "popupNativeReceiptInvalid";
                        }
                    }
                    Observation::Native(
                        NativeSmokeObservation::DialogClosing { .. },
                        callback_thread,
                        callback_sta,
                    ) => {
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        if !dialog_selected {
                            state.failure = "unexpectedDialogClosing";
                        }
                    }
                    Observation::Native(
                        NativeSmokeObservation::Dialog(receipt),
                        callback_thread,
                        callback_sta,
                    ) => {
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        let valid = dialog_selected
                            && policy.as_ref().is_some_and(|p| {
                                let issued =
                                    matches!(receipt.stage(), NativeDialogStage::Submitted)
                                        && receipt.facts().navigation_submissions == 1;
                                let mut install = p.lock().unwrap();
                                if issued {
                                    install
                                        .native_refusal
                                        .as_mut()
                                        .unwrap()
                                        .navigation_not_issued = false;
                                }
                                install
                                    .dialog_refusal
                                    .as_mut()
                                    .is_some_and(|d| d.observe(receipt, &request))
                            });
                        if !valid {
                            state.failure = "dialogNativeReceiptInvalid";
                        }
                    }
                    Observation::Native(
                        NativeSmokeObservation::TopClosing { .. },
                        callback_thread,
                        callback_sta,
                    ) => {
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        if !top_selected {
                            state.failure = "unexpectedTopClosing";
                        }
                    }
                    Observation::Native(
                        NativeSmokeObservation::ChildClosing { .. },
                        callback_thread,
                        callback_sta,
                    ) => {
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        if !child_selected {
                            state.failure = "unexpectedChildClosing";
                        }
                    }
                    Observation::Native(
                        NativeSmokeObservation::ChildRefusal(receipt),
                        callback_thread,
                        callback_sta,
                    ) => {
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        let valid = child_selected
                            && policy.as_ref().is_some_and(|p| {
                                p.lock()
                                    .unwrap()
                                    .child_refusal
                                    .as_mut()
                                    .is_some_and(|c| c.observe(receipt, &request))
                            });
                        if !valid {
                            state.failure = "childNativeReceiptInvalid";
                        }
                    }
                    Observation::Native(
                        NativeSmokeObservation::ChildCompletion(receipt),
                        callback_thread,
                        callback_sta,
                    ) => {
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        let valid = child_selected
                            && policy.as_ref().is_some_and(|p| {
                                p.lock()
                                    .unwrap()
                                    .child_refusal
                                    .as_mut()
                                    .is_some_and(|c| c.completion(receipt, &request))
                            });
                        if !valid {
                            state.failure = "childCompletionInvalid";
                        }
                    }
                    Observation::Native(
                        NativeSmokeObservation::ChildFinal(receipt),
                        callback_thread,
                        callback_sta,
                    ) => {
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        let valid = child_selected
                            && policy.as_ref().is_some_and(|p| {
                                p.lock()
                                    .unwrap()
                                    .child_refusal
                                    .as_mut()
                                    .is_some_and(|c| c.final_receipt(receipt, &request))
                            });
                        if !valid {
                            state.failure = "childFinalInventoryInvalid";
                        }
                    }
                    Observation::Native(
                        NativeSmokeObservation::TopRefusal(receipt),
                        callback_thread,
                        callback_sta,
                    ) => {
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        let valid = top_selected
                            && policy.as_ref().is_some_and(|policy| {
                                policy
                                    .lock()
                                    .unwrap()
                                    .top_refusal
                                    .as_mut()
                                    .is_some_and(|top| top.observe(receipt, &request))
                            });
                        if !valid {
                            state.failure = "topNativeReceiptInvalid";
                        }
                    }
                    Observation::Native(
                        NativeSmokeObservation::Policy(event),
                        callback_thread,
                        callback_sta,
                    ) => {
                        state.callback_affinity &= callback_thread == main_thread && callback_sta;
                        if let Some(coverage) = &coverage {
                            match event {
                                PolicyObservation::StaleDisposition(receipt) => {
                                    let mut install = policy.as_ref().unwrap().lock().unwrap();
                                    if !coverage.lock().unwrap().observe_stale(
                                        receipt,
                                        &request,
                                        &mut install,
                                        closing_since.is_some(),
                                    ) {
                                        state.failure = "staleReceiptInvalid";
                                    }
                                }
                                PolicyObservation::Containment(receipt) => {
                                    let mut install = policy.as_ref().unwrap().lock().unwrap();
                                    if !coverage.lock().unwrap().observe_containment(
                                        receipt,
                                        &request,
                                        &mut install,
                                        closing_since.is_some(),
                                    ) {
                                        state.failure = "containmentReceiptInvalid";
                                    }
                                }
                                PolicyObservation::BarrierReady(receipt) => {
                                    let valid = closing_since.is_none()
                                        && coverage_case
                                            == Some(PolicyFixtureCase::InstallRealmsNoPassthrough)
                                        && policy.as_ref().unwrap().lock().unwrap().observe(
                                            PolicyObservation::BarrierReady(receipt),
                                            &request,
                                        );
                                    if !valid {
                                        state.failure = "coverageUnexpectedBarrier";
                                    } else {
                                        drop(state);
                                        let loaded = driver.load_fixture_policy_document(&request);
                                        state = observed.lock().unwrap();
                                        if loaded.is_err() {
                                            state.failure = "containmentLoadDenied";
                                        }
                                    }
                                }
                                PolicyObservation::FirstControlledLoad(receipt) => {
                                    if closing_since.is_some()
                                        || coverage_case
                                            != Some(PolicyFixtureCase::InstallRealmsNoPassthrough)
                                        || !policy.as_ref().unwrap().lock().unwrap().observe(
                                            PolicyObservation::FirstControlledLoad(receipt),
                                            &request,
                                        )
                                    {
                                        state.failure = "coverageLoadReceiptInvalid";
                                    }
                                }
                                PolicyObservation::ControlledDocumentReady(_)
                                | PolicyObservation::FaultTarget(_)
                                | PolicyObservation::FaultApplied(_)
                                | PolicyObservation::FaultFinal(_) => {
                                    state.failure = "coverageUnexpectedObservation";
                                }
                            }
                        } else if let Some(fault) = &fault {
                            match event {
                                PolicyObservation::FaultTarget(receipt) => {
                                    let mut fault = fault.lock().unwrap();
                                    if closing_since.is_some()
                                        || !fault.observe(receipt, &request, false)
                                    {
                                        state.failure = "faultTargetReceiptInvalid";
                                    } else if fault.case.async_application() {
                                        let mut install = policy.as_ref().unwrap().lock().unwrap();
                                        install.script_acknowledged = fault.application[1];
                                        install.cdp_acknowledged = fault.application[2];
                                    }
                                }
                                PolicyObservation::FaultApplied(receipt) => {
                                    if closing_since.is_some()
                                        || !fault.lock().unwrap().observe_applied(receipt, &request)
                                    {
                                        state.failure = "faultAppliedReceiptInvalid";
                                    }
                                }
                                PolicyObservation::FaultFinal(receipt) => {
                                    let mut fault = fault.lock().unwrap();
                                    if closing_since.is_none()
                                        || !fault.observe(receipt, &request, true)
                                    {
                                        state.failure = "faultFinalReceiptInvalid";
                                    } else if fault.case.async_application() {
                                        let mut install = policy.as_ref().unwrap().lock().unwrap();
                                        install.script_acknowledged = fault.application[1];
                                        install.cdp_acknowledged = fault.application[2];
                                    }
                                }
                                PolicyObservation::BarrierReady(receipt) => {
                                    let valid = closing_since.is_none()
                                        && policy.as_ref().unwrap().lock().unwrap().observe(
                                            PolicyObservation::BarrierReady(receipt),
                                            &request,
                                        );
                                    let fault = fault.lock().unwrap();
                                    state.failure = if state.failure != "notCompleted" {
                                        state.failure
                                    } else if !valid {
                                        "faultBarrierReceiptInvalid"
                                    } else if fault.target_events == 1
                                        && fault.target_receipts == 1
                                        && fault.injection_receipts == 0
                                    {
                                        "faultNotApplied"
                                    } else {
                                        "faultTargetPreconditionFailed"
                                    };
                                    // An unexpected real barrier proves continuation. Close
                                    // through R4 below; never call the controlled loader.
                                }
                                PolicyObservation::FirstControlledLoad(_)
                                | PolicyObservation::ControlledDocumentReady(_)
                                | PolicyObservation::StaleDisposition(_)
                                | PolicyObservation::Containment(_) => {
                                    state.failure = "unexpectedFaultLoad";
                                }
                            }
                        } else if let Some(policy) = &policy {
                            let barrier = matches!(&event, PolicyObservation::BarrierReady(_));
                            let valid = closing_since.is_none()
                                && policy.lock().unwrap().observe(event, &request);
                            if !valid {
                                state.failure = "policyReceiptInvalid";
                            } else if barrier {
                                // Genuine native barrier was observed first. Neither
                                // summary nor policy locks cross the finite loader.
                                drop(state);
                                let loaded = driver.load_fixture_policy_document(&request);
                                state = observed.lock().unwrap();
                                if loaded.is_err() {
                                    state.failure = "policyDocumentLoadDenied";
                                }
                            }
                        } else {
                            state.failure = "unexpectedPolicyObservation";
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
                        if policy.is_some() {
                            state.failure = "unexpectedInlineMessage";
                        }
                    }
                    Observation::Native(
                        NativeSmokeObservation::BrowserProcessExited,
                        callback_thread,
                        callback_sta,
                    ) => {
                        browser_exit_count += 1;
                        if capability_selected {
                            policy
                                .as_ref()
                                .unwrap()
                                .lock()
                                .unwrap()
                                .capability
                                .as_mut()
                                .unwrap()
                                .browser_process_exited_once = browser_exit_count == 1;
                        }
                        if chooser_selected {
                            policy
                                .as_ref()
                                .unwrap()
                                .lock()
                                .unwrap()
                                .chooser
                                .as_mut()
                                .unwrap()
                                .browser_process_exited_once = browser_exit_count == 1;
                        }
                        if download_selected {
                            policy
                                .as_ref()
                                .unwrap()
                                .lock()
                                .unwrap()
                                .download_refusal
                                .as_mut()
                                .unwrap()
                                .browser_process_exited_once = browser_exit_count == 1;
                        }
                        if popup_selected {
                            policy
                                .as_ref()
                                .unwrap()
                                .lock()
                                .unwrap()
                                .popup_refusal
                                .as_mut()
                                .unwrap()
                                .browser_process_exited_once = browser_exit_count == 1;
                        }
                        if dialog_selected {
                            policy
                                .as_ref()
                                .unwrap()
                                .lock()
                                .unwrap()
                                .dialog_refusal
                                .as_mut()
                                .unwrap()
                                .browser_process_exited_once = browser_exit_count == 1;
                        }
                        if child_selected {
                            policy
                                .as_ref()
                                .unwrap()
                                .lock()
                                .unwrap()
                                .child_refusal
                                .as_mut()
                                .unwrap()
                                .browser_process_exited_once = browser_exit_count == 1;
                        }
                        if top_selected {
                            policy
                                .as_ref()
                                .unwrap()
                                .lock()
                                .unwrap()
                                .top_refusal
                                .as_mut()
                                .unwrap()
                                .browser_process_exited_once = browser_exit_count == 1;
                        }
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
                    Observation::Session(
                        owned,
                        Code0WindowEvent::Failed(reason),
                        callback_thread,
                        callback_sta,
                    ) => {
                        ui_diagnostics.n4_session_stage(&reason, true);
                        if capability_selected {
                            state.callback_affinity &=
                                callback_thread == main_thread && callback_sta;
                            let mut install = policy.as_ref().unwrap().lock().unwrap();
                            let capability = install.capability.as_mut().unwrap();
                            let prior =
                                capability.receipts[1] == 1 && capability.facts.failure.is_some();
                            if owned == request
                                && reason == Code0SessionFailure::PolicyRejected
                                && prior
                            {
                                capability.policy_failures =
                                    capability.policy_failures.saturating_add(1).min(2);
                                capability.failure_after_disposition = true;
                                if capability.policy_failures != 1 {
                                    state.failure = "capabilityRepeatedNativeFailure";
                                }
                            } else {
                                state.failure = "capabilityUnexpectedNativeFailure";
                            }
                        } else if chooser_selected {
                            state.callback_affinity &=
                                callback_thread == main_thread && callback_sta;
                            let mut install = policy.as_ref().unwrap().lock().unwrap();
                            let chooser = install.chooser.as_mut().unwrap();
                            if owned == request && reason == Code0SessionFailure::PolicyRejected {
                                chooser.policy_failures =
                                    chooser.policy_failures.saturating_add(1).min(2);
                                chooser.action_before_policy_failure = chooser.target_positive();
                                if chooser.policy_failures != 1
                                    || !chooser.action_before_policy_failure
                                {
                                    state.failure = "chooserFailureWithoutNativeTarget";
                                }
                            } else {
                                state.failure = "chooserUnexpectedNativeFailure";
                            }
                        } else if download_selected {
                            state.callback_affinity &=
                                callback_thread == main_thread && callback_sta;
                            let mut install = policy.as_ref().unwrap().lock().unwrap();
                            let download = install.download_refusal.as_mut().unwrap();
                            if owned == request && reason == Code0SessionFailure::PolicyRejected {
                                // Only this actual Session event joined to the prior
                                // genuine download action may increment N5's count.
                                download.action_before_policy_failure = download.target_positive();
                                if download.action_before_policy_failure {
                                    download.policy_failures =
                                        download.policy_failures.saturating_add(1).min(2);
                                }
                                if download.policy_failures != 1
                                    || !download.action_before_policy_failure
                                {
                                    state.failure = "downloadPreemptedOrTargetUnobserved";
                                }
                            } else {
                                state.failure = "downloadUnexpectedNativeFailure";
                            }
                        } else if popup_selected {
                            state.callback_affinity &=
                                callback_thread == main_thread && callback_sta;
                            let mut install = policy.as_ref().unwrap().lock().unwrap();
                            let popup = install.popup_refusal.as_mut().unwrap();
                            if owned == request && reason == Code0SessionFailure::PolicyRejected {
                                // The actual native events -> Observation::Session channel
                                // alone increments this count, joined to the native receipt.
                                popup.policy_failures =
                                    popup.policy_failures.saturating_add(1).min(2);
                                popup.action_before_policy_failure = popup.target_positive();
                                if popup.policy_failures != 1 || !popup.action_before_policy_failure
                                {
                                    state.failure = "popupFailureWithoutNativeAction";
                                }
                            } else {
                                state.failure = "popupUnexpectedNativeFailure";
                            }
                        } else if dialog_selected {
                            state.callback_affinity &=
                                callback_thread == main_thread && callback_sta;
                            let mut install = policy.as_ref().unwrap().lock().unwrap();
                            let dialog = install.dialog_refusal.as_mut().unwrap();
                            if owned == request && reason == Code0SessionFailure::PolicyRejected {
                                dialog.policy_failures = dialog.policy_failures.saturating_add(1);
                                dialog.action_before_policy_failure = dialog.target_positive();
                                if dialog.policy_failures != 1
                                    || !dialog.action_before_policy_failure
                                {
                                    state.failure = "dialogFailureWithoutNativeAction";
                                }
                            } else {
                                state.failure = "dialogUnexpectedNativeFailure";
                            }
                        } else if child_selected {
                            state.callback_affinity &=
                                callback_thread == main_thread && callback_sta;
                            let mut install = policy.as_ref().unwrap().lock().unwrap();
                            let child = install.child_refusal.as_mut().unwrap();
                            if owned == request && reason == Code0SessionFailure::PolicyRejected {
                                child.policy_failures = child.policy_failures.saturating_add(1);
                                child.action_before_policy_failure = child.target_receipts == 1
                                    && child.receipt_installation_matched
                                    && ChildRefusalEvidence::facts_positive(&child.facts);
                                if child.policy_failures != 1 || !child.action_before_policy_failure
                                {
                                    state.failure = "childFailureWithoutNativeAction";
                                }
                            } else {
                                state.failure = "childUnexpectedNativeFailure";
                            }
                        } else if top_selected {
                            state.callback_affinity &=
                                callback_thread == main_thread && callback_sta;
                            let mut install = policy.as_ref().unwrap().lock().unwrap();
                            let top = install.top_refusal.as_mut().unwrap();
                            if owned == request && reason == Code0SessionFailure::PolicyRejected {
                                // Count only the real once-only events(request, Failed)
                                // channel. The opaque receipt joins this request to
                                // the same native installation; no extra generation
                                // is invented for the public session event.
                                top.policy_failures = top.policy_failures.saturating_add(1);
                                top.action_before_policy_failure = top.target_receipts == 1
                                    && top.receipt_installation_matched
                                    && TopRefusalEvidence::facts_positive(&top.facts);
                                if top.policy_failures != 1 || !top.action_before_policy_failure {
                                    state.failure = "topFailureWithoutNativeAction";
                                }
                                // Native Context already marked failed/closing.
                                // The original close budget was already adopted
                                // from the actual top callback; no failure-event
                                // ACK or NavigationCompleted is awaited.
                            } else {
                                state.failure = if reason == Code0SessionFailure::CleanupFailed {
                                    "topNativeCleanupFailed"
                                } else {
                                    "topUnexpectedNativeFailure"
                                };
                            }
                        } else if let Some(coverage) = &coverage {
                            state.callback_affinity &=
                                callback_thread == main_thread && callback_sta;
                            let mut coverage = coverage.lock().unwrap();
                            if owned == request
                                && reason == Code0SessionFailure::PolicyRejected
                                && coverage.case.stale_disposition()
                            {
                                coverage.stale.policy_failures =
                                    coverage.stale.policy_failures.saturating_add(1);
                                if coverage.stale.held_receipts != 1
                                    || coverage.stale.close_revocations != 1
                                    || coverage.stale.drained_receipts != 1
                                    || coverage.stale.policy_failures != 1
                                {
                                    state.failure = "staleFailureWithoutDrain";
                                }
                            } else {
                                if owned == request && reason == Code0SessionFailure::PolicyRejected
                                {
                                    coverage.containment.policy_failures =
                                        coverage.containment.policy_failures.saturating_add(1);
                                }
                                state.failure = if reason == Code0SessionFailure::CleanupFailed {
                                    "coverageNativeCleanupFailed"
                                } else {
                                    "coverageUnexpectedNativeFailure"
                                };
                            }
                        } else if let Some(fault) = &fault {
                            state.callback_affinity &=
                                callback_thread == main_thread && callback_sta;
                            let mut fault = fault.lock().unwrap();
                            if owned == request && reason == Code0SessionFailure::PolicyRejected {
                                fault.policy_failures = fault.policy_failures.saturating_add(1);
                                state.failure = if fault.target_events == 1
                                    && fault.injection_receipts == 1
                                    && fault.policy_failures == 1
                                {
                                    state.failure
                                } else {
                                    "faultFailureWithoutInjection"
                                };
                            } else {
                                state.failure = if reason == Code0SessionFailure::CleanupFailed {
                                    "faultNativeCleanupFailed"
                                } else {
                                    "faultUnexpectedNativeFailure"
                                };
                            }
                        } else {
                            state.failure = "unexpectedProductionEvent";
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
            let fault_target_ready = fault.as_ref().is_some_and(|fault| {
                let fault = fault.lock().unwrap();
                (fault.case == PolicyFixtureCase::InstallPrematureLoadDenied
                    && fault.target_events == 1)
                    || fault.policy_failures != 0
            });
            let coverage_ready = coverage.as_ref().is_some_and(|coverage| {
                let coverage = coverage.lock().unwrap();
                if coverage.case.stale_disposition() {
                    coverage.stale.held_receipts == 1
                } else {
                    coverage.containment.complete_receipts == 1
                }
            });
            let policy_document_ready = policy
                .as_ref()
                .is_some_and(|policy| policy.lock().unwrap().document_receipts != 0);
            let native_install_observed = !capability_selected
                && !chooser_selected
                && !download_selected
                && !popup_selected
                && !top_selected
                && !child_selected
                && !dialog_selected
                && policy.as_ref().is_some_and(|policy| {
                    policy
                        .lock()
                        .unwrap()
                        .native_refusal
                        .as_ref()
                        .is_some_and(|native| native.receipts == 1)
                });
            let capability_action_observed =
                capability_selected && native_capability_close.get().is_some();
            let chooser_action_observed = chooser_selected && native_chooser_close.get().is_some();
            let download_action_observed =
                download_selected && native_download_close.get().is_some();
            let popup_action_observed = popup_selected && native_popup_close.get().is_some();
            let dialog_action_observed = dialog_selected && native_dialog_close.get().is_some();
            let top_action_observed = top_selected && native_top_close.get().is_some();
            let child_action_observed = child_selected && native_child_close.get().is_some();
            if policy.is_some()
                && closing_since.is_none()
                && started.elapsed() >= Duration::from_secs(45)
                && state.failure == "notCompleted"
            {
                state.failure = if capability_selected {
                    "capabilityTargetUnobserved"
                } else if chooser_selected {
                    "chooserTargetUnobserved"
                } else if download_selected {
                    "downloadTargetUnobserved"
                } else if popup_selected {
                    "popupTargetUnobserved"
                } else if dialog_selected {
                    "dialogTargetUnobserved"
                } else if child_selected {
                    "childTargetUnobserved"
                } else if top_selected {
                    "topTargetUnobserved"
                } else if coverage.is_some() {
                    "coveragePreconditionMissing"
                } else if fault.is_some() {
                    "faultTargetPreconditionFailed"
                } else {
                    "installAcknowledgementsMissing"
                };
            }
            if !object_free_failure
                && closing_since.is_none()
                && (state.renderer_message
                    || policy_document_ready
                    || native_install_observed
                    || top_action_observed
                    || child_action_observed
                    || dialog_action_observed
                    || popup_action_observed
                    || download_action_observed
                    || chooser_action_observed
                    || capability_action_observed
                    || fault_target_ready
                    || coverage_ready
                    || started.elapsed() >= Duration::from_secs(45)
                    || state.failure != "notCompleted")
            {
                let closing = if capability_selected {
                    native_capability_close.get()
                } else if chooser_selected {
                    native_chooser_close.get()
                } else if download_selected {
                    native_download_close.get()
                } else if popup_selected {
                    native_popup_close.get()
                } else if dialog_selected {
                    native_dialog_close.get()
                } else if child_selected {
                    native_child_close.get()
                } else {
                    native_top_close.get()
                }
                .unwrap_or_else(Instant::now);
                closing_since = Some(closing);
                ui_diagnostics.close_budget_started(closing);
                drop(state);
                ui_diagnostics.phase(DP::CloseRequest, false);
                driver.close(instance);
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
                    let snapshot = ui_diagnostics.close_deadline_snapshot_once(closing);
                    state.failure = "closeEvidenceTimeout";
                    if let Some(snapshot) = snapshot {
                        drop(state);
                        snapshot.emit();
                        ui_diagnostics.emit_popup_close_deadline();
                        state = observed.lock().unwrap();
                    }
                }
                if state.destroyed && state.owned_references_released {
                    let positive = policy.as_ref().map_or_else(
                        || state.positive(),
                        |policy| {
                            let install = policy.lock().unwrap();
                            coverage.as_ref().map_or_else(
                                || {
                                    fault.as_ref().map_or_else(
                                        || install.positive(&state),
                                        |fault| fault.lock().unwrap().positive(&install, &state),
                                    )
                                },
                                |coverage| coverage.lock().unwrap().positive(&install, &state),
                            )
                        },
                    );
                    if let Some(fault) = &fault
                        && !positive
                        && state.failure == "notCompleted"
                    {
                        let fault = fault.lock().unwrap();
                        state.failure = if fault.target_events != 1 {
                            "faultTargetPreconditionFailed"
                        } else if fault.final_events != 1 {
                            "faultFinalReceiptMissing"
                        } else {
                            "faultPositiveOracleMismatch"
                        };
                    }
                    if coverage.is_some() && !positive && state.failure == "notCompleted" {
                        state.failure = "coveragePositiveOracleMismatch";
                    }
                    if capability_selected && !positive && state.failure == "notCompleted" {
                        state.failure = policy
                            .as_ref()
                            .unwrap()
                            .lock()
                            .unwrap()
                            .capability
                            .as_ref()
                            .unwrap()
                            .failure();
                    }
                    if chooser_selected && !positive && state.failure == "notCompleted" {
                        state.failure = policy
                            .as_ref()
                            .unwrap()
                            .lock()
                            .unwrap()
                            .chooser
                            .as_ref()
                            .unwrap()
                            .failure();
                    }
                    if download_selected && !positive && state.failure == "notCompleted" {
                        state.failure = policy
                            .as_ref()
                            .unwrap()
                            .lock()
                            .unwrap()
                            .download_refusal
                            .as_ref()
                            .unwrap()
                            .failure();
                    }
                    if popup_selected && !positive && state.failure == "notCompleted" {
                        state.failure = policy
                            .as_ref()
                            .unwrap()
                            .lock()
                            .unwrap()
                            .popup_refusal
                            .as_ref()
                            .unwrap()
                            .failure();
                    }
                    if dialog_selected && !positive && state.failure == "notCompleted" {
                        state.failure = policy
                            .as_ref()
                            .unwrap()
                            .lock()
                            .unwrap()
                            .dialog_refusal
                            .as_ref()
                            .unwrap()
                            .failure();
                    }
                    let code = if positive && state.failure == "notCompleted" {
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
        if let Some(policy) = &final_policy
            && let Some(capability) = policy.lock().unwrap().capability.as_mut()
        {
            capability.facts.temp_dir_closed = cleanup_succeeded && state.run_returned;
        }
        let positive = final_policy.as_ref().map_or_else(
            || state.positive(),
            |policy| {
                let install = policy.lock().unwrap();
                final_coverage.as_ref().map_or_else(
                    || {
                        final_fault.as_ref().map_or_else(
                            || install.positive(&state),
                            |fault| fault.lock().unwrap().positive(&install, &state),
                        )
                    },
                    |coverage| coverage.lock().unwrap().positive(&install, &state),
                )
            },
        );
        state.pass =
            exit_code == 0 && state.failure == "notCompleted" && positive && cleanup_succeeded;
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
    let result = {
        use ai_token_meter_windows::platform::windows::code0_webview::fixture::PolicyFixtureCase;
        let arguments: Vec<_> = std::env::args_os().skip(1).collect();
        match arguments.as_slice() {
            [] => smoke::run(),
            [selector] if selector == "--code0-policy-fixture=install-ack-before-first-load" => {
                smoke::run_policy(false)
            }
            [selector]
                if selector
                    == "--code0-policy-fixture=install-native-refusal-hooks-before-load" =>
            {
                smoke::run_policy(true)
            }
            [selector]
                if selector == "--code0-policy-fixture=refusal-download-initial-data-empty" =>
            {
                smoke::run_download_refusal()
            }
            [selector] if selector=="--code0-policy-fixture=s4-capability-open-main" => smoke::run_capability(ai_token_meter_windows::platform::windows::code0_webview::fixture::NativeCapabilitySelector::OpenMain),
            [selector] if selector=="--code0-policy-fixture=s4-capability-write-main" => smoke::run_capability(ai_token_meter_windows::platform::windows::code0_webview::fixture::NativeCapabilitySelector::WriteMain),
            [selector] if selector=="--code0-policy-fixture=s4-capability-open-child-prototype" => smoke::run_capability(ai_token_meter_windows::platform::windows::code0_webview::fixture::NativeCapabilitySelector::OpenChildPrototype),
            [selector] if selector=="--code0-policy-fixture=s4-capability-dispose-missing" => smoke::run_capability(ai_token_meter_windows::platform::windows::code0_webview::fixture::NativeCapabilitySelector::DisposeMissing),
            [selector]
                if selector == "--code0-policy-fixture=chooser-prebound-owned-empty-file" =>
            {
                smoke::run_chooser()
            }
            [selector] if selector == "--code0-policy-fixture=refusal-popup-initial-blank" => {
                smoke::run_popup_refusal()
            }
            [selector] if selector == "--code0-policy-fixture=refusal-dialog-owned-empty-html" => {
                smoke::run_dialog_refusal()
            }
            [selector] if selector == "--code0-policy-fixture=refusal-child-initial-data" => {
                smoke::run_child_refusal()
            }
            [selector] if selector == "--code0-policy-fixture=refusal-top-owned-file" => {
                smoke::run_top_refusal()
            }
            [selector] if selector == "--code0-policy-fixture=install-premature-load-denied" => {
                smoke::run_fault(PolicyFixtureCase::InstallPrematureLoadDenied)
            }
            [selector] if selector == "--code0-policy-fixture=install-setting-injected-failure" => {
                smoke::run_fault(PolicyFixtureCase::InstallSettingInjectedFailure)
            }
            [selector] if selector == "--code0-policy-fixture=install-add-injected-failure" => {
                smoke::run_fault(PolicyFixtureCase::InstallAddInjectedFailure)
            }
            [selector] if selector == "--code0-policy-fixture=install-filter-injected-failure" => {
                smoke::run_fault(PolicyFixtureCase::InstallFilterInjectedFailure)
            }
            [selector] if selector == "--code0-policy-fixture=install-script-injected-failure" => {
                smoke::run_fault(PolicyFixtureCase::InstallScriptInjectedFailure)
            }
            [selector] if selector == "--code0-policy-fixture=install-cdp-injected-failure" => {
                smoke::run_fault(PolicyFixtureCase::InstallCdpInjectedFailure)
            }
            [selector] if selector == "--code0-policy-fixture=install-script-stale-disposition" => {
                smoke::run_coverage(PolicyFixtureCase::InstallScriptStaleDisposition)
            }
            [selector] if selector == "--code0-policy-fixture=install-cdp-stale-disposition" => {
                smoke::run_coverage(PolicyFixtureCase::InstallCdpStaleDisposition)
            }
            [selector] if selector == "--code0-policy-fixture=install-realms-no-passthrough" => {
                smoke::run_coverage(PolicyFixtureCase::InstallRealmsNoPassthrough)
            }
            _ => Err("unknown or multiple fixture selectors".into()),
        }
    };
    #[cfg(all(windows, debug_assertions, feature = "code0-native-fixture"))]
    match result {
        Ok(true) => (),
        Ok(false) => std::process::exit(1),
        Err(_) => {
            eprintln!("Code0 native fixture failed; inspect fresh fixed-schema summary");
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
