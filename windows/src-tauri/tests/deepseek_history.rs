use ai_token_meter_windows::collectors::deepseek_history::{
    AcceptOutcome, DeepSeekHistoryAssembler, DeepSeekHistoryChunk, DeepSeekHistoryError,
    apply_history,
};
use ai_token_meter_windows::domain::{
    MetricKind, MetricUnit, ProviderId, Ratio, UsageMetric, UsageSnapshot, UsageStatus,
};
use ai_token_meter_windows::platform::windows::deepseek_history_window::{
    DeepSeekHistoryChunkOutcome, DeepSeekHistoryReadyResolution, DeepSeekHistoryWindowAction,
    DeepSeekHistoryWindowActionExecutor, DeepSeekHistoryWindowCoordinator,
    DeepSeekHistoryWindowExecution, DeepSeekHistoryWindowStatus, execute_window_actions,
};
use ai_token_meter_windows::platform::windows::deepseek_webview::{
    DeepSeekBridgeCallback, is_allowed_deepseek_navigation, isolated_profile_directory,
    parse_bridge_callback, parse_bridge_message,
};
use ai_token_meter_windows::{ProviderDetailEffect, provider_detail_effects};
use reqwest::Url;
use time::macros::datetime;

const NONCE: &str = "0123456789abcdef0123456789abcdef";
const NEXT_NONCE: &str = "fedcba9876543210fedcba9876543210";

#[test]
fn initial_open_creates_one_hidden_window_and_repeated_open_focuses_it() {
    let mut coordinator = DeepSeekHistoryWindowCoordinator::default();

    assert_eq!(coordinator.status(), DeepSeekHistoryWindowStatus::Idle);
    let initial = coordinator.open(NONCE, datetime!(2026-09-04 10:00 UTC));
    assert_eq!(
        initial.actions,
        [
            DeepSeekHistoryWindowAction::CreateHidden,
            DeepSeekHistoryWindowAction::EmitStatus(DeepSeekHistoryWindowStatus::Opening),
        ]
    );
    assert_eq!(coordinator.status(), DeepSeekHistoryWindowStatus::Opening);
    assert!(coordinator.has_session());

    let repeated = coordinator.open(NEXT_NONCE, datetime!(2026-09-04 10:00:01 UTC));
    assert_eq!(repeated.generation, initial.generation);
    assert_eq!(
        repeated.actions,
        [DeepSeekHistoryWindowAction::FocusExisting]
    );
    assert_eq!(coordinator.status(), DeepSeekHistoryWindowStatus::Opening);
    assert!(coordinator.has_session());
}

#[test]
fn page_load_completion_cannot_activate_without_a_nonce_bound_ready_message() {
    let mut coordinator = DeepSeekHistoryWindowCoordinator::default();
    let open = coordinator.open(NONCE, datetime!(2026-09-04 10:00 UTC));

    assert!(coordinator.navigation_finished(open.generation).is_empty());
    assert_eq!(coordinator.status(), DeepSeekHistoryWindowStatus::Opening);
    assert!(
        coordinator
            .claim_ready(open.generation, NEXT_NONCE)
            .is_none()
    );
    let ready = coordinator
        .claim_ready(open.generation, NONCE)
        .expect("current official ready message should claim activation");
    assert_eq!(ready.actions(), &[DeepSeekHistoryWindowAction::ShowFocused]);
    let mut executor = FakeWindowExecutor::default();
    let execution = execute_window_actions(ready.actions(), &mut executor);
    let DeepSeekHistoryReadyResolution::Activated(actions) =
        coordinator.finish_ready(ready, &execution)
    else {
        panic!("successful focus should activate the claimed generation");
    };
    assert_eq!(
        actions,
        [DeepSeekHistoryWindowAction::EmitStatus(
            DeepSeekHistoryWindowStatus::Active
        )]
    );
    assert_eq!(coordinator.status(), DeepSeekHistoryWindowStatus::Active);
}

