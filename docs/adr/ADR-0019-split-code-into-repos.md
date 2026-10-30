# ADR-0019 — Split the code into several repos of the Sobr-cc organization

- **Status**: Proposed (becomes Accepted when Thibault answers the points below and approves the PR)
- **Date**: 2026-10-30
- **Deciders**: Thibault (decision); CTO recommendation
- **Technical context**: Foundations, Structure milestone (SOB-65)

> **Summary.** The code leaves the single repo along delivery units, not folders. `sobr` keeps what is compiled and shipped together (engine, desktop app, organization server, Svelte app, docs and ADRs). Five pieces get their own repo in three waves: `.github`, `site` and `design-system`, then `extension`, then `data`. No Rust code is shared across repos; every other interface is a versioned contract. Each wave is a stopping point. Four questions for Thibault come first.

## Points to decide

Four questions and one note for Thibault. The recommendations rest on the Decision section.

1. **The core stays in one repo (`sobr`).** Recommended: yes. The desktop app and the organization server both embed the same `web/` build (ADR-0021) and share the crates `core`, `audit` and `export`, so a change to the types between Rust and `web/` stays in one PR. This is the closest point to a monolith: `sobr` stays the biggest repo. Alternative: move `web/` into its own repo, at the price of a versioned artefact that the app and the server must fetch, and of DTOs kept in sync across two repos.
2. **No Rust code is shared across repos.** Recommended: yes. `data` owns the datasets it ships (datacenter records, territory files, RTE mix) in a `datasets/` folder, and copies the seed constant (default 42, CLAUDE.md §6); `sobr` consumes the Gold artefact, one way. Alternative: `data` keeps a `git` dependency on `sobr`, pinned by tag. A change to one datacenter record then takes four steps, each PR reviewed by Thibault: a `sobr` tag, a bump PR in `data`, a `data` release, a consume PR in `sobr`.
3. **All of `docs/` stays in `sobr` (a copy of `LICENSES-FONTS.md` goes with the fonts); the site rebuilds daily from `sobr@main`.** Recommended: yes. Docs can lag by up to a day and no cross-repo token is needed. Alternatives: a `repository_dispatch` from `sobr` at every docs change (a token secret in `sobr`), or docs per repo (one fetch per repo in the site).
4. **Claude Code components.** Recommended: the log import is a crate of `sobr` (it runs in the app and writes to its session store); the plugin goes in `claude-code`; the MCP server goes in `claude-code` if it is a separate process that talks to the HTTP API, and in `sobr` if it is a module of the server. The ADR of the "Claude Code and MCP" project confirms.

Note, no answer expected here: **delivery of data to the app** (model catalog, territory files, datacenter records). Today nothing links the Gold of `sobria-ingest` to the app (see Context). One ADR issue is planned in wave 3; it gates the "Model catalog" project, not the moves.

## Context and problem statement

On October 30, 2026, Thibault decided that the projects live in separate repos of the `Sobr-cc` GitHub organization, with no monolith (comment on SOB-33). Everything is in `github.com/Sobr-cc/sobr` today: 11 crates, `web/`, `web-team/`, `extension/`, `site/`, the design system, the data pipeline (DVC) and the documentation. The "Repos" decision of the routes document ("main repo sobr (Cargo and pnpm workspaces), site, .github") places neither the extension, nor the data, nor the design system, and does not say how they share code. It also speaks of pnpm workspaces, while every JavaScript folder uses npm with its own `package-lock.json`.

Findings on `main` on 2026-10-30:

