//! GET /api/v1/admin/analytics?from&to&group_by=day|week|month
//!
//! Returns 4 aggregated sections over the `[from, to]` window:
//! - `series`           : time bucketing by `group_by` (default `day`).
//! - `top_models`       : top 10 models by descending gCO₂eq.
//! - `top_users`        : opt-in sharing participants + anonymous aggregate
//!   (ADR-0015 §3 — never again a named ranking by default).
//! - `method_breakdown` : afnor_sobria vs ecologits.
//!
//! **k-anonymity (ADR-0015 §2)**: if the number of active users in
//! the window is below `k = max(3, config.k_anonymity_min)` (default
//! 5), the sections are empty and `k_anonymity.blocked = true` — an aggregate
//! of a very small team amounts to individual data.
//!
//! The `dim` parameter (`user|model|method`) is accepted and ignored: every
//! section is returned.

use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use super::parse_date_or;
use crate::commands::config::K_ANONYMITY_MIN;
use crate::policy::{self, VisibilityPolicy};
use crate::server::auth::middleware::AuthenticatedAdmin;
use crate::server::error::{ApiError, ApiResult};
use crate::server::ServerState;
use crate::storage::analytics::{
    self, GroupBy, MethodBreakdown, ModelTop, ProjectBreakdown, TimeBucket, TopUsersShared,
};

/// Default k threshold if `config.k_anonymity_min` is missing/unreadable.
const K_DEFAULT: u32 = K_ANONYMITY_MIN.default as u32;
/// Hard floor: no configuration can go below this k.
const K_FLOOR: u32 = K_ANONYMITY_MIN.floor as u32;

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    /// RFC3339; default: `now - 30 days`.
    #[serde(default)]
    pub from: Option<String>,
    /// RFC3339; default: `now`.
    #[serde(default)]
    pub to: Option<String>,
    /// `day` (default) | `week` | `month`.
    #[serde(default)]
    pub group_by: Option<String>,
    /// `user | model | method`, accepted and ignored.
    #[allow(dead_code)]
    #[serde(default)]
    pub dim: Option<String>,
    /// Top N (default: 10, max 100).
    #[serde(default)]
    pub top: Option<u32>,
}

/// k-anonymity check status attached to every response (the UI explains it).
#[derive(Debug, Serialize)]
pub struct KAnonymity {
    /// Applied threshold (`max(K_FLOOR, config)`).
    pub required: u32,
    /// Distinct active users in the window.
    pub active_users: u64,
    /// `true` → empty sections, aggregates refused.
    pub blocked: bool,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsResponse {
    pub from: String,
    pub to: String,
    pub group_by: String,
    /// Effective visibility policy (ADR-0016) — shown by the UI.
    pub policy: &'static str,
    pub k_anonymity: KAnonymity,
    pub series: Vec<TimeBucket>,
    pub top_models: Vec<ModelTop>,
    pub top_users: TopUsersShared,
    /// Per-project aggregates — folded into "other projects" below k
    /// in anonymous/opt_in modes.
    pub projects: Vec<ProjectBreakdown>,
    pub method_breakdown: Vec<MethodBreakdown>,
}

pub async fn handle(
    State(state): State<ServerState>,
    _admin: AuthenticatedAdmin,
    Query(q): Query<AnalyticsQuery>,
) -> ApiResult<Json<AnalyticsResponse>> {
    let now = Utc::now();
    let from = parse_date_or(q.from.as_deref(), now - Duration::days(30))?;
    let to = parse_date_or(q.to.as_deref(), now)?;
    if from >= to {
        return Err(ApiError::BadRequest("from doit être < to".into()));
    }
    let group_by = match q.group_by.as_deref().unwrap_or("day") {
        "day" => GroupBy::Day,
        "week" => GroupBy::Week,
        "month" => GroupBy::Month,
        other => {
            return Err(ApiError::BadRequest(format!(
                "group_by inconnu: {other} (day|week|month)"
            )))
        },
    };
    let top_n = q.top.unwrap_or(10).clamp(1, 100);

    let storage = state
        .storage
        .lock()
        .map_err(|_| ApiError::InternalMsg("storage mutex poisoned".into()))?;
    let conn = storage.connection();

    // ── Visibility policy (ADR-0016) then k guard (ADR-0015 §2). ──
    let pol = policy::load(&storage);
    let k_required = storage
        .get_config(K_ANONYMITY_MIN.key)
        .ok()
        .flatten()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(K_DEFAULT)
        .max(K_FLOOR);
    let active_users = analytics::active_user_count(conn, from, to)?;
    // In `identified` mode, k-anonymity does not apply (individual data
    // is visible by organization policy).
    let blocked = pol != VisibilityPolicy::Identified && active_users < u64::from(k_required);
    let project_k = match pol {
        VisibilityPolicy::Identified => None,
        _ => Some(k_required),
    };

    let (series, top_models, top_users, projects, method_breakdown) = if blocked {
        (
            Vec::new(),
            Vec::new(),
            TopUsersShared::default(),
            Vec::new(),
            Vec::new(),
        )
    } else {
        let top_users = match pol {
            // Strict anonymous: no identification, even voluntary —
            // everyone is aggregated in the anonymous block.
            VisibilityPolicy::Anonymous => {
                let mut t = analytics::top_users_shared(conn, from, to, top_n)?;
                if !t.identified.is_empty() {
                    for u in t.identified.drain(..) {
                        t.anonymous_users += 1;
                        t.anonymous_count += u.count;
                        t.anonymous_gco2eq_g += u.gco2eq_g;
                    }
                }
                t
            },
            VisibilityPolicy::OptIn => analytics::top_users_shared(conn, from, to, top_n)?,
            VisibilityPolicy::Identified => TopUsersShared {
                identified: analytics::top_users_all(conn, from, to, top_n)?,
                anonymous_users: 0,
                anonymous_count: 0,
                anonymous_gco2eq_g: 0.0,
            },
        };
        (
            analytics::time_buckets(conn, None, from, to, group_by)?,
            analytics::top_models(conn, None, from, to, top_n)?,
            top_users,
            analytics::project_breakdown(conn, from, to, project_k)?,
            analytics::method_breakdown(conn, None, from, to)?,
        )
    };

    Ok(Json(AnalyticsResponse {
        from: from.to_rfc3339(),
        to: to.to_rfc3339(),
        group_by: q.group_by.unwrap_or_else(|| "day".to_string()),
        policy: pol.as_str(),
        k_anonymity: KAnonymity {
            required: k_required,
            active_users,
            blocked,
        },
        series,
        top_models,
        top_users,
        projects,
        method_breakdown,
    }))
}
