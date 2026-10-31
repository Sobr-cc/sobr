//! Serves the embedded Svelte dashboard (`web-team/build/`) via `rust-embed`.
//!
//! Routing strategy:
//!
//! 1. The exact requested route matches an embedded file → return it
//!    with its guessed MIME type.
//! 2. Otherwise, if the path looks like an asset (contains a `.`), 404.
//! 3. Otherwise, SPA fallback: return `index.html` (the SvelteKit client
//!    router takes over).
//!
//! If the bundle was not built (`web-team/build/index.html` missing),
//! the handler returns an explanatory message instead of a silent 404.

use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../web-team/build"]
// Bundle missing (fresh clone, Rust CI) → empty embed and `bundle_missing_response`.
#[allow_missing = true]
#[include = "*.html"]
#[include = "*.js"]
#[include = "*.css"]
#[include = "*.json"]
#[include = "*.svg"]
#[include = "*.png"]
#[include = "*.ico"]
#[include = "*.webp"]
#[include = "*.woff2"]
struct WebAssets;

/// Whether this binary embeds the web-team bundle (its `index.html`).
pub fn bundle_embedded() -> bool {
    WebAssets::get("index.html").is_some()
}

/// Handler for the root `/`.
pub async fn index() -> Response {
    serve_or_fallback("index.html")
}

/// Generic handler for any `/*rest` path.
pub async fn handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    if path.is_empty() {
        return serve_or_fallback("index.html");
    }
    serve_or_fallback(path)
}

fn serve_or_fallback(path: &str) -> Response {
    match WebAssets::get(path) {
        Some(file) => respond_with_file(path, file.data.into_owned()),
        None => {
            if path.contains('.') {
                // Path looks like an asset → do not mask it as a 200.
                StatusCode::NOT_FOUND.into_response()
            } else {
                // SPA fallback.
                match WebAssets::get("index.html") {
                    Some(idx) => respond_with_file("index.html", idx.data.into_owned()),
                    None => bundle_missing_response(),
                }
            }
        },
    }
}

fn respond_with_file(path: &str, bytes: Vec<u8>) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let cache = if path.contains("_app/immutable/") {
        // SvelteKit emits content-hashed files under _app/immutable/,
        // so cache for 1 year.
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime.as_ref())
        .header(header::CACHE_CONTROL, cache)
        .body(Body::from(bytes))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn bundle_missing_response() -> Response {
    let html = r#"<!doctype html>
<html lang="fr"><head><meta charset="utf-8"><title>Sobr.ia — Mode Équipe</title>
<style>body{background:#0a0d0b;color:#f0ece3;font-family:system-ui,sans-serif;padding:40px;line-height:1.5}
code{background:#131815;padding:2px 6px;border-radius:4px;color:#c5f04a}</style></head>
<body>
<h1 style="color:#c5f04a">Sobr.ia — Dashboard équipe</h1>
<p>Le bundle frontend n'a pas été buildé. Lancez :</p>
<pre><code>cd web-team &amp;&amp; npm ci &amp;&amp; npm run build</code></pre>
<p>puis rebuildez le binaire <code>cargo build -p sobria-team-aggregator</code>.</p>
<p>L'API REST <code>/api/v1/*</code> reste fonctionnelle.</p>
</body></html>"#;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(html))
        .expect("response build")
}
