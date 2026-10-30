//! End-to-end characterization of the `sobria-bridge` binary (SOB-11), run the
//! way the browser runs it: frames on stdin, frames read back from stdout.
//!
//! Each run points `HOME` (spool) and `XDG_RUNTIME_DIR` (app socket) to a temp
//! dir, so the real `~/.sobria` and a real app are never touched. Other env
//! vars are inherited so coverage tooling keeps tracking the binary.
//!
//! Unix only: on Windows the spool follows the OS user folder (not `HOME`) and
//! the app pipe has a fixed name, so the process cannot be isolated.
#![cfg(unix)]

mod common;

use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use common::fake_app::{serve, Reply};
use common::{
    frame, json_frame, parse_frames, MAX_MESSAGE_BYTES, PAIR_OFFLINE_REFUSAL,
    REVOKE_OFFLINE_REFUSAL,
};
use serde_json::{json, Value};
use sobria_bridge::short_hash;
use tempfile::TempDir;

struct Session {
    dir: TempDir,
}

impl Session {
    fn new() -> Self {
        Self {
            dir: TempDir::new().unwrap(),
        }
    }

    fn spool(&self) -> PathBuf {
        self.dir.path().join(".sobria/spool/incoming.jsonl")
    }

    fn socket(&self) -> PathBuf {
        self.dir.path().join("sobria-bridge.sock")
    }

    fn app(&self, replies: Vec<Reply>) -> std::thread::JoinHandle<Vec<Value>> {
        serve(UnixListener::bind(self.socket()).unwrap(), replies)
    }

    fn run(&self, input: &[u8]) -> Output {
        let home = self.dir.path();
        let mut child = Command::new(env!("CARGO_BIN_EXE_sobria-bridge"))
            .env("HOME", home)
            .env("XDG_RUNTIME_DIR", home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn bridge");
        // Drain output while waiting: a reply larger than the pipe buffer would
        // otherwise block the bridge on write.
        let stdout = drain(child.stdout.take().expect("stdout"));
        let stderr = drain(child.stderr.take().expect("stderr"));
        // The bridge may exit before reading everything (invalid message).
        let _ = child.stdin.take().expect("stdin").write_all(input);

        // A bridge that never exits would hang CI until its global timeout.
        let deadline = Instant::now() + Duration::from_secs(30);
        let status = loop {
            if let Some(status) = child.try_wait().expect("bridge status") {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill().expect("kill bridge");
                panic!("the bridge did not exit within 30 s");
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        Output {
            status,
            stdout: stdout.join().expect("read stdout"),
            stderr: stderr.join().expect("read stderr"),
        }
    }
}

fn drain(mut pipe: impl Read + Send + 'static) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        pipe.read_to_end(&mut bytes).expect("read bridge output");
        bytes
    })
}

fn input(requests: &[Value]) -> Vec<u8> {
    requests.iter().flat_map(json_frame).collect()
}

fn answers(output: &Output) -> Vec<Value> {
    parse_frames(&output.stdout)
}

fn assert_exits_with_error(output: &Output, needle: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains(needle), "{stderr}");
}

fn ping(id: &str) -> Value {
    json!({"type": "ping", "reqId": id})
}

fn pong(id: &str) -> Value {
    json!({"reqId": id, "ok": true, "pong": true})
}

fn pair(id: &str) -> Value {
    json!({"type": "pair", "reqId": id, "code": "123456"})
}

fn estimate(id: &str, secret: &str, payload: &Value) -> Value {
    json!({"type": "estimate", "reqId": id, "secret": secret, "payload": payload})
}

// ───────────────────── Desktop app unreachable ─────────────────────

#[test]
fn empty_input_ends_cleanly_with_no_output() {
    let out = Session::new().run(b"");
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
}

#[test]
fn ping_is_answered_by_the_bridge_itself() {
    let out = Session::new().run(&input(&[ping("1")]));
    assert!(out.status.success());
    assert_eq!(answers(&out), [pong("1")]);
}

#[test]
fn pair_and_revoke_are_refused_with_the_offline_messages() {
    let revoke = json!({"type": "revoke", "reqId": "v", "secret": "s"});
    let out = Session::new().run(&input(&[pair("p"), revoke]));
    assert!(out.status.success());
    assert_eq!(
        answers(&out),
        [
            json!({"reqId": "p", "ok": false, "error": PAIR_OFFLINE_REFUSAL}),
            json!({"reqId": "v", "ok": false, "error": REVOKE_OFFLINE_REFUSAL}),
        ]
    );
}

#[test]
fn estimate_is_acknowledged_and_appended_to_the_spool() {
    let session = Session::new();
    let secret = "0123abcdef".repeat(6);
    let first = json!({"estimate": {"modelId": "m", "tokensIn": 3}});
    let second = json!({"estimate": {"modelId": "n", "tokensIn": 5}});
    let out = session.run(&input(&[
        estimate("e1", &secret, &first),
        estimate("e2", &secret, &second),
    ]));

    assert!(out.status.success());
    assert_eq!(
        answers(&out),
        [
            json!({"reqId": "e1", "ok": true}),
            json!({"reqId": "e2", "ok": true})
        ]
    );

    let spool = std::fs::read_to_string(session.spool()).unwrap();
    assert!(
        !spool.contains(&secret),
        "the secret must not be stored in clear"
    );
    let lines: Vec<Value> = spool
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 2);
    for (line, payload) in lines.iter().zip([&first, &second]) {
        let received_at = line["received_at"].as_str().unwrap();
        chrono::DateTime::parse_from_rfc3339(received_at).expect("received_at in RFC 3339 format");
        // Full equality pins both the keys and their values.
        assert_eq!(
            line,
            &json!({
                "secret_hash": short_hash(&secret),
                "payload": payload,
                "received_at": received_at,
            })
        );
    }
}

