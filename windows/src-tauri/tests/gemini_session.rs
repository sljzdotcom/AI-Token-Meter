use ai_token_meter_windows::collectors::CollectionError;
use ai_token_meter_windows::collectors::gemini_session::{
    GeminiTerminal, SessionTiming, collect_session,
};
use ai_token_meter_windows::platform::windows::process::CancellationToken;
use std::{collections::VecDeque, time::Duration};
const READY: &[u8] = include_bytes!("../../../contracts/gemini-cli/0.58.0/ready.ansi.txt");
const OPEN: &[u8] =
    include_bytes!("../../../contracts/gemini-cli/0.58.0/authenticated-model-open.ansi.txt");
struct ScriptedTerminal {
    output: VecDeque<Vec<u8>>,
    input: Vec<u8>,
    stopped: bool,
}
impl GeminiTerminal for ScriptedTerminal {
    fn read(&mut self) -> Result<Vec<u8>, CollectionError> {
        if self.input.ends_with(b"/quit\r") {
            return Err(CollectionError::Transport);
        }
        Ok(self.output.pop_front().unwrap_or_default())
    }
    fn send(&mut self, bytes: &[u8]) -> Result<(), CollectionError> {
        assert_eq!(bytes.len(), 1, "every command byte is sent separately");
        self.input.extend(bytes);
        if self.input == b"/model\r" {
            self.output.push_back(
                b"\x1b[2J\x1b[H"
                    .iter()
                    .copied()
                    .chain(OPEN.iter().copied())
                    .collect(),
            );
        }
        if bytes == b"\x1b" {
            self.output.push_back(
                b"\x1b[2J\x1b[H"
                    .iter()
                    .copied()
                    .chain(READY.iter().copied())
                    .collect(),
            );
        }
        Ok(())
    }
    fn stop(&mut self) -> Result<Vec<u8>, CollectionError> {
        self.stopped = true;
        Ok(Vec::new())
    }
}
fn timing() -> SessionTiming {
    SessionTiming {
        deadline: Duration::from_millis(500),
        stable: Duration::from_millis(3),
        key_delay: Duration::from_millis(1),
        poll: Duration::from_millis(1),
    }
}
#[test]
fn only_ready_main_input_receives_fixed_model_then_escape_quit() {
    let mut terminal = ScriptedTerminal {
        output: VecDeque::from([READY.to_vec()]),
        input: vec![],
        stopped: false,
    };
    let snapshot = collect_session(
        &mut terminal,
        "2026-09-08T09:00:00Z",
        &CancellationToken::new(),
        timing(),
    )
    .unwrap();
    assert_eq!(snapshot.used_ratio.unwrap().get(), 0.6);
    assert_eq!(terminal.input, b"/model\r\x1b/quit\r");
    assert!(terminal.stopped);
}
#[test]
fn authentication_unknown_ui_timeout_and_cancellation_stop_without_model_input() {
    for raw in [
        b"Enter the authorization code:".to_vec(),
        b"Select a theme".to_vec(),
        b"Unknown menu".to_vec(),
    ] {
        let mut terminal = ScriptedTerminal {
            output: VecDeque::from([raw]),
            input: vec![],
            stopped: false,
        };
        let result = collect_session(
            &mut terminal,
            "2026-09-08T09:00:00Z",
            &CancellationToken::new(),
            timing(),
        );
        assert!(result.is_err());
        assert!(terminal.input.is_empty());
        assert!(terminal.stopped);
    }
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let mut terminal = ScriptedTerminal {
        output: VecDeque::from([READY.to_vec()]),
        input: vec![],
        stopped: false,
    };
    assert!(matches!(
        collect_session(&mut terminal, "now", &cancellation, timing()),
        Err(CollectionError::Cancelled)
    ));
    assert!(terminal.stopped);
    assert!(terminal.input.is_empty());
}

