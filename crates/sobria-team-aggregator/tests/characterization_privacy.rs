//! Characterization of the privacy rules on aggregates (SOB-10): k-anonymity
//! (ADR-0015) and the visibility policies (ADR-0016), over HTTP.
//!
//! Employees are seeded straight in the database (see `Server::seed_team`),
//! so the numbers are fixed. The configuration is changed live through the
//! storage, as the CLI does. If an expected value moves, the privacy
//! behavior of the server moved.

mod common;

use common::Server;
use serde_json::{json, Value};
use sobria_team_aggregator::policy::POLICY_KEY;

/// A window that contains every seeded estimation.
const WINDOW: &str = "from=2026-05-15T00:00:00Z&to=2026-05-17T00:00:00Z";

async fn analytics(server: &Server, token: &str, label: &str, extra_query: &str) -> Value {
    let path = format!("/api/v1/admin/analytics?{WINDOW}{extra_query}");
    let (status, body) = server
        .record(label, server.get(&path).bearer_auth(token))
        .await;
    assert_eq!(status, 200, "{label}: {body}");
    body
}

/// Asserts the k-anonymity block of a response.
fn assert_k(body: &Value, required: u64, active_users: u64, blocked: bool) {
    assert_eq!(
        body["k_anonymity"],
        json!({"required": required, "active_users": active_users, "blocked": blocked}),
        "k_anonymity block: {body}"
    );
}

/// Asserts that a blocked response shows no aggregate at all.
fn assert_hidden(body: &Value) {
    assert_eq!(body["series"], json!([]), "series must be hidden");
    assert_eq!(body["top_models"], json!([]), "top_models must be hidden");
    assert_eq!(body["projects"], json!([]), "projects must be hidden");
    assert_eq!(
        body["method_breakdown"],
        json!([]),
        "method_breakdown must be hidden"
    );
    assert_eq!(
        body["top_users"],
        json!({
            "identified": [],
            "anonymous_users": 0,
            "anonymous_count": 0,
            "anonymous_gco2eq_g": 0.0
        }),
        "top_users must be hidden"
    );
}

