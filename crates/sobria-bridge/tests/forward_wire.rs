//! Characterization of the forward to the desktop app (SOB-11): exact bytes
//! sent by `try_forward_to`, relay of pairing replies, and which transport
//! failures return `Err` (the binary falls back to `handle_request` on `Err`).
//!
//! Extends `socket_forward.rs`, which only covers the happy path. Pairing
//! replies reuse the real texts of `sobria-app/src/bridge_server.rs`.
//!
//! Unix only: on Windows the forward uses a named pipe that cannot be faked
//! without an extra dependency. That Windows forward also has no read or write
//! timeout, so a frozen app blocks it (SOB-96).
#![cfg(unix)]

mod common;

use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use common::fake_app::{serve, Reply};
use common::{err_text, frame, MAX_MESSAGE_BYTES};
use serde_json::{json, Value};
use sobria_bridge::{try_forward_to, BridgeRequest, BridgeResponse};
use tempfile::TempDir;

fn bind() -> (TempDir, PathBuf, UnixListener) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("b.sock");
    let listener = UnixListener::bind(&path).unwrap();
    (dir, path, listener)
}

/// Returns the forward result and the request as the app received it.
fn exchange(req: &BridgeRequest, reply: Reply) -> (anyhow::Result<BridgeResponse>, Value) {
    let (_dir, path, listener) = bind();
    let server = serve(listener, vec![reply]);
    let result = try_forward_to(&path, req);
    let mut received = server.join().expect("fake server");
    (result, received.remove(0))
}

fn ping() -> BridgeRequest {
    BridgeRequest::Ping { req_id: "a".into() }
}

fn ack(req_id: &str) -> Reply {
    Reply::Json(json!({"reqId": req_id, "ok": true}))
}

#[test]
fn each_message_type_reaches_the_app_as_tagged_camel_case_json() {
    let cases = [
        (
            BridgeRequest::Ping {
                req_id: "r1".into(),
            },
            json!({"type": "ping", "reqId": "r1"}),
        ),
        (
            BridgeRequest::Pair {
                req_id: "r2".into(),
                code: "123456".into(),
            },
            json!({"type": "pair", "reqId": "r2", "code": "123456"}),
        ),
        (
            BridgeRequest::Estimate {
                req_id: "r3".into(),
                secret: "s".into(),
                payload: json!({"estimate": {"tokensIn": 3}}),
            },
            json!({"type": "estimate", "reqId": "r3", "secret": "s",
                   "payload": {"estimate": {"tokensIn": 3}}}),
        ),
        (
            BridgeRequest::Revoke {
                req_id: "r4".into(),
                secret: "s".into(),
            },
            json!({"type": "revoke", "reqId": "r4", "secret": "s"}),
        ),
    ];
    for (req, expected) in cases {
        let id = req.req_id().to_owned();
        let (result, received) = exchange(&req, ack(&id));
        assert!(result.expect("forward").ok, "{id}");
        assert_eq!(received, expected, "{id}");
    }
}

#[test]
fn pairing_success_reply_is_relayed_field_for_field() {
    let secret = "ab".repeat(32);
    let req = BridgeRequest::Pair {
        req_id: "p".into(),
        code: "123456".into(),
    };
    let reply = Reply::Json(json!({
        "reqId": "p", "ok": true, "secret": secret,
        "pairingId": "0b9a-uuid", "fingerprint": "sobria-bridge-native-messaging"
    }));
    let (result, _) = exchange(&req, reply);
    let resp = result.expect("forward");
    assert!(resp.ok);
    assert_eq!(resp.secret.as_deref(), Some(secret.as_str()));
    assert_eq!(resp.pairing_id.as_deref(), Some("0b9a-uuid"));
    assert_eq!(
        resp.fingerprint.as_deref(),
        Some("sobria-bridge-native-messaging")
    );
    assert!(resp.error.is_none());
}