fn frame(pro: u8) -> Vec<u8> {
    format!("\x1b[2J\x1b[HSelect Model\r\nModel usage\r\nPro ━━━ {pro}%\r\nFlash ━━━ 60%\r\n(Press Esc to close)\r\n╰────╯\r\n").into_bytes()
}
struct ConflictingTerminal {
    inner: ScriptedTerminal,
    chunk_size: usize,
    conflict_on_exit: bool,
}
impl GeminiTerminal for ConflictingTerminal {
    fn read(&mut self) -> Result<Vec<u8>, CollectionError> {
        self.inner.read()
    }
    fn send(&mut self, bytes: &[u8]) -> Result<(), CollectionError> {
        self.inner.send(bytes)?;
        if self.inner.input == b"/model\r" {
            self.inner.output.clear();
            let mut frames = frame(25);
            if !self.conflict_on_exit {
                frames.extend(frame(30));
            }
            self.inner
                .output
                .extend(frames.chunks(self.chunk_size).map(<[u8]>::to_vec));
        }
        Ok(())
    }
    fn stop(&mut self) -> Result<Vec<u8>, CollectionError> {
        self.inner.stop()?;
        Ok(if self.conflict_on_exit {
            frame(30)
        } else {
            Vec::new()
        })
    }
}
#[test]
fn complete_conflicting_frames_are_rejected_independent_of_read_chunking() {
    for chunk_size in [usize::MAX, frame(25).len(), 1] {
        let mut terminal = ConflictingTerminal {
            inner: ScriptedTerminal {
                output: VecDeque::from([READY.to_vec()]),
                input: vec![],
                stopped: false,
            },
            chunk_size,
            conflict_on_exit: false,
        };
        let result = collect_session(
            &mut terminal,
            "now",
            &CancellationToken::new(),
            fragmented_timing(),
        );
        assert!(
            matches!(result, Err(CollectionError::UnrecognizedOutput)),
            "chunk {chunk_size}: {result:?}"
        );
        assert!(terminal.inner.stopped);
    }
}

#[test]
fn final_conflicting_frame_drained_after_quit_cannot_return_fresh() {
    let mut terminal = ConflictingTerminal {
        inner: ScriptedTerminal {
            output: VecDeque::from([READY.to_vec()]),
            input: vec![],
            stopped: false,
        },
        chunk_size: usize::MAX,
        conflict_on_exit: true,
    };
    let result = collect_session(&mut terminal, "now", &CancellationToken::new(), timing());
    assert!(
        matches!(result, Err(CollectionError::UnrecognizedOutput)),
        "{result:?}"
    );
    assert_eq!(terminal.inner.input, b"/model\r\x1b/quit\r");
    assert!(terminal.inner.stopped);
}

struct ExitTerminal {
    inner: ScriptedTerminal,
    tail: Vec<u8>,
}
impl GeminiTerminal for ExitTerminal {
    fn read(&mut self) -> Result<Vec<u8>, CollectionError> {
        self.inner.read()
    }
    fn send(&mut self, bytes: &[u8]) -> Result<(), CollectionError> {
        self.inner.send(bytes)
    }
    fn stop(&mut self) -> Result<Vec<u8>, CollectionError> {
        self.inner.stop()?;
        Ok(self.tail.clone())
    }
}
#[test]
fn exit_clear_and_repeated_identical_frame_preserve_capture_but_auth_does_not() {
    let repeated = [b"\x1b[2J\x1b[H".as_slice(), OPEN, b"\x1b[2J\x1b[HGoodbye!"].concat();
    for (tail, auth) in [
        (b"\x1b[2J\x1b[HGoodbye!".to_vec(), false),
        (repeated, false),
        (b"\x1b[2J\x1b[HEnter the authorization code:".to_vec(), true),
    ] {
        let mut terminal = ExitTerminal {
            inner: ScriptedTerminal {
                output: VecDeque::from([READY.to_vec()]),
                input: vec![],
                stopped: false,
            },
            tail,
        };
        let result = collect_session(&mut terminal, "now", &CancellationToken::new(), timing());
        if auth {
            assert!(matches!(
                result,
                Err(CollectionError::AuthenticationRequired)
            ));
        } else {
            assert_eq!(result.unwrap().used_ratio.unwrap().get(), 0.6);
        }
        assert!(terminal.inner.stopped);
    }
}

