//! POST /api/v1/refresh — refresh token rotation.
//!
//! Pattern: verify the old token (selector+verifier), revoke it, issue a
//! new access+refresh pair. The client must replace both tokens
//! after each call.

use axum::{extract::State, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::server::auth::{issue_session, jwt::Role};
use crate::server::error::{ApiError, ApiResult};
use crate::server::ServerState;
use crate::storage::tokens;

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub role: &'static str,
    pub access_expires_at: String,
    pub refresh_expires_at: String,
}

pub async fn handle(
    State(state): State<ServerState>,
    Json(req): Json<RefreshRequest>,
) -> ApiResult<Json<RefreshResponse>> {
    let now = Utc::now();
    let storage = state.lock_storage()?;

    let row = tokens::verify_refresh_token(storage.connection(), &req.refresh_token, now)?
        .ok_or(ApiError::InvalidToken)?;

    // Rotation: the old one is revoked BEFORE issuing the new one.
    tokens::revoke(storage.connection(), &row.id, now)?;

    let (role, sub) = match (row.user_id.as_deref(), row.admin_id.as_deref()) {
        (Some(uid), None) => (Role::User, uid),
        (None, Some(aid)) => (Role::Admin, aid),
        _ => return Err(ApiError::InvalidToken),
    };

    let tokens = issue_session(&state.jwt_signing_key, &storage, role, sub, now)?;

    Ok(Json(RefreshResponse {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        role: role.as_str(),
        access_expires_at: tokens.access_expires_at,
        refresh_expires_at: tokens.refresh_expires_at,
    }))
}
