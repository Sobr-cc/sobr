Audit ledger signature, chain, schema, export and purge tests (2026-10-31, SOB-223)

- **`sobria-audit`**: `tests/ledger_integrity.rs` pins golden signature values
  (single entries, fractional seconds, a three-entry chain), the schema
  (v2 columns, the two indexes, WAL), the exact NDJSON lines of fixed entries and
  the `purge_before` boundary and count. Four weaknesses of `verify_chain` are
  pinned as they are, in tests named `quirk_*`: cut last entries, an edited
  `method`, a row marked purged that skips its own check, and a panic on a
  non-ASCII `prev_sig`; values shifted across the `|` separator give the same
  signature. They are tracked in SOB-237 and SOB-238. No production code changed.
