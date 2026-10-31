//! POST /api/v1/admin/codes — creates N enrollment codes.
//! DELETE /api/v1/admin/codes/:id — revokes a code by id.
//!
//! Codes are shown in clear ONLY in the POST response.
//! Argon2id PHC in the database, nothing reversible afterwards.

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::crypto::secret::enrollment_code;
use crate::server::auth::middleware::AuthenticatedAdmin;
use crate::server::error::{ApiError, ApiResult};
use crate::server::ServerState;
use crate::storage::enrollment_codes;

/// Reasonable limits to prevent abuse.
const MAX_CODES_PER_BATCH: u32 = 500;
const MAX_TTL_DAYS: i64 = 365;

#[derive(Debug, Deserialize)]
pub struct CreateCodesRequest {
    pub count: u32,
    #[serde(default = "default_ttl_days")]
    pub ttl_days: i64,
}

fn default_ttl_days() -> i64 {
    7
}

#[derive(Debug, Serialize)]
pub struct CreatedCode {
    pub id: String,
    pub code: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize)]
pub struct CreateCodesResponse {
    pub codes: Vec<CreatedCode>,
}

pub async fn handle_create(
    State(state): State<ServerState>,
    admin: AuthenticatedAdmin,
    Json(req): Json<CreateCodesRequest>,
) -> ApiResult<Json<CreateCodesResponse>> {
    if req.count == 0 || req.count > MAX_CODES_PER_BATCH {
        return Err(ApiError::BadRequest(format!(
            "count doit être ∈ [1, {MAX_CODES_PER_BATCH}]"
        )));
    }
    if req.ttl_days <= 0 || req.ttl_days > MAX_TTL_DAYS {
        return Err(ApiError::BadRequest(format!(
            "ttl_days doit être ∈ [1, {MAX_TTL_DAYS}]"
        )));
    }

    let storage = state.lock_storage()?;
    let now = Utc::now();
    let expires_at = now + Duration::days(req.ttl_days);

    let mut out = Vec::with_capacity(req.count as usize);
    for _ in 0..req.count {
        let id = Ulid::new().to_string();
        let code = enrollment_code();
        enrollment_codes::insert(
            storage.connection(),
            &id,
            &code,
            &admin.claims.sub,
            now,
            expires_at,
        )?;
        out.push(CreatedCode {
            id,
            code,
            expires_at: expires_at.to_rfc3339(),
        });
    }
    Ok(Json(CreateCodesResponse { codes: out }))
}

#[derive(Debug, Serialize)]
pub struct RevokeResponse {
    pub revoked: bool,
}

pub async fn handle_revoke(
    State(state): State<ServerState>,
    _admin: AuthenticatedAdmin,
    Path(id): Path<String>,
) -> ApiResult<Json<RevokeResponse>> {
    let storage = state.lock_storage()?;
    let revoked = enrollment_codes::revoke(storage.connection(), &id, Utc::now())?;
    Ok(Json(RevokeResponse { revoked }))
}
