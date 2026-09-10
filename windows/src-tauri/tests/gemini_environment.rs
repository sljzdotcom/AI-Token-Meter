use ai_token_meter_windows::collectors::{CollectionError, gemini_environment::GeminiEnvironment};
use std::ffi::OsString;

fn inputs(home: &std::path::Path) -> Vec<(OsString, OsString)> {
    vec![
        ("USERPROFILE".into(), home.into()),
        ("HOME".into(), home.into()),
        ("PATH".into(), "synthetic-bin".into()),
    ]
}

#[test]
fn preserves_login_home_and_builds_exact_headless_commands() {
    let dir = tempfile::tempdir().unwrap();
    let environment = GeminiEnvironment::prepare(dir.path(), inputs(dir.path()), &[]).unwrap();
    let map: std::collections::HashMap<_, _> = environment.variables.iter().cloned().collect();
    assert_eq!(
        map.get(&OsString::from("HOME")),
        Some(&dir.path().as_os_str().to_os_string())
    );
    assert_eq!(
        map.get(&OsString::from("NO_BROWSER")),
        Some(&OsString::from("true"))
    );
    assert_eq!(
        std::fs::read(environment.directory.join(".env")).unwrap(),
        b""
    );
    assert_eq!(&environment.arguments(true)[..1], ["--version".to_owned()]);
    assert_eq!(
        &environment.arguments(false)[..4],
        [
            "-p".to_owned(),
            "/usage".to_owned(),
            "--print-timeout".to_owned(),
            "20s".to_owned()
        ]
    );
    for arguments in [environment.arguments(true), environment.arguments(false)] {
        assert!(arguments.windows(2).any(|pair| {
            pair[0] == "--log-file"
                && pair[1] == environment.directory.join("agy.log").to_string_lossy()
        }));
    }
    let path = environment.directory.clone();
    drop(environment);
    assert!(!path.exists());
}

#[test]
fn inherited_code_execution_and_network_overrides_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    for key in [
        "AGY_DEBUG",
        "GEMINI_API_KEY",
        "GOOGLE_API_KEY",
        "GCLOUD_PROJECT",
        "CLOUDSDK_CONFIG",
        "NODE_OPTIONS",
        "HTTP_PROXY",
        "DYLD_INSERT_LIBRARIES",
        "LD_PRELOAD",
        "BASH_ENV",
    ] {
        let mut environment = inputs(dir.path());
        environment.push((key.into(), "synthetic".into()));
        assert!(
            matches!(
                GeminiEnvironment::prepare(dir.path(), environment, &[]),
                Err(CollectionError::UnsupportedConfiguration)
            ),
            "{key}"
        );
    }
}

#[test]
fn user_settings_and_legacy_policy_paths_are_never_read_or_modified() {
    let dir = tempfile::tempdir().unwrap();
    let settings_directory = dir.path().join(".gemini");
    std::fs::create_dir(&settings_directory).unwrap();
    let settings = settings_directory.join("settings.json");
    std::fs::write(&settings, "{malformed-and-private").unwrap();
    let legacy_policy = dir.path().join("system.json");
    std::fs::write(&legacy_policy, "keep-me").unwrap();

    let environment = GeminiEnvironment::prepare(
        dir.path(),
        inputs(dir.path()),
        std::slice::from_ref(&legacy_policy),
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(settings).unwrap(),
        "{malformed-and-private"
    );
    assert_eq!(std::fs::read_to_string(legacy_policy).unwrap(), "keep-me");
    drop(environment);
}
