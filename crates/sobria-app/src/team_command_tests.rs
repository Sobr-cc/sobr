//! Characterization of the `team_*` IPC commands of `main.rs` (SOB-188).
//!
//! The commands are private functions of the binary and take a
//! `tauri::State`, so they are called from here with the state of a mock
//! Tauri app (`tauri::test`). The server of the team is replaced by a small
//! HTTP server on a random local port that records what it receives.
//!
//! The effect of `team_set_accept_invalid_certs` on a TLS connection needs a
//! TLS stub with a self-signed certificate: it is tested in the `tls` module at
//! the end of this file (SOB-200). Up here, only the stored flag is pinned.
//!
//! The error `code` of an `IpcError` is the contract with the UI; its message
//! is human text. The tests check the code exactly and only a fragment of the
//! message (the status and the body of the server).

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
const NO_URL: &str = "URL serveur équipe non configurée";

/// `(status, body of the server, code of the IPC error, fragment of its message)`:
/// what every command that reads an answer of the server does with a failure.
const FAILURES: [(u16, &str, &str, &str); 4] = [
    (500, "boom", "http_error", "500: boom"),
    (400, "bad input", "bad_request", "bad input"),
    (401, "{}", "unauthorized", ""),
    (200, "not json", "transport", ""),
];

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

/// `(status, body)` for a request. It decides on the request alone, like a server.
type Handler = dyn Fn(&Seen) -> (u16, String) + Send + Sync;

/// The server stops with the test: the runtime of `#[tokio::test]` drops its task.
struct Stub {
    url: String,
    seen: Arc<Mutex<Vec<Seen>>>,
}

impl Stub {
    async fn start(handler: impl Fn(&Seen) -> (u16, String) + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let handler: Arc<Handler> = Arc::new(handler);
        let log = Arc::clone(&seen);
        tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                tokio::spawn(answer(socket, Arc::clone(&handler), Arc::clone(&log)));
            }
        });
        Self { url, seen }
    }

    async fn always(status: u16, body: &'static str) -> Self {
        Self::start(move |_| (status, body.to_string())).await
    }

    /// A server that closes every connection at once: any request fails with a
    /// transport error, without depending on a port that is free.
    async fn hangup() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { while listener.accept().await.is_ok() {} });
        Self {
            url,
            seen: Arc::default(),
        }
    }

    fn requests(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
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
    let (status, reply) = handler(&seen);
    log.lock().unwrap().push(seen);
    let response = format!(
        "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
        reply.len()
    );
    let _ = socket.write_all(response.as_bytes()).await;
    let _ = socket.shutdown().await;
}

// ─── The app state behind the commands ───────────────────────────────────────

struct Fixture {
    _dir: tempfile::TempDir,
    app: tauri::App<MockRuntime>,
}

