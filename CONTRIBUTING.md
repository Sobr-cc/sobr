# Contributing to Sobr.ia

Thank you for your interest in the project. Sobr.ia is open source (MIT, Etalab 2.0, CC-BY) and open to contributions.

---

## Before you start

1. Read [`README.md`](README.md), [`CLAUDE.md`](CLAUDE.md) and the [specification](docs/CAHIER-DES-CHARGES-v1.0.md).
2. Find the active sprint in [`docs/ROADMAP.md`](docs/ROADMAP.md) and its brief.
3. Follow the [ADRs](docs/adr/): the architecture decisions are settled.

---

## Contribution cycle

1. **Discuss first:** open a GitHub issue that describes the idea before you write code.
2. **Fork or branch:** branch `feature/<scope>-<short-desc>` or `fix/<short-desc>`
   (maintainers: prefix with the Linear issue, for example `feature/sob-123-<short-desc>`; ADR-0018).
3. **Code** following the [Definition of Done](CLAUDE.md#5-definition-of-done-dod).
4. **Changelog:** add one fragment file in [`changelog.d/`](changelog.d/README.md), named `<issue>-<slug>.<category>.md`. Without a Linear issue, `<issue>` is `gh-` and the number of your pull request, for example `gh-58-retry-the-upload.fixed.md`. Do not edit `CHANGELOG.md`: the fragments are moved there at release.
5. **Commits** in the Conventional Commits format (see CLAUDE.md §4.3).
6. **Pull request:** reference the issue and the ADRs it touches.
7. **Review:** at least one maintainer, green CI, and every discussion resolved.

---

## Technical standards

| Aspect | Tool | Target |
|--------|------|--------|
| Rust format | `cargo fmt --all` | required |
| Rust lint | `cargo clippy --workspace --all-targets -- -D warnings` | required |
| Test coverage | `cargo tarpaulin --workspace` | ≥ 80 % |
| TS format | `prettier` | required |
| TS lint | strict `eslint` | required |
| Security audit | `cargo audit` + `cargo deny check` | 0 critical |
| Accessibility | `axe-core` through Playwright | RGAA AA |

---

## Developer Certificate of Origin (DCO)

By opening a PR, you certify that you have the right to submit the
contribution and that you accept the DCO ([official text](https://developercertificate.org/)).

Sign your commits:

```bash
git commit -s -m "feat(estimator): add Monte-Carlo propagation"
```

---

## Scientific method: strict rules

See CLAUDE.md §6 and `docs/methodology/`.

- ✅ Every formula is sourced (a comment with a DOI or URL).
- ✅ Every assumption has a documented uncertainty distribution.
- ✅ Every result shows a P5-P95 interval.
- ✅ Reproducible with a seed (`SOBRIA_SEED`, 42 by default).
- ❌ No "magic" number without a citation.

---

## Medallion architecture: non-negotiable rules

See ADR-0009.

- All external data goes through Copper → Silver → Gold.
- A new source = one `DataLayer` trait to implement.
- No *ad hoc* transformation outside the pipeline.

---

## Communication

- **Issues:** for bugs, ideas and questions.
- **Discussions:** for architecture topics.
- **Maintainers' email:** for security or a conflict (see SECURITY.md).

All exchanges follow the [Code of Conduct](CODE_OF_CONDUCT.md) (Contributor Covenant 2.1).
