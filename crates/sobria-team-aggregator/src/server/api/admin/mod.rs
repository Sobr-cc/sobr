//! Admin routes `/api/v1/admin/*`. All of them require `AuthenticatedAdmin`.

pub mod alerts;
pub mod analytics;
pub mod codes;
pub mod exports;
pub mod user_detail;
pub mod users;

use axum::{
    routing::{delete, get, post},
    Router,
};
use chrono::{DateTime, Duration, Utc};

use crate::server::error::{ApiError, ApiResult};
use crate::server::ServerState;

/// Parses an optional RFC 3339 query date; absent or empty gives `default`.
fn parse_date_or(raw: Option<&str>, default: DateTime<Utc>) -> ApiResult<DateTime<Utc>> {
    match raw {
        None | Some("") => Ok(default),
        Some(s) => DateTime::parse_from_rfc3339(s)
            .map(|d| d.with_timezone(&Utc))
            .map_err(|e| ApiError::BadRequest(format!("date RFC3339 invalide ({s}): {e}"))),
    }
}

/// Parses the `from`/`to` window; defaults to the 30 days up to `now`.
fn parse_window(
    from: Option<&str>,
    to: Option<&str>,
    now: DateTime<Utc>,
) -> ApiResult<(DateTime<Utc>, DateTime<Utc>)> {
    let from = parse_date_or(from, now - Duration::days(30))?;
    let to = parse_date_or(to, now)?;
    Ok((from, to))
}

/// Sub-router mounted under `/admin` by [`crate::server::api::router`].
pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/users", get(users::handle))
        .route("/codes", post(codes::handle_create))
        .route("/codes/:id", delete(codes::handle_revoke))
        .route("/analytics", get(analytics::handle))
        .route("/users/:id/analytics", get(user_detail::handle))
        .route("/exports/csrd", post(exports::handle_csrd))
        .route("/exports/prov-o", post(exports::handle_provo))
        .route("/exports/csv", post(exports::handle_csv))
        // threshold alerts
        .route("/alerts", get(alerts::handle_list).post(alerts::handle_create))
        .route("/alerts/:id", delete(alerts::handle_delete))
        .route("/alerts/triggers", get(alerts::handle_triggers))
}