1. **A block built and shipped together.** `sobria-app` embeds `web/build` (`crates/sobria-app/tauri.conf.json`, `frontendDist`). `sobria-team-aggregator` embeds `web-team/build` with `rust-embed` (`src/server/embedded_web.rs`), and ADR-0021 replaces `web-team/` by `web/`. The app and the server share `core`, `audit` and `export`. The TypeScript types of `web/` copy the Rust DTOs by hand (`web/src/lib/api.ts`).
2. **Weak links elsewhere.** `extension/` has no code dependency: the bridge protocol and the API client are copied, the logos are copied by a manual script. `site/` depends only on `docs/` (`scripts/sync-docs.mjs`). The design system is used only by the logo script of the extension, and holds no font. `sobria-ingest` uses only `DEFAULT_SEED` from `sobria-core` and the datacenter records from `sobria-geoloc`; its schemas are read at compile time (`include_str!`).
3. **Drift already present.** The CSS tokens of `web/` are copied into three other folders, the 8 fonts exist in 3 copies, the old URL `BkOff-fr/defis-lia-generatif` is hard-coded in 17 files, one `vX.Y.Z` tag triggers 3 release workflows, two nightly DVC workflows replay the same pipeline (`ingest-nightly` refers to a remote `origin` missing from `.dvc/config`), and `ci.yml` has no path filter.
4. **A data → app link that does not exist yet.** The app creates `referentiel.sqlite` in the user's data directory and writes its own tables in it (preferences, goals, projects). No release workflow supplies the Gold produced by `sobria-ingest`. The Gold is also not reproducible: its datasheet embeds the build time, so its hashes change on every run. The `data` → `sobr` contract is to define, not to preserve.
5. **A governance constraint.** Every merge needs Thibault's approval ("How we work"). A change that touches N repos costs N reviews.
6. **Decisions already taken.** The site goes to `Sobr-cc/site` (SOB-69), the shared files to `Sobr-cc/.github` (SOB-70), the infra repo is named by the SOB-73 ADR, `web-team` is removed after the merge (ADR-0021, SOB-64), `sobria-import` is removed (SOB-67), and no issue of another project touches a folder before its Deep cleanup (Foundations project).

Question: which repos, with what content, which code sharing, in which order?

## Splitting criterion

Two rules, which also place future components:

1. What is compiled into the same binary, or embedded in it, and changes together, stays in the same repo.
2. What has its own delivery channel (extension store, static host, data pipeline), or is a resource shared by several repos, and talks to the rest only through a versioned contract (bridge protocol, HTTP API, SQLite file, design tokens), gets its own repo.

## Options considered

| Option | Principle | Cost | Risk | Reversibility |
|--------|-----------|------|------|---------------|
| A | Status quo: one repo | none | contradicts the October 30 decision | — (ruled out) |
| B | One repo per folder or crate (about fifteen) | high | high | hard |
| C | One repo per delivery unit, in 3 waves | about 54 points (§6) | medium, concentrated on `data` | good: each wave is a stopping point |
| D | Only `site` and `.github` (current text of the routes document) | about 8 points | low | excellent |

- **B**: splitting `web/` from the crates turns each DTO change into two PRs plus a version change, and the crossed versions among `core`, `estimator`, `audit` and `export` into a compatibility matrix. Merging repos back later is possible but long.
- **D**: cheap and reversible, but leaves in the main repo components with different delivery rhythms (extension, data, design system). It is part of wave 1 of option C.
- **C**: cuts where coupling is already weak or copied by hand, and keeps together what ships together. The gain is real for the extension (independent store review), the data (nightly or weekly rhythm) and the site (static deployment).

## Decision

**Option C is proposed.**

### 1. Target repos

Names follow those already set (`sobr`, `site`, `.github`). No repo is created before it has content.

