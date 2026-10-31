//! C29.3 integration test: `regen_self_signed` on the data dir created
//! by `init`. Checks: backups written, new cert different, fingerprint
//! consistent.

use sobria_team_aggregator::{commands::init, config::DataPaths, crypto::tls};
use tempfile::tempdir;

#[test]
fn regen_after_init_replaces_cert_and_backs_up_old() {
    let dir = tempdir().unwrap();
    let paths = DataPaths::new(dir.path());
    init::run(&paths, "admin", "init-strong-pw", false).expect("init");
    assert!(paths.cert().exists());
    assert!(paths.key().exists());

    let cert_before = std::fs::read_to_string(paths.cert()).unwrap();
    let key_before = std::fs::read_to_string(paths.key()).unwrap();
    let fp_before = tls::cert_fingerprint_sha256(&cert_before);

    // Wait 1 second to ensure the unix_ts of the backup differs from any
    // file created in init (defensive: in practice the UNIX seconds
    // already differ).
    std::thread::sleep(std::time::Duration::from_secs(1));
    let outcome = tls::regen_self_signed(&paths.cert(), &paths.key()).expect("regen");

    // Backups written, holding the old content
    assert!(outcome.cert_backup_path.exists());
    assert!(outcome.key_backup_path.exists());
    let cert_bak = std::fs::read_to_string(&outcome.cert_backup_path).unwrap();
    let key_bak = std::fs::read_to_string(&outcome.key_backup_path).unwrap();
    assert_eq!(cert_bak, cert_before);
    assert_eq!(key_bak, key_before);

    // New cert different + fingerprint consistent
    let cert_after = std::fs::read_to_string(paths.cert()).unwrap();
    let fp_after = tls::cert_fingerprint_sha256(&cert_after);
    assert_ne!(cert_before, cert_after, "cert non régénéré");
    assert_ne!(fp_before, fp_after, "fingerprint identique malgré regen");
    assert_eq!(outcome.new_fingerprint, fp_after);

    // The new cert still carries the default CN (`localhost` or the OS
    // hostname; not checked strictly because it is not deterministic in CI).
    assert!(cert_after.contains("BEGIN CERTIFICATE"));
}

#[test]
fn regen_without_init_fails() {
    let dir = tempdir().unwrap();
    let paths = DataPaths::new(dir.path());
    paths.ensure_dir().unwrap();
    let err = tls::regen_self_signed(&paths.cert(), &paths.key());
    assert!(err.is_err());
}
