# ADR-0019 — Split the code into several repos of the Sobr-cc organization

- **Status**: Accepted (2026-10-31)
- **Amendment 1** (2026-10-31, Accepted): `web/` and site links, parity cases, `LICENSE-DATA` and counts. See the section below.
- **Date**: 2026-10-30
- **Deciders**: Thibault (answers of 2026-10-30, recorded in SOB-113, and of 2026-10-31, recorded in SOB-139); CTO recommendation
- **Technical context**: Foundations, Structure milestone (SOB-65)

> **Summary.** The code leaves the single repo along delivery units, not folders. `sobr` keeps what is compiled and shipped together (engine, desktop app, organization server, Svelte app, ADRs and technical documentation). Five pieces get their own repo in three waves: `.github`, `site` and `design-system`, then `extension`, then `data`. No Rust code is shared across repos; every other interface is a versioned contract. The public documentation and the community files go to `.github`. Each wave is a stopping point. Thibault's four answers come first, with answer 1 confirmed on October 31 (SOB-139).

## Decisions of Thibault

Thibault answered the four points of the proposal on October 30, 2026, in his session with the orchestrator; the orchestrator recorded the answers in SOB-113. He confirmed his reading of answer 1 on October 31 (SOB-139).

1. **The core stays in one repo (`sobr`).** Option 1a. The app and the panel (`web/`, the organization server and the engine) stay in `sobr`; the showcase site has its own repo, `site`. **Confirmed in SOB-139 (October 31):** Thibault chose the plan as drafted (option C), so `design-system`, `extension` and `data` also get their own repos, in waves 1 to 3, and the panel and the app stay in `sobr`. His words of October 30 were « Pour le site vitrine pas dans le même rpo mais pour ce qui est du panel si » (site apart, panel in the same repo); the word "only" that SOB-113 added to the record was not part of his answer. The desktop app and the organization server embed the same `web/` build (ADR-0021) and share the crates `core`, `audit` and `export`, so a change to the types between Rust and `web/` stays in one PR. `sobr` stays the biggest repo, the closest point to a monolith. Not retained: moving `web/` into its own repo, at the price of a versioned artefact that the app and the server must fetch, and of DTOs kept in sync across two repos.
2. **No Rust code is shared across repos.** Option 2a. `data` owns the datasets it ships (datacenter records, territory files, RTE mix) in a `datasets/` folder, and copies the seed constant (default 42, CLAUDE.md §6); `sobr` consumes the Gold artefact, one way. Not retained: a `git` dependency of `data` on `sobr`, pinned by tag, where a change to one datacenter record takes four steps, each PR reviewed by Thibault.
3. **Documentation.** None of the three options proposed (all of `docs/` in `sobr`, a trigger at every docs change, docs per repo). The public documentation (user documentation and the methodology pages shown on the site) and the community files of the organization go to the `.github` repo. ADRs and technical documentation stay in `sobr`. The site gets the public documentation from `.github`. The exact list of pages is set in the `.github` issue (§6), by the criterion of §3.
4. **Claude Code components.** Option 4a. The log import is a crate of `sobr` (it runs in the app and writes to its session store); the plugin goes in `claude-code`; the MCP server goes in `claude-code` if it is a separate process that talks to the HTTP API, and in `sobr` if it is a module of the server. The ADR of the "Claude Code and MCP" project confirms.

Note, no answer needed: **delivery of data to the app** (model catalog, territory files, datacenter records). Today the Gold reaches the app only through an implicit file path and two table names (see Context). One ADR issue is planned in wave 3; it gates the "Model catalog" project, not the moves.

## Context and problem statement

On October 30, 2026, Thibault decided that the projects live in separate repos of the `Sobr-cc` GitHub organization, with no monolith (comment on SOB-33). Everything is in `github.com/Sobr-cc/sobr` today: 10 crates (11 until `sobria-import` was removed, SOB-67), `web/`, `web-team/`, `extension/`, `site/`, the design system, the data pipeline (DVC) and the documentation. The "Repos" decision of the routes document ("main repo sobr (Cargo and pnpm workspaces), site, .github") places neither the extension, nor the data, nor the design system, and does not say how they share code. It also speaks of pnpm workspaces, while every JavaScript folder uses npm with its own `package-lock.json`.

Findings on `main` on 2026-10-30:

1. **A block built and shipped together.** `sobria-app` embeds `web/build` (`crates/sobria-app/tauri.conf.json`, `frontendDist`). `sobria-team-aggregator` embeds `web-team/build` with `rust-embed` (`src/server/embedded_web.rs`), and ADR-0021 replaces `web-team/` by `web/`. The app and the server share `core`, `audit` and `export`. The TypeScript types of `web/` copy the Rust DTOs by hand (`web/src/lib/api.ts`).
2. **Weak links elsewhere.** `extension/` has no build dependency on the other folders: the bridge protocol and the API client are copied, the logos are copied by a manual script, and it carries hand ports of the estimator (`ecologits.ts`, `afnor.ts`, `equivalents.ts`) whose parity test reads values copied by hand from the Rust cases. `site/` depends only on `docs/` (`scripts/sync-docs.mjs`). The design system is used only by the logo script of the extension, and holds no font. `sobria-ingest` uses only `DEFAULT_SEED` from `sobria-core` and the datacenter records from `sobria-geoloc`; its schemas are read at compile time (`include_str!`).
3. **Drift already present.** The CSS tokens of `web/` are copied into three other folders, the 8 fonts exist in 3 copies, the old URL `BkOff-fr/defis-lia-generatif` is hard-coded in 17 files, one `vX.Y.Z` tag triggers 3 release workflows, two nightly DVC workflows replay the same pipeline (`ingest-nightly` refers to a remote `origin` missing from `.dvc/config`), and `ci.yml` has no path filter.
4. **A data → app link that is only implicit.** The app creates `referentiel.sqlite` in the user's data directory and writes its own tables in it (preferences, goals, projects). No release workflow supplies the Gold produced by `sobria-ingest`, but `sobria-referentiel` already reads two Gold tables by name when the file is present: `sources` and `model_overview`. Only the ingest crate creates them, and `schemas/gold/` covers the datasheet, not the tables. The Gold is also not reproducible: its datasheet embeds the build time, so its hashes change on every run. The `data` → `sobr` contract is to formalize, starting from these two tables.
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
| C | One repo per delivery unit, in 3 waves | about 76 points (§6) | medium, concentrated on `data` | good: each wave is a stopping point |
| D | Only `site` and `.github` (current text of the routes document) | about 8 points | low | excellent |

- **B**: splitting `web/` from the crates turns each DTO change into two PRs plus a version change, and the crossed versions among `core`, `estimator`, `audit` and `export` into a compatibility matrix. Merging repos back later is possible but long.
- **D**: cheap and reversible, but leaves in the main repo components with different delivery rhythms (extension, data, design system). It is part of wave 1 of option C.
- **C**: cuts where coupling is already weak or copied by hand, and keeps together what ships together. The gain is real for the extension (independent store review), the data (nightly or weekly rhythm) and the site (static deployment).

## Decision

**Option C is retained.**

### 1. Target repos

Names follow those already set (`sobr`, `site`, `.github`). No repo is created before it has content.

| Repo | Content | Owner role | Creation |
|------|---------|------------|----------|
| `sobr` (exists) | crates of the engine, the desktop app and the server; the Svelte app (`web/`); ADR register and technical documentation; test and research tooling (details in §2) | Lead Rust (`web/`: Svelte front end) | exists |
| `.github` | organization profile, community files, issue and PR templates, reusable CI workflows, and the public documentation (§3) | DevOps | wave 1 |
| `site` | Astro site (S1 to S4, research); it fetches the public documentation from `.github` | Svelte front end | wave 1 |
| `design-system` | design system, CSS tokens, one copy of the fonts and a copy of their licenses, logos, a `package.json`, tagged versions | Product designer | wave 1 |
| `extension` | browser extension (Chrome, Firefox and Edge, per the October 30 decision of the routes document) | Dev extensions | wave 2 |
| `data` | `sobria-ingest`, schemas, DVC pipeline, nightly workflow, model catalog, research dataset, and the datasets it ships (datacenter records, territory files, RTE mix) | Data and method | wave 3 |
| `infra` | reserved: name and content set by SOB-73 (creation: SOB-74) | Infra | Infrastructure project |
| `vscode` | VS Code extension | Dev extensions | when the "VS Code extension" project opens |
| `claude-code` | Claude Code plugin (hooks, status line) and, per decision 4, the MCP server | Dev integrations | when the "Claude Code and MCP" project opens |

### 2. Destination of each current folder

