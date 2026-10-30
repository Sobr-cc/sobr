//! Characterization of the admin endpoints (SOB-10): enrollment codes, exports
//! and alert thresholds. The analytics and the visibility policies are in
//! `characterization_privacy.rs`.
//!
//! Pins the current status code and body, including the validation bounds and
//! the refusals. The exports are also recorded under a policy and a number of
//! employees that the analytics would hide, to show what they expose.

mod common;

use chrono::{DateTime, Utc};
use common::Server;
use serde_json::{json, Value};
use sobria_team_aggregator::{alerts::store::try_insert_trigger, policy::POLICY_KEY};

/// A window that contains every seeded estimation.
const FROM: &str = "2026-05-15T00:00:00Z";
const TO: &str = "2026-05-17T00:00:00Z";

fn timestamp(rfc3339: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(rfc3339)
        .expect("fixed timestamp")
        .with_timezone(&Utc)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enrollment_codes() {
    let server = Server::start().await;
    let token = server.admin_token().await;
    let enrolled = server.enroll("fp-alice", "Alice").await;
    let user_token = enrolled["access_token"].as_str().expect("token").to_owned();
    let create = |body: Value| {
        server
            .post("/api/v1/admin/codes")
            .bearer_auth(&token)
            .json(&body)
    };

    let (status, batch) = server
        .record(
            "codes: three codes, default lifetime",
            create(json!({"count": 3})),
        )
        .await;
    assert_eq!(status, 200);
    let expires_at = DateTime::parse_from_rfc3339(
        batch["codes"][0]["expires_at"]
            .as_str()
            .expect("expires_at"),
    )
    .expect("RFC 3339");
    let lifetime_days =
        ((expires_at.with_timezone(&Utc) - Utc::now()).num_minutes() as f64 / 1440.0).round();
    server.note("default lifetime in days", json!(lifetime_days));
    server
        .record(
            "codes: one code, 30 days",
            create(json!({"count": 1, "ttl_days": 30})),
        )
        .await;
    for (label, body) in [
        ("count 0", json!({"count": 0})),
        ("count 501", json!({"count": 501})),
        ("lifetime 0", json!({"count": 1, "ttl_days": 0})),
        ("lifetime 366", json!({"count": 1, "ttl_days": 366})),
        ("lifetime -1", json!({"count": 1, "ttl_days": -1})),
        ("count missing", json!({"ttl_days": 7})),
        ("count negative", json!({"count": -1})),
        ("count is a string", json!({"count": "3"})),
    ] {
        server
            .record(&format!("codes: {label}"), create(body))
            .await;
    }
    server
        .record(
            "codes: body is not JSON",
            server
                .post("/api/v1/admin/codes")
                .bearer_auth(&token)
                .header("content-type", "application/json")
                .body("{"),
        )
        .await;
    server
        .record(
            "codes: no auth",
            server
                .post("/api/v1/admin/codes")
                .json(&json!({"count": 1})),
        )
        .await;
    server
        .record(
            "codes: user token",
            server
                .post("/api/v1/admin/codes")
                .bearer_auth(&user_token)
                .json(&json!({"count": 1})),
        )
        .await;

    let first_id = batch["codes"][0]["id"].as_str().expect("id").to_owned();
    let first_code = batch["codes"][0]["code"].as_str().expect("code").to_owned();
    let revoke = |id: &str| {
        server
            .delete(&format!("/api/v1/admin/codes/{id}"))
            .bearer_auth(&token)
    };
    server.record("revoke: ok", revoke(&first_id)).await;
    server
        .record("revoke: the same code again", revoke(&first_id))
        .await;
    server
        .record("revoke: unknown id", revoke("does-not-exist"))
        .await;
    server
        .record(
            "revoke: no auth",
            server.delete(&format!("/api/v1/admin/codes/{first_id}")),
        )
        .await;
    server
        .record(
            "revoke: user token",
            server
                .delete(&format!("/api/v1/admin/codes/{first_id}"))
                .bearer_auth(&user_token),
        )
        .await;
    server
        .record(
            "enroll with the revoked code",
            server.post("/api/v1/enroll").json(&json!({
                "code": first_code,
                "password": "user-password-strong",
                "fingerprint": "fp-bob",
            })),
        )
        .await;

    server.assert_golden("admin_codes");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exports() {
    let server = Server::start().await;
    let token = server.admin_token().await;
    let enrolled = server.enroll("fp-alice", "Alice").await;
    let user_token = enrolled["access_token"].as_str().expect("token").to_owned();
    // Two employees: under the k threshold, and under the strictest policy.
    server.seed_team(1..=2);
    server
        .storage()
        .set_config(POLICY_KEY, "anonymous")
        .expect("config");
    let window = json!({"from": FROM, "to": TO});
    let with = |extra: Value| {
        let mut body = window.clone();
        body.as_object_mut()
            .expect("object")
            .extend(extra.as_object().expect("object").clone());
        body
    };
    let post = |kind: &str, body: Value| {
        server
            .post(&format!("/api/v1/admin/exports/{kind}"))
            .bearer_auth(&token)
            .json(&body)
    };

    server
        .record_download(
            "csv: 2 employees, policy anonymous",
            post("csv", window.clone()),
        )
        .await;
    server
        .record_download(
            "csv: anonymize = true",
            post("csv", with(json!({"anonymize": true}))),
        )
        .await;
    server
        .record_download(
            "csv: window without data",
            post(
                "csv",
                json!({"from": "2026-01-01T00:00:00Z", "to": "2026-01-02T00:00:00Z"}),
            ),
        )
        .await;
    server
        .record_download(
            "prov-o: 2 employees, policy anonymous",
            post("prov-o", window.clone()),
        )
        .await;
    server
        .record_download(
            "prov-o: anonymize = true, entity name",
            post(
                "prov-o",
                with(json!({"anonymize": true, "entity_name": "ACME"})),
            ),
        )
        .await;
    server
        .record_download("csrd: default entity", post("csrd", window.clone()))
        .await;
    server
        .record_download(
            "csrd: entity name",
            post("csrd", with(json!({"entity_name": "ACME Corp"}))),
        )
        .await;

    for kind in ["csv", "prov-o", "csrd"] {
        server
            .record(
                &format!("{kind}: from after to"),
                post(kind, json!({"from": TO, "to": FROM})),
            )
            .await;
        server
            .record(
                &format!("{kind}: from is not a date"),
                post(kind, json!({"from": "yesterday"})),
            )
            .await;
        server
            .record(
                &format!("{kind}: no auth"),
                server
                    .post(&format!("/api/v1/admin/exports/{kind}"))
                    .json(&window),
            )
            .await;
        server
            .record(
                &format!("{kind}: user token"),
                server
                    .post(&format!("/api/v1/admin/exports/{kind}"))
                    .bearer_auth(&user_token)
                    .json(&window),
            )
            .await;
    }
    server
        .record(
            "csv: body is not JSON",
            server
                .post("/api/v1/admin/exports/csv")
                .bearer_auth(&token)
                .header("content-type", "application/json")
                .body("{"),
        )
        .await;

    server.assert_golden("admin_exports");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn alert_thresholds() {
    let server = Server::start().await;
    let token = server.admin_token().await;
    let enrolled = server.enroll("fp-alice", "Alice").await;
    let user_token = enrolled["access_token"].as_str().expect("token").to_owned();
    server.seed_team(1..=1);
    let employee = Server::user_id(1);
    let create = |body: Value| {
        server
            .post("/api/v1/admin/alerts")
            .bearer_auth(&token)
            .json(&body)
    };

    server
        .record(
            "list: empty",
            server.get("/api/v1/admin/alerts").bearer_auth(&token),
        )
        .await;
    let (status, team) = server
        .record(
            "create: team, daily, log only",
            create(json!({
                "scope": "team", "period": "daily", "gco2eq_max": 10.0, "notify_kind": "log_only"
            })),
        )
        .await;
    assert_eq!(status, 200);
    let team_id = team["id"].as_str().expect("id").to_owned();
    server
        .record(
            "create: user, weekly, webhook",
            create(json!({
                "scope": "user", "target_id": employee, "period": "weekly", "gco2eq_max": 2.5,
                "notify_kind": "webhook", "notify_target": "https://example.invalid/hook"
            })),
        )
        .await;
    // The target is not checked against the users table.
    server
        .record(
            "create: user scope on an id that is not an employee",
            create(json!({
                "scope": "user", "target_id": "not-an-employee", "period": "monthly",
                "gco2eq_max": 1.0, "notify_kind": "log_only"
            })),
        )
        .await;
    server
        .record(
            "create: team, monthly, email",
            create(json!({
                "scope": "team", "period": "monthly", "gco2eq_max": 100.0,
                "notify_kind": "email", "notify_target": "ops@example.invalid"
            })),
        )
        .await;

    for (label, body) in [
        (
            "gco2eq_max is 0",
            json!({"scope": "team", "period": "daily", "gco2eq_max": 0.0, "notify_kind": "log_only"}),
        ),
        (
            "gco2eq_max is negative",
            json!({"scope": "team", "period": "daily", "gco2eq_max": -1.0, "notify_kind": "log_only"}),
        ),
        (
            "user scope without target",
            json!({"scope": "user", "period": "daily", "gco2eq_max": 1.0, "notify_kind": "log_only"}),
        ),
        (
            "team scope with a target",
            json!({"scope": "team", "target_id": employee, "period": "daily", "gco2eq_max": 1.0, "notify_kind": "log_only"}),
        ),
        (
            "webhook without target",
            json!({"scope": "team", "period": "daily", "gco2eq_max": 1.0, "notify_kind": "webhook"}),
        ),
        (
            "email without target",
            json!({"scope": "team", "period": "daily", "gco2eq_max": 1.0, "notify_kind": "email"}),
        ),
        (
            "unknown period",
            json!({"scope": "team", "period": "hourly", "gco2eq_max": 1.0, "notify_kind": "log_only"}),
        ),
        (
            "unknown scope",
            json!({"scope": "org", "period": "daily", "gco2eq_max": 1.0, "notify_kind": "log_only"}),
        ),
        (
            "unknown notify kind",
            json!({"scope": "team", "period": "daily", "gco2eq_max": 1.0, "notify_kind": "sms"}),
        ),
        ("missing fields", json!({"scope": "team"})),
    ] {
        server
            .record(&format!("create: {label}"), create(body))
            .await;
    }
    server
        .record(
            "create: no auth",
            server.post("/api/v1/admin/alerts").json(&json!({
                "scope": "team", "period": "daily", "gco2eq_max": 1.0, "notify_kind": "log_only"
            })),
        )
        .await;
    server
        .record(
            "create: user token",
            server
                .post("/api/v1/admin/alerts")
                .bearer_auth(&user_token)
                .json(&json!({
                    "scope": "team", "period": "daily", "gco2eq_max": 1.0, "notify_kind": "log_only"
                })),
        )
        .await;

    server
        .record(
            "list: four thresholds",
            server.get("/api/v1/admin/alerts").bearer_auth(&token),
        )
        .await;
    let delete = |id: &str| {
        server
            .delete(&format!("/api/v1/admin/alerts/{id}"))
            .bearer_auth(&token)
    };
    server.record("delete: ok", delete(&team_id)).await;
    server
        .record("delete: the same threshold again", delete(&team_id))
        .await;
    server
        .record("delete: unknown id", delete("does-not-exist"))
        .await;
    server
        .record(
            "list: a disabled threshold stays listed",
            server.get("/api/v1/admin/alerts").bearer_auth(&token),
        )
        .await;
    server
        .record(
            "delete: no auth",
            server.delete(&format!("/api/v1/admin/alerts/{team_id}")),
        )
        .await;
    server
        .record(
            "delete: user token",
            server
                .delete(&format!("/api/v1/admin/alerts/{team_id}"))
                .bearer_auth(&user_token),
        )
        .await;
    server
        .record("list: no auth", server.get("/api/v1/admin/alerts"))
        .await;
    server
        .record(
            "list: user token",
            server.get("/api/v1/admin/alerts").bearer_auth(&user_token),
        )
        .await;

    server.assert_golden("admin_alert_thresholds");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn alert_triggers() {
    let server = Server::start().await;
    let token = server.admin_token().await;
    let enrolled = server.enroll("fp-alice", "Alice").await;
    let user_token = enrolled["access_token"].as_str().expect("token").to_owned();
    let (_, threshold) = server
        .fetch(
            server
                .post("/api/v1/admin/alerts")
                .bearer_auth(&token)
                .json(&json!({
                    "scope": "team", "period": "daily", "gco2eq_max": 1.0, "notify_kind": "log_only"
                })),
        )
        .await;
    let threshold_id = threshold["id"].as_str().expect("id").to_owned();
    let get = |query: &str| {
        server
            .get(&format!("/api/v1/admin/alerts/triggers{query}"))
            .bearer_auth(&token)
    };

    server.record("triggers: none yet", get("")).await;

    // Three triggers, one per day, observed 3.0, 4.0 and 5.0 gCO2eq.
    {
        let storage = server.storage();
        for (n, day, observed) in [(1, 14, 3.0), (2, 15, 4.0), (3, 16, 5.0)] {
            let id = format!("01HZ{}{n:03}", "T".repeat(19));
            let start = timestamp(&format!("2026-05-{day}T00:00:00Z"));
            let end = timestamp(&format!("2026-05-{:02}T00:00:00Z", day + 1));
            let triggered = timestamp(&format!("2026-05-{day}T12:00:00Z"));
            let inserted = try_insert_trigger(
                storage.connection(),
                &id,
                &threshold_id,
                start,
                end,
                observed,
                triggered,
            )
            .expect("seed trigger");
            assert!(inserted.is_some(), "trigger {n} must be new");
        }
    }

    server
        .record("triggers: all three, newest first", get(""))
        .await;
    server.record("triggers: limit 2", get("?limit=2")).await;
    server
        .record("triggers: limit 0 is raised to 1", get("?limit=0"))
        .await;
    server
        .record("triggers: limit 1000 is capped at 500", get("?limit=1000"))
        .await;
    server
        .record(
            "triggers: from the second day",
            get("?from=2026-05-15T00:00:00Z"),
        )
        .await;
    server
        .record(
            "triggers: up to the second day",
            get("?to=2026-05-15T23:59:59Z"),
        )
        .await;
    server
        .record(
            "triggers: only the second day",
            get("?from=2026-05-15T00:00:00Z&to=2026-05-15T23:59:59Z"),
        )
        .await;
    server
        .record("triggers: from is not a date", get("?from=yesterday"))
        .await;
    server
        .record("triggers: limit is not a number", get("?limit=many"))
        .await;
    server
        .record(
            "triggers: no auth",
            server.get("/api/v1/admin/alerts/triggers"),
        )
        .await;
    server
        .record(
            "triggers: user token",
            server
                .get("/api/v1/admin/alerts/triggers")
                .bearer_auth(&user_token),
        )
        .await;

    server.assert_golden("admin_alert_triggers");
}
