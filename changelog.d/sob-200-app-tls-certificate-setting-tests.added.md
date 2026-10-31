Tests of the self-signed certificate setting of the app (2026-10-31, SOB-200)

- **`sobria-app`**: 3 tests in `team_command_tests.rs` run `team_ping` against a
  TLS server with a self-signed certificate. With the setting off (the default)
  the ping fails with `transport` and the server sees the client break the
  handshake off; with `team_set_accept_invalid_certs(true)` it returns the health
  answer and records the last-seen time; turning the setting on, off and on again
  is followed on each call.
- Two dev-dependencies for these tests only, `rcgen` (makes the certificate at
  test time, so no private key is committed) and `tokio-rustls`, with the `ring`
  backend as in the team server. Both are already in `Cargo.lock`. No production
  code changed.
