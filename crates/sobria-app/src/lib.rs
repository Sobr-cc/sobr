//! # sobria-app — Tauri IPC facade
//!
//! This library exposes the **business core** as testable functions,
//! independent of the Tauri runtime. The `sobria-app` binary
//! registers these functions as IPC commands through `#[tauri::command]`,
//! but the inner functions (`logic::*`) can be **called directly
//! from tests** without starting the native window.
//!
//! See `docs/archive/chantiers/C09-tauri-integration.md`.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::needless_pass_by_value)]

pub mod batch;
pub mod bridge_install;
pub mod bridge_server;
pub mod dashboard;
pub mod dto;
pub mod error;
pub mod extension_store;
pub mod goals_store;
pub mod logic;
pub mod pairing;
pub mod preferences_store;
pub mod project_store;
pub mod state;
pub mod team_client;
pub mod team_settings;

pub use error::{IpcError, IpcResult};
pub use state::AppState;
