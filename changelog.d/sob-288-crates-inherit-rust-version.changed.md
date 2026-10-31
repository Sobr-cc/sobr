The workspace crates declare the minimum Rust version (2026-10-31, SOB-288)

- **`crates/*/Cargo.toml`**: the ten workspace crates now inherit
  `rust-version` from `[workspace.package]` (1.89). Cargo then refuses an
  older compiler by naming our crates, and tools that read a crate's minimum
  Rust version see it. Before, only the dependency `aes` carried the floor.
