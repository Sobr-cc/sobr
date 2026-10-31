CI covers the whole repo (2026-10-31, SOB-8)

- **`.github/workflows/ci.yml`**: new jobs for web-team (lint, check, build) and
  the site (docs sync, check, lint, build); the extension job also runs
  `tsc --noEmit`; the Playwright job runs two suites, `a11y` and the new `smoke`.
  The Linux jobs that compile the Tauri crates install the system libraries
  first (composite action `.github/actions/tauri-linux-deps`), which fixes the
  missing `glib-2.0`. Comments and job names are in English.
- **`web/tests/smoke/smoke.spec.ts`**, `npm run test:smoke`: opens the first
  screen, waits for the app to mount (the root layout patches `history.pushState`
  then) and fails on a page or console error. It fails with JavaScript disabled
  and with an error thrown in the layout's `onMount`, so it cannot pass on the
  server-rendered HTML alone.
- Not part of this entry: Windows `duckdb` (SOB-168), the macOS tests (SOB-167)
  and the `cargo audit` advisories (SOB-125).
