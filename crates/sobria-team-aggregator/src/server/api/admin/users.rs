//! GET /api/v1/admin/users — list of enrolled employees + totals.

use axum::{extract::State, Json};
use serde::Serialize;

use crate::policy::VisibilityPolicy;
use crate::server::auth::middleware::AuthenticatedAdmin;
use crate::server::error::ApiResult;
use crate::server::ServerState;
use crate::storage::users::{self, UserWithTotals};

#[derive(Debug, Serialize)]
pub struct UsersResponse {
    pub users: Vec<UserWithTotals>,
}

pub async fn handle(
    State(state): State<ServerState>,
    _admin: AuthenticatedAdmin,
) -> ApiResult<Json<UsersResponse>> {
    let storage = state.lock_storage()?;
    let conn = storage.connection();
    // ADR-0016: `identified` → totals for everyone; `anonymous` → no
    // totals (even opt-in); `opt_in` → ADR-0015 behavior (already
    // applied by the storage).
    let users = match crate::policy::load(&storage) {
        VisibilityPolicy::Identified => users::list_all_with_totals_unmasked(conn)?,
        VisibilityPolicy::Anonymous => {
            let mut users = users::list_all_with_totals(conn)?;
            for u in &mut users {
                u.totals = None;
            }
            users
        },
        VisibilityPolicy::OptIn => users::list_all_with_totals(conn)?,
    };
    Ok(Json(UsersResponse { users }))
}