impl Fixture {
    fn new() -> Self {
        // The stub is on localhost: keep a proxy of the machine from catching it.
        static NO_PROXY: Once = Once::new();
        NO_PROXY.call_once(|| {
            let has_proxy = ["HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"]
                .iter()
                .any(|name| std::env::var_os(name).is_some());
            if has_proxy {
                let current = std::env::var("NO_PROXY").unwrap_or_default();
                std::env::set_var("NO_PROXY", format!("{current},127.0.0.1,localhost"));
            }
        });
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

    fn delete(&self, key: &str) {
        self.state()
            .team_settings
            .lock()
            .unwrap()
            .delete(key)
            .unwrap();
    }

    fn get(&self, key: &str) -> Option<String> {
        self.state().team_settings.lock().unwrap().get(key).unwrap()
    }

    /// What the UI gets from `team_status`.
    fn status(&self) -> serde_json::Value {
        serde_json::to_value(team_status(self.state()).unwrap()).unwrap()
    }

    /// A second connection to the database file of the app.
    fn database(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(&self.state().referentiel_path).unwrap()
    }

    /// A user enrolled on `url`, in `mode`, with tokens `jwt-1` / `rt-1`.
    fn enrolled_on(&self, url: &str, mode: &str) {
        self.set(KEY_URL, url);
        self.set(KEY_MODE, mode);
        self.set(KEY_ACCESS_TOKEN, "jwt-1");
        self.set(KEY_REFRESH_TOKEN, "rt-1");
        self.set(KEY_USER_ID, "01HZ-user");
    }

    /// The tokens, the user and the mode set by `enrolled_on(_, "team")` are still there.
    #[track_caller]
    fn assert_still_enrolled(&self) {
        assert_eq!(self.get(KEY_ACCESS_TOKEN).as_deref(), Some("jwt-1"));
        assert_eq!(self.get(KEY_REFRESH_TOKEN).as_deref(), Some("rt-1"));
        assert_eq!(self.get(KEY_USER_ID).as_deref(), Some("01HZ-user"));
        assert_eq!(self.get(KEY_MODE).as_deref(), Some("team"));
    }
}

/// The IPC error has this `code` (the contract with the UI) and a message that
/// contains `fragment`.
#[track_caller]
fn assert_err<T: std::fmt::Debug>(result: IpcResult<T>, code: &str, fragment: &str) {
    let error = result.expect_err("the command should fail");
    assert_eq!(error.code, code, "message: {}", error.message);
    assert!(
        error.message.contains(fragment),
        "message: {}",
        error.message
    );
}

#[track_caller]
fn assert_rfc3339(value: Option<String>) {
    let value = value.expect("a timestamp");
    chrono::DateTime::parse_from_rfc3339(&value).expect("RFC 3339");
}

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
fn team_status_is_enrolled_only_when_an_access_token_is_stored() {
    let fx = Fixture::new();
    fx.set(KEY_REFRESH_TOKEN, "rt");
    fx.set(KEY_USER_ID, "u-1");
    assert_eq!(fx.status()["enrolled"], json!(false));
    fx.set(KEY_ACCESS_TOKEN, "jwt");
    assert_eq!(fx.status()["enrolled"], json!(true));
}

#[test]
fn team_status_quirk_an_empty_access_token_counts_as_enrolled() {
    let fx = Fixture::new();
    fx.set(KEY_ACCESS_TOKEN, "");
    assert_eq!(fx.status()["enrolled"], json!(true));
}

#[test]
fn team_settings_are_read_under_the_keys_of_existing_installs() {
    // Rows written by an earlier version of the app, with literal key names:
    // renaming a key would log every existing install out.
    let fx = Fixture::new();
    let db = fx.database();
    for (key, value) in [
        ("url", "https://team.example"),
        ("mode", "both"),
        ("user_id", "u-1"),
        ("access_token", "jwt"),
        ("refresh_token", "rt"),
        ("fingerprint", "app-win-1"),
        ("enrolled_at", "2026-05-16T08:00:00Z"),
        ("accept_invalid_certs", "1"),
        ("last_seen_at", "2026-05-16T09:00:00Z"),
        ("estimations_sent", "7"),
    ] {
        db.execute(
            "INSERT INTO team_settings (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )
        .unwrap();
    }

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

#[tokio::test]
async fn a_broken_settings_store_gives_internal_or_storage_errors() {
    let fx = Fixture::new();
    fx.database()
        .execute("DROP TABLE team_settings", [])
        .unwrap();

    assert_err(team_status(fx.state()), "internal", "team_status");
    assert_err(
        team_set_url("https://team.example".to_string(), fx.state()),
        "internal",
        "",
    );
    assert_err(
        team_set_mode("team".to_string(), fx.state()),
        "internal",
        "",
    );
    assert_err(
        team_set_accept_invalid_certs(true, fx.state()),
        "internal",
        "",
    );
    assert_err(team_logout(fx.state()), "internal", "team_logout");
    assert_err(
        team_push_estimation(payload(), fx.state()).await,
        "internal",
        "",
    );
    // The client reads the store through its own error type.
    assert_err(team_ping(fx.state()).await, "storage", "");
    assert_err(enroll(&fx, None).await, "storage", "");
}

// ─── team_set_url, team_set_mode, team_set_accept_invalid_certs ─────────────

#[test]
fn team_set_url_refuses_other_schemes_and_a_bare_host() {
    let fx = Fixture::new();
    for url in [
        "http://team.example",
        "ftp://team.example",
        "team.example",
        "",
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
fn team_set_url_quirk_refuses_a_padded_or_upper_case_https() {
    let fx = Fixture::new();
    for url in [" https://team.example", "HTTPS://team.example"] {
        assert_err(team_set_url(url.to_string(), fx.state()), "bad_request", "");
    }
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
fn team_set_mode_stores_the_three_modes() {
    let fx = Fixture::new();
    for mode in ["team", "both", "local", "team"] {
        team_set_mode(mode.to_string(), fx.state()).unwrap();
        assert_eq!(fx.get(KEY_MODE).as_deref(), Some(mode));
    }
}

#[test]
fn team_set_mode_quirk_any_other_value_is_stored_as_local() {
    let fx = Fixture::new();
    for given in ["TEAM", "everything", ""] {
        team_set_mode("both".to_string(), fx.state()).unwrap();
        team_set_mode(given.to_string(), fx.state()).unwrap();
        assert_eq!(
            fx.get(KEY_MODE).as_deref(),
            Some("local"),
            "given {given:?}"
        );
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
    for (status, body, code, fragment) in FAILURES {
        let fx = Fixture::new();
        let stub = Stub::always(status, body).await;
        fx.set(KEY_URL, &stub.url);

        assert_err(team_ping(fx.state()).await, code, fragment);
        assert_eq!(
            fx.get(KEY_LAST_SEEN_AT),
            None,
            "a failed ping is not a sign of life"
        );
    }
}

#[tokio::test]
async fn team_ping_fails_with_transport_when_the_server_hangs_up() {
    let fx = Fixture::new();
    fx.set(KEY_URL, &Stub::hangup().await.url);
    assert_err(team_ping(fx.state()).await, "transport", "");
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
    assert_eq!(fx.status()["enrolled"], json!(false));
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
    // An enrollment does not mark the server as seen (unlike a ping or a push).
    assert_eq!(fx.get(KEY_LAST_SEEN_AT), None);
    assert_eq!(fx.status()["enrolled"], json!(true));
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
async fn team_enroll_over_an_existing_session_replaces_both_tokens() {
    let fx = Fixture::new();
    let stub = Stub::always(200, ENROLLED).await;
    fx.enrolled_on(&stub.url, "team");
    fx.set(KEY_ACCESS_TOKEN, "jwt-old");
    fx.set(KEY_REFRESH_TOKEN, "rt-old");

    enroll(&fx, None).await.unwrap();

    assert_eq!(fx.get(KEY_ACCESS_TOKEN).as_deref(), Some("jwt-1"));
    assert_eq!(fx.get(KEY_REFRESH_TOKEN).as_deref(), Some("ulid.uuid-1"));
}

#[tokio::test]
async fn team_enroll_maps_the_refusals_and_keeps_the_session_it_had() {
    let conflict = r#"{"error":"empreinte déjà enrôlée","code":"fingerprint_already_enrolled"}"#;
    let refusals = [(409, conflict, "http_error", "409")]
        .into_iter()
        .chain(FAILURES);
    for (status, body, code, fragment) in refusals {
        let fx = Fixture::new();
        let stub = Stub::always(status, body).await;
        fx.enrolled_on(&stub.url, "team");

        assert_err(enroll(&fx, None).await, code, fragment);

        fx.assert_still_enrolled();
        assert_eq!(
            fx.get(KEY_FINGERPRINT),
            None,
            "nothing of the failed enrollment is stored"
        );
    }
}

#[tokio::test]
async fn team_enroll_fails_with_transport_on_an_incomplete_answer() {
    let fx = Fixture::new();
    let stub = Stub::always(200, r#"{"user_id":"u"}"#).await;
    fx.set(KEY_URL, &stub.url);
    assert_err(enroll(&fx, None).await, "transport", "");
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

/// The server in the refresh scenario: `jwt-1` has expired, the refresh gives
/// `jwt-new`, and `after_refresh` is what the estimations route answers to it.
fn refreshing_server(after_refresh: (u16, &'static str)) -> impl Fn(&Seen) -> (u16, String) {
    move |seen| match (seen.path.as_str(), seen.header("authorization")) {
        ("/api/v1/estimations", Some("Bearer jwt-1")) => (401, "{}".to_string()),
        ("/api/v1/refresh", _) => (200, REFRESHED.to_string()),
        ("/api/v1/estimations", Some("Bearer jwt-new")) => {
            (after_refresh.0, after_refresh.1.to_string())
        },
        _ => (500, "unexpected".to_string()),
    }
}

#[tokio::test]
async fn team_push_estimation_is_not_sent_in_local_mode_or_when_not_enrolled() {
    let stub = Stub::always(200, ACK).await;

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
        let stub = Stub::always(200, ACK).await;
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
    let stub = Stub::start(refreshing_server((200, ACK))).await;
    fx.enrolled_on(&stub.url, "team");

    assert!(team_push_estimation(payload(), fx.state()).await.unwrap());

    let seen = stub.requests();
    assert_eq!(seen.len(), 3, "the push, the refresh, the retry");
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
    fx.delete(KEY_REFRESH_TOKEN);

    assert_err(
        team_push_estimation(payload(), fx.state()).await,
        "unauthorized",
        "",
    );

    assert_eq!(stub.requests().len(), 1, "no refresh attempt");
}

#[tokio::test]
async fn team_push_estimation_quirk_keeps_a_session_it_cannot_use_when_the_refresh_is_refused() {
    let fx = Fixture::new();
    let stub = Stub::always(401, "{}").await;
    fx.enrolled_on(&stub.url, "team");

    assert_err(
        team_push_estimation(payload(), fx.state()).await,
        "unauthorized",
        "",
    );

    assert_eq!(stub.requests().len(), 2, "the push, then one refresh");
    // Unlike the browser extension, the app does not log the user out.
    fx.assert_still_enrolled();
    assert_eq!(fx.status()["enrolled"], json!(true));
}

#[tokio::test]
async fn team_push_estimation_quirk_loses_the_new_tokens_when_the_retry_after_a_refresh_fails() {
    let fx = Fixture::new();
    let stub = Stub::start(refreshing_server((500, "boom"))).await;
    fx.enrolled_on(&stub.url, "team");

    assert_err(
        team_push_estimation(payload(), fx.state()).await,
        "http_error",
        "500: boom",
    );

    // The server rotated the refresh token, but the pair it sent is dropped.
    fx.assert_still_enrolled();
    assert_eq!(fx.get(KEY_ESTIMATIONS_SENT), None);
}

#[tokio::test]
async fn team_push_estimation_maps_the_failures_of_the_server() {
    for (status, body, code, fragment) in FAILURES {
        let fx = Fixture::new();
        let stub = Stub::always(status, body).await;
        fx.enrolled_on(&stub.url, "team");

        assert_err(
            team_push_estimation(payload(), fx.state()).await,
            code,
            fragment,
        );

        assert_eq!(fx.get(KEY_ESTIMATIONS_SENT), None);
        assert_eq!(fx.get(KEY_LAST_SEEN_AT), None);
    }
}

#[tokio::test]
async fn team_push_estimation_fails_with_transport_when_the_server_hangs_up() {
    let fx = Fixture::new();
    fx.enrolled_on(&Stub::hangup().await.url, "team");
    assert_err(
        team_push_estimation(payload(), fx.state()).await,
        "transport",
        "",
    );
    assert_eq!(fx.get(KEY_ESTIMATIONS_SENT), None);
}

// ─── team_ping over TLS (SOB-200) ────────────────────────────────────────────

/// The "accept invalid certificates" setting on a real TLS connection.
///
/// `rcgen` and `tokio-rustls` are dev-dependencies for this only (see the
/// comment in `Cargo.toml`).
mod tls {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use tokio_rustls::rustls::{
        crypto::ring::default_provider,
        pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer},
        ServerConfig,
    };
    use tokio_rustls::TlsAcceptor;

    use super::*;

    /// A server with a self-signed certificate that answers `HEALTH` to any
    /// request. It counts the handshakes it completes and the ones the client
    /// breaks off, which is what a client does when it does not trust the
    /// certificate.
    struct TlsStub {
        url: String,
        completed: Arc<AtomicUsize>,
        refused: Arc<AtomicUsize>,
    }

    impl TlsStub {
        async fn start() -> Self {
            let certified = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_string()])
                .expect("a self-signed certificate");
            let key = PrivatePkcs8KeyDer::from(certified.key_pair.serialize_der());
            let config = ServerConfig::builder_with_provider(Arc::new(default_provider()))
                .with_safe_default_protocol_versions()
                .expect("TLS versions")
                .with_no_client_auth()
                .with_single_cert(
                    vec![certified.cert.der().clone()],
                    PrivateKeyDer::Pkcs8(key),
                )
                .expect("the certificate and its key");
            let acceptor = TlsAcceptor::from(Arc::new(config));

            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let url = format!("https://{}", listener.local_addr().unwrap());
            let completed = Arc::new(AtomicUsize::new(0));
            let refused = Arc::new(AtomicUsize::new(0));
            let counts = (Arc::clone(&completed), Arc::clone(&refused));
            tokio::spawn(async move {
                while let Ok((socket, _)) = listener.accept().await {
                    let acceptor = acceptor.clone();
                    let (completed, refused) = counts.clone();
                    tokio::spawn(async move {
                        let Ok(mut stream) = acceptor.accept(socket).await else {
                            refused.fetch_add(1, Ordering::SeqCst);
                            return;
                        };
                        completed.fetch_add(1, Ordering::SeqCst);
                        let mut head = Vec::new();
                        let mut chunk = [0u8; 1024];
                        while !head.windows(4).any(|w| w == b"\r\n\r\n") {
                            match stream.read(&mut chunk).await {
                                Ok(0) | Err(_) => return,
                                Ok(n) => head.extend_from_slice(&chunk[..n]),
                            }
                        }
                        let response = format!(
                            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{HEALTH}",
                            HEALTH.len()
                        );
                        let _ = stream.write_all(response.as_bytes()).await;
                        let _ = stream.shutdown().await;
                    });
                }
            });
            Self {
                url,
                completed,
                refused,
            }
        }

        fn completed(&self) -> usize {
            self.completed.load(Ordering::SeqCst)
        }

        /// The server learns of a refused handshake a moment after the client.
        async fn wait_for_refused(&self, expected: usize) {
            for _ in 0..250 {
                if self.refused.load(Ordering::SeqCst) >= expected {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            assert_eq!(self.refused.load(Ordering::SeqCst), expected);
        }
    }

    /// The team URL is set with the real command, as the UI does.
    fn fixture_on(server: &TlsStub) -> Fixture {
        let fx = Fixture::new();
        team_set_url(server.url.clone(), fx.state()).unwrap();
        fx
    }

    #[tokio::test]
    async fn team_ping_over_tls_fails_with_transport_when_the_certificate_is_self_signed() {
        let server = TlsStub::start().await;
        let fx = fixture_on(&server);

        // The flag was never set: it is off.
        assert_err(team_ping(fx.state()).await, "transport", "");

        // The cause is the certificate: the client broke the handshake off.
        server.wait_for_refused(1).await;
        assert_eq!(server.completed(), 0);
        assert_eq!(fx.get(KEY_LAST_SEEN_AT), None);
    }

    #[tokio::test]
    async fn team_ping_over_tls_returns_the_health_when_invalid_certificates_are_accepted() {
        let server = TlsStub::start().await;
        let fx = fixture_on(&server);
        team_set_accept_invalid_certs(true, fx.state()).unwrap();

        let health = team_ping(fx.state()).await.unwrap();

        assert!(health.ok);
        assert_eq!(health.version, "0.9.0");
        assert_eq!(server.completed(), 1);
        assert_eq!(server.refused.load(Ordering::SeqCst), 0);
        assert_rfc3339(fx.get(KEY_LAST_SEEN_AT));
    }

    #[tokio::test]
    async fn team_ping_over_tls_follows_the_flag_each_time_it_is_turned_on_and_off() {
        let server = TlsStub::start().await;
        let fx = fixture_on(&server);

        team_set_accept_invalid_certs(true, fx.state()).unwrap();
        assert!(team_ping(fx.state()).await.unwrap().ok);

        team_set_accept_invalid_certs(false, fx.state()).unwrap();
        assert_err(team_ping(fx.state()).await, "transport", "");
        server.wait_for_refused(1).await;

        team_set_accept_invalid_certs(true, fx.state()).unwrap();
        assert!(team_ping(fx.state()).await.unwrap().ok);
        assert_eq!(
            server.completed(),
            2,
            "one handshake for each accepted ping"
        );
    }
}
