use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::domain::LocalActivity;

use super::ActivityError;

const MAX_LINE_BYTES: usize = 2 * 1024 * 1024;
const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
const MAX_FILE_COUNT: usize = 4096;
const MAX_ENTRY_COUNT: usize = 16_384;

pub fn read_claude_activity(
    projects_directory: &Path,
    window_start_epoch: i64,
    window_end_epoch: i64,
) -> Result<LocalActivity, ActivityError> {
    if window_end_epoch <= window_start_epoch {
        return Err(ActivityError::InvalidData);
    }
    let mut files = Vec::new();
    collect_jsonl_files(projects_directory, &mut files)?;
    files.sort();
    let mut total_bytes = 0_u64;
    let mut sessions = HashSet::new();
    let mut session_spans: HashMap<String, (i64, i64)> = HashMap::new();
    let mut active_days = HashSet::new();
    let mut tokens = 0_u64;

    for path in files.into_iter().take(MAX_FILE_COUNT) {
        let metadata = fs::symlink_metadata(&path).map_err(|_| ActivityError::ReadFailure)?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.len() > MAX_FILE_BYTES
            || metadata.len() > MAX_TOTAL_BYTES.saturating_sub(total_bytes)
        {
            continue;
        }
        total_bytes = total_bytes.saturating_add(metadata.len());
        let is_subagent = path
            .components()
            .any(|component| component.as_os_str() == "subagents");
        for_each_bounded_line(&path, |line| {
            let Ok(entry) = serde_json::from_slice::<ClaudeLogEntry>(line) else {
                return;
            };
            let Ok(timestamp) = OffsetDateTime::parse(&entry.timestamp, &Rfc3339) else {
                return;
            };
            let epoch = timestamp.unix_timestamp();
            if !(window_start_epoch..window_end_epoch).contains(&epoch) {
                return;
            }
            let Some(components) = entry.message.and_then(|message| message.usage) else {
                return;
            };
            let values = [
                components.input_tokens.unwrap_or(0),
                components.output_tokens.unwrap_or(0),
                components.cache_creation_input_tokens.unwrap_or(0),
                components.cache_read_input_tokens.unwrap_or(0),
            ];
            if values.iter().any(|value| *value < 0) {
                return;
            }
            let entry_tokens = values
                .into_iter()
                .fold(0_u64, |total, value| total.saturating_add(value as u64));
            tokens = tokens.saturating_add(entry_tokens);
            active_days.insert(timestamp.date());
            if !is_subagent
                && let Some(session_id) = entry.session_id.filter(|value| !value.is_empty())
            {
                sessions.insert(session_id.clone());
                session_spans
                    .entry(session_id)
                    .and_modify(|span| {
                        span.0 = span.0.min(epoch);
                        span.1 = span.1.max(epoch);
                    })
                    .or_insert((epoch, epoch));
            }
        })?;
    }

    let longest_session_seconds = session_spans
        .values()
        .map(|(start, end)| end.saturating_sub(*start) as u64)
        .max();
    Ok(LocalActivity {
        period_days: period_days(window_start_epoch, window_end_epoch),
        sessions: sessions.len() as u64,
        tokens,
        active_days: active_days.len() as u64,
        longest_session_seconds,
    })
}

fn collect_jsonl_files(directory: &Path, output: &mut Vec<PathBuf>) -> Result<(), ActivityError> {
    let metadata = fs::symlink_metadata(directory).map_err(|_| ActivityError::Unavailable)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ActivityError::Unavailable);
    }
    let mut pending = vec![directory.to_owned()];
    let mut visited_entries = 0_usize;
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(current).map_err(|_| ActivityError::ReadFailure)? {
            visited_entries += 1;
            if visited_entries > MAX_ENTRY_COUNT {
                return Ok(());
            }
            let entry = entry.map_err(|_| ActivityError::ReadFailure)?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).map_err(|_| ActivityError::ReadFailure)?;
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("jsonl"))
                && output.len() < MAX_FILE_COUNT
            {
                output.push(path);
            }
        }
    }
    Ok(())
}

fn for_each_bounded_line(path: &Path, mut body: impl FnMut(&[u8])) -> Result<(), ActivityError> {
    let file = File::open(path).map_err(|_| ActivityError::ReadFailure)?;
    let mut reader = BufReader::new(file);
    let mut chunk = [0_u8; 64 * 1024];
    let mut line = Vec::new();
    let mut discarding = false;
    loop {
        let count = reader
            .read(&mut chunk)
            .map_err(|_| ActivityError::ReadFailure)?;
        if count == 0 {
            break;
        }
        for byte in &chunk[..count] {
            if *byte == b'\n' {
                if !discarding && !line.is_empty() {
                    body(&line);
                }
                line.clear();
                discarding = false;
            } else if !discarding {
                if line.len() < MAX_LINE_BYTES {
                    line.push(*byte);
                } else {
                    line.clear();
                    discarding = true;
                }
            }
        }
    }
    if !discarding && !line.is_empty() {
        body(&line);
    }
    Ok(())
}

fn period_days(start: i64, end: i64) -> u64 {
    u64::try_from(end.saturating_sub(start))
        .unwrap_or(u64::MAX)
        .div_ceil(86_400)
        .max(1)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeLogEntry {
    timestamp: String,
    #[serde(alias = "session_id")]
    session_id: Option<String>,
    message: Option<ClaudeMessage>,
}

#[derive(Deserialize)]
struct ClaudeMessage {
    usage: Option<ClaudeUsage>,
}

#[derive(Deserialize)]
struct ClaudeUsage {
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cache_creation_input_tokens: Option<i64>,
    cache_read_input_tokens: Option<i64>,
}
