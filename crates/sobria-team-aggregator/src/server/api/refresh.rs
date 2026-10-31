//! POST /api/v1/refresh — refresh token rotation.
//!
//! Pattern: verify the old token (selector+verifier), revoke it, issue a
//! new access+refresh pair. The client must replace both tokens
//! after each call.

use axum::{extract::State, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::server::auth::{
    jwt::{self, Role},
    refresh as refresh_issuer,
};
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
    let storage = state
        .storage
        .lock()
        .map_err(|_| ApiError::InternalMsg("storage mutex poisoned".into()))?;

    let row = tokens::verify_refresh_token(storage.connection(), &req.refresh_token, now)?
        .ok_or(ApiError::InvalidToken)?;

    // Rotation: the old one is revoked BEFORE issuing the new one.
    tokens::revoke(storage.connection(), &row.id, now)?;

    let (role, sub, role_label) = match (row.user_id.as_deref(), row.admin_id.as_deref()) {
        (Some(uid), None) => (Role::User, uid.to_string(), "user"),
        (None, Some(aid)) => (Role::Admin, aid.to_string(), "admin"),
        _ => return Err(ApiError::InvalidToken),
    };

    let access_token = jwt::issue(&state.jwt_signing_key, &sub, role, now)?;
    let issued = refresh_issuer::issue(
        &storage,
        if role == Role::User { Some(&sub) } else { None },
        if role == Role::Admin {
            Some(&sub)
        } else {
            None
        },
        now,
    )?;

    Ok(Json(RefreshResponse {
        access_token,
        refresh_token: issued.combined,
        role: role_label,
        access_expires_at: (now + jwt::ACCESS_TTL).to_rfc3339(),
        refresh_expires_at: issued.expires_at.to_rfc3339(),
    }))
}
