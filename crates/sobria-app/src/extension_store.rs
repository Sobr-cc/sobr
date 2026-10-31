//! Sobr.ia — extension pairing persistence + event ingestion.
//!
//! Two SQLite tables (stored in `referentiel.sqlite`):
//!
//! - `device_pairings(id, fingerprint, secret_hash, created_at,
//!   last_seen_at, revoked_at)`
//!   One unique ID per extension ↔ app pairing. `secret_hash`
//!   is an **Argon2id PHC string** (see `crate::pairing`) that embeds
//!   salt + params; no clear secret in the DB. The former `salt_hex`
//!   column was dropped by the v2 → v3 migration.
//!
//! - `extension_events(id, pairing_id, ts, method, model_id, tokens_in,
//!   tokens_out, gco2eq_p50, water_ml, energy_wh, raw_payload_json,
//!   ingested_at)`
//!   One row per estimate sent up by the bridge, referencing
//!   `device_pairings.id` (no cascade: unpairing only marks the pairing
//!   revoked).
//!
//! `drain_spool` reads the spool `~/.sobria/spool/incoming.jsonl` and inserts
//! its events into `extension_events`.

use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::pairing::PairingSecret;

/// DDL schema v3 (idempotent: `CREATE TABLE IF NOT EXISTS`).
///
/// `secret_hash` is a self-contained Argon2id PHC string: the old
/// `salt_hex` column is dropped by [`migrate_device_pairings_v3`] on
/// existing databases.
const SCHEMA: &str = r"
CREATE TABLE IF NOT EXISTS device_pairings (
    id           TEXT PRIMARY KEY,
    fingerprint  TEXT NOT NULL,
    secret_hash  TEXT NOT NULL,
    created_at   TEXT NOT NULL,
    last_seen_at TEXT,
    revoked_at   TEXT,
    UNIQUE(fingerprint)
);
CREATE INDEX IF NOT EXISTS idx_device_pairings_fp ON device_pairings(fingerprint);

CREATE TABLE IF NOT EXISTS extension_events (
    id               TEXT PRIMARY KEY,
    pairing_id       TEXT NOT NULL,
    ts               TEXT NOT NULL,
    method           TEXT NOT NULL,
    model_id         TEXT NOT NULL,
    tokens_in        INTEGER NOT NULL,
    tokens_out       INTEGER NOT NULL,
    gco2eq_p50       REAL NOT NULL,
    water_ml         REAL NOT NULL,
    energy_wh        REAL NOT NULL,
    raw_payload_json TEXT NOT NULL,
    ingested_at      TEXT NOT NULL,
    FOREIGN KEY (pairing_id) REFERENCES device_pairings(id)
);
CREATE INDEX IF NOT EXISTS idx_extension_events_ts ON extension_events(ts);
CREATE INDEX IF NOT EXISTS idx_extension_events_pairing ON extension_events(pairing_id);
";

/// DTO exposed over IPC to list the paired extensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingRow {
    pub id: String,
    pub fingerprint: String,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Normalized DTO of an extension event after ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionEventRow {
    pub id: String,
    pub pairing_id: String,
    pub ts: DateTime<Utc>,
    pub method: String,
    pub model_id: String,
    pub tokens_in: u32,
    pub tokens_out: u32,
    pub gco2eq_p50: f64,
    pub water_ml: f64,
    pub energy_wh: f64,
    pub ingested_at: DateTime<Utc>,
}

/// Store of extension pairings + events.
pub struct ExtensionStore {
    conn: Connection,
}

