//! Characterization tests of `sobria-core` (SOB-219): the JSON shapes the other crates
//! store and send, the legacy payloads that must stay readable, the persona bundles and the
//! limits of `EstimationRequest::validate`.
//!
//! The golden files are in `tests/golden/`; see `test-support/golden.rs` to rewrite them.

#[path = "../../../../test-support/golden.rs"]
mod golden;

mod bundles;
mod legacy;
mod limits;
mod shapes;
