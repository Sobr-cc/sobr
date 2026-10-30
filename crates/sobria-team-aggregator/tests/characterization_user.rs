//! Characterization of the user endpoints (SOB-10): `POST /api/v1/estimations`,
//! `GET /api/v1/me/usage`, `GET` and `PUT /api/v1/me/sharing`.
//!
//! Pins the current status code and JSON body, including the normalization of
//! the optional fields (checked in the database) and the refusals.

mod common;

use common::Server;
use serde_json::{json, Value};

/// A valid estimation pushed at `2026-05-16T12:00:<second>Z`; `extra` is
/// merged at the top level of the payload.
fn payload(second: u32, extra: Value) -> Value {
    let mut body = json!({
        "estimate": {
            "method": "afnor_sobria",
            "modelId": "llama-3-1-70b",
            "tokensIn": 120,
            "tokensOut": 800,
            "gco2eq": 0.42,
            "waterMl": 1.5,
            "energyWh": 0.42
        },
        "ts": format!("2026-05-16T12:00:{second:02}Z"),
    });
    if let (Some(target), Some(extra)) = (body.as_object_mut(), extra.as_object()) {
        target.extend(extra.clone());
    }
    body
}

/// What the server stored, in time order.
fn stored_estimations(server: &Server) -> Value {
    let storage = server.storage();
    let mut statement = storage
        .connection()
        .prepare(
            "SELECT ts, method, model_id, tokens_in, tokens_out, region, project
             FROM estimations ORDER BY ts",
        )
        .expect("prepare");
    let rows: Vec<Value> = statement
        .query_map([], |row| {
            Ok(json!({
                "ts": row.get::<_, String>(0)?,
                "method": row.get::<_, String>(1)?,
                "model_id": row.get::<_, String>(2)?,
                "tokens_in": row.get::<_, u32>(3)?,
                "tokens_out": row.get::<_, u32>(4)?,
                "region": row.get::<_, Option<String>>(5)?,
                "project": row.get::<_, Option<String>>(6)?,
            }))
        })
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows");
    Value::Array(rows)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn estimations() {
    let server = Server::start().await;
    let token = server.user_token(&server.seed_user(1, false));
    let admin_token = server.admin_token();
    let post = |body: Value| {
        server
            .post("/api/v1/estimations")
            .bearer_auth(&token)
            .json(&body)
    };

    let (status, _) = server
        .record("estimation: minimal", post(payload(1, json!({}))))
        .await;
    assert_eq!(status, 200);
    server
        .record(
            "estimation: with host, display name, region and project",
            post(payload(
                2,
                json!({
                    "host": "chatgpt",
                    "modelDisplayName": "GPT-4o",
                    "region": "FR",
                    "project": "Alpha",
                }),
            )),
        )
        .await;
    server
        .record(
            "estimation: project is trimmed",
            post(payload(3, json!({"project": "  Alpha  "}))),
        )
        .await;
    server
        .record(
            "estimation: project cut at 64 characters",
            post(payload(4, json!({"project": "x".repeat(70)}))),
        )
        .await;
    server
        .record(
            "estimation: blank project",
            post(payload(5, json!({"project": "   "}))),
        )
        .await;
    for (second, label, pointer, value) in [
        (
            6,
            "method ecologits",
            "/estimate/method",
            json!("ecologits"),
        ),
        (7, "unknown method", "/estimate/method", json!("made_up")),
        (8, "negative gco2eq", "/estimate/gco2eq", json!(-0.1)),
        (9, "negative token count", "/estimate/tokensIn", json!(-1)),
    ] {
        let mut body = payload(second, json!({}));
        *body.pointer_mut(pointer).expect("field") = value;
        server
            .record(&format!("estimation: {label}"), post(body))
            .await;
    }
    server
        .record(
            "estimation: missing ts",
            post({
                let mut body = payload(10, json!({}));
                body.as_object_mut().expect("object").remove("ts");
                body
            }),
        )
        .await;
    server
        .record(
            "estimation: ts is not a date",
            post({
                let mut body = payload(11, json!({}));
                body["ts"] = json!("yesterday");
                body
            }),
        )
        .await;
    server
        .record_refused("estimation", "admin token", &admin_token, || {
            server
                .post("/api/v1/estimations")
                .json(&payload(12, json!({})))
        })
        .await;

    server.note("stored estimations", stored_estimations(&server));
    server
        .record(
            "me/usage after the accepted estimations",
            server.get("/api/v1/me/usage").bearer_auth(&token),
        )
        .await;

    server.assert_golden("user_estimations");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn me_usage_and_sharing() {
    let server = Server::start().await;
    let token = server.user_token(&server.seed_user(1, false));
    let admin_token = server.admin_token();
    let get = |path: &str| server.get(path).bearer_auth(&token);
    let put = |body: Value| {
        server
            .put("/api/v1/me/sharing")
            .bearer_auth(&token)
            .json(&body)
    };

    server
        .record("me/usage: nothing sent yet", get("/api/v1/me/usage"))
        .await;
    server
        .record("me/sharing: default", get("/api/v1/me/sharing"))
        .await;
    server
        .record("me/sharing: opt in", put(json!({"share_identified": true})))
        .await;
    server
        .record("me/sharing: read back", get("/api/v1/me/sharing"))
        .await;
    server
        .record(
            "me/sharing: the policy key is ignored",
            put(json!({"share_identified": false, "policy": "identified"})),
        )
        .await;
    server
        .record(
            "me/sharing: read back after the ignored key",
            get("/api/v1/me/sharing"),
        )
        .await;
    server
        .record("me/sharing: missing field", put(json!({})))
        .await;
    server
        .record(
            "me/sharing: wrong type",
            put(json!({"share_identified": "yes"})),
        )
        .await;
    server
        .record_refused("me/sharing GET", "admin token", &admin_token, || {
            server.get("/api/v1/me/sharing")
        })
        .await;
    server
        .record_refused("me/sharing PUT", "admin token", &admin_token, || {
            server
                .put("/api/v1/me/sharing")
                .json(&json!({"share_identified": true}))
        })
        .await;
    server
        .record("me/usage: no auth", server.get("/api/v1/me/usage"))
        .await;

    server.assert_golden("user_usage_and_sharing");
}
