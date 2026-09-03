use std::fs;
use std::path::{Path, PathBuf};

use ai_token_meter_windows::accounts::cli_account::CliProvider;
use ai_token_meter_windows::platform::windows::environment::DiscoveryInputs;
use ai_token_meter_windows::platform::windows::executable_locator::{
    CandidateOrigin, DiscoveryBudget, ExecutableLocator, RuntimeSource,
};
use ai_token_meter_windows::platform::windows::wsl::{
    build_wsl_invocation, build_wsl_list_invocation, decode_distribution_list, wsl_profile_path,
};
use tempfile::tempdir;

#[test]
fn custom_path_wins_over_every_automatic_location() {
    let fixture = LocatorFixture::new();
    let custom = fixture.executable("custom", "codex.exe");
    let process = fixture.executable("process", "codex.exe");
    let inputs = DiscoveryInputs {
        custom_path: Some(custom.clone()),
        process_paths: vec![process],
        ..fixture.inputs()
    };

    let candidate = ExecutableLocator::new(inputs)
        .locate(CliProvider::Codex, |_| true)
        .expect("candidate");

    assert_eq!(candidate.executable, canonical(&custom));
    assert_eq!(candidate.origin, CandidateOrigin::Custom);
}

#[test]
fn an_explicit_custom_path_is_canonicalized_and_must_pass_its_own_health_check() {
    let fixture = LocatorFixture::new();
    let custom = fixture.executable("custom/child/..", "codex.exe");
    let locator = ExecutableLocator::new(fixture.inputs());

    let accepted = locator
        .validate_custom(CliProvider::Codex, &custom, |_| true)
        .expect("healthy custom candidate");
    assert_eq!(accepted.executable, canonical(&custom));
    assert_eq!(accepted.origin, CandidateOrigin::Custom);
    assert!(
        locator
            .validate_custom(CliProvider::Codex, &custom, |_| false)
            .is_none()
    );
    assert!(
        locator
            .validate_custom(CliProvider::Claude, &custom, |_| true)
            .is_none()
    );
}

#[test]
fn automatic_locations_follow_the_documented_priority() {
    let fixture = LocatorFixture::new();
    let user_registry = fixture.executable("user-registry", "claude.exe");
    let system_registry = fixture.executable("system-registry", "claude.exe");
    let conventional = fixture.executable("conventional", "claude.exe");
    let inputs = DiscoveryInputs {
        user_registry_paths: vec![parent(&user_registry)],
        system_registry_paths: vec![parent(&system_registry)],
        conventional_paths: vec![parent(&conventional)],
        ..fixture.inputs()
    };

    let candidate = ExecutableLocator::new(inputs)
        .locate(CliProvider::Claude, |_| true)
        .expect("candidate");

    assert_eq!(candidate.executable, canonical(&user_registry));
    assert_eq!(candidate.origin, CandidateOrigin::UserRegistryPath);
}

#[test]
fn unhealthy_directory_wrong_name_and_symlink_loop_are_rejected() {
    let fixture = LocatorFixture::new();
    let directory = fixture.directory("codex.exe");
    let wrong_name = fixture.executable("wrong", "other.exe");
    let unhealthy = fixture.executable("unhealthy", "codex.exe");
    let loop_path = fixture.root().join("loop").join("codex.exe");
    fs::create_dir_all(loop_path.parent().expect("loop parent")).expect("loop directory");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&loop_path, &loop_path).expect("symlink loop");
    let inputs = DiscoveryInputs {
        custom_path: Some(directory),
        process_paths: vec![parent(&wrong_name), parent(&unhealthy), parent(&loop_path)],
        ..fixture.inputs()
    };

    let candidate = ExecutableLocator::new(inputs).locate(CliProvider::Codex, |candidate| {
        candidate.executable != canonical(&unhealthy)
    });

    assert!(candidate.is_none());
}

