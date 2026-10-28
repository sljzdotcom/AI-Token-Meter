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
        Code0SessionFailure, Code0WindowDriving, Code0WindowEvent, Code0WindowMode,
        Code0WindowRequest,
    };
    use ai_token_meter_windows::platform::windows::code0_webview::{
        Code0NativeDriver,
        fixture::{
            ContainmentReceipt, DiagnosticMilestone as DM, DiagnosticPhase as DP, Diagnostics,
            NativeRefusalInstallReceipt, NativeSmokeObservation, PolicyFaultReceipt,
            PolicyFixtureCase, PolicyObservation, StaleDispositionReceipt, StaleDispositionStage,
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
            receipt: NativeRefusalInstallReceipt,
            request: &Code0WindowRequest,
        ) -> bool {
            self.receipts = self.receipts.saturating_add(1);
            if self.receipts != 1 || !receipt.matches(request) {
                return false;
            }
            self.native_instance_validated = true;
            // The opaque native receipt is read from the live Instance's only
            // supported registry state: all ten explicitly NotInstalled. This
            // is not an installed counter, cast or native-event coverage claim.
            self.source_bound_registry = true;
            self.required_mask = 0x03ff;
            self.installed_mask = 0;
            self.not_installed_hooks = receipt.not_installed().map(|hook| hook.name()).to_vec();
            self.navigation_not_issued = true;
            self.load_rejected = true;
            self.not_installed_hooks == NATIVE_REFUSAL_HOOKS
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
    }

    impl PolicyEvidence {
        fn positive(&self, lifecycle: &Summary) -> bool {
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
    }

    impl<'a> PolicySummary<'a> {
        fn snapshot(state: &'a Summary, install: &'a PolicyEvidence) -> Self {
            Self {
                schema: if install.native_refusal.is_some() {
                    6
                } else {
                    1
                },
                run_id: &state.run_id,
                required_case: if install.native_refusal.is_some() {
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

    pub fn run_policy(native_refusal: bool) -> Result<bool, Box<dyn std::error::Error>> {
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
            ..PolicyEvidence::default()
        }));
        write_summary(
            &mut file,
            &PolicySummary::snapshot(&summary.lock().unwrap(), &install.lock().unwrap()),
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
                        &PolicySummary::snapshot(
                            &failed,
                            &PolicyEvidence {
                                native_refusal: native_refusal.then(NativeRefusalEvidence::default),
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
        let write_result = write_summary(
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
                                if policy.lock().unwrap().native_refusal.is_some() {
                                    let receipt =
                                        driver.require_fixture_native_refusal_install(&request);
                                    let mut install = policy.lock().unwrap();
                                    let native = install.native_refusal.as_mut().unwrap();
                                    let valid_missing = receipt
                                        .is_ok_and(|receipt| native.observe(receipt, &request));
                                    drop(install);
                                    state = observed.lock().unwrap();
                                    state.failure = if valid_missing {
                                        "nativeHooksMissing"
                                    } else {
                                        "nativeHookInventoryPreconditionFailed"
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
                        if let Some(coverage) = &coverage {
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
            if policy.is_some()
                && closing_since.is_none()
                && started.elapsed() >= Duration::from_secs(45)
                && state.failure == "notCompleted"
            {
                state.failure = if coverage.is_some() {
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
                    || fault_target_ready
                    || coverage_ready
                    || started.elapsed() >= Duration::from_secs(45)
                    || state.failure != "notCompleted")
            {
                let closing = Instant::now();
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
