//! C29.2 integration test: the CLI flow `admin reset-password` + `admin list`
//! through the public API of the crate (`commands::admin::*`).
//!
//! The brief §C29.2 asks: "create an admin, reset, check the hash changed +
//! tokens revoked". The unit tests in `commands/admin.rs` already cover the
//! details; this test checks the public contract and records the ops
//! procedure.

use chrono::{Duration, Utc};
use sobria_team_aggregator::{
    commands::{admin, init},
    config::DataPaths,
    crypto::password,
    storage::{admins as admins_store, tokens, Storage},
};
use tempfile::tempdir;

#[test]
fn reset_password_full_cycle() {
    let dir = tempdir().unwrap();
    let paths = DataPaths::new(dir.path());

    // 1. init creates the database + admin "alice"
    init::run(&paths, "alice", "initial-strong-pw", false).expect("init");

    // 2. Read the initial hash
    let storage = Storage::open(&paths.db()).unwrap();
    let alice_pre = admins_store::find_by_username(storage.connection(), "alice")
        .unwrap()
        .unwrap();
    let initial_hash = alice_pre.password_hash.clone();
    let admin_id = alice_pre.id.clone();

    // 3. Add 3 active admin tokens
    let now = Utc::now();
    for (i, secret) in ["a", "b", "c"].iter().enumerate() {
        let h = password::hash_password(secret).unwrap();
        tokens::insert(
            storage.connection(),
            &format!("t-{i}"),
            None,
            Some(&admin_id),
            &h,
            now,
            now + Duration::days(7),
        )
        .unwrap();
    }
    drop(storage);

    // 4. Reset password
    let out = admin::reset_password(&paths, "alice", "rotated-strong-pw").unwrap();
    assert_eq!(out.username, "alice");
    assert_eq!(out.revoked_tokens, 3);

    // 5. Check the hash changed + verify succeeds with the new password
    let storage = Storage::open(&paths.db()).unwrap();
    let alice_post = admins_store::find_by_username(storage.connection(), "alice")
        .unwrap()
        .unwrap();
    assert_ne!(alice_post.password_hash, initial_hash);
    assert!(password::verify_password(
        &alice_post.password_hash,
        "rotated-strong-pw"
    ));
    assert!(
        !password::verify_password(&alice_post.password_hash, "initial-strong-pw"),
        "l'ancien password ne doit plus marcher"
    );

    // 6. Every token is revoked
    for i in 0..3 {
        let t = tokens::find_by_id(storage.connection(), &format!("t-{i}"))
            .unwrap()
            .unwrap();
        assert!(t.revoked_at.is_some(), "token t-{i} doit être révoqué");
    }
}

#[test]
fn list_admins_after_init_returns_seed_admin() {
    let dir = tempdir().unwrap();
    let paths = DataPaths::new(dir.path());
    init::run(&paths, "ops", "first-strong-pw", false).expect("init");

    let list = admin::list_admins(&paths).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].username, "ops");
    assert!(list[0].last_login_at.is_none());
}
