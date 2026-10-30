# Golden files of the team server

These JSON files pin the **current behavior** of `sobria-team-aggregator` over HTTP
(SOB-10): status code, content type and body of the API endpoints, including the
refusals, and the privacy rules on aggregates (k-anonymity, ADR-0015; visibility
policies, ADR-0016). They are read by `tests/characterization_*.rs`.

- Each test starts a real HTTPS server and records every response in order.
- Values the server generates at random (ids, tokens, enrollment codes, clock
  timestamps, the crate version) are checked for their shape, then replaced by a
  placeholder such as `<ulid>` or `<jwt>`. A value of an unexpected shape fails the
  test. Employees and estimations are seeded straight in the database with fixed
  ids, so the numbers are fixed.
- Numbers are compared with a relative tolerance of 1e-9; everything else is exact.
- A failing test means the behavior of the server changed. That is not always a bug:
  an intended change is accepted by regenerating the files **and saying so in the PR**.

Regenerate, only if the change is intended:

```bash
SOBRIA_UPDATE_GOLDEN=1 cargo test -p sobria-team-aggregator --no-fail-fast
```

```powershell
$env:SOBRIA_UPDATE_GOLDEN = "1"; cargo test -p sobria-team-aggregator --no-fail-fast
Remove-Item Env:SOBRIA_UPDATE_GOLDEN
```

A test that rewrites a file **fails on purpose**, so that a variable left exported
cannot hide a regression: run again without it to check, then read `git diff`.
Only the value `1` rewrites, and it is refused when `CI` is set. If a test is
renamed, delete the old `.json` file by hand.

These tests do not say whether a behavior is **right**, only that it did not move.
A behavior that looks wrong is pinned as it is and filed as a separate issue; it is
not fixed here.
