//! Auth: HS256 JWT access tokens + selector/verifier refresh tokens +
//! axum extractors.

pub mod jwt;
pub mod middleware;
pub mod refresh;

use chrono::{DateTime, Utc};

use crate::server::error::ApiResult;
use crate::storage::Storage;
use jwt::Role;

/// Access + refresh tokens issued together, expiries in RFC 3339.
#[derive(Debug, Clone)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: String,
    pub refresh_expires_at: String,
}

/// Issues the access JWT then the refresh token of `sub` acting as `role`.
pub fn issue_session(
    signing_key_hex: &str,
    storage: &Storage,
    role: Role,
    sub: &str,
    now: DateTime<Utc>,
) -> ApiResult<TokenPair> {
    let access_token = jwt::issue(signing_key_hex, sub, role, now)?;
    let (user_id, admin_id) = match role {
        Role::User => (Some(sub), None),
        Role::Admin => (None, Some(sub)),
    };
    let issued = refresh::issue(storage, user_id, admin_id, now)?;
    Ok(TokenPair {
        access_token,
        refresh_token: issued.combined,
        access_expires_at: (now + jwt::ACCESS_TTL).to_rfc3339(),
        refresh_expires_at: issued.expires_at.to_rfc3339(),
    })
}
