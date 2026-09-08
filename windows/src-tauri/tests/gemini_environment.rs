use ai_token_meter_windows::collectors::CollectionError;
use ai_token_meter_windows::collectors::gemini_environment::GeminiEnvironment;
use std::ffi::OsString;

fn inputs(home: &std::path::Path) -> Vec<(OsString, OsString)> {
    vec![
        ("USERPROFILE".into(), home.into()),
        ("HOME".into(), home.into()),
        ("PATH".into(), "synthetic-bin".into()),
    ]
}
#[test]
fn normal_oauth_keeps_home_and_disables_startup_side_effects_for_version_too() {
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
        map.get(&OsString::from("GEMINI_CLI_TRUST_WORKSPACE")),
        Some(&OsString::from("false"))
    );
    let settings: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            map.get(&OsString::from("GEMINI_CLI_SYSTEM_SETTINGS_PATH"))
                .unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(settings["hooksConfig"]["enabled"], false);
    assert_eq!(settings["privacy"]["usageStatisticsEnabled"], false);
    assert!(settings["security"]["folderTrust"].is_null());
    assert_eq!(
        std::fs::read(environment.directory.join(".env")).unwrap(),
        b""
    );
    let args = environment.arguments(true);
    assert!(args.contains(&"--version".to_owned()));
    assert!(args.windows(2).any(|a| a == ["-e", "none"]));
    assert!(
        args.windows(2)
            .any(|a| a[0] == "--allowed-mcp-server-names" && a[1].starts_with("ai-meter-"))
    );
    assert!(!args.iter().any(|a| a == "-p" || a == "--ignore-env"));
    let other = GeminiEnvironment::prepare(dir.path(), inputs(dir.path()), &[]).unwrap();
    assert_ne!(environment.arguments(false), other.arguments(false));
    let path = environment.directory.clone();
    drop(environment);
    assert!(!path.exists());
}
#[test]
fn unsupported_environment_or_existing_system_policy_is_never_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    for key in [
        "GEMINI_API_KEY",
        "GOOGLE_API_KEY",
        "GOOGLE_GENAI_USE_VERTEXAI",
        "GEMINI_FORCE_ENCRYPTED_FILE_STORAGE",
        "NODE_OPTIONS",
        "NODE_PATH",
        "GEMINI_CLI_SYSTEM_SETTINGS_PATH",
        "HTTP_PROXY",
        "GEMINI_SANDBOX",
        "SANDBOX",
        "CLOUD_SHELL",
        "BUILD_SANDBOX",
        "GEMINI_CLI_TRUST_WORKSPACE",
    ] {
        let mut env = inputs(dir.path());
        env.push((key.into(), "synthetic".into()));
        assert!(
            matches!(
                GeminiEnvironment::prepare(dir.path(), env, &[]),
                Err(CollectionError::UnsupportedConfiguration)
            ),
            "{key}"
        );
    }
    let system = dir.path().join("system.json");
    std::fs::write(&system, "{}").unwrap();
    assert!(
        GeminiEnvironment::prepare(
            dir.path(),
            inputs(dir.path()),
            std::slice::from_ref(&system)
        )
        .is_err()
    );
    assert_eq!(std::fs::read(system).unwrap(), b"{}");
}
#[test]
fn unsupported_user_settings_stop_before_any_cli_execution() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".gemini")).unwrap();
    for settings in [
        r#"{"security":{"auth":{"selectedType":"gemini-api-key"}}}"#,
        r#"{"advanced":{"ignoreLocalEnv":true}}"#,
        r#"{"tools":{"discoveryCommand":"echo sentinel"}}"#,
        r#"{"tools":{"sandbox":true}}"#,
        r#"{"security":{"toolSandboxing":true}}"#,
        r#"{"security":{"auth":{"useExternal":true}}}"#,
        r#"{"tools":{"callCommand":"synthetic-command"}}"#,
        "{broken",
    ] {
        std::fs::write(dir.path().join(".gemini/settings.json"), settings).unwrap();
        assert!(
            matches!(
                GeminiEnvironment::prepare(dir.path(), inputs(dir.path()), &[]),
                Err(CollectionError::UnsupportedConfiguration)
            ),
            "{settings}"
        );
    }
    std::fs::write(dir.path().join(".gemini/settings.json"), r#"{"security":{"auth":{"selectedType":"oauth-personal","enforcedType":"oauth-personal"}}}"#).unwrap();
    assert!(GeminiEnvironment::prepare(dir.path(), inputs(dir.path()), &[]).is_ok());
}
