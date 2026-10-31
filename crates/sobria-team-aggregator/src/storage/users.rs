//! SQL helpers for the `users` table.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::error::AggregatorResult;
use crate::storage::estimations::UsageTotals;
use crate::storage::parse_ts;

/// Enrolled employee. `password_hash` is an Argon2id PHC (see `crypto::password`).
#[derive(Debug, Clone)]
pub struct UserRow {
    pub id: String,
    pub enrollment_code_id: Option<String>,
    pub fingerprint: String,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

/// Inserts a new user (the Argon2id hash must be computed beforehand).
pub fn insert(
    conn: &Connection,
    id: &str,
    enrollment_code_id: Option<&str>,
    fingerprint: &str,
    password_hash: &str,
    display_name: Option<&str>,
    created_at: DateTime<Utc>,
) -> AggregatorResult<()> {
    conn.execute(
        "INSERT INTO users
            (id, enrollment_code_id, fingerprint, password_hash, display_name, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id,
            enrollment_code_id,
            fingerprint,
            password_hash,
            display_name,
            created_at.to_rfc3339()
        ],
    )?;
    Ok(())
}

/// Looks up a user by id (ULID).
pub fn find_by_id(conn: &Connection, id: &str) -> AggregatorResult<Option<UserRow>> {
    let row = conn
        .query_row(
            "SELECT id, enrollment_code_id, fingerprint, password_hash,
                    display_name, created_at, last_seen_at
             FROM users WHERE id = ?1",
            params![id],
            map_row,
        )
        .optional()?;
    Ok(row)
}

/// Looks up a user by fingerprint (uniqueness enforced by the schema).
pub fn find_by_fingerprint(
    conn: &Connection,
    fingerprint: &str,
) -> AggregatorResult<Option<UserRow>> {
    let row = conn
        .query_row(
            "SELECT id, enrollment_code_id, fingerprint, password_hash,
                    display_name, created_at, last_seen_at
             FROM users WHERE fingerprint = ?1",
            params![fingerprint],
            map_row,
        )
        .optional()?;
    Ok(row)
}

/// User row for the admin list: a MANAGEMENT view (ADR-0015).
///
/// `totals` is set only if the employee turned on identified sharing
/// (`share_identified`). Otherwise `None`: the admin manages accounts
/// (enrollment, last contact, revocation) without seeing individual
/// usage.
#[derive(Debug, Clone, Serialize)]
pub struct UserWithTotals {
    pub id: String,
    pub fingerprint: String,
    pub display_name: Option<String>,
    pub enrollment_code_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
    /// The employee's opt-in consent (ADR-0015 §3).
    pub share_identified: bool,
    /// Usage totals: `None` while `share_identified` is false.
    pub totals: Option<UsageTotals>,
}

/// Lists every user with their totals (LEFT JOIN estimations).
/// Sorted by last activity, descending (active users first).
/// Totals of accounts without sharing on are masked (`None`): the mask is
/// applied HERE, server-side, not in the UI.
pub fn list_all_with_totals(conn: &Connection) -> AggregatorResult<Vec<UserWithTotals>> {
    list_all_with_totals_inner(conn, true)
}

/// UNMASKED variant: reserved for the `identified` policy
/// (ADR-0016); totals set for everyone, opt-in ignored.
pub fn list_all_with_totals_unmasked(conn: &Connection) -> AggregatorResult<Vec<UserWithTotals>> {
    list_all_with_totals_inner(conn, false)
}

fn list_all_with_totals_inner(
    conn: &Connection,
    mask: bool,
) -> AggregatorResult<Vec<UserWithTotals>> {
    let mut stmt = conn.prepare(
        "SELECT u.id, u.fingerprint, u.display_name, u.enrollment_code_id,
                u.created_at, u.last_seen_at, u.share_identified,
                COUNT(e.id),
                COALESCE(SUM(e.tokens_in), 0),
                COALESCE(SUM(e.tokens_out), 0),
                COALESCE(SUM(e.gco2eq_p50), 0.0),
                COALESCE(SUM(e.water_ml), 0.0),
                COALESCE(SUM(e.energy_wh), 0.0)
         FROM users u
         LEFT JOIN estimations e ON e.user_id = u.id
         GROUP BY u.id, u.fingerprint, u.display_name, u.enrollment_code_id,
                  u.created_at, u.last_seen_at, u.share_identified
         ORDER BY COALESCE(u.last_seen_at, u.created_at) DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        let created_at_s: String = row.get(4)?;
        let last_seen_s: Option<String> = row.get(5)?;
        let share_identified: bool = row.get::<_, i64>(6)? != 0;
        let totals = UsageTotals::from_row(row, 7)?;
        Ok(UserWithTotals {
            id: row.get(0)?,
            fingerprint: row.get(1)?,
            display_name: row.get(2)?,
            enrollment_code_id: row.get(3)?,
            created_at: parse_ts(&created_at_s, 4)?,
            last_seen_at: last_seen_s.map(|s| parse_ts(&s, 5)).transpose()?,
            share_identified,
            totals: if mask {
                share_identified.then_some(totals)
            } else {
                Some(totals)
            },
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Reads the identified-sharing consent (ADR-0015 §3).
pub fn share_identified(conn: &Connection, id: &str) -> AggregatorResult<bool> {
    let v: i64 = conn.query_row(
        "SELECT share_identified FROM users WHERE id = ?1",
        params![id],
        |r| r.get(0),
    )?;
    Ok(v != 0)
}

/// Writes the identified-sharing consent (idempotent).
pub fn set_share_identified(conn: &Connection, id: &str, share: bool) -> AggregatorResult<()> {
    conn.execute(
        "UPDATE users SET share_identified = ?1 WHERE id = ?2",
        params![i64::from(share), id],
    )?;
    Ok(())
}

/// Updates `last_seen_at` (idempotent).
pub fn touch_last_seen(conn: &Connection, id: &str, now: DateTime<Utc>) -> AggregatorResult<()> {
    conn.execute(
        "UPDATE users SET last_seen_at = ?1 WHERE id = ?2",
        params![now.to_rfc3339(), id],
    )?;
    Ok(())
}

fn map_row(row: &rusqlite::Row) -> rusqlite::Result<UserRow> {
    let created_at_s: String = row.get(5)?;
    let last_seen_s: Option<String> = row.get(6)?;
    Ok(UserRow {
        id: row.get(0)?,
        enrollment_code_id: row.get(1)?,
        fingerprint: row.get(2)?,
        password_hash: row.get(3)?,
        display_name: row.get(4)?,
        created_at: parse_ts(&created_at_s, 5)?,
        last_seen_at: last_seen_s.map(|s| parse_ts(&s, 6)).transpose()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;

    #[test]
    fn insert_and_lookup_by_fingerprint() {
        let s = Storage::open_in_memory().unwrap();
        insert(
            s.connection(),
            "u-1",
            None,
            "chrome-mac-abc",
            "$argon2id$h",
            Some("Alice"),
            Utc::now(),
        )
        .unwrap();
        let row = find_by_fingerprint(s.connection(), "chrome-mac-abc")
            .unwrap()
            .unwrap();
        assert_eq!(row.id, "u-1");
        assert_eq!(row.display_name.as_deref(), Some("Alice"));
        assert!(find_by_fingerprint(s.connection(), "unknown")
            .unwrap()
            .is_none());
    }

    #[test]
    fn fingerprint_is_unique() {
        let s = Storage::open_in_memory().unwrap();
        insert(s.connection(), "u-1", None, "fp", "h", None, Utc::now()).unwrap();
        let dup = insert(s.connection(), "u-2", None, "fp", "h", None, Utc::now());
        assert!(dup.is_err());
    }

    #[test]
    fn list_all_with_totals_includes_users_without_estimations() {
        use crate::storage::estimations;
        let s = Storage::open_in_memory().unwrap();
        insert(
            s.connection(),
            "u-1",
            None,
            "fp-1",
            "h",
            Some("Alice"),
            Utc::now(),
        )
        .unwrap();
        insert(s.connection(), "u-2", None, "fp-2", "h", None, Utc::now()).unwrap();

        // u-1 has 2 estimations, u-2 none.
        for (id, g) in [("e-1", 0.4), ("e-2", 0.6)] {
            let est = estimations::NewEstimation {
                id,
                user_id: "u-1",
                ts: Utc::now(),
                method: "afnor_sobria",
                model_id: "m",
                tokens_in: 10,
                tokens_out: 20,
                gco2eq_p50: g,
                gco2eq_p5: None,
                gco2eq_p95: None,
                water_ml: 0.0,
                energy_wh: 0.0,
                region: None,
                project: None,
                raw_payload_json: "{}",
                received_at: Utc::now(),
            };
            estimations::insert(s.connection(), &est).unwrap();
        }

        // By default (no opt-in): totals masked for everyone.
        let rows = list_all_with_totals(s.connection()).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows
            .iter()
            .all(|r| !r.share_identified && r.totals.is_none()));

        // u-1 turns on sharing → their totals appear, not u-2's.
        set_share_identified(s.connection(), "u-1", true).unwrap();
        let rows = list_all_with_totals(s.connection()).unwrap();
        let by_id: std::collections::HashMap<_, _> =
            rows.iter().map(|r| (r.id.clone(), r.clone())).collect();
        let t1 = by_id["u-1"]
            .totals
            .as_ref()
            .expect("u-1 a opté pour le partage");
        assert_eq!(t1.count, 2);
        assert!((t1.gco2eq_p50_g - 1.0).abs() < 1e-9);
        assert!(by_id["u-2"].totals.is_none());
    }

    #[test]
    fn unmasked_variant_exposes_all_totals() {
        let s = Storage::open_in_memory().unwrap();
        insert(
            s.connection(),
            "u-1",
            None,
            "fp-1",
            "h",
            Some("Alice"),
            Utc::now(),
        )
        .unwrap();
        // No opt-in: masked → None, unmasked → Some.
        assert!(list_all_with_totals(s.connection()).unwrap()[0]
            .totals
            .is_none());
        assert!(list_all_with_totals_unmasked(s.connection()).unwrap()[0]
            .totals
            .is_some());
    }

    #[test]
    fn share_identified_roundtrip_defaults_to_false() {
        let s = Storage::open_in_memory().unwrap();
        insert(s.connection(), "u-1", None, "fp", "h", None, Utc::now()).unwrap();
        assert!(!share_identified(s.connection(), "u-1").unwrap());
        set_share_identified(s.connection(), "u-1", true).unwrap();
        assert!(share_identified(s.connection(), "u-1").unwrap());
        set_share_identified(s.connection(), "u-1", false).unwrap();
        assert!(!share_identified(s.connection(), "u-1").unwrap());
    }
}
