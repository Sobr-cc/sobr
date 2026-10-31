//! GET /api/v1/admin/users/:id/analytics — usage detail of ONE
//! employee, governed by the visibility policy (ADR-0016):
//!
//! - `identified` : available for every employee.
//! - `opt_in`     : available ONLY if the employee turned on identified
//!   sharing (ADR-0015 §3) — otherwise an explicit 403.
//! - `anonymous`  : always 403 (no identification, even voluntary).

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use super::parse_date_or;
use crate::policy::{self, VisibilityPolicy};
use crate::server::auth::middleware::AuthenticatedAdmin;
use crate::server::error::{ApiError, ApiResult};
use crate::server::ServerState;
use crate::storage::analytics::{self, GroupBy, MethodBreakdown, ModelTop, TimeBucket};
use crate::storage::estimations::{self, UsageTotals};
use crate::storage::users;

#[derive(Debug, Deserialize)]
pub struct DetailQuery {
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserHeader {
    pub id: String,
    pub display_name: Option<String>,
    pub fingerprint: String,
    pub share_identified: bool,
}

#[derive(Debug, Serialize)]
pub struct UserDetailResponse {
    pub policy: &'static str,
    pub from: String,
    pub to: String,
    pub user: UserHeader,
    pub totals: UsageTotals,
    pub series: Vec<TimeBucket>,
    pub top_models: Vec<ModelTop>,
    pub method_breakdown: Vec<MethodBreakdown>,
}

pub async fn handle(
    State(state): State<ServerState>,
    _admin: AuthenticatedAdmin,
    Path(user_id): Path<String>,
    Query(q): Query<DetailQuery>,
) -> ApiResult<Json<UserDetailResponse>> {
    let now = Utc::now();
    let from = parse_date_or(q.from.as_deref(), now - Duration::days(30))?;
    let to = parse_date_or(q.to.as_deref(), now)?;

    let storage = state.lock_storage()?;
    let conn = storage.connection();

    let pol = policy::load(&storage);
    let row = users::find_by_id(conn, &user_id)?
        .ok_or_else(|| ApiError::BadRequest(format!("employé inconnu: {user_id}")))?;
    let share = users::share_identified(conn, &user_id)?;

    match pol {
        VisibilityPolicy::Identified => {},
        VisibilityPolicy::OptIn if share => {},
        // 403: the web-team UI words it by the current policy
        // (opt-in not turned on / strict anonymous) — `ApiError::Forbidden`
        // is a unit variant, the context lives on the client.
        VisibilityPolicy::OptIn | VisibilityPolicy::Anonymous => {
            return Err(ApiError::Forbidden);
        },
    }

    Ok(Json(UserDetailResponse {
        policy: pol.as_str(),
        from: from.to_rfc3339(),
        to: to.to_rfc3339(),
        user: UserHeader {
            id: row.id,
            display_name: row.display_name,
            fingerprint: row.fingerprint,
            share_identified: share,
        },
        totals: estimations::totals_for_user(conn, &user_id)?,
        series: analytics::time_buckets(conn, Some(&user_id), from, to, GroupBy::Day)?,
        top_models: analytics::top_models(conn, Some(&user_id), from, to, 10)?,
        method_breakdown: analytics::method_breakdown(conn, Some(&user_id), from, to)?,
    }))
}
