The three zstd advisories are ignored until polars is upgraded (2026-10-31, SOB-261)

- **`deny.toml`, `.cargo/audit.toml`**: RUSTSEC-2026-0391, RUSTSEC-2026-0398 and RUSTSEC-2026-0403 (`zstd` 0.13 and `zstd-safe` 7, pulled by polars 0.46) are ignored, each with its reason and a re-check date of 2026-11-14. None of them can be reached through the way `polars-parquet` uses `zstd` (SOB-196). SOB-245 removes the entries when polars is upgraded. No other advisory is ignored.