#[test]
fn env_node_script_requires_an_explicit_node_launcher() {
    let fixture = LocatorFixture::new();
    let script = fixture.file(
        "nvm/bin/codex",
        "#!/usr/bin/env node\nconsole.log('codex')\n",
    );
    let inputs_without_node = DiscoveryInputs {
        custom_path: Some(script.clone()),
        ..fixture.inputs()
    };
    assert!(
        ExecutableLocator::new(inputs_without_node)
            .locate(CliProvider::Codex, |_| true)
            .is_none()
    );

    let node = fixture.file("nvm/bin/node.exe", "node fixture");
    let inputs_with_node = DiscoveryInputs {
        custom_path: Some(script.clone()),
        ..fixture.inputs()
    };
    let candidate = ExecutableLocator::new(inputs_with_node)
        .locate(CliProvider::Codex, |_| true)
        .expect("node-backed candidate");

    assert_eq!(candidate.executable, canonical(&script));
    assert_eq!(candidate.launcher, Some(canonical(&node)));
}

#[test]
fn cmd_wrapper_requires_the_system_command_interpreter() {
    let fixture = LocatorFixture::new();
    let wrapper = fixture.executable("npm", "claude.cmd");
    let system_root = fixture.root().join("Windows");
    let cmd = fixture.file("Windows/System32/cmd.exe", "cmd fixture");
    let inputs = DiscoveryInputs {
        custom_path: Some(wrapper.clone()),
        system_root: Some(system_root),
        ..fixture.inputs()
    };

    let candidate = ExecutableLocator::new(inputs)
        .locate(CliProvider::Claude, |_| true)
        .expect("cmd-backed candidate");

    assert_eq!(candidate.executable, canonical(&wrapper));
    assert_eq!(candidate.launcher, Some(canonical(&cmd)));
}

#[test]
fn wsl_utf16_output_is_normalized_and_arguments_never_use_a_shell_string() {
    let utf16 = "Ubuntu\r\nDebian Test\r\n"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();

    let distributions = decode_distribution_list(&utf16);
    let invocation = build_wsl_invocation(
        PathBuf::from(r"C:\Windows\System32\wsl.exe"),
        &distributions[1],
        CliProvider::Codex,
        &["login".to_owned()],
    );

    assert_eq!(distributions, ["Ubuntu", "Debian Test"]);
    assert_eq!(
        invocation.arguments,
        ["--distribution", "Debian Test", "--exec", "codex", "login"]
    );
}

#[test]
fn wsl_is_only_considered_after_native_candidates_and_uses_structured_list_arguments() {
    let fixture = LocatorFixture::new();
    let system_root = fixture.root().join("Windows");
    let wsl = fixture.file("Windows/System32/wsl.exe", "wsl fixture");
    let native = fixture.executable("native", "codex.exe");
    let distributions = "* Ubuntu\r\nDebian Test\r\n"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    let inputs = DiscoveryInputs {
        process_paths: vec![parent(&native)],
        system_root: Some(system_root.clone()),
        ..fixture.inputs()
    };

    let locator = ExecutableLocator::new(inputs);
    let list_invocation = build_wsl_list_invocation(&system_root).expect("WSL list invocation");
    assert_eq!(list_invocation.executable, canonical(&wsl));
    assert_eq!(list_invocation.arguments, ["--list", "--quiet"]);

    let selected = locator
        .locate_with_wsl_output(CliProvider::Codex, Some(&distributions), |_| true)
        .expect("native candidate");
    assert_eq!(selected.origin, CandidateOrigin::ProcessPath);

    let wsl_only = ExecutableLocator::new(DiscoveryInputs {
        system_root: Some(system_root),
        ..fixture.inputs()
    })
    .locate_with_wsl_output(CliProvider::Codex, Some(&distributions), |_| true)
    .expect("WSL candidate");
    assert_eq!(wsl_only.origin, CandidateOrigin::Wsl);
    assert_eq!(
        wsl_only.source,
        RuntimeSource::Wsl {
            distribution: "Ubuntu".to_owned()
        }
    );
}

