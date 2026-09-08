use ai_token_meter_windows::accounts::cli_account::CliProvider;
use ai_token_meter_windows::accounts::cli_discovery::{
    CliDiscovery, CliProbe, CliWslList, discover_cli,
};
use ai_token_meter_windows::accounts::installation::{
    InstallationDecision, installation_decision_for_discovery,
};
use ai_token_meter_windows::persistence::{CliRuntimeMode, ProviderCliSettings};
use ai_token_meter_windows::platform::windows::environment::DiscoveryInputs;

#[test]
fn automatic_discovery_keeps_healthy_wsl_fallback_after_bad_native_candidate() {
    use ai_token_meter_windows::platform::windows::executable_locator::{
        ExecutableLocator, RuntimeSource,
    };
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("System32")).unwrap();
    std::fs::write(root.path().join("System32/wsl.exe"), "fixture").unwrap();
    std::fs::write(root.path().join("codex.exe"), "unhealthy").unwrap();
    let inputs = DiscoveryInputs {
        system_root: Some(root.path().to_owned()),
        conventional_paths: vec![root.path().to_owned()],
        ..Default::default()
    };
    let expected = ExecutableLocator::new(inputs.clone())
        .locate_with_wsl_output(CliProvider::Codex, Some(b"Ubuntu\n"), |candidate| {
            matches!(candidate.source, RuntimeSource::Wsl { .. })
        })
        .unwrap();
    let result = discover_cli(
        CliProvider::Codex,
        &Default::default(),
        inputs.clone(),
        |candidate| {
            if matches!(candidate.source, RuntimeSource::Wsl { .. }) {
                CliProbe::Healthy
            } else {
                CliProbe::Unavailable
            }
        },
        || CliWslList::Output(b"Ubuntu\n".to_vec()),
    );
    let CliDiscovery::Found(found) = result else {
        panic!("Healthy WSL must remain discoverable")
    };
    assert_eq!(found, expected);
    assert_eq!(
        found.source,
        RuntimeSource::Wsl {
            distribution: "Ubuntu".to_owned()
        }
    );
    let uncertain = discover_cli(
        CliProvider::Codex,
        &Default::default(),
        inputs,
        |_| CliProbe::Missing,
        || CliWslList::Missing,
    );
    assert!(matches!(uncertain, CliDiscovery::Unavailable));
}

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
        || CliWslList::Missing,
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
        || CliWslList::Unavailable,
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
            || CliWslList::Missing
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
        || CliWslList::Missing,
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
            || CliWslList::Output(b"Ubuntu\n".to_vec()),
        );
        assert_eq!(
            installation_decision_for_discovery(&Default::default(), &result)
                == InstallationDecision::Launch,
            launch
        );
    }
}

#[test]
fn cancelled_candidate_probe_is_not_folded_into_unavailable_or_missing() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("codex.exe"), "fixture").unwrap();
    let result = discover_cli(
        CliProvider::Codex,
        &ProviderCliSettings {
            mode: CliRuntimeMode::NativeWindows,
            ..Default::default()
        },
        DiscoveryInputs {
            conventional_paths: vec![root.path().to_owned()],
            ..Default::default()
        },
        |_| CliProbe::Cancelled,
        || panic!("native-only discovery must not inspect WSL"),
    );
    assert!(matches!(result, CliDiscovery::Cancelled));
}

#[test]
fn cancelled_wsl_listing_is_not_folded_into_unavailable() {
    let result = discover_cli(
        CliProvider::Codex,
        &ProviderCliSettings {
            mode: CliRuntimeMode::Wsl,
            ..Default::default()
        },
        DiscoveryInputs::default(),
        |_| panic!("cancelled listing must not probe candidates"),
        || CliWslList::Cancelled,
    );
    assert!(matches!(result, CliDiscovery::Cancelled));
}