#[test]
fn pairing_refusals_from_the_app_are_returned_as_ok_not_as_errors() {
    // `main.rs` only falls back to `handle_request` on `Err`, so an app refusal
    // (`ok: false`) is final and reaches the extension unchanged.
    let refusals = [
        "code invalide ou expiré",
        "code à 6 chiffres requis",
        "aucun code de pairing en attente — génère-en un depuis Sobr.ia /parametres",
    ];
    for message in refusals {
        let req = BridgeRequest::Pair {
            req_id: "p".into(),
            code: "000000".into(),
        };
        let reply = Reply::Json(json!({"reqId": "p", "ok": false, "error": message}));
        let resp = exchange(&req, reply).0.expect("forward");
        assert!(!resp.ok, "{message}");
        assert_eq!(resp.error.as_deref(), Some(message));
        assert!(resp.secret.is_none() && resp.pairing_id.is_none());
    }
}

#[test]
fn reply_request_id_is_not_checked_against_the_request() {
    let resp = exchange(&ping(), ack("zzz")).0.expect("forward");
    assert_eq!(resp.req_id, "zzz");
}

#[test]
fn reply_that_is_not_a_bridge_response_is_an_error() {
    for reply in [
        Reply::Raw(frame(b"not json")),
        Reply::Json(json!({"reqId": "a"})),
        Reply::Json(json!([])),
    ] {
        let message = err_text(exchange(&ping(), reply).0);
        assert!(message.contains("decode BridgeResponse"), "{message}");
    }
}

#[test]
fn reply_over_one_megabyte_is_refused() {
    let len = MAX_MESSAGE_BYTES + 1;
    let message = err_text(exchange(&ping(), Reply::Raw(len.to_le_bytes().to_vec())).0);
    assert!(
        message.contains(&format!("response too large: {len}")),
        "{message}"
    );
}

#[test]
fn reply_of_exactly_one_megabyte_passes_the_size_gate() {
    // Strict threshold (`>`): exactly 1 MiB passes, then fails for lack of a body.
    let reply = Reply::Raw(MAX_MESSAGE_BYTES.to_le_bytes().to_vec());
    let message = err_text(exchange(&ping(), reply).0);
    assert!(message.contains("read response payload"), "{message}");
    assert!(!message.contains("too large"), "{message}");
}

#[test]
fn truncated_reply_payload_is_an_error() {
    let mut bytes = 50u32.to_le_bytes().to_vec();
    bytes.extend_from_slice(b"{\"r");
    let message = err_text(exchange(&ping(), Reply::Raw(bytes)).0);
    assert!(message.contains("read response payload"), "{message}");
}

#[test]
fn app_that_closes_without_replying_is_an_error() {
    let message = err_text(exchange(&ping(), Reply::Close).0);
    assert!(message.contains("read response length"), "{message}");
}

#[test]
fn silent_app_makes_the_forward_fail_after_about_two_seconds() {
    let (_dir, path, listener) = bind();
    let (release, hold) = mpsc::channel::<()>();
    let server = serve(listener, vec![Reply::Hold(hold)]);

    let started = Instant::now();
    let result = try_forward_to(&path, &ping());
    let elapsed = started.elapsed();

    drop(release);
    server.join().expect("fake server");
    let message = err_text(result);
    assert!(message.contains("read response length"), "{message}");
    assert!(
        // The fake server releases itself after 10 s: without a bridge read
        // timeout, the measured time would be about 10 s.
        elapsed >= Duration::from_millis(1500) && elapsed < Duration::from_secs(9),
        "observed delay: {elapsed:?}"
    );
}

#[test]
fn stale_socket_file_is_a_connection_error() {
    // Real case after an app crash: the socket file remains, nobody listens.
    let (_dir, path, listener) = bind();
    drop(listener);
    assert!(path.exists());
    let message = err_text(try_forward_to(&path, &ping()));
    assert!(message.contains("connect"), "{message}");
}

#[test]
fn path_that_is_not_a_socket_is_a_connection_error() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("plain-file");
    std::fs::write(&path, b"").unwrap();
    let message = err_text(try_forward_to(&path, &ping()));
    assert!(message.contains("connect"), "{message}");
}