| Repo | Content | Owner role | Creation |
|------|---------|------------|----------|
| `sobr` (exists) | crates of the engine, the desktop app and the server; the Svelte app (`web/`); ADR register and `docs/`; test and research tooling (details in §2) | Lead Rust (`web/`: Svelte front end) | exists |
| `.github` | organization profile, community files, issue and PR templates, reusable CI workflows | DevOps | wave 1 |
| `site` | Astro site (S1 to S4, research) | Svelte front end | wave 1 |
| `design-system` | design system, CSS tokens, one copy of the fonts and a copy of their licenses, logos, a `package.json`, tagged versions | Product designer | wave 1 |
| `extension` | browser extension (Chrome, Firefox and Edge, per the October 30 decision of the routes document) | Dev extensions | wave 2 |
| `data` | `sobria-ingest`, schemas, DVC pipeline, nightly workflow, model catalog, research dataset, and the datasets it ships (datacenter records, territory files, RTE mix) | Data and method | wave 3 |
| `infra` | reserved: name and content set by SOB-73 (creation: SOB-74) | Infra | Infrastructure project |
| `vscode` | VS Code extension | Dev extensions | when the "VS Code extension" project opens |
| `claude-code` | Claude Code plugin (hooks, status line) and, per point 4, the MCP server | Dev integrations | when the "Claude Code and MCP" project opens |

### 2. Destination of each current folder

| Today | Destination | Remark |
|-------|-------------|--------|
| `crates/sobria-core`, `sobria-referentiel`, `sobria-estimator`, `sobria-audit`, `sobria-export`, `sobria-bridge`, `sobria-app`, `sobria-team-aggregator` | `sobr` | |
| `crates/sobria-geoloc` | `sobr` | keeps its embedded copy of the datasets (datacenter records, territory files, RTE mix); their source of truth moves to `data` (wave 3), and the delivery ADR says how updates arrive |
| `crates/sobria-ingest` | `data` | wave 3, with the layout of §4 |
| `crates/sobria-import` | none | removed by SOB-67; the Claude Code log import follows point 4 |
| `web/` | `sobr` | |
| `web-team/` | none | merged into `web/` then removed (ADR-0021, SOB-64); never moves |
| `extension/` | `extension` | |
| `site/` | `site` | |
| `sobr-ia-design-system/` | `design-system` | the folder takes the repo name; it holds no font and no `package.json` today, so the move issue adds one font copy from `web/static/fonts`, `favicon.svg` and `apple-touch-icon.svg` from `web/static`, and a `package.json` |
| `schemas/` | `data` | read at compile time by `sobria-ingest` |
| `dvc.yaml`, `.dvc/`, `.dvcignore` | `data` | |
| `tools/` (`fixturegen`) | `sobr` | generates the fixtures of `web/` from `core` and `estimator` (ADR-0017) |
| `deploy/` | `sobr` | shipped with the binaries; Sobr's own hosting belongs to SOB-73 |
| `scripts/bootstrap.sh`, `scripts/build-all.sh` | `sobr` | the steps of moved folders leave with them (§4) |
| `scripts/sync-docs.mjs` | `site` | replaced by a fetch of `docs/` (§3) |
| `docs/LICENSES-FONTS.md` | `design-system` | copied in wave 1; the copy in `sobr` stays until the last font copy of `web/` and `extension/` is gone, because `web/static/fonts/README.md`, `extension/vite.config.ts` and `extension/src/styles/tokens.css` link to it |
| `docs/` (the rest, with `docs/adr/`) | `sobr` | one tree that the site fetches; single ADR register, since a decision concerns the organization, not one repo |
| `briefs/`, `research/`, `notebook/` | `sobr` | the notebook validates the estimator |
| `.claude/` (agents, skills `orchestrate` and `linear-issue-writing`) | `sobr` | source of the working rules; the other repos get the part that concerns them (§4) |
| `.github/workflows/ci.yml`, `app-release.yml`, `team-aggregator-release.yml`, `team-docker.yml`, `zenodo.yml` | `sobr` | the `ci.yml` jobs of the moved folders go with them |
| `.github/workflows/extension-release.yml` | `extension` | |
| `.github/workflows/site-deploy.yml` | `site` | |
| `.github/workflows/dvc-nightly.yml`, `ingest-nightly.yml` | `data` | merged into one |
| `CODE_OF_CONDUCT.md`, `SECURITY.md`, `CONTRIBUTING.md` | `.github` | SOB-70; a repo keeps its own file if it needs a different one |
| `LICENSE-DATA` | `data` | |
| `LICENSE`, `LICENSE-DOCS`, `.zenodo.json`, `Cargo.toml`, `Cargo.lock`, `deny.toml`, `rust-toolchain.toml`, `rustfmt.toml`, `CLAUDE.md`, `CHANGELOG.md`, `README.md`, `.env.example`, `.gitattributes`, `.gitignore` | `sobr` | every other repo gets its own version; `LICENSE` is copied everywhere (ADR-0006) |

