#![cfg(windows)]
use ai_token_meter_windows::{
    collectors::{
        CollectionError,
        gemini_environment::GeminiEnvironment,
        gemini_runtime::NativeTerminal,
        gemini_session::{SessionTiming, collect_session},
    },
    platform::windows::{
        executable_locator::{CandidateOrigin, ExecutableCandidate, RuntimeSource},
        process::CancellationToken,
    },
};
use std::{path::PathBuf, sync::Arc, time::Duration};

#[test]
fn actual_conpty_fixed_input_success_timeout_and_cancel_reap_the_child() {
    for scenario in ["success", "hang", "cancel", "exit-conflict"] {
        let dir = tempfile::tempdir().unwrap();
        let mut environment = GeminiEnvironment::prepare(
            dir.path(),
            vec![
                ("USERPROFILE".into(), dir.path().into()),
                ("SystemRoot".into(), std::env::var_os("SystemRoot").unwrap()),
            ],
            &[],
        )
        .unwrap();
        let fixture =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gemini-terminal.cjs");
        let input = dir.path().join("input");
        let raw_input = dir.path().join("raw-input");
        let pid = dir.path().join("pid");
        environment.variables.extend([
            ("AI_METER_TEST_SCENARIO".into(), scenario.into()),
            ("AI_METER_TEST_PID".into(), pid.clone().into_os_string()),
            ("AI_METER_TEST_INPUT".into(), input.clone().into_os_string()),
            (
                "AI_METER_TEST_RAW_INPUT".into(),
                raw_input.clone().into_os_string(),
            ),
            (
                "AI_METER_TEST_FIXTURES".into(),
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../contracts/gemini-cli/0.58.0")
                    .into_os_string(),
            ),
        ]);
        let node = std::env::split_paths(&std::env::var_os("PATH").unwrap())
            .map(|p| p.join("node.exe"))
            .find_map(|p| p.canonicalize().ok())
            .expect("Windows CI Node");
        let candidate = ExecutableCandidate {
            selected_path: fixture.clone(),
            executable: fixture,
            launcher: Some(node),
            source: RuntimeSource::NativeWindows,
            origin: CandidateOrigin::Custom,
        };
        let cancellation = Arc::new(CancellationToken::new());
        let trigger = Arc::clone(&cancellation);
        let cancel = if scenario == "cancel" {
            Some(std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(600));
                trigger.cancel();
            }))
        } else {
            None
        };
        let mut terminal = NativeTerminal::open(&candidate, &environment).unwrap();
        let result = collect_session(
            &mut terminal,
            "2026-09-08T08:47:00Z",
            &cancellation,
            SessionTiming {
                deadline: Duration::from_secs(
                    if ["success", "exit-conflict"].contains(&scenario) {
                        8
                    } else {
                        2
                    },
                ),
                stable: Duration::from_millis(80),
                key_delay: Duration::from_millis(100),
                poll: Duration::from_millis(10),
            },
        );
        if let Some(cancel) = cancel {
            cancel.join().unwrap();
        }
        match scenario {
            "success" => {
                assert_eq!(result.unwrap().used_ratio.unwrap().get(), 0.6);
                assert_eq!(std::fs::read(&input).unwrap(), b"/model\r\x1b/quit\r");
            }
            "exit-conflict" => assert!(matches!(result, Err(CollectionError::UnrecognizedOutput))),
            "cancel" => assert!(matches!(result, Err(CollectionError::Cancelled))),
            _ => assert!(matches!(result, Err(CollectionError::TimedOut))),
        }
        let raw_input = std::fs::read_to_string(raw_input).unwrap();
        assert!(
            raw_input.contains("\x1b[1;1R"),
            "ConPTY cursor-position handshake was not observed in {scenario}"
        );
        assert_eq!(
            raw_input.replace("\x1b[1;1R", ""),
            std::fs::read_to_string(&input).unwrap(),
            "fixture did not isolate terminal protocol input in {scenario}"
        );
        let pid: u32 = std::fs::read_to_string(&pid).unwrap().parse().unwrap();
        unsafe {
            use windows_sys::Win32::{
                Foundation::{CloseHandle, WAIT_OBJECT_0},
                System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
            };
            let process = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
            if !process.is_null() {
                assert_eq!(
                    WaitForSingleObject(process, 0),
                    WAIT_OBJECT_0,
                    "owned child still running"
                );
                CloseHandle(process);
            }
        }
    }
}
