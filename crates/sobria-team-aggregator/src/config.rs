//! Paths, constants and operator runtime keys of the `sobria-team-aggregator`.
//!
//! The data dir holds `team.sqlite`, `cert.pem` and `key.pem`. Defaults to
//! `./team-data/` (relative to the CWD). The admin can override it with
//! `--data-dir`.

use std::path::{Path, PathBuf};

use crate::error::AggregatorResult;
use crate::storage::Storage;

/// Main SQLite file name.
pub const DB_FILENAME: &str = "team.sqlite";

/// TLS certificate file name (PEM).
pub const CERT_FILENAME: &str = "cert.pem";

/// TLS private key file name (PEM).
pub const KEY_FILENAME: &str = "key.pem";

/// Default HTTPS port.
pub const DEFAULT_PORT: u16 = 8443;

/// Default bind address (all interfaces).
pub const DEFAULT_BIND: &str = "0.0.0.0";

/// Default data dir (relative to the CWD at launch).
pub const DEFAULT_DATA_DIR: &str = "./team-data";

/// Bundle of paths derived from the data dir.
#[derive(Debug, Clone)]
pub struct DataPaths {
    pub data_dir: PathBuf,
}

impl DataPaths {
    #[must_use]
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    #[must_use]
    pub fn db(&self) -> PathBuf {
        self.data_dir.join(DB_FILENAME)
    }

    #[must_use]
    pub fn cert(&self) -> PathBuf {
        self.data_dir.join(CERT_FILENAME)
    }

    #[must_use]
    pub fn key(&self) -> PathBuf {
        self.data_dir.join(KEY_FILENAME)
    }

    /// `true` if at least one critical artifact already exists.
    #[must_use]
    pub fn already_initialized(&self) -> bool {
        self.db().exists() || self.cert().exists() || self.key().exists()
    }

    /// Creates the directory if it does not exist.
    pub fn ensure_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.data_dir)
    }

    #[must_use]
    pub fn as_path(&self) -> &Path {
        self.data_dir.as_path()
    }
}

/// Runtime key exposed to the operator: (name, default, floor, description).
pub struct RuntimeKey {
    pub key: &'static str,
    pub default: i64,
    pub floor: i64,
    pub description: &'static str,
}

impl RuntimeKey {
    /// Stored integer value, else the default, never below the floor.
    pub fn effective(&self, storage: &Storage) -> AggregatorResult<i64> {
        Ok(storage
            .get_config(self.key)?
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(self.default)
            .max(self.floor))
    }
}

/// k-anonymity threshold of the team analytics (ADR-0015 §2).
pub const K_ANONYMITY_MIN: RuntimeKey = RuntimeKey {
    key: "k_anonymity_min",
    default: 5,
    floor: 3,
    description: "Seuil k-anonymat des analytics équipe (utilisateurs actifs minimum)",
};

/// Retention of the estimations, in days.
pub const RETENTION_DAYS: RuntimeKey = RuntimeKey {
    key: "retention_days",
    default: 730,
    floor: 30,
    description: "Rétention des estimations en jours (purge quotidienne au-delà)",
};

/// Allow-list of operator keys (ADR-0015).
pub const RUNTIME_KEYS: &[RuntimeKey] = &[K_ANONYMITY_MIN, RETENTION_DAYS];
