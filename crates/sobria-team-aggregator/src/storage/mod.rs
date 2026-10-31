//! Persistence layer of the `sobria-team-aggregator`.
//!
//! [`Storage`] wraps one single-threaded `rusqlite::Connection` (WAL and
//! foreign keys on). The server shares it as `Arc<Mutex<Storage>>`, since the
//! connection is not `Sync`.
//!
//! Per-table helpers live in the submodules as free functions taking
//! `&Connection`. `Storage` itself only opens the database, runs the
//! migrations and reads and writes the `config` key-value table.

pub mod admins;
pub mod analytics;
pub mod enrollment_codes;
pub mod estimations;
pub mod schema;
pub mod tokens;
pub mod users;

use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::AggregatorResult;

/// Reads an RFC 3339 text column as UTC; a parse failure reports column `col`.
pub(crate) fn parse_ts(s: &str, col: usize) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(col, rusqlite::types::Type::Text, Box::new(e))
        })
}

/// Façade SQLite mono-connexion (cf. doc du module).
pub struct Storage {
    conn: Connection,
}

impl Storage {
    /// Ouvre `team.sqlite` (le crée si absent) et applique le schéma v1.
    pub fn open(path: &Path) -> AggregatorResult<Self> {
        let conn = Connection::open(path)?;
        schema::install(&conn)?;
        Ok(Self { conn })
    }

    /// Ouvre une base en mémoire (tests).
    pub fn open_in_memory() -> AggregatorResult<Self> {
        let conn = Connection::open_in_memory()?;
        schema::install(&conn)?;
        Ok(Self { conn })
    }

    /// Accès partagé en lecture-écriture à la connexion pour les helpers
    /// par entité (`users::insert(storage.connection(), …)`).
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Lit une valeur du KV `config`.
    pub fn get_config(&self, key: &str) -> AggregatorResult<Option<String>> {
        let value = self
            .conn
            .query_row(
                "SELECT value FROM config WHERE key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(value)
    }

    /// Pose une valeur du KV `config` (upsert).
    pub fn set_config(&self, key: &str, value: &str) -> AggregatorResult<()> {
        self.conn.execute(
            "INSERT INTO config (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_set_get_roundtrip() {
        let s = Storage::open_in_memory().unwrap();
        assert!(s.get_config("jwt_secret").unwrap().is_none());
        s.set_config("jwt_secret", "deadbeef").unwrap();
        assert_eq!(
            s.get_config("jwt_secret").unwrap().as_deref(),
            Some("deadbeef")
        );
        s.set_config("jwt_secret", "cafebabe").unwrap();
        assert_eq!(
            s.get_config("jwt_secret").unwrap().as_deref(),
            Some("cafebabe")
        );
    }
}