#[test]
fn stale_window_callbacks_cannot_change_a_reopened_generation() {
    let mut coordinator = DeepSeekHistoryWindowCoordinator::default();
    let first = coordinator.open(NONCE, datetime!(2026-09-04 10:00 UTC));
    let cancelled = coordinator
        .claim_closed(first.generation)
        .expect("first close should claim cancellation");
    let mut executor = FakeWindowExecutor::default();
    let execution = execute_window_actions(cancelled.actions(), &mut executor);
    assert_eq!(
        coordinator.finish_terminal(cancelled, &execution),
        [DeepSeekHistoryWindowAction::EmitStatus(
            DeepSeekHistoryWindowStatus::Cancelled
        )]
    );
    let second = coordinator.open(NEXT_NONCE, datetime!(2026-09-04 10:00:01 UTC));
    assert_ne!(second.generation, first.generation);

    assert!(coordinator.navigation_finished(first.generation).is_empty());
    assert!(coordinator.claim_ready(first.generation, NONCE).is_none());
    assert!(coordinator.claim_timeout(first.generation).is_none());
    assert!(coordinator.claim_closed(first.generation).is_none());
    assert_eq!(
        coordinator
            .accept_chunk(
                first.generation,
                chunk("https://platform.deepseek.com", 0, 1, valid_payload()),
                datetime!(2026-09-04 10:00:02 UTC),
            )
            .unwrap(),
        DeepSeekHistoryChunkOutcome::Ignored
    );
    assert_eq!(coordinator.current_generation(), Some(second.generation));
    assert_eq!(coordinator.status(), DeepSeekHistoryWindowStatus::Opening);
    assert!(coordinator.has_session());
}

#[test]
fn unready_official_page_times_out_and_recovers() {
    let mut coordinator = DeepSeekHistoryWindowCoordinator::default();
    let open = coordinator.open(NONCE, datetime!(2026-09-04 10:00 UTC));
    coordinator.navigation_finished(open.generation);

    let timeout = coordinator
        .claim_timeout(open.generation)
        .expect("an unready opening generation must time out");
    assert_eq!(
        timeout.actions(),
        &[
            DeepSeekHistoryWindowAction::DestroyHistory,
            DeepSeekHistoryWindowAction::RestoreDetail,
        ]
    );
    let mut executor = FakeWindowExecutor::default();
    let execution = execute_window_actions(timeout.actions(), &mut executor);
    assert_eq!(
        coordinator.finish_terminal(timeout, &execution),
        [DeepSeekHistoryWindowAction::EmitStatus(
            DeepSeekHistoryWindowStatus::Failed
        )]
    );
    assert_eq!(coordinator.status(), DeepSeekHistoryWindowStatus::Failed);
    assert!(!coordinator.has_session());
}

#[test]
fn completed_payload_owns_the_terminal_before_timeout_or_close_can_win() {
    let mut coordinator = DeepSeekHistoryWindowCoordinator::default();
    let open = activate(&mut coordinator, NONCE);

    let DeepSeekHistoryChunkOutcome::Complete { history, terminal } = coordinator
        .accept_chunk(
            open.generation,
            chunk("https://platform.deepseek.com", 0, 1, valid_payload()),
            datetime!(2026-09-04 10:00:01 UTC),
        )
        .unwrap()
    else {
        panic!("complete chunk should reserve the terminal");
    };
    assert_eq!(history.days.len(), 1);
    assert!(coordinator.claim_timeout(open.generation).is_none());
    assert!(coordinator.claim_closed(open.generation).is_none());
    let mut executor = FakeWindowExecutor::default();
    let execution = execute_window_actions(terminal.actions(), &mut executor);
    assert_eq!(
        coordinator.finish_terminal(terminal, &execution),
        [DeepSeekHistoryWindowAction::EmitStatus(
            DeepSeekHistoryWindowStatus::Completed
        )]
    );
    assert_eq!(coordinator.status(), DeepSeekHistoryWindowStatus::Completed);
}

#[test]
fn close_ownership_prevents_a_late_complete_from_producing_history() {
    let mut coordinator = DeepSeekHistoryWindowCoordinator::default();
    let open = activate(&mut coordinator, NONCE);
    let _close = coordinator
        .claim_closed(open.generation)
        .expect("close should reserve the terminal");

    assert_eq!(
        coordinator
            .accept_chunk(
                open.generation,
                chunk("https://platform.deepseek.com", 0, 1, valid_payload()),
                datetime!(2026-09-04 10:00:01 UTC),
            )
            .unwrap(),
        DeepSeekHistoryChunkOutcome::Ignored
    );
}

