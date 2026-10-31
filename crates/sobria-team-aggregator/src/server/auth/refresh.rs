//! Issues refresh tokens (format `<token_id>.<secret>`).
//!
//! See `storage::tokens` for the selector + verifier pattern.
//! TTL 7 days (see brief C28.2).

use chrono::{DateTime, Duration, Utc};
use ulid::Ulid;
use uuid::Uuid;

use crate::crypto::password;
use crate::error::AggregatorResult;
use crate::storage::{tokens, Storage};

/// Refresh token TTL.
pub const REFRESH_TTL: Duration = Duration::days(7);

/// Issued token (to send back to the client). `combined` is the opaque value
/// the client keeps.
#[derive(Debug, Clone)]
pub struct IssuedRefreshToken {
    pub combined: String,
    pub expires_at: DateTime<Utc>,
}

/// Issues a refresh token for a user OR an admin (XOR guaranteed by the caller).
pub fn issue(
    storage: &Storage,
    user_id: Option<&str>,
    admin_id: Option<&str>,
    now: DateTime<Utc>,
) -> AggregatorResult<IssuedRefreshToken> {
    debug_assert!(user_id.is_some() ^ admin_id.is_some(), "user XOR admin");
    let token_id = Ulid::new().to_string();
    let secret = Uuid::new_v4().simple().to_string();
    let hash = password::hash_password(&secret)?;
    let expires_at = now + REFRESH_TTL;
    tokens::insert(
        storage.connection(),
        &token_id,
        user_id,
        admin_id,
        &hash,
        now,
        expires_at,
    )?;
    Ok(IssuedRefreshToken {
        combined: format!("{token_id}.{secret}"),
        expires_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{users, Storage};

    #[test]
    fn issue_round_trips_through_verify() {
        let s = Storage::open_in_memory().unwrap();
        users::insert(s.connection(), "u-1", None, "fp", "h", None, Utc::now()).unwrap();

        let now = Utc::now();
        let issued = issue(&s, Some("u-1"), None, now).unwrap();
        assert!(issued.combined.contains('.'));

        let row = tokens::verify_refresh_token(s.connection(), &issued.combined, now)
            .unwrap()
            .expect("doit vérifier");
        assert_eq!(row.user_id.as_deref(), Some("u-1"));
        assert!(row.admin_id.is_none());
    }
}
