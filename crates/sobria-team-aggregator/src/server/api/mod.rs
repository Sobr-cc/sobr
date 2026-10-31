//! `/api/v1/*` routes gathered in a sub-router.

pub mod admin;
pub mod enroll;
pub mod estimations;
pub mod login;
pub mod me;
pub mod refresh;

use axum::{
    routing::{get, post},
    Router,
};

use crate::server::ServerState;

/// Sub-router mounted under `/api/v1` by [`crate::server::build_router`].
pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/enroll", post(enroll::handle))
        .route("/login", post(login::handle))
        .route("/refresh", post(refresh::handle))
        .route("/estimations", post(estimations::handle))
        .route("/me/usage", get(me::handle))
        .route("/me/sharing", get(me::sharing_get).put(me::sharing_put))
        .nest("/admin", admin::router())
}
