//! Characterization of the `team_*` IPC commands of `main.rs` (SOB-188).
//!
//! The commands are private functions of the binary and take a
//! `tauri::State`, so they are called from here with the state of a mock
//! Tauri app (`tauri::test`). The server of the team is replaced by a small
//! HTTP server on a random local port that records what it receives.

#![allow(
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::items_after_statements
)]

use std::sync::{Arc, Mutex, Once};

use serde_json::json;
use tauri::test::{mock_app, MockRuntime};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use super::*;
use team_settings::{
    KEY_ACCEPT_INVALID_CERTS, KEY_ACCESS_TOKEN, KEY_ENROLLED_AT, KEY_ESTIMATIONS_SENT,
    KEY_FINGERPRINT, KEY_LAST_SEEN_AT, KEY_MODE, KEY_REFRESH_TOKEN, KEY_URL, KEY_USER_ID,
};

const HEALTH: &str = r#"{"ok":true,"version":"0.9.0"}"#;
const ENROLLED: &str = r#"{"user_id":"01HZ-user","access_token":"jwt-1","refresh_token":"ulid.uuid-1","access_expires_at":"2026-05-17T00:00:00Z","refresh_expires_at":"2026-08-14T00:00:00Z"}"#;
const ACK: &str = r#"{"id":"est-1","ack":true}"#;
const NO_ACK: &str = r#"{"id":"est-1","ack":false}"#;
const REFRESHED: &str = r#"{"access_token":"jwt-new","refresh_token":"rt-new"}"#;

// ─── A local HTTP server that records the requests ───────────────────────────

#[derive(Debug, Clone)]
struct Seen {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: String,
}

impl Seen {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.body).expect("the body is JSON")
    }
}

/// `(status, body)` for the n-th request received (counted from 0).
type Handler = dyn Fn(&Seen, usize) -> (u16, String) + Send + Sync;

struct Stub {
    url: String,
    seen: Arc<Mutex<Vec<Seen>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Stub {
    async fn start(
        handler: impl Fn(&Seen, usize) -> (u16, String) + Send + Sync + 'static,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let handler: Arc<Handler> = Arc::new(handler);
        let log = Arc::clone(&seen);
        let task = tokio::spawn(async move {
            loop {
                let Ok((socket, _)) = listener.accept().await else {
                    return;
                };
                tokio::spawn(answer(socket, Arc::clone(&handler), Arc::clone(&log)));
            }
        });
        Self { url, seen, task }
    }

    /// A server that always answers the same thing.
    async fn always(status: u16, body: &'static str) -> Self {
        Self::start(move |_, _| (status, body.to_string())).await
    }

    fn requests(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
    }
}

impl Drop for Stub {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn answer(mut socket: TcpStream, handler: Arc<Handler>, log: Arc<Mutex<Vec<Seen>>>) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        let n = socket.read(&mut chunk).await.unwrap_or(0);
        if n == 0 {
            return;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos + 4;
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.split("\r\n");
    let mut request_line = lines.next().unwrap_or_default().split(' ');
    let method = request_line.next().unwrap_or_default().to_string();
    let path = request_line.next().unwrap_or_default().to_string();
    let headers: Vec<(String, String)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    let length: usize = headers
        .iter()
        .find(|(k, _)| k == "content-length")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);
    while buf.len() < head_end + length {
        let n = socket.read(&mut chunk).await.unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let body = String::from_utf8_lossy(&buf[head_end..]).to_string();
    let seen = Seen {
        method,
        path,
        headers,
        body,
    };
    let (status, reply) = {
        let mut log = log.lock().unwrap();
        let answer = handler(&seen, log.len());
        log.push(seen);
        answer
    };
    let response = format!(
        "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
        reply.len()
    );
    let _ = socket.write_all(response.as_bytes()).await;
    let _ = socket.shutdown().await;
}

/// An address on which nothing listens.
async fn closed_port_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    url
}

// ─── The app state behind the commands ───────────────────────────────────────

struct Fixture {
    _dir: tempfile::TempDir,
    app: tauri::App<MockRuntime>,
}

impl Fixture {
    fn new() -> Self {
        // The stub is on localhost: do not send it through a proxy of the machine.
        static NO_PROXY: Once = Once::new();
        NO_PROXY.call_once(|| std::env::set_var("NO_PROXY", "127.0.0.1,localhost"));
        let dir = tempfile::tempdir().unwrap();
        let app = mock_app();
        app.manage(AppState::init_in(dir.path()).unwrap());
        Self { _dir: dir, app }
    }

