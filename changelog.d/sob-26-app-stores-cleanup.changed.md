sobria-app Deep cleanup, part 1: stores, state and errors (2026-10-31, SOB-26)

- **`sobria-app`**, no behavior change: Tauri commands, stored data, error
  texts and the list of 307 tests are unchanged.
- **Shared code:**
  - one `db::open_referentiel` opens `referentiel.sqlite` in WAL mode for
    the preferences, goals, project and team settings stores (the extension
    store follows in part 2);
  - one RFC 3339 column parser, used by the project store;
  - JSON read and write helpers in the preferences store;
  - cached SQL statements for the preferences and team settings keys and
    the project reads.
- **Removed:** the unused `sha2` dependency, single-statement transactions,
  and serde derives that no code used (one of them encoded `co2eq` as
  `co2_eq`).
- **Docs:** comments of the store, state and error files translated to
  English; the reserved preference keys and `AppState::init` docs fixed.
