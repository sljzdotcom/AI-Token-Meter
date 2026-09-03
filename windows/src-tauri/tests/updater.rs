use ai_token_meter_windows::updater::{UpdatePhase, UpdateState};

#[test]
fn update_state_allows_only_explicit_check_then_explicit_install() {
    let mut state = UpdateState::new("0.2.2");
    assert_eq!(state.phase, UpdatePhase::Idle);
    assert!(!state.can_install());

    state.begin_check().expect("manual check");
    assert_eq!(state.phase, UpdatePhase::Checking);
    state
        .finish_check(Some("0.3.0-preview.1".to_owned()))
        .expect("available update");
    assert_eq!(state.phase, UpdatePhase::Available);
    assert!(state.can_install());

    state.begin_install().expect("explicit install");
    assert_eq!(state.phase, UpdatePhase::Downloading);
    assert!(!state.can_install());
    state.report_progress(25, Some(100));
    assert_eq!(state.progress_percent, Some(25));
    state.mark_installing();
    assert_eq!(state.phase, UpdatePhase::Installing);
}

#[test]
fn no_update_and_failures_never_enable_installation_or_erase_current_version() {
    let mut state = UpdateState::new("0.2.2");
    state.begin_check().expect("manual check");
    state.finish_check(None).expect("up to date");
    assert_eq!(state.phase, UpdatePhase::UpToDate);
    assert_eq!(state.current_version, "0.2.2");
    assert!(!state.can_install());
    assert!(state.begin_install().is_err());

    state.begin_check().expect("retry check");
    state.fail("Update service is temporarily unavailable");
    assert_eq!(state.phase, UpdatePhase::Failed);
    assert_eq!(state.current_version, "0.2.2");
    assert_eq!(
        state.message.as_deref(),
        Some("Update service is temporarily unavailable")
    );
    assert!(!state.can_install());
}

#[test]
fn stale_or_invalid_progress_cannot_corrupt_the_displayed_percentage() {
    let mut state = UpdateState::new("0.2.2");
    state.begin_check().expect("manual check");
    state
        .finish_check(Some("0.3.0-preview.1".to_owned()))
        .expect("available update");
    state.begin_install().expect("install");

    state.report_progress(50, Some(0));
    assert_eq!(state.progress_percent, None);
    state.report_progress(250, Some(100));
    assert_eq!(state.progress_percent, Some(100));
    state.report_progress(1, None);
    assert_eq!(state.progress_percent, None);
}

#[test]
fn overlapping_checks_and_invalid_versions_are_rejected() {
    let mut state = UpdateState::new("0.2.2");
    state.begin_check().expect("first check");
    assert!(state.begin_check().is_err());
    assert!(
        state
            .finish_check(Some("not a version".to_owned()))
            .is_err()
    );
    assert_eq!(state.phase, UpdatePhase::Failed);
    assert!(!state.can_install());
}
