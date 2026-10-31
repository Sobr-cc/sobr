Stub-server tests of the ODRÉ territory fetches (2026-10-31, SOB-194)

- **`sobria-ingest`** (`sources/territoire_fr.rs`): `discover_datasets_at`,
  `fetch_industrial_sites_at` and `fetch_rte_mix_at` take the Opendatasoft
  base URL (a mirror, or a stub server); the existing functions call them
  with the ODRÉ address, so the CLI is unchanged.
- `tests/territoire_fetch_stub.rs` pins today's behavior of the three fetches
  against a wiremock server: success, empty answers, HTTP errors, malformed
  JSON, the pagination of the industrial sites and the year left at 0 when
  `annee` is missing (SOB-122).
