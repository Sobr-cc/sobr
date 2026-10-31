//! Visibility policy of the deployment (ADR-0016, C44).
//!
//! Three regimes, chosen by the ORGANIZATION (not by the admin on the fly:
//! the value lives in `config` and `identified` mode requires a recorded
//! attestation):
//!
//! - `anonymous`: k-anonymous aggregates only, no individual identification
//!   on the admin side (opt-ins are ignored in the display).
//! - `opt_in`: the default, ADR-0015 behavior (k-anonymity + identified
//!   sharing controlled by each employee).
//! - `identified`: full views by name; turning it on requires an attestation
//!   that the works council (CSE) and employees were informed (stored in the
//!   database, displayed).

use serde::{Deserialize, Serialize};

use crate::error::{AggregatorError, AggregatorResult};
use crate::storage::Storage;

/// Config key of the policy.
pub const POLICY_KEY: &str = "visibility_policy";
/// Config key of the attestation (JSON `{text, set_at, via}`).
pub const ATTESTATION_KEY: &str = "visibility_attestation";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisibilityPolicy {
    Anonymous,
    OptIn,
    Identified,
}

impl VisibilityPolicy {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Anonymous => "anonymous",
            Self::OptIn => "opt_in",
            Self::Identified => "identified",
        }
    }

    pub fn parse(s: &str) -> AggregatorResult<Self> {
        match s {
            "anonymous" => Ok(Self::Anonymous),
            "opt_in" => Ok(Self::OptIn),
            "identified" => Ok(Self::Identified),
            other => Err(AggregatorError::Config(format!(
                "visibility_policy inconnue: {other} (anonymous | opt_in | identified)"
            ))),
        }
    }
}

/// Reads the effective policy (`opt_in` if missing or unreadable).
pub fn load(storage: &Storage) -> VisibilityPolicy {
    storage
        .get_config(POLICY_KEY)
        .ok()
        .flatten()
        .and_then(|v| VisibilityPolicy::parse(&v).ok())
        .unwrap_or(VisibilityPolicy::OptIn)
}

/// Attestation stored with the `identified` mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Attestation {
    pub text: String,
    /// RFC 3339.
    pub set_at: String,
    /// Origin (`cli`, `api`).
    pub via: String,
}

pub fn load_attestation(storage: &Storage) -> Option<Attestation> {
    storage
        .get_config(ATTESTATION_KEY)
        .ok()
        .flatten()
        .and_then(|v| serde_json::from_str(&v).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_roundtrip_and_default() {
        for p in [
            VisibilityPolicy::Anonymous,
            VisibilityPolicy::OptIn,
            VisibilityPolicy::Identified,
        ] {
            assert_eq!(VisibilityPolicy::parse(p.as_str()).unwrap(), p);
        }
        assert!(VisibilityPolicy::parse("nominatif").is_err());
        let s = Storage::open_in_memory().unwrap();
        assert_eq!(load(&s), VisibilityPolicy::OptIn);
        s.set_config(POLICY_KEY, "identified").unwrap();
        assert_eq!(load(&s), VisibilityPolicy::Identified);
        s.set_config(POLICY_KEY, "garbage").unwrap();
        assert_eq!(load(&s), VisibilityPolicy::OptIn);
    }
}