| Today | Destination | Remark |
|-------|-------------|--------|
| `crates/sobria-core`, `sobria-referentiel`, `sobria-estimator`, `sobria-audit`, `sobria-export`, `sobria-bridge`, `sobria-app`, `sobria-team-aggregator` | `sobr` | |
| `crates/sobria-geoloc` | `sobr` | keeps its embedded copy of the datasets (datacenter records, territory files, RTE mix); their source of truth moves to `data` (wave 3), and the delivery ADR says how updates arrive |
| `crates/sobria-ingest` | `data` | wave 3, with the layout of §4 |
| `crates/sobria-import` | none | removed by SOB-67; the Claude Code log import follows decision 4 |
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
| `scripts/sync-docs.mjs` | `site` | replaced by a fetch of the public documentation from `.github` and of the ADR pages and the pages cited by code or by `web/` from `sobr` (§3) |
| `docs/LICENSES-FONTS.md` | `design-system` | copied in wave 1; the copy in `sobr` stays until the last font copy of `web/` and `extension/` is gone, because `web/static/fonts/README.md`, `extension/vite.config.ts` and `extension/src/styles/tokens.css` link to it |
| `docs/` public documentation (§3) | `.github` | copied with its history into a `docs/` folder of `.github` in wave 1; the site move PR syncs the listed pages again from `sobr`, then removes them, so the build never breaks and no edit is lost; the list follows the criterion of §3 and is approved by Thibault in the `.github` docs issue (§6); pages cited by path in the code, tests, golden files or links of a folder that still waits for its Deep cleanup (a crate, or `web/`) stay in `sobr` until the second docs step (§5) |
| `docs/adr/` | `sobr` | single ADR register, since a decision concerns the organization, not one repo |
| `docs/` (the rest: technical documentation, project documents, archive) | `sobr` | |
| `briefs/`, `research/`, `notebook/` | `sobr` | the notebook validates the estimator |
| `.claude/` (agents, skills `orchestrate` and `linear-issue-writing`) | `sobr` | source of the working rules; the other repos get the part that concerns them (§4) |
| `.github/workflows/ci.yml`, `app-release.yml`, `team-aggregator-release.yml`, `team-docker.yml`, `zenodo.yml` | `sobr` | the `ci.yml` jobs of the moved folders go with them |
| `.github/workflows/extension-release.yml` | `extension` | |
| `.github/workflows/site-deploy.yml` | `site` | |
| `.github/workflows/dvc-nightly.yml`, `ingest-nightly.yml` | `data` | merged into one |
| `CODE_OF_CONDUCT.md`, `SECURITY.md` | `.github` | SOB-70; a repo keeps its own file if it needs a different one |
| `CONTRIBUTING.md` | `sobr` | its quality gates are the cargo ones; `.github` gets a short generic one (SOB-70) that the other repos inherit, and a repo with its own gates adds its own |
| `LICENSE-DATA` | `data`, with a copy in `sobr` | `sobria-geoloc` embeds Etalab data in the binaries of `sobr`; in both copies the catalog path becomes the URL of the page where it lives after the docs copy (Amendment 1) |
| `LICENSE`, `LICENSE-DOCS`, `.zenodo.json`, `Cargo.toml`, `Cargo.lock`, `deny.toml`, `rust-toolchain.toml`, `rustfmt.toml`, `CLAUDE.md`, `CHANGELOG.md`, `README.md`, `.env.example`, `.gitattributes`, `.gitignore` | `sobr` | every other repo gets its own version; `LICENSE` is copied everywhere (ADR-0006), and `LICENSE-DOCS` also goes to `.github`, which now holds docs |

### 3. Code sharing and contracts

No Rust code is shared across repos (decision 2). Front-end assets and interfaces go through versioned contracts. There is no package registry for now: a crates.io or npm account is a human action and commits to the stability of a public API. Moving to a registry later changes only the dependency line.