impl ExtensionStore {
    /// Opens / creates the tables in `referentiel.sqlite`.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).with_context(|| format!("open {}", path.display()))?;
        conn.execute_batch(SCHEMA)
            .context("install extension_store schema")?;
        migrate_device_pairings_v3(&conn)
            .context("migration device_pairings v2 → v3 (Argon2id)")?;
        Ok(Self { conn })
    }

    /// Builds an in-memory store (useful for tests).
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        migrate_device_pairings_v3(&conn)?;
        Ok(Self { conn })
    }

    /// Records a new pairing.
    ///
    /// If `fingerprint` already exists (UNIQUE), the entry is replaced (re-pairing
    /// after an unpair). Returns the assigned `id` (ULID). `secret_hash` is an
    /// Argon2id PHC string that embeds salt + params.
    pub fn record_pairing(&self, fingerprint: &str, secret: &PairingSecret) -> Result<String> {
        let id = ulid();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO device_pairings(id, fingerprint, secret_hash, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(fingerprint) DO UPDATE SET
               id = excluded.id,
               secret_hash = excluded.secret_hash,
               created_at = excluded.created_at,
               last_seen_at = NULL,
               revoked_at = NULL",
            params![id, fingerprint, secret.secret_hash, now],
        )?;
        Ok(id)
    }

    /// Lists all pairings (revoked ones included, sorted by creation).
    pub fn list_pairings(&self) -> Result<Vec<PairingRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, fingerprint, created_at, last_seen_at, revoked_at
             FROM device_pairings
             ORDER BY created_at DESC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                let created: String = row.get(2)?;
                let last_seen: Option<String> = row.get(3)?;
                let revoked: Option<String> = row.get(4)?;
                Ok(PairingRow {
                    id: row.get(0)?,
                    fingerprint: row.get(1)?,
                    created_at: parse_dt(&created)?,
                    last_seen_at: last_seen.map(|s| parse_dt(&s)).transpose()?,
                    revoked_at: revoked.map(|s| parse_dt(&s)).transpose()?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Marks a pairing as revoked (does not delete the row: the history is
    /// kept for audit).
    pub fn revoke_pairing(&self, id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let affected = self.conn.execute(
            "UPDATE device_pairings SET revoked_at = ?1 WHERE id = ?2 AND revoked_at IS NULL",
            params![now, id],
        )?;
        if affected == 0 {
            anyhow::bail!("pairing introuvable ou déjà révoqué: {id}");
        }
        Ok(())
    }

    /// Checks that a clear secret matches an active pairing.
    /// Returns the `id` of the matched pairing, or `None` if the secret is
    /// invalid / the pairing is revoked. The stored hash is an Argon2id PHC
    /// string; the v3 migration ensures no legacy SHA-256 hash stays active.
    pub fn verify_secret(&self, fingerprint: &str, candidate_hex: &str) -> Result<Option<String>> {
        let row = self.conn.query_row(
            "SELECT id, secret_hash FROM device_pairings
             WHERE fingerprint = ?1 AND revoked_at IS NULL",
            params![fingerprint],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        );
        match row {
            Ok((id, hash)) => {
                if PairingSecret::verify_against(&hash, candidate_hex) {
                    // Updates `last_seen_at` (best-effort).
                    let now = Utc::now().to_rfc3339();
                    let _ = self.conn.execute(
                        "UPDATE device_pairings SET last_seen_at = ?1 WHERE id = ?2",
                        params![now, id],
                    );
                    Ok(Some(id))
                } else {
                    Ok(None)
                }
            },
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Inserts an event (already validated for an active pairing).
    pub fn record_event(&self, event: &ExtensionEventInput) -> Result<String> {
        let id = ulid();
        let ingested = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO extension_events(
                id, pairing_id, ts, method, model_id, tokens_in, tokens_out,
                gco2eq_p50, water_ml, energy_wh, raw_payload_json, ingested_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                id,
                event.pairing_id,
                event.ts.to_rfc3339(),
                event.method,
                event.model_id,
                event.tokens_in,
                event.tokens_out,
                event.gco2eq_p50,
                event.water_ml,
                event.energy_wh,
                event.raw_payload_json,
                ingested,
            ],
        )?;
        Ok(id)
    }

    /// Lists the events (paginated: `limit` + `offset`).
    pub fn list_events(&self, limit: usize, offset: usize) -> Result<Vec<ExtensionEventRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, pairing_id, ts, method, model_id, tokens_in, tokens_out,
                    gco2eq_p50, water_ml, energy_wh, ingested_at
             FROM extension_events
             ORDER BY ts DESC
             LIMIT ?1 OFFSET ?2",
        )?;
        let limit_i64 = i64::try_from(limit).unwrap_or(i64::MAX);
        let offset_i64 = i64::try_from(offset).unwrap_or(i64::MAX);
        let rows = stmt
            .query_map(params![limit_i64, offset_i64], |row| {
                let ts: String = row.get(2)?;
                let ingested: String = row.get(10)?;
                let tokens_in: i64 = row.get(5)?;
                let tokens_out: i64 = row.get(6)?;
                Ok(ExtensionEventRow {
                    id: row.get(0)?,
                    pairing_id: row.get(1)?,
                    ts: parse_dt(&ts)?,
                    method: row.get(3)?,
                    model_id: row.get(4)?,
                    tokens_in: u32::try_from(tokens_in).unwrap_or(0),
                    tokens_out: u32::try_from(tokens_out).unwrap_or(0),
                    gco2eq_p50: row.get(7)?,
                    water_ml: row.get(8)?,
                    energy_wh: row.get(9)?,
                    ingested_at: parse_dt(&ingested)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
}

/// Entry to insert into `extension_events` once the secret is validated.
#[derive(Debug, Clone)]
pub struct ExtensionEventInput {
    pub pairing_id: String,
    pub ts: DateTime<Utc>,
    pub method: String,
    pub model_id: String,
    pub tokens_in: u32,
    pub tokens_out: u32,
    pub gco2eq_p50: f64,
    pub water_ml: f64,
    pub energy_wh: f64,
    pub raw_payload_json: String,
}

/// Drains the spool file `incoming.jsonl` written by `sobria-bridge`.
///
/// Renames the spool, reads it and deletes it, then for each JSON line
/// credits the event to the first active pairing and inserts it into
/// `extension_events`. Malformed lines and failed inserts are skipped. The
/// secret is not checked yet (SOB-119). Returns the number of inserted events.
pub fn drain_spool(store: &ExtensionStore, spool_path: &Path) -> Result<usize> {
    if !spool_path.exists() {
        return Ok(0);
    }
    // Reads + renames atomically to avoid losing concurrent bridge
    // writes during the drain.
    let tmp = spool_path.with_extension("draining");
    std::fs::rename(spool_path, &tmp).with_context(|| {
        format!(
            "rename {} → {} (drain)",
            spool_path.display(),
            tmp.display()
        )
    })?;
    let content = std::fs::read_to_string(&tmp)?;
    let _ = std::fs::remove_file(&tmp);

    let mut inserted = 0;
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let envelope: SpoolEnvelope = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(error = %e, "skip ligne spool malformée");
                continue;
            },
        };
        let Some(input) = envelope.into_event_input(store)? else {
            tracing::warn!("skip ligne spool : aucun pairing actif ne valide le secret");
            continue;
        };
        if store.record_event(&input).is_ok() {
            inserted += 1;
        }
    }
    Ok(inserted)
}

/// Expected format of a spool line (see `sobria-bridge::handle_request`).
#[derive(Debug, Deserialize)]
struct SpoolEnvelope {
    secret_hash: Option<String>,
    payload: Value,
    received_at: Option<String>,
}

impl SpoolEnvelope {
    fn into_event_input(self, store: &ExtensionStore) -> Result<Option<ExtensionEventInput>> {
        let payload = &self.payload;
        let Some(estimate) = payload.get("estimate") else {
            return Ok(None);
        };
        // The payload carries no fingerprint, so the first active pairing
        // gets the event (SOB-119).
        let Some(pairing_id) = store.first_active_pairing()? else {
            return Ok(None);
        };
        let ts = self
            .received_at
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map_or_else(Utc::now, |d| d.with_timezone(&Utc));
        // The secret hash is read but not checked (SOB-119).
        let _ = self.secret_hash;
        Ok(Some(ExtensionEventInput {
            pairing_id,
            ts,
            method: estimate
                .get("method")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("afnor_sobria")
                .to_string(),
            model_id: estimate
                .get("modelId")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            tokens_in: u32::try_from(
                estimate
                    .get("tokensIn")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0),
            )
            .unwrap_or(u32::MAX),
            tokens_out: u32::try_from(
                estimate
                    .get("tokensOut")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0),
            )
            .unwrap_or(u32::MAX),
            gco2eq_p50: estimate
                .get("gco2eq")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            water_ml: estimate
                .get("waterMl")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            energy_wh: estimate
                .get("energyWh")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            raw_payload_json: payload.to_string(),
        }))
    }
}

impl ExtensionStore {
    /// Returns the `id` of the first active pairing (revoked ones excluded),
    /// which `drain_spool` credits while the payload has no fingerprint
    /// (SOB-119).
    fn first_active_pairing(&self) -> Result<Option<String>> {
        let row = self.conn.query_row(
            "SELECT id FROM device_pairings WHERE revoked_at IS NULL
             ORDER BY created_at DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        );
        match row {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

// ─── Private helpers ─────────────────────────────────────────────────────────

fn parse_dt(s: &str) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })
}

/// Crockford Base32 alphabet (ULID spec: no I, L, O, U).
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generates a ULID: 26 characters, time-sortable, no external dependency.
///
/// Format follows <https://github.com/ulid/spec>:
/// - 10 timestamp chars (48 bits of ms since epoch, big-endian)
/// - 16 random chars (80 bits, OS RNG via `rand::thread_rng`)
///
/// Advantage over UUID v4: lexicographic order = chronological order,
/// useful for `ORDER BY id` in SQL and for B-tree indexes (monotonic
/// inserts, no page splits).
fn ulid() -> String {
    use rand::RngCore;
    use std::time::{SystemTime, UNIX_EPOCH};

    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());

    // 10 chars: 48-bit timestamp, packed 5 bits per char.
    // 48 bits = 10×5 - 2, so the 1st char has 2 padding bits at the top.
    let mut out = [0u8; 26];
    let mut t = ms;
    for i in (0..10).rev() {
        out[i] = CROCKFORD[(t & 0x1f) as usize];
        t >>= 5;
    }

    // 16 chars: 80 random bits.
    let mut rand_bytes = [0u8; 10];
    rand::thread_rng().fill_bytes(&mut rand_bytes);
    // Reads the 80 bits as a right-aligned 128-bit integer and
    // shifts 5 bits at a time.
    let mut r: u128 = 0;
    for b in &rand_bytes {
        r = (r << 8) | u128::from(*b);
    }
    for i in (10..26).rev() {
        out[i] = CROCKFORD[(r & 0x1f) as usize];
        r >>= 5;
    }

    // SAFETY: all bytes come from CROCKFORD (alphanumeric ASCII).
    String::from_utf8(out.to_vec()).expect("ULID chars are ASCII by construction")
}

