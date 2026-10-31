//! HTTP layer of the `sobria-team-aggregator`: the axum router with
//! `/health`, the `/api/v1` user and admin routes, and the embedded dashboard.

pub mod api;
pub mod auth;
pub mod embedded_web;
pub mod error;
pub mod routes;
pub mod tls;

use std::sync::{Arc, Mutex};

use axum::http::{header, Method};
use axum::Router;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::storage::Storage;

/// State shared by the handlers. `Storage` sits behind a `Mutex` because
/// `rusqlite::Connection` is not `Sync`, so every request takes this one lock.
#[derive(Clone)]
pub struct ServerState {
    pub storage: Arc<Mutex<Storage>>,
    /// HS256 key (hex, read from `config.jwt_signing_key` at `serve` startup).
    pub jwt_signing_key: Arc<String>,
}

impl ServerState {
    pub fn new(storage: Storage, jwt_signing_key: String) -> Self {
        Self {
            storage: Arc::new(Mutex::new(storage)),
            jwt_signing_key: Arc::new(jwt_signing_key),
        }
    }
}

/// Builds the axum application (routes + tracing middleware).
///
/// axum matching order:
/// 1. `/health` (static routes)
/// 2. `/api/v1/*` (REST API)
/// 3. `/` → index.html
/// 4. catch-all → static asset OR SPA fallback to index.html
pub fn build_router(state: ServerState) -> Router {
    use axum::routing::get;
    Router::new()
        .merge(routes::router())
        .nest("/api/v1", api::router())
        .route("/", get(embedded_web::index))
        .fallback(get(embedded_web::handler))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(extension_cors())
}

/// CORS for the extension's (MV3) service worker: its `fetch` comes from a
/// `chrome-extension://…` / `moz-extension://…` origin outside
/// `host_permissions`, so without these headers Chrome blocks the response and
/// the best-effort send fails silently (extension/src/lib/team-client.ts).
/// The API is protected by a Bearer token (never cookies): we only reflect
/// browser extension schemes, no website is allowed.
fn extension_cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            [
                b"chrome-extension://".as_slice(),
                b"moz-extension://".as_slice(),
                b"safari-web-extension://".as_slice(),
            ]
            .iter()
            .any(|scheme| origin.as_bytes().starts_with(scheme))
        }))
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
}
