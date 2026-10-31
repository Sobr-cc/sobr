# CLAUDE.md — Sobr.ia project context

> **This file is read first by Claude Code at every session.**
> It holds all the context needed to produce code aligned with the project, its constraints and its conventions.

---

## 1. The project in one sentence

**Sobr.ia** is a native cross-platform application (Rust + Tauri 2 + SvelteKit) that measures and visualizes the environmental impact of LLM use, uses the official datasets of the data.gouv.fr challenge (**ComparIA** + **RTE IRIS** industrial sites), adds a French territorial angle at IRIS level, and comes with a browser extension, a consolidated dataset and a validated scientific methodology — a submission to the data.gouv.fr challenge « L'impact environnemental de l'IA générative ».

---

## 2. Reference documents (read before acting)

| Document | When to read it |
|----------|-----------------|
| `docs/CAHIER-DES-CHARGES-v1.0.md` (the CDC, i.e. the specification) | Always first, gives the full scope |
| Linear (`linear.app/sobrcc`, team `SOB`) | To know what is in progress — source of truth for statuses (ADR-0018) |
| `docs/ROADMAP.md` | Initial plan, kept for the record (statuses no longer maintained) |
| `docs/adr/*.md` | To respect the architecture decisions |
| `docs/adr/ADR-0009-medallion-architecture.md` | **Before any ingestion work** — Copper/Silver/Gold pattern imposed |
| `briefs/chantiers/Cxx-*.md` | Only when the Linear issue or its project links one (ADR-0018, Amendment 1) |
| `docs/sources/CATALOGUE-SOURCES.md` | Before any work on ingestion |
| `docs/methodology/AFNOR-SPEC-2314-synthese.md` | Before any work on the estimator |

---

## 3. Imposed tech stack

| Layer | Technology | Target version |
|-------|------------|----------------|
| Native wrapper | Tauri | 2.x |
| Backend | Rust | stable (≥ 1.79) |
| Frontend | SvelteKit + TypeScript | SvelteKit 2.x, TS 5.x |
| Dataviz | Observable Plot + D3 | Plot 0.6+, D3 7.x |
| Transactional DB | SQLite (WAL) via `rusqlite` | rusqlite ≥ 0.31 |
| Analytical DB | DuckDB via `duckdb-rs` | duckdb-rs ≥ 1.0 |
| Rust ETL | reqwest + serde + polars-rs | latest stable |
| Notebook | Quarto | 1.4+ |
| Browser extension | WebExtension MV3 (TypeScript) | Chrome 120+, Firefox 120+ |
| Data versioning | DVC | 3.x |
| CI/CD | GitHub Actions | - |

**Anti-patterns to avoid**:
- ❌ No Electron (the project is anti-Electron on principle).
- ❌ No Node.js as a runtime (except for build / extension).
- ❌ No heavy UI framework on the Svelte side (light custom Skeleton CSS only).
- ❌ No tracking / telemetry without explicit opt-in.
- ❌ No sending of user prompts to an external server.
- ❌ No optional dependencies that an ADR does not justify.
- ❌ No blocking API key (v1.0 must be installable and work without any key).
- ❌ No environmental methodology parallel to EcoLogits without justification: **we align with the official ComparIA / Data for Good methodology**.

**Priority datasets (Tier 1, challenge)**:
- 🎯 **ComparIA** (Beta.gouv / Ministry of Culture) — 5 GB Parquet, Etalab 2.0, EcoLogits methodology built in
- 🎯 **RTE/NaTran/Teréga IRIS** (ODRÉ) — industrial electricity + gas consumption at IRIS level, Etalab 2.0
- Full catalog: `docs/sources/CATALOGUE-SOURCES.md`

---

## 3bis. Medallion architecture — STRICT RULE

All external data MUST go through the 3-layer pipeline defined in `ADR-0009`:

