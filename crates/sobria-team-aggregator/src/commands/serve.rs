//! `sobria-team-aggregator serve` command — starts the HTTPS server.
//!
//! Two entry points:
//!
//! - [`run`]: binds `bind:port` and waits forever. Used by the CLI.
//! - [`run_with_listener`]: takes an already bound `std::net::TcpListener`.
//!   Used by the integration tests to get a random port.

use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;

use anyhow::{Context, Result};
use axum_server::tls_rustls::RustlsConfig;

use crate::config::DataPaths;
use crate::server::{build_router, tls as server_tls, ServerState};
use crate::storage::Storage;

/// Checks that init has run, then builds the shared state and the TLS config.
fn prepare(paths: &DataPaths) -> Result<(ServerState, RustlsConfig)> {
    let storage = Storage::open(&paths.db()).context("open team.sqlite")?;
    let jwt_signing_key = storage
        .get_config(crate::commands::init::CFG_JWT_SIGNING_KEY)
        .context("lire jwt_signing_key")?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "data dir {} non initialisé — exécuter `init` d'abord",
                paths.as_path().display()
            )
        })?;

    let server_config = server_tls::load_server_config_arc(&paths.cert(), &paths.key())
        .context("charger cert TLS")?;
    let tls = RustlsConfig::from_config(Arc::clone(&server_config));

    let state = ServerState::new(storage, jwt_signing_key);
    Ok((state, tls))
}

/// Starts the server bound to `bind:port` (blocks until interrupted).
pub async fn run(paths: &DataPaths, bind: &str, port: u16) -> Result<()> {
    let (state, tls) = prepare(paths)?;
    spawn_retention_task(state.clone());
    let app = build_router(state);
    let addr: SocketAddr = format!("{bind}:{port}")
        .parse()
        .with_context(|| format!("addr invalide: {bind}:{port}"))?;

    tracing::info!(%addr, "sobria-team-aggregator HTTPS en écoute");
    axum_server::bind_rustls(addr, tls)
        .serve(app.into_make_service())
        .await
        .context("axum_server serve")?;
    Ok(())
}

/// Test variant: uses an already bound listener (any port).
pub async fn run_with_listener(listener: TcpListener, paths: &DataPaths) -> Result<()> {
    let (state, tls) = prepare(paths)?;
    spawn_retention_task(state.clone());
    let app = build_router(state);

    // axum_server expects a non-blocking `std::net::TcpListener`.
    listener
        .set_nonblocking(true)
        .context("set listener nonblocking")?;
    let addr = listener.local_addr().context("local_addr")?;
    tracing::info!(%addr, "sobria-team-aggregator HTTPS en écoute (listener externe)");

    axum_server::from_tcp_rustls(listener, tls)
        .serve(app.into_make_service())
        .await
        .context("axum_server serve from_tcp_rustls")?;
    Ok(())
}

// ─── Estimation retention (ADR-0015 "Conséquences") ────────────────────────

/// Purges at boot, then every 24 h, the estimations older than
/// `config.retention_days` (default 730 d, floor 30 — see
/// `commands::config::RUNTIME_KEYS`). Best-effort: a failure is logged,
/// never fatal to the server.
fn spawn_retention_task(state: crate::server::ServerState) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
        loop {
            tick.tick().await;
            match run_retention_purge(&state) {
                Ok((days, 0)) => tracing::debug!(retention_days = days, "rétention: rien à purger"),
                Ok((days, n)) => {
                    tracing::info!(
                        retention_days = days,
                        purged = n,
                        "rétention: purge effectuée"
                    );
                },
                Err(e) => tracing::warn!(error = %e, "rétention: purge échouée (non-fatal)"),
            }
        }
    });
}

/// Reads `retention_days` (default/floor from the CLI allow-list) and purges.
fn run_retention_purge(state: &crate::server::ServerState) -> anyhow::Result<(i64, u64)> {
    use crate::commands::config::RETENTION_DAYS;

    let storage = state
        .storage
        .lock()
        .map_err(|_| anyhow::anyhow!("storage mutex poisoned"))?;
    let days = storage
        .get_config(RETENTION_DAYS.key)?
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(RETENTION_DAYS.default)
        .max(RETENTION_DAYS.floor);
    let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
    let purged = crate::storage::estimations::purge_older_than(storage.connection(), cutoff)?;
    Ok((days, purged))
}
