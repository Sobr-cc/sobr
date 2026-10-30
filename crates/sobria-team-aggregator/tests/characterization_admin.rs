//! Characterization of the admin endpoints (SOB-10): enrollment codes, exports
//! and alert thresholds. The analytics and the visibility policies are in
//! `characterization_privacy.rs`.
//!
//! Pins the current status code and body, including the validation bounds and
//! the refusals. The exports are also recorded under a policy and a number of
//! employees that the analytics would hide, to show what they expose.

mod common;

use common::{hours_from_now, timestamp, Server, USER_PASSWORD};
use serde_json::{json, Value};
use sobria_team_aggregator::{alerts::store::try_insert_trigger, policy::POLICY_KEY};

/// A window that contains every seeded estimation.
const FROM: &str = "2026-05-15T00:00:00Z";
const TO: &str = "2026-05-17T00:00:00Z";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enrollment_codes() {
    let server = Server::start().await;
    let token = server.admin_token();
    let user_token = server.user_token(&server.seed_user(1, false));
    let create = |body: Value| {
        server
            .post("/api/v1/admin/codes")
            .bearer_auth(&token)
            .json(&body)
    };
    let lifetime = |batch: &Value| {
        hours_from_now(
            batch["codes"][0]["expires_at"]
                .as_str()
                .expect("expires_at"),
        )
    };

    let (status, batch) = server
        .record(
            "codes: three codes, default lifetime",
            create(json!({"count": 3})),
        )
        .await;
    assert_eq!(status, 200);
    server.note("default lifetime in hours", json!(lifetime(&batch)));
    let (_, thirty_days) = server
        .record(
            "codes: one code, 30 days",
            create(json!({"count": 1, "ttl_days": 30})),
        )
        .await;
    server.note("30-day lifetime in hours", json!(lifetime(&thirty_days)));
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
        .record_refused("codes", "user token", &user_token, || {
            server
                .post("/api/v1/admin/codes")
                .json(&json!({"count": 1}))
        })
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
        .record_refused("revoke", "user token", &user_token, || {
            server.delete(&format!("/api/v1/admin/codes/{first_id}"))
        })
        .await;
    server
        .record(
            "enroll with the revoked code",
            server.post("/api/v1/enroll").json(&json!({
                "code": first_code,
                "password": USER_PASSWORD,
                "fingerprint": "fp-bob",
            })),
        )
        .await;

    server.assert_golden("admin_codes");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exports() {
    let server = Server::start().await;
    let token = server.admin_token();
    // Two employees: under the k threshold, and under the strictest policy.
    server.seed_team(1..=2);
    let user_token = server.user_token(&server.seed_user(3, false));
    server.set_config(POLICY_KEY, "anonymous");
    let window = json!({"from": FROM, "to": TO});
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
            post("csv", json!({"from": FROM, "to": TO, "anonymize": true})),
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
                json!({"from": FROM, "to": TO, "anonymize": true, "entity_name": "ACME"}),
            ),
        )
        .await;
    server
        .record_download("csrd: default entity", post("csrd", window.clone()))
        .await;
    server
        .record_download(
            "csrd: entity name",
            post(
                "csrd",
                json!({"from": FROM, "to": TO, "entity_name": "ACME Corp"}),
            ),
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
            .record_refused(kind, "user token", &user_token, || {
                server
                    .post(&format!("/api/v1/admin/exports/{kind}"))
                    .json(&window)
            })
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
    let token = server.admin_token();
    server.seed_team(1..=1);
    let user_token = server.user_token(&server.seed_user(2, false));
    let employee = Server::user_id(1);
    let create = |body: Value| {
        server
            .post("/api/v1/admin/alerts")
            .bearer_auth(&token)
            .json(&body)
    };
    let list = || server.get("/api/v1/admin/alerts").bearer_auth(&token);

    server.record("list: empty", list()).await;
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
        .record_refused("create", "user token", &user_token, || {
            server.post("/api/v1/admin/alerts").json(&json!({
                "scope": "team", "period": "daily", "gco2eq_max": 1.0, "notify_kind": "log_only"
            }))
        })
        .await;

    server.record("list: four thresholds", list()).await;
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
        .record("list: a disabled threshold stays listed", list())
        .await;
    server
        .record_refused("delete", "user token", &user_token, || {
            server.delete(&format!("/api/v1/admin/alerts/{team_id}"))
        })
        .await;
    server
        .record_refused("list", "user token", &user_token, || {
            server.get("/api/v1/admin/alerts")
        })
        .await;

    server.assert_golden("admin_alert_thresholds");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn alert_triggers() {
    let server = Server::start().await;
    let token = server.admin_token();
    let user_token = server.user_token(&server.seed_user(1, false));
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
            let inserted = try_insert_trigger(
                storage.connection(),
                &id,
                &threshold_id,
                timestamp(&format!("2026-05-{day}T00:00:00Z")),
                timestamp(&format!("2026-05-{:02}T00:00:00Z", day + 1)),
                observed,
                timestamp(&format!("2026-05-{day}T12:00:00Z")),
            )
            .expect("seed trigger");
            assert!(inserted.is_some(), "trigger {n} must be new");
        }
    }

    for (label, query) in [
        ("all three, newest first", ""),
        ("limit 2", "?limit=2"),
        ("limit 0 is raised to 1", "?limit=0"),
        ("limit 1000 is capped at 500", "?limit=1000"),
        ("from the second day", "?from=2026-05-15T00:00:00Z"),
        ("up to the second day", "?to=2026-05-15T23:59:59Z"),
        (
            "only the second day",
            "?from=2026-05-15T00:00:00Z&to=2026-05-15T23:59:59Z",
        ),
        ("from is not a date", "?from=yesterday"),
        ("limit is not a number", "?limit=many"),
    ] {
        server
            .record(&format!("triggers: {label}"), get(query))
            .await;
    }
    server
        .record_refused("triggers", "user token", &user_token, || {
            server.get("/api/v1/admin/alerts/triggers")
        })
        .await;

    server.assert_golden("admin_alert_triggers");
}