| Shared surface | Producer → consumers | Mechanism | Versioning |
|----------------|----------------------|-----------|------------|
| CSS tokens, fonts, logos | `design-system` → `web/`, `extension`, `site`, `vscode` | npm dependency `github:Sobr-cc/design-system#vX.Y.Z`, ready-to-use files in the repo; replaces the copies | SemVer; major = a token removed or renamed |
| Bridge protocol (ADR-0013) | `sobr` → `extension` | JSON Schema generated from the Rust types, published with each release; contract test in the extension | protocol version in the messages |
| HTTP API of the server | `sobr` → `extension`, `vscode`, `claude-code`, the app | OpenAPI published with each release | `/api/v1` prefix; a breaking change opens `/api/v2` in parallel |
| Estimator parity | `sobr` → `extension` | reference cases exported as JSON from `REPRODUCTION_CASES` (`crates/sobria-estimator/src/validation/cases.rs`: inputs, Python targets, tolerances) by a second output of `tools/fixturegen`, which reads the public cases and leaves `sobria-estimator` unchanged, and published with each release; the extension test runs the EcoLogits cases (usage only, no modalities, zero overhead: three cases) with the tolerance of each case (1 %) instead of a fixed 2 % constant; today the test copies the same three cases by hand (CLAUDE.md §6); the five `c34-*` AFNOR cases stay Rust-only | version of the release |
| Reference data (ADR-0007, ADR-0009) | `data` → `sobr` | artefact published as a `data` release (CalVer tag, `MANIFEST.sha256`); `data` adds a versioned schema of the Gold tables that `sobr` reads (Context 4), next to the datasheet schema, and `sobr` has a contract test against it; how the app receives the file: the note in Decisions | schema version of the Gold tables + CalVer |
| Public documentation | `.github` → `site` | the site workflow clones the public documentation of `.github@main` at build time; it runs on a push to `site`, daily, and by hand | none: always `main` |
| ADR pages | `sobr` → `site` | same mechanism, from `docs/adr/` of `sobr@main`; the site publishes them today and decision 3 does not change that | none: always `main` |
| Pages cited by path in `sobr` code or in `web/` links | `sobr` → `site` | same mechanism, until the second docs step moves them to `.github` (§5) | none: always `main` |
| Common CI jobs | `.github` → all | reusable workflows | tag |

Public documentation is written for users, operators and readers of the method (user and operator guides, methodology pages). Technical documentation is written for contributors (ADRs, component architecture, internal operations, QA, planning, archive). The docs pages the site links today (the methodology, the operator guides, the sources catalogs) are the candidates for the first list; a page that fits neither is settled in that list, with Thibault's approval. A page that user-facing output of `sobr` cites by path (a result `source`, an error message), or that `web/` links to, is classified by this criterion; if it moves, the cited path is rewritten in the second docs step (§5).

### 4. History, open PRs, CI, DVC, releases

- **History**: each folder leaves with its history (`git filter-repo` on a fresh clone). Authors, dates and `Refs: SOB-n` are kept; GitHub PR and issue numbers stay in `sobr` and their links stay valid.
- **Open PRs**: a folder does not move while a PR touches it: the PR is merged, closed, or replayed in the new repo.
- **CI**: each repo starts with a copy of the `ci.yml` jobs it needs, and switches to the reusable workflows when they exist. `sobr` drops the moved jobs in the same PR that removes the folder, so CI stays green.
- **Scripts**: `scripts/build-all.sh` (the release command of CLAUDE.md §10) builds the extension, and `scripts/bootstrap.sh` installs DVC, runs `sobria-ingest fetch` and runs `npm ci` in `extension/`. Each move PR removes its steps, with their help lines and messages, so both scripts keep working after each wave.
- **Links**: every relative link that leaves a moved folder (`../docs/...` in `site/README.md` and `extension/README.md`, `../CLAUDE.md` in `extension/README.md`) is rewritten to the URL of the repo that holds the target (`Sobr-cc/sobr` or `Sobr-cc/.github`) in the move PR, otherwise it returns 404. References to a moved page from files that stay in `sobr` (READMEs, `CLAUDE.md`, ADRs, code strings, doc comments, golden files such as the estimator `source` and error messages) are rewritten too; the code ones wait for the Deep cleanup of their crate or of `web/` (§5), and the PR states that golden files are regenerated.
- **Releases**: `sobr` keeps the `vX.Y.Z` tags, because the app and the server embed one `web/` build (ADR-0021); `extension` has its own tags. Each README holds a compatibility matrix (protocol version and `/api/v1`, §3).
- **`data` and DVC**: `data` keeps the current relative layout: `crates/<ingest crate>/`, `schemas/`, `dvc.yaml` and `.dvc/` at the root, because `include_str!("../../../schemas/...")` and the `dvc.yaml` dependencies use these paths. The datasets it ships live in `datasets/`, listed in the `gold` stage dependencies; `sobria-ingest fetch` writes there by default (today `crates/sobria-geoloc/data/`). A root `Cargo.toml` holds the ingest crate only, with the workspace package and dependency entries it inherits today; `Cargo.lock` is copied, so versions match `sobr`. The `copper` stage of `dvc.yaml` depends today on `crates/sobria-core/src`; that dependency is dropped, because ingest no longer depends on `sobria-core` (decision 2). The `dvc.lock` is committed in the move PR, and the single command of CLAUDE.md §3bis (`pipeline run`) runs in `data`. Success test of the move: `dvc repro` completes in `data` and `validate` passes. A "same `MANIFEST.sha256`" test cannot pass yet (Context 4); making the Gold reproducible is a separate fix in `sobria-ingest`.
- **Human actions per repo**: creation, protection of `main` (SOB-17), deploy or remote secrets, coverage of Linear's GitHub integration (SOB-46).
- **Agents' working rules**: they stay in `sobr` (`.claude/`, SOB-43, SOB-44). Each new repo gets a short `CLAUDE.md` (scope, commands, pointer to the common rules). The wave 2 and wave 3 move issues also move the rules and commands of the moved folder out of the `CLAUDE.md` of `sobr` (the extension dev command; the medallion rules, `pipeline run` and `dvc repro`) into the one of the new repo, so that an agent in `sobr` is never told to run a missing command. The conventions of ADR-0018 are unchanged.

