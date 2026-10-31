//! Fixtures shared by the unit tests.

use chrono::Utc;
use sobria_core::{ContextOverhead, EstimationRequest};

/// A text-only request: no datacenter, no modality, no context overhead.
pub(crate) fn request(model_id: &str, tokens_in: u32, tokens_out: u32) -> EstimationRequest {
    EstimationRequest {
        model_id: model_id.into(),
        tokens_in,
        tokens_out_estimated: tokens_out,
        datacenter_id: None,
        timestamp: Utc::now(),
        modalities: Vec::new(),
        overhead: ContextOverhead::default(),
    }
}
