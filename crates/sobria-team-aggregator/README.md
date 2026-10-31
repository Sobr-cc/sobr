# sobria-team-aggregator

Self-hosted HTTPS server of the Sobr.ia **team mode** (ADR-0013, ADR-0015,
ADR-0016).

A company runs this binary on its own machine (admin workstation, NAS,
internal VPS) to aggregate the carbon estimates of its employees, sent by the
browser extension and the desktop app. **No Sobr.ia cloud is involved.**

## What it does

- **CLI:** `init`, `serve`, `code create|list|revoke`,
  `admin list|reset-password`, `config list|get|set`. Run `--help` on each
  command for its options.
- **HTTPS API** (axum, rustls with ring):
  - `/health`;
  - under `/api/v1`: `enroll`, `login`, `refresh`, `estimations`,
    `me/usage`, `me/sharing`;
  - under `/api/v1/admin`: users, enrollment codes, analytics, per-user
    analytics, CSRD/PROV-O/CSV exports, alerts and alert triggers.
- **Team dashboard:** `web-team/build/` is embedded at build time and served
  at `/`.
- **Runtime settings** (`config`): the visibility policy (`anonymous`,
  `opt_in`, `identified`, ADR-0016), the k-anonymity threshold of the
  analytics (`k_anonymity_min`, ADR-0015) and the retention of estimations
  (`retention_days`).

## Quickstart

```bash
# 1. Create a data dir (database, TLS certificate, first admin)
cargo run -p sobria-team-aggregator -- \
    --data-dir ./team-data \
    init \
    --admin-username admin \
    --admin-password 'change-me-quickly'

# 2. Start the HTTPS server
cargo run -p sobria-team-aggregator -- \
    --data-dir ./team-data \
    serve --port 8443

# 3. Check /health from another terminal
curl -k https://localhost:8443/health
# → {"ok":true,"version":"<crate version>"}
```

The data dir holds:

- `team.sqlite`: the SQLite database (WAL, foreign keys).
- `cert.pem`: the self-signed certificate (valid 10 years; SANs `localhost`,
  `127.0.0.1`, `::1` and the OS hostname).
- `key.pem`: the ECDSA P-256 private key (mode 600 on Unix).

## Security

- No OpenSSL: rustls and ring only.
- Argon2id hashes for admin passwords, enrollment codes and refresh tokens.
  Access tokens are JWTs valid 24 h.
- Protect the data dir with OS permissions (see
  `docs/operations/team-aggregator.md`).
- The binary sends nothing to Sobr.ia (CLAUDE.md §7).

## Tests

```bash
cargo test -p sobria-team-aggregator
```

`tests/characterization_*.rs` pins today's API outputs in `tests/golden/`
(SOB-10); see `tests/golden/README.md` before changing a golden file.

## See also

- ADR-0013 (extension, pairing and team mode), ADR-0015 (team mode privacy),
  ADR-0016 (visibility policy).
- `docs/operations/deploiement-equipe.md`: deployment guide.
- `docs/archive/chantiers/C28-mode-equipe-self-hosted.md`: original brief.