#[test]
fn spool_failure_is_reported_and_the_bridge_keeps_running() {
    let session = Session::new();
    // `~/.sobria` is a file, so the spool directory cannot be created.
    std::fs::write(session.dir.path().join(".sobria"), b"").unwrap();
    let out = session.run(&input(&[estimate("e", "s", &json!({})), ping("after")]));

    assert!(out.status.success());
    let replies = answers(&out);
    assert_eq!(replies.len(), 2);
    assert_eq!(replies[0]["ok"], false);
    assert!(replies[0]["error"]
        .as_str()
        .unwrap()
        .starts_with("spool write: "));
    assert_eq!(replies[1], pong("after"));
}

// ───────────────────── Desktop app reachable ─────────────────────

#[test]
fn app_reply_to_a_pairing_reaches_the_extension_unchanged() {
    let session = Session::new();
    let reply = json!({
        "reqId": "p", "ok": true, "secret": "ab".repeat(32),
        "pairingId": "0b9a-uuid", "fingerprint": "sobria-bridge-native-messaging"
    });
    let app = session.app(vec![Reply::Json(reply.clone())]);
    let out = session.run(&input(&[pair("p")]));

    assert_eq!(app.join().unwrap(), [pair("p")]);
    assert_eq!(answers(&out), [reply]);
}

#[test]
fn messages_answered_by_the_app_are_never_spooled() {
    let replies = [
        json!({"reqId": "e", "ok": true}),
        json!({"reqId": "e", "ok": false, "error": "secret invalide ou pairing révoqué"}),
    ];
    for reply in replies {
        let session = Session::new();
        let app = session.app(vec![Reply::Json(reply.clone())]);
        let out = session.run(&input(&[estimate("e", "s", &json!({}))]));
        app.join().unwrap();
        assert_eq!(answers(&out), [reply]);
        assert!(!session.spool().exists(), "no fallback to the spool");
    }
}

#[test]
fn each_message_opens_its_own_connection_to_the_app() {
    let session = Session::new();
    let app = session.app(vec![
        Reply::Json(pong("1")),
        Reply::Json(json!({"reqId": "p", "ok": false, "error": "code invalide ou expiré"})),
    ]);
    let out = session.run(&input(&[ping("1"), pair("p")]));

    assert_eq!(app.join().unwrap(), [ping("1"), pair("p")]);
    assert_eq!(
        answers(&out),
        [
            pong("1"),
            json!({"reqId": "p", "ok": false, "error": "code invalide ou expiré"}),
        ]
    );
}

// ───────────── Faulty desktop app: fallback to local handling ─────────────

#[test]
fn unreadable_app_reply_falls_back_to_local_handling() {
    let session = Session::new();
    let app = session.app(vec![Reply::Raw(frame(b"not json"))]);
    let out = session.run(&input(&[ping("1")]));

    assert_eq!(app.join().unwrap(), [ping("1")]);
    assert_eq!(answers(&out), [pong("1")]);
}

#[test]
fn app_that_hangs_up_falls_back_to_local_handling() {
    let session = Session::new();
    let app = session.app(vec![Reply::Close]);
    let out = session.run(&input(&[pair("p")]));

    app.join().unwrap();
    assert_eq!(
        answers(&out),
        [json!({"reqId": "p", "ok": false, "error": PAIR_OFFLINE_REFUSAL})]
    );
}

#[test]
fn stale_socket_file_falls_back_to_local_handling() {
    let session = Session::new();
    drop(UnixListener::bind(session.socket()).unwrap());
    assert!(session.socket().exists());
    let out = session.run(&input(&[ping("1")]));
    assert_eq!(answers(&out), [pong("1")]);
}

// ───────────────────── Invalid input ─────────────────────

#[test]
fn undecodable_message_ends_the_session_without_a_reply() {
    // Quirk: no error reply is sent, the bridge exits with code 1 and later
    // messages are never handled.
    let bad = json!({"type": "nope", "reqId": "x"});
    let out = Session::new().run(&input(&[ping("1"), bad, ping("3")]));

    assert_exits_with_error(&out, "decode JSON");
    assert_eq!(answers(&out), [pong("1")]);
}

#[test]
fn oversized_message_ends_the_session() {
    let out = Session::new().run(&(MAX_MESSAGE_BYTES + 1).to_le_bytes());
    assert_exits_with_error(&out, "exceeds 1 MB");
    assert!(out.stdout.is_empty());
}

#[test]
fn message_cut_short_ends_the_session() {
    let mut bytes = 100u32.to_le_bytes().to_vec();
    bytes.extend_from_slice(b"{\"t");
    let out = Session::new().run(&bytes);
    assert_exits_with_error(&out, "read native messaging payload");
    assert!(out.stdout.is_empty());
}