/// v2 → v3 migration of the `device_pairings` schema.
///
/// Old v2 schema: `(id, fingerprint, secret_hash[SHA-256 hex], salt_hex,
/// created_at, last_seen_at, revoked_at)`.
/// New v3 schema: same **without** `salt_hex`; `secret_hash` is a
/// self-contained Argon2id PHC string.
///
/// Strategy on an existing database:
/// 1. Detect the `salt_hex` column via `PRAGMA table_info`.
/// 2. Revoke every still-active pairing whose `secret_hash` is NOT
///    an Argon2id PHC (rationale: v0.6.0 has just shipped, few pairings
///    in prod; forcing a manual re-pairing is acceptable and better than
///    keeping a fragile dual-check mechanism).
/// 3. `ALTER TABLE ... DROP COLUMN salt_hex` (SQLite ≥ 3.35, bundled via
///    `rusqlite[bundled]`).
///
/// On a fresh v3 database (created by `SCHEMA` above), the function is
/// idempotent: no `salt_hex` column, so nothing to do.
fn migrate_device_pairings_v3(conn: &Connection) -> Result<()> {
    let has_salt_hex = conn
        .prepare("PRAGMA table_info(device_pairings)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(rusqlite::Result::ok)
        .any(|name| name == "salt_hex");
    if !has_salt_hex {
        return Ok(());
    }
    let now = Utc::now().to_rfc3339();
    let revoked = conn.execute(
        "UPDATE device_pairings SET revoked_at = ?1
         WHERE revoked_at IS NULL AND secret_hash NOT LIKE '$argon2id$%'",
        params![now],
    )?;
    conn.execute_batch("ALTER TABLE device_pairings DROP COLUMN salt_hex;")?;
    tracing::info!(
        revoked,
        "extension_store: migration v2 → v3 (Argon2id) — pairings legacy révoqués"
    );
    Ok(())
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_secret() -> PairingSecret {
        PairingSecret::new()
    }

    #[test]
    fn open_creates_tables() {
        let store = ExtensionStore::open_in_memory().unwrap();
        let _ = store.list_pairings().unwrap();
        let _ = store.list_events(10, 0).unwrap();
    }

    /// v2 → v3 migration: on a database with the old schema (`salt_hex`
    /// column, secret_hash = SHA-256 hex), the legacy pairings must be
    /// revoked and the column dropped.
    #[test]
    fn migration_v2_to_v3_revokes_legacy_pairings_and_drops_salt_hex() {
        let conn = Connection::open_in_memory().unwrap();
        // Recreates the v2 schema by hand (without calling open()).
        conn.execute_batch(
            r"CREATE TABLE device_pairings (
                id TEXT PRIMARY KEY,
                fingerprint TEXT NOT NULL,
                secret_hash TEXT NOT NULL,
                salt_hex TEXT NOT NULL,
                created_at TEXT NOT NULL,
                last_seen_at TEXT,
                revoked_at TEXT,
                UNIQUE(fingerprint)
            );",
        )
        .unwrap();
        // 1 legacy SHA-256 pairing (64 hex), 1 already migrated (argon2id PHC).
        conn.execute(
            "INSERT INTO device_pairings(id, fingerprint, secret_hash, salt_hex, created_at)
             VALUES ('a', 'fp-legacy', ?1, 'deadbeef', '2026-05-01T00:00:00Z')",
            params![&"a".repeat(64)],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO device_pairings(id, fingerprint, secret_hash, salt_hex, created_at)
             VALUES ('b', 'fp-phc',
                     '$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHRzYWx0c2FsdA$Zm9v',
                     'unused', '2026-05-01T00:00:00Z')",
            [],
        )
        .unwrap();

        migrate_device_pairings_v3(&conn).unwrap();

        // salt_hex column dropped.
        let cols: Vec<String> = conn
            .prepare("PRAGMA table_info(device_pairings)")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .filter_map(rusqlite::Result::ok)
            .collect();
        assert!(
            !cols.iter().any(|c| c == "salt_hex"),
            "salt_hex must be dropped"
        );

        // The legacy one is revoked, the PHC one stays active.
        let revoked_legacy: Option<String> = conn
            .query_row(
                "SELECT revoked_at FROM device_pairings WHERE id = 'a'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(revoked_legacy.is_some(), "legacy pairing must be revoked");
        let revoked_phc: Option<String> = conn
            .query_row(
                "SELECT revoked_at FROM device_pairings WHERE id = 'b'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(revoked_phc.is_none(), "argon2id pairing must remain active");

        // Idempotence: a second call does not break the database.
        migrate_device_pairings_v3(&conn).unwrap();
    }

    #[test]
    fn record_and_list_pairing() {
        let store = ExtensionStore::open_in_memory().unwrap();
        let s = make_secret();
        let id = store.record_pairing("chrome-mac-abc123", &s).unwrap();
        let list = store.list_pairings().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id);
        assert_eq!(list[0].fingerprint, "chrome-mac-abc123");
        assert!(list[0].revoked_at.is_none());
    }

    #[test]
    fn record_same_fingerprint_replaces() {
        let store = ExtensionStore::open_in_memory().unwrap();
        let id1 = store.record_pairing("fp1", &make_secret()).unwrap();
        let id2 = store.record_pairing("fp1", &make_secret()).unwrap();
        assert_ne!(id1, id2);
        let list = store.list_pairings().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id2);
    }

    #[test]
    fn verify_secret_matches_pairing() {
        let store = ExtensionStore::open_in_memory().unwrap();
        let s = make_secret();
        let id = store.record_pairing("fp", &s).unwrap();
        let matched = store.verify_secret("fp", &s.secret_hex).unwrap();
        assert_eq!(matched, Some(id));
    }

    #[test]
    fn verify_secret_rejects_wrong() {
        let store = ExtensionStore::open_in_memory().unwrap();
        let s = make_secret();
        store.record_pairing("fp", &s).unwrap();
        let matched = store.verify_secret("fp", "00".repeat(32).as_str()).unwrap();
        assert_eq!(matched, None);
    }

    #[test]
    fn verify_secret_rejects_revoked() {
        let store = ExtensionStore::open_in_memory().unwrap();
        let s = make_secret();
        let id = store.record_pairing("fp", &s).unwrap();
        store.revoke_pairing(&id).unwrap();
        let matched = store.verify_secret("fp", &s.secret_hex).unwrap();
        assert_eq!(matched, None, "secret révoqué doit être rejeté");
    }

    #[test]
    fn revoke_unknown_fails() {
        let store = ExtensionStore::open_in_memory().unwrap();
        assert!(store.revoke_pairing("does-not-exist").is_err());
    }

    #[test]
    fn record_event_then_list() {
        let store = ExtensionStore::open_in_memory().unwrap();
        let s = make_secret();
        let pid = store.record_pairing("fp", &s).unwrap();
        store
            .record_event(&ExtensionEventInput {
                pairing_id: pid.clone(),
                ts: Utc::now(),
                method: "afnor_sobria".into(),
                model_id: "gpt-4o".into(),
                tokens_in: 100,
                tokens_out: 500,
                gco2eq_p50: 0.42,
                water_ml: 1.8,
                energy_wh: 0.12,
                raw_payload_json: "{}".into(),
            })
            .unwrap();
        let events = store.list_events(10, 0).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].pairing_id, pid);
        assert_eq!(events[0].model_id, "gpt-4o");
    }

    #[test]
    fn drain_spool_empty_file_returns_zero() {
        let dir = TempDir::new().unwrap();
        let store = ExtensionStore::open_in_memory().unwrap();
        let path = dir.path().join("incoming.jsonl");
        let inserted = drain_spool(&store, &path).unwrap();
        assert_eq!(inserted, 0);
    }

    #[test]
    fn drain_spool_inserts_events() {
        let dir = TempDir::new().unwrap();
        let store = ExtensionStore::open_in_memory().unwrap();
        let s = make_secret();
        store.record_pairing("fp1", &s).unwrap();
        let path = dir.path().join("incoming.jsonl");
        // 2 valid lines
        let envelope = serde_json::json!({
            "secret_hash": "abcdef",
            "payload": {
                "estimate": {
                    "method": "afnor_sobria",
                    "modelId": "gpt-4o",
                    "tokensIn": 100,
                    "tokensOut": 500,
                    "gco2eq": 0.42,
                    "waterMl": 1.8,
                    "energyWh": 0.12
                }
            },
            "received_at": "2026-05-16T12:00:00Z"
        });
        let line = serde_json::to_string(&envelope).unwrap();
        std::fs::write(&path, format!("{line}\n{line}\n")).unwrap();
        let inserted = drain_spool(&store, &path).unwrap();
        assert_eq!(inserted, 2);
        assert!(!path.exists(), "spool doit être vidé après drain");
        let events = store.list_events(10, 0).unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn drain_spool_skips_malformed_lines() {
        let dir = TempDir::new().unwrap();
        let store = ExtensionStore::open_in_memory().unwrap();
        store.record_pairing("fp1", &make_secret()).unwrap();
        let path = dir.path().join("incoming.jsonl");
        let valid = serde_json::json!({
            "payload": { "estimate": { "method": "afnor_sobria", "modelId": "gpt-4o",
                "tokensIn": 1, "tokensOut": 1, "gco2eq": 0.1, "waterMl": 0.1, "energyWh": 0.01 }}
        });
        std::fs::write(
            &path,
            format!(
                "{}\nnot-json garbage\n\n{}\n",
                serde_json::to_string(&valid).unwrap(),
                serde_json::to_string(&valid).unwrap()
            ),
        )
        .unwrap();
        let inserted = drain_spool(&store, &path).unwrap();
        assert_eq!(inserted, 2, "deux lignes valides, garbage ignoré");
    }

    #[test]
    fn drain_spool_no_pairing_skips_all() {
        let dir = TempDir::new().unwrap();
        let store = ExtensionStore::open_in_memory().unwrap();
        // No pairing → first_active_pairing returns None.
        let path = dir.path().join("incoming.jsonl");
        let envelope = serde_json::json!({
            "payload": { "estimate": { "method": "x", "modelId": "y",
                "tokensIn": 1, "tokensOut": 1, "gco2eq": 0.1, "waterMl": 0.1, "energyWh": 0.01 }}
        });
        std::fs::write(&path, serde_json::to_string(&envelope).unwrap()).unwrap();
        let inserted = drain_spool(&store, &path).unwrap();
        assert_eq!(inserted, 0);
    }

    #[test]
    fn ulid_format() {
        let id = ulid();
        assert_eq!(id.len(), 26, "ULID = 26 chars");
        for c in id.bytes() {
            assert!(
                CROCKFORD.contains(&c),
                "char {} ({}) doit être dans Crockford Base32",
                c as char,
                c
            );
        }
    }

    #[test]
    fn ulid_is_unique_and_sortable() {
        // 10 consecutive ULIDs: all distinct AND lex order = time order.
        let mut ids: Vec<String> = Vec::with_capacity(10);
        for _ in 0..10 {
            ids.push(ulid());
            // Short sleep to guarantee a 1 ms gap on some OSes.
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), 10, "10 ULID doivent être distincts");
        // The time prefix (first 10 chars) must be increasing.
        let prefixes: Vec<&str> = ids.iter().map(|s| &s[..10]).collect();
        let mut sorted = prefixes.clone();
        sorted.sort_unstable();
        assert_eq!(prefixes, sorted, "préfixes temporels triés croissants");
    }
}