### 5. Order of the moves

**Conditions common to every wave**: this ADR accepted; CI green and `main` protected (SOB-8, SOB-17); GitHub links fixed (SOB-40, so that each folder leaves with the right URLs); the Deep cleanup of the folder done if it has one (Foundations rule).

1. **Wave 1: `.github`, `site`, `design-system`.** Outside the Deep cleanup, no code coupling; the only dependency is the documentation the site shows: `.github` goes first in the wave, with a copy of the public documentation, then `site`, whose move removes the documentation from `sobr`; `design-system` runs in parallel. Wave 1 touches no folder that awaits its Deep cleanup: the pages that the code of a crate, or the links of `web/`, cite by path stay in `sobr` for the second docs step below, and the site fetches them from `sobr` like the ADR pages. Starts as soon as the common conditions are met and SOB-66 is done. It validates the procedure at the lowest risk. Its move issue also adds the fonts, icons, `package.json` and a copy of `LICENSES-FONTS.md` (§2), applies the `sobria-design` to `sobr-design` rename of ADR-0020 before the first tag. The script `extension/scripts/sync-logos.js` (manual; the extension build does not use it) stops working when the design system leaves: wave 1 does not touch `extension/`, and the extension move deletes the script in wave 2, because its icons are committed and the tokens replace it.
2. **Wave 2: `extension`.** After its Deep cleanup (SOB-31) and after the contracts are published (§3), otherwise the hand-copied protocol drifts between two repos.
3. **Wave 3: `data`.** After the Deep cleanup of ingest (SOB-21) and the rename of the crates (SOB-37), so that `data` is born with the final names. It is the riskiest wave (DVC, secrets, nightly workflow). Waves 2 and 3 do not depend on each other: the riskier one goes last.

**Second docs step, per folder that cites a moved page, after that folder's Deep cleanup** (today, among the candidate pages of §3: estimator SOB-19, team server SOB-25, desktop app SOB-26 if `docs/operations/dvc.md` is on the list, geoloc SOB-76, and `web/` SOB-28 and SOB-29; `sobria-bridge` cites none; Amendment 1). The Foundations rule keeps a folder untouched before its cleanup, and SOB-19 requires the golden files to stay unchanged until then, so the code side waits. The step copies the cited pages to `.github` and points the site fetch there; then each folder rewrites its strings, comments, tests and links to point to `.github` (the estimator regenerates its golden files), and a page leaves `sobr` when the last folder stops citing it, so no link is dead between the two. The site move does not wait for it.

Token adoption follows the move of each consumer; `web/` waits for the end of its Deep cleanup (SOB-27 to SOB-29). The work inside `sobr` (single app ADR-0021, rename ADR-0020, removal of `import` SOB-67) does not depend on the waves, and issues that live in `sobr` (the contracts, the reference cases, making ingest independent, the delivery ADR) may start before their wave: only the moves wait for their gates. Each wave is a stopping point: at its end, `sobr` builds and its CI is green.

**Rename (ADR-0020).** The repo names are set here; the names of crates, folders and packages belong to ADR-0020, applied repo by repo. Zenodo and DOI are per repo: `sobr` (software) and `data` (dataset) each have their own `.zenodo.json` and Zenodo workflow (the software ones stay in `sobr`, §2). The `sobria-*` names in this ADR are those of 2026-10-30.

**Single app (ADR-0021).** Rule 1 holds with one or two apps; ADR-0021 relies on it.

### 6. Issues to create

Titles in English, to be created in Linear by the orchestrator (SOB-66, SOB-69 and SOB-70 already exist), each of 8 points at most. Points are indicative; the official estimate is made at creation. Total: 76 points (75 without the conditional desktop-app issue), not counting the `vscode` and `claude-code` repos.

