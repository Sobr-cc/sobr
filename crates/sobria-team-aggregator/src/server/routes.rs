//! Public route `/health`. The `/api/v1` routes live in [`super::api`].

use axum::{routing::get, Json, Router};
use serde::Serialize;

use crate::server::ServerState;

/// `/health` response.
#[derive(Debug, Serialize)]
pub struct Health {
    pub ok: bool,
    pub version: &'static str,
}

/// Sub-router for the public routes (health). Merged in
/// [`super::build_router`].
pub fn router() -> Router<ServerState> {
    Router::new().route("/health", get(health))
}

async fn health() -> Json<Health> {
    Json(Health {
        ok: true,
        version: env!("CARGO_PKG_VERSION"),
    })
}