#[test]
fn destroy_or_restore_failure_never_claims_a_success_terminal() {
    for failed_action in [
        DeepSeekHistoryWindowAction::DestroyHistory,
        DeepSeekHistoryWindowAction::RestoreDetail,
    ] {
        let mut coordinator = DeepSeekHistoryWindowCoordinator::default();
        let open = activate(&mut coordinator, NONCE);
        let DeepSeekHistoryChunkOutcome::Complete { terminal, .. } = coordinator
            .accept_chunk(
                open.generation,
                chunk("https://platform.deepseek.com", 0, 1, valid_payload()),
                datetime!(2026-09-04 10:00:01 UTC),
            )
            .unwrap()
        else {
            panic!("complete chunk should reserve the terminal");
        };
        let mut executor = FakeWindowExecutor::failing(failed_action);
        let execution = execute_window_actions(terminal.actions(), &mut executor);
        assert_eq!(
            executor.attempted,
            [
                DeepSeekHistoryWindowAction::DestroyHistory,
                DeepSeekHistoryWindowAction::RestoreDetail,
            ]
        );
        assert_eq!(execution.failed_actions(), &[failed_action]);
        assert_eq!(
            coordinator.finish_terminal(terminal, &execution),
            [DeepSeekHistoryWindowAction::EmitStatus(
                DeepSeekHistoryWindowStatus::Failed
            )]
        );
        assert_eq!(coordinator.status(), DeepSeekHistoryWindowStatus::Failed);
    }
}

#[test]
fn apply_or_publish_failure_never_claims_completed() {
    let mut coordinator = DeepSeekHistoryWindowCoordinator::default();
    let open = activate(&mut coordinator, NONCE);
    let DeepSeekHistoryChunkOutcome::Complete { terminal, .. } = coordinator
        .accept_chunk(
            open.generation,
            chunk("https://platform.deepseek.com", 0, 1, valid_payload()),
            datetime!(2026-09-04 10:00:01 UTC),
        )
        .unwrap()
    else {
        panic!("complete chunk should reserve the terminal");
    };
    let mut executor = FakeWindowExecutor::default();
    let mut execution: DeepSeekHistoryWindowExecution =
        execute_window_actions(terminal.actions(), &mut executor);
    execution.record_operation_failure();

    assert_eq!(
        coordinator.finish_terminal(terminal, &execution),
        [DeepSeekHistoryWindowAction::EmitStatus(
            DeepSeekHistoryWindowStatus::Failed
        )]
    );
    assert_eq!(coordinator.status(), DeepSeekHistoryWindowStatus::Failed);
}

#[test]
fn focus_failure_recovers_without_ever_claiming_active() {
    let mut coordinator = DeepSeekHistoryWindowCoordinator::default();
    let open = coordinator.open(NONCE, datetime!(2026-09-04 10:00 UTC));
    let ready = coordinator.claim_ready(open.generation, NONCE).unwrap();
    let mut executor = FakeWindowExecutor::failing(DeepSeekHistoryWindowAction::ShowFocused);
    let execution = execute_window_actions(ready.actions(), &mut executor);
    let DeepSeekHistoryReadyResolution::Recover(terminal) =
        coordinator.finish_ready(ready, &execution)
    else {
        panic!("failed focus must reserve a failed terminal");
    };
    assert_eq!(coordinator.status(), DeepSeekHistoryWindowStatus::Opening);
    let mut recovery_executor = FakeWindowExecutor::default();
    let recovery = execute_window_actions(terminal.actions(), &mut recovery_executor);
    assert_eq!(
        coordinator.finish_terminal(terminal, &recovery),
        [DeepSeekHistoryWindowAction::EmitStatus(
            DeepSeekHistoryWindowStatus::Failed
        )]
    );
    assert_eq!(coordinator.status(), DeepSeekHistoryWindowStatus::Failed);
}

#[derive(Default)]
struct FakeWindowExecutor {
    failed_action: Option<DeepSeekHistoryWindowAction>,
    attempted: Vec<DeepSeekHistoryWindowAction>,
}