| When | Issue | Role | Points |
|------|-------|------|--------|
| Before the waves | Share the agent working rules across the Sobr-cc repos (after SOB-43 and SOB-44) | CTO | 3 |
| Wave 1 | SOB-66 Create the site and .github repos in Sobr-cc (existing; add `design-system`) | DevOps | 1 |
| Wave 1 | SOB-70 Set up the Sobr-cc/.github repo (existing; a short generic CONTRIBUTING.md instead of the cargo one) | DevOps | 2 |
| Wave 1 | Copy the public documentation, with its history, to Sobr-cc/.github from a list approved by Thibault | DevOps | 3 |
| Wave 1 | Rewrite the references to moved pages in the READMEs, CLAUDE.md and ADRs that stay in sobr, and the catalog path of LICENSE-DATA; starts once the docs copy exists and lands before or with the site move | DevOps | 1 |
| Wave 1 | SOB-69 Move the site to the Sobr-cc/site repo (existing; after the docs copy; add the daily fetch of the public documentation from .github, and of the ADR pages and the pages cited by code from sobr, sync the listed pages again and remove them from sobr, copy LICENSES-FONTS.md with site/public/fonts, point "Edit on GitHub" to each source repo, remove the two links of the application page to technical documents, point the extension architecture link to its GitHub page (Amendment 1), and fix the README links; re-estimated) | DevOps | 5 |
| Wave 1 | Move the design system to Sobr-cc/design-system (fonts, icons, package.json, LICENSES-FONTS.md copy, ADR-0020 rename), and tag its tokens | DevOps | 5 |
| Wave 1 | Add reusable CI workflows to Sobr-cc/.github (before wave 2) | DevOps | 3 |
| Wave 1 | Use the design tokens by tag in the site | Svelte front end | 2 |
| Second docs step | After the docs copy and the site move: copy the pages cited by code or by `web/` to Sobr-cc/.github and point the site fetch to them; the last folder to stop citing a page removes it from sobr | DevOps | 1 |
| Second docs step | Estimator, after SOB-19: rewrite the references to moved pages in code strings, doc comments and tests, and regenerate its five golden files | Lead Rust | 2 |
| Second docs step | Team server, after SOB-25: rewrite the references in its README, `cli.rs` and `tls.rs` | Lead Rust | 1 |
| Second docs step | Desktop app, after SOB-26: rewrite the reference in `logic.rs`; only if `docs/operations/dvc.md` is on the list, otherwise no issue (75 points) | Lead Rust | 1 |
| Second docs step | Geoloc, after SOB-76: rewrite the catalog reference in `datacenters.rs` and in the embedded `datacenters_demo.json` | Lead Rust | 1 |
| Second docs step | `web/`, after SOB-28 and SOB-29: repoint the links to the glossary and to the team server guide (`/methodo`, `/parametres`) to `.github`, and rewrite the paths named in its comments and visible text, `docs/operations/dvc.md` included if it is on the list | Svelte front end | 1 |
| Wave 2 | Create the extension repo in Sobr-cc (human-only) | DevOps | 1 |
| Wave 2 | Publish the bridge JSON Schema and the server OpenAPI with each release | Lead Rust | 5 |
| Wave 2 | Export the reference cases of validation/cases.rs (inputs, Python targets, tolerances) as JSON, as a second output of tools/fixturegen (no change to sobria-estimator), with their method, modalities and overhead, and publish them with each release | Lead Rust | 3 |
| Wave 2 | Move the extension to Sobr-cc/extension; remove its steps from build-all.sh and bootstrap.sh, delete sync-logos, move its rules and commands from CLAUDE.md, and rewrite its README links | DevOps | 5 |
| Wave 2 | Add contract tests against the bridge schema and the API in the extension | Dev extensions | 3 |
| Wave 2 | Make the parity test of the extension read the published reference cases (the EcoLogits ones, with the tolerance of each case) instead of values copied by hand | Dev extensions | 3 |
| Wave 2 | Use the design tokens by tag in the extension | Dev extensions | 2 |
| Wave 3 | Create the data repo in Sobr-cc (human-only) | DevOps | 1 |
| Wave 3 | Make sobria-ingest independent of sobria-core and sobria-geoloc (copy the seed; read and write the datasets in a datasets/ folder, fetch included) | Lead Rust | 3 |
| Wave 3 | Version the schema of the Gold tables that sobr reads (sources, model_overview), next to the datasheet schema | Lead Rust | 1 |
| Wave 3 | Move sobria-ingest, the schemas and the DVC pipeline to Sobr-cc/data, with the layout of §4, a committed dvc.lock, one nightly workflow, no ingest or DVC step, help line or message left in the scripts, the medallion rules and commands moved from the CLAUDE.md of sobr, and a copy of LICENSE-DATA (sobr keeps its own) | DevOps | 8 |
| Wave 3 | Add .zenodo.json and the Zenodo workflow for the dataset to Sobr-cc/data (enabling Zenodo is a human step) | DevOps | 2 |
| Wave 3 | Write the ADR for delivering data updates (catalog, territory files, datacenter records) to the app | CTO | 3 |
| Wave 3, after the delivery ADR | Add a contract test in sobr against the Gold schema of the latest data release | Lead Rust | 2 |
| After the Deep cleanup of `web/` | Use the design tokens by tag in web | Svelte front end | 2 |
| Later | Create the vscode repo; create the claude-code repo (human-only, when their project opens) | DevOps | not counted |

