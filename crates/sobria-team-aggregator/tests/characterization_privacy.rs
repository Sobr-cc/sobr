//! Characterization of the privacy rules on aggregates (SOB-10): k-anonymity
//! (ADR-0015) and the visibility policies (ADR-0016), over HTTP.
//!
//! Employees are seeded straight in the database (see `Server::seed_team`),
//! so the numbers are fixed. The configuration is changed live through the
//! storage, without the checks of the CLI. If an expected value moves, the
//! privacy behavior of the server moved.

mod common;

use common::Server;
use serde_json::{json, Value};
use sobria_team_aggregator::policy::POLICY_KEY;

/// A window that contains every seeded estimation.
const WINDOW: &str = "from=2026-05-15T00:00:00Z&to=2026-05-17T00:00:00Z";

/// A window without any estimation.
const EMPTY_WINDOW: &str = "from=2026-01-01T00:00:00Z&to=2026-01-02T00:00:00Z";

/// Records `GET /admin/analytics?<query>` and checks that it answers 200.
async fn analytics_query(server: &Server, token: &str, label: &str, query: &str) -> Value {
    let path = format!("/api/v1/admin/analytics?{query}");
    let (status, body) = server
        .record(label, server.get(&path).bearer_auth(token))
        .await;
    assert_eq!(status, 200, "{label}: {body}");
    body
}

/// Analytics over the window that contains every seeded estimation.
async fn analytics(server: &Server, token: &str, label: &str, extra_query: &str) -> Value {
    analytics_query(server, token, label, &format!("{WINDOW}{extra_query}")).await
}

/// Asserts the k-anonymity block of a response, then that the aggregates are
/// hidden when `blocked` and shown otherwise.
fn assert_k(body: &Value, required: u64, active_users: u64, blocked: bool) {
    assert_eq!(
        body["k_anonymity"],
        json!({"required": required, "active_users": active_users, "blocked": blocked}),
        "k_anonymity block: {body}"
    );
    if blocked {
        assert_hidden(body);
    } else {
        assert_shown(body);
    }
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
    let token = server.admin_token();

    server.seed_team(1..=2);
    let body = analytics(&server, &token, "2 employees, default k", "").await;
    assert_k(&body, 5, 2, true);

    server.set_config("k_anonymity_min", "3");
    let body = analytics(&server, &token, "2 employees, k = 3", "").await;
    assert_k(&body, 3, 2, true);

    // A configured value under the floor of 3 is raised to 3.
    server.set_config("k_anonymity_min", "1");
    let body = analytics(&server, &token, "2 employees, k = 1 (floor 3)", "").await;
    assert_k(&body, 3, 2, true);

    server.seed_team(3..=3);
    let body = analytics(&server, &token, "3 employees, k = 1 (floor 3)", "").await;
    assert_k(&body, 3, 3, false);

    // An unreadable value falls back to the default of 5.
    server.set_config("k_anonymity_min", "abc");
    let body = analytics(&server, &token, "3 employees, k unreadable", "").await;
    assert_k(&body, 5, 3, true);

    server.seed_team(4..=4);
    let body = analytics(&server, &token, "4 employees, k unreadable (k - 1)", "").await;
    assert_k(&body, 5, 4, true);

    server.seed_team(5..=5);
    let body = analytics(&server, &token, "5 employees, k unreadable (k)", "").await;
    assert_k(&body, 5, 5, false);

    // Only the employees active in the queried window count.
    let body = analytics_query(
        &server,
        &token,
        "5 employees, window without estimation",
        EMPTY_WINDOW,
    )
    .await;
    assert_k(&body, 5, 0, true);

    server.set_config("k_anonymity_min", "10");
    let body = analytics(&server, &token, "5 employees, k = 10", "").await;
    assert_k(&body, 10, 5, true);

    server.assert_golden("privacy_k_anonymity");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn k_anonymity_under_each_policy() {
    let server = Server::start().await;
    let token = server.admin_token();
    server.seed_team(1..=2);

    // `identified` switches the k check off; the other policies keep it.
    for (policy, blocked) in [("anonymous", true), ("opt_in", true), ("identified", false)] {
        server.set_config(POLICY_KEY, policy);
        let label = format!("2 employees, policy {policy}");
        let body = analytics(&server, &token, &label, "").await;
        assert_eq!(body["policy"], policy);
        assert_k(&body, 5, 2, blocked);
    }

    // A policy value the server does not know falls back to opt_in.
    server.set_config(POLICY_KEY, "something-else");
    let body = analytics(&server, &token, "2 employees, unknown policy value", "").await;
    assert_eq!(body["policy"], "opt_in");

    server.assert_golden("privacy_k_under_policies");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn analytics_views_and_errors() {
    let server = Server::start().await;
    let token = server.admin_token();
    server.seed_team(1..=5);
    let user_token = server.user_token(&server.seed_user(6, false));

    analytics(&server, &token, "group by day (default)", "").await;
    analytics(&server, &token, "group by day (explicit)", "&group_by=day").await;
    analytics(&server, &token, "group by week", "&group_by=week").await;
    analytics(&server, &token, "group by month", "&group_by=month").await;
    analytics(&server, &token, "top 1", "&top=1").await;
    analytics(&server, &token, "top 0 is raised to 1", "&top=0").await;
    analytics(&server, &token, "top 1000 is accepted", "&top=1000").await;
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

    // Without `from` and `to` the window is the last 30 days. What it contains
    // depends on today's date, so only its length and the echoed settings are pinned.
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
        }),
    );

    server
        .record_refused("analytics", "user token", &user_token, || {
            server.get("/api/v1/admin/analytics")
        })
        .await;

    server.assert_golden("privacy_analytics_views");
}

