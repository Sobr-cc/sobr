The 20 MB binary size limit applies to the desktop app only (2026-10-31, SOB-240)

- **`.github/workflows/ci.yml`**, job "Frugality — binary sizes": only `target/release/sobria-app` is held to the 20 MB limit of NF-03 (a target for the desktop binary, CDC line 392). The log still prints the size of every `sobria-*` binary, also when the app is over the limit, and the job fails if `sobria-app` is missing, so the check cannot pass without looking at it. `sobria-ingest` (21.7 MB) no longer fails the job.
