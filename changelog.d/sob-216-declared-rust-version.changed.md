The declared Rust version is 1.89 (2026-10-31, SOB-216)

- **`Cargo.toml`, `CLAUDE.md` (section 3), `README.md`**: the minimum Rust version goes from 1.79 to 1.89, the highest `rust_version` among the packages locked in `Cargo.lock` (`aes` 0.9.3). Nearly every dependency already needed more than 1.79 (`tokio`, `quick-xml`, `plist`, `time`), so the old figure was false. The toolchain channel stays `stable`.
