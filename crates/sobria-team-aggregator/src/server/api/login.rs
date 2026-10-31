//! POST /api/v1/login — admin OR user authentication.
//!
//! For a user: `username = fingerprint` + the `password` set at enrollment.
//! For an admin: admin `username` + password. The role is passed explicitly
//! to avoid confusion (in practice an admin and a user cannot share a
//! username because fingerprints are prefixed `chrome-*` / `firefox-*`,
//! but asking for the role is cleaner).

use axum::{extract::State, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::crypto::password::verify_password;
use crate::server::auth::{issue_session, jwt::Role};
use crate::server::error::{ApiError, ApiResult};
use crate::server::ServerState;
use crate::storage::{admins, users};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub role: Option<Role>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub role: &'static str,
    pub subject_id: String,
    pub access_expires_at: String,
    pub refresh_expires_at: String,
}

pub async fn handle(
    State(state): State<ServerState>,
    Json(req): Json<LoginRequest>,
) -> ApiResult<Json<LoginResponse>> {
    if req.password.is_empty() || req.username.trim().is_empty() {
        return Err(ApiError::BadRequest("username + password requis".into()));
    }

    let storage = state.lock_storage()?;
    let now = Utc::now();
    let role = req.role.unwrap_or(Role::User);

    let subject_id = match role {
        Role::Admin => {
            let admin = admins::find_by_username(storage.connection(), &req.username)?
                .ok_or(ApiError::InvalidCredentials)?;
            if !verify_password(&admin.password_hash, &req.password) {
                return Err(ApiError::InvalidCredentials);
            }
            admins::touch_last_login(storage.connection(), &admin.id, now)?;
            admin.id
        },
        Role::User => {
            // Users authenticate with fingerprint (= username) + password.
            let user = users::find_by_fingerprint(storage.connection(), &req.username)?
                .ok_or(ApiError::InvalidCredentials)?;
            if !verify_password(&user.password_hash, &req.password) {
                return Err(ApiError::InvalidCredentials);
            }
            users::touch_last_seen(storage.connection(), &user.id, now)?;
            user.id
        },
    };
    let tokens = issue_session(&state.jwt_signing_key, &storage, role, &subject_id, now)?;

    Ok(Json(LoginResponse {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        role: role.as_str(),
        subject_id,
        access_expires_at: tokens.access_expires_at,
        refresh_expires_at: tokens.refresh_expires_at,
    }))
}