struct ChunkedValidTerminal {
    inner: ScriptedTerminal,
    chunk_size: usize,
    model: Vec<u8>,
}
impl GeminiTerminal for ChunkedValidTerminal {
    fn read(&mut self) -> Result<Vec<u8>, CollectionError> {
        self.inner.read()
    }
    fn send(&mut self, bytes: &[u8]) -> Result<(), CollectionError> {
        self.inner.send(bytes)?;
        if self.inner.input == b"/model\r" {
            self.inner.output.clear();
            self.inner
                .output
                .extend(self.model.chunks(self.chunk_size).map(<[u8]>::to_vec));
        }
        Ok(())
    }
    fn stop(&mut self) -> Result<Vec<u8>, CollectionError> {
        self.inner.stop()
    }
}
#[test]
fn valid_quota_succeeds_whole_two_chunks_and_bytewise_including_split_escape() {
    for chunk_size in [usize::MAX, 1, frame(25).len() / 2] {
        let mut terminal = ChunkedValidTerminal {
            inner: ScriptedTerminal {
                output: VecDeque::from([READY.to_vec()]),
                input: vec![],
                stopped: false,
            },
            chunk_size,
            model: frame(25),
        };
        let result = collect_session(
            &mut terminal,
            "now",
            &CancellationToken::new(),
            fragmented_timing(),
        );
        assert_eq!(
            result.as_ref().map(|s| s.used_ratio.unwrap().get()),
            Ok(0.6),
            "chunk {chunk_size}: {result:?}"
        );
        assert_eq!(terminal.inner.input, b"/model\r\x1b/quit\r");
        assert!(terminal.inner.stopped);
    }
}
#[test]
fn final_auth_or_invalid_complete_quota_cannot_be_erased_by_goodbye() {
    for (bad, expected) in [
        (
            b"\x1b[2J\x1b[HEnter the authorization code:".to_vec(),
            CollectionError::AuthenticationRequired,
        ),
        (frame(110), CollectionError::UnrecognizedOutput),
    ] {
        let tail = [bad.as_slice(), b"\x1b[2J\x1b[HGoodbye!"].concat();
        let mut terminal = ExitTerminal {
            inner: ScriptedTerminal {
                output: VecDeque::from([READY.to_vec()]),
                input: vec![],
                stopped: false,
            },
            tail,
        };
        let result = collect_session(&mut terminal, "now", &CancellationToken::new(), timing());
        assert_eq!(result.err(), Some(expected));
        assert!(terminal.inner.stopped);
    }
}

fn fragmented_timing() -> SessionTiming {
    SessionTiming {
        deadline: Duration::from_secs(2),
        stable: Duration::ZERO,
        key_delay: Duration::ZERO,
        poll: Duration::ZERO,
    }
}

#[test]
fn authentication_and_invalid_quota_history_fail_during_live_reads_too() {
    for (bad, expected) in [
        (
            b"\x1b[2J\x1b[HEnter the authorization code:".to_vec(),
            CollectionError::AuthenticationRequired,
        ),
        (frame(110), CollectionError::UnrecognizedOutput),
    ] {
        for chunk_size in [usize::MAX, 1] {
            let model = [frame(25), bad.clone(), b"\x1b[2J\x1b[HGoodbye!".to_vec()].concat();
            let mut terminal = ChunkedValidTerminal {
                inner: ScriptedTerminal {
                    output: VecDeque::from([READY.to_vec()]),
                    input: vec![],
                    stopped: false,
                },
                chunk_size,
                model,
            };
            let result = collect_session(
                &mut terminal,
                "now",
                &CancellationToken::new(),
                fragmented_timing(),
            );
            assert_eq!(result.err(), Some(expected));
            assert!(terminal.inner.stopped);
        }
    }
}