```
🟫 Copper  → raw immutable data, dated, hashed (original format)
🥈 Silver  → validated, normalized Parquet, versioned schema (lineage to Copper)
🥇 Gold    → referentiel.sqlite + analytics.parquet (consumed by the app)
```

**Non-negotiable rules**:
1. ❌ Never direct access from application code to external sources: only `sobria-ingest` talks to the APIs.
2. ❌ Never an *ad hoc* transformation outside the `DataLayer` trait.
3. ❌ Never Gold without Silver, never Silver without validated Copper.
4. ✅ Every new source = a single trait to implement (`ingest_copper`, `promote_silver`, `contribute_gold`).
5. ✅ Versioned Silver schemas (`schemas/silver/<entity>-v<n>.json`).
6. ✅ `proptest` tests + golden files on every transformation.
7. ✅ Single command: `cargo run -p sobria-ingest -- pipeline run`.
8. ✅ Orchestration via `dvc.yaml` — `dvc repro` must reproduce identically.

See `ADR-0009-medallion-architecture.md` for details (trait interface, examples, DVC stages).

---

## 4. Code conventions

### 4.1 Rust

- **Edition**: 2021
- **Linter**: `cargo clippy -- -D warnings`
- **Formatter**: `cargo fmt` (rustfmt config in `rustfmt.toml`)
- **Tests**: `cargo test` + coverage measured by `cargo-tarpaulin`
- **Crate naming**: `sobria-*` (snake_case but with the `sobria` prefix)
- **Organization**: Cargo workspace, one crate per responsibility (see CDC §7.1)
- **Errors**: `thiserror` for public errors, `anyhow` for binaries
- **Async**: `tokio` (multi-thread runtime), `async-trait` if needed
- **Serialization**: `serde` everywhere, stable formats (JSON, MessagePack, Parquet)
- **Documentation**: rustdoc (`///` and `//!`) on every public item, one short line per item, an example only when it is useful; other comments follow the rule of §13

### 4.2 TypeScript / SvelteKit

- **Strict mode**: `"strict": true` in tsconfig
- **Linter**: ESLint + `@typescript-eslint`, strict rules
- **Formatter**: Prettier (shared config at the root)
- **Svelte components**: one component = one folder if over 100 lines
- **Stores**: prefer typed Svelte 5 stores (runes) if we adopt the rune syntax
- **Tests**: Vitest + Playwright for e2e
- **i18n**: `svelte-i18n` or `@inlang/paraglide`, keys in kebab-case