impl FakeWindowExecutor {
    fn failing(action: DeepSeekHistoryWindowAction) -> Self {
        Self {
            failed_action: Some(action),
            attempted: Vec::new(),
        }
    }
}

impl DeepSeekHistoryWindowActionExecutor for FakeWindowExecutor {
    type Error = ();

    fn execute(&mut self, action: DeepSeekHistoryWindowAction) -> Result<(), Self::Error> {
        self.attempted.push(action);
        if self.failed_action == Some(action) {
            Err(())
        } else {
            Ok(())
        }
    }
}

fn activate(
    coordinator: &mut DeepSeekHistoryWindowCoordinator,
    nonce: &str,
) -> ai_token_meter_windows::platform::windows::deepseek_history_window::DeepSeekHistoryWindowOpen {
    let open = coordinator.open(nonce, datetime!(2026-09-04 10:00 UTC));
    let ready = coordinator.claim_ready(open.generation, nonce).unwrap();
    let mut executor = FakeWindowExecutor::default();
    let execution = execute_window_actions(ready.actions(), &mut executor);
    assert!(matches!(
        coordinator.finish_ready(ready, &execution),
        DeepSeekHistoryReadyResolution::Activated(_)
    ));
    open
}

#[test]
fn opening_deepseek_detail_with_empty_history_does_not_start_official_history_sync() {
    let effects = provider_detail_effects(&balance_snapshot());

    assert_eq!(
        effects,
        [
            ProviderDetailEffect::ShowDetailWindow,
            ProviderDetailEffect::EmitActiveDetail,
        ]
    );
}

#[test]
fn webview_policy_uses_an_isolated_profile_and_exact_navigation_allowlist() {
    let local_app_data = std::path::Path::new(r"C:\Users\test\AppData\Local");
    assert_eq!(
        isolated_profile_directory(local_app_data),
        local_app_data
            .join("AI Token Meter")
            .join("WebView2")
            .join("DeepSeek")
    );

    assert!(is_allowed_deepseek_navigation(
        &Url::parse("https://platform.deepseek.com/usage").unwrap()
    ));
    assert!(is_allowed_deepseek_navigation(
        &Url::parse("https://platform.deepseek.com/sign_in").unwrap()
    ));
    assert!(!is_allowed_deepseek_navigation(
        &Url::parse("http://platform.deepseek.com/usage").unwrap()
    ));
    assert!(!is_allowed_deepseek_navigation(
        &Url::parse("https://platform.deepseek.com.evil.example/usage").unwrap()
    ));
}

#[test]
fn bridge_callback_parses_only_the_private_callback_scheme() {
    let callback = Url::parse(&format!(
        "aimeter-deepseek://history?nonce={NONCE}&origin=https%3A%2F%2Fplatform.deepseek.com&sequence=0&total=1&payload=%7B%7D"
    ))
    .unwrap();
    let chunk = parse_bridge_callback(&callback).unwrap();
    assert_eq!(chunk.nonce, NONCE);
    assert_eq!(chunk.origin, "https://platform.deepseek.com");
    assert_eq!(chunk.sequence, 0);
    assert_eq!(chunk.total, 1);
    assert_eq!(chunk.payload_fragment, "{}");

    for value in [
        "https://platform.deepseek.com/usage",
        "aimeter-deepseek://other?nonce=x",
        "aimeter-deepseek://history?nonce=x&origin=x&sequence=0&total=1",
    ] {
        assert!(parse_bridge_callback(&Url::parse(value).unwrap()).is_err());
    }
}

#[test]
fn ready_callback_requires_the_nonce_and_exact_official_origin() {
    let ready = Url::parse(&format!(
        "aimeter-deepseek://history?nonce={NONCE}&origin=https%3A%2F%2Fplatform.deepseek.com&ready=1"
    ))
    .unwrap();
    assert_eq!(
        parse_bridge_message(&ready).unwrap(),
        DeepSeekBridgeCallback::Ready {
            nonce: NONCE.to_owned(),
            origin: "https://platform.deepseek.com".to_owned(),
        }
    );

    for value in [
        format!(
            "aimeter-deepseek://history?nonce={NONCE}&origin=https%3A%2F%2Fplatform.deepseek.com.evil.example&ready=1"
        ),
        "aimeter-deepseek://history?nonce=wrong&origin=https%3A%2F%2Fplatform.deepseek.com&ready=1"
            .to_owned(),
        format!(
            "aimeter-deepseek://history?nonce={NONCE}&origin=https%3A%2F%2Fplatform.deepseek.com&ready=0"
        ),
    ] {
        assert!(parse_bridge_message(&Url::parse(&value).unwrap()).is_err());
    }
}

