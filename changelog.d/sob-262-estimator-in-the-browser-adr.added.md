ADR-0027, the estimator in the browser (2026-10-31, SOB-262)

- **`docs/adr/ADR-0027-estimator-in-the-browser.md`** (Proposed): how the
  Understand pages compute their numbers in the browser on the web, as
  Thibault chose in SOB-246. A thin wrapper crate exposes one `call`
  function with JSON in and out. It is built with `cargo` and
  `wasm-bindgen-cli` at the version in `Cargo.lock`, included in every
  shipped build of `web/`, and loaded only in the web context. A parity test
  holds it to the native numbers. Five points are for Thibault, including
  where the Estimate page lives on the web.
- **`docs/adr/ADR-0021-single-app-desktop-and-web.md`**: Amendment 1,
  proposed with ADR-0027. In the web context, the Understand commands are
  answered by the module, and the CSP adds `'wasm-unsafe-eval'`.