### 4.3 Commit messages (Conventional Commits)

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`.
Scopes: `core`, `estimator`, `referentiel`, `ingest`, `geoloc`, `export`, `audit`, `app`, `ui`, `ext`, `docs`, `ci`.

Example: `feat(estimator): add Monte-Carlo uncertainty propagation`. Commit messages, PR titles and PR descriptions are in English (§13).

Footer: `Refs: SOB-123` links the commit to the Linear ticket; `Fixes SOB-123` in the PR description closes the ticket at merge (ADR-0018).

### 4.4 Branching

- `main`: protected, releases only
- `develop`: integration
- `feature/sob-123-<short-desc>`: features
- `fix/sob-123-<short-desc>`: fixes
- `docs/sob-123-<short-desc>`: docs only

`sob-123` is the identifier of the Linear ticket: Linear links the branch and the PR to the ticket (ADR-0018).

---

## 5. Definition of Done (DoD)

A task is **done** when ALL of the following are true:

- [ ] Code written + reviewed (self-review)
- [ ] Unit tests written, local coverage ≥ 80 %
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `npm run lint && npm run check` pass on the front end
- [ ] Documentation up to date (`cargo doc`, user docs if the feature is visible)
- [ ] Commit message follows Conventional Commits
- [ ] If the task affects an architecture decision → ADR updated
- [ ] If the task touches the referential → versioned schema
- [ ] If the task touches the UI → screenshot in the PR + a11y check (axe)

---

## 6. Scientific methodology (to follow)

See `docs/methodology/` for details.

- Every calculation formula is **sourced** in the code (comment with DOI/URL).
- Every assumption has a documented **uncertainty distribution**.
- Every result comes with a **P5-P95 interval**.
- Calculations are **reproducible** from a seed (`SOBRIA_SEED` env var, default 42).
- The engine passes the **3 cross-validation tests** (Luccioni 2023, Patterson 2021, EcoLogits 2024) within ±15 %.

---

## 7. Security and privacy

- **Privacy by design**: all processing is local by default, no external sending.
- **Opt-in required** for any telemetry or sharing.
- **Encryption**: `argon2` for hashes, `rustls` for TLS, no OpenSSL.
- **Secrets**: no secret in clear text, `.env` never committed, CI secrets through GitHub Encrypted Secrets.
- **Dependencies**: `cargo deny` + `cargo audit` on every CI run, blocked on a critical vulnerability.
- **Browser extension**: minimal permissions, no remote code, audit before store submission.

---

## 8. Frugality by default

The project embodies its subject. Every technical decision passes the filter:

> *"Is this choice the most frugal acceptable?"*

In practice:
- Prefer compiling at build time to doing it at runtime (Svelte vs React, Rust vs Node).
- Avoid dependencies ≥ 1 MB unless absolutely necessary.
- Measure and publish the CO₂eq footprint of Sobr.ia itself (meta-consistency).
- Size-optimized builds (`opt-level = "z"`, `lto = true`, `strip = true`, `panic = "abort"`).

---

## 9. Multi-platform

| Platform | Status | Priority |
|----------|--------|----------|
| Windows | 1st class | Blocking for v1.0 |
| macOS | 1st class | Blocking for v1.0 |
| Linux | 1st class | Blocking for v1.0 |
| Web (Wasm) | 2nd class | Blocking for v1.0 (demo) |
| Android | 3rd class | Bonus (Tauri 2 mobile) |
| iOS | 3rd class | Bonus (Tauri 2 mobile) |

---

## 10. Useful commands (to know)

```bash
# Bootstrap (from the cloned repo)
./scripts/bootstrap.sh             # installs all deps

# Dev
cargo run -p sobria-app             # runs Tauri in dev
cd web && npm run dev               # SvelteKit frontend with hot reload
cd extension && npm run dev         # extension in dev mode

# Tests
cargo test --workspace              # Rust tests
cargo tarpaulin --workspace         # coverage
cd web && npm run test              # Vitest tests
cd web && npm run e2e               # Playwright tests

# Lint
cargo clippy -- -D warnings
cargo fmt --check
cd web && npm run lint && npm run check

# Medallion pipeline (ADR-0009)
cargo run -p sobria-ingest -- pipeline run                 # the whole Copper→Silver→Gold pipeline
cargo run -p sobria-ingest -- pipeline run --incremental   # only re-ingests what changed
cargo run -p sobria-ingest -- pipeline run --source comparia    # single source: ComparIA
cargo run -p sobria-ingest -- pipeline run --source rte-iris    # single source: RTE IRIS
cargo run -p sobria-ingest -- copper --all                 # raw layer only
cargo run -p sobria-ingest -- silver --all                 # Copper→Silver promotion
cargo run -p sobria-ingest -- gold                         # builds the final Gold
cargo run -p sobria-ingest -- validate                     # checks integrity + lineage
dvc repro                                                  # replays the changed stages
dvc push                                                   # publishes the new referential to the remote

# Production build
./scripts/build-all.sh              # produces all binaries + web + ext

