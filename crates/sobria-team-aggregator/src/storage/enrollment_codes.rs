//! SQL helpers for the `enrollment_codes` table.
//!
//! Usage pattern:
//!
//! 1. An admin calls [`create_batch`](crate::commands::code::create_batch) with N
//!    12-digit codes + a TTL. Each code is hashed with Argon2id (see
//!    `crypto::password`) and inserted. The admin gets the plain list ONCE
//!    (for distribution).
//! 2. On enroll, [`verify_active_code`] walks every still-valid code and
//!    checks the PHC. Cost O(N×argon2). Fine for N ≤ a few thousand
//!    (/enroll calls stay rare over a team's life).
//! 3. [`mark_used`] locks the code (single-use) by setting `used_at` +
//!    `used_by_user_id`.
//! 4. [`revoke`] lets the admin cancel an unused code.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

use crate::crypto::password::{self, verify_password};
use crate::error::AggregatorResult;
use crate::storage::parse_ts;

/// A stored code (the clear text is never available).
#[derive(Debug, Clone)]
pub struct EnrollmentCodeRow {
    pub id: String,
    pub code_hash: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub used_by_user_id: Option<String>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Plain code + metadata returned by
/// [`create_batch`](crate::commands::code::create_batch) (show it to the admin ONCE).
#[derive(Debug, Clone)]
pub struct CreatedCode {
    pub id: String,
    pub code: String,
    pub expires_at: DateTime<Utc>,
}

/// Inserts one code (`code` in clear, Argon2id-hashed before insertion).
pub fn insert(
    conn: &Connection,
    id: &str,
    code: &str,
    created_by: &str,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> AggregatorResult<()> {
    let hash = password::hash_password(code)?;
    conn.execute(
        "INSERT INTO enrollment_codes
            (id, code_hash, created_by, created_at, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            id,
            hash,
            created_by,
            created_at.to_rfc3339(),
            expires_at.to_rfc3339()
        ],
    )?;
    Ok(())
}

/// Walks every active code and returns the one matching `candidate`.
///
/// O(N) in active codes (Argon2 is slow: ~50 ms/verify).
/// Returns the full row; the caller must call `mark_used` before
/// issuing tokens.
pub fn verify_active_code(
    conn: &Connection,
    candidate: &str,
    now: DateTime<Utc>,
) -> AggregatorResult<Option<EnrollmentCodeRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, code_hash, created_by, created_at, expires_at,
                used_at, used_by_user_id, revoked_at
         FROM enrollment_codes
         WHERE used_at IS NULL
           AND revoked_at IS NULL
           AND expires_at > ?1",
    )?;
    let rows = stmt.query_map(params![now.to_rfc3339()], map_row)?;
    for row in rows {
        let row = row?;
        if verify_password(&row.code_hash, candidate) {
            return Ok(Some(row));
        }
    }
    Ok(None)
}

/// Marks a code as consumed (single-use). Returns `false` if the code is
/// missing, already used, or revoked (transaction safe).
pub fn mark_used(
    conn: &Connection,
    code_id: &str,
    user_id: &str,
    now: DateTime<Utc>,
) -> AggregatorResult<bool> {
    let n = conn.execute(
        "UPDATE enrollment_codes
         SET used_at = ?1, used_by_user_id = ?2
         WHERE id = ?3
           AND used_at IS NULL
           AND revoked_at IS NULL",
        params![now.to_rfc3339(), user_id, code_id],
    )?;
    Ok(n == 1)
}

/// Revokes a code (admin → blocks future enrollment).
pub fn revoke(conn: &Connection, code_id: &str, now: DateTime<Utc>) -> AggregatorResult<bool> {
    let n = conn.execute(
        "UPDATE enrollment_codes
         SET revoked_at = ?1
         WHERE id = ?2 AND revoked_at IS NULL",
        params![now.to_rfc3339(), code_id],
    )?;
    Ok(n == 1)
}

