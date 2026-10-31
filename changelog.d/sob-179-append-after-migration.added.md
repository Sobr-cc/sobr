Append after the audit ledger migration (2026-10-31, SOB-179)

- **`sobria-audit`**: `tests/ledger_migration.rs` appends an entry to a migrated
  copy of the v1 fixture and pins that the new entry gets id 5, chains on the
  signature of entry 4, stores the `method` of the appended result (not the
  migration default), leaves the four historical rows unchanged, and that the
  chain verifies with five entries. No production code changed.