    fn state(&self) -> tauri::State<'_, AppState> {
        self.app.state::<AppState>()
    }

    fn set(&self, key: &str, value: &str) {
        self.state()
            .team_settings
            .lock()
            .unwrap()
            .set(key, value)
            .unwrap();
    }

    fn get(&self, key: &str) -> Option<String> {
        self.state().team_settings.lock().unwrap().get(key).unwrap()
    }

    /// What the UI gets from `team_status`.
    fn status(&self) -> serde_json::Value {
        serde_json::to_value(team_status(self.state()).unwrap()).unwrap()
    }

    /// A user enrolled on `url`, in `mode`, with tokens `jwt-1` / `rt-1`.
    fn enrolled_on(&self, url: &str, mode: &str) {
        self.set(KEY_URL, url);
        self.set(KEY_MODE, mode);
        self.set(KEY_ACCESS_TOKEN, "jwt-1");
        self.set(KEY_REFRESH_TOKEN, "rt-1");
        self.set(KEY_USER_ID, "01HZ-user");
    }
}

#[track_caller]
fn assert_err<T: std::fmt::Debug>(result: IpcResult<T>, code: &str, message: &str) {
    let error = result.expect_err("the command should fail");
    assert_eq!((error.code, error.message.as_str()), (code, message));
}

#[track_caller]
fn assert_err_starts<T: std::fmt::Debug>(result: IpcResult<T>, code: &str, prefix: &str) {
    let error = result.expect_err("the command should fail");
    assert_eq!(error.code, code);
    assert!(
        error.message.starts_with(prefix),
        "message: {}",
        error.message
    );
}

#[track_caller]
fn assert_rfc3339(value: Option<String>) {
    let value = value.expect("a timestamp");
    chrono::DateTime::parse_from_rfc3339(&value).expect("RFC 3339");
}

const NO_URL: &str = "URL serveur équipe non configurée";
const UNAUTHORIZED: &str = "authentification requise (pas enrôlé ou token expiré)";

// ─── team_status ─────────────────────────────────────────────────────────────

#[test]
fn team_status_of_a_fresh_install_is_local_and_not_enrolled() {
    let fx = Fixture::new();
    assert_eq!(
        fx.status(),
        json!({
            "enrolled": false, "url": null, "user_id": null, "mode": "local",
            "fingerprint": null, "enrolled_at": null, "accept_invalid_certs": false,
            "last_seen_at": null, "estimations_sent": 0
        })
    );
}

#[test]
fn team_status_reflects_the_stored_settings() {
    let fx = Fixture::new();
    fx.set(KEY_URL, "https://team.example");
    fx.set(KEY_MODE, "both");
    fx.set(KEY_USER_ID, "u-1");
    fx.set(KEY_ACCESS_TOKEN, "jwt");
    fx.set(KEY_FINGERPRINT, "app-win-1");
    fx.set(KEY_ENROLLED_AT, "2026-05-16T08:00:00Z");
    fx.set(KEY_ACCEPT_INVALID_CERTS, "1");
    fx.set(KEY_LAST_SEEN_AT, "2026-05-16T09:00:00Z");
    fx.set(KEY_ESTIMATIONS_SENT, "7");

    assert_eq!(
        fx.status(),
        json!({
            "enrolled": true, "url": "https://team.example", "user_id": "u-1", "mode": "both",
            "fingerprint": "app-win-1", "enrolled_at": "2026-05-16T08:00:00Z",
            "accept_invalid_certs": true, "last_seen_at": "2026-05-16T09:00:00Z",
            "estimations_sent": 7
        })
    );
}

#[test]
fn team_status_is_enrolled_as_soon_as_an_access_token_is_stored() {
    let fx = Fixture::new();
    fx.set(KEY_REFRESH_TOKEN, "rt");
    fx.set(KEY_USER_ID, "u-1");
    assert_eq!(fx.status()["enrolled"], json!(false));
    fx.set(KEY_ACCESS_TOKEN, "");
    assert_eq!(
        fx.status()["enrolled"],
        json!(true),
        "even an empty token counts"
    );
}

