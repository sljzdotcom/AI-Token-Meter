use ai_token_meter_windows::collectors::CollectionError;
use ai_token_meter_windows::collectors::gemini_session::{
    GeminiTerminal, SessionTiming, collect_session,
};
use ai_token_meter_windows::platform::windows::process::CancellationToken;
use std::{collections::VecDeque, time::Duration};

const READY: &[u8] = include_bytes!("../../../contracts/gemini-cli/0.58.0/ready.ansi.txt");
const UNTRUSTED_OPEN: &[u8] = include_bytes!(
    "../../../contracts/gemini-cli/0.58.0/authenticated-untrusted-model-open.ansi.txt"
);

struct ChunkedTranscriptTerminal {
    output: VecDeque<Vec<u8>>,
    input: Vec<u8>,
    stopped: bool,
    chunk_size: usize,
}

impl GeminiTerminal for ChunkedTranscriptTerminal {
    fn read(&mut self) -> Result<Vec<u8>, CollectionError> {
        Ok(self.output.pop_front().unwrap_or_default())
    }

    fn send(&mut self, bytes: &[u8]) -> Result<(), CollectionError> {
        assert_eq!(bytes.len(), 1, "every command byte is sent separately");
        self.input.extend(bytes);
        if self.input == b"/model\r" {
            self.output
                .extend(UNTRUSTED_OPEN.chunks(self.chunk_size).map(<[u8]>::to_vec));
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

#[test]
fn real_untrusted_transcript_succeeds_at_conpty_sized_read_boundaries() {
    for chunk_size in [4096, 137] {
        let mut terminal = ChunkedTranscriptTerminal {
            output: VecDeque::from([READY.to_vec()]),
            input: Vec::new(),
            stopped: false,
            chunk_size,
        };
        let result = collect_session(
            &mut terminal,
            "now",
            &CancellationToken::new(),
            SessionTiming {
                deadline: Duration::from_secs(2),
                stable: Duration::ZERO,
                key_delay: Duration::ZERO,
                poll: Duration::ZERO,
            },
        );
        assert_eq!(
            result
                .as_ref()
                .map(|snapshot| snapshot.used_ratio.unwrap().get()),
            Ok(0.6),
            "chunk {chunk_size}: {result:?}"
        );
        assert_eq!(terminal.input, b"/model\r\x1b/quit\r");
        assert!(terminal.stopped);
    }
}