### 3. Code sharing and contracts

No Rust code is shared across repos (point 2). Front-end assets and interfaces go through versioned contracts. There is no package registry for now: a crates.io or npm account is a human action and commits to the stability of a public API. Moving to a registry later changes only the dependency line.

| Shared surface | Producer → consumers | Mechanism | Versioning |
|----------------|----------------------|-----------|------------|
| CSS tokens, fonts, logos | `design-system` → `web/`, `extension`, `site`, `vscode` | npm dependency `github:Sobr-cc/design-system#vX.Y.Z`, ready-to-use files in the repo; replaces the copies | SemVer; major = a token removed or renamed |
| Bridge protocol (ADR-0013) | `sobr` → `extension` | JSON Schema generated from the Rust types, published with each release; contract test in the extension | protocol version in the messages |
| HTTP API of the server | `sobr` → `extension`, `vscode`, `claude-code`, the app | OpenAPI published with each release | `/api/v1` prefix; a breaking change opens `/api/v2` in parallel |
| Reference data (ADR-0007, ADR-0009) | `data` → `sobr` | artefact published as a `data` release (CalVer tag, `MANIFEST.sha256`); how the app receives it: the note in Points to decide | schema version in the file + CalVer |
| `docs/` | `sobr` → `site` | the site workflow clones `docs/` of `sobr@main` at build time; it runs on a push to `site`, daily, and by hand | none: always `main` |
| Common CI jobs | `.github` → all | reusable workflows | tag |

### 4. History, open PRs, CI, DVC, releases

- **History**: each folder leaves with its history (`git filter-repo` on a fresh clone). Authors, dates and `Refs: SOB-n` are kept; GitHub PR and issue numbers stay in `sobr` and their links stay valid.
- **Open PRs**: a folder does not move while a PR touches it: the PR is merged, closed, or replayed in the new repo.
- **CI**: each repo starts with a copy of the `ci.yml` jobs it needs, and switches to the reusable workflows when they exist. `sobr` drops the moved jobs in the same PR that removes the folder, so CI stays green.
- **Scripts**: `scripts/build-all.sh` (the release command of CLAUDE.md §10) builds the extension, and `scripts/bootstrap.sh` installs DVC, runs `sobria-ingest fetch` and runs `npm ci` in `extension/`. Each move PR removes its steps, with their help lines and messages, so both scripts keep working after each wave.
- **Releases**: `sobr` keeps the `vX.Y.Z` tags, because the app and the server embed one `web/` build (ADR-0021); `extension` has its own tags. Each README holds a compatibility matrix (protocol version and `/api/v1`, §3).
- **`data` and DVC**: `data` keeps the current relative layout: `crates/<ingest crate>/`, `schemas/`, `dvc.yaml` and `.dvc/` at the root, because `include_str!("../../../schemas/...")` and the `dvc.yaml` dependencies use these paths. The datasets it ships live in a `datasets/` folder that the `gold` stage lists in its dependencies. A root `Cargo.toml` workspace holds the ingest crate only. The `copper` stage of `dvc.yaml` depends today on `crates/sobria-core/src`; that dependency is dropped, because ingest no longer depends on `sobria-core` (point 2). The `dvc.lock` is committed in the move PR, and the single command of CLAUDE.md §3bis (`pipeline run`) runs in `data`. Success test of the move: `dvc repro` completes in `data` and `validate` passes. A "same `MANIFEST.sha256`" test cannot pass yet (Context 4); making the Gold reproducible is a separate fix in `sobria-ingest`.
- **Human actions per repo**: creation, protection of `main` (SOB-17), deploy or remote secrets, coverage of Linear's GitHub integration (SOB-46).
- **Agents' working rules**: they stay in `sobr` (`.claude/`, SOB-43, SOB-44). Each new repo gets a short `CLAUDE.md` (scope, commands, pointer to the common rules). The conventions of ADR-0018 are unchanged.

