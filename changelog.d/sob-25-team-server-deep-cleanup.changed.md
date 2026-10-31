sobria-team-aggregator Deep cleanup (2026-10-31, SOB-25)

- **`sobria-team-aggregator`**, no behavior change: the 16 golden files, error
  texts, CLI output and the list of 138 tests are unchanged.
- **Merged duplicates:**
  - one parser for SQLite timestamp columns, which replaces 6 copies;
  - one parser for the admin query dates, which replaces 3 copies;
  - one 12-digit enrollment code generator, which replaces 2 copies;
  - named runtime keys (`k_anonymity_min`, `retention_days`), read by the
    analytics and the retention purge;
  - `EmpreinteMethod` instead of hand-matched method strings.
- **Removed:** dead code, and the unused direct `tokio-rustls` dependency
  (TLS features unchanged).
- **Docs:**
  - README and stale module docs rewritten (routes, schema versions,
    migrations, storage);
  - two broken doc links fixed;
  - comments and rustdoc translated to English (CLAUDE.md §13). The CLI
    help text stays French, as it is product output.
- **Filed instead of fixed:**
  - SOB-250 (flaky webhook alert test);
  - SOB-252 (API types the app re-declares);
  - SOB-114 to SOB-117 and SOB-229 to SOB-235 (behavior changes from the
    audit).
