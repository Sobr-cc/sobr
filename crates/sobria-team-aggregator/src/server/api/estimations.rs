//! POST /api/v1/estimations — an authenticated user pushes an estimation.
//!
//! The body is the `EstimatePayload v1` of extension v0.6.0 (camelCase):
//!
//! ```json
//! {
//!   "estimate": { "method": "afnor_sobria", "modelId": "llama-3-1-70b",
//!                 "tokensIn": 120, "tokensOut": 800, "gco2eq": 0.42,
//!                 "waterMl": 1.5, "energyWh": 0.42, ... },
//!   "host": "chatgpt",
//!   "modelDisplayName": "GPT-4o",
//!   "ts": "2026-05-16T12:34:56Z"
//! }
//! ```
//!
//! We extract the typed fields into `estimations` (dedicated columns
//! for analytics) and keep the full payload in `raw_payload_json`
//! for audit / reproducibility.

use axum::{extract::State, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sobria_core::EmpreinteMethod;
use ulid::Ulid;

use crate::alerts::{self, store::mark_trigger_notified, NotifyKind};
use crate::server::auth::middleware::AuthenticatedUser;
use crate::server::error::{ApiError, ApiResult};
use crate::server::ServerState;
use crate::storage::{estimations, users};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimatePayload {
    pub estimate: Estimate,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub model_display_name: Option<String>,
    pub ts: DateTime<Utc>,
    /// Optional region (the current extension does not send this field;
    /// accepted for future compatibility).
    #[serde(default)]
    pub region: Option<String>,
    /// Project label — tagged per conversation by the extension.
    /// Normalized: trim + 64 chars max, empty → None.
    #[serde(default)]
    pub project: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Estimate {
    pub method: String,
    pub model_id: String,
    pub tokens_in: u32,
    pub tokens_out: u32,
    pub gco2eq: f64,
    pub water_ml: f64,
    pub energy_wh: f64,
}

#[derive(Debug, Serialize)]
pub struct EstimateAck {
    pub id: String,
    pub ack: bool,
}

pub async fn handle(
    State(state): State<ServerState>,
    user: AuthenticatedUser,
    Json(payload): Json<EstimatePayload>,
) -> ApiResult<Json<EstimateAck>> {
    // Business guards: no arbitrary method, no negative tokens
    // (the u32 type already covers it), no absurd gCO₂eq.
    if EmpreinteMethod::from_str(&payload.estimate.method).is_none() {
        return Err(ApiError::BadRequest(format!(
            "methode inconnue: {}",
            payload.estimate.method
        )));
    }
    if !payload.estimate.gco2eq.is_finite() || payload.estimate.gco2eq < 0.0 {
        return Err(ApiError::BadRequest("gco2eq invalide".into()));
    }

    let now = Utc::now();

    // Keep the full payload for audit + reproducibility.
    let raw = serde_json::to_string(&serde_json::json!({
        "estimate": {
            "method": payload.estimate.method,
            "modelId": payload.estimate.model_id,
            "tokensIn": payload.estimate.tokens_in,
            "tokensOut": payload.estimate.tokens_out,
            "gco2eq": payload.estimate.gco2eq,
            "waterMl": payload.estimate.water_ml,
            "energyWh": payload.estimate.energy_wh,
        },
        "host": payload.host,
        "modelDisplayName": payload.model_display_name,
        "ts": payload.ts,
        "region": payload.region,
    }))
    .unwrap_or_else(|_| "{}".to_string());

    let id = Ulid::new().to_string();
    // Project label normalization: trim, 64 chars max,
    // empty → None (NULL in the database = "no project").
    let project_norm = payload
        .project
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| p.chars().take(64).collect::<String>());
    let new_estimation = estimations::NewEstimation {
        id: &id,
        user_id: &user.claims.sub,
        ts: payload.ts,
        method: &payload.estimate.method,
        model_id: &payload.estimate.model_id,
        tokens_in: payload.estimate.tokens_in,
        tokens_out: payload.estimate.tokens_out,
        gco2eq_p50: payload.estimate.gco2eq,
        gco2eq_p5: None,
        gco2eq_p95: None,
        water_ml: payload.estimate.water_ml,
        energy_wh: payload.estimate.energy_wh,
        region: payload.region.as_deref(),
        project: project_norm.as_deref(),
        raw_payload_json: &raw,
        received_at: now,
    };

    let storage = state.lock_storage()?;
    estimations::insert(storage.connection(), &new_estimation)?;
    let _ = users::touch_last_seen(storage.connection(), &user.claims.sub, now);

    // Check the alert thresholds that apply to this user
    // (team thresholds included). Detected events are inserted in the database
    // under the lock; the async notification then runs outside the lock.
    let events = alerts::check_thresholds_for_user(storage.connection(), &user.claims.sub, now)
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "alerts: check échoué — ack quand même");
            Vec::new()
        });
    // Snapshot the SMTP config before dropping the lock (email events only).
    let smtp_cfg = if events
        .iter()
        .any(|ev| ev.threshold.notify_kind == NotifyKind::Email)
    {
        alerts::notify::read_smtp_config(&storage)
    } else {
        None
    };
    drop(storage);

    if !events.is_empty() {
        let state_for_persist = state.clone();
        tokio::spawn(async move {
            for ev in events {
                let outcome = alerts::notify::notify(&ev, smtp_cfg.as_ref()).await;
                let Ok(s) = state_for_persist.storage.lock() else {
                    continue;
                };
                let _ = mark_trigger_notified(
                    s.connection(),
                    &ev.trigger_id,
                    Utc::now(),
                    outcome.error.as_deref(),
                );
            }
        });
    }

    Ok(Json(EstimateAck { id, ack: true }))
}