If an estimate exceeds 8 points at creation, the issue is split by repo (the new-repo side and the `sobr` removal side).

Other existing issues to adjust: SOB-38 (split per repo if the extension has already moved), SOB-39 (closed into the design-system move), SOB-6 and SOB-71 (the catalog arrives in `data`).

### 7. Text that replaces the "Repos" decision of the routes document

> **Repos** — GitHub organization Sobr-cc, one repo per delivery unit: `sobr` (engine, desktop app, organization server, Svelte app, ADRs and technical documentation), `extension`, `site`, `design-system`, `data`, `.github` (community files and public documentation) and an infra repo (name set by SOB-73); `vscode` and `claude-code` when their projects open. See ADR-0019.

## Amendment 1 — Links, parity cases, data license and counts (2026-10-31)

**Status.** Accepted (2026-10-31), SOB-169; merging the PR is Thibault's approval. Written in place by PR (SOB-87). The recommended option of each choice below is applied; if Thibault picks another in the PR, the text follows before the merge. Thibault's four answers, the options and the three waves are unchanged. The lists, counts and rows that were wrong are corrected in the text above; this section says which.

**Why.** The review of PR #2 at 404ddae (sobria-07) finished after the merge and found five gaps. Each was checked against `main` on 2026-10-31.

**What changes.**

1. **`web/` links to pages that move.** The rule that keeps a cited page in `sobr` until its second docs step covered only "the code of a crate". It now covers every folder that awaits its Deep cleanup, `web/` included (§2, §3, §4, §5). `web/` links to `docs/methodology/GLOSSAIRE.md` (`web/src/routes/methodo/+page.svelte:373`) and to `docs/operations/team-aggregator.md` (`web/src/routes/parametres/+page.svelte:1230` and `:1662`) through the old GitHub URL, and it names such paths in comments and visible text (`methodo/+page.svelte:33` and `:377`, `parametres/+page.svelte:1023` and `:1467`, `lib/lexique.ts:3`, and the comments of `datacenters` and `modeles`). The line numbers are those of 2026-10-31. Without the rule, the desktop app and the web demo ship dead links once the site move removes the pages. SOB-108 (after SOB-27) repoints the links to `Sobr-cc/sobr`; the second docs step repoints them to `.github`, after SOB-28 (`/methodo`) and SOB-29 (`/parametres`).
2. **Which folders cite a page.** Checked in the code, among the candidate pages of §3: the estimator (SOB-19), the team server (SOB-25), the desktop app (SOB-26, only `docs/operations/dvc.md`, if it is on the list), geoloc (SOB-76) and `web/` (SOB-28, SOB-29). `sobria-bridge` cites none and leaves the list.
3. **Site links to technical pages.** The site links three pages that §3 keeps in `sobr`: `site/src/pages/candidature.astro:152` (requirements document) and `:158` (application document), and `telecharger.astro:90` (extension architecture). The fetch of §3 does not publish them. The outcome is in the choices below.
4. **The estimator parity contract.** Today the extension test copies three EcoLogits cases by hand and checks them with a fixed 2 % constant (`extension/tests/unit/empreinte.spec.ts:70`). The export carries every case of `REPRODUCTION_CASES` with its `method`, modalities, overhead and `tolerance` (§3). The extension runs the three EcoLogits cases with the tolerance of each case (1 %). The five `c34-*` AFNOR cases (25 to 30 %) stay Rust-only, because the JS port computes none of their modalities, overhead or Monte-Carlo. If the port misses 1 % on a case, the fix is its own issue, and the tolerance is not loosened without a decision.
5. **`LICENSE-DATA`.** `sobria-geoloc` embeds Etalab data in the binaries of `sobr` (`data/territoire_fr.json`, `rte_mix_fr.json`, `datacenters_demo.json`). `LICENSE-DATA` goes to `data` and a copy stays in `sobr` (§2). In both copies, the path `docs/sources/CATALOGUE-SOURCES.md` becomes the URL of the page where it lives after the docs copy (`.github` if it is on the first list of §3, `sobr` otherwise).
6. **Counts.** `main` has 10 crates since SOB-67 (Context). The second docs step is one issue per folder, and the total of §6 is 76.

