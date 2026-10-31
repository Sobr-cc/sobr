Territory data contract tests (2026-10-31, SOB-189)

- **`sobria-geoloc`**: `tests/ingest_contract.rs` reads two samples of the ingest
  output (`tests/fixtures/territoire_fr.sample.json`, `rte_mix_fr.sample.json`)
  with the geoloc structs and checks every field; any key renamed by the ingest
  side is refused; fields geoloc does not know are ignored in every struct; the
  shipped `data/territoire_fr.json` is read and validated.
- **`sobria-ingest`**: `tests/territoire_contract.rs` checks that the ingest and
  geoloc structs both write exactly the fields of those samples, and that a file
  written by `write_artifact_json` is loaded by geoloc (the test lives here
  because ingest depends on geoloc). `tests/territoire_fetch_failures.rs` pins the
  transport-failure message of `discover_datasets`, `fetch_industrial_sites` and
  `fetch_rte_mix` through a local refusing proxy. The success, HTTP error, empty
  and malformed-JSON paths of these functions need an injectable base URL, which
  needs a production change and is tracked in SOB-194. No production code changed.
