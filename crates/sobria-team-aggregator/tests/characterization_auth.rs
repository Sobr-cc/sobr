//! Characterization of the public and authentication endpoints (SOB-10).
//!
//! Pins the current status code and JSON body of `/health`, `/api/v1/enroll`,
//! `/api/v1/login`, `/api/v1/refresh`, and of the access guards on protected
//! routes. Nothing here changes behavior: if an expected value moves, the
//! behavior of the server moved.

mod common;

use common::{hours_from_now, Server, ADMIN_PASSWORD, ADMIN_USERNAME, USER_PASSWORD};
use reqwest::header::AUTHORIZATION;
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn health_and_unknown_routes() {
    let server = Server::start().await;

    server.record("GET /health", server.get("/health")).await;
    server.record("POST /health", server.post("/health")).await;
    // An unknown API path falls through to the embedded UI (single-page app).
    server
        .record_status(
            "GET /api/v1/does-not-exist",
            server.get("/api/v1/does-not-exist"),
        )
        .await;

    server.assert_golden("auth_health");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enroll() {
    let server = Server::start().await;
    let codes = server.create_codes(2);
    let enroll = |code: &str, password: &str, fingerprint: &str| {
        server.post("/api/v1/enroll").json(&json!({
            "code": code,
            "password": password,
            "fingerprint": fingerprint,
            "display_name": "Alice",
        }))
    };

    let (status, body) = server
        .record("enroll: ok", enroll(&codes[0], USER_PASSWORD, "fp-alice"))
        .await;
    assert_eq!(status, 200);
    server.note(
        "token lifetimes in hours",
        json!({
            "access": hours_from_now(body["access_expires_at"].as_str().expect("access")),
            "refresh": hours_from_now(body["refresh_expires_at"].as_str().expect("refresh")),
        }),
    );
    // The second code is not consumed by the refused attempts below.
    server
        .record(
            "enroll: same fingerprint, fresh code",
            enroll(&codes[1], USER_PASSWORD, "fp-alice"),
        )
        .await;
    server
        .record(
            "enroll: code already used",
            enroll(&codes[0], USER_PASSWORD, "fp-bob"),
        )
        .await;
    server
        .record(
            "enroll: unknown code",
            enroll("000000000000", USER_PASSWORD, "fp-bob"),
        )
        .await;
    server
        .record(
            "enroll: code with a wrong format",
            enroll("123", USER_PASSWORD, "fp-bob"),
        )
        .await;
    server
        .record(
            "enroll: password too short",
            enroll(&codes[1], "short", "fp-bob"),
        )
        .await;
    server
        .record(
            "enroll: empty fingerprint",
            enroll(&codes[1], USER_PASSWORD, ""),
        )
        .await;
    server
        .record(
            "enroll: missing field",
            server
                .post("/api/v1/enroll")
                .json(&json!({"code": codes[1], "fingerprint": "fp-bob"})),
        )
        .await;
    server
        .record(
            "enroll: body is not JSON",
            server
                .post("/api/v1/enroll")
                .header("content-type", "application/json")
                .body("{"),
        )
        .await;
    let (status, _) = server
        .record(
            "enroll: without display name",
            server.post("/api/v1/enroll").json(&json!({
                "code": codes[1],
                "password": USER_PASSWORD,
                "fingerprint": "fp-carol",
            })),
        )
        .await;
    assert_eq!(status, 200);

    server.assert_golden("auth_enroll");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn login() {
    let server = Server::start().await;
    server.enroll("fp-alice", "Alice").await;

    for (label, body) in [
        (
            "user, role omitted",
            json!({"username": "fp-alice", "password": USER_PASSWORD}),
        ),
        (
            "user, role user",
            json!({"username": "fp-alice", "password": USER_PASSWORD, "role": "user"}),
        ),
        (
            "user, wrong password",
            json!({"username": "fp-alice", "password": "wrong-password"}),
        ),
        (
            "user, unknown fingerprint",
            json!({"username": "fp-nobody", "password": USER_PASSWORD}),
        ),
        (
            "empty username",
            json!({"username": "", "password": USER_PASSWORD}),
        ),
        (
            "empty password",
            json!({"username": "fp-alice", "password": ""}),
        ),
        (
            "admin",
            json!({"username": ADMIN_USERNAME, "password": ADMIN_PASSWORD, "role": "admin"}),
        ),
        (
            "admin, wrong password",
            json!({"username": ADMIN_USERNAME, "password": "wrong-password", "role": "admin"}),
        ),
        (
            "admin name with the user role",
            json!({"username": ADMIN_USERNAME, "password": ADMIN_PASSWORD, "role": "user"}),
        ),
        (
            "unknown role",
            json!({"username": "fp-alice", "password": USER_PASSWORD, "role": "root"}),
        ),
        ("missing password", json!({"username": "fp-alice"})),
    ] {
        server
            .record(
                &format!("login: {label}"),
                server.post("/api/v1/login").json(&body),
            )
            .await;
    }

    server.assert_golden("auth_login");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refresh() {
    let server = Server::start().await;
    let enrolled = server.enroll("fp-alice", "Alice").await;
    let first = enrolled["refresh_token"]
        .as_str()
        .expect("refresh")
        .to_owned();
    let refresh = |token: &str| {
        server
            .post("/api/v1/refresh")
            .json(&json!({"refresh_token": token}))
    };

    let (status, body) = server.record("refresh: user", refresh(&first)).await;
    assert_eq!(status, 200);
    let second = body["refresh_token"].as_str().expect("rotated").to_owned();
    server
        .record("refresh: user, old token replayed", refresh(&first))
        .await;
    server
        .record("refresh: user, rotated token", refresh(&second))
        .await;
    server
        .record("refresh: garbage token", refresh("not-a-token"))
        .await;
    server.record("refresh: empty token", refresh("")).await;
    server
        .record(
            "refresh: missing field",
            server.post("/api/v1/refresh").json(&json!({})),
        )
        .await;

    let (_, admin) = server
        .record(
            "login: admin (to get a refresh token)",
            server.post("/api/v1/login").json(
                &json!({"username": ADMIN_USERNAME, "password": ADMIN_PASSWORD, "role": "admin"}),
            ),
        )
        .await;
    let admin_refresh = admin["refresh_token"]
        .as_str()
        .expect("admin refresh")
        .to_owned();
    server
        .record("refresh: admin", refresh(&admin_refresh))
        .await;
    server
        .record(
            "refresh: admin, old token replayed",
            refresh(&admin_refresh),
        )
        .await;

    server.assert_golden("auth_refresh");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn access_guards() {
    let server = Server::start().await;
    let user_id = server.seed_user(1, false);
    let user_token = server.user_token(&user_id);
    let forged_token = server.forged_user_token(&user_id);
    let expired_token = server.expired_user_token(&user_id);
    let admin_token = server.admin_token();
    let attempts = [
        ("no header", None),
        ("Basic scheme", Some("Basic YWJjOmRlZg==".to_owned())),
        ("garbage bearer", Some("Bearer garbage".to_owned())),
        (
            "token signed with another key",
            Some(format!("Bearer {forged_token}")),
        ),
        ("expired token", Some(format!("Bearer {expired_token}"))),
        ("user token", Some(format!("Bearer {user_token}"))),
        ("admin token", Some(format!("Bearer {admin_token}"))),
    ];

    for (route, label) in [
        ("/api/v1/me/usage", "user route"),
        ("/api/v1/admin/users", "admin route"),
    ] {
        for (attempt, header) in &attempts {
            let mut request = server.get(route);
            if let Some(value) = header {
                request = request.header(AUTHORIZATION, value);
            }
            server
                .record(&format!("GET {route}: {attempt} ({label})"), request)
                .await;
        }
    }

    server.assert_golden("auth_access_guards");
}