#[test]
fn accepts_only_the_official_https_console_origin() {
    let mut assembler = DeepSeekHistoryAssembler::new(NONCE, datetime!(2026-09-03 12:00 UTC));

    let accepted = assembler.accept(
        chunk("https://platform.deepseek.com", 0, 1, valid_payload()),
        datetime!(2026-09-03 12:00:01 UTC),
    );
    assert!(matches!(accepted, Ok(AcceptOutcome::Complete(_))));

    for origin in [
        "http://platform.deepseek.com",
        "https://platform.deepseek.com.evil.example",
        "https://user@platform.deepseek.com",
        "https://api.deepseek.com",
    ] {
        let mut assembler = DeepSeekHistoryAssembler::new(NONCE, datetime!(2026-09-03 12:00 UTC));
        let error = assembler
            .accept(
                chunk(origin, 0, 1, valid_payload()),
                datetime!(2026-09-03 12:00:01 UTC),
            )
            .expect_err("non-console origins must be rejected");
        assert_eq!(error, DeepSeekHistoryError::UnsafeOrigin);
    }
}

#[test]
fn reassembles_out_of_order_chunks_and_merges_duplicate_days() {
    let payload = r#"{"schemaVersion":1,"days":[{"date":"2026-09-02","costCny":1.25,"requests":4,"tokens":1000},{"date":"2026-09-02","costCny":0.75,"requests":2,"tokens":500},{"date":"2026-08-01","costCny":99,"requests":99,"tokens":99},{"date":"2026-09-03","costCny":2.5,"requests":7,"tokens":2500}]}"#;
    let split = payload.len() / 2;
    let mut assembler = DeepSeekHistoryAssembler::new(NONCE, datetime!(2026-09-03 12:00 UTC));

    assert_eq!(
        assembler
            .accept(
                chunk("https://platform.deepseek.com", 1, 2, &payload[split..]),
                datetime!(2026-09-03 12:00:01 UTC),
            )
            .unwrap(),
        AcceptOutcome::Waiting
    );
    let AcceptOutcome::Complete(history) = assembler
        .accept(
            chunk("https://platform.deepseek.com", 0, 2, &payload[..split]),
            datetime!(2026-09-03 12:00:02 UTC),
        )
        .unwrap()
    else {
        panic!("second chunk should complete the payload");
    };

    assert_eq!(history.days.len(), 2);
    assert_eq!(history.days[0].date, "2026-09-02");
    assert_eq!(history.days[0].cost_cny, 2.0);
    assert_eq!(history.days[0].requests, 6);
    assert_eq!(history.days[0].tokens, 1500);
    assert_eq!(history.total_cost_cny, 4.5);
    assert_eq!(history.total_requests, 13);
    assert_eq!(history.total_tokens, 4000);
}

