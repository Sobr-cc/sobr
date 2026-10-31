Characterization tests of sobria-core (2026-10-31, SOB-219)

- **`sobria-core`**: `tests/characterization/` pins what the other crates store and
  send. Golden JSON files (`tests/golden/`, read back by the tests) fix the shape of
  `EstimationRequest` (with every field and with none of the optional ones),
  `EstimationResult`, `ContextOverhead`, every `InputModality` variant,
  `UncertaintyInterval` and the identifiers of personas, modules, methods and
  indicators; the written key order is pinned separately, because the comparator
  sorts keys. Legacy requests and results (optional fields absent, unknown fields)
  are read and written back; the exact module list of each persona bundle and the
  order of `ModuleId::all()` are pinned; each limit of `EstimationRequest::validate`
  is pinned at its edge with its error text. The issue also names `TokenCounts`,
  which does not exist in the code. No production code changed.
