//! SQLite helpers shared by the stores of `referentiel.sqlite`.

use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::Connection;

/// Opens the database at `path` in WAL journal mode.
pub(crate) fn open_referentiel(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    Ok(conn)
}

/// Conversion error for the TEXT column `col`.
pub(crate) fn conversion_failure(
    col: usize,
    e: impl std::error::Error + Send + Sync + 'static,
) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(col, rusqlite::types::Type::Text, Box::new(e))
}

/// Parses the RFC 3339 text of column `col` as a UTC time.
pub(crate) fn parse_rfc3339_col(s: &str, col: usize) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| conversion_failure(col, e))
}
