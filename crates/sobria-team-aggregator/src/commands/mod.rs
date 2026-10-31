//! CLI commands of `sobria-team-aggregator`.
//!
//! - [`init`]: prepares a blank data dir (DB + cert + admin).
//! - [`serve`]: starts the axum HTTPS server.
//! - [`code`]: creates / lists / revokes enrollment codes.
//! - [`admin`]: reset-password + list.

pub mod admin;
pub mod code;
pub mod config;
pub mod init;
pub mod serve;
