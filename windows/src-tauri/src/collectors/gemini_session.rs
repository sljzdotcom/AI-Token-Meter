use super::{
    CollectionError,
    gemini::{TerminalState, parse_visible_quota, visible_state},
    gemini_terminal::observe_frames,
};
use crate::{domain::UsageSnapshot, platform::windows::process::CancellationToken};
use std::time::{Duration, Instant};

pub trait GeminiTerminal {
    fn read(&mut self) -> Result<Vec<u8>, CollectionError>;
    fn send(&mut self, bytes: &[u8]) -> Result<(), CollectionError>;
    /// Observes process exit or forcibly terminates the owned job before returning.
    fn stop(&mut self) -> Result<Vec<u8>, CollectionError>;
}
pub struct SessionTiming {
    pub deadline: Duration,
    pub stable: Duration,
    pub key_delay: Duration,
    pub poll: Duration,
}
impl Default for SessionTiming {
    fn default() -> Self {
        Self {
            deadline: Duration::from_secs(25),
            stable: Duration::from_millis(300),
            key_delay: Duration::from_millis(100),
            poll: Duration::from_millis(20),
        }
    }
}

pub fn collect_session(
    terminal: &mut impl GeminiTerminal,
    fetched_at: &str,
    cancellation: &CancellationToken,
    timing: SessionTiming,
) -> Result<UsageSnapshot, CollectionError> {
    let mut raw = Vec::new();
    let result = run(terminal, fetched_at, cancellation, timing, &mut raw);
    let cleanup = terminal.stop();
    match result {
        Ok(snapshot) => {
            let tail = cleanup?;
            if raw.len() + tail.len() > 512 * 1024 {
                return Err(CollectionError::UnrecognizedOutput);
            }
            raw.extend(tail);
            let text =
                std::str::from_utf8(&raw).map_err(|_| CollectionError::UnrecognizedOutput)?;
            validate_frames(text, fetched_at)?;
            Ok(snapshot)
        }
        Err(error) => {
            let _ = cleanup;
            Err(error)
        }
    }
}
fn run(
    terminal: &mut impl GeminiTerminal,
    fetched_at: &str,
    cancellation: &CancellationToken,
    timing: SessionTiming,
    raw: &mut Vec<u8>,
) -> Result<UsageSnapshot, CollectionError> {
    let deadline = Instant::now() + timing.deadline;
    let mut previous = String::new();
    let mut changed = Instant::now();
    let mut phase = 0;
    let mut typing: Option<(&[u8], usize)> = None;
    let mut next_key = Instant::now();
    let mut quota = None;
    loop {
        if cancellation.is_cancelled() {
            return Err(CollectionError::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(CollectionError::TimedOut);
        }
        if phase == 3 {
            return quota.unwrap_or(Err(CollectionError::QuotaUnavailable));
        }
        let bytes = terminal.read()?;
        if raw.len() + bytes.len() > 512 * 1024 {
            return Err(CollectionError::UnrecognizedOutput);
        }
        let has_output = !bytes.is_empty();
        raw.extend(bytes);
        let text = match std::str::from_utf8(raw) {
            Ok(s) => s,
            Err(error) if error.error_len().is_none() => {
                std::str::from_utf8(&raw[..error.valid_up_to()])
                    .map_err(|_| CollectionError::UnrecognizedOutput)?
            }
            Err(_) => return Err(CollectionError::UnrecognizedOutput),
        };
        let visible = if has_output {
            validate_frames(text, fetched_at)?
        } else {
            previous.clone()
        };
        if visible != previous {
            changed = Instant::now();
            previous = visible;
        }
        let state = visible_state(&previous)?;
        if let Some((keys, index)) = typing.as_mut() {
            if Instant::now() >= next_key {
                terminal.send(&keys[*index..*index + 1])?;
                *index += 1;
                next_key = Instant::now() + timing.key_delay;
                if *index == keys.len() {
                    typing = None;
                    phase += 1;
                    changed = Instant::now();
                }
            }
        } else if changed.elapsed() >= timing.stable {
            match phase {
                0 if state == TerminalState::Ready => {
                    typing = Some((b"/model\r", 0));
                    next_key = Instant::now();
                }
                1 if state == TerminalState::Model && complete_dialog(&previous) => {
                    // Only the current complete dialog is eligible; no accumulation of old frames.
                    let parsed = parse_visible_quota(&previous, fetched_at);
                    if matches!(parsed, Err(CollectionError::UnrecognizedOutput)) {
                        return parsed;
                    }
                    quota = Some(parsed);
                    terminal.send(b"\x1b")?;
                    phase = 2;
                    changed = Instant::now();
                }
                2 if state == TerminalState::Ready => {
                    typing = Some((b"/quit\r", 0));
                    next_key = Instant::now();
                }
                3 => return quota.unwrap_or(Err(CollectionError::QuotaUnavailable)),
                _ => {}
            }
        }
        std::thread::sleep(timing.poll);
    }
}

fn validate_frames(text: &str, fetched_at: &str) -> Result<String, CollectionError> {
    let mut observed = None;
    observe_frames(text, |visible, dialog_bottom| {
        // Validate blocking UI before any erase, including the final shutdown drain.
        visible_state(visible)?;
        if !dialog_bottom {
            return Ok(());
        }
        let parsed = parse_visible_quota(visible, fetched_at);
        if observed.is_some()
            && complete_dialog(visible)
            && matches!(parsed, Err(CollectionError::QuotaUnavailable))
        {
            return Err(CollectionError::QuotaUnavailable);
        }
        if matches!(parsed, Err(CollectionError::UnrecognizedOutput))
            && visible
                .rsplit_once("Select Model")
                .is_some_and(|(_, dialog)| {
                    dialog.contains("Model usage") && dialog.contains("(Press Esc to close)")
                })
        {
            return Err(CollectionError::UnrecognizedOutput);
        }
        if let Ok(snapshot) = parsed {
            if observed
                .as_ref()
                .is_some_and(|previous| previous != &snapshot.gemini_quota_metrics)
            {
                return Err(CollectionError::UnrecognizedOutput);
            }
            observed = Some(snapshot.gemini_quota_metrics);
        }
        Ok(())
    })
}

fn complete_dialog(visible: &str) -> bool {
    visible
        .rsplit_once("Select Model")
        .and_then(|(_, dialog)| dialog.split_once("(Press Esc to close)"))
        .is_some_and(|(_, bottom)| bottom.contains('╯'))
}
