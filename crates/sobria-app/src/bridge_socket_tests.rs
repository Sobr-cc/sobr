//! Characterization of the real bridge socket server (SOB-188).
//!
//! Unlike `bridge_server.rs`, which calls `dispatch_request` directly, these
//! tests start `bridge_server::serve` (the code `run` uses at app start-up)
//! on a private endpoint (a Unix socket in a temp folder, or a uniquely named
//! pipe on Windows) and talk to it as the `sobria-bridge` binary does:
//! one connection per request, a `u32` little-endian length, then the JSON.
//!
//! The server needs an `AppHandle` to find the `AppState`; the tests give it
//! the handle of a mock Tauri app (`tauri::test`).
//!
//! They are tests of the binary (a `#[cfg(test)]` module of `main.rs`), not an
//! integration test under `tests/`: a test executable that really links Tauri
//! needs the Windows manifest that `tauri-build` embeds in the binary target
//! only, and without it the process cannot start (`STATUS_ENTRYPOINT_NOT_FOUND`).

use std::path::{Path, PathBuf};
use std::time::Duration;

use sobria_app::dto::{ExtensionEventDto, PairingDto};
use sobria_app::{bridge_server, logic, AppState};
use sobria_bridge::{BridgeRequest, BridgeResponse};
use tauri::test::{mock_app, MockRuntime};
use tauri::{App, Manager};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::task::JoinHandle;

#[cfg(unix)]
type Client = tokio::net::UnixStream;
#[cfg(windows)]
type Client = tokio::net::windows::named_pipe::NamedPipeClient;

/// Same limit as the server (`MAX_PAYLOAD_BYTES` in `bridge_server.rs`).
const MAX_PAYLOAD_BYTES: u32 = 1024 * 1024;

const NO_PENDING_CODE: &str =
    "aucun code de pairing en attente — génère-en un depuis Sobr.ia /parametres";
const BRIDGE_FINGERPRINT: &str = "sobria-bridge-native-messaging";
const UNKNOWN_SECRET: &str = "secret invalide ou pairing révoqué";

// ─── Client side ─────────────────────────────────────────────────────────────

