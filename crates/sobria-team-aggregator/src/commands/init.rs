//! `sobria-team-aggregator init` command — prepares a blank data dir:
//!
//! 1. SQLite `team.sqlite` + migration v1.
//! 2. JWT signing key (32 bytes OS RNG, hex) stored in `config`.
//! 3. Self-signed TLS certificate (rcgen + ring) + private key (chmod 600 on Unix).
//! 4. Initial admin (Argon2id PHC, configurable username).
//!
//! Idempotent only with `--force`: refuses to reset an already populated
//! data dir so team data is not wiped by mistake.

use anyhow::{Context, Result};
use chrono::Utc;
use ulid::Ulid;

use crate::config::DataPaths;
use crate::crypto::{password, secret, tls};
use crate::error::AggregatorError;
use crate::storage::{admins, schema, Storage};

/// `config` KV keys used by `init`.
pub const CFG_JWT_SIGNING_KEY: &str = "jwt_signing_key";
pub const CFG_SCHEMA_INSTALLED_AT: &str = "schema_installed_at";
pub const CFG_SCHEMA_VERSION: &str = "schema_version";

/// Result returned to the CLI for the post-init summary.
#[derive(Debug)]
pub struct InitOutcome {
    pub admin_id: String,
    pub admin_username: String,
    pub cert_path: std::path::PathBuf,
    pub key_path: std::path::PathBuf,
    pub db_path: std::path::PathBuf,
    pub hostname: Option<String>,
}

/// Runs the full init.
///
/// `admin_password` is used in clear once to compute the Argon2id PHC;
/// it does not linger in memory afterwards.
pub fn run(
    paths: &DataPaths,
    admin_username: &str,
    admin_password: &str,
    force: bool,
) -> Result<InitOutcome> {
    if paths.already_initialized() && !force {
        return Err(
            AggregatorError::AlreadyInitialized(paths.as_path().display().to_string()).into(),
        );
    }

    paths.ensure_dir().context("ensure data dir")?;

    // 1. DB + schema v1
    let storage = Storage::open(&paths.db()).context("open team.sqlite")?;

    // 2. JWT signing key + metadata
    let jwt_key = secret::jwt_signing_key();
    storage.set_config(CFG_JWT_SIGNING_KEY, &jwt_key)?;
    storage.set_config(CFG_SCHEMA_INSTALLED_AT, &Utc::now().to_rfc3339())?;
    storage.set_config(CFG_SCHEMA_VERSION, &schema::SCHEMA_VERSION.to_string())?;

    // 3. Self-signed certificate (SANs: localhost, 127.0.0.1, ::1, OS hostname)
    let hostname = tls::os_hostname();
    let sans = tls::DEFAULT_SANS.map(String::from);
    let bundle = tls::generate_self_signed(&sans, hostname.as_deref())
        .context("generate self-signed cert")?;
    tls::write_cert_files(&paths.cert(), &paths.key(), &bundle)
        .context("write cert.pem / key.pem")?;

    // 4. Initial admin
    let admin_id = Ulid::new().to_string();
    let admin_hash = password::hash_password(admin_password).context("hash admin password")?;
    admins::insert(
        storage.connection(),
        &admin_id,
        admin_username,
        &admin_hash,
        Utc::now(),
    )
    .context("insert admin")?;

    Ok(InitOutcome {
        admin_id,
        admin_username: admin_username.to_string(),
        cert_path: paths.cert(),
        key_path: paths.key(),
        db_path: paths.db(),
        hostname,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn init_populates_data_dir() {
        let dir = tempdir().unwrap();
        let paths = DataPaths::new(dir.path());
        let outcome = run(&paths, "admin", "init-password", false).unwrap();

        assert!(paths.db().exists(), "team.sqlite manquant");
        assert!(paths.cert().exists(), "cert.pem manquant");
        assert!(paths.key().exists(), "key.pem manquant");
        assert_eq!(outcome.admin_username, "admin");
        assert!(!outcome.admin_id.is_empty());

        // The admin is in the DB and the password verifies.
        let storage = Storage::open(&paths.db()).unwrap();
        let row = admins::find_by_username(storage.connection(), "admin")
            .unwrap()
            .unwrap();
        assert!(password::verify_password(
            &row.password_hash,
            "init-password"
        ));
        assert!(!password::verify_password(&row.password_hash, "wrong"));

        // The JWT signing key is set.
        let jwt = storage.get_config(CFG_JWT_SIGNING_KEY).unwrap().unwrap();
        assert_eq!(jwt.len(), 64);
    }

    #[test]
    fn init_refuses_existing_data_dir_without_force() {
        let dir = tempdir().unwrap();
        let paths = DataPaths::new(dir.path());
        run(&paths, "admin", "p1", false).unwrap();
        let err = run(&paths, "admin", "p2", false).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("data dir déjà initialisé"), "msg = {msg}");
    }
}
