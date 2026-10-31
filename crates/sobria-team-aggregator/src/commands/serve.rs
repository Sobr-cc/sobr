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
use axum::Router;
use axum_server::tls_rustls::RustlsConfig;

use crate::config::DataPaths;
use crate::server::{build_router, tls as server_tls, ServerState};
use crate::storage::Storage;

/// Checks that init has run, loads the TLS config, starts the retention task
/// and builds the router.
fn prepare(paths: &DataPaths) -> Result<(Router, RustlsConfig)> {
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

    let server_config =
        server_tls::load_server_config(&paths.cert(), &paths.key()).context("charger cert TLS")?;
    let tls = RustlsConfig::from_config(Arc::new(server_config));

    let state = ServerState::new(storage, jwt_signing_key);
    spawn_retention_task(state.clone());
    Ok((build_router(state), tls))
}

/// Starts the server bound to `bind:port` (blocks until interrupted).
pub async fn run(paths: &DataPaths, bind: &str, port: u16) -> Result<()> {
    let (app, tls) = prepare(paths)?;
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
    let (app, tls) = prepare(paths)?;

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
/// `config::RUNTIME_KEYS`). Best-effort: a failure is logged,
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

/// Reads `retention_days` (default and floor from `config::RETENTION_DAYS`) and purges.
fn run_retention_purge(state: &crate::server::ServerState) -> anyhow::Result<(i64, u64)> {
    use crate::config::RETENTION_DAYS;

    let storage = state
        .storage
        .lock()
        .map_err(|_| anyhow::anyhow!("storage mutex poisoned"))?;
    let days = RETENTION_DAYS.effective(&storage)?;
    let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
    let purged = crate::storage::estimations::purge_older_than(storage.connection(), cutoff)?;
    Ok((days, purged))
}
