//! GET /api/v1/admin/users — list of enrolled employees + totals.

use axum::{extract::State, Json};
use serde::Serialize;

use crate::server::auth::middleware::AuthenticatedAdmin;
use crate::server::error::{ApiError, ApiResult};
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
    let storage = state
        .storage
        .lock()
        .map_err(|_| ApiError::InternalMsg("storage mutex poisoned".into()))?;
    let pol = crate::policy::load(&storage);
    let mut users = users::list_all_with_totals(storage.connection())?;
    // ADR-0016: `identified` → totals for everyone; `anonymous` → no
    // totals (even opt-in); `opt_in` → ADR-0015 behavior (already
    // applied by the storage).
    match pol {
        crate::policy::VisibilityPolicy::Identified => {
            users = users::list_all_with_totals_unmasked(storage.connection())?;
        },
        crate::policy::VisibilityPolicy::Anonymous => {
            for u in &mut users {
                u.totals = None;
            }
        },
        crate::policy::VisibilityPolicy::OptIn => {},
    }
    Ok(Json(UsersResponse { users }))
}
