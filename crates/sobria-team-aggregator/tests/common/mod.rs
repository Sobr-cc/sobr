//! Shared harness for the SOB-10 characterization tests.
//!
//! Each test starts a real HTTPS server on a random port, sends requests and
//! records every response (status, content type, body). Values the server
//! generates at random (ids, tokens, codes, timestamps) are checked for their
//! shape and replaced by a placeholder, so the recorded file is stable.
//! The recording is then compared with `tests/golden/<name>.json`.

#![allow(dead_code)]

pub mod golden;

use std::{net::TcpListener, sync::Mutex, time::Duration};

use reqwest::RequestBuilder;
use serde_json::{json, Value};
use chrono::{DateTime, Utc};
use sobria_team_aggregator::{
    commands::{code, init, serve},
    config::DataPaths,
    storage::{
        estimations::{self, NewEstimation},
        users, Storage,
    },
};
use tempfile::TempDir;

pub const ADMIN_USERNAME: &str = "admin";
pub const ADMIN_PASSWORD: &str = "admin-pw-not-secret";
pub const USER_PASSWORD: &str = "user-password-strong";

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
    _dir: TempDir,
    pub paths: DataPaths,
    pub base: String,
    pub client: reqwest::Client,
    handle: tokio::task::JoinHandle<()>,
    log: Mutex<Vec<Value>>,
}

