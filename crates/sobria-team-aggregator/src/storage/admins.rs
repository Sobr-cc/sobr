//! SQL helpers for the `admins` table.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::AggregatorResult;
use crate::storage::{get_count, parse_ts};

/// Admin DTO as stored in the database.
#[derive(Debug, Clone)]
pub struct AdminRow {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

/// Inserts a new admin (the Argon2id PHC hash must be computed beforehand).
pub fn insert(
    conn: &Connection,
    id: &str,
    username: &str,
    password_hash: &str,
    created_at: DateTime<Utc>,
) -> AggregatorResult<()> {
    conn.execute(
        "INSERT INTO admins (id, username, password_hash, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![id, username, password_hash, created_at.to_rfc3339()],
    )?;
    Ok(())
}

/// Total number of admins.
pub fn count(conn: &Connection) -> AggregatorResult<u32> {
    let n = conn.query_row("SELECT COUNT(*) FROM admins", [], |row| get_count(row, 0))?;
    Ok(n as u32)
}

/// Looks up an admin by username.
pub fn find_by_username(conn: &Connection, username: &str) -> AggregatorResult<Option<AdminRow>> {
    let row = conn
        .query_row(
            "SELECT id, username, password_hash, created_at, last_login_at
             FROM admins WHERE username = ?1",
            params![username],
            map_row,
        )
        .optional()?;
    Ok(row)
}

/// Looks up an admin by id (ULID).
pub fn find_by_id(conn: &Connection, id: &str) -> AggregatorResult<Option<AdminRow>> {
    let row = conn
        .query_row(
            "SELECT id, username, password_hash, created_at, last_login_at
             FROM admins WHERE id = ?1",
            params![id],
            map_row,
        )
        .optional()?;
    Ok(row)
}

/// Updates `last_login_at` (idempotent, writes `now`).
pub fn touch_last_login(conn: &Connection, id: &str, now: DateTime<Utc>) -> AggregatorResult<()> {
    conn.execute(
        "UPDATE admins SET last_login_at = ?1 WHERE id = ?2",
        params![now.to_rfc3339(), id],
    )?;
    Ok(())
}

/// Lists every admin (used by the `admin list` CLI).
pub fn list_all(conn: &Connection) -> AggregatorResult<Vec<AdminRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, username, password_hash, created_at, last_login_at
         FROM admins
         ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map([], map_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Rewrites an admin's password hash (`admin reset-password`).
/// Also sets `last_login_at = NULL` to drop the notion of an ongoing
/// session. Returns the number of affected rows (0 if the username is unknown).
pub fn set_password_hash(
    conn: &Connection,
    username: &str,
    new_hash: &str,
) -> AggregatorResult<usize> {
    let n = conn.execute(
        "UPDATE admins
         SET password_hash = ?2, last_login_at = NULL
         WHERE username = ?1",
        params![username, new_hash],
    )?;
    Ok(n)
}

fn map_row(row: &rusqlite::Row) -> rusqlite::Result<AdminRow> {
    let created_at_s: String = row.get(3)?;
    let last_login_s: Option<String> = row.get(4)?;
    Ok(AdminRow {
        id: row.get(0)?,
        username: row.get(1)?,
        password_hash: row.get(2)?,
        created_at: parse_ts(&created_at_s, 3)?,
        last_login_at: last_login_s.map(|s| parse_ts(&s, 4)).transpose()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;

    #[test]
    fn insert_and_lookup() {
        let s = Storage::open_in_memory().unwrap();
        assert_eq!(count(s.connection()).unwrap(), 0);

        insert(s.connection(), "a-1", "alice", "$argon2id$fake", Utc::now()).unwrap();

        assert_eq!(count(s.connection()).unwrap(), 1);
        let by_username = find_by_username(s.connection(), "alice").unwrap().unwrap();
        assert_eq!(by_username.id, "a-1");
        assert!(by_username.last_login_at.is_none());

        let by_id = find_by_id(s.connection(), "a-1").unwrap().unwrap();
        assert_eq!(by_id.username, "alice");
        assert!(find_by_username(s.connection(), "bob").unwrap().is_none());
    }

    #[test]
    fn username_is_unique() {
        let s = Storage::open_in_memory().unwrap();
        insert(s.connection(), "a-1", "alice", "h", Utc::now()).unwrap();
        let dup = insert(s.connection(), "a-2", "alice", "h", Utc::now());
        assert!(dup.is_err());
    }

    #[test]
    fn touch_last_login_updates_field() {
        let s = Storage::open_in_memory().unwrap();
        insert(s.connection(), "a-1", "alice", "h", Utc::now()).unwrap();
        let now = Utc::now();
        touch_last_login(s.connection(), "a-1", now).unwrap();
        let row = find_by_id(s.connection(), "a-1").unwrap().unwrap();
        let stored = row.last_login_at.unwrap();
        // Allows <1 second of drift (RFC3339 serialization).
        assert!((stored - now).num_seconds().abs() < 2);
    }
}
