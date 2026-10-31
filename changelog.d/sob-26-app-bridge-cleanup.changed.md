sobria-app Deep cleanup, part 2: bridge, pairing and extension store (2026-10-31, SOB-26)

- **`sobria-app`**, no behavior change: the bridge socket bytes, the
  `BridgeResponse` JSON, stored data, error texts and the list of 307 tests
  are unchanged.
- **Shared code:**
  - one constructor turns a bridge or spool estimate into an extension
    event; each path keeps its own handling of token counts above
    `u32::MAX`;
  - the bridge handlers return `Result` and build replies from
    `BridgeResponse::ok`;
  - one secret check for Estimate and Revoke;
  - `NATIVE_HOST_NAME` and the 1 MB limit come from `sobria-bridge`;
  - the extension store opens through `db::open_referentiel` and uses the
    shared column parser and cached statements;
  - the manifest paths are built from one table per OS.
- **Docs:** comments translated to English. Docs that described missing code
  are fixed: the pipe is not limited to the current user, there is no
  cascading purge, and the spool drain checks no secret (SOB-119).
- **Filed instead of fixed:** SOB-279 (ship the bridge), SOB-285 (bridge
  server hardening), SOB-286 (one manifest per browser on Windows), SOB-287
  (message validation and spool drain).