impl Server {
    /// `init` + HTTPS server on 127.0.0.1:0, ready when `/health` answers.
    pub async fn start() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = DataPaths::new(dir.path());
        init::run(&paths, ADMIN_USERNAME, ADMIN_PASSWORD, false).expect("init");

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let base = format!("https://{}", listener.local_addr().expect("addr"));
        let served = paths.clone();
        let handle = tokio::spawn(async move {
            let _ = serve::run_with_listener(listener, &served).await;
        });
        let client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(20))
            .build()
            .expect("client");

        for _ in 0..50 {
            if let Ok(r) = client.get(format!("{base}/health")).send().await {
                if r.status().is_success() {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Self {
            _dir: dir,
            paths,
            base,
            client,
            handle,
            log: Mutex::new(Vec::new()),
        }
    }

    /// A direct handle on the database (to seed data or change the config).
    pub fn storage(&self) -> Storage {
        Storage::open(&self.paths.db()).expect("open storage")
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
        let body = if content_type == "application/json" {
            serde_json::from_str(&text).expect("a JSON content type must carry JSON")
        } else {
            Value::String(text)
        };
        let mut masked = body.clone();
        mask(&mut masked);
        self.push(json!({
            "request": label,
            "status": status,
            "content_type": content_type,
            "body": masked,
        }));
        (status, body)
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
                mask(&mut value);
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

    /// Not recorded: an admin access token.
    pub async fn admin_token(&self) -> String {
        let (status, body) = send_json(
            self.post("/api/v1/login").json(
                &json!({"username": ADMIN_USERNAME, "password": ADMIN_PASSWORD, "role": "admin"}),
            ),
        )
        .await;
        assert_eq!(status, 200, "admin login: {body}");
        body["access_token"].as_str().expect("access_token").to_owned()
    }

    /// Not recorded: enrolls a user with a fresh code, returns the response.
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

    /// Not recorded: sends the request and returns `(status, body)`.
    pub async fn fetch(&self, request: RequestBuilder) -> (u16, Value) {
        send_json(request).await
    }

    /// Seeds employees `numbers` straight in the database (no password, no
    /// Argon2), each with two estimations on 2026-05-15 and 2026-05-16.
    /// Employee 1 has opted in to being identified. Ids are fixed.
    ///
    /// Employee `n` has: day 1, model `llama-3-1-70b`, method `afnor_sobria`,
    /// project `Alpha`, `0.1 * n` gCO2eq; day 2, model `gpt-4o`, method
    /// `ecologits`, project `Beta` when `n` is even and none otherwise,
    /// `0.2 * n` gCO2eq.
    pub fn seed_team(&self, numbers: std::ops::RangeInclusive<u32>) {
        let storage = self.storage();
        let conn = storage.connection();
        for n in numbers {
            let weight = f64::from(n);
            let user_id = Self::user_id(n);
            users::insert(
                conn,
                &user_id,
                None,
                &format!("fp-employee-{n}"),
                "$argon2id$not-a-real-hash",
                Some(&format!("Employee {n}")),
                timestamp("2026-05-01T00:00:00Z"),
            )
            .expect("seed user");
            if n == 1 {
                users::set_share_identified(conn, &user_id, true).expect("opt in");
            }
            let project_day_two = (n % 2 == 0).then_some("Beta");
            for (index, (ts, model_id, method, gco2eq, project)) in [
                (
                    "2026-05-15T10:00:00Z",
                    "llama-3-1-70b",
                    "afnor_sobria",
                    0.1 * weight,
                    Some("Alpha"),
                ),
                (
                    "2026-05-16T10:00:00Z",
                    "gpt-4o",
                    "ecologits",
                    0.2 * weight,
                    project_day_two,
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let id = format!("01HZ{}{:03}", "E".repeat(19), n * 2 + index as u32);
                estimations::insert(
                    conn,
                    &NewEstimation {
                        id: &id,
                        user_id: &user_id,
                        ts: timestamp(ts),
                        method,
                        model_id,
                        tokens_in: 100,
                        tokens_out: 400,
                        gco2eq_p50: gco2eq,
                        gco2eq_p5: Some(gco2eq * 0.5),
                        gco2eq_p95: Some(gco2eq * 1.5),
                        water_ml: 1.0,
                        energy_wh: gco2eq,
                        region: None,
                        project,
                        raw_payload_json: "{}",
                        received_at: timestamp(ts),
                    },
                )
                .expect("seed estimation");
            }
        }
    }

    /// Fixed id of seeded employee `n` (see [`Self::seed_team`]).
    pub fn user_id(n: u32) -> String {
        format!("01HZ{}{n:03}", "Z".repeat(19))
    }

    /// Compares everything recorded so far with `tests/golden/<name>.json`.
    pub fn assert_golden(&self, name: &str) {
        let log = self.log.lock().expect("log").clone();
        golden::assert_golden(name, &Value::Array(log));
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.handle.abort();
    }
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
    (status, serde_json::from_str(&text).unwrap_or(Value::String(text)))
}

fn timestamp(rfc3339: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(rfc3339)
        .expect("fixed timestamp")
        .with_timezone(&Utc)
}

fn is_ulid(s: &str) -> bool {
    s.len() == 26
        && s.chars()
            .all(|c| c.is_ascii_digit() || ("ABCDEFGHJKMNPQRSTVWXYZ".contains(c)))
}

/// Replaces server-generated random values by a placeholder, after checking
/// their shape. A value of an unexpected shape fails the test.
pub fn mask(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                mask_field(key, child);
                mask(child);
            }
        },
        Value::Array(items) => items.iter_mut().for_each(mask),
        Value::String(s) if is_ulid(s) => *s = "<ulid>".into(),
        _ => {},
    }
}

fn mask_field(key: &str, value: &mut Value) {
    let Value::String(s) = value else { return };
    let placeholder = match key {
        k if SERVER_TIMESTAMP_KEYS.contains(&k) => {
            chrono::DateTime::parse_from_rfc3339(s)
                .unwrap_or_else(|e| panic!("{key} is not RFC 3339 ({s}): {e}"));
            "<timestamp>"
        },
        "access_token" => {
            assert_eq!(s.split('.').count(), 3, "access_token is not a JWT");
            "<jwt>"
        },
        "refresh_token" => {
            assert!(s.contains('.'), "refresh_token has no separator");
            "<refresh_token>"
        },
        "code" if s.len() == 12 && s.chars().all(|c| c.is_ascii_digit()) => "<enrollment_code>",
        k if k == "version" || k.ends_with(":version") => {
            assert_eq!(s, env!("CARGO_PKG_VERSION"), "unexpected version in {key}");
            "<crate_version>"
        },
        _ => return,
    };
    *s = placeholder.into();
}
