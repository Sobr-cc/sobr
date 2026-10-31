Characterization tests of the app's team logic and bridge socket (2026-10-31, SOB-188)

- **`sobria-app`**: tests record what the code does today. They call the
  eight `team_*` commands of `main.rs` (status, URL, mode, certificates,
  ping, enroll, logout, push of an estimation) against a local stub HTTP
  server, and start the real bridge server to talk to it through a real Unix
  socket or Windows named pipe (valid and refused requests, a connection that
  breaks the protocol, a silent client, the size limit, the real
  `sobria-bridge` client). They are `#[cfg(test)]` modules of `main.rs`: a
  test executable that links Tauri does not start on Windows outside the
  binary target (`STATUS_ENTRYPOINT_NOT_FOUND`).
- **Test hooks, no behavior change**: `bridge_server::serve` runs the server
  on an explicit endpoint and `run` calls it; the server functions are generic
  over the Tauri runtime, so a test can use a mock app; `tauri`'s empty `test`
  feature is enabled in the dev-dependencies.
- The header of `tests/bridge_server.rs` said that a Windows test and a
  `SOBRIA_BRIDGE_SOCKET` variable existed; neither does. It now says what the
  file does.
- Not covered: the effect of the "accept invalid certificates" setting on a
  TLS connection (only the stored flag is pinned).
- Pinned quirks, filed in Triage: the new token pair is lost when the retry
  after a refresh fails (SOB-199), and `team_set_url("https://")` is stored as
  `https:` (SOB-201). The certificate setting is SOB-200; the dead variable
  of `tests/bridge_server.rs` is SOB-202.