### 5. Order of the moves

**Conditions common to every wave**: this ADR approved; CI green and `main` protected (SOB-8, SOB-17); GitHub links fixed (SOB-40, so that each folder leaves with the right URLs); the Deep cleanup of the folder done if it has one (Foundations rule).

1. **Wave 1: `.github`, `site`, `design-system`.** Outside the Deep cleanup, no code coupling; the only dependency is `docs/` for the site. Starts as soon as the common conditions are met and SOB-66 is done. It validates the procedure at the lowest risk. Its move issue also adds the fonts, icons, `package.json` and a copy of `LICENSES-FONTS.md` (§2), applies the `sobria-design` to `sobr-design` rename of ADR-0020 before the first tag, and repoints `extension/scripts/sync-logos.js` (a manual script; the extension build is unaffected). The extension move deletes that script in wave 2: its icons are committed and the tokens replace it.
2. **Wave 2: `extension`.** After its Deep cleanup (SOB-31) and after the contracts are published (§3), otherwise the hand-copied protocol drifts between two repos.
3. **Wave 3: `data`.** After the Deep cleanup of ingest (SOB-21) and the rename of the crates (SOB-37), so that `data` is born with the final names. It is the riskiest wave (DVC, secrets, nightly workflow). Waves 2 and 3 do not depend on each other: the riskier one goes last.

Token adoption follows the move of each consumer; `web/` waits for the end of its Deep cleanup (SOB-27 to SOB-29). The work inside `sobr` (single app ADR-0021, rename ADR-0020, removal of `import` SOB-67) does not depend on the waves, and issues that live in `sobr` (the contracts, making ingest independent, the delivery ADR) may start before their wave: only the moves wait for their gates. Each wave is a stopping point: at its end, `sobr` builds and its CI is green.

**Rename (ADR-0020).** The repo names are set here; the names of crates, folders and packages belong to ADR-0020, applied repo by repo. Zenodo and DOI are per repo: `sobr` (software) and `data` (dataset) each have their citation metadata. The `sobria-*` names in this ADR are those of 2026-10-30.

**Single app (ADR-0021).** Rule 1 holds with one or two apps; ADR-0021 relies on it.

### 6. Issues to create after approval

Titles in English, to be created in Linear by the orchestrator (SOB-66, SOB-69 and SOB-70 already exist), each of 8 points at most. Points are indicative; the official estimate is made at creation. Total: 54 points, not counting the `vscode` and `claude-code` repos.

