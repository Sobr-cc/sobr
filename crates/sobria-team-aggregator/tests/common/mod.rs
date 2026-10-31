//! Shared harness for the SOB-10 characterization tests.
//!
//! Each test starts a real HTTPS server on a random port, sends requests and
//! records every response (status, content type, body). Values the server
//! generates at random (ids, tokens, codes, timestamps) are checked for their
//! shape and replaced by a placeholder, so the recorded file is stable. A
//! random id keeps the same placeholder everywhere in a test (`<ulid#1>`), so
//! a response that returns the wrong id still shows. The recording is then
//! compared with `tests/golden/<name>.json`.

#![allow(dead_code)]

#[path = "../../../../test-support/golden.rs"]
pub mod golden;

use std::{collections::HashMap, net::TcpListener, sync::Mutex, time::Duration};

use chrono::{DateTime, Utc};
use reqwest::RequestBuilder;
use serde_json::{json, Value};
use sobria_team_aggregator::{
    commands::{
        code,
        init::{self, CFG_JWT_SIGNING_KEY},
        serve,
    },
    config::DataPaths,
    server::auth::jwt::{self, Role},
    storage::{
        admins,
        estimations::{self, NewEstimation},
        users, Storage,
    },
};
use tempfile::TempDir;

pub const ADMIN_USERNAME: &str = "admin";
pub const ADMIN_PASSWORD: &str = "admin-pw-not-secret";
pub const USER_PASSWORD: &str = "user-password-strong";

/// Ids of the seeded employees and estimations start with this prefix; they
/// are fixed, so they are kept readable in the recordings.
const SEEDED_ID_PREFIX: &str = "01HZ";

/// Timestamps that the tests seed are in May 2026 and stay readable; the clock
/// of the server is always later.
const SEEDED_TIMESTAMP_PREFIX: &str = "2026-05-";

/// Keys whose string value is a timestamp chosen by the server (clock).
const SERVER_TIMESTAMP_KEYS: &[&str] = &[
    "created_at",
    "last_seen_at",
    "expires_at",
    "access_expires_at",
    "refresh_expires_at",
    "triggered_at",
    "notified_at",
    "disabled_at",
];

pub struct Server {
    pub paths: DataPaths,
    pub base: String,
    pub client: reqwest::Client,
    /// Runs the server: the accept loop, the retention task and every
    /// connection. `Drop` shuts it down, which closes the database.
    runtime: Option<tokio::runtime::Runtime>,
    log: Mutex<Vec<Value>>,
    /// Random value -> placeholder, in order of first appearance.
    placeholders: Mutex<HashMap<String, String>>,
    /// Last field: fields drop in order, so the folder is removed after the
    /// server was shut down (a database file cannot be removed while it is
    /// open on Windows).
    _dir: TempDir,
}

/// The seeded estimations are from May 2026. The purge of the server keeps
/// the last 730 days by default, so it would delete them from mid-2028: the
/// tests keep 100 years.
const TEST_RETENTION_DAYS: &str = "36500";