/// An alert on one employee is not filtered by the policy nor by the consent
/// of that employee: the trigger row carries the individual consumption.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_alert_ignores_policy_and_consent() {
    let server = Server::start().await;
    let token = server.admin_token();
    let user_id = server.seed_user(1, false);
    let user_token = server.user_token(&user_id);
    server.set_config(POLICY_KEY, "anonymous");

    server
        .record(
            "the employee did not opt in",
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
    // The period of a daily alert is computed from the server clock when the
    // estimation arrives.
    common::avoid_utc_midnight().await;
    server
        .record(
            "estimation above the threshold",
            server
                .post("/api/v1/estimations")
                .bearer_auth(&user_token)
                .json(&json!({
                    "estimate": {
                        "method": "afnor_sobria", "modelId": "llama-3-1-70b", "tokensIn": 120,
                        "tokensOut": 800, "gco2eq": 0.42, "waterMl": 1.5, "energyWh": 0.33
                    },
                    "ts": chrono::Utc::now().to_rfc3339(),
                })),
        )
        .await;

    // The threshold is checked, and the row written, before the acknowledgement.
    // The notification is sent by a task that runs afterwards and then sets
    // `notified_at` and `notify_error`: wait for it before reading them.
    let mut rows = Vec::new();
    for _ in 0..50 {
        let (_, body) = server
            .fetch(
                server
                    .get("/api/v1/admin/alerts/triggers")
                    .bearer_auth(&token),
            )
            .await;
        rows = body["triggers"].as_array().expect("triggers").clone();
        if rows.first().is_some_and(|row| !row["notified_at"].is_null()) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert_eq!(rows.len(), 1, "one trigger row");
    assert!(
        !rows[0]["notified_at"].is_null(),
        "the notification was never marked as sent"
    );
    let parse = |key: &str| {
        chrono::DateTime::parse_from_rfc3339(rows[0][key].as_str().expect(key)).expect("RFC 3339")
    };
    server.note(
        "trigger seen by the admin, once notified",
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
    let token = server.admin_token();
    // Employee 1 opted in; employees 2 to 5 did not; employee 6 has no estimation.
    server.seed_team(1..=5);
    let user_token = server.user_token(&server.seed_user(6, false));
    let opted_in = Server::user_id(1);
    let private = Server::user_id(2);
    let detail = |id: &str, query: &str| {
        server
            .get(&format!("/api/v1/admin/users/{id}/analytics{query}"))
            .bearer_auth(&token)
    };

    // (policy, identified rows in top_users, users with totals in the list,
    //  status of the detail of an opted-in employee, of a private employee)
    for (policy, identified_rows, users_with_totals, detail_opted_in, detail_private) in [
        ("anonymous", 0, 0, 403, 403),
        ("opt_in", 1, 1, 200, 403),
        ("identified", 5, 6, 200, 200),
    ] {
        server.set_config(POLICY_KEY, policy);
        let body = analytics(&server, &token, &format!("[{policy}] analytics"), "").await;
        assert_k(&body, 5, 5, false);
        assert_eq!(
            body["top_users"]["identified"]
                .as_array()
                .expect("rows")
                .len(),
            identified_rows,
            "[{policy}] employees named in top_users"
        );

        let (_, users) = server
            .record(
                &format!("[{policy}] users list"),
                server.get("/api/v1/admin/users").bearer_auth(&token),
            )
            .await;
        let with_totals = users["users"]
            .as_array()
            .expect("users")
            .iter()
            .filter(|user| !user["totals"].is_null())
            .count();
        assert_eq!(
            with_totals, users_with_totals,
            "[{policy}] users with totals"
        );

        for (who, id, expected) in [
            ("opted in", &opted_in, detail_opted_in),
            ("not opted in", &private, detail_private),
        ] {
            let (status, _) = server
                .record(
                    &format!("[{policy}] user detail, {who}"),
                    detail(id, &format!("?{WINDOW}")),
                )
                .await;
            assert_eq!(status, expected, "[{policy}] user detail, {who}");
        }
    }

    // A window of the first day only: the series follows the window, the totals
    // are checked against it in the recording.
    server.set_config(POLICY_KEY, "opt_in");
    server
        .record(
            "user detail: window of the first day only",
            detail(
                &opted_in,
                "?from=2026-05-15T00:00:00Z&to=2026-05-16T00:00:00Z",
            ),
        )
        .await;
    server
        .record(
            "user detail: unknown user",
            detail(&Server::user_id(99), &format!("?{WINDOW}")),
        )
        .await;
    server
        .record(
            "user detail: from is not a date",
            detail(&opted_in, "?from=yesterday"),
        )
        .await;
    server
        .record(
            "user detail: from after to",
            detail(
                &opted_in,
                "?from=2026-05-17T00:00:00Z&to=2026-05-15T00:00:00Z",
            ),
        )
        .await;
    server
        .record_refused("user detail", "user token", &user_token, || {
            server.get(&format!(
                "/api/v1/admin/users/{opted_in}/analytics?{WINDOW}"
            ))
        })
        .await;

    server.assert_golden("privacy_policies_views");
}