// ─── team_set_url, team_set_mode, team_set_accept_invalid_certs ─────────────

#[test]
fn team_set_url_refuses_anything_that_does_not_start_with_https() {
    let fx = Fixture::new();
    for url in [
        "http://team.example",
        "ftp://team.example",
        "team.example",
        "",
        " https://team.example",
        "HTTPS://team.example",
    ] {
        assert_err(
            team_set_url(url.to_string(), fx.state()),
            "bad_request",
            "url doit commencer par https://",
        );
    }
    assert_eq!(fx.get(KEY_URL), None, "a refused URL is not stored");
}

#[test]
fn team_set_url_stores_the_url_without_its_trailing_slashes() {
    let fx = Fixture::new();
    for (given, stored) in [
        ("https://team.example", "https://team.example"),
        ("https://team.example/", "https://team.example"),
        ("https://team.example:8443///", "https://team.example:8443"),
        (
            "https://team.example/sobria/",
            "https://team.example/sobria",
        ),
    ] {
        team_set_url(given.to_string(), fx.state()).unwrap();
        assert_eq!(fx.get(KEY_URL).as_deref(), Some(stored), "given {given:?}");
    }
}

#[test]
fn team_set_url_quirk_a_bare_scheme_is_accepted_and_stored_as_https_colon() {
    let fx = Fixture::new();
    team_set_url("https://".to_string(), fx.state()).unwrap();
    assert_eq!(fx.get(KEY_URL).as_deref(), Some("https:"));
}

#[test]
fn team_set_mode_stores_the_known_modes_and_turns_anything_else_into_local() {
    let fx = Fixture::new();
    for (given, stored) in [
        ("team", "team"),
        ("both", "both"),
        ("local", "local"),
        ("team", "team"),
        ("TEAM", "local"),
        ("everything", "local"),
        ("both", "both"),
        ("", "local"),
    ] {
        team_set_mode(given.to_string(), fx.state()).unwrap();
        assert_eq!(fx.get(KEY_MODE).as_deref(), Some(stored), "given {given:?}");
    }
}

#[test]
fn team_set_accept_invalid_certs_stores_one_or_zero() {
    let fx = Fixture::new();
    team_set_accept_invalid_certs(true, fx.state()).unwrap();
    assert_eq!(fx.get(KEY_ACCEPT_INVALID_CERTS).as_deref(), Some("1"));
    assert_eq!(fx.status()["accept_invalid_certs"], json!(true));

    team_set_accept_invalid_certs(false, fx.state()).unwrap();
    assert_eq!(fx.get(KEY_ACCEPT_INVALID_CERTS).as_deref(), Some("0"));
    assert_eq!(fx.status()["accept_invalid_certs"], json!(false));
}

// ─── team_ping ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn team_ping_without_a_url_fails_with_no_url() {
    let fx = Fixture::new();
    assert_err(team_ping(fx.state()).await, "no_url", NO_URL);
    assert_eq!(fx.get(KEY_LAST_SEEN_AT), None);
}

#[tokio::test]
async fn team_ping_returns_the_health_of_the_server_and_marks_it_seen() {
    let fx = Fixture::new();
    let stub = Stub::always(200, HEALTH).await;
    // A trailing slash in the stored URL does not double the slash in the request.
    fx.set(KEY_URL, &format!("{}/", stub.url));
    fx.set(KEY_ACCESS_TOKEN, "jwt-1");

    let health = team_ping(fx.state()).await.unwrap();

    assert!(health.ok);
    assert_eq!(health.version, "0.9.0");
    let [seen] = stub.requests().try_into().expect("one request");
    assert_eq!(
        (seen.method.as_str(), seen.path.as_str()),
        ("GET", "/health")
    );
    assert_eq!(
        seen.header("authorization"),
        None,
        "the ping does not send the token"
    );
    assert!(seen
        .header("user-agent")
        .is_some_and(|ua| ua.starts_with("sobria-app/")));
    assert_rfc3339(fx.get(KEY_LAST_SEEN_AT));
    assert_eq!(
        fx.get(KEY_ESTIMATIONS_SENT),
        None,
        "a ping is not an estimation"
    );
}

