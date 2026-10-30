//! Socket forward integration tests (C27 patch 2, v0.6.0): `try_forward_to`
//! round trip against `common::fake_app` over a Unix socket.
//!
//! On Windows only the missing-pipe case is tested: the named-pipe forward has
//! no round-trip test (see `forward_wire.rs` and SOB-96).

mod common;

use sobria_bridge::{try_forward_to, BridgeRequest};

#[cfg(unix)]
mod unix {
    use super::*;
    use crate::common::fake_app::{serve, Reply};
    use serde_json::json;
    use std::os::unix::net::UnixListener;
    use tempfile::TempDir;

    #[test]
    fn forward_ping_returns_pong() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("sobria-bridge-test.sock");
        let handle = serve(
            UnixListener::bind(&path).expect("bind"),
            vec![Reply::Json(
                json!({"reqId": "ping-1", "ok": true, "pong": true}),
            )],
        );

        let req = BridgeRequest::Ping {
            req_id: "ping-1".into(),
        };
        let resp = try_forward_to(&path, &req).expect("forward ok");
        assert_eq!(resp.req_id, "ping-1");
        assert!(resp.ok);
        assert_eq!(resp.pong, Some(true));
        assert_eq!(
            handle.join().expect("server done"),
            [json!({"type": "ping", "reqId": "ping-1"})]
        );
    }

    #[test]
    fn forward_pair_returns_serialized_request() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("sobria-bridge-test.sock");
        let handle = serve(
            UnixListener::bind(&path).expect("bind"),
            vec![Reply::Json(
                json!({"reqId": "pair-1", "ok": true, "pong": false}),
            )],
        );

        let req = BridgeRequest::Pair {
            req_id: "pair-1".into(),
            code: "123456".into(),
        };
        let resp = try_forward_to(&path, &req).expect("forward ok");
        assert_eq!(resp.req_id, "pair-1");
        assert!(resp.ok);
        assert_eq!(resp.pong, Some(false));
        assert_eq!(
            handle.join().expect("server done"),
            [json!({"type": "pair", "reqId": "pair-1", "code": "123456"})]
        );
    }

    #[test]
    fn forward_to_nonexistent_socket_errors_fast() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("never-bound.sock");
        let req = BridgeRequest::Ping {
            req_id: "ghost".into(),
        };
        let result = try_forward_to(&path, &req);
        assert!(result.is_err(), "should fail when socket doesn't exist");
    }
}

#[cfg(windows)]
mod windows_pipe {
    use super::*;

    /// Opening a missing pipe fails at once, without waiting for a timeout.
    #[test]
    fn forward_to_nonexistent_pipe_errors_fast() {
        // Unique name to avoid clashing with a running sobria-bridge.
        let pipe = format!(r"\\.\pipe\sobria-bridge-test-{}", std::process::id());
        let req = BridgeRequest::Ping {
            req_id: "ghost-win".into(),
        };
        let result = try_forward_to(std::path::Path::new(&pipe), &req);
        assert!(result.is_err(), "should fail when pipe doesn't exist");
    }
}

#[test]
fn serialization_of_bridge_request_round_trips() {
    let req = BridgeRequest::Estimate {
        req_id: "rt-1".into(),
        secret: "0123abcdef".repeat(6),
        payload: serde_json::json!({ "k": 1 }),
    };
    let bytes = serde_json::to_vec(&req).unwrap();
    let parsed: BridgeRequest = serde_json::from_slice(&bytes).unwrap();
    match parsed {
        BridgeRequest::Estimate {
            req_id,
            secret,
            payload,
        } => {
            assert_eq!(req_id, "rt-1");
            assert!(secret.starts_with("0123"));
            assert_eq!(payload["k"], 1);
        },
        _ => panic!("expected Estimate"),
    }
}