# Methodology
quarto render notebook/validation.qmd
```

---

## 11. Workflow and working rules

Source: the Linear document ["How we work"](https://linear.app/sobrcc/document/how-we-work-cb9e7156febb). Linear (workspace `sobrcc`, team `SOB`) drives progress (ADR-0018). All agents are Claude agents; Linear's AI features stay disabled.

### Who does what

| Actor | Does | Never does |
|-------|------|------------|
| Thibault | Sets priorities, approves the design, handles accounts, payments, stores and legal decisions. Every PR is assigned to him; the reviewer agents merge on their own "ready to merge" verdict | — |
| Orchestrator (main Claude Code session, skill `/orchestrate`) | Sorts Triage, writes tasks with the `linear-issue-writing` skill, splits whatever goes over 8 points, writes the weekly update of each project | Code, merge, close a task |
| Claude Design | Produces the screens of a Design task (no access to Linear: Thibault or Claude attaches the link to the task) | Invent a screen absent from the routes |
| Claude Code (in the repo, with the Linear MCP) | Takes a Todo task `agent:claude-code`, moves it to In Progress, creates a branch named after the identifier, codes, verifies, opens a linked PR, moves it to In Review with a summary | Merge its own PR, widen the scope (it creates a new issue in Triage instead), touch a `human-only` issue |

Claude Code's roles are described in `.claude/agents/` (one file per role). To decide, Thibault answers in a comment on a Decision task. To steer, he changes the priority or the Role label, or sends a task back to Backlog.

### Working rules

- One task = one PR, 8 points at most. Beyond that, split into sub-tasks before Todo.
- Each task cites the screens it concerns by their route identifier (E1, Z2, V3…) and links the two reference documents (routes and architecture).
- Branches, commits and PRs: §4.3 and §4.4 (ADR-0018); commits, PR titles and PR descriptions in English (§13). The GitHub integration links the PR to the task and advances the state.
- When moving a task to In Review, a three-part comment, signed with the role:

  ```
  Done: what changed, in one to three sentences.
  Verified: the command or steps run, and the result.
  Left: anything not done, follow-up issue links, or questions for Thibault.
  -- role:rust (model:sonnet)
  ```

- Any scope deviation becomes a new task in Triage, never a silent addition.
- There is no limit on the number of issues In Review at the same time (Thibault, October 31). Agents keep working while PRs wait for review.
- Merging (Thibault, October 30 and 31): a reviewer agent merges a PR (squash, English message) when its own verdict on the current head is "ready to merge", it has re-read all the comments, there is no conflict, no other reviewer has posted a newer "changes requested" on the same head, and every failing CI job also fails on main for the same cause. A newer "changes requested" from another reviewer on the same head blocks the merge. It does not wait for Thibault, and Thibault moving the issue to Done is not required. Never merge on "changes requested".
- The `human-only` label covers accounts, payments, stores, secrets and legal matters: no agent works on them.
- Any decision that changes an ADR goes through a Docs issue and a PR on the ADR. An ADR PR may be merged as Proposed by the reviewer; the step to Accepted needs Thibault's knowing acceptance. An in-place amendment of an Accepted ADR (a dated section, original text kept) follows the same rule: it is merged with its own status Proposed and takes effect when Thibault knowingly accepts it, unless it only records decisions that he already took in writing (an answered Decision issue), in which case it is in force at merge.
- Every Friday, an update on each active project: health (On track, At risk, Off track) and three lines (Progress, Risks, Next), under 120 words, facts only.
- Decision of October 30: no monolith; the projects live in separate repositories of the Sobr-cc GitHub organization; the split is set by SOB-65.

### Ready (before a task moves to Todo)

- A title that starts with a verb and describes the result, under 70 characters.
- Why, What, Acceptance criteria and How to verify are filled in.
- Estimate set, 8 at most.
- Type, Area, Agent, Role, Model labels, project and priority set.
- No open question. If one remains, the task stays in Backlog and a Decision or Research task is linked to it.

### Definition of Done (code tasks)

On top of the definition in §5 (clippy, tests, lint, changelog fragment):

- `tsc --noEmit` passes.
- `/simplify` and `/code-review` run on the diff, findings addressed.
- A real check: `/verify`, a Playwright test, or a `curl` call on the changed endpoint.

### Estimate and model

Points measure the size of the task, review included: 1 (under an hour), 2 (half a day), 3 (about a day), 5 (2 to 3 days), 8 (about a week, maximum). The `Model` label follows size and risk, not the prestige of the role: `model:haiku` for a mechanical 1-point task with no decision, `model:sonnet` for 2 to 5 points on a known path (default), `model:opus` for 8 points or high risk. The orchestrator starts the agent with that model; the agent's file only carries the role's default model. Fable is never the default: only if Opus has failed twice, with Thibault's agreement. The `Model` label moves up one level only after two blocked attempts: the agent notes it in a comment and the orchestrator changes the label.

### Session flow

At every new session:

1. Read `CLAUDE.md` (this file).
2. Identify the Linear issue (`SOB-123`). Read a `briefs/chantiers/Cxx-*.md` brief only when the issue or its project links one.
3. Read the ADRs relevant to the task.
4. For each task:
   a. Understand — re-read the relevant CDC § + ADR + research/bibliography if scientific.
   b. Quick plan — 3-7 sub-steps.
   c. Implement — a small commit per sub-step if possible.
   d. Test — add tests + make the linters pass.
   e. Document — code docs + user docs if visible.
   f. PR description: CDC section + ADR + the brief when one is linked + `Fixes SOB-123`.

---

## 12. People & roles

| Role | Person | Tools |
|------|--------|-------|
| Project lead, architect, methodology | Claude Cowork (assistant) | Docs, planning, validation |
| Implementation | Claude Code | All crates, frontend, extension |
| Product decisions, validation | Thibault | Final validation, UX choices |
| Scientific mentor | (Ecolab/ADEME) | Methodology review |
| User testers | (5 people) | Interviews S11 |

---

## 13. What you must never do

- ❌ NEVER implement a scientific calculation without a documented source.
- ❌ NEVER introduce a dependency that is not listed in the CDC or an ADR.
- ❌ NEVER commit a secret, an API key, a full dataset (DVC is for that).
- ❌ NEVER `force push` on `main` or `develop`.
- ❌ NEVER skip the tests to "save time".
- ❌ NEVER introduce silent user tracking.
- ❌ NEVER write in French (Linear, code comments, rustdoc, commits, PR titles and descriptions, ADRs, CHANGELOG, README): all documentation is in English only. Only the product texts visible to the user (interface, i18n) are in French and English (decision of October 30). The rule applies to what is written from now on: an existing French document is translated the next time an agent modifies it, and code comments are translated by the Deep cleanup task of their area. New ADRs are in English; an accepted ADR is amended in place, by PR, and the amendment is written in English without translating the original text (SOB-87). The CDC deliverables (NF-16, L5) stay in French and English until Thibault has listed the documents meant for users.
- ❌ NEVER comment what the code already says: comments at the strict minimum, one short line for a complex function or a non-obvious choice. Exceptions: the rustdoc of public items (§4.1), the sources required by §6 and §14 (formulas, assumptions, numeric values), `TODO(SOB-123)` (§14).
- ❌ NEVER resolve an ambiguity in the CDC without asking for confirmation.

---

## 14. What you must always do

- ✅ Cite the source of every formula, assumption, numeric value.
- ✅ Add `#[cfg(test)]` mod tests to every non-trivial Rust module.
- ✅ Prefix TODOs: `// TODO(SOB-123):` with an existing Linear ticket (ADR-0018).
- ✅ Measure the size of the binaries at every release build.
- ✅ Add a fragment in `changelog.d/` for every feature or fix (see `changelog.d/README.md`); CHANGELOG.md is assembled at release.
- ✅ Ask when an ambiguity appears in the CDC or the brief.

---

*This file evolves with the project. Any structural change implies an ADR.*
