The local coverage command uses the LLVM engine too (2026-10-31, SOB-253)

- **`tarpaulin.toml`**: the LLVM engine and `exclude-files = ["test-support/*"]` are set once for `cargo tarpaulin --workspace`, the command of CLAUDE.md section 10 and CONTRIBUTING.md. The default ptrace engine fails to compile the tests on Linux (`polars-arrow` 0.46, SOB-208), and the test helper `test-support/golden.rs` no longer counts as uncovered lines.
- **`.github/workflows/ci.yml`**: the coverage job has no engine flag on its command line any more, and installs `cargo-tarpaulin` 0.37.5 instead of the latest release.
