DuckDB marked as deferred in the docs (2026-10-31, SOB-174)

- **README**: the stack row says DuckDB is deferred to the analytics step
  (SOB-172) and no longer gives a version.
- **ADR-0003, Amendment 1** (Proposed): the decision stands; the `duckdb`
  crate is not in the build until the analytics step. Note for whoever adds it
  back: use `duckdb` 1.10506.1 or newer, since the `fmt` bundled with older
  versions does not build with MSVC 14.50 and later.
- **`sobria-geoloc`**: the doc comment of `territoire_fr.rs` no longer says
  that `sobria-ingest` uses `duckdb` (comment only).
