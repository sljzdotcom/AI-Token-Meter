use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OpenFlags, params};
use time::OffsetDateTime;

use crate::domain::LocalActivity;

use super::ActivityError;

pub fn read_codex_activity(
    database: &Path,
    window_start_epoch: i64,
    window_end_epoch: i64,
) -> Result<LocalActivity, ActivityError> {
    if window_end_epoch <= window_start_epoch || !database.is_file() {
        return Err(ActivityError::Unavailable);
    }
    let connection = Connection::open_with_flags(
        database,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| ActivityError::ReadFailure)?;
    connection
        .busy_timeout(Duration::from_secs(1))
        .and_then(|_| connection.pragma_update(None, "query_only", true))
        .map_err(|_| ActivityError::ReadFailure)?;
    let mut statement = connection
        .prepare(
            "SELECT tokens_used, created_at, updated_at FROM threads \
             WHERE updated_at >= ?1 AND updated_at < ?2 ORDER BY updated_at LIMIT 100000",
        )
        .map_err(|_| ActivityError::ReadFailure)?;
    let rows = statement
        .query_map(params![window_start_epoch, window_end_epoch], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|_| ActivityError::ReadFailure)?;
    let mut sessions = 0_u64;
    let mut tokens = 0_u64;
    let mut active_days = HashSet::new();
    let mut longest = None;
    for row in rows {
        let (row_tokens, created_at, updated_at) = row.map_err(|_| ActivityError::ReadFailure)?;
        if row_tokens < 0 {
            continue;
        }
        sessions = sessions.saturating_add(1);
        tokens = tokens.saturating_add(row_tokens as u64);
        for epoch in [created_at, updated_at] {
            if (window_start_epoch..window_end_epoch).contains(&epoch)
                && let Ok(timestamp) = OffsetDateTime::from_unix_timestamp(epoch)
            {
                active_days.insert(timestamp.date());
            }
        }
        let duration = updated_at.saturating_sub(created_at).max(0) as u64;
        longest = Some(longest.unwrap_or(0).max(duration));
    }
    Ok(LocalActivity {
        period_days: u64::try_from(window_end_epoch.saturating_sub(window_start_epoch))
            .unwrap_or(u64::MAX)
            .div_ceil(86_400)
            .max(1),
        sessions,
        tokens,
        active_days: active_days.len() as u64,
        longest_session_seconds: longest,
    })
}