/// Asserts that an allowed response shows aggregates.
fn assert_shown(body: &Value) {
    assert!(
        !body["series"].as_array().expect("series").is_empty(),
        "series must be shown: {body}"
    );
    assert!(
        !body["top_models"]
            .as_array()
            .expect("top_models")
            .is_empty(),
        "top_models must be shown: {body}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn k_anonymity_threshold_and_boundaries() {
    let server = Server::start().await;
    let token = server.admin_token().await;

    server.seed_team(1..=2);
    let body = analytics(&server, &token, "2 employees, default k", "").await;
    assert_k(&body, 5, 2, true);
    assert_hidden(&body);

    server
        .storage()
        .set_config("k_anonymity_min", "3")
        .expect("config");
    let body = analytics(&server, &token, "2 employees, k = 3", "").await;
    assert_k(&body, 3, 2, true);
    assert_hidden(&body);

    // A configured value under the floor of 3 is raised to 3.
    server
        .storage()
        .set_config("k_anonymity_min", "1")
        .expect("config");
    let body = analytics(&server, &token, "2 employees, k = 1 (floor 3)", "").await;
    assert_k(&body, 3, 2, true);
    assert_hidden(&body);

    server.seed_team(3..=3);
    let body = analytics(&server, &token, "3 employees, k = 1 (floor 3)", "").await;
    assert_k(&body, 3, 3, false);
    assert_shown(&body);

    // An unreadable value falls back to the default of 5.
    server
        .storage()
        .set_config("k_anonymity_min", "abc")
        .expect("config");
    let body = analytics(&server, &token, "3 employees, k unreadable", "").await;
    assert_k(&body, 5, 3, true);
    assert_hidden(&body);

    server.seed_team(4..=4);
    let body = analytics(&server, &token, "4 employees, k unreadable (k - 1)", "").await;
    assert_k(&body, 5, 4, true);
    assert_hidden(&body);

    server.seed_team(5..=5);
    let body = analytics(&server, &token, "5 employees, k unreadable (k)", "").await;
    assert_k(&body, 5, 5, false);
    assert_shown(&body);

    server
        .storage()
        .set_config("k_anonymity_min", "10")
        .expect("config");
    let body = analytics(&server, &token, "5 employees, k = 10", "").await;
    assert_k(&body, 10, 5, true);
    assert_hidden(&body);

    server.assert_golden("privacy_k_anonymity");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn k_anonymity_under_each_policy() {
    let server = Server::start().await;
    let token = server.admin_token().await;
    server.seed_team(1..=2);

    for (policy, blocked) in [("anonymous", true), ("opt_in", true), ("identified", false)] {
        server
            .storage()
            .set_config(POLICY_KEY, policy)
            .expect("config");
        let body = analytics(
            &server,
            &token,
            &format!("2 employees, policy {policy}"),
            "",
        )
        .await;
        assert_eq!(body["policy"], policy);
        assert_k(&body, 5, 2, blocked);
        if blocked {
            assert_hidden(&body);
        } else {
            // `identified` switches the k check off.
            assert_shown(&body);
        }
    }

    // A policy value the server does not know falls back to opt_in.
    server
        .storage()
        .set_config(POLICY_KEY, "something-else")
        .expect("config");
    let body = analytics(&server, &token, "2 employees, unknown policy value", "").await;
    assert_eq!(body["policy"], "opt_in");

    server.assert_golden("privacy_k_under_policies");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn analytics_views_and_errors() {
    let server = Server::start().await;
    let token = server.admin_token().await;
    let enrolled = server.enroll("fp-alice", "Alice").await;
    let user_token = enrolled["access_token"].as_str().expect("token").to_owned();
    server.seed_team(1..=5);

    analytics(&server, &token, "group by day (default)", "").await;
    analytics(&server, &token, "group by day (explicit)", "&group_by=day").await;
    analytics(&server, &token, "group by week", "&group_by=week").await;
    analytics(&server, &token, "group by month", "&group_by=month").await;
    analytics(&server, &token, "top 1", "&top=1").await;
    analytics(&server, &token, "top 0 is raised to 1", "&top=0").await;
    analytics(&server, &token, "top 1000 is capped at 100", "&top=1000").await;
    analytics(&server, &token, "dim is accepted and ignored", "&dim=user").await;

    for (label, query) in [
        ("unknown group_by", format!("?{WINDOW}&group_by=hourly")),
        (
            "from after to",
            "?from=2026-05-17T00:00:00Z&to=2026-05-15T00:00:00Z".to_owned(),
        ),
        (
            "from equal to",
            "?from=2026-05-15T00:00:00Z&to=2026-05-15T00:00:00Z".to_owned(),
        ),
        ("from is not a date", "?from=yesterday".to_owned()),
        ("top is not a number", format!("?{WINDOW}&top=many")),
    ] {
        server
            .record(
                &format!("error: {label}"),
                server
                    .get(&format!("/api/v1/admin/analytics{query}"))
                    .bearer_auth(&token),
            )
            .await;
    }

    // Without `from` and `to` the window is the last 30 days: the seeded
    // estimations (May 2026) are outside it, so only the shape is pinned.
    let (status, body) = server
        .fetch(server.get("/api/v1/admin/analytics").bearer_auth(&token))
        .await;
    let parse = |key: &str| {
        chrono::DateTime::parse_from_rfc3339(body[key].as_str().expect(key)).expect("RFC 3339")
    };
    server.note(
        "default window",
        json!({
            "status": status,
            "window_days": (parse("to") - parse("from")).num_days(),
            "group_by": body["group_by"],
            "policy": body["policy"],
            "active_users": body["k_anonymity"]["active_users"],
        }),
    );

    server
        .record("no auth", server.get("/api/v1/admin/analytics"))
        .await;
    server
        .record(
            "user token",
            server
                .get("/api/v1/admin/analytics")
                .bearer_auth(&user_token),
        )
        .await;

    server.assert_golden("privacy_analytics_views");
}

/// An alert on one employee is not filtered by the policy nor by the consent
/// of that employee: the trigger row carries the individual consumption.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_alert_ignores_policy_and_consent() {
    let server = Server::start().await;
    let token = server.admin_token().await;
    let enrolled = server.enroll("fp-alice", "Alice").await;
    let user_token = enrolled["access_token"].as_str().expect("token").to_owned();
    let user_id = enrolled["user_id"].as_str().expect("user_id").to_owned();
    server
        .storage()
        .set_config(POLICY_KEY, "anonymous")
        .expect("config");

    server
        .record(
            "alice did not opt in",
            server.get("/api/v1/me/sharing").bearer_auth(&user_token),
        )
        .await;
    server
        .record(
            "create: user alert on a non-consenting employee, policy anonymous",
            server
                .post("/api/v1/admin/alerts")
                .bearer_auth(&token)
                .json(&json!({
                    "scope": "user", "target_id": user_id, "period": "daily",
                    "gco2eq_max": 0.1, "notify_kind": "log_only"
                })),
        )
        .await;
    server
        .record(
            "estimation above the threshold",
            server
                .post("/api/v1/estimations")
                .bearer_auth(&user_token)
                .json(&json!({
                    "estimate": {
                        "method": "afnor_sobria", "modelId": "llama-3-1-70b", "tokensIn": 120,
                        "tokensOut": 800, "gco2eq": 0.42, "waterMl": 1.5, "energyWh": 0.42
                    },
                    "ts": chrono::Utc::now().to_rfc3339(),
                })),
        )
        .await;

    // The threshold is checked after the acknowledgement: wait for the row.
    let mut triggers = Value::Null;
    for _ in 0..50 {
        let (_, body) = server
            .fetch(
                server
                    .get("/api/v1/admin/alerts/triggers")
                    .bearer_auth(&token),
            )
            .await;
        if body["triggers"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty())
        {
            triggers = body;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let rows = triggers["triggers"].as_array().expect("a trigger row");
    let parse = |key: &str| {
        chrono::DateTime::parse_from_rfc3339(rows[0][key].as_str().expect(key)).expect("RFC 3339")
    };
    server.note(
        "trigger seen by the admin",
        json!({
            "rows": rows.len(),
            "observed_gco2eq": rows[0]["observed_gco2eq"],
            "period_hours": (parse("period_end") - parse("period_start")).num_hours(),
            "notify_error": rows[0]["notify_error"],
        }),
    );

    server.assert_golden("privacy_user_alert");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn policies_on_analytics_users_and_user_detail() {
    let server = Server::start().await;
    let token = server.admin_token().await;
    let enrolled = server.enroll("fp-alice", "Alice").await;
    let user_token = enrolled["access_token"].as_str().expect("token").to_owned();
    // Employee 1 opted in; employees 2..=5 did not; Alice has no estimation.
    server.seed_team(1..=5);
    let opted_in = Server::user_id(1);
    let private = Server::user_id(2);

    for policy in ["anonymous", "opt_in", "identified"] {
        server
            .storage()
            .set_config(POLICY_KEY, policy)
            .expect("config");
        let body = analytics(&server, &token, &format!("[{policy}] analytics"), "").await;
        assert_k(&body, 5, 5, false);

        server
            .record(
                &format!("[{policy}] users list"),
                server.get("/api/v1/admin/users").bearer_auth(&token),
            )
            .await;
        for (who, id) in [("opted in", &opted_in), ("not opted in", &private)] {
            server
                .record(
                    &format!("[{policy}] user detail, {who}"),
                    server
                        .get(&format!("/api/v1/admin/users/{id}/analytics?{WINDOW}"))
                        .bearer_auth(&token),
                )
                .await;
        }
    }

    // A window of the first day only: the series follows the window, the totals
    // are checked against it in the recording.
    server
        .storage()
        .set_config(POLICY_KEY, "opt_in")
        .expect("config");
    server
        .record(
            "user detail: window of the first day only",
            server
                .get(&format!(
                    "/api/v1/admin/users/{opted_in}/analytics?from=2026-05-15T00:00:00Z&to=2026-05-16T00:00:00Z"
                ))
                .bearer_auth(&token),
        )
        .await;
    server
        .record(
            "user detail: unknown user",
            server
                .get(&format!(
                    "/api/v1/admin/users/{}/analytics?{WINDOW}",
                    Server::user_id(99)
                ))
                .bearer_auth(&token),
        )
        .await;
    server
        .record(
            "user detail: from is not a date",
            server
                .get(&format!(
                    "/api/v1/admin/users/{opted_in}/analytics?from=yesterday"
                ))
                .bearer_auth(&token),
        )
        .await;
    server
        .record(
            "user detail: from after to",
            server
                .get(&format!(
                    "/api/v1/admin/users/{opted_in}/analytics?from=2026-05-17T00:00:00Z&to=2026-05-15T00:00:00Z"
                ))
                .bearer_auth(&token),
        )
        .await;
    server
        .record(
            "user detail: no auth",
            server.get(&format!(
                "/api/v1/admin/users/{opted_in}/analytics?{WINDOW}"
            )),
        )
        .await;
    server
        .record(
            "user detail: user token",
            server
                .get(&format!(
                    "/api/v1/admin/users/{opted_in}/analytics?{WINDOW}"
                ))
                .bearer_auth(&user_token),
        )
        .await;

    server.assert_golden("privacy_policies_views");
}
