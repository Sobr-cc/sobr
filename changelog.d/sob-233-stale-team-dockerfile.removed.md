Stale team server Dockerfile in the crate folder (2026-10-31, SOB-233)

- **`sobria-team-aggregator`**: the old Dockerfile at the crate root is
  removed. It pinned Rust 1.79, but the crate calls `usize::is_multiple_of`
  (Rust 1.87). `deploy/team/Dockerfile`, the one CI builds, is the only team
  server image. The release notes of `team-aggregator-release.yml` and the
  README now point to it.
