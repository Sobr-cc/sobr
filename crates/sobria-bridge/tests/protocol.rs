//! Native Messaging length-prefixed JSON round trip, and the append-only spool
//! with rotation.

mod common;

use std::io::Cursor;

use common::{frame, parse_frames};
use serde_json::json;
use sobria_bridge::{
    append_to_spool_at, handle_request, read_message, write_message, BridgeRequest, BridgeResponse,
    SPOOL_MAX_BYTES,
};

#[test]
fn read_ping_request() {
    let req_json = r#"{"type":"ping","reqId":"a1"}"#;
    let buf = frame(req_json.as_bytes());
    let mut cursor = Cursor::new(buf);
    let req = read_message(&mut cursor).unwrap().unwrap();
    assert!(matches!(req, BridgeRequest::Ping { .. }));
    assert_eq!(req.req_id(), "a1");
}

#[test]
fn read_estimate_request() {
    let req_json = r#"{"type":"estimate","reqId":"e1","secret":"s","payload":{"k":42}}"#;
    let buf = frame(req_json.as_bytes());
    let mut cursor = Cursor::new(buf);
    let req = read_message(&mut cursor).unwrap().unwrap();
    match req {
        BridgeRequest::Estimate {
            req_id,
            secret,
            payload,
        } => {
            assert_eq!(req_id, "e1");
            assert_eq!(secret, "s");
            assert_eq!(payload["k"], 42);
        },
        _ => panic!("expected Estimate"),
    }
}

#[test]
fn eof_returns_none() {
    let empty: Vec<u8> = Vec::new();
    let mut cursor = Cursor::new(empty);
    assert!(read_message(&mut cursor).unwrap().is_none());
}

#[test]
fn ping_handle_writes_pong() {
    let resp = handle_request(BridgeRequest::Ping { req_id: "x".into() });
    let mut buf: Vec<u8> = Vec::new();
    write_message(&mut buf, &resp).unwrap();
    let [parsed] = parse_frames(&buf).try_into().expect("one frame");
    assert_eq!(parsed["reqId"], "x");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["pong"], true);
}

#[test]
fn pair_handle_returns_error() {
    let resp = handle_request(BridgeRequest::Pair {
        req_id: "p".into(),
        code: "000000".into(),
    });
    assert!(!resp.ok);
    assert!(resp.error.is_some());
}

#[test]
fn estimate_handle_writes_to_spool() {
    // Calls `append_to_spool_at` with a temp dir: `handle_request` would write
    // to the real spool.
    let temp = tempfile::tempdir().unwrap();
    let spool = temp.path().join("incoming.jsonl");
    append_to_spool_at(&spool, &json!({"foo": "bar", "n": 1}), SPOOL_MAX_BYTES).unwrap();
    append_to_spool_at(&spool, &json!({"foo": "baz", "n": 2}), SPOOL_MAX_BYTES).unwrap();
    let content = std::fs::read_to_string(&spool).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 2);
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["n"], 1);
}

#[test]
fn append_to_spool_rotates_at_threshold() {
    let temp = tempfile::tempdir().unwrap();
    let spool = temp.path().join("incoming.jsonl");
    // The first write (~8 bytes) exceeds the 5-byte threshold, so the second rotates.
    append_to_spool_at(&spool, &json!({"a": 1}), 5).unwrap();
    append_to_spool_at(&spool, &json!({"b": 2}), 5).unwrap();
    let bak = spool.with_extension("jsonl.bak");
    assert!(bak.exists(), ".bak must exist after rotation");
    let content = std::fs::read_to_string(&spool).unwrap();
    assert!(content.contains("\"b\":2"));
    assert!(!content.contains("\"a\":1"));
}

#[test]
fn response_serialization_includes_secret_when_set() {
    let mut resp = BridgeResponse::ok("p1");
    resp.secret = Some("xx".into());
    resp.pairing_id = Some("uuid-1".into());
    resp.fingerprint = Some("chrome-mac".into());
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("\"secret\":\"xx\""));
    assert!(json.contains("\"pairingId\":\"uuid-1\""));
    assert!(json.contains("\"fingerprint\":\"chrome-mac\""));
}