/// A fresh endpoint for one test: no two tests share a socket or a pipe.
fn endpoint(dir: &Path) -> PathBuf {
    #[cfg(unix)]
    {
        dir.join("bridge-test.sock")
    }
    #[cfg(windows)]
    {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let _ = dir;
        PathBuf::from(format!(
            r"\\.\pipe\sobria-bridge-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }
}

/// Connects, retrying while the server is still binding (or, on Windows,
/// while every pipe instance is busy).
async fn connect(endpoint: &Path) -> Client {
    // The server task has not run yet: give it its turn before the first attempt.
    tokio::task::yield_now().await;
    for _ in 0..1000 {
        #[cfg(unix)]
        let attempt = tokio::net::UnixStream::connect(endpoint).await;
        #[cfg(windows)]
        let attempt = tokio::net::windows::named_pipe::ClientOptions::new().open(endpoint);
        if let Ok(client) = attempt {
            return client;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!(
        "the bridge server never accepted a connection on {}",
        endpoint.display()
    );
}

/// A frame (length + payload) as the server expects it.
fn frame(payload: &[u8]) -> Vec<u8> {
    let mut bytes = u32::try_from(payload.len()).unwrap().to_le_bytes().to_vec();
    bytes.extend_from_slice(payload);
    bytes
}

/// Reads one answer; `Err` when the server closes the connection without one.
async fn read_frame<S: AsyncRead + Unpin>(stream: &mut S) -> std::io::Result<Vec<u8>> {
    let mut len = [0u8; 4];
    stream.read_exact(&mut len).await?;
    let mut payload = vec![0u8; u32::from_le_bytes(len) as usize];
    stream.read_exact(&mut payload).await?;
    Ok(payload)
}

async fn send_raw<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    payload: &[u8],
) -> BridgeResponse {
    stream.write_all(&frame(payload)).await.unwrap();
    let answer = read_frame(stream).await.expect("the server answers");
    serde_json::from_slice(&answer).expect("the answer is a BridgeResponse")
}

/// One request on its own connection, like the bridge binary does.
async fn send(endpoint: &Path, request: BridgeRequest) -> BridgeResponse {
    let mut client = connect(endpoint).await;
    send_raw(&mut client, &serde_json::to_vec(&request).unwrap()).await
}

fn ping(req_id: &str) -> BridgeRequest {
    BridgeRequest::Ping {
        req_id: req_id.into(),
    }
}

fn pair(req_id: &str, code: &str) -> BridgeRequest {
    BridgeRequest::Pair {
        req_id: req_id.into(),
        code: code.into(),
    }
}

fn estimate(req_id: &str, secret: &str, payload: serde_json::Value) -> BridgeRequest {
    BridgeRequest::Estimate {
        req_id: req_id.into(),
        secret: secret.into(),
        payload,
    }
}

fn revoke(req_id: &str, secret: &str) -> BridgeRequest {
    BridgeRequest::Revoke {
        req_id: req_id.into(),
        secret: secret.into(),
    }
}

// ─── Server side ─────────────────────────────────────────────────────────────

/// The real server, running against a mock app that manages a fresh `AppState`.
/// It stops with the test: the runtime of `#[tokio::test]` drops its task.
struct Server {
    _dir: tempfile::TempDir,
    app: App<MockRuntime>,
    endpoint: PathBuf,
}

impl Server {
    fn start() -> Self {
        Self::start_with(|_| {})
    }

    /// `prepare` runs on the endpoint before the server starts.
    fn start_with(prepare: impl FnOnce(&Path)) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let app = mock_app();
        app.manage(AppState::init_in(dir.path()).unwrap());
        let endpoint = endpoint(dir.path());
        prepare(&endpoint);
        Self::spawn(&app, &endpoint);
        Self {
            _dir: dir,
            app,
            endpoint,
        }
    }

    fn spawn(app: &App<MockRuntime>, endpoint: &Path) -> JoinHandle<anyhow::Result<()>> {
        let handle = app.handle().clone();
        let endpoint = endpoint.to_path_buf();
        tokio::spawn(async move { bridge_server::serve(handle, &endpoint).await })
    }

    fn state(&self) -> tauri::State<'_, AppState> {
        self.app.state::<AppState>()
    }

    async fn send(&self, request: BridgeRequest) -> BridgeResponse {
        send(&self.endpoint, request).await
    }

    /// A pending pairing code, as the app shows it in its settings.
    fn pending_code(&self) -> String {
        logic::regenerate_pairing_code(self.state().inner())
            .unwrap()
            .code
    }

    fn events(&self) -> Vec<ExtensionEventDto> {
        logic::list_extension_events(self.state().inner(), 10, 0).unwrap()
    }

    fn pairings(&self) -> Vec<PairingDto> {
        logic::list_pairings(self.state().inner()).unwrap()
    }

    /// The server closes a connection that breaks the protocol without an answer.
    async fn assert_no_answer(&self, bytes: &[u8], what: &str) {
        let mut client = connect(&self.endpoint).await;
        client.write_all(bytes).await.unwrap();
        // An answer, or a connection left open and silent, both fail the test.
        let read = tokio::time::timeout(Duration::from_secs(5), read_frame(&mut client))
            .await
            .unwrap_or_else(|_| panic!("{what}: the server left the connection open"));
        assert!(read.is_err(), "{what}: expected no answer");
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn ping_gets_a_pong_through_the_socket() {
    let server = Server::start();

    let resp = server.send(ping("p1")).await;

    assert_eq!(resp.req_id, "p1");
    assert!(resp.ok);
    assert_eq!(resp.pong, Some(true));
    assert!(resp.error.is_none());
    assert!(resp.secret.is_none() && resp.pairing_id.is_none() && resp.fingerprint.is_none());
}

#[tokio::test]
async fn pair_estimate_and_revoke_work_through_the_socket() {
    let server = Server::start();

    let code = server.pending_code();
    let paired = server.send(pair("r-pair", &code)).await;
    assert!(paired.ok, "pairing refused: {:?}", paired.error);
    assert_eq!(paired.req_id, "r-pair");
    let secret = paired.secret.clone().expect("a secret");
    assert_eq!(secret.len(), 64);
    let pairing_id = paired.pairing_id.clone().expect("a pairing id");
    assert_eq!(paired.fingerprint.as_deref(), Some(BRIDGE_FINGERPRINT));

    let payload = serde_json::json!({
        "estimate": {
            "method": "afnor_sobria", "modelId": "gpt-4o-mini", "tokensIn": 50,
            "tokensOut": 200, "gco2eq": 0.42, "waterMl": 1.8, "energyWh": 0.12
        }
    });
    let stored = server.send(estimate("r-est", &secret, payload)).await;
    assert!(stored.ok, "estimate refused: {:?}", stored.error);
    assert_eq!(stored.pairing_id.as_deref(), Some(pairing_id.as_str()));
    let events = server.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].model_id, "gpt-4o-mini");
    assert_eq!((events[0].tokens_in, events[0].tokens_out), (50, 200));

    let revoked = server.send(revoke("r-rev", &secret)).await;
    assert!(revoked.ok, "revoke refused: {:?}", revoked.error);
    assert_eq!(revoked.pairing_id.as_deref(), Some(pairing_id.as_str()));

    // A revoked secret is refused by both requests.
    let again = server.send(revoke("r-rev2", &secret)).await;
    assert!(!again.ok);
    assert_eq!(
        again.error.as_deref(),
        Some("secret invalide — déjà révoqué ?")
    );
    let late = server
        .send(estimate(
            "r-late",
            &secret,
            serde_json::json!({"estimate": {}}),
        ))
        .await;
    assert!(!late.ok);
    assert_eq!(late.error.as_deref(), Some(UNKNOWN_SECRET));
    assert_eq!(server.events().len(), 1);
}

#[tokio::test]
async fn refused_requests_get_their_pinned_answers_through_the_socket() {
    let server = Server::start();

    // No pending code at all.
    let resp = server.send(pair("r1", "123456")).await;
    assert!(!resp.ok);
    assert_eq!(resp.req_id, "r1");
    assert_eq!(resp.error.as_deref(), Some(NO_PENDING_CODE));
    assert!(resp.secret.is_none() && resp.pairing_id.is_none() && resp.pong.is_none());

    // A pending code: malformed, then wrong. Refusals keep the code pending.
    let code = server.pending_code();
    let wrong = format!("{:06}", (code.parse::<u32>().unwrap() + 1) % 1_000_000);
    assert_eq!(
        server.send(pair("r2", "abc")).await.error.as_deref(),
        Some("code à 6 chiffres requis")
    );
    for attempt in 0..3 {
        assert_eq!(
            server.send(pair("r3", &wrong)).await.error.as_deref(),
            Some("code invalide ou expiré"),
            "attempt {attempt}"
        );
    }
    assert!(server.pairings().is_empty());

    // The right code still pairs, once.
    let paired = server.send(pair("r4", &code)).await;
    assert!(paired.ok, "pairing refused: {:?}", paired.error);
    assert_eq!(
        server.send(pair("r5", &code)).await.error.as_deref(),
        Some(NO_PENDING_CODE)
    );

    // Estimate and revoke with a secret nobody paired with, then with the
    // secret of the pairing altered in its first digit.
    let secret = paired.secret.unwrap();
    let altered = format!(
        "{}{}",
        if secret.starts_with('0') { '1' } else { '0' },
        &secret[1..]
    );
    for unknown in ["0".repeat(64), altered] {
        let resp = server
            .send(estimate(
                "r6",
                &unknown,
                serde_json::json!({"estimate": {}}),
            ))
            .await;
        assert_eq!(resp.error.as_deref(), Some(UNKNOWN_SECRET));
        assert_eq!(
            server.send(revoke("r7", &unknown)).await.error.as_deref(),
            Some("secret invalide — déjà révoqué ?")
        );
    }

    // A valid secret, but a payload without `estimate`.
    let resp = server
        .send(estimate("r8", &secret, serde_json::json!({"other": 1})))
        .await;
    assert!(!resp.ok);
    assert_eq!(resp.error.as_deref(), Some("payload sans champ `estimate`"));
    assert!(server.events().is_empty());
}

#[tokio::test]
async fn a_connection_that_breaks_the_protocol_gets_no_answer_and_the_server_keeps_serving() {
    let server = Server::start();

    // A length above the limit: the server refuses it after reading the 4 bytes.
    server
        .assert_no_answer(
            &(MAX_PAYLOAD_BYTES + 1).to_le_bytes(),
            "length over the limit",
        )
        .await;
    assert_eq!(
        server.send(ping("after-oversized")).await.req_id,
        "after-oversized"
    );

    // A frame that is not JSON, then JSON that is not a request.
    server
        .assert_no_answer(&frame(b"not json"), "payload that is not JSON")
        .await;
    server
        .assert_no_answer(
            &frame(br#"{"type":"unknown","reqId":"x"}"#),
            "unknown request type",
        )
        .await;
    assert_eq!(
        server.send(ping("after-garbage")).await.req_id,
        "after-garbage"
    );

    // A frame cut short, and a client that connects and leaves.
    let mut client = connect(&server.endpoint).await;
    client
        .write_all(&[100u32.to_le_bytes().as_slice(), b"short"].concat())
        .await
        .unwrap();
    drop(client);
    drop(connect(&server.endpoint).await);
    assert_eq!(
        server.send(ping("after-hangups")).await.req_id,
        "after-hangups"
    );
}

#[tokio::test]
async fn a_request_of_exactly_the_size_limit_is_served() {
    let server = Server::start();
    // `{"type":"ping","reqId":"<padding>"}` padded to exactly the limit.
    let (head, tail) = (r#"{"type":"ping","reqId":""#, r#""}"#);
    let padding = "x".repeat(MAX_PAYLOAD_BYTES as usize - head.len() - tail.len());
    let payload = format!("{head}{padding}{tail}");
    assert_eq!(payload.len(), MAX_PAYLOAD_BYTES as usize);

    let mut client = connect(&server.endpoint).await;
    let resp = send_raw(&mut client, payload.as_bytes()).await;

    assert!(resp.ok);
    assert_eq!(resp.req_id.len(), padding.len());
}

#[tokio::test]
async fn several_clients_are_served_one_after_the_other_and_at_once() {
    let server = Server::start();

    for n in 0..5 {
        let id = format!("seq-{n}");
        assert_eq!(server.send(ping(&id)).await.req_id, id);
    }

    let (a, b, c) = tokio::join!(
        server.send(ping("par-a")),
        server.send(ping("par-b")),
        server.send(ping("par-c"))
    );
    assert_eq!(
        (a.req_id.as_str(), b.req_id.as_str(), c.req_id.as_str()),
        ("par-a", "par-b", "par-c")
    );
    assert!(a.ok && b.ok && c.ok);
}

#[tokio::test]
async fn a_silent_client_does_not_block_the_others() {
    let server = Server::start();
    // Half of a length prefix, then nothing: the server waits for the rest.
    let mut silent = connect(&server.endpoint).await;
    silent.write_all(&[0u8, 0u8]).await.unwrap();

    let resp = tokio::time::timeout(Duration::from_secs(5), server.send(ping("p1")))
        .await
        .expect("the server still answers while another client is silent");

    assert!(resp.ok);
    drop(silent);
}

#[tokio::test]
async fn the_real_bridge_client_round_trips_with_the_server() {
    let server = Server::start();
    // The first request makes sure the server is accepting before the blocking client runs.
    assert!(server.send(ping("warm-up")).await.ok);
    // `sobria_bridge::try_forward_to` is what the bridge binary calls. It blocks,
    // so it runs on a blocking thread and leaves the runtime to the server.
    let forward = |request: BridgeRequest| {
        let path = server.endpoint.clone();
        async move {
            tokio::task::spawn_blocking(move || sobria_bridge::try_forward_to(&path, &request))
                .await
                .unwrap()
                .expect("the bridge client gets an answer")
        }
    };

    let pong = forward(ping("via-client")).await;
    assert_eq!(
        (pong.req_id.as_str(), pong.ok, pong.pong),
        ("via-client", true, Some(true))
    );

    let code = server.pending_code();
    let paired = forward(pair("via-client-pair", &code)).await;
    assert!(paired.ok, "pairing refused: {:?}", paired.error);
    let secret = paired.secret.expect("a secret");

    let payload = serde_json::json!({"estimate": {"modelId": "gpt-4o", "tokensIn": 7}});
    let stored = forward(estimate("via-client-est", &secret, payload)).await;
    assert!(stored.ok, "estimate refused: {:?}", stored.error);
    assert_eq!(server.events().len(), 1);
}

/// Unix: a socket file left by a crash does not prevent the next start.
#[cfg(unix)]
#[tokio::test]
async fn a_stale_socket_file_is_replaced_at_start() {
    let server =
        Server::start_with(|endpoint| std::fs::write(endpoint, b"left by a crash").unwrap());

    assert_eq!(server.send(ping("p1")).await.req_id, "p1");
}

/// Windows: the pipe is created with `first_pipe_instance`, so a second
/// server on the same name fails to start instead of sharing the pipe.
#[cfg(windows)]
#[tokio::test]
async fn a_second_server_on_the_same_pipe_fails_to_start() {
    let server = Server::start();
    assert!(server.send(ping("p1")).await.ok); // the first server is up

    let second = Server::spawn(&server.app, &server.endpoint);
    let result = tokio::time::timeout(Duration::from_secs(5), second)
        .await
        .expect("the second server returns")
        .expect("the second server does not panic");

    result.expect_err("the second server cannot create the pipe");
    assert!(
        server.send(ping("p2")).await.ok,
        "the first server is unaffected"
    );
}