impl Server {
    /// `init` + HTTPS server on 127.0.0.1:0, ready when `/health` answers.
    pub async fn start() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = DataPaths::new(dir.path());
        init::run(&paths, ADMIN_USERNAME, ADMIN_PASSWORD, false).expect("init");
        // Before the server starts: its purge runs at boot.
        Storage::open(&paths.db())
            .expect("open storage")
            .set_config("retention_days", TEST_RETENTION_DAYS)
            .expect("retention");

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let base = format!("https://{}", listener.local_addr().expect("addr"));
        let client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(20))
            .build()
            .expect("client");
        let served = paths.clone();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("server runtime");
        runtime.spawn(async move {
            let _ = serve::run_with_listener(listener, &served).await;
        });

        // The runtime is owned by `Server` from here on, so that a failed
        // boot below still goes through `Drop` instead of dropping the runtime
        // inside the async test (which panics).
        let server = Self {
            _dir: dir,
            paths,
            base,
            client,
            runtime: Some(runtime),
            log: Mutex::new(Vec::new()),
            placeholders: Mutex::new(HashMap::new()),
        };
        let mut ready = false;
        for _ in 0..50 {
            let health = server
                .client
                .get(format!("{}/health", server.base))
                .send()
                .await;
            if health.is_ok_and(|r| r.status().is_success()) {
                ready = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(ready, "the server never answered /health");
        server
    }

    /// A direct handle on the database (to seed data or change the config).
    pub fn storage(&self) -> Storage {
        Storage::open(&self.paths.db()).expect("open storage")
    }

    /// Writes a value of the configuration straight in the database. Unlike
    /// the CLI (`config set`), nothing is validated: the tests use it to reach
    /// states that the CLI refuses (k under its floor, an unreadable value, the
    /// identified policy without attestation).
    pub fn set_config(&self, key: &str, value: &str) {
        self.storage().set_config(key, value).expect("config");
    }

    /// `count` enrollment codes (clear text), valid 7 days.
    pub fn create_codes(&self, count: u32) -> Vec<String> {
        code::create_batch(&self.paths, count, 7, ADMIN_USERNAME)
            .expect("create codes")
            .into_iter()
            .map(|c| c.code)
            .collect()
    }

    pub fn get(&self, path: &str) -> RequestBuilder {
        self.client.get(format!("{}{path}", self.base))
    }
    pub fn post(&self, path: &str) -> RequestBuilder {
        self.client.post(format!("{}{path}", self.base))
    }
    pub fn put(&self, path: &str) -> RequestBuilder {
        self.client.put(format!("{}{path}", self.base))
    }
    pub fn delete(&self, path: &str) -> RequestBuilder {
        self.client.delete(format!("{}{path}", self.base))
    }

    /// Sends the request, records the masked response, returns the raw
    /// `(status, body)` so the test can chain on ids and tokens.
    pub async fn record(&self, label: &str, request: RequestBuilder) -> (u16, Value) {
        let (status, content_type, text) = send(request).await;
        // A body that is not valid JSON is recorded as text, so that a change
        // shows in the diff instead of aborting the test.
        let body = match serde_json::from_str(&text) {
            Ok(json) if content_type == "application/json" => json,
            _ => Value::String(strip_position(&text)),
        };
        let mut masked = body.clone();
        self.mask(&mut masked);
        self.push(json!({
            "request": label,
            "status": status,
            "content_type": content_type,
            "body": masked,
        }));
        (status, body)
    }

    /// Records a request without a token, then the same request with `token`
    /// (labels `"<prefix>: no auth"` and `"<prefix>: <who>"`).
    pub async fn record_refused(
        &self,
        prefix: &str,
        who: &str,
        token: &str,
        request: impl Fn() -> RequestBuilder,
    ) {
        self.record(&format!("{prefix}: no auth"), request()).await;
        self.record(&format!("{prefix}: {who}"), request().bearer_auth(token))
            .await;
    }

    /// Like [`Self::record`] but keeps only status and content type (for
    /// bodies that depend on the environment, such as the embedded UI).
    pub async fn record_status(&self, label: &str, request: RequestBuilder) -> u16 {
        let (status, content_type, _) = send(request).await;
        self.push(json!({
            "request": label,
            "status": status,
            "content_type": content_type,
        }));
        status
    }

    /// Records a file download: status, content type, file name and the body
    /// (text as is, JSON masked, PDF reduced to a check of its header).
    pub async fn record_download(&self, label: &str, request: RequestBuilder) -> u16 {
        let response = request.send().await.expect("request");
        let status = response.status().as_u16();
        let header = |name: &str| {
            response
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_owned()
        };
        let content_type = header("content-type");
        let disposition = header("content-disposition");
        let bytes = response.bytes().await.expect("body");
        let body = match content_type.split(';').next().unwrap_or("").trim() {
            "application/pdf" => {
                assert!(bytes.starts_with(b"%PDF-"), "not a PDF");
                assert!(bytes.len() > 1000, "a PDF report is never this small");
                json!("<pdf>")
            },
            "application/ld+json" | "application/json" => {
                let mut value: Value = serde_json::from_slice(&bytes).expect("JSON body");
                self.mask(&mut value);
                value
            },
            _ => Value::String(String::from_utf8_lossy(&bytes).into_owned()),
        };
        self.push(json!({
            "request": label,
            "status": status,
            "content_type": content_type,
            "content_disposition": disposition,
            "body": body,
        }));
        status
    }

    /// Records a value computed by the test itself (for example a count read
    /// from the database).
    pub fn note(&self, label: &str, value: Value) {
        self.push(json!({ "note": label, "value": value }));
    }

    fn push(&self, entry: Value) {
        self.log.lock().expect("log").push(entry);
    }

    /// Not recorded: sends the request and returns `(status, body)`.
    pub async fn fetch(&self, request: RequestBuilder) -> (u16, Value) {
        send_json(request).await
    }

    fn signing_key(&self) -> String {
        self.storage()
            .get_config(CFG_JWT_SIGNING_KEY)
            .expect("config")
            .expect("signing key")
    }

    /// An admin access token signed with the server key (no login, no hashing).
    pub fn admin_token(&self) -> String {
        let storage = self.storage();
        let admin = admins::find_by_username(storage.connection(), ADMIN_USERNAME)
            .expect("query")
            .expect("admin");
        jwt::issue(&self.signing_key(), &admin.id, Role::Admin, Utc::now()).expect("admin token")
    }

    /// An access token of the employee `user_id`, who must exist
    /// (see [`Self::seed_user`]).
    pub fn user_token(&self, user_id: &str) -> String {
        jwt::issue(&self.signing_key(), user_id, Role::User, Utc::now()).expect("user token")
    }

    /// A well-formed user token signed with a key that is not the server one.
    pub fn forged_user_token(&self, user_id: &str) -> String {
        let other_key = "ab".repeat(32);
        jwt::issue(&other_key, user_id, Role::User, Utc::now()).expect("forged token")
    }

    /// A user token signed with the server key that expired an hour ago.
    pub fn expired_user_token(&self, user_id: &str) -> String {
        let issued = Utc::now() - jwt::ACCESS_TTL - chrono::Duration::hours(1);
        jwt::issue(&self.signing_key(), user_id, Role::User, issued).expect("expired token")
    }

    /// Not recorded: enrolls a user through the API with a fresh code, returns
    /// the response (for the tests of the enrollment and login flows).
    pub async fn enroll(&self, fingerprint: &str, display_name: &str) -> Value {
        let code = self.create_codes(1).remove(0);
        let (status, body) = send_json(self.post("/api/v1/enroll").json(&json!({
            "code": code,
            "password": USER_PASSWORD,
            "fingerprint": fingerprint,
            "display_name": display_name,
        })))
        .await;
        assert_eq!(status, 200, "enroll {fingerprint}: {body}");
        body
    }

    /// Fixed id of seeded employee `n` (see [`Self::seed_user`]).
    pub fn user_id(n: u32) -> String {
        format!("{SEEDED_ID_PREFIX}{}{n:03}", "Z".repeat(19))
    }

    /// Seeds employee `n` (1 to 59) straight in the database: fixed id,
    /// fingerprint `fp-employee-<n>`, name `Employee <n>`, no usable password.
    /// Returns the id.
    pub fn seed_user(&self, n: u32, opted_in: bool) -> String {
        let storage = self.storage();
        let id = Self::user_id(n);
        users::insert(
            storage.connection(),
            &id,
            None,
            &format!("fp-employee-{n}"),
            "$argon2id$not-a-real-hash",
            Some(&format!("Employee {n}")),
            timestamp(&format!("2026-05-01T00:00:{n:02}Z")),
        )
        .expect("seed user");
        if opted_in {
            users::set_share_identified(storage.connection(), &id, true).expect("opt in");
        }
        id
    }

    /// Seeds employees `numbers` (see [`Self::seed_user`]), each with two
    /// estimations on 2026-05-15 and 2026-05-16, at second `n` of 10:00.
    /// Employee 1 has opted in to being identified.
    ///
    /// Employee `n` has: day 1, model `llama-3-1-70b`, method `afnor_sobria`,
    /// project `Alpha`, `0.1 * n` gCO2eq, `0.5 * n` Wh, 2 mL of water; day 2,
    /// model `gpt-4o`, method `ecologits`, project `Beta` when `n` is even and
    /// none otherwise, `0.2 * n` gCO2eq, `0.25 * n` Wh, 3 mL of water. The
    /// three quantities differ, so a column mixed up with another shows.
    pub fn seed_team(&self, numbers: std::ops::RangeInclusive<u32>) {
        for n in numbers {
            let user_id = self.seed_user(n, n == 1);
            let storage = self.storage();
            let weight = f64::from(n);
            let project_day_two = (n % 2 == 0).then_some("Beta");
            let rows = [
                (
                    format!("2026-05-15T10:00:{n:02}Z"),
                    "llama-3-1-70b",
                    "afnor_sobria",
                    (0.1 * weight, 0.5 * weight, 2.0),
                    Some("Alpha"),
                ),
                (
                    format!("2026-05-16T10:00:{n:02}Z"),
                    "gpt-4o",
                    "ecologits",
                    (0.2 * weight, 0.25 * weight, 3.0),
                    project_day_two,
                ),
            ];
            for (index, (ts, model_id, method, (gco2eq, energy_wh, water_ml), project)) in
                rows.into_iter().enumerate()
            {
                let id = format!(
                    "{SEEDED_ID_PREFIX}{}{:03}",
                    "E".repeat(19),
                    n * 2 + index as u32
                );
                estimations::insert(
                    storage.connection(),
                    &NewEstimation {
                        id: &id,
                        user_id: &user_id,
                        ts: timestamp(&ts),
                        method,
                        model_id,
                        tokens_in: 100,
                        tokens_out: 400,
                        gco2eq_p50: gco2eq,
                        gco2eq_p5: Some(gco2eq * 0.5),
                        gco2eq_p95: Some(gco2eq * 1.5),
                        water_ml,
                        energy_wh,
                        region: None,
                        project,
                        raw_payload_json: "{}",
                        received_at: timestamp(&ts),
                    },
                )
                .expect("seed estimation");
            }
        }
    }

    /// Compares everything recorded so far with `tests/golden/<name>.json`.
    pub fn assert_golden(&self, name: &str) {
        let log = self.log.lock().expect("log").clone();
        golden::assert_golden(name, &Value::Array(log));
    }

    /// Replaces server-generated random values by a placeholder, after
    /// checking their shape. A value of an unexpected shape fails the test.
    fn mask(&self, value: &mut Value) {
        match value {
            Value::Object(map) => {
                for (key, child) in map.iter_mut() {
                    self.mask_field(key, child);
                    self.mask(child);
                }
            },
            Value::Array(items) => items.iter_mut().for_each(|item| self.mask(item)),
            Value::String(s) if is_ulid(s) && !s.starts_with(SEEDED_ID_PREFIX) => {
                *s = self.placeholder("ulid", s);
            },
            _ => {},
        }
    }

    fn mask_field(&self, key: &str, value: &mut Value) {
        let Value::String(s) = value else { return };
        let placeholder = match key {
            k if SERVER_TIMESTAMP_KEYS.contains(&k) => {
                DateTime::parse_from_rfc3339(s)
                    .unwrap_or_else(|e| panic!("{key} is not RFC 3339 ({s}): {e}"));
                if s.starts_with(SEEDED_TIMESTAMP_PREFIX) {
                    return;
                }
                "<timestamp>".to_owned()
            },
            "access_token" => {
                assert_eq!(s.split('.').count(), 3, "access_token is not a JWT");
                "<jwt>".to_owned()
            },
            "refresh_token" => {
                assert!(s.contains('.'), "refresh_token has no separator");
                "<refresh_token>".to_owned()
            },
            "code" if s.len() == 12 && s.chars().all(|c| c.is_ascii_digit()) => {
                self.placeholder("code", s)
            },
            k if k == "version" || k.ends_with(":version") => {
                assert_eq!(s, env!("CARGO_PKG_VERSION"), "unexpected version in {key}");
                "<crate_version>".to_owned()
            },
            _ => return,
        };
        *s = placeholder;
    }

    /// `<kind#n>`, `n` counting the distinct values of that kind seen so far.
    fn placeholder(&self, kind: &str, value: &str) -> String {
        let mut known = self.placeholders.lock().expect("placeholders");
        if let Some(existing) = known.get(value) {
            return existing.clone();
        }
        let prefix = format!("<{kind}#");
        let number = known.values().filter(|p| p.starts_with(&prefix)).count() + 1;
        let created = format!("{prefix}{number}>");
        known.insert(value.to_owned(), created.clone());
        created
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.take() {
            // Waits for the tasks to be dropped. Every test uses the
            // multi-thread flavor, which `block_in_place` needs.
            tokio::task::block_in_place(|| runtime.shutdown_timeout(Duration::from_secs(10)));
        }
    }
}