**Choices for Thibault.** Three points were left open. The text above follows the recommended option of each.

| Point | Option | Cost and risk |
|-------|--------|---------------|
| Site links (point 3) | A. The site fetches the three pages from `sobr` | Three more paths, same mechanism as the ADR pages; the site stays coupled to project documents of `sobr` and keeps publishing the application document |
| | B. SOB-69 removes the link. **Recommended for the two links of the application page** (`candidature.astro`), which presents an application that is off (SOB-104) | Lowest cost; the page loses a link |
| | C. SOB-69 points the link to the GitHub page in `sobr`. **Recommended for the extension architecture**, a technical page of §3 | Low cost; the reader leaves the site |
| `LICENSE-DATA` (point 5) | A. A copy stays in `sobr`. **Recommended**: `sobr` ships Etalab data, and the licence must travel with it | Two copies to keep in step; `data` holds the master |
| | B. No copy | False while geoloc embeds the files |
| Second docs step (point 6) | A. One issue per folder. **Recommended** | 6 points for the folders, 7 with the copy row; each waits for its own Deep cleanup, as the Foundations rule asks |
| | B. One issue after the last cleanup | 3 points; the pages stay in `sobr`, and the site fetches them there, until SOB-76 and SOB-29 are done |

**Checks added to the move issues.** `git grep -n "docs/methodology\|docs/operations" -- web/src` lists only references that the second docs step rewrites (links, comments and visible text). `git grep -n 'href="/docs/[A-Za-z]' -- site/src/pages` lists the links to docs pages, and once SOB-69 has removed or repointed the three links each target must be a page of the site fetch; the other links (sources and operations) are on the first list of §3.

**Not covered here.** The status of ADR-0020 and ADR-0021 (SOB-111).

**Issues.** SOB-169, SOB-108, SOB-69, SOB-70, SOB-19, SOB-25, SOB-26, SOB-76, SOB-28, SOB-29, SOB-67, SOB-104.

## Consequences

**Positive**:
- Independent delivery: the extension ships without rebuilding the app, the site deploys without touching Rust, the data pipeline runs at its own rhythm.
- CI per repo, with natural filters: the end of the single `ci.yml` that rebuilds everything on each push (CLAUDE.md §8, frugality).
- A clear owner per repo, by role.
- Contracts (bridge schema, OpenAPI, reference cases, versioned tokens) replace hand copies that are checked only against values copied by hand.
- No code dependency between repos: the only ones left are versioned contracts and a one-way data artefact.
- Reversible by waves (§5).

**Negative**:
- The work listed in §6, with no visible feature.
- Contracts to maintain, and a compatibility matrix between `sobr`, `extension` and `data`.
- Duplicated configuration (`deny.toml`, `rustfmt.toml`, lint): reduced by the reusable workflows of `.github`.
- Human actions at each repo (§4).
- `sobr` stays big (decision 1).
- Documentation lives in two repos: a link between a technical page and a public one crosses repos (§4, Links).
- The datasets exist twice for a while (source in `data`, embedded copy in `sobr`) until the delivery ADR.

**Neutral**:
- One ADR register in `sobr`; numbering stays continuous.

## Links

- Linear: SOB-65, SOB-33 (the "no monolith" decision), SOB-66, SOB-69, SOB-70, SOB-73, SOB-74
- ADR-0006 (licenses), ADR-0007 (DVC), ADR-0009 (medallion), ADR-0013 (bridge), ADR-0017 (web demo), ADR-0018 (Linear conventions), ADR-0020 (rename), ADR-0021 (single app)
- CLAUDE.md §3bis, §6, §8, §10
- [Routes, screen by screen](https://claude.ai/artifact/Cua7SK84VzMCwYFZRQfUHD) (decision "Repos", replaced by §7; decision "Platforms" for Edge)
- Linear, "How we work"
