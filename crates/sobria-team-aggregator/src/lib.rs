//! Sobr.ia `sobria-team-aggregator`: the self-hosted HTTPS server of team mode.
//!
//! See ADR-0013, ADR-0015, ADR-0016 and the crate README.

#![forbid(unsafe_code)]

pub mod alerts;
pub mod cli;
pub mod commands;
pub mod config;
pub mod crypto;
pub mod error;
pub mod exports;
pub mod policy;
pub mod server;
pub mod storage;
