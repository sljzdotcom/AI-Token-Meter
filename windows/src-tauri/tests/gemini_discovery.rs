use ai_token_meter_windows::{
    collectors::{
        CollectionError, gemini_environment::GeminiEnvironment, gemini_runtime::discover,
    },
    platform::windows::{environment::DiscoveryInputs, process::CancellationToken},
};
use std::sync::Arc;
#[cfg(unix)]
#[test]
fn version_preflight_runs_real_bounded_process_in_the_same_isolation() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let environment = GeminiEnvironment::prepare(
        dir.path(),
        vec![("USERPROFILE".into(), dir.path().into())],
        &[],
    )
    .unwrap();
    let exe = dir.path().join("agy.exe");
    std::fs::write(&exe,"#!/bin/sh\n[ -f .env ] && [ \"$NO_BROWSER\" = true ] && [ \"$TERM\" = dumb ] || exit 7\ncase \"$*\" in *'--version'*'--log-file'*) printf '1.1.28\\n';; *) exit 9;; esac\n").unwrap();
    std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o700)).unwrap();
    let inputs = DiscoveryInputs {
        conventional_paths: vec![dir.path().into()],
        ..Default::default()
    };
    assert!(
        discover(
            &environment,
            inputs.clone(),
            Arc::new(CancellationToken::new())
        )
        .unwrap()
        .is_some()
    );
    std::fs::write(&exe, "#!/bin/sh\nprintf '1.1.27\\n'\n").unwrap();
    assert!(matches!(
        discover(
            &environment,
            inputs.clone(),
            Arc::new(CancellationToken::new())
        ),
        Err(CollectionError::UnsupportedVersion)
    ));
    std::fs::write(&exe, "#!/bin/sh\nprintf '2.0.0\\n'\n").unwrap();
    assert!(matches!(
        discover(
            &environment,
            inputs.clone(),
            Arc::new(CancellationToken::new())
        ),
        Err(CollectionError::UnsupportedVersion)
    ));
    std::fs::write(&exe, "not executable format").unwrap();
    assert!(matches!(
        discover(&environment, inputs, Arc::new(CancellationToken::new())),
        Err(CollectionError::Transport)
    ));
}
#[test]
fn no_candidate_and_cancelled_discovery_are_distinct() {
    let dir = tempfile::tempdir().unwrap();
    let env = GeminiEnvironment::prepare(
        dir.path(),
        vec![("USERPROFILE".into(), dir.path().into())],
        &[],
    )
    .unwrap();
    assert!(matches!(
        discover(
            &env,
            DiscoveryInputs::default(),
            Arc::new(CancellationToken::new())
        ),
        Ok(None)
    ));
    let token = Arc::new(CancellationToken::new());
    token.cancel();
    assert!(matches!(
        discover(&env, DiscoveryInputs::default(), token),
        Err(CollectionError::Cancelled)
    ));
}
