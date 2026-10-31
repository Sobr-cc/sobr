Intel macOS app build no longer uses the retired macos-13 runner (2026-10-31, SOB-45)

- **CI**: GitHub retired the `macos-13` runner on 2025-12-04. The Intel
  macOS build now runs on `macos-latest` with `--target x86_64-apple-darwin`.
