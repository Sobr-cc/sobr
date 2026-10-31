Cheaper desktop build on pull requests (2026-10-31, SOB-251)

- **CI, "Desktop app — unsigned build"**: it runs only when a pull request
  changes the app: `crates/`, `web/`, `Cargo.toml`, `Cargo.lock`,
  `rust-toolchain.toml`, `ci.yml`, `app-build.yml` or `.github/actions/`. A
  new "Desktop app — changed paths" job decides, so docs-only pull requests
  skip the four builds.
- **Tauri CLI**: `@tauri-apps/cli` 2.11.1 is a pinned dev dependency of
  `web/`, installed with `npm ci` as prebuilt binaries. It matches the
  `tauri` crate in `Cargo.lock`, and a step fails when their minor versions
  differ. Compiling `tauri-cli` took about 6 minutes per runner, and
  `--version "^2"` took whatever 2.x was newest.
