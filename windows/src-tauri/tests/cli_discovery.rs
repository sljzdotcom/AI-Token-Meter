use ai_token_meter_windows::accounts::cli_account::CliProvider;
use ai_token_meter_windows::accounts::cli_discovery::{CliDiscovery, CliProbe, discover_cli};
use ai_token_meter_windows::accounts::installation::{
    InstallationDecision, installation_decision_for_discovery,
};
use ai_token_meter_windows::persistence::{CliRuntimeMode, ProviderCliSettings};
use ai_token_meter_windows::platform::windows::environment::DiscoveryInputs;

#[test]
fn unhealthy_existing_file_never_becomes_install_permission() {
    for delay in [
        std::time::Duration::ZERO,
        std::time::Duration::from_millis(15),
    ] {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("codex.exe"), "fixture").unwrap();
        let settings = ProviderCliSettings {
            mode: CliRuntimeMode::NativeWindows,
            ..Default::default()
        };
        let result = discover_cli(
            CliProvider::Codex,
            &settings,
            DiscoveryInputs {
                conventional_paths: vec![root.path().to_owned()],
                ..Default::default()
            },
            |_| {
                std::thread::sleep(delay);
                CliProbe::Unavailable
            },
            || panic!("Native must not inspect WSL"),
        );
        assert!(matches!(result, CliDiscovery::Unavailable));
        assert_eq!(
            installation_decision_for_discovery(&settings, &result),
            InstallationDecision::Unavailable
        );
    }
}

#[test]
fn confirmed_absence_and_incomplete_wsl_discovery_are_distinct() {
    let settings = ProviderCliSettings::default();
    let missing = discover_cli(
        CliProvider::Codex,
        &settings,
        Default::default(),
        |_| panic!("No candidate"),
        || Ok(None),
    );
    assert!(matches!(missing, CliDiscovery::Missing));
    assert_eq!(
        installation_decision_for_discovery(&settings, &missing),
        InstallationDecision::Launch
    );
    let failed = discover_cli(
        CliProvider::Codex,
        &settings,
        Default::default(),
        |_| CliProbe::Unavailable,
        || Err(()),
    );
    assert!(matches!(failed, CliDiscovery::Unavailable));
    assert_eq!(
        installation_decision_for_discovery(&settings, &failed),
        InstallationDecision::Unavailable
    );
}

#[test]
fn unlaunchable_existing_script_and_explicit_missing_path_are_unavailable() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("codex"), "#!/usr/bin/env node\n").unwrap();
    let inputs = DiscoveryInputs {
        conventional_paths: vec![root.path().to_owned()],
        ..Default::default()
    };
    assert!(matches!(
        discover_cli(
            CliProvider::Codex,
            &Default::default(),
            inputs,
            |_| CliProbe::Unavailable,
            || Ok(None)
        ),
        CliDiscovery::Unavailable
    ));
    let settings = ProviderCliSettings {
        custom_path: Some(
            root.path()
                .join("missing.exe")
                .to_string_lossy()
                .into_owned(),
        ),
        ..Default::default()
    };
    let result = discover_cli(
        CliProvider::Codex,
        &settings,
        Default::default(),
        |_| CliProbe::Unavailable,
        || Ok(None),
    );
    assert!(matches!(result, CliDiscovery::Unavailable));
    assert_eq!(
        installation_decision_for_discovery(&settings, &result),
        InstallationDecision::ManualRequired
    );
}

#[test]
fn reachable_wsl_with_confirmed_missing_cli_allows_auto_install_but_probe_failure_does_not() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("System32")).unwrap();
    std::fs::write(root.path().join("System32").join("wsl.exe"), "fixture").unwrap();
    let inputs = DiscoveryInputs {
        system_root: Some(root.path().to_owned()),
        ..Default::default()
    };
    for (probe, launch) in [(CliProbe::Missing, true), (CliProbe::Unavailable, false)] {
        let result = discover_cli(
            CliProvider::Codex,
            &Default::default(),
            inputs.clone(),
            |_| probe,
            || Ok(Some(b"Ubuntu\n".to_vec())),
        );
        assert_eq!(
            installation_decision_for_discovery(&Default::default(), &result)
                == InstallationDecision::Launch,
            launch
        );
    }
}
