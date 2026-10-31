sobria-estimator Deep cleanup (2026-10-31, SOB-19)

- **`sobria-estimator`**, no output change (golden files, error texts and the
  180 existing tests unchanged): one definition of the default PUE, WUE and
  electricity mix; shared helpers for the indicators, equivalents,
  hypotheses and indicator lookup of both engines; the two forecasts share
  their checks and growth step (`forecast.rs`, now with its own tests); the
  unit tests use `validation::request_for` as their one request fixture.
  Wrong docs fixed (reproduction targets computed from the presets, J
  written for kJ, "annual" fields, a missing attribution file, the seed,
  `# Panics`), history comments removed, README rewritten.
- Code comments and rustdoc of the crate translated to English (CLAUDE.md
  §13); error and panic texts stay French, as they are product output.
- Filed instead of fixed: SOB-209 (unsourced figures and claims), SOB-210
  (estimator behaviors), SOB-227 (module structure), SOB-228 (logic the app
  re-implements).