#[tokio::test]
async fn team_ping_maps_the_failures_of_the_server() {
    for (status, body, code, message) in [
        (
            500,
            "boom",
            "http_error",
            "erreur HTTP 500: boom".to_string(),
        ),
        (
            404,
            "nope",
            "http_error",
            "erreur HTTP 404: nope".to_string(),
        ),
        (401, "", "unauthorized", UNAUTHORIZED.to_string()),
        (
            400,
            "bad input",
            "bad_request",
            "requête invalide: bad input".to_string(),
        ),
    ] {
        let fx = Fixture::new();
        let stub = Stub::always(status, body).await;
        fx.set(KEY_URL, &stub.url);

        assert_err(team_ping(fx.state()).await, code, &message);
        assert_eq!(
            fx.get(KEY_LAST_SEEN_AT),
            None,
            "a failed ping is not a sign of life"
        );
    }
}

#[tokio::test]
async fn team_ping_fails_with_transport_when_the_answer_is_not_a_health_response() {
    let fx = Fixture::new();
    let stub = Stub::always(200, "not json").await;
    fx.set(KEY_URL, &stub.url);
    assert_err_starts(
        team_ping(fx.state()).await,
        "transport",
        "erreur transport:",
    );
}

#[tokio::test]
async fn team_ping_fails_with_transport_when_the_server_is_unreachable() {
    let fx = Fixture::new();
    fx.set(KEY_URL, &closed_port_url().await);
    assert_err_starts(
        team_ping(fx.state()).await,
        "transport",
        "erreur transport:",
    );
    assert_eq!(fx.get(KEY_LAST_SEEN_AT), None);
}

// ─── team_enroll ─────────────────────────────────────────────────────────────

async fn enroll(
    fx: &Fixture,
    display_name: Option<&str>,
) -> IpcResult<team_client::EnrollResponse> {
    team_enroll(
        "123456789012".to_string(),
        "a-password".to_string(),
        "app-win-abcd1234".to_string(),
        display_name.map(str::to_string),
        fx.state(),
    )
    .await
}

#[tokio::test]
async fn team_enroll_without_a_url_fails_with_no_url() {
    let fx = Fixture::new();
    assert_err(enroll(&fx, None).await, "no_url", NO_URL);
    assert!(!fx.status()["enrolled"].as_bool().unwrap());
}

#[tokio::test]
async fn team_enroll_posts_the_credentials_and_stores_the_session() {
    let fx = Fixture::new();
    let stub = Stub::always(200, ENROLLED).await;
    fx.set(KEY_URL, &stub.url);

    let resp = enroll(&fx, Some("Ada")).await.unwrap();

    assert_eq!(resp.user_id, "01HZ-user");
    assert_eq!(resp.access_token, "jwt-1");
    assert_eq!(resp.refresh_token, "ulid.uuid-1");
    let [seen] = stub.requests().try_into().expect("one request");
    assert_eq!(
        (seen.method.as_str(), seen.path.as_str()),
        ("POST", "/api/v1/enroll")
    );
    assert_eq!(seen.header("content-type"), Some("application/json"));
    assert_eq!(seen.header("authorization"), None);
    assert_eq!(
        seen.json(),
        json!({
            "code": "123456789012", "password": "a-password",
            "fingerprint": "app-win-abcd1234", "display_name": "Ada"
        })
    );
    assert_eq!(fx.get(KEY_ACCESS_TOKEN).as_deref(), Some("jwt-1"));
    assert_eq!(fx.get(KEY_REFRESH_TOKEN).as_deref(), Some("ulid.uuid-1"));
    assert_eq!(fx.get(KEY_USER_ID).as_deref(), Some("01HZ-user"));
    assert_eq!(fx.get(KEY_FINGERPRINT).as_deref(), Some("app-win-abcd1234"));
    assert_rfc3339(fx.get(KEY_ENROLLED_AT));
    assert_eq!(
        fx.get(KEY_MODE).as_deref(),
        Some("both"),
        "local becomes both"
    );
    assert_eq!(
        fx.get(KEY_LAST_SEEN_AT),
        None,
        "an enrollment does not mark the server as seen"
    );
    assert!(fx.status()["enrolled"].as_bool().unwrap());
}

