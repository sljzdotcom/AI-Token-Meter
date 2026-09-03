use std::fs;

use ai_token_meter_windows::collectors::ActivityError;
use ai_token_meter_windows::collectors::claude_activity::{
    read_claude_activity, read_claude_activity_with_cancellation,
};
use ai_token_meter_windows::collectors::codex_activity::{
    read_codex_activity, read_codex_activity_with_cancellation,
};
use ai_token_meter_windows::platform::windows::process::CancellationToken;
use rusqlite::Connection;

#[test]
fn claude_activity_reads_only_allowlisted_usage_fields() {
    let directory = tempfile::tempdir().expect("activity directory");
    let log = directory.path().join("project").join("session.jsonl");
    fs::create_dir_all(log.parent().expect("log parent")).expect("create project");
    fs::write(
        &log,
        [
            claude_entry("2026-09-01T01:00:00Z", "session-1", 10, 20, 30, 40),
            claude_entry("2026-09-01T02:30:00Z", "session-1", 1, 2, 3, 4),
            "{malformed".to_owned(),
            claude_entry("2026-07-01T01:00:00Z", "old", 9_999, 0, 0, 0),
        ]
        .join("\n"),
    )
    .expect("write Claude log");

    let summary = read_claude_activity(
        directory.path(),
        epoch("2026-08-03T00:00:00Z"),
        epoch("2026-09-02T00:00:00Z"),
    )
    .expect("Claude activity");

    assert_eq!(summary.period_days, 30);
    assert_eq!(summary.sessions, 1);
    assert_eq!(summary.tokens, 110);
    assert_eq!(summary.active_days, 1);
    assert_eq!(summary.longest_session_seconds, Some(5_400));
}

#[test]
fn local_activity_scans_stop_before_touching_storage_when_cancelled() {
    let cancellation = CancellationToken::new();
    cancellation.cancel();

    assert_eq!(
        read_claude_activity_with_cancellation(
            std::path::Path::new("missing-claude-directory"),
            1,
            2,
            &cancellation,
        ),
        Err(ActivityError::Cancelled),
    );
    assert_eq!(
        read_codex_activity_with_cancellation(
            std::path::Path::new("missing-codex-database"),
            1,
            2,
            &cancellation,
        ),
        Err(ActivityError::Cancelled),
    );
}

#[test]
fn claude_activity_skips_symbolic_links_and_oversized_files() {
    let directory = tempfile::tempdir().expect("activity directory");
    let outside = tempfile::NamedTempFile::new().expect("outside log");
    fs::write(
        outside.path(),
        claude_entry("2026-09-01T01:00:00Z", "secret", 99, 0, 0, 0),
    )
    .expect("outside content");
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path(), directory.path().join("linked.jsonl"))
        .expect("symlink");
    fs::write(
        directory.path().join("oversized-line.jsonl"),
        format!(
            "{}\n{}",
            "x".repeat(2 * 1024 * 1024 + 1),
            claude_entry("2026-09-01T01:00:00Z", "valid", 7, 0, 0, 0)
        ),
    )
    .expect("oversized line fixture");

    let summary = read_claude_activity(
        directory.path(),
        epoch("2026-08-03T00:00:00Z"),
        epoch("2026-09-02T00:00:00Z"),
    )
    .expect("bounded activity");

    assert_eq!(summary.tokens, 7);
    assert_eq!(summary.sessions, 1);
}

#[test]
fn codex_activity_reads_the_local_database_in_read_only_mode() {
    let directory = tempfile::tempdir().expect("Codex directory");
    let database = directory.path().join("state_5.sqlite");
    let connection = Connection::open(&database).expect("create database");
    connection
        .execute_batch(
            "CREATE TABLE threads(tokens_used INTEGER, created_at INTEGER, updated_at INTEGER);\
             INSERT INTO threads VALUES (1000, 1788220800, 1788307200);\
             INSERT INTO threads VALUES (500, 1788393600, 1788397200);\
             INSERT INTO threads VALUES (9999, 1700000000, 1700000100);",
        )
        .expect("seed threads");
    drop(connection);

    let summary = read_codex_activity(
        &database,
        epoch("2026-08-05T00:00:00Z"),
        epoch("2026-09-04T00:00:00Z"),
    )
    .expect("Codex activity");

    assert_eq!(summary.period_days, 30);
    assert_eq!(summary.sessions, 2);
    assert_eq!(summary.tokens, 1_500);
    assert_eq!(summary.active_days, 3);
    assert_eq!(summary.longest_session_seconds, Some(86_400));
}

fn claude_entry(
    timestamp: &str,
    session: &str,
    input: i64,
    output: i64,
    cache_creation: i64,
    cache_read: i64,
) -> String {
    serde_json::json!({
        "timestamp": timestamp,
        "sessionId": session,
        "cwd": r"C:\private",
        "message": {
            "content": "must not be read 999999",
            "usage": {
                "input_tokens": input,
                "output_tokens": output,
                "cache_creation_input_tokens": cache_creation,
                "cache_read_input_tokens": cache_read
            }
        }
    })
    .to_string()
}

fn epoch(value: &str) -> i64 {
    time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
        .expect("timestamp")
        .unix_timestamp()
}
