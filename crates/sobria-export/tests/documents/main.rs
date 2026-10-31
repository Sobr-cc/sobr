//! Characterization tests of the documents `sobria-export` writes (SOB-221):
//! the CSRD report (PDF and PROV-O) and the Gebru datasheet.
//!
//! They record what the documents say today, not that they are well formed
//! (the unit tests in `src/` do that). Tests named `quirk` pin behavior the
//! codebase audit flagged; SOB-23 (cleanup) and the issues named in the PR
//! decide what to do with it. One test binary, so the crate is linked once.
//!
//! To rewrite the golden files, see `test-support/golden.rs`
//! (`SOBRIA_UPDATE_GOLDEN=1`).

mod common;
mod datasheet_documents;
mod report_documents;
