#![cfg(windows)]
use ai_token_meter_windows::{
    collectors::{
        CollectionError,
        gemini::{parse_terminal_quota, terminal_state},
        gemini_environment::GeminiEnvironment,
        gemini_runtime::NativeTerminal,
        gemini_session::{GeminiTerminal, SessionTiming, collect_session},
    },
    platform::windows::{
        executable_locator::{CandidateOrigin, ExecutableCandidate, RuntimeSource},
        process::CancellationToken,
    },
};
use std::{
    fmt::Write as _,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

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
        let stage = dir.path().join("stage");
        environment.variables.extend([
            ("AI_METER_TEST_SCENARIO".into(), scenario.into()),
            ("AI_METER_TEST_PID".into(), pid.clone().into_os_string()),
            ("AI_METER_TEST_INPUT".into(), input.clone().into_os_string()),
            (
                "AI_METER_TEST_RAW_INPUT".into(),
                raw_input.clone().into_os_string(),
            ),
            ("AI_METER_TEST_STAGE".into(), stage.clone().into_os_string()),
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
        let mut terminal =
            RecordingTerminal::new(NativeTerminal::open(&candidate, &environment).unwrap());
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
        let diagnostic = format!(
            "stage={:?}, raw={}, normalized={}, {}",
            std::fs::read_to_string(&stage)
                .unwrap_or_else(|error| format!("unavailable:{:?}", error.kind())),
            bounded_bytes(&raw_input),
            bounded_bytes(&input),
            terminal.diagnostic()
        );
        match scenario {
            "success" => {
                let snapshot = result.unwrap_or_else(|error| {
                    panic!("success scenario returned {error:?}; {diagnostic}")
                });
                assert_eq!(snapshot.used_ratio.unwrap().get(), 0.6, "{diagnostic}");
                assert_eq!(std::fs::read(&input).unwrap(), b"/model\r\x1b/quit\r");
            }
            "exit-conflict" => assert!(
                matches!(result, Err(CollectionError::UnrecognizedOutput)),
                "{diagnostic}"
            ),
            "cancel" => assert!(
                matches!(result, Err(CollectionError::Cancelled)),
                "{diagnostic}"
            ),
            _ => assert!(
                matches!(result, Err(CollectionError::TimedOut)),
                "{diagnostic}"
            ),
        }
        let raw_input = std::fs::read_to_string(raw_input).unwrap();
        let normalized_input = std::fs::read_to_string(&input).unwrap();
        assert!(
            contains_primary_device_attributes_reply(&raw_input),
            "ConPTY primary device-attributes reply was not observed in {scenario}"
        );
        assert!(
            !normalized_input.contains("\x1b[?") && !normalized_input.contains("\x1b[1;1R"),
            "fixture did not isolate recognized terminal protocol input in {scenario}"
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

fn bounded_bytes(path: &Path) -> String {
    match std::fs::read(path) {
        Ok(bytes) => format!(
            "len={} bytes={:?}",
            bytes.len(),
            &bytes[..bytes.len().min(128)]
        ),
        Err(error) => format!("unavailable:{:?}", error.kind()),
    }
}

fn contains_primary_device_attributes_reply(input: &str) -> bool {
    input.split("\x1b[?").skip(1).any(|tail| {
        tail.split_once('c').is_some_and(|(parameters, _)| {
            !parameters.is_empty()
                && parameters.split(';').all(|parameter| {
                    !parameter.is_empty() && parameter.bytes().all(|byte| byte.is_ascii_digit())
                })
        })
    })
}

struct RecordingTerminal<T> {
    inner: T,
    output: Vec<u8>,
    read_ends: Vec<usize>,
    stopped_at: Option<usize>,
}

impl<T> RecordingTerminal<T> {
    fn new(inner: T) -> Self {
        Self {
            inner,
            output: Vec::new(),
            read_ends: Vec::new(),
            stopped_at: None,
        }
    }

    fn diagnostic(&self) -> String {
        let stopped_at = self.stopped_at.unwrap_or(self.output.len());
        let output = &self.output[..stopped_at];
        let reads = self
            .read_ends
            .iter()
            .copied()
            .filter(|end| *end <= stopped_at)
            .map(|end| {
                let prefix = &output[..end];
                match valid_utf8_prefix(prefix) {
                    Ok(text) => format!(
                        "{end}:{:?}/{:?}",
                        terminal_state(text),
                        parse_terminal_quota(text, "diagnostic")
                            .map(|snapshot| snapshot.used_ratio.map(|ratio| ratio.get()))
                    ),
                    Err(()) => format!("{end}:invalid-utf8"),
                }
            })
            .collect::<Vec<_>>();
        format!(
            "output={}, output_hex={}, reads={reads:?}",
            bounded_slice(output),
            bounded_hex(output)
        )
    }
}

impl<T: GeminiTerminal> GeminiTerminal for RecordingTerminal<T> {
    fn read(&mut self) -> Result<Vec<u8>, CollectionError> {
        let bytes = self.inner.read()?;
        if !bytes.is_empty() {
            self.output.extend_from_slice(&bytes);
            self.read_ends.push(self.output.len());
        }
        Ok(bytes)
    }

    fn send(&mut self, bytes: &[u8]) -> Result<(), CollectionError> {
        self.inner.send(bytes)
    }

    fn stop(&mut self) -> Result<Vec<u8>, CollectionError> {
        self.stopped_at = Some(self.output.len());
        let tail = self.inner.stop()?;
        self.output.extend_from_slice(&tail);
        Ok(tail)
    }
}

fn valid_utf8_prefix(bytes: &[u8]) -> Result<&str, ()> {
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok(text),
        Err(error) if error.error_len().is_none() => {
            std::str::from_utf8(&bytes[..error.valid_up_to()]).map_err(|_| ())
        }
        Err(_) => Err(()),
    }
}

fn bounded_slice(bytes: &[u8]) -> String {
    let head_length = bytes.len().min(96);
    let tail_start = bytes.len().saturating_sub(256).max(head_length);
    format!(
        "len={} head={:?} tail={:?}",
        bytes.len(),
        &bytes[..head_length],
        &bytes[tail_start..]
    )
}

fn bounded_hex(bytes: &[u8]) -> String {
    const LIMIT: usize = 16 * 1024;
    if bytes.len() > LIMIT {
        return format!("omitted(len={})", bytes.len());
    }
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut encoded, "{byte:02x}").unwrap();
    }
    encoded
}
