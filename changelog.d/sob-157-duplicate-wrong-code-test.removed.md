Duplicate wrong-code pairing test (2026-10-31, SOB-157)

- **`sobria-app`** (`bridge_server.rs`, test module): removed
  `pair_with_wrong_code_returns_invalid_error`, a looser duplicate of
  `pair_with_wrong_six_digit_code_gets_the_invalid_message`. No production
  code changed.
