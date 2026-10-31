Rust coverage job compiles again (2026-10-31, SOB-208)

- **CI, "Rust — coverage (informational)"**: `cargo tarpaulin` now runs with
  `--engine llvm`. The default ptrace engine passes `-Clink-dead-code` to
  rustc. That flag compiles the trait methods of `polars-arrow` that are never
  called, which fires their `const { assert!(false, "should not be called") }`
  (`polars-utils` macro `no_call_const!`), so the tests never compiled. The
  job stays informational, with no coverage gate.