#[test]
fn rejects_expired_mismatched_oversized_and_secret_bearing_payloads() {
    let cases = [
        (
            DeepSeekHistoryChunk {
                nonce: "wrong".to_owned(),
                ..chunk("https://platform.deepseek.com", 0, 1, valid_payload())
            },
            datetime!(2026-09-03 12:00:01 UTC),
            DeepSeekHistoryError::NonceMismatch,
        ),
        (
            chunk("https://platform.deepseek.com", 0, 1, valid_payload()),
            datetime!(2026-09-03 12:00:21 UTC),
            DeepSeekHistoryError::Expired,
        ),
        (
            chunk("https://platform.deepseek.com", 0, 65, "{}"),
            datetime!(2026-09-03 12:00:01 UTC),
            DeepSeekHistoryError::ChunkLimitExceeded,
        ),
        (
            chunk(
                "https://platform.deepseek.com",
                0,
                1,
                r#"{"schemaVersion":1,"authorization":"Bearer secret","days":[]}"#,
            ),
            datetime!(2026-09-03 12:00:01 UTC),
            DeepSeekHistoryError::SensitivePayload,
        ),
    ];

    for (chunk, now, expected) in cases {
        let mut assembler = DeepSeekHistoryAssembler::new(NONCE, datetime!(2026-09-03 12:00 UTC));
        assert_eq!(assembler.accept(chunk, now).unwrap_err(), expected);
    }

    let mut assembler = DeepSeekHistoryAssembler::new(NONCE, datetime!(2026-09-03 12:00 UTC));
    let huge = "x".repeat(16 * 1024 + 1);
    assert_eq!(
        assembler
            .accept(
                chunk("https://platform.deepseek.com", 0, 1, &huge),
                datetime!(2026-09-03 12:00:01 UTC),
            )
            .unwrap_err(),
        DeepSeekHistoryError::ChunkLimitExceeded
    );
}

#[test]
fn malformed_history_never_overwrites_the_balance_snapshot() {
    let balance = balance_snapshot();
    let mut assembler = DeepSeekHistoryAssembler::new(NONCE, datetime!(2026-09-03 12:00 UTC));
    let result = assembler.accept(
        chunk(
            "https://platform.deepseek.com",
            0,
            1,
            r#"{"schemaVersion":1,"days":[{"date":"2026-09-03","costCny":-1,"requests":1,"tokens":1}]}"#,
        ),
        datetime!(2026-09-03 12:00:01 UTC),
    );

    assert_eq!(result.unwrap_err(), DeepSeekHistoryError::InvalidPayload);
    assert_eq!(balance, balance_snapshot());
}

#[test]
fn applying_valid_history_changes_only_the_history_fields() {
    let original = balance_snapshot();
    let mut assembler = DeepSeekHistoryAssembler::new(NONCE, datetime!(2026-09-03 12:00 UTC));
    let AcceptOutcome::Complete(history) = assembler
        .accept(
            chunk("https://platform.deepseek.com", 0, 1, valid_payload()),
            datetime!(2026-09-03 12:00:01 UTC),
        )
        .unwrap()
    else {
        panic!("single chunk must complete");
    };

    let updated = apply_history(&original, &history).unwrap();
    assert_eq!(updated.daily_history, history.days);
    assert_eq!(
        updated.history_fetched_at.as_deref(),
        Some("2026-09-03T12:00:01Z")
    );
    assert_eq!(updated.primary_metric, original.primary_metric);
    assert_eq!(updated.used_ratio, original.used_ratio);
    assert_eq!(updated.status, original.status);
    assert_eq!(updated.fetched_at, original.fetched_at);
}

fn chunk(origin: &str, sequence: u16, total: u16, payload: &str) -> DeepSeekHistoryChunk {
    DeepSeekHistoryChunk {
        nonce: NONCE.to_owned(),
        origin: origin.to_owned(),
        sequence,
        total,
        payload_fragment: payload.to_owned(),
    }
}

fn valid_payload() -> &'static str {
    r#"{"schemaVersion":1,"days":[{"date":"2026-09-03","costCny":2.5,"requests":7,"tokens":2500}]}"#
}

fn balance_snapshot() -> UsageSnapshot {
    UsageSnapshot {
        schema_version: 1,
        provider_id: ProviderId::DeepSeek,
        display_name: "DeepSeek".to_owned(),
        status: UsageStatus::Fresh,
        used_ratio: Some(Ratio::new(0.22).unwrap()),
        primary_metric: Some(UsageMetric {
            label: "Balance".to_owned(),
            current: 77.99,
            limit: Some(100.0),
            unit: MetricUnit::Cny,
            kind: MetricKind::Balance,
            reset_at: None,
            reset_description: None,
        }),
        secondary_metric: None,
        fetched_at: "2026-09-03T12:00:00Z".to_owned(),
        stale_after_seconds: 300,
        source_version: None,
        status_message: None,
        reset_credits: Vec::new(),
        local_activity: None,
        daily_history: Vec::new(),
        history_fetched_at: None,
    }
}