#[test]
fn an_explicit_wsl_distribution_never_falls_back_to_another_distribution() {
    let fixture = LocatorFixture::new();
    let system_root = fixture.root().join("Windows");
    fixture.file("Windows/System32/wsl.exe", "wsl fixture");
    let distributions = "Ubuntu\r\nDebian Test\r\n";
    let locator = ExecutableLocator::new(DiscoveryInputs {
        system_root: Some(system_root),
        ..fixture.inputs()
    });

    let selected = locator
        .locate_wsl_with_output(
            CliProvider::Claude,
            distributions.as_bytes(),
            Some("Debian Test"),
            |_| true,
        )
        .expect("selected WSL candidate");
    assert_eq!(
        selected.source,
        RuntimeSource::Wsl {
            distribution: "Debian Test".to_owned(),
        }
    );
    assert!(
        locator
            .locate_wsl_with_output(
                CliProvider::Claude,
                distributions.as_bytes(),
                Some("Missing"),
                |_| true,
            )
            .is_none()
    );
}

#[test]
fn auxiliary_fixture_documents_the_stable_candidate_order() {
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../contracts/fixtures/auxiliary/windows-cli-locations.json"
    ))
    .expect("fixture JSON");

    assert_eq!(
        value["candidateOrder"],
        serde_json::json!([
            "custom",
            "processPath",
            "userRegistryPath",
            "systemRegistryPath",
            "conventional",
            "desktopApplication",
            "wsl"
        ])
    );
}

struct LocatorFixture {
    directory: tempfile::TempDir,
}

impl LocatorFixture {
    fn new() -> Self {
        Self {
            directory: tempdir().expect("temporary directory"),
        }
    }

    fn root(&self) -> &Path {
        self.directory.path()
    }

    fn inputs(&self) -> DiscoveryInputs {
        DiscoveryInputs::default()
    }

    fn directory(&self, relative: &str) -> PathBuf {
        let path = self.root().join(relative);
        fs::create_dir_all(&path).expect("fixture directory");
        path
    }

    fn executable(&self, directory: &str, name: &str) -> PathBuf {
        self.file(&format!("{directory}/{name}"), "fixture executable")
    }

    fn file(&self, relative: &str, contents: &str) -> PathBuf {
        let path = self.root().join(relative);
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture parent");
        fs::write(&path, contents).expect("fixture file");
        path
    }
}

fn parent(path: &Path) -> PathBuf {
    path.parent().expect("parent").to_owned()
}

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().expect("canonical fixture")
}
#[test]
fn only_native_windows_runtime_may_read_windows_profile_activity() {
    assert!(RuntimeSource::NativeWindows.may_read_windows_profile());
    assert!(
        !RuntimeSource::Wsl {
            distribution: "Ubuntu".to_owned(),
        }
        .may_read_windows_profile()
    );
}

#[test]
fn discovery_budget_enforces_one_deadline_and_a_total_process_limit() {
    let mut budget = DiscoveryBudget::new(std::time::Duration::from_secs(10), 2);
    assert_eq!(
        budget.next_process_timeout(std::time::Duration::from_secs(4)),
        Some(std::time::Duration::from_secs(4))
    );
    assert!(
        budget
            .next_process_timeout(std::time::Duration::from_secs(4))
            .is_some_and(|timeout| timeout <= std::time::Duration::from_secs(4))
    );
    assert_eq!(
        budget.next_process_timeout(std::time::Duration::from_secs(4)),
        None
    );

    let mut expired = DiscoveryBudget::new(std::time::Duration::ZERO, 4);
    assert_eq!(
        expired.next_process_timeout(std::time::Duration::from_secs(4)),
        None
    );
}

#[test]
fn wsl_profile_paths_are_scoped_to_one_valid_distribution_and_absolute_home() {
    assert_eq!(
        wsl_profile_path("Ubuntu-24.04", "/home/miller"),
        Some(PathBuf::from(r"\\wsl.localhost\Ubuntu-24.04\home\miller"))
    );
    assert_eq!(wsl_profile_path("../Ubuntu", "/home/miller"), None);
    assert_eq!(wsl_profile_path("Ubuntu", "/home/../root"), None);
    assert_eq!(wsl_profile_path("Ubuntu", "relative/home"), None);
}
