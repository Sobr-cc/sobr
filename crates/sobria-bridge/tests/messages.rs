//! Characterization of the bridge message layer (SOB-11): accepted and refused
//! requests, stdin/stdout framing, and `handle_request` replies when the
//! desktop app is unreachable.
//!
//! These tests pin current behavior, quirks included. A failing test flags a
//! behavior change to decide on, not necessarily a regression.
//!
//! `Estimate` is not sent through `handle_request` here because it writes to
//! the real `~/.sobria/spool`; `bridge_process.rs` covers it with an isolated
//! `HOME`.

mod common;

use std::io::Cursor;

use common::{
    err_text, frame, parse_frames, MAX_MESSAGE_BYTES, PAIR_OFFLINE_REFUSAL, REVOKE_OFFLINE_REFUSAL,
};
use serde_json::{json, Value};
use sobria_bridge::{
    append_to_spool_at, default_socket_path, handle_request, read_message, short_hash,
    write_message, BridgeRequest, BridgeResponse, NATIVE_HOST_NAME, SPOOL_MAX_BYTES,
};

fn read_bytes(bytes: Vec<u8>) -> anyhow::Result<Option<BridgeRequest>> {
    read_message(&mut Cursor::new(bytes))
}

fn read_json(body: &str) -> anyhow::Result<Option<BridgeRequest>> {
    read_bytes(frame(body.as_bytes()))
}

fn accepted(body: &str) -> BridgeRequest {
    read_json(body)
        .unwrap_or_else(|e| panic!("message refused: {e:#}\n{body}"))
        .expect("empty stream")
}

fn refused(bytes: Vec<u8>) -> String {
    err_text(read_bytes(bytes))
}

fn refusal(body: &str) -> String {
    refused(frame(body.as_bytes()))
}

fn wire(resp: &BridgeResponse) -> Value {
    let mut buf = Vec::new();
    write_message(&mut buf, resp).unwrap();
    let mut frames = parse_frames(&buf);
    assert_eq!(frames.len(), 1, "one response = one frame");
    frames.remove(0)
}

// ───────────────────────── Accepted messages ─────────────────────────

#[test]
fn ping_message_is_accepted() {
    match accepted(r#"{"type":"ping","reqId":"r1"}"#) {
        BridgeRequest::Ping { req_id } => assert_eq!(req_id, "r1"),
        other => panic!("expected Ping, got {other:?}"),
    }
}

#[test]
fn pair_message_is_accepted() {
    match accepted(r#"{"type":"pair","reqId":"r2","code":"123456"}"#) {
        BridgeRequest::Pair { req_id, code } => {
            assert_eq!(req_id, "r2");
            assert_eq!(code, "123456");
        },
        other => panic!("expected Pair, got {other:?}"),
    }
}

#[test]
fn estimate_message_is_accepted() {
    let body =
        r#"{"type":"estimate","reqId":"r3","secret":"s3","payload":{"estimate":{"tokensIn":7}}}"#;
    match accepted(body) {
        BridgeRequest::Estimate {
            req_id,
            secret,
            payload,
        } => {
            assert_eq!(req_id, "r3");
            assert_eq!(secret, "s3");
            assert_eq!(payload, json!({"estimate": {"tokensIn": 7}}));
        },
        other => panic!("expected Estimate, got {other:?}"),
    }
}

#[test]
fn revoke_message_is_accepted() {
    match accepted(r#"{"type":"revoke","reqId":"r4","secret":"s4"}"#) {
        BridgeRequest::Revoke { req_id, secret } => {
            assert_eq!(req_id, "r4");
            assert_eq!(secret, "s4");
        },
        other => panic!("expected Revoke, got {other:?}"),
    }
}

