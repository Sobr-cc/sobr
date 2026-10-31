//! Threshold alerts (brief §C29.4).
//!
//! Lets the admin define rules "if the gCO₂eq usage of a user (or of the
//! whole team) over a period (daily / weekly / monthly) exceeds
//! `gco2eq_max`, notify via webhook, email or plain
//! log_only".
//!
//! Wiring: called from `server::api::estimations::handle` after each
//! successful estimation insert. The logic is inline (5s timeout on
//! webhook) to stay simple; under higher load we will move to a
//! background job.
//!
//! See ADR-0013 Phase 2 and `docs/archive/chantiers/C29-v0.7.1-polish-mode-equipe.md`.

pub mod checker;
pub mod notify;
pub mod periods;
pub mod store;

pub use checker::{check_thresholds_for_user, AlertEvent};
pub use periods::{period_bounds, AlertPeriod};
pub use store::{
    delete_threshold, insert_threshold, list_active_thresholds, list_thresholds_admin,
    list_triggers_admin, AlertScope, NewThreshold, NotifyKind, Threshold, TriggerRow,
};