/// Hours between now and an RFC 3339 timestamp, rounded: the lifetime of a
/// token or a code, whatever the exact time of the request.
pub fn hours_from_now(rfc3339: &str) -> f64 {
    let at = DateTime::parse_from_rfc3339(rfc3339).expect("RFC 3339");
    ((at.with_timezone(&Utc) - Utc::now()).num_seconds() as f64 / 3600.0).round()
}

/// Waits until a few seconds after UTC midnight when the test starts just
/// before it: the periods of the alerts are computed from the UTC date.
pub async fn avoid_utc_midnight() {
    let now = Utc::now();
    let seconds_into_day = i64::from(chrono::Timelike::num_seconds_from_midnight(&now));
    let seconds_to_midnight = 86_400 - seconds_into_day;
    if seconds_to_midnight < 10 {
        tokio::time::sleep(Duration::from_secs(seconds_to_midnight.unsigned_abs() + 2)).await;
    }
}

pub fn timestamp(rfc3339: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(rfc3339)
        .expect("fixed timestamp")
        .with_timezone(&Utc)
}

/// `(status, media type, body text)`.
async fn send(request: RequestBuilder) -> (u16, String, String) {
    let response = request.send().await.expect("request");
    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .unwrap_or("")
        .trim()
        .to_owned();
    let text = response.text().await.expect("body");
    (status, content_type, text)
}

async fn send_json(request: RequestBuilder) -> (u16, Value) {
    let (status, _, text) = send(request).await;
    (
        status,
        serde_json::from_str(&text).unwrap_or(Value::String(text)),
    )
}

/// Removes the trailing ` at line N column M` of a serde error: the column
/// depends on the order of the keys of the request, which depends on the
/// `serde_json` features that the whole workspace turns on.
fn strip_position(text: &str) -> String {
    const MARKER: &str = " at line ";
    if let Some(index) = text.rfind(MARKER) {
        let tail = &text[index + MARKER.len()..];
        if let Some((line, column)) = tail.split_once(" column ") {
            let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
            if digits(line) && digits(column) {
                return text[..index].to_owned();
            }
        }
    }
    text.to_owned()
}

fn is_ulid(s: &str) -> bool {
    s.len() == 26
        && s.chars()
            .all(|c| c.is_ascii_digit() || "ABCDEFGHJKMNPQRSTVWXYZ".contains(c))
}
