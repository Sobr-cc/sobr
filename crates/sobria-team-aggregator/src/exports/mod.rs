//! Exports of `sobria-team-aggregator`.
//!
//! Three formats for the admin:
//!
//! - [`csrd`]: AFNOR SPEC 2314 PDF + PROV-O sidecar through `sobria-export`
//!   (team estimations are converted into `AuditEntry` shims).
//! - [`prov_o`]: team-specific PROV-O variant (per-user `prov:Agent`,
//!   per-estimation `prov:Activity`).
//! - [`csv`]: flat RFC 4180 dump of the requested window.

pub mod csrd;
pub mod csv;
pub mod prov_o;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Parameters shared by the 3 exports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequest {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    /// Organization name for the PDF + PROV-O header.
    #[serde(default)]
    pub entity_name: Option<String>,
    /// Anonymizes fingerprints/display_names in PROV-O + CSV
    /// (the aggregated PDF reveals no identity by default).
    #[serde(default)]
    pub anonymize: bool,
}