/// Lists every code (admin view).
pub fn list_all(conn: &Connection) -> AggregatorResult<Vec<EnrollmentCodeRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, code_hash, created_by, created_at, expires_at,
                used_at, used_by_user_id, revoked_at
         FROM enrollment_codes
         ORDER BY created_at DESC",
    )?;
    let rows = stmt
        .query_map([], map_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn map_row(row: &rusqlite::Row) -> rusqlite::Result<EnrollmentCodeRow> {
    let created_at_s: String = row.get(3)?;
    let expires_at_s: String = row.get(4)?;
    let used_at_s: Option<String> = row.get(5)?;
    let revoked_at_s: Option<String> = row.get(7)?;
    Ok(EnrollmentCodeRow {
        id: row.get(0)?,
        code_hash: row.get(1)?,
        created_by: row.get(2)?,
        created_at: parse_ts(&created_at_s, 3)?,
        expires_at: parse_ts(&expires_at_s, 4)?,
        used_at: used_at_s.map(|s| parse_ts(&s, 5)).transpose()?,
        used_by_user_id: row.get(6)?,
        revoked_at: revoked_at_s.map(|s| parse_ts(&s, 7)).transpose()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{admins, Storage};
    use chrono::Duration;

    fn setup_admin(s: &Storage) -> String {
        admins::insert(s.connection(), "a-1", "admin", "h", Utc::now()).unwrap();
        "a-1".to_string()
    }

    #[test]
    fn insert_and_verify_active_code() {
        let s = Storage::open_in_memory().unwrap();
        let admin_id = setup_admin(&s);
        let now = Utc::now();
        let exp = now + Duration::days(7);

        insert(s.connection(), "c-1", "384729104857", &admin_id, now, exp).unwrap();

        // The right code is found.
        let row = verify_active_code(s.connection(), "384729104857", now)
            .unwrap()
            .expect("code trouvé");
        assert_eq!(row.id, "c-1");

        // A wrong code → None.
        let none = verify_active_code(s.connection(), "000000000000", now).unwrap();
        assert!(none.is_none());
    }

    #[test]
    fn expired_code_is_rejected() {
        let s = Storage::open_in_memory().unwrap();
        let admin_id = setup_admin(&s);
        let now = Utc::now();
        let exp = now - Duration::seconds(1);

        insert(
            s.connection(),
            "c-1",
            "111111111111",
            &admin_id,
            now - Duration::days(8),
            exp,
        )
        .unwrap();
        let none = verify_active_code(s.connection(), "111111111111", now).unwrap();
        assert!(none.is_none());
    }

    #[test]
    fn mark_used_makes_code_inactive() {
        let s = Storage::open_in_memory().unwrap();
        let admin_id = setup_admin(&s);
        let now = Utc::now();
        insert(
            s.connection(),
            "c-1",
            "222222222222",
            &admin_id,
            now,
            now + Duration::days(7),
        )
        .unwrap();

        // Creates a dummy user (the users FK is not tested here: turning FKs off
        // would not cover arbitrary ids; we keep the strict contract).
        crate::storage::users::insert(
            s.connection(),
            "u-1",
            Some("c-1"),
            "chrome-mac-abc",
            "$argon2id$fake",
            None,
            now,
        )
        .unwrap();

        let ok = mark_used(s.connection(), "c-1", "u-1", now).unwrap();
        assert!(ok);

        // Code checked again → no longer active.
        let none = verify_active_code(s.connection(), "222222222222", now).unwrap();
        assert!(none.is_none());

        // mark_used again on the same code → false (single-use).
        let again = mark_used(s.connection(), "c-1", "u-1", now).unwrap();
        assert!(!again);
    }

    #[test]
    fn revoke_makes_code_inactive() {
        let s = Storage::open_in_memory().unwrap();
        let admin_id = setup_admin(&s);
        let now = Utc::now();
        insert(
            s.connection(),
            "c-1",
            "333333333333",
            &admin_id,
            now,
            now + Duration::days(7),
        )
        .unwrap();

        assert!(revoke(s.connection(), "c-1", now).unwrap());
        let none = verify_active_code(s.connection(), "333333333333", now).unwrap();
        assert!(none.is_none());
        // Revoke again → false.
        assert!(!revoke(s.connection(), "c-1", now).unwrap());
    }
}