| When | Issue | Role | Points |
|------|-------|------|--------|
| Before the waves | Share the agent working rules across the Sobr-cc repos (after SOB-43 and SOB-44) | CTO | 3 |
| Wave 1 | SOB-66 Create the site and .github repos in Sobr-cc (existing; add `design-system`) | DevOps | 1 |
| Wave 1 | SOB-70 Set up the Sobr-cc/.github repo (existing) | DevOps | 2 |
| Wave 1 | SOB-69 Move the site to the Sobr-cc/site repo (existing; add the daily docs fetch; re-estimated) | DevOps | 5 |
| Wave 1 | Move the design system to Sobr-cc/design-system (fonts, icons, package.json, LICENSES-FONTS.md copy, ADR-0020 rename), repoint sync-logos, and tag its tokens | DevOps | 5 |
| Wave 1 | Add reusable CI workflows to Sobr-cc/.github (before wave 2) | DevOps | 3 |
| Wave 1 | Use the design tokens by tag in the site | Svelte front end | 2 |
| Wave 2 | Create the extension repo in Sobr-cc (human-only) | DevOps | 1 |
| Wave 2 | Publish the bridge JSON Schema and the server OpenAPI with each release | Lead Rust | 5 |
| Wave 2 | Move the extension to Sobr-cc/extension; remove its steps from build-all.sh and bootstrap.sh and delete sync-logos | DevOps | 5 |
| Wave 2 | Add contract tests against the bridge schema and the API in the extension | Dev extensions | 3 |
| Wave 2 | Use the design tokens by tag in the extension | Dev extensions | 2 |
| Wave 3 | Create the data repo in Sobr-cc (human-only) | DevOps | 1 |
| Wave 3 | Make sobria-ingest independent of sobria-core and sobria-geoloc (copy the seed, read the datasets from a datasets/ folder) | Lead Rust | 3 |
| Wave 3 | Move sobria-ingest, the schemas and the DVC pipeline to Sobr-cc/data, with the layout of §4, a committed dvc.lock, one nightly workflow, and no ingest or DVC step, help line or message left in the scripts | DevOps | 8 |
| Wave 3 | Write the ADR for delivering data updates (catalog, territory files, datacenter records) to the app | CTO | 3 |
| After the Deep cleanup of `web/` | Use the design tokens by tag in web | Svelte front end | 2 |
| Later | Create the vscode repo; create the claude-code repo (human-only, when their project opens) | DevOps | not counted |

Other existing issues to adjust: SOB-38 (split per repo if the extension has already moved), SOB-39 (closed into the design-system move), SOB-6 and SOB-71 (the catalog arrives in `data`).

### 7. Text that replaces the "Repos" decision of the routes document

> **Repos** — GitHub organization Sobr-cc, one repo per delivery unit: `sobr` (engine, desktop app, organization server, Svelte app, docs and ADRs), `extension`, `site`, `design-system`, `data`, `.github` and an infra repo (name set by SOB-73); `vscode` and `claude-code` when their projects open. See ADR-0019.

## Consequences

**Positive**:
- Independent delivery: the extension ships without rebuilding the app, the site deploys without touching Rust, the data pipeline runs at its own rhythm.
- CI per repo, with natural filters: the end of the single `ci.yml` that rebuilds everything on each push (CLAUDE.md §8, frugality).
- A clear owner per repo, by role.
- Contracts (bridge schema, OpenAPI, versioned tokens) replace hand copies that are untested today.
- No code dependency between repos: the only ones left are versioned contracts and a one-way data artefact.
- Reversible by waves (§5).

**Negative**:
- The work listed in §6, with no visible feature.
- Contracts to maintain, and a compatibility matrix between `sobr`, `extension` and `data`.
- Duplicated configuration (`deny.toml`, `rustfmt.toml`, lint): reduced by the reusable workflows of `.github`.
- Human actions at each repo (§4).
- `sobr` stays big (point 1).
- The datasets exist twice for a while (source in `data`, embedded copy in `sobr`) until the delivery ADR.

**Neutral**:
- One ADR register in `sobr`; numbering stays continuous.

## Links

- Linear: SOB-65, SOB-33 (the "no monolith" decision), SOB-66, SOB-69, SOB-70, SOB-73, SOB-74
- ADR-0006 (licenses), ADR-0007 (DVC), ADR-0009 (medallion), ADR-0013 (bridge), ADR-0017 (web demo), ADR-0018 (Linear conventions), ADR-0020 (rename), ADR-0021 (single app)
- CLAUDE.md §3bis, §6, §8, §10
- [Routes, screen by screen](https://claude.ai/artifact/Cua7SK84VzMCwYFZRQfUHD) (decision "Repos", replaced by §7; decision "Platforms" for Edge)
- Linear, "How we work"
