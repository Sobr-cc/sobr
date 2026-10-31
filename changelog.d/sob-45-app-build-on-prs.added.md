Unsigned desktop app build on each pull request (2026-10-31, SOB-45)

- **CI**: every pull request to `main` builds the desktop app with
  `cargo tauri build` (unsigned) on Windows, macOS (Apple silicon and Intel)
  and Linux, and keeps the bundles as run artifacts for 14 days. The build
  lives in the reusable workflow `.github/workflows/app-build.yml`, shared
  with `app-release.yml`, which now uploads to the release from a separate
  `publish` job.