#[tokio::test]
async fn team_enroll_sends_no_display_name_when_none_is_given() {
    let fx = Fixture::new();
    let stub = Stub::always(200, ENROLLED).await;
    fx.set(KEY_URL, &stub.url);

    enroll(&fx, None).await.unwrap();

    assert_eq!(
        stub.requests()[0].json(),
        json!({"code": "123456789012", "password": "a-password", "fingerprint": "app-win-abcd1234"})
    );
}

#[tokio::test]
async fn team_enroll_keeps_the_mode_the_user_already_chose() {
    for mode in ["team", "both"] {
        let fx = Fixture::new();
        let stub = Stub::always(200, ENROLLED).await;
        fx.set(KEY_URL, &stub.url);
        fx.set(KEY_MODE, mode);

        enroll(&fx, None).await.unwrap();

        assert_eq!(fx.get(KEY_MODE).as_deref(), Some(mode));
    }
}

#[tokio::test]
async fn team_enroll_maps_the_refusals_and_stores_nothing() {
    let conflict = r#"{"error":"empreinte déjà enrôlée","code":"fingerprint_already_enrolled"}"#;
    for (status, body, code, message) in [
        (
            409,
            conflict,
            "http_error",
            format!("erreur HTTP 409: {conflict}"),
        ),
        (401, "{}", "unauthorized", UNAUTHORIZED.to_string()),
        (
            400,
            "bad code",
            "bad_request",
            "requête invalide: bad code".to_string(),
        ),
        (
            500,
            "boom",
            "http_error",
            "erreur HTTP 500: boom".to_string(),
        ),
    ] {
        let fx = Fixture::new();
        let stub = Stub::always(status, body).await;
        fx.set(KEY_URL, &stub.url);

        assert_err(enroll(&fx, None).await, code, &message);

        assert_eq!(fx.get(KEY_ACCESS_TOKEN), None);
        assert_eq!(fx.get(KEY_USER_ID), None);
        assert_eq!(fx.get(KEY_MODE), None, "the mode is not touched");
    }
}

