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
    /// Clé HS256 (hex, lue de `config.jwt_signing_key` à `serve` startup).
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

/// Construit l'application axum (routes + middleware tracing).
///
/// Ordre de matching axum :
/// 1. `/health` (routes statiques)
/// 2. `/api/v1/*` (API REST)
/// 3. `/` → index.html
/// 4. catch-all → static asset OU SPA fallback vers index.html
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

/// CORS pour le service worker de l'extension (MV3) : son `fetch` part
/// d'une origine `chrome-extension://…` / `moz-extension://…` hors
/// `host_permissions`, donc sans ces en-têtes Chrome bloque la réponse et
/// l'envoi best-effort échoue en silence (extension/src/lib/team-client.ts).
/// L'API est protégée par jeton Bearer (jamais de cookies) : on ne reflète
/// que les schémas d'extensions navigateur, aucun site web n'est autorisé.
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
