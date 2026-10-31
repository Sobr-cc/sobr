Characterization tests of sobria-referentiel (2026-10-31, SOB-220)

- **`sobria-referentiel`**: 8 tests in `tests/characterization.rs` record what
  the crate does today, without changing it. Without `SOBRIA_REFERENTIEL_PATH`,
  `load` opens `data/gold/referentiel.sqlite` from the current folder and finds
  nothing from another folder; `status().path` is that relative path although
  its documentation says absolute. `snapshot_date` is the modification time of
  the file, so a copy gets a new date. The hash changes when one byte changes.
  A file that is not SQLite opens, and only `status` fails. A missing `sources`
  table is an error while a missing `model_overview` is 0, and a
  `model_overview` that cannot be counted is reported as 0 models.
- The default-path tests run the test binary again as a child process with its
  own folder and environment, so no test changes the folder or the environment
  of the process. No production code changed.
- Pinned quirks are named `quirk`; the audit findings are left to SOB-20 (the
  relative paths of the app's `reload_referentiel` are SOB-126).