#[tokio::test]
async fn team_enroll_fails_with_transport_on_an_incomplete_answer() {
    let fx = Fixture::new();
    let stub = Stub::always(200, r#"{"user_id":"u"}"#).await;
    fx.set(KEY_URL, &stub.url);
    assert_err_starts(enroll(&fx, None).await, "transport", "erreur transport:");
    assert_eq!(fx.get(KEY_ACCESS_TOKEN), None);
}

// ─── team_logout ─────────────────────────────────────────────────────────────

#[test]
fn team_logout_clears_the_session_and_goes_back_to_local() {
    let fx = Fixture::new();
    fx.enrolled_on("https://team.example", "team");
    fx.set(KEY_FINGERPRINT, "app-win-1");
    fx.set(KEY_ENROLLED_AT, "2026-05-16T08:00:00Z");
    fx.set(KEY_LAST_SEEN_AT, "2026-05-16T09:00:00Z");
    fx.set(KEY_ESTIMATIONS_SENT, "7");
    fx.set(KEY_ACCEPT_INVALID_CERTS, "1");

    team_logout(fx.state()).unwrap();

    assert_eq!(
        fx.status(),
        json!({
            "enrolled": false, "url": "https://team.example", "user_id": null, "mode": "local",
            "fingerprint": null, "enrolled_at": null, "accept_invalid_certs": true,
            "last_seen_at": null, "estimations_sent": 0
        })
    );
    assert_eq!(fx.get(KEY_REFRESH_TOKEN), None);
}

#[test]
fn team_logout_works_on_an_install_that_never_enrolled() {
    let fx = Fixture::new();
    team_logout(fx.state()).unwrap();
    assert_eq!(fx.get(KEY_MODE).as_deref(), Some("local"));
    assert_eq!(fx.status()["enrolled"], json!(false));
}

// ─── team_push_estimation ────────────────────────────────────────────────────

fn payload() -> serde_json::Value {
    json!({
        "estimate": {"method": "afnor_sobria", "modelId": "gpt-4o", "gco2eq": 1.5},
        "host": "chatgpt", "modelDisplayName": "GPT-4o", "ts": "2026-05-16T08:00:00Z"
    })
}

fn estimations_ok(seen: &Seen, _: usize) -> (u16, String) {
    match seen.path.as_str() {
        "/api/v1/estimations" => (200, ACK.to_string()),
        _ => (404, "no route".to_string()),
    }
}

#[tokio::test]
async fn team_push_estimation_is_not_sent_in_local_mode_or_when_not_enrolled() {
    let stub = Stub::start(estimations_ok).await;

    // Enrolled, but in local mode.
    let fx = Fixture::new();
    fx.enrolled_on(&stub.url, "local");
    assert!(!team_push_estimation(payload(), fx.state()).await.unwrap());

    // Team mode, but no access token: not enrolled.
    let fx = Fixture::new();
    fx.set(KEY_URL, &stub.url);
    fx.set(KEY_MODE, "team");
    assert!(!team_push_estimation(payload(), fx.state()).await.unwrap());

    // Not even a URL: still a quiet `false`, not an error.
    let fx = Fixture::new();
    assert!(!team_push_estimation(payload(), fx.state()).await.unwrap());

    assert!(stub.requests().is_empty(), "nothing was sent");
    assert_eq!(fx.get(KEY_ESTIMATIONS_SENT), None);
}

#[tokio::test]
async fn team_push_estimation_without_a_url_fails_when_it_should_be_sent() {
    let fx = Fixture::new();
    fx.set(KEY_MODE, "team");
    fx.set(KEY_ACCESS_TOKEN, "jwt-1");
    assert_err(
        team_push_estimation(payload(), fx.state()).await,
        "no_url",
        NO_URL,
    );
}

#[tokio::test]
async fn team_push_estimation_posts_with_the_bearer_token_and_counts_the_ack() {
    for mode in ["team", "both"] {
        let fx = Fixture::new();
        let stub = Stub::start(estimations_ok).await;
        fx.enrolled_on(&stub.url, mode);

        assert!(team_push_estimation(payload(), fx.state()).await.unwrap());

        let [seen] = stub.requests().try_into().expect("one request");
        assert_eq!(
            (seen.method.as_str(), seen.path.as_str()),
            ("POST", "/api/v1/estimations")
        );
        assert_eq!(seen.header("authorization"), Some("Bearer jwt-1"));
        assert_eq!(seen.header("content-type"), Some("application/json"));
        assert_eq!(seen.json(), payload());
        assert_eq!(fx.get(KEY_ESTIMATIONS_SENT).as_deref(), Some("1"));
        assert_rfc3339(fx.get(KEY_LAST_SEEN_AT));

        assert!(team_push_estimation(payload(), fx.state()).await.unwrap());
        assert_eq!(
            fx.get(KEY_ESTIMATIONS_SENT).as_deref(),
            Some("2"),
            "mode {mode}"
        );
        assert_eq!(
            fx.get(KEY_ACCESS_TOKEN).as_deref(),
            Some("jwt-1"),
            "tokens untouched"
        );
    }
}

#[tokio::test]
async fn team_push_estimation_answers_false_and_counts_nothing_when_the_server_does_not_ack() {
    let fx = Fixture::new();
    let stub = Stub::always(200, NO_ACK).await;
    fx.enrolled_on(&stub.url, "team");

    assert!(!team_push_estimation(payload(), fx.state()).await.unwrap());

    assert_eq!(fx.get(KEY_ESTIMATIONS_SENT), None);
    assert_eq!(fx.get(KEY_LAST_SEEN_AT), None);
}

#[tokio::test]
async fn team_push_estimation_refreshes_the_tokens_on_a_401_and_retries_once() {
    let fx = Fixture::new();
    let stub = Stub::start(|seen, n| match (n, seen.path.as_str()) {
        (0, "/api/v1/estimations") => (401, "{}".to_string()),
        (1, "/api/v1/refresh") => (200, REFRESHED.to_string()),
        (2, "/api/v1/estimations") => (200, ACK.to_string()),
        _ => (500, "unexpected".to_string()),
    })
    .await;
    fx.enrolled_on(&stub.url, "team");

    assert!(team_push_estimation(payload(), fx.state()).await.unwrap());

    let seen = stub.requests();
    assert_eq!(seen.len(), 3);
    assert_eq!(seen[0].header("authorization"), Some("Bearer jwt-1"));
    assert_eq!(seen[1].path, "/api/v1/refresh");
    assert_eq!(seen[1].json(), json!({"refresh_token": "rt-1"}));
    assert_eq!(seen[1].header("authorization"), None);
    assert_eq!(seen[2].header("authorization"), Some("Bearer jwt-new"));
    assert_eq!(seen[2].json(), payload());
    assert_eq!(fx.get(KEY_ACCESS_TOKEN).as_deref(), Some("jwt-new"));
    assert_eq!(fx.get(KEY_REFRESH_TOKEN).as_deref(), Some("rt-new"));
    assert_eq!(fx.get(KEY_ESTIMATIONS_SENT).as_deref(), Some("1"));
}

#[tokio::test]
async fn team_push_estimation_fails_with_unauthorized_without_a_refresh_token() {
    let fx = Fixture::new();
    let stub = Stub::always(401, "{}").await;
    fx.enrolled_on(&stub.url, "team");
    fx.state()
        .team_settings
        .lock()
        .unwrap()
        .delete(KEY_REFRESH_TOKEN)
        .unwrap();

    assert_err(
        team_push_estimation(payload(), fx.state()).await,
        "unauthorized",
        UNAUTHORIZED,
    );

    assert_eq!(stub.requests().len(), 1, "no refresh attempt");
}

#[tokio::test]
async fn team_push_estimation_does_not_log_out_when_the_refresh_is_refused() {
    let fx = Fixture::new();
    let stub = Stub::always(401, "{}").await;
    fx.enrolled_on(&stub.url, "team");

    assert_err(
        team_push_estimation(payload(), fx.state()).await,
        "unauthorized",
        UNAUTHORIZED,
    );

    assert_eq!(stub.requests().len(), 2, "the push, then one refresh");
    // Unlike the browser extension, the app keeps the session it cannot use.
    assert_eq!(fx.get(KEY_ACCESS_TOKEN).as_deref(), Some("jwt-1"));
    assert_eq!(fx.get(KEY_REFRESH_TOKEN).as_deref(), Some("rt-1"));
    assert!(fx.status()["enrolled"].as_bool().unwrap());
}

#[tokio::test]
async fn team_push_estimation_quirk_loses_the_new_tokens_when_the_retry_after_a_refresh_fails() {
    let fx = Fixture::new();
    let stub = Stub::start(|seen, n| match (n, seen.path.as_str()) {
        (0, "/api/v1/estimations") => (401, "{}".to_string()),
        (1, "/api/v1/refresh") => (200, REFRESHED.to_string()),
        _ => (500, "boom".to_string()),
    })
    .await;
    fx.enrolled_on(&stub.url, "team");

    assert_err(
        team_push_estimation(payload(), fx.state()).await,
        "http_error",
        "erreur HTTP 500: boom",
    );

    // The server rotated the refresh token, but the pair it sent is dropped.
    assert_eq!(fx.get(KEY_ACCESS_TOKEN).as_deref(), Some("jwt-1"));
    assert_eq!(fx.get(KEY_REFRESH_TOKEN).as_deref(), Some("rt-1"));
    assert_eq!(fx.get(KEY_ESTIMATIONS_SENT), None);
}

#[tokio::test]
async fn team_push_estimation_maps_the_failures_of_the_server() {
    for (status, body, code, message) in [
        (
            500,
            "boom",
            "http_error",
            "erreur HTTP 500: boom".to_string(),
        ),
        (
            400,
            "bad payload",
            "bad_request",
            "requête invalide: bad payload".to_string(),
        ),
        (200, "not json", "transport", String::new()),
    ] {
        let fx = Fixture::new();
        let stub = Stub::always(status, body).await;
        fx.enrolled_on(&stub.url, "team");

        let result = team_push_estimation(payload(), fx.state()).await;

        if message.is_empty() {
            assert_err_starts(result, code, "erreur transport:");
        } else {
            assert_err(result, code, &message);
        }
        assert_eq!(fx.get(KEY_ESTIMATIONS_SENT), None);
        assert_eq!(fx.get(KEY_LAST_SEEN_AT), None);
    }
}

#[tokio::test]
async fn team_push_estimation_fails_with_transport_when_the_server_is_unreachable() {
    let fx = Fixture::new();
    fx.enrolled_on(&closed_port_url().await, "team");
    assert_err_starts(
        team_push_estimation(payload(), fx.state()).await,
        "transport",
        "erreur transport:",
    );
    assert_eq!(fx.get(KEY_ESTIMATIONS_SENT), None);
}
