quinn-proto updated to fix five advisories (2026-10-31, SOB-215)

- **`Cargo.lock`**: `quinn-proto` 0.11.14 to 0.11.19 (adds `rand_pcg` 0.10.2) fixes RUSTSEC-2026-0185, RUSTSEC-2026-0338, RUSTSEC-2026-0339, RUSTSEC-2026-0340 and RUSTSEC-2026-0342, which `cargo audit` reports. No target builds `quinn-proto` (it is behind the unused `http3` feature of `reqwest`), so the compiled code does not change. The `getrandom` 0.3.4 entry loses its `js-sys` and `wasm-bindgen` dependencies, which only `quinn-proto` 0.11.14 asked for.