#[test]
fn req_id_accessor_returns_the_id_of_every_message_type() {
    for (body, id) in [
        (r#"{"type":"ping","reqId":"a"}"#, "a"),
        (r#"{"type":"pair","reqId":"b","code":"1"}"#, "b"),
        (
            r#"{"type":"estimate","reqId":"c","secret":"s","payload":1}"#,
            "c",
        ),
        (r#"{"type":"revoke","reqId":"d","secret":"s"}"#, "d"),
    ] {
        assert_eq!(accepted(body).req_id(), id, "{body}");
    }
}

// ─────────────── Accepted quirks (current behavior) ───────────────

#[test]
fn unknown_extra_fields_are_ignored() {
    let req = accepted(r#"{"type":"ping","reqId":"r","extra":{"a":1},"v":2}"#);
    assert!(matches!(req, BridgeRequest::Ping { .. }));
}

#[test]
fn empty_req_id_is_accepted() {
    assert_eq!(accepted(r#"{"type":"ping","reqId":""}"#).req_id(), "");
}

#[test]
fn pair_code_is_opaque_to_the_bridge() {
    // The app (`sobria-app/src/pairing.rs`) validates the code (6 digits,
    // 5 min TTL, single use); the bridge passes any code through.
    for code in [
        "123456",
        "000000",
        "abc",
        "",
        "1234567",
        " 123456 ",
        "１２３４５６",
    ] {
        let body = json!({"type": "pair", "reqId": "r", "code": code}).to_string();
        match accepted(&body) {
            BridgeRequest::Pair { code: kept, .. } => assert_eq!(kept, code),
            other => panic!("expected Pair, got {other:?}"),
        }
    }
}

#[test]
fn estimate_payload_accepts_any_json_value() {
    for payload in [json!(null), json!(42), json!("s"), json!([1, 2]), json!({})] {
        let body = json!({"type": "estimate", "reqId": "r", "secret": "s", "payload": payload})
            .to_string();
        match accepted(&body) {
            BridgeRequest::Estimate { payload: kept, .. } => assert_eq!(kept, payload),
            other => panic!("expected Estimate, got {other:?}"),
        }
    }
}

// ───────────────────────── Refused messages ─────────────────────────

#[test]
fn unknown_message_types_are_refused() {
    // Only lowercase `ping`, `pair`, `estimate` and `revoke` exist.
    for kind in ["hello", "Ping", "PING", "", "pair ", "estimates", "pairing"] {
        let body = json!({"type": kind, "reqId": "r"}).to_string();
        let message = refusal(&body);
        assert!(message.contains("decode JSON"), "{kind:?} → {message}");
        assert!(
            message.contains(&format!("unknown variant `{kind}`")),
            "{kind:?} → {message}"
        );
    }
}

#[test]
fn messages_missing_a_required_field_are_refused() {
    let cases = [
        (r#"{"reqId":"r"}"#, "type"),
        (r#"{"type":"ping"}"#, "reqId"),
        (r#"{"type":"ping","req_id":"r"}"#, "reqId"),
        (r#"{"type":"pair","reqId":"r"}"#, "code"),
        (r#"{"type":"pair","code":"123456"}"#, "reqId"),
        (r#"{"type":"estimate","reqId":"r","payload":{}}"#, "secret"),
        (r#"{"type":"estimate","reqId":"r","secret":"s"}"#, "payload"),
        (r#"{"type":"revoke","reqId":"r"}"#, "secret"),
    ];
    for (body, field) in cases {
        let message = refusal(body);
        assert!(
            message.contains(&format!("missing field `{field}`")),
            "{body} → {message}"
        );
    }
}

#[test]
fn fields_with_the_wrong_json_type_are_refused() {
    // A numeric pairing code is refused: "000123" would lose its leading
    // zeros, so the extension must send a string.
    let cases = [
        r#"{"type":"ping","reqId":7}"#,
        r#"{"type":"pair","reqId":"r","code":123456}"#,
        r#"{"type":"pair","reqId":"r","code":null}"#,
        r#"{"type":"estimate","reqId":"r","secret":42,"payload":{}}"#,
        r#"{"type":"revoke","reqId":"r","secret":null}"#,
        r#"{"type":1,"reqId":"r"}"#,
    ];
    for body in cases {
        let message = refusal(body);
        assert!(message.contains("decode JSON"), "{body} → {message}");
    }
}

#[test]
fn payloads_that_are_not_a_json_object_are_refused() {
    for body in ["[]", "\"ping\"", "null", "42", "{", "not json", ""] {
        let message = refusal(body);
        assert!(
            message.contains(&format!("decode JSON ({} bytes)", body.len())),
            "{body:?} → {message}"
        );
    }
}

#[test]
fn payload_that_is_not_utf8_is_refused() {
    let message = refused(frame(&[0xff, 0xfe, 0x00]));
    assert!(message.contains("decode JSON (3 bytes)"), "{message}");
}

// ───────────────────────── Message framing ─────────────────────────

#[test]
fn length_prefix_is_u32_little_endian() {
    let body = r#"{"type":"ping","reqId":"a"}"#;
    assert_eq!(body.len(), 27);
    let mut bytes = vec![27, 0, 0, 0];
    bytes.extend_from_slice(body.as_bytes());
    let req = read_bytes(bytes).unwrap().unwrap();
    assert_eq!(req.req_id(), "a");
}

#[test]
fn big_endian_length_prefix_is_not_understood() {
    let body = r#"{"type":"ping","reqId":"a"}"#;
    let mut bytes = vec![0, 0, 0, 27];
    bytes.extend_from_slice(body.as_bytes());
    let message = refused(bytes);
    assert!(message.contains("exceeds 1 MB"), "{message}");
}

#[test]
fn empty_stream_is_a_clean_end() {
    assert!(read_bytes(Vec::new()).unwrap().is_none());
}

#[test]
fn truncated_length_prefix_is_treated_as_a_clean_end() {
    // Quirk: 1 to 3 prefix bytes then EOF is not an error; the message is
    // dropped silently like a normal end of stream.
    for n in 1..=3 {
        assert!(read_bytes(vec![27; n]).unwrap().is_none(), "{n} byte(s)");
    }
}

#[test]
fn truncated_payload_is_an_error() {
    let mut bytes = 100u32.to_le_bytes().to_vec();
    bytes.extend_from_slice(b"{\"t");
    let message = refused(bytes);
    assert!(
        message.contains("read native messaging payload"),
        "{message}"
    );
}

#[test]
fn message_over_one_megabyte_is_refused_before_its_payload_is_read() {
    // Only the header is given, so the refusal does not depend on reading the
    // body and no buffer of that size is allocated.
    for len in [MAX_MESSAGE_BYTES + 1, u32::MAX] {
        let message = refused(len.to_le_bytes().to_vec());
        assert!(
            message.contains(&format!("message length exceeds 1 MB: {len}")),
            "{message}"
        );
    }
}

#[test]
fn message_of_exactly_one_megabyte_passes_the_size_gate() {
    // Strict threshold (`>`): exactly 1 MiB passes, then fails for lack of a body.
    let message = refused(MAX_MESSAGE_BYTES.to_le_bytes().to_vec());
    assert!(
        message.contains("read native messaging payload"),
        "{message}"
    );
    assert!(!message.contains("exceeds"), "{message}");
}

#[test]
fn message_of_exactly_one_megabyte_is_accepted_when_complete() {
    let prefix = r#"{"type":"estimate","reqId":"big","secret":"s","payload":""#;
    let suffix = r#""}"#;
    let max = MAX_MESSAGE_BYTES as usize;
    let filler = max - prefix.len() - suffix.len();
    let body = format!("{prefix}{}{suffix}", "x".repeat(filler));
    assert_eq!(body.len(), max);
    assert_eq!(accepted(&body).req_id(), "big");
}

#[test]
fn messages_are_read_in_order_until_the_end_of_the_stream() {
    let mut bytes = frame(br#"{"type":"ping","reqId":"1"}"#);
    bytes.extend(frame(br#"{"type":"revoke","reqId":"2","secret":"s"}"#));
    bytes.extend(frame(br#"{"type":"ping","reqId":"3"}"#));
    let mut cursor = Cursor::new(bytes);
    for expected in ["1", "2", "3"] {
        let req = read_message(&mut cursor).unwrap().unwrap();
        assert_eq!(req.req_id(), expected);
    }
    assert!(read_message(&mut cursor).unwrap().is_none());
}

#[test]
fn reading_can_continue_after_an_undecodable_message() {
    // `read_message` alone consumes the whole refused body, so the next message
    // stays readable. The binary stops at the first failure instead (see
    // `undecodable_message_ends_the_session_without_a_reply`).
    let mut bytes = frame(br#"{"type":"nope","reqId":"x"}"#);
    bytes.extend(frame(br#"{"type":"ping","reqId":"next"}"#));
    let mut cursor = Cursor::new(bytes);
    assert!(read_message(&mut cursor).is_err());
    assert_eq!(read_message(&mut cursor).unwrap().unwrap().req_id(), "next");
}

// ─────────── Local `handle_request` replies (app unreachable) ───────────

#[test]
fn ping_is_answered_locally_with_a_pong() {
    let resp = handle_request(BridgeRequest::Ping { req_id: "p".into() });
    assert_eq!(wire(&resp), json!({"reqId": "p", "ok": true, "pong": true}));
}

#[test]
fn pair_is_refused_offline_whatever_the_code() {
    for code in ["123456", "000000", "abc", "", "1234567"] {
        let resp = handle_request(BridgeRequest::Pair {
            req_id: "p".into(),
            code: code.into(),
        });
        assert_eq!(
            wire(&resp),
            json!({"reqId": "p", "ok": false, "error": PAIR_OFFLINE_REFUSAL}),
            "code {code:?}"
        );
    }
}

#[test]
fn revoke_is_refused_offline_whatever_the_secret() {
    for secret in ["", "s", &"0123abcdef".repeat(6)] {
        let resp = handle_request(BridgeRequest::Revoke {
            req_id: "v".into(),
            secret: secret.into(),
        });
        assert_eq!(
            wire(&resp),
            json!({"reqId": "v", "ok": false, "error": REVOKE_OFFLINE_REFUSAL}),
            "secret {secret:?}"
        );
    }
}

#[test]
fn local_answers_echo_the_request_id() {
    let id = "req-é-42";
    let answers = [
        handle_request(BridgeRequest::Ping { req_id: id.into() }),
        handle_request(BridgeRequest::Pair {
            req_id: id.into(),
            code: "123456".into(),
        }),
        handle_request(BridgeRequest::Revoke {
            req_id: id.into(),
            secret: "s".into(),
        }),
    ];
    for resp in answers {
        assert_eq!(resp.req_id, id);
    }
}

// ───────────────────────── Response shape ─────────────────────────

#[test]
fn ok_response_carries_only_req_id_and_ok() {
    assert_eq!(
        wire(&BridgeResponse::ok("x")),
        json!({"reqId": "x", "ok": true})
    );
}

#[test]
fn error_response_carries_the_message() {
    assert_eq!(
        wire(&BridgeResponse::err("x", "boom")),
        json!({"reqId": "x", "ok": false, "error": "boom"})
    );
}

#[test]
fn pairing_success_response_uses_camel_case_keys() {
    let mut resp = BridgeResponse::ok("p");
    resp.secret = Some("ab12".into());
    resp.pairing_id = Some("id-1".into());
    resp.fingerprint = Some("fp".into());
    assert_eq!(
        wire(&resp),
        json!({
            "reqId": "p", "ok": true,
            "secret": "ab12", "pairingId": "id-1", "fingerprint": "fp"
        })
    );
}

#[test]
fn response_frame_starts_with_the_little_endian_body_length() {
    let mut buf = Vec::new();
    write_message(&mut buf, &BridgeResponse::ok("x")).unwrap();
    let declared = u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize;
    assert_eq!(declared, buf.len() - 4);
    let body: Value = serde_json::from_slice(&buf[4..]).unwrap();
    assert_eq!(body, json!({"reqId": "x", "ok": true}));
}

#[test]
fn response_decoding_requires_req_id_and_ok() {
    assert!(serde_json::from_value::<BridgeResponse>(json!({"ok": true})).is_err());
    assert!(serde_json::from_value::<BridgeResponse>(json!({"reqId": "x"})).is_err());
    let minimal: BridgeResponse =
        serde_json::from_value(json!({"reqId": "x", "ok": false})).unwrap();
    assert!(!minimal.ok);
    assert!(minimal.error.is_none() && minimal.pong.is_none() && minimal.secret.is_none());
}

#[test]
fn response_decoding_ignores_unknown_fields_and_snake_case_pairing_id() {
    let resp: BridgeResponse = serde_json::from_value(json!({
        "reqId": "x", "ok": true, "extra": 1, "pairing_id": "ignored"
    }))
    .unwrap();
    assert!(resp.pairing_id.is_none());
}

#[test]
fn write_message_reports_a_closed_pipe() {
    struct ClosedPipe;
    impl std::io::Write for ClosedPipe {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    assert!(write_message(&mut ClosedPipe, &BridgeResponse::ok("x")).is_err());
}

// ───────────────────────── Secret hash ─────────────────────────

#[test]
fn short_hash_matches_reference_vectors() {
    // Independently computed: acc = acc * 31 + byte (wrapping u64) over the
    // UTF-8 bytes, printed as 16 hex digits.
    let vectors = [
        ("", "0000000000000000"),
        ("a", "0000000000000061"),
        ("ab", "0000000000000c21"),
        ("sobria", "00000000ca87c5c4"),
        ("é", "0000000000001846"),
    ];
    for (input, expected) in vectors {
        assert_eq!(short_hash(input), expected, "{input:?}");
    }
    assert_eq!(short_hash(&"0123abcdef".repeat(6)), "bca8e8af2793ef9e");
}

#[test]
fn short_hash_is_sixteen_lowercase_hex_digits_and_depends_on_the_input() {
    // Known gap: the `short_hash` doc (lib.rs) says 8 "FNV-like" characters;
    // the code returns 16 digits of a ×31 polynomial hash (SOB-81).
    let hash = short_hash(&"0123abcdef".repeat(6));
    assert_eq!(hash.len(), 16);
    assert!(hash.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')));
    assert_ne!(short_hash("a"), short_hash("b"));
}

// ───────────────────────── Spool (Estimate path) ─────────────────────────

#[test]
fn spool_creates_missing_parent_directories() {
    let temp = tempfile::tempdir().unwrap();
    let spool = temp.path().join("a").join("b").join("incoming.jsonl");
    append_to_spool_at(&spool, &json!({"n": 1}), SPOOL_MAX_BYTES).unwrap();
    assert_eq!(std::fs::read_to_string(&spool).unwrap(), "{\"n\":1}\n");
}

#[test]
fn spool_rotation_threshold_is_strict() {
    let temp = tempfile::tempdir().unwrap();
    let spool = temp.path().join("incoming.jsonl");
    let bak = temp.path().join("incoming.jsonl.bak");

    append_to_spool_at(&spool, &json!({"a": 1}), SPOOL_MAX_BYTES).unwrap();
    let line_len = std::fs::metadata(&spool).unwrap().len();

    append_to_spool_at(&spool, &json!({"a": 1}), line_len).unwrap();
    assert!(!bak.exists(), "size == threshold: no rotation");
    assert_eq!(std::fs::read_to_string(&spool).unwrap().lines().count(), 2);

    append_to_spool_at(&spool, &json!({"a": 1}), 2 * line_len - 1).unwrap();
    assert!(bak.exists(), "size > threshold: rotation");
    assert_eq!(std::fs::read_to_string(&spool).unwrap().lines().count(), 1);
}

#[test]
fn spool_rotation_replaces_the_previous_backup() {
    let temp = tempfile::tempdir().unwrap();
    let spool = temp.path().join("incoming.jsonl");
    let bak = temp.path().join("incoming.jsonl.bak");
    for n in 1..=3 {
        append_to_spool_at(&spool, &json!({"n": n}), 1).unwrap();
    }
    assert_eq!(std::fs::read_to_string(&bak).unwrap(), "{\"n\":2}\n");
    assert_eq!(std::fs::read_to_string(&spool).unwrap(), "{\"n\":3}\n");
}

// ───────────────────────── Public constants ─────────────────────────

#[test]
fn public_constants_keep_their_values() {
    assert_eq!(NATIVE_HOST_NAME, "com.sobria.bridge");
    assert_eq!(SPOOL_MAX_BYTES, 10 * 1024 * 1024);
}

#[test]
fn default_socket_path_targets_the_sobria_bridge_endpoint() {
    let path = default_socket_path();
    #[cfg(windows)]
    assert_eq!(path, std::path::PathBuf::from(r"\\.\pipe\sobria-bridge"));
    #[cfg(unix)]
    assert_eq!(path.file_name().unwrap(), "sobria-bridge.sock");
}
