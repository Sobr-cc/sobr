# Changelog Sobr.ia

All notable changes are documented here, following [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/) and [SemVer](https://semver.org/).

Format: `[X.Y.Z] - YYYY-MM-DD`
Types: `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`.

> **New entries go to `changelog.d/`**, one fragment file per pull request
> (see [`changelog.d/README.md`](changelog.d/README.md)). Do not add entries
> here: `scripts/changelog-assemble.sh` moves the fragments into this file at
> each release. This file keeps the history.

## [Unreleased]

### Fixed — two team server tests that failed in CI (2026-10-31, SOB-167)

- **`sobria-team-aggregator`** (tests, no behavior change):
  `admin_user_detail_route_respects_visibility_policy` read the user detail
  with the default window (last 30 days), which stopped covering its fixed
  `2026-05-16` estimation in June; it now passes a fixed `from`/`to` window,
  and checks the default window with a recent estimation.
  `embedded_dashboard_is_served` skips when the binary has no web-team
  bundle, as in the Rust CI jobs (new `embedded_web::bundle_embedded()`); the
  reason shows with `--nocapture`, and `SOBRIA_REQUIRE_EMBEDDED_UI=1` turns
  the skip into a failure for a job that builds the bundle.

### Changed — sobria-bridge Deep cleanup (2026-10-31, SOB-24)

- **`sobria-bridge`**, no behavior change (same replies, error texts and spool
  lines; same tests): one `MAX_MESSAGE_BYTES` constant and one frame writer
  for the stdio replies and the forward to the app, each frame and spool line
  sent in a single write, `unsafe` code denied, `append_to_spool` private.
  Doc comments that were wrong are fixed (the local channel is not restricted
  to the user, SOB-107; no forward timeout on Windows, SOB-96; `short_hash`,
  SOB-81). The README is rewritten, and the unused
  `manifest/com.sobria.bridge.json.tmpl` is removed (the scripts use
  `manifest.template.json`). Test helpers are shared through `tests/common`.
- Audit findings that change behavior are filed: SOB-190 (fallback after the
  app received the request, exit on a bad message) and SOB-193 (estimate
  mapping in the app).

### Added — Characterization tests of the browser extension (2026-10-31, SOB-158)

- **`extension`**: Vitest tests in `tests/unit/` record what the code does
  today, without changing it: the service worker (every message, local
  storage, forwarding to the native bridge and to the team server), native
  messaging, the team client (token refresh on 401, errors), team storage
  (including the browser fingerprint) and storage; the three site adapters,
  driven through their own `extractModelId` and `onSubmit` so the real alias
  tables and selectors are used; and a comparison of the 34 presets with the
  Rust registry (read from the estimator's `model_registry.json`). Line
  coverage goes from 40 % to 74 % and function coverage from 53 % to 88 %.
  Quirks found are pinned and filed: SOB-171 (four preset labels differ from
  the Rust registry) and SOB-176 (labels resolved to the wrong model); the
  double counting is SOB-118.

### Removed — unused `duckdb` dependency (2026-10-31, SOB-168)

- **`sobria-ingest`**: dropped `duckdb`, which no code called (decision
  SOB-172; it comes back with the analytics step). The bundled `fmt` of
  `libduckdb-sys` 1.10502.0 used `stdext::checked_array_iterator`, which the
  MSVC 14.51 STL of the `windows-latest` CI image no longer has, so the
  workspace no longer built on Windows. Also drops `zip` 6
  (RUSTSEC-2026-0367, -0368, -0369) and 52 other locked packages.

### Changed — Sources catalog translated to English (2026-10-31, SOB-149)

- **`docs/sources/CATALOGUE-SOURCES.md`** (version 2.2): the French part (sources
  S01 to S09, tiers, quick matrix, Silver schema overview, prerequisites,
  retention policy) is translated to English, following "translate when touched"
  (SOB-100). No fact, URL, identifier or number changed; the model catalog
  section of SOB-6 is untouched. Proper names (Teréga, ODRÉ) keep their accents.

### Changed — ADR-0020 and ADR-0021 marked Accepted (2026-10-31, SOB-111)

- **`docs/adr/`**: both ADR files and their rows in the index say Accepted
  (2026-10-30), the day Thibault approved their PRs. No other line changes.

### Added — ADR-0023, the Sobr MCP server contract (2026-10-31, SOB-144)

- **ADR-0023** (Accepted): Claude reads and acts on Sobr through MCP tools,
  within the rights of the person who connected it. Thibault answered its
  eight points on 2026-10-31 (SOB-152): every recommendation. It is
  Accepted because ADR-0019 and ADR-0022 are merged.
  - Two front doors share one contract and one repo (`sobr`): a stdio server
    for the personal space and a Streamable HTTP route in the Sobr server for
    an organization. One connection serves one space.
  - Access comes from a grant made in Sobr (`read`, `write`, `act`), first as
    a scoped token, later through OAuth 2.1. Rights are looked up on every
    call, and a grant only narrows them.
  - 22 read tools (C2) and 25 write tools (C3) are listed with inputs,
    outputs and permissions. Irreversible tools return a pending action that
    the person approves in Sobr, so a client without a screen works. A few
    actions never go through the MCP.
  - Every write and approval goes to an append-only, chained audit log.
  - 22 build issues (111 points), marking those that wait for a Deep cleanup.
  - Settles the condition of ADR-0019, decision 4. Nothing is built yet.

### Added — Audit ledger migration test (2026-10-31, SOB-160)

- **`sobria-audit`**: `tests/ledger_migration.rs` opens a committed v1 ledger
  (`tests/fixtures/ledger-v1.sqlite`, no `method` column; `make_ledger_v1.py`
  rebuilds it) and pins the v1 to v2 migration: the `method` column is added last
  with `afnor_sobria` on every entry, the existing rows, their order, signatures and
  the purged entry are unchanged, the chain still verifies and the indexes exist.
  A second test pins that opening the migrated file again changes nothing.
  `*.sqlite` is now marked binary in `.gitattributes`.

### Changed — ADR-0016 breakdown by project (2026-10-31, SOB-153)

- **ADR-0016, Amendment 1** (Accepted): in the `anonymous` and `opt_in`
  policies, the breakdown by project appears only when every group shown,
  "other projects" and "no project" included, has at least k contributors
  (SOB-148), the rule ADR-0022 sets at task level. The code work waits for
  the Deep cleanup of the team server (SOB-25).

### Added — ADR-0024, Kubernetes platform on Proxmox (2026-10-31, SOB-72)

- **ADR-0024** (Proposed; Thibault chose every recommendation in SOB-175,
  and it becomes Accepted once he confirms the server facts in SOB-181):
  Sobr runs on Kubernetes in Proxmox VMs, with Talos Linux nodes managed
  from code.
  - Cilium gives the network, network policies, addresses and the Gateway
    API entry, and cert-manager gets the certificates. Knative Serving runs
    functions that scale to zero.
  - The managed cloud runs one Sobr server for all spaces, as ADR-0022
    sets. A small EU virtual server in front forwards encrypted traffic and
    holds no secret.
  - Customer data is protected in four layers:
    - a continuous encrypted copy of every database in a three-zone EU
      object store, from which the server restores itself;
    - etcd snapshots off the machine;
    - Proxmox VM rollback;
    - monthly restore tests and a full rebuild test.

    An erasure log is replayed after any restore.
  - Disk keys come from a key server on a small machine Thibault controls
    elsewhere. CI runners are throwaway VMs on a firewalled network.
  - The managed cloud opens, trial included, only once these protections,
    erasure, export and the GDPR documents exist. A second failure domain
    is needed before the public launch.
  - It records Thibault's ten choices, lists the server facts still to
    confirm, and lists seventeen issues to create.

### Changed — One golden-file comparator for the estimator and team server tests (2026-10-31, SOB-150)

- **Tests** (`sobria-estimator`, `sobria-team-aggregator`): the two copies of
  the golden-file comparator are now one file, `test-support/golden.rs`,
  included by both crates with `#[path]`. No new crate and no new dependency.
  Tolerance (1e-9), update mode and the CI guard are unchanged. The team server
  tests now get the estimator's `Goldens` collector and its layout for rewritten
  files (arrays of numbers on one line, which its committed files already
  have: an update run on them rewrites nothing). Two differences remain for
  them: a `computed_at` field would be masked, and a difference in an array of
  integers is reported on one summary line (pass or fail is the same). The
  comparator's messages are in English. The estimator keeps the comparator's
  unit tests in `tests/characterization/golden.rs`.

### Fixed — deterministic wrong-code pairing test (2026-10-31, SOB-92)

- **`sobria-app`** (`bridge_server.rs`, test module):
  `pair_with_wrong_code_returns_invalid_error` sent the fixed code `000000`,
  which matched the random pending code once in a million runs. It now sends
  `wrong_code_for(pending code)`, which always differs. The assertion keeps
  its three accepted messages (tightening it is SOB-157). No production code
  changed.

### Changed — Two estimator golden files in the renderer's layout (2026-10-31, SOB-141)

- **`sobria-estimator`** (characterization tests): `simulate_ecologits.json` and
  `forecast_yearly_ecologits.json` now use the layout the golden renderer
  writes (arrays of numbers on one line). Parsed as JSON they equal the
  previous files, number text included, so no golden value changed. An update
  run (`SOBRIA_UPDATE_GOLDEN=1`) no longer rewrites them, which keeps the
  water formula change (SOB-120) free of layout-only diffs.

### Changed — ADR-0019 links, parity cases, data license and counts (2026-10-31, SOB-169)

- **ADR-0019, Amendment 1** (Accepted): the rule that keeps cited pages in
  `sobr` covers `web/`, whose links to the methodology glossary and the team
  server guide would break; `sobria-bridge` leaves the second docs step list.
  The extension parity test uses the three EcoLogits cases with each case's
  tolerance. `sobr` keeps a copy of `LICENSE-DATA` for the Etalab data of
  `sobria-geoloc`. The site links to three technical pages have a stated
  outcome. The second docs step is one issue per folder, plus the demo fixtures: 77 points.

### Added — ADR-0022, organization, project, task and session model (2026-10-31, SOB-143)

- **ADR-0022** (Accepted 2026-10-31; Thibault took every recommendation in
  SOB-151): every use of an AI tool is a session that belongs
  to one person and, once known, one space: personal or organization. A
  project and a task are optional, a task belongs to one project, and an
  unclassified session waits in "to classify".
  - Projects and tasks are made by hand or linked to Linear, GitHub and
    later trackers through one generic external link. Solo mode is one
    hidden project.
  - Only the person attaches their own sessions; project editors act
    through project links, and others see daily totals, never single
    sessions (ADR-0015). A session can be withdrawn from an organization to
    the personal space, never the other way.
  - One Rust crate holds the model for the desktop app and the server. Each
    space has its own database and member IDs, with random UUIDs. Every
    client uploads through one contract with no text and no clock time,
    only the day, and the server refuses any other field.
  - Everything starts fresh: nothing is converted, and the old stores stay
    readable and exportable. The ADR lists fifteen issues to create and
    the Deep cleanups they wait for.
  - Breakdowns by project and by task appear only when every group shown,
    the remainder included, has k contributors (SOB-148, option 1); SOB-153
    amends ADR-0016 to match.
  - A personal space can sync between machines through the Sobr server
    (ADR-0013 and ADR-0014, Amendment 2). Session identity across sources
    stays with SOB-146.

### Changed — ADR README says an accepted ADR is amended in place (2026-10-31, SOB-127)

- **`docs/adr/README.md`**: the "immutable once Accepted" rule is replaced by the
  rule Thibault chose in SOB-87: an accepted ADR is amended in place, by a PR he
  approves; `Supersedes` stays for a new ADR he explicitly asks for; an ADR with
  its own revision section (ADR-0014) keeps its extra steps. The file is
  translated to English (translate when touched, SOB-100); the ADR titles in the
  index are left as they are.

### Added — Claude Code usage capture note (2026-10-31, SOB-145)

- **`docs/sources/CLAUDE-CODE-USAGE-CAPTURE.md`**: proposal for measuring
  Claude Code without storing prompts. It covers the local transcripts (an
  undocumented, unstable format), the plugin (status line and hooks),
  OpenTelemetry for organizations, double counting and where the import is
  built (ADR-0019 decision 4), with an official source for each fact and a
  list of what is stored per session. Thibault's answers to SOB-162 (read the
  transcripts, with guard rails) and SOB-164 (one consent step enables the
  status line) are recorded; the open follow-ups are SOB-161, 163 and 166.
  No code changed.

### Changed — ADR-0014 phasing and feature table aligned with October 30 (2026-10-31, SOB-103)

- **ADR-0014, Amendment 3** (Accepted): the phasing follows the calendar of
  October 30, the data.gouv.fr application is off (SOB-104), the cloud is
  announced with the V1 launch, and the README changes then. The card for the
  organizations' trial is kept (SOB-105). Numbered after Amendment 2
  (SOB-154).

### Changed — Free personal space (2026-10-31, SOB-154)

- **ADR-0014, Amendment 2** (Accepted): the personal space on the Sobr server
  is free for individuals; the 30-day trial and the per-seat price apply to
  organizations only. It settles the two open points of Amendment 1: the price
  and the conflict with ADR-0013. Limits on the free space and the date of sync
  between machines are not decided here.
- **ADR-0013, Amendment 2** (Accepted): the personal space is an explicit
  exception to "no hosted accounts" and "no multi-device sync"; organizations
  keep the rule. Thibault's answer: SOB-136.

### Changed — smaller debug builds (2026-10-31, SOB-106)

- **Cargo profiles**: `dev` and `test` keep line tables only for workspace
  crates, and dependencies build without debug info. Backtraces still show
  file and line for workspace crates; dependency frames show names only.
  A debugger no longer sees local variables. `CARGO_PROFILE_DEV_DEBUG=true`
  (`CARGO_PROFILE_TEST_DEBUG=true` for tests) restores full debug info for
  workspace crates; dependencies also need
  `--config 'profile.dev.package."*".debug=true'`. A clean
  `cargo test --workspace --no-run` on Windows drops from 27.8 GB to 8.1 GB
  of `target/`, and a clean `cargo test -p sobria-estimator --no-run` from
  374 MB to 263 MB.

### Added — Characterization tests of the team server (2026-10-30, SOB-10)

- **`sobria-team-aggregator`**: `tests/characterization_*.rs` pin the status code,
  content type and body of every API endpoint, refusals included (enroll, login,
  refresh, access guards, estimations, `/me/*`, enrollment codes, exports, alert
  thresholds and triggers), and the privacy rules on aggregates: k-anonymity
  (default of 5, floor of 3, unreadable value, `k - 1` / `k` boundary, identified
  mode) checked as hidden below the threshold, and the three visibility policies
  on analytics, users and user detail. Responses are recorded from a real HTTPS
  server, random values are masked after a shape check, and the recordings are
  compared with `tests/golden/*.json` (see the README there). The CSRD export is
  checked for its header and size only. No production code changed, no
  dependency added.

### Changed — One golden update run rewrites every file (2026-10-31, SOB-137)

- **`sobria-estimator`** (characterization tests): with `SOBRIA_UPDATE_GOLDEN=1`,
  a test that checks several golden files now rewrites all the changed ones in
  one run, then fails once, listing them (new `Goldens` collector in
  `tests/characterization/golden.rs`). Before, it stopped at the first
  rewritten file. `assert_golden` is unchanged for single-file tests. Test
  code only: the estimator and the golden values are untouched.

### Added — Official model sources per provider (2026-10-31, SOB-6)

- **`docs/sources/CATALOGUE-SOURCES.md`**: new section listing, for the nine
  providers of the model referential, the official model list API, pricing
  page, terms on automated access, published context windows and retired
  models, and whether scraping would be needed. A Hugging Face section covers
  the no-key source for the open-weight models of the referential (Llama, Phi,
  Mistral, DeepSeek); the closed models need each provider's API key.
  Providers without an official machine-readable source (Meta Llama,
  Microsoft) are listed apart with a recommendation. No code and no model
  data changed.

### Removed — sobria-import skeleton crate (2026-10-30, SOB-67)

- **`sobria-import`**: removed the empty skeleton crate (one constant, no
  code, no tests), its workspace entry and the unused dependency in
  `sobria-app` (decision SOB-33, option 2). Any future log import will live
  where the repo split ADR (SOB-65, proposed) places it. The README crate
  list now matches the workspace, and the README and `Cargo.toml` no longer
  hard-code a crate count.

### Changed — ADR-0013 amended for automatic pairing (2026-10-30, SOB-34)

- **ADR-0013, Amendment 1** (Accepted 2026-10-30): after one explicit
  "Connect my browsers" choice in the app, the browser extension pairs
  automatically, with no 6-digit code to type (screen E2, October 30
  arbitration).
  - The app writes native messaging manifests that allow only the published
    Sobr extension IDs.
  - Unpairing is done in the settings on either side and sticks for that
    installation. A reinstalled extension pairs again, with a notice.
    Removing a browser in the app's settings stops it until it is added
    back: the app refuses that browser, since on Windows Edge falls back to
    Chrome's and Chromium's manifests.
  - The amendment keeps the original text and records that it reverses
    alternative 4.
  - It lists the conditions automatic pairing must meet before it ships:
    - store IDs created;
    - the bridge shipped with the app;
    - channels restricted to the current OS user;
    - one pairing per browser profile;
    - a notice to the person;
    - stopping a browser holds in the app;
    - the offline spool checks the secret;
    - nothing measured while the connection is off is kept;
    - "Export JSON" leaves the secret out.
  - ADR-0014 gets a note on its "Pairing perso 6 chiffres" row.
  - Nothing is built yet (SOB-107).

### Changed — ADR-0018 amended to match the Linear organization (2026-10-30, SOB-36)

- **ADR-0018, Amendment 1** (Accepted 2026-10-30): point 1 is replaced, and
  the rest stands.
  - Projects are thematic, not one per chantier: 13 today, including Model
    catalog and Infrastructure.
  - Briefs stay as specs only where they exist (C31, C35, C36).
  - Cycles last two weeks and start on Monday.
  - Labels come in five groups: Type, Area, Agent, Role, Model.
  - Only Claude agents do the work; Linear AI features stay off.
  - The branch format from SOB-12 is confirmed.
  - The working rules stay in "How we work", which the ADR links.
  - CLAUDE.md §2 and §11, CONTRIBUTING.md and the label lists of the issue
    skill follow in SOB-138.

### Added — A refused pairing keeps the code pending (2026-10-30, SOB-91)

- **`sobria-app`** (`bridge_server.rs`, test module): three tests pin what
  `dispatch_request` does for `Pair` after a refusal (ADR-0013). A refusal does
  not consume the pending code: after 20 wrong codes in a row, the right code
  still pairs (there is no attempt limit in the bridge path today; the IPC path
  `logic::verify_pairing_code` is not covered), and an expired code stays
  pending (the second attempt gets the same "code invalide ou expiré"). A
  refused `Pair` records no pairing. No production code changed.

### Added — Bridge characterization tests (SOB-11, 2026-10-30)

- **`sobria-bridge`**: tests (`tests/messages.rs`, `manifest_template.rs`,
  `forward_wire.rs`, `bridge_process.rs`, helpers in `tests/common/`) that
  pin the current behavior before the bridge cleanup (SOB-24) and the
  automatic pairing (E2): accepted and refused messages, framing
  stdin/stdout, local offline replies, spool, forward to the app and
  fallback, manifest shape and the tokens that the scripts
  `install-dev.*` fill in (these are not run).
  No production code change.
- **Scope**: the 6-digit pairing code (TTL 5 min, single use) and
  the per-browser manifest installation live in `sobria-app`, not
  in the bridge, where offline `Pair` and `Revoke` only return a
  fixed refusal. The bridge tests therefore pin this refusal and relay the app's
  replies as is; those of `sobria-app` remain to be completed (SOB-80).
- The files `forward_wire.rs` and `bridge_process.rs` are Unix-only
  (Unix socket, isolated `HOME`); on Windows they are compiled empty.

### Changed — 30-day trial and personal space (2026-10-30, SOB-35)

- **ADR-0014, Amendment 1** (Accepted): the managed cloud has a 30-day
  trial that needs a payment card to start, and is then paid per seat,
  replacing the real free tier of condition 5; local and self-hosted stay
  free. A personal space on the Sobr server is allowed (condition 1
  clarified, guest mode stays the default). MIT stays for all the code. The
  price of the personal space and its conflict with ADR-0013 are left open
  (SOB-136).

### Added — Single app on desktop and web (2026-10-30, SOB-14)

- **ADR-0021** (Proposed): `web/` becomes the only interface. One build is
  embedded by the desktop app and by the organization server, in place of
  `web-team/`. The app detects three contexts at start-up: desktop, web served
  by the server, demo. On the web, the merge covers the organization space;
  the Understand section waits for a decision on prompt privacy. Organization
  data goes through a `$lib/org` client, relayed by Rust on desktop so tokens
  never reach the webview. The ADR maps the 7 web-team pages to screens (M2,
  Z6, Z7, Z9, B2, A6), sets sign-in per context and the security headers of
  the served HTML, and amends rule 2 of ADR-0017. Waiting for Thibault's
  approval.

### Changed — Writing rules in English (2026-10-30, SOB-97)

- **CLAUDE.md** (§13, with pointers in §4.1, §4.3 and §11): everything agents
  write is in English, all documentation included (rustdoc, ADRs, CHANGELOG,
  READMEs); only the product's user-facing text (UI, i18n) stays in French and
  English. Existing French documents are translated when next edited; code
  comments follow the Deep cleanup of their zone. New comment
  rule: the strict minimum, with three exceptions; rustdoc on public items is
  one short line, with an example only when useful (§4.1). The old "FR + EN
  required for docs" rule is removed.
- **`.claude/agents/*.md`** and **`.claude/skills/orchestrate/SKILL.md`**
  translated into English, rules unchanged.

### Added — Pairing code rules pinned in the dispatcher (2026-10-30, SOB-80)

- **`sobria-app`** (`bridge_server.rs`, test module): four tests pin the
  current behavior of `dispatch_request` for `Pair` (ADR-0013), before the
  automatic pairing change. A code is single use (the second
  `Pair` gets « aucun code de pairing en attente… »), an expired code is
  refused (« code invalide ou expiré »), a malformed code (including with
  spaces or full-width digits) and a wrong 6-digit code each get
  their exact message. No production code modified.

### Added — Renaming to Sobr.cc (2026-10-30, SOB-13)

- **ADR-0020** (Proposed): rename Sobr.ia to Sobr.cc under four rules
  by kind of name, crate prefix `sobr-`, with the impact of each
  item for existing users. Waiting for Thibault's approval.

### Added — Estimator characterization tests (2026-10-30, SOB-9)

- **`sobria-estimator`**: safety net before the crate cleanup
  (SOB-19). `tests/characterization/` pins the current outputs (P5/P50/P95,
  histograms, assumptions, errors) of `MonteCarloEngine`, `EcoLogitsEngine`,
  `simulate`, `forecast_yearly`, the model registry (34 presets), the effective
  tokens, the equivalents, the methodologies and `validation`,
  with seed 42 and a fixed timestamp. References in `tests/golden/*.json`
  (compared to within 1e-9, since CI runs on 3 OSes); regeneration with
  `SOBRIA_UPDATE_GOLDEN=1`. No dependency added, no change of
  behavior.

### Added — Claude Code agents by role (2026-10-30)

- **`.claude/agents/`**: seven agents, one per role active in November
  (`po-individuals`, `designer`, `cto`, `rust`, `frontend`, `devops`,
  `qa`). Each file gives the mission, the projects, what the role never
  does and its default model, based on "How we work" and the Organisation
  document in Linear (SOB-43). The orchestrator stays the main
  session.

### Added — Split into several repos (2026-10-30, SOB-65)

- **ADR-0019** (Accepted): split the code into several repos of the Sobr-cc
  organization by delivery unit, in three waves (`.github`, `site` and
  `design-system`, then `extension`, then `data`). The engine, desktop app,
  organization server, Svelte app, ADRs and technical documentation stay in
  `sobr`; the public documentation and the community files go to `.github`.
  It maps every folder to a repo, shares no Rust code across repos, and
  lists the contracts, the order of the moves and the issues to create.
  Thibault's four answers are recorded (SOB-113) and his reading of answer 1
  is confirmed (SOB-139).

### Added — Progress tracking in Linear (2026-10-30)

- **ADR-0018**: Linear (workspace `sobrcc`, team `SOB`) becomes the
  source of truth for progress. CLAUDE.md §2/§4.3/§4.4/§11/§14 and
  CONTRIBUTING.md updated: branches `feature/sob-123-<desc>`,
  `TODO(SOB-123)`, footer `Refs: SOB-123`. `docs/ROADMAP.md` becomes a
  historical document.

### Added — /orchestrate skill and working rules (2026-10-30)

- **`.claude/skills/orchestrate/`**: the orchestrator's routine (sort
  through Triage and propose one action per task, write and split tasks,
  set the Role and Model labels, limit of three In Review tasks,
  Friday updates).
- **`.claude/skills/linear-issue-writing/`**: copy of the team skill
  for writing Linear tasks (SOB-44).
- **CLAUDE.md §11**: the working rules, aligned with the Linear
  document "How we work" (who does what, rules, "Ready", "Done",
  estimation and model).

### Removed — Repository cleanup (2026-10-30)

- **Docs**: Claude Code prompts, commit plans and "superpowers" plans
  deleted (duplicates of the briefs, kept in git history).
  Closed workstreams C01-C45, 2026-05-17 status review, C32.0 product audit,
  text mockup and v0.8.0 smoke test moved to `docs/archive/`
  (index `docs/archive/README.md`), **now excluded from the site** — all of
  `docs/**` was published there, internal archives included. `briefs/` keeps
  only the open work (C31, C35, C36, S0).
- **Stray files**: stale copy `web/crates/sobria-geoloc/data/`
  (output of a `fetch` run from `web/`), root `package.json`
  (leaflet installed by mistake), lock file `.claude/scheduled_tasks.lock`.
- **Dead code**: site sections/three/ui (v1 index, orphaned) and deps
  `three`, `@types/three`, `motion`; `ComingSoon.svelte`, `moduleInfo`,
  `isModuleEnabled`, aliases `$stores`/`$ipc`/`$components`, dep
  `@tauri-apps/plugin-shell` (web); `thread-finder.ts`,
  `find-chat-thread.ts`, `pickTone`, `gradeColor`, `AVAILABLE_*`,
  `availablePresets` and a no-op MutationObserver (extension).
- **Unused Rust deps**: `serde_with`, `arrow`, `arrow-schema`,
  `blake3`, `criterion`, `geo` (workspace) and `[profile.bench]` without bench;
  `tokio`/`anyhow`/`tracing`/`sobria-core` in export, geoloc, referentiel.

### Changed — Factorization (cleanliness audit 2026-10-30)

- **`web/src/lib/format.ts`** (+ Vitest tests): `fmtSig`, `pickScale`,
  `fmtTotal`… replace the copies in ResultBlock, comparator, Verdict,
  tracking, datasheets and CSRD report (unit strings already diverged).
  `npm test` (web) limited to `src/**/*.test.ts` — it was picking up the
  Playwright specs.
- **Extension**: `lib/format.ts` (`fmtFr`), `toneOf` in `grade.ts`,
  car factor 192 g/km exported only once (3 copies).
- **Rust**: SHA-256 hex via `format!("{:x}")` / `hash::hex_encode` instead of
  5 in-house encoders; spool via `sobria_bridge::spool_path()`; Team Mode
  exports: global lock released before generating PDF/CSV/PROV-O.

### Fixed — Audits runnable on a fresh clone / on Windows

- `sobria-team-aggregator` compiles without `web-team/build` (`allow_missing`
  rust-embed): the documented "bundle missing" fallback becomes real, clippy
  and `cargo test` pass on a clean clone.
- `.gitattributes` (`eol=lf`): with `core.autocrlf=true`, Prettier
  rejected every file on Windows.
- Clippy 1.99 lints (`assert_is_empty`), site lint (vendored three.js
  ignored, inline script), site/web-team Prettier formatting, `.gitignore`
  (rule with an inline comment that matched nothing).

### Added — C46 CI repaired + real a11y gate

- **Real a11y suite** (`web/tests/a11y/`): axe-core audit (WCAG 2A/AA)
  of the 5 key pages in demo mode, gate on the violations
  critical/serious — the CI job "a11y" pointed to a directory
  nonexistent since its creation.
- **`<html lang="fr">`**: the `%lang%` placeholder was never
  substituted and shipped as is in prod (a11y defect detected by the audit).
- **Dedicated e2e port** (`SOBRIA_E2E_PORT`, `--strictPort`):
  `reuseExistingServer` could silently test the dev server of ANOTHER
  project open on 5173.
- **CI**: extension unit tests added to the job (90 tests, never
  run in CI before); tarpaulin coverage made informational
  with a published artifact — the `--fail-under 80` gate had never produced
  a single measurement in 41 runs.

### Fixed — C46 Real a11y violations (first pass of the axe gate)

- **AA contrast**: `--ivory-3`/`--ivory-4` recalibrated (4.63→5.5:1 and
  2.9→4.9:1 on the lightest panel) — the old ivory-4 was
  documented "decorative only" but carried useful text in
  ~30 components (up to 33 non-compliant nodes on /comparer).
- **ARIA**: accessible name on the datacenter combobox, `role="img"` on
  the Oui/Non icons of the vendors table, `role="listitem"` on
  the methodology rows (the parent `role="list"` was orphaned —
  critical violation).

### Fixed — C46 Pre-commit stabilization (audit 2026-06-12)

- **Broken employee detail route**: `/admin/users/{id}/analytics`
  used the axum 0.8 parameter syntax in an axum
  0.7 project (`{id}` is a literal there) → systematic 404 on the
  flagship endpoint of C44. Fixed to `:id` + HTTP integration test covering
  the 3 ADR-0016 policies (the route had none).
- **`cargo test -p sobria-team-aggregator` compiles again**: test helper
  for the alerts not updated after the DDL v4 (field `project`
  missing).
- **CORS for the extension**: the MV3 service worker calls the team
  API from a `chrome-extension://…` origin — without CORS headers
  Chrome blocked the response and the best-effort send failed silently.
  `CorsLayer` limited to browser extension schemes (never a
  web origin), preflight covered by an integration test.
- **Extension manifests resynchronized**: Firefox frozen at 0.6.0 with a
  description predating Team Mode → aligned to 0.9.0; typo
  « sou self-hosted » fixed on the Chrome side; local packaging
  artifacts (`Sobria.zip`, copies of `dist-firefox`) ignored by git.

### Added — Closing docs (post-C45)

- **ADR-0017**: the "web demo contract" (C37) finally formalized —
  real-engine fixtures, never in the desktop app, permanent
  transparency, partial coverage acknowledged.
- **`briefs/COMMIT-PLAN-C43-C45.md`**: addendum to the commit plan
  (batches 9-16) with a reminder of the pre-commit checks.
- `/equipe`: ADR-0016 link wired up (was bare text).

### Added — C45 Immersive manifesto « Le poids invisible »

- **`/manifeste`**: experience in 7 word scenes (Invisible · Mesurer ·
  Douter · Choisir · Ensemble · Ouvrir · Commencer) — giant
  declarations, scroll reveals, marquees, animated counter and k-anonymous
  folding, real engine figures (seed 42). Direction inspired by the
  validated references (scene-based manifesto + typographic play).
  Strict progressive enhancement: complete page without JS,
  full `prefers-reduced-motion` support, zero dependencies.
- Entry points: hero link on the home page + footer. Art direction + storyboard:
  `docs/archive/chantiers/C45-manifeste-immersif.md`.

> ⚠ C45: v1 written WITHOUT screen rendering (environment unavailable) —
> visual iteration mandatory before featuring it (brief §5).

### Added — C44 Visibility policy + project dimension (ADR-0016)

- **`visibility_policy`** (anonymous | opt_in default | identified):
  chosen at deployment, changeable via CLI; the named mode requires a
  **CSE (works council)/employees attestation** (`config set visibility_policy
  identified --attest "…"`), stored and logged. Matrix enforced on the
  server side for analytics, employee list and individual detail.
- **Per-employee detail**: `GET /admin/users/:id/analytics` + web-team page
  `/admin/users/[id]` (daily series, models, methodologies) —
  403 explained when the policy forbids it.
- **Project dimension**: `estimations.project` (DDL v4), label per
  CONVERSATION chosen in the extension popup (tab URL key,
  resolved at dispatch via sender), « Par projet » aggregates on the dashboard
  with k-anonymous fallback (« autres projets ») outside named mode.
- `/me/sharing` returns the policy: each employee knows under which
  regime they work (dedicated line in their space).
- **Site**: new pages `/produit`, `/equipe` (3 policies,
  comparison table), `/methode`; `/telecharger` restructured;
  `/cloud` → `/equipe`; topbar/footer rewritten (51 pages, green build).

> ⚠ C44: partial verification (environment outage during the
> workstream) — see `docs/archive/chantiers/C44-politique-projets-site.md` §5
> for the verified/unverified boundary and the commands to run.

### Changed — C43 Showcase: extension + presentation site

- **Extension popup reworked** into 3 levels (today's figure, pairing
  status, actions) with collapsed details, polished empty states,
  formal address, AA contrasts, ≥ 12px; calmer in-page indicators
  (« fourchette x – y g (confiance 90 %) », end of the infinite pulse).
  +19 tests (84/84 green).
- **Site (scrollytelling home page)**: Équipe chapter aligned with
  ADR-0015 (opt-in participants + anonymous aggregate + k threshold — end of the
  fictional named ranking), new chapter « Mesurer, puis
  réduire » (real engine deltas), full formal address, defensible figures
  (28 documented datacenters), v0.9.0, final CTA with 3 buttons
  (extension · app · Team Mode self-hosted).


### Changed — C42 Readable slugs + non-Tauri consistency

- **Human-readable URLs**: `/m9 /m15 /m17 /m25` → `/modeles /suivi /datasheets
  /eco-budget`, client redirects from the old addresses
  (query/hash preserved). Last internal codes removed from the facade.
- **No more actions that lie outside Tauri**: banner on load of
  rapport-csrd (+ Générer disabled), journal ledger buttons,
  « Nouveau projet » (datasheets) and eco-budget submit disabled with
  an explanation on hover.

### Added — C42

- **`team-docker` CI**: build of the Team Mode image (C40 kit) + smoke
  test `/health` in a container — validates what could not be validated in
  session.
- **`briefs/COMMIT-PLAN-C37-C42.md`**: 8 proposed commits, exact files,
  pre-existing WIP isolated (224 files out of scope).


### Changed — C41 Usability finishing touches

- **Composer default model**: `gpt-4o-mini` (deprecated) →
  `claude-haiku-4-5` — the « Réduire » loop shows real
  alternatives from the first estimate.
- **« Comparer en détail » pre-fills the comparator**
  (`?models=…&tin=…&tout=…`, validated ids, ≥ 2 required).
- **Inline glossary**: 10 definitions (P50, P5–P95, prefill, PUE…) as
  accessible tooltips linked to the M8 glossary — placed on ResultBlock
  and Composer (`lib/lexique.ts`, `Term.svelte`).

### Fixed — C41

- **Paramètres**: split bootstrap — Runtime/Référentiel/Méthodologies
  display in demo even when pairing (desktop-only) rejects
  (C37 bug §1); last `cargo run` message replaced.
- **ESLint**: global `__APP_VERSION__` declared (no-undef went unnoticed,
  as the eslint step was truncated by sandbox timeouts).


### Added — C40 Team Mode deployment kit (Docker + communication)

### Added — C40 First-hour journey + « Réduire » loop

- **Onboarding**: « Mesure automatique » card at the final step —
  generation of the extension pairing code without going through Paramètres.
- **« Réduire » loop**: under each result, up to 3 more frugal alternatives
  re-estimated by the engine (`estimate_for_comparison`),
  % deltas, Comparer / Éco-budget links. Never client-side heuristics.
- **M15**: welcoming empty state with CTA (first prompt / extension).
- **UAT kit** (docs/qa/uat/): protocol, SUS script, 27 tasks across
  5 personas, grids — ready to run (C36).
- e2e: C39 rail asserted (essentials + « Plus » toggle) and Réduire
  loop covered in estimate.spec.

- **`deploy/team/`**: multi-stage Dockerfile (SvelteKit build `web-team`
  → Rust build with dashboard embedded via rust-embed → runtime
  non-root `debian:stable-slim`, `/data` volume, `/health` healthcheck),
  `docker-compose.yml` (single service, named volume) and `entrypoint.sh`
  (automatic `init` on first start via
  `SOBRIA_TEAM_ADMIN_PASSWORD`, then `serve`).
- **`docs/operations/deploiement-equipe.md`**: micro-business/SME guide ~30 min —
  Compose startup, Option A self-signed cert / Option B reverse proxy
  Caddy + Let's Encrypt, enrollment codes, ADR-0015 privacy settings,
  SQLite/WAL backup, update, troubleshooting. ⚠️ Images not tested in
  CI to date.
- **`docs/operations/modeles-communication.md`**: email templates
  for CSE information-consultation + employee announcement (ADR-0015 guarantees,
  disclaimer « à faire valider par votre juriste »).

### Changed — C39 Simplified rail + jargon removed

- **Navigation rail redesigned**: 5 essentials with labels (Estimer,
  Comparer, Suivi, Modèles, Datacenters), 9 modules behind « Plus »
  (state persisted, auto-expands if the page is active) — no more of the 13
  mystery icons. Width 76 → 96px, single snippet instead of 3 duplications.
- **Module codes removed from the UI**: eyebrows « Module M9 · … »
  humanized on 12 pages (codes kept in the docs).

### Added — C38.x aggregator finishing touches

- **Retention**: automatic purge of estimations > `retention_days`
  (default 730 d) at startup then daily.
- **CLI `config list|get|set`** (k_anonymity_min, retention_days) with
  validated floors; extended operator doc (privacy, CSE, GDPR).

### Fixed

- **web-team formatting**: pre-existing Prettier drift cleared across
  the whole package.


### Added — C38 Privacy-first team dashboard (ADR-0015)

- **ADR-0015**: privacy scope of Team Mode — k-anonymity of
  admin aggregates (threshold `config.k_anonymity_min`, default 5, floor 3),
  identification **opt-in controlled by the employee**, full
  self-service, masking applied server-side.
- **`users.share_identified`** (DDL v3) + **`GET|PUT /api/v1/me/sharing`**
  (user-only route, admin → 403) + toggle in the web-team employee space.
- **k-anonymity guard** on `/api/v1/admin/analytics`: empty sections +
  `k_anonymity{required, active_users, blocked}` block if activity is
  insufficient; explanatory card on the dashboard side.

### Changed — C38

- **`top_users` (named) → `top_users_shared`**: opt-in participants
  named + anonymous aggregate `N participants`; removal of the
  client-side « Anonymiser » checkbox (pseudo-protection).
- **`/api/v1/admin/users`**: `totals: null` without consent — the
  Employés page becomes an enrollment management view.
- Tests: +5 unit (92 total on the aggregator), admin integration
  rewritten on the ADR-0015 contract (k blocking, opt-in, admin 403).


### Added — C37 Web demo mode (real-engine fixtures)

- **Demo mode outside Tauri**: the deployed web demo now serves
  fixtures precomputed by `sobria-estimator` (seed 42, N = 10⁴) instead
  of empty screens — 204 Monte-Carlo results (34 models × 2 methodologies
  × 3 sizes), M9 catalog, M12 datacenters, M15 dashboard labeled
  « (démo) ». Lazy loading: the desktop bundle embeds nothing.
  « Mode démo » banner + `mode_demo` assumption on every result.
  Reproducible generator: `tools/fixturegen/`. See
  `docs/archive/chantiers/C37-mode-demo-web.md`.
- **`tools/fixturegen/`**: out-of-workspace crate generating the fixtures
  from the real engine (models/methodologies/estimates JSON).

### Changed — C37 a11y, typography, voice

- **WCAG AA contrasts**: `--ivory-3` 3.94:1 → 5.0:1 (#83817a);
  `--ivory-4` 2.01:1 → 3.1:1 (#62605a, decorative only).
- **rem typographic scale** (respects the browser setting),
  body 14 → 15px, floor 12px (308 sizes of 8-11px found in
  41 files, SVG annotations ≥ 10px).
- **Dynamic version** in the rail (`__APP_VERSION__` via Vite define,
  was hardcoded « v0.3.0 ») + suffix LOCAL/DÉMO.
- **Unified formal address** and end-user-oriented error messages
  (no more `cargo run -p sobria-app` visible; label « Application
  de bureau requise »).
- **Home eyebrow**: « Module M2 » → « Module M1 » (M2 out of v1.0 scope).
- **Playwright e2e suite** migrated from the "no-mock contract" to the "demo
  contract" (30 passed, 2 documented skipped); header of
  `playwright.config.ts` rewritten.

### Fixed — C37

- **`leaflet` missing from `web/package.json`** (used by M12/M20,
  installed locally without `--save`): `npm ci` would have failed in CI.
  Added with `@types/leaflet`, lock regenerated.
- **Dead hydration in dev**: the runtime import of `package.json` in
  the layout violated `server.fs.allow` (403) → version injected at
  compile time by `define`.


### Added — Claude Opus 4.8 + extension catalog sync

- **Claude Opus 4.8** added to the Rust `MODEL_REGISTRY` registry
  (`crates/sobria-estimator/src/model_presets.rs`) — parameters extrapolated
  from the same size class as Opus 4.7 (2000B, ε_decode P50 = 50 000 mJ/tok,
  embodied P50 = 0.5 gCO₂eq/req), `release_date: "2026-05-15"`.
- **`MODEL_PRESETS` extension** synchronized 8 → **34 presets**:
  full C34 catalog (Claude Opus/Sonnet/Haiku 4.x + 3.7 Sonnet,
  GPT-5.5 / 5.5 Thinking / 5.5 Pro / o3, Gemini 3.5 Flash / 3.1 Pro /
  2.5 Pro, Llama 4 Scout/Maverick + 3.3 70B, Mistral Medium 3.5 / Small 4
  / Large 3, DeepSeek V4 Pro / R1, Grok 4, Qwen 3.6 Plus, Phi-4
  Reasoning + Vision) + 8 models from 2024 kept as `deprecated` for
  audit ledger reproducibility and historical test parity.
- **Content script mappings** extended to recognize the new
  models on vendor UIs (claude.ai: Opus/Sonnet/Haiku 4.x;
  chatgpt.com: GPT-5.5 / o3; chat.mistral.ai: Medium 3.5 / Large 3 /
  Small 4). `specific-first` mode enforced to avoid greedy matching
  of `String.includes()`.

### Fixed

- **Latent ChatGPT model matching bug**: `text.includes('gpt-4o')`
  matched before `gpt-4o mini` (`Object.entries` insertion order).
  Reordered specific-first in the 3 content scripts. Without the fix,
  a user on ChatGPT « GPT-4o mini » had it calculated as GPT-4o.

### Fixed — UI visual polish (ghost tokens + redundancy)

- **Unreadable DatacenterPicker**: replacement of nonexistent design tokens
  (`--accent` → `--lime`, `--ink-mute` → `--ivory-3`) and of
  `color: var(--ink)` (near-black text on a dark background) → `--ivory`. The
  dropdown panel goes from a translucent background (`--surface`) to an
  opaque one (`--ink-2` + `--border-hi`). The selected value and the options
  are now readable.
- **M12 filters / drill-down** (`CountryDrillDown`, `DatacenterDrillDown`,
  `DatacenterFilters`): invalid `--ink-mute` in `color-mix` borders
  → `--ivory-3`.
- **Sankey M20**: TWh sub-labels `#72706a` → `#b8b4ac` (contrast) + size
  8 → 9 px.

### Changed

- **M1 — equivalences**: removed the `EquivalenceCarbon` text line
  (« ≈ cm voiture · s streaming · s LED ») that duplicated the visual
  tiles « Pour mettre cela en perspective ». The component is still used
  in M15/M25.
- **A11y**: focus affordance (lime `:focus-within`) on the
  M9/M12/M20 filter rows; hover/focus states on the DatacenterPicker search;
  `.kpi` spacing of the model detail drawer (M9) 2 → 5 px.

## [0.9.0] — 2026-05-20 — 2026 catalog + input modalities + overhead (C34)

> Scientific credibility of the engine. The catalog catches up with 2 years of
> LLM releases (Claude 4.x, GPT-5.5, Gemini 3.x, Llama 4, DeepSeek V4,
> Mistral Large 3, Grok 4, Qwen 3.6, Phi-4 reasoning). The engine
> finally models non-text modalities (vision / document / audio)
> and system overhead (system prompt + tools + memory + thinking
> tokens of reasoning models). This release comes before the C33 website:
> an engine cannot be marketed with a 2024 catalog.

### Added — C34.2 Catalog of 25 presets

- **17 new 2025-2026 presets** in `model_presets.rs`:
  Claude Opus 4.7 (2026-04-16), Claude Sonnet 4.6 (2026-02-17), Claude
  Haiku 4.5, Claude Opus 4, Claude Sonnet 4, Claude 3.7 Sonnet, GPT-5.5
  (2026-04-23) + GPT-5.5 Thinking + GPT-5.5 Pro, OpenAI o3, Gemini 3.5
  Flash (2026-05), Gemini 3.1 Pro, Gemini 2.5 Pro, Llama 4 Scout (MoE
  109B/17B), Llama 4 Maverick (MoE 400B/17B), Llama 3.3 70B, Mistral
  Large 3 (MoE 675B/41B), Mistral Small 4, Mistral Medium 3.5, DeepSeek
  V4 Pro (MoE 1600B/49B), DeepSeek R1 (MoE 671B/37B), Grok 4, Qwen
  3.6-Plus, Phi-4 Reasoning, Phi-4 Reasoning Vision 15B.
- **3 new enums**: `ModelFamily` (anthropic / open_ai / google_deep_mind
  / meta_ai / mistral_ai / deep_seek / xai / alibaba / microsoft / other),
  `ArchitectureKind` (dense_transformer / moe { experts, active_experts }
  / mamba / hybrid), `VisionPricing` (open_ai_tiles / anthropic_area /
  gemini_native / llama_patches).
- **`VisionPricing::tokens_for(count, width, height, high_detail)`** —
  formulas sourced from official vendor docs (OpenAI Vision pricing,
  Anthropic vision, Gemini API vision, Llama 4 multimodal blog).
- **`available_models_filtered(include_deprecated: bool)`** — new
  helper to filter out deprecated models in the UI.
- **35 new tests** on `model_presets.rs` (vision_pricing,
  release_date validation, source_url HTTPS, MoE active < total, etc.).

### Added — C34.3 Input modalities + system overhead

- **`sobria_core::InputModality`** tagged union: `text` | `vision_low
  { image_count }` | `vision_high { image_count, avg_width, avg_height }`
  | `document { page_count }` | `audio_input { duration_seconds }`. With
  `default_token_count()` preset-agnostic (generic fallback formula).
- **`sobria_core::ContextOverhead`** struct: `system_prompt_tokens`,
  `tools_definition_tokens`, `memory_tokens`, `thinking_tokens_p50`.
  Physically separates `total_input()` (prefill) and `total_output()`
  (decode/thinking).
- **`EstimationRequest`** extended with `modalities: Vec<InputModality>`
  and `overhead: ContextOverhead`, `#[serde(default)]` for compat with audit
  ledger v0.8.x.
- **Module `sobria_estimator::effective_tokens`** — bridge between
  `InputModality` (sobria-core) and `VisionPricing` (sobria-estimator).
  Computes `effective_in = tokens_in + overhead.total_input() + Σ modalities`
  and `effective_out = tokens_out + overhead.total_output() + auto_thinking`.
- **Automatic thinking tokens** for reasoning models: if the user
  did not provide `overhead.thinking_tokens_p50`, the engine adds
  `output × geometric_mean(P5, P95)` of the `thinking_token_multiplier`
  of the preset (sources: o3 system cards, DeepSeek R1 arXiv 2501.12948,
  Anthropic extended thinking doc, Gemini 2.5+ thinking doc).
- **AFNOR Monte-Carlo + EcoLogits engines** integrated into
  `effective_tokens()` — modalities contribute to the energy pipeline.

### Added — C34.4 M1 UI modalities + technical details

- **`ModalitiesPanel.svelte`** (new component under the Composer):
  - Toggles Image / Document PDF / Audio (disabled if the preset does not
    support the capability — N/A badge).
  - Sub-forms: image_count + radio low/high resolution +
    avg_width × avg_height for Vision High; page_count for Document;
    duration_seconds for Audio.
  - Simple/Expert mode (persisted in localStorage). Simple hides
    technical details by default.
  - **Blue reasoning badge** displayed automatically if the model has
    reasoning_capable, showing "~N tokens thinking (P50, ratio P5×-P95×
    P5-P95)".
  - Technical details (`<details>`): system_prompt_tokens (pre-filled
    from preset.default_context_overhead_tokens on each model
    change), tools_definition_tokens, memory_tokens, and thinking
    override for reasoning models. Footer shows total overhead.
  - **± 50 % disclaimer** in yellow on the summary, in line with the
    decision by Thibault (cf. brief C34 §5).
- **`+page.svelte` M1**: integrates `ModalitiesPanel`, state
  `modalities: InputModality[]` and `overhead: ContextOverhead`, estimation
  payload enriched only if non-empty (audit ledger compat).

### Added — C34.5 UI M9 capabilities + vision viz + reasoning viz

- **Section "Capacités"** in `ModelDetailDrawer.svelte`:
  - Header with release_date.
  - Vision (blue) / Audio (purple) / Reasoning (lime) /
    MoE+Dense (amber/gray) / Deprecated (coral) badges.
  - Cap row "Paramètres actifs" for MoE (active drives energy).
  - Cap row "System prompt typique" with ± 50 % disclaimer.
  - **Vision viz** (if vision_capable): 3 cases computed via a JS mirror
    of `VisionPricing::tokens_for` — 512×512 low, 1024×1024 high,
    2048×2048 high + link to the vendor doc.
  - **Reasoning viz** (if reasoning_capable): table of P5 / P50
    (geomean P5×P95) / P95 of the multiplier + example "500 tokens output
    → ~N tokens thinking (P50)".
  - Button "Model card officielle" linking to source_url.
- **`ModelPresetDto` + `ModelDetailDto`** extended with capabilities,
  vision_pricing tagged JSON, and MoE meta (experts / active_experts).
- **TS mirror `VisionPricing`** typed tagged union in `api.ts`.

### Added — C34.6 ReproductionCase modalities

- **5 new `ReproductionCase`** in `validation/cases.rs`:
  - `c34-vision-gpt4o-2-high-images-fr`: GPT-4o + 2 images 1024×1024
    high res. (1530 vision tokens via OpenAI tiles formula).
  - `c34-document-gpt4o-5-pages-fr`: 5-page PDF → 5500 generic
    tokens (1100 tokens/page).
  - `c34-audio-gpt4o-30s-fr`: 30 seconds of audio → 300 tokens (Whisper
    rate 10 tokens/s).
  - `c34-reasoning-o3-complex-fr`: o3 reasoning, auto-thinking ~12k
    tokens (geomean √(5×30) ≈ 12.25 × 1000 output).
  - `c34-overhead-claude-3-7-fr`: Claude 3.7 Sonnet + claude.ai overhead
    2000 tokens system + auto thinking (geomean √(2×50) = 10 × 500 output).
- **`ReproductionCase` extended** with `modalities: &'static [InputModality]`
  and `overhead: ContextOverhead` (zeros by default, legacy cases).

### Changed — C34 Old 2024 presets deprecated

- The 8 presets from 2024 (`gpt-4o`, `gpt-4o-mini`, `claude-3-5-sonnet`,
  `mistral-large-2`, `mistral-medium-3`, `llama-3-1-70b`, `llama-3-1-8b`,
  `gemini-2-0-flash`) are marked `deprecated: true`. Kept for
  historical reproducibility of the audit ledger (past estimates
  remain valid). The C32.4 vendor_disclosures (Mistral × ADEME,
  Meta location/market-based, Google Gemini) are preserved intact.

### Changed — C34.3 Rename `Modality` → `ModelDomain` (breaking)

- **Breaking**: `sobria_core::Modality` renamed to `ModelDomain`.
  Semantics unchanged: qualifies the **whole model** (LLM/Image/
  Audio/Video). The rename resolves the conflict with the new
  `InputModality` (the type of a prompt's INPUT).
- The `Model.modality: ModelDomain` field remains accessible under the
  same field name. Only the type declaration is renamed.

### Methodology

- **System overhead disclaimer**: « Estimation overhead système ± 50 %
  — basée sur leaks publics et reverse-engineering interfaces vendor
  (Claude.ai, ChatGPT app, Gemini app). À surcharger en mode Expert si
  vous connaissez votre valeur exacte. » Visible in M1 tooltips and
  M9 doc.
- **Vision token sources**:
  - OpenAI : <https://platform.openai.com/docs/guides/vision/calculating-costs>
  - Anthropic : <https://docs.anthropic.com/en/docs/build-with-claude/vision>
  - Google Gemini : <https://ai.google.dev/gemini-api/docs/vision>
  - Meta Llama : <https://ai.meta.com/blog/llama-4-multimodal-intelligence/>
- **Thinking token sources**:
  - OpenAI o3 / GPT-5.5 Thinking: system cards (P5=5, P95=30)
  - DeepSeek R1 / V4: <https://arxiv.org/abs/2501.12948> (P5=8, P95=25)
  - Claude extended thinking: Anthropic doc (P5=2, P95=50, configurable)
  - Gemini 2.5+ thinking: Google doc (P5=3, P95=25)
  - Phi-4 reasoning: Microsoft Research technical report (P5=5, P95=15)
- **Cutoff**: shortlist validated by 10 official WebSearches on
  2026-05-20. Zero ghost presets (unreleased models excluded).
  Documentation: `docs/archive/chantiers/C34-shortlist-models-validated.md`.

## [0.8.0] — 2026-05-17 — Product clarity (C32)

> Intermediate release centered on UX and messaging before the
> data.gouv.fr v1.0 application. No new business module — but the product
> finally tells its story for the 5 personas (student / pro_tech /
> enterprise / public_sector / researcher) identified in
> `docs/archive/AUDIT-PRODUIT-2026-Q3.md`. Average UX clarity score
> 6/10 → target 8/10 minimum across the 5 personas.

### Added — C32.1 Messaging + labels + cleanup

- **README v2 rewritten**: value proposition highlighted at the top, a
  « Sobr.ia, c'est quoi ? » block in plain language without jargon, a
  « Pour qui ? » section with 5 persona cards + specific docs links. The
  technical content (methodologies, architecture) moves down after
  the cards. The « Méthodologies disponibles » section is preserved
  intact (C32.0 audit safeguard).
- **5 persona guides** in `docs/personas/` (`student.md`,
  `pro-tech.md`, `enterprise.md`, `public-sector.md`, `researcher.md`)
  — ~1 page each with pitch, top use cases, relevant modules and
  copy-paste quickstart.
- **C32.0 product audit** versioned in
  `docs/archive/AUDIT-PRODUIT-2026-Q3.md`.

### Changed — C32.1 Clarified module labels

- **M9** « Référentiel modèles » → **« Bibliothèque de modèles »**.
- **M17** « Empreinte projet » → **« Datasheet scientifique »**.
- **M22** « Rapport CSRD / AGEC » → **« Rapport réglementaire (CSRD/AGEC) »**.
- Nav rail (`+layout.svelte`), `<title>` titles + breadcrumbs of the
  M9 / M17 / CSRD report pages aligned with the new labels.
- Playwright specs `m9.spec.ts`, `m17.spec.ts`, `rapport-csrd.spec.ts`
  updated for the new titles.

### Removed — C32.1 Cleanup of IDs in the user UI

- Removed the `<span class="module-id">{m.toUpperCase()}</span>` from
  the onboarding (Bundle step × 2) and `/parametres` (3 modules
  sections). The IDs `M1`, `M3`, etc. no longer leak into the user
  UI but remain **unchanged** in URLs (`/m3`, `/m13`) and
  in Rust/TS code (`ModuleId::M3`).
- **M14 « À propos » removed from the 5 default persona bundles**
  (`sobria_core::Persona::default_modules`). M14 remains accessible via
  the « À propos » rail entry (`alwaysVisible: true`). New
  invariant test `no_persona_bundle_contains_m14` that seals the decision.
- **TS bundles aligned with Rust**: pre-existing drift (M11, M24, M5,
  M21, M2, M6, M10, M16, M19, M23 in the TS but not in the Rust) resolved
  in favor of the Rust canon.

### Added — C32.2 Educational onboarding + narrative thread

- **New step 2 « Sobr.ia en 30 secondes »** inserted between Splash
  and Persona Picker: 4 sentences in plain language + SVG equivalence
  diagram (1 typical Mistral Large 2 prompt = 1.14 g CO₂eq
  ≈ 5 m by car / 5 min of LED TV). « Continuer » button + discreet
  « Passer cette étape » link with persistence `localStorage.sobria_welcome_skipped=true`.
- **Onboarding renumbering**: 4 → 5 steps (Splash 1, Intro 2,
  Persona 3, Bundle 4, Ready 5). Progress dots, eyebrows, section IDs,
  focus management: all consistent.
- **« Et après ? » banner** under the M1 Atelier result after the
  1st submitted prompt. 3 contextual cards: Comparer ce modèle →
  /comparer · Voir votre usage cumulé → /m15 · Fixer un budget mensuel
  → /m25. Permanent dismiss via `localStorage.sobria_narrative_banner_dismissed=true`.
- **« Pourquoi ces modules ? » tooltips** in the onboarding (Bundle
  step) and `/parametres` (Section 2 « Vos modules actifs »).
  Persona-specific lookup (~30 entries) via the new function
  `moduleReason(persona, id): string | undefined`. Native `title`
  attribute — works on hover AND on keyboard focus.

### Added — C32.3 Human equivalences + guided Team Mode

- **`<EquivalenceCarbon />` component**, reusable, in
  `web/src/lib/components/`. Props
  `{ gco2eq, waterMl?, energyWh?, compact? }`. 4 canonical factors
  exported as constants: `CAR_GCO2EQ_PER_M=0.2` (ADEME Base Empreinte
  2025), `STREAMING_GCO2EQ_PER_MIN=0.25` (Shift Project Lean ICT 2019),
  `SHOWER_L=8` (ADEME eco shower), `LED_WH_PER_MIN=1`. Adaptive format
  by magnitude: cm/m/km car, s/min/h streaming and LED,
  mL/%/showers. Source tooltip per equivalent + « ordre de
  grandeur » disclaimer.
- **M1 Atelier integration**: under `ResultBlock`, straight line.
- **M15 Dashboard integration**: under the 3 stat-cards CO₂/energy/water,
  bluish box « Cette période, vous avez consommé l'équivalent
  de : … ».
- **M25 Eco-budget integration**: under each budget progress bar,
  `compact` mode. Displays the equivalence according to the budget's indicator.
- **« Activer Mode Équipe » panel** in `/parametres` (visible
  only if `!$teamStore.url`). Primary lime button opens a
  **3-step `aria-modal` dialog**: (1) Download the binary →
  GitHub Releases, (2) Initialize the server → 3 copy-paste
  commands, (3) Distribute the codes to employees → admin flow.
- **`docs/operations/team-aggregator.md` expanded**: new
  **TL;DR** block at the top (micro-businesses/SMEs/freelancers vs IT department), renaming
  `## Pour les TPE/PME` → `## Pour les non-IT (TPE/PME, freelances)`
  aligned with C32.0 audit finding #6, warning « si vous n'avez pas
  d'équipe IT, le binaire fait tout tout seul ».

### Added — C32.4 Vendors disclosure

- **New Rust struct `VendorDisclosure`** in
  `crates/sobria-estimator/src/model_presets.rs` with enums
  `VendorScope` (`Training` | `InferencePerPrompt`) and `VendorUnit`
  (`TCo2Eq` | `GCo2Eq` | `Wh` | `MlWater` | `M3Water`). Field
  `vendor_disclosures: &[VendorDisclosure]` added to `ModelPreset`.
- **3 vendor disclosures integrated into `MODEL_REGISTRY`**:
  - **Mistral Large 2** × ADEME × Carbone 4 (2025-08): training
    20 400 tCO₂eq + 281 000 m³ water + inference 1.14 gCO₂eq for 400
    tokens. First complete LCA published by a global vendor.
  - **Gemini 2.0 Flash** (Google paper 2025-08): 0.03 gCO₂eq + 0.24
    Wh + 0.26 mL water for a median text prompt. Methodological note
    « médian Google, sous-estime les requêtes complexes ».
  - **Llama 3.1 70B** (Meta model card 2024-07): training
    location-based 11 390 tCO₂eq vs market-based 0 tCO₂eq, with an
    educational anti-greenwashing note on the REC distinction.
- **Idempotent SQLite Gold v3 migration** in
  `crates/sobria-ingest/src/gold.rs`: `CREATE TABLE IF NOT EXISTS
  vendor_disclosures (id, model_id, vendor, scope CHECK IN (training |
  inference_per_prompt), value, unit, source_url, published_at,
  methodology_note)` + 2 indexes (model_id, vendor).
- **DTO + IPC**: `VendorDisclosureDto` and `VendorComparisonRowDto`
  in `crates/sobria-app/src/dto.rs`. New IPC handler
  `list_vendor_comparison()` that aggregates by manufacturer for the
  M9 table (5 rows: Mistral / Google / Meta / Anthropic / OpenAI).
- **« Données vendor disclosure » box** in
  `ModelDetailDrawer.svelte` (M9 sheet), visible only if the
  model has at least one disclosure. Differentiated cards training
  (purple) / inference (lime), methodological notes, clickable
  source links.
- **Vendor comparison table** on the main M9 page under the hero
  (4 columns: Fabricant / Prompt-level / Training / Source; explicit ✅
  or ❌, « Pas de disclosure officielle » for Anthropic +
  OpenAI).

### Added — C32.5 Zenodo DOI + 5-persona smoke test

- **Zenodo DOI**: GitHub Actions workflow
  `.github/workflows/zenodo.yml` that triggers Zenodo archiving on
  each release. DOI badge added at the top of the README. BibTeX
  « Citation » section ready for academic reproducibility. The
  effective DOI is generated by Zenodo at the first v0.8.0 release
  published on GitHub.
- **5-persona smoke test**: structured runbook in
  `docs/archive/smoke-test-v0.8.0-2026-05.md` that covers the complete
  walkthrough per persona (Splash → 30 sec Intro → Persona → Bundle →
  Ready → 1st prompt → « Et après ? » banner). Serves as a manual
  checklist before GitHub publication.

### Tests

- **`cargo test --workspace`**: 100% green on the touched crates —
  `sobria-core` 49/49 (including new invariant
  `no_persona_bundle_contains_m14`), `sobria-app` 247/247 (+3 vendor
  disclosure tests), `sobria-estimator` 96/96 (+5 vendor
  disclosure tests).
- **`cargo clippy --workspace -- -D warnings`** clean on the
  touched crates.
- **`cargo fmt --check`** clean.
- **`cd web && npm run check && npm run lint`** clean on the
  touched files (pre-existing C29 prettier warnings out of scope).

### Security + privacy

- **No new network flow** introduced by v0.8.0. Local-first mode
  preserved (see ADR-0014). Vendor disclosures stored as
  static data compiled into the binary, no runtime fetch.
- **« Et après ? » banner**: dismiss persisted in localStorage
  only, never sent to the server. Consistent with CLAUDE.md §7.
- **Persona tooltips**: pure client UI, no telemetry.

### Bump versions

- `Cargo.toml` workspace: `0.7.1` → `0.8.0` (propagates to the 11 crates
  via `version.workspace = true`).
- `crates/sobria-app/tauri.conf.json`: `0.7.1` → `0.8.0`.
- `web/package.json`: `0.7.1` → `0.8.0`.
- `web-team/package.json`: `0.7.1` → `0.8.0`.
- `extension/package.json` + `extension/manifest.json`: `0.7.1` →
  `0.8.0`.

## [0.7.1] — 2026-05-16 — Polish Team Mode (C29)

> Patch release that closes the 4 gaps honestly noted in C28:
> Team Mode UI on the Tauri app side (the 8 `team_*` IPCs were ready but
> not wired to the front end), admin reset-password, TLS rotation, and CSRD
> threshold alerts. See `docs/archive/chantiers/C29-v0.7.1-polish-mode-equipe.md`
> and `docs/operations/team-aggregator.md` § « Opérations courantes ».

### Added — C29.1 Team Mode UI (Tauri app)

- **Non-breaking Rust backend**: `team_settings::TeamStatus` extended with
  `last_seen_at: Option<String>` and `estimations_sent: u32`. The IPCs
  `team_ping` and `team_push_estimation` update these fields on
  success. 5 new unit tests + extended tests (244 `sobria-app` tests).
- **TypeScript**: 7 IPC wrappers + types in `web/src/lib/api.ts`
  (`TeamMode`, `TeamStatusDto`, `TeamHealthResponseDto`,
  `TeamEnrollResponseDto`).
- **Store** `web/src/lib/team-store.ts`: typed `writable<TeamState>` +
  `loadTeam()` + optimistic `saveTeamField()` + rollback.
- **UI section « Mode Équipe self-hosted »** in `/parametres` between
  Extension navigateur and Runtime — 4 sub-blocks: Statut (pill
  lime/amber/coral), Configuration (URL + self-signed cert toggle),
  Enrôlement (12-digit code + 8+ password + display name), Dispatcher
  (radios Local/Équipe/Les deux). 2 no-mock Playwright tests green.
- **Ephemeral fingerprint** per session: `crypto.randomUUID()` on the UI side,
  persistence on the Rust side after `/enroll`.

### Added — C29.2 CLI admin reset-password + list

- New subcommand `admin {list, reset-password}` in
  `sobria-team-aggregator`.
- **`reset-password`**: double prompt via `rpassword`, Argon2id PHC hash,
  `last_login_at = NULL`, revocation of **ALL** active admin tokens.
  Displays « Mot de passe de X réinitialisé. N token(s) révoqué(s). ».
- **`list`**: table `id | username | created_at | last_login_at`.
- Storage helpers: `admins::list_all`, `admins::set_password_hash`,
  `tokens::revoke_all_for_admin`. 4 unit tests + 2 integration tests
  (`tests/cli_admin.rs`).

### Added — C29.3 TLS rotation (`serve --regen-cert`)

- New flag `serve --regen-cert`: before binding, backs up
  `cert.pem` + `key.pem` to `*.pem.bak.<unix_ts>` then regenerates via
  `rcgen` (same SANs `localhost`/`127.0.0.1`/`::1` + OS hostname,
  10-year validity). Displays the SHA-256 fingerprint of the new cert.
- Helpers `crypto::tls::regen_self_signed` + `cert_fingerprint_sha256`.
- 4 unit tests + 2 integration tests (`tests/regen_cert.rs`).
- **`docs/operations/team-aggregator.md`** expanded: new section
  « Opérations courantes » with detailed « Rotation TLS » + admin reset
  password + alerts configuration (webhook + SMTP).

### Added — C29.4 Threshold alerts (CSRD)

- **SQLite migration v2** in `storage/schema.rs` (idempotent,
  driven by `PRAGMA user_version`):
  - `alert_thresholds` (id ULID, scope user|team, target_id?, period
    daily|weekly|monthly, gco2eq_max, notify_kind webhook|email|log_only,
    notify_target?, created_by_admin_id, created_at, disabled_at) with
    CHECK XOR on (scope, target_id).
  - `alert_triggers` (id, threshold_id, period_start, period_end,
    observed_gco2eq, triggered_at, notified_at, notify_error) with
    UNIQUE on `(threshold_id, period_start)` — guarantees **1 trigger
    per period** even under concurrency.
- **Module `src/alerts/`**:
  - `periods.rs` — UTC bounds for daily / ISO weekly / monthly,
    leap year safe. 7 unit tests.
  - `store.rs` — typed CRUD on the 2 tables, business validations
    (consistent scope/target, consistent notify_kind/notify_target,
    gco2eq_max > 0). 5 unit tests.
  - `checker.rs` — `check_thresholds_for_user(conn, user_id, now)`:
    lists active thresholds (user + team), computes the observed value, inserts
    trigger on overrun (UNIQUE protects against re-triggering).
    4 unit tests.
  - `notify.rs` — `notify(event, smtp_cfg).await`:
    - **webhook**: POST JSON `reqwest` 5s timeout, documented payload.
    - **email**: `lettre = "0.11"` SMTP (smtp:// + smtps://), optional
      `user:pass@` creds. Config read from KV `config`
      (`smtp_url` + `smtp_from`).
    - **fallback log_only** if SMTP not configured, webhook without URL,
      or email without recipient (graceful degrade required by the brief).
    - 4 unit tests.
- **Admin REST routes** `/api/v1/admin/alerts*`:
  - `POST   /alerts`          : create a threshold.
  - `GET    /alerts`          : list (active + disabled).
  - `DELETE /alerts/:id`      : soft delete (`disabled_at`).
  - `GET    /alerts/triggers` : history (limit 1..500, default 50).
- **Wiring** in `server::api::estimations::handle`: after
  `estimations::insert`, calls `alerts::checker::check_thresholds_for_user`
  under the SQLite lock, drops the lock, then `tokio::spawn`s the
  notification (non-blocking for the client's ack). Persists
  `notified_at`/`notify_error` after sending.
- **UI `/admin/alerts`** (web-team): creation form (scope radio,
  conditional user picker, period radio, gCO₂eq max, notify_kind
  + conditional target), thresholds table with « Désactiver » button,
  history of the last 50 triggers (timestamp, threshold, observed,
  notification badge OK/erreur/en cours). New entry in the admin
  nav.
- **Integration tests** `tests/integration_alerts.rs`:
  - End-to-end CRUD (POST scope=user without target_id → 400, POST
    team OK, DELETE soft + idempotent).
  - **Real webhook via wiremock**: 10 estimates exceeding a "team daily"
    threshold of 5g → 1 trigger inserted, webhook POST received by the
    mock (`expect(1..)` + `verify().await`).

### Changed

- `team_settings::TeamSettingsStore::clear_session` now also purges
  `last_seen_at` + `estimations_sent` (clean rotation at logout).
- `team_settings::TeamMode` is exported on the TS side as `'local' | 'team' | 'both'`.
- `IpcErrorCode` (`web/src/lib/api.ts`) extends the list of
  team codes (`no_url`, `bad_request`, `unauthorized`, `http_error`,
  `transport`, `storage`).
- Idempotent SQLite migration v1 → v2: C28 (v1) databases only
  reinstall the v2 DDL (alert tables), not the v1 DDL.

### Tests

- `sobria-team-aggregator`: **106 tests passing** (88 unit + 18 integration,
  vs 81 + 12 in v0.7.0).
- `sobria-app`: +5 `team_settings` tests → 244 tests passing (+0 regression).
- `web` (Playwright): +2 `parametres-mode-equipe` no-mock tests passing.
- `cargo clippy --workspace -D warnings` clean, `cargo fmt --check` clean.
- `cd web && npm run check && npm run lint` clean (on the touched
  files; the other warnings are pre-existing).
- `cd web-team && npm run check && npm run lint` clean.

### Security

- Argon2id PHC everywhere for hashes (admin reset, refresh tokens).
- No OpenSSL: rustls + ring + rcgen.
- Email notifications via `lettre` with rustls backend (no native-tls).
- Webhook payload does **not** contain user prompts (only
  threshold aggregates). Compliant with CLAUDE.md §7.
- `log_only` fallback when SMTP not configured → no crash, clear log
  with `notify_error`.

## [0.7.0] — 2026-05-16 — Self-hosted Team Mode (C28)

> Standalone Rust binary `sobria-team-aggregator` deployable by a
> company on its infrastructure (admin workstation, NAS, internal VPS).
> **No Sobr.ia cloud is involved.** See
> `docs/archive/chantiers/C28-mode-equipe-self-hosted.md`, ADR-0013 Phase 2, and
> `docs/operations/team-aggregator.md`.

### Added — C28.1 Bootstrap + TLS

- **New crate** `crates/sobria-team-aggregator/` (HTTPS binary ~15 MB,
  rustls + ring, no OpenSSL). CLI `init` + `serve`:
  - `init`: creates `team.sqlite` (full v1 schema, WAL), generates a
    self-signed TLS cert via rcgen (SANs `localhost`/`127.0.0.1`/`::1`/hostname of the OS,
    10-year validity), sets a 32-byte JWT signing key, creates the initial admin
    Argon2id PHC.
  - `serve`: axum 0.7 + axum-server + tokio-rustls, `/health` route,
    tracing middleware. `serve --regen-cert` planned for v0.7.1.
- **SQLite schema v1**: `admins`, `enrollment_codes`, `users`, `tokens`,
  `estimations`, `config`. STRICT tables, `CHECK ((user_id IS NULL) <>
  (admin_id IS NULL))` on `tokens` for role XOR. Idempotent migration
  via `PRAGMA user_version`.
- **Argon2id PHC** everywhere (admin/user passwords, hash of the codes, hash of the
  refresh tokens). Aligned with the C27 v0.6.0 pattern.

### Added — C28.2 Auth + core REST API

- **JWT HS256 24h** + **refresh tokens 7d** in the format `<ulid>.<uuid_v4>`
  (selector+verifier — O(1) lookup by PRIMARY KEY then Argon2id verify).
  Rotation at each `/refresh` (old one revoked before issuing the new one).
- **`/api/v1` routes**: `POST /enroll` (12-digit code + password + fingerprint),
  `POST /login` (admin OR user), `POST /refresh` (rotation), `POST /estimations`
  (user auth, payload compatible with the v0.6.0 extension camelCase), `GET /me/usage`
  (aggregated totals of the user).
- **ApiError → IntoResponse**: clean HTTP mapping (401/403/409/400/500)
  + JSON body `{ error, code }` + structured logs.
- **CLI `code`**: `create N --ttl-days 7`, `list`, `revoke <id>`. Codes
  of 12 digits from the OS RNG, hashed with Argon2id, displayed in plain text ONLY ONCE.

### Added — C28.3 Admin API + SQL analytics

- **Admin routes** under `/api/v1/admin/*` (RequireAdmin → 401 without Bearer,
  403 if role=user): `GET /users` (list + LEFT JOIN totals), `POST /codes`
  (clamp count ∈ [1, 500]), `DELETE /codes/:id` (idempotent soft delete),
  `GET /analytics?from&to&group_by=day|week|month` (4 aggregated sections).
- **`storage/analytics.rs`**: 4 pure SQL functions (`time_buckets`,
  `top_models`, `top_users`, `method_breakdown`) with `strftime` for
  time bucketing.

### Added — C28.4 Embedded Svelte dashboard

- **New project `web-team/`** (SvelteKit 2 + adapter-static + Svelte 5
  runes + strict TypeScript). 5 pages: `/login` (3 tabs admin / employé
  / s'enrôler), `/admin/dashboard` (4 cards + 3 SVG charts + breakdown),
  `/admin/codes`, `/admin/users`, `/user/dashboard`.
- **Manual SVG charts** (`LineChart`, `BarChart`, `DonutChart`) — no
  Plot/D3, ~3 KB each gzip. Savings ~200 KB. System fonts (no embedded woff2)
  → -300 KB additional. Final bundle 201 KB.
- **Client auth**: JWT access token in memory (not localStorage — XSS),
  refresh in sessionStorage, auto retry on 401, rotation honored.
- **`rust-embed` + SPA fallback**: `web-team/build/` embedded via
  `#[derive(RustEmbed)]` (features `include-exclude` + `debug-embed`).
  Cache `_app/immutable/*` 1 year, `index.html` SPA fallback. Inline « Bundle
  non buildé » page when compiled without frontend.

### Added — C28.5 Aggregated team CSRD + PROV-O + CSV exports

- **3 admin routes** `POST /api/v1/admin/exports/{csrd|prov-o|csv}` →
  `application/pdf`, `application/ld+json`, `text/csv` with
  `Content-Disposition: attachment`.
- **CSRD**: reuses `sobria-export::generate_report` (PDF visually
  identical to the desktop report). Converts each team `estimations` row
  into an `AuditEntry` shim; synthesis `[P50×0.85, P50×1.15]` when P5/P95
  are missing.
- **PROV-O**: team-specific variant (different from the audit-ledger sidecar).
  `@graph`: `prov:Bundle` + `prov:SoftwareAgent` + N `prov:Agent` (users,
  anonymizable → `fingerprint = null`, `displayName` → `Employé #N`) +
  M `prov:Activity` (`prov:wasAssociatedWith` → user).
- **CSV**: RFC 4180 UTF-8, 13 columns. Stable `Employé #N` aliases per
  `user_id`.

### Added — C28.6 Team Mode in extension + Tauri app

- **Extension**:
  - `src/content/shared/team-storage.ts` (chrome.storage wrapper:
    server URL, mode `local|team|both`, tokens, user_id, fingerprint).
  - `src/lib/team-client.ts` (REST + Bearer JWT + automatic refresh rotation
    on 401, typed `TeamApiError`).
  - **Options page**: « Mode Équipe self-hosted » section (URL + ping +
    enroll + radio dispatch + logout). Visible warning on self-signed cert
    (user must accept the cert in another tab — limitation of
    `chrome.fetch`).
  - **`service-worker.ts` dual-dispatch** on `estimation_submitted`:
    native bridge (mode=local|both) + team server (mode=team|both),
    best-effort on each destination.
- **Tauri app**:
  - `crates/sobria-app/src/team_settings.rs` (SQLite KV in
    `referentiel.sqlite`).
  - `crates/sobria-app/src/team_client.rs` (reqwest + rustls + opt-in
    `accept_invalid_certs`). `ClientConfig` snapshot pattern so as not to
    hold a `MutexGuard` across `.await`.
  - **8 IPC commands**: `team_status`, `team_set_url`, `team_set_mode`,
    `team_set_accept_invalid_certs`, `team_ping`, `team_enroll`,
    `team_logout`, `team_push_estimation`.
  - Auto activation of `mode=both` on first enrollment.

### Added — C28.7 Doc + packaging

- **`docs/operations/team-aggregator.md`** (~300 lines): 5 min quickstart,
  micro-business/SME sections (systemd) + IT department (reverse proxy Caddy/nginx + Let's
  Encrypt + firewall + Windows ACLs), SQLite `.backup` backups,
  upgrade, troubleshooting (cert refused, code rejected, refresh expired,
  bundle not built, performance).
- **`crates/sobria-team-aggregator/Dockerfile`** (bonus): multi-stage
  build (node 22 web → rust 1.79 builder → debian:bookworm-slim runtime),
  volume `/data`, expose 8443, healthcheck `/health`, non-root user.
- **`.github/workflows/team-aggregator-release.yml`**: tag trigger
  `v*.*.*`, matrix Linux x86_64 / macOS arm64 / Windows x86_64, build
  Svelte frontend before cargo build, upload assets + sha256 sidecars.
- **Root README**: section « Mode Équipe self-hosted (v0.7.0) ».

### Tests

- **`sobria-team-aggregator`**: 68 passing tests (56 unit + 1 admin
  integration + 3 user API integration + 1 health + 1 embedded UI +
  6 exports integration).
- **`sobria-app`**: +13 `team_*` tests (5 settings + 8 client).
- **Extension**: `npm run check` clean.
- `cargo clippy --workspace -D warnings` clean. fmt clean everywhere.

### Deferred v0.7.1+

- `/parametres → Mode Équipe` page on the `web/` side (Svelte frontend of the
  Tauri app) — the 8 IPCs are ready to be consumed via `invoke`. **→ Delivered
  in v0.7.1 (C29.1).**
- Threshold alerts (`alert_thresholds` table). **→ Delivered in v0.7.1 (C29.4).**
- `admin reset-password` command. **→ Delivered in v0.7.1 (C29.2).**
- `serve --regen-cert` / `--regen-key` command. **→ Delivered in v0.7.1 (C29.3).**

### Security

- No traffic to Sobr.ia — the server belongs to the company.
- No OpenSSL: rustls + ring everywhere (axum, axum-server, tokio-rustls,
  reqwest, rcgen).
- Argon2id PHC for all hashes (passwords, codes, refresh tokens).
- TLS 1.2/1.3 accepted; the self-signed `init` cert is valid for 10 years.
- The data dir must be protected by OS ACLs (see deployment doc).

## [0.6.0] — 2026-05-16 — Browser extension + personal pairing (C27)

### Added — C27.1/2/3/4 WebExtension MV3 extension

- **New logical workspace crate**: `extension/` (strict TypeScript,
  Vite multi-pass, vanilla DOM). Manifest V3 Chrome + Firefox, two
  packaged artifacts: `sobria-extension-chrome-v0.6.0.zip` (~207 KB) and
  `sobria-extension-firefox-v0.6.0.xpi` (~207 KB) — well under the 500 KB
  targeted by the DoD.
- **JS port of the Sobr.ia engine** in `src/lib/empreinte/`: AFNOR/Sobr.ia +
  EcoLogits 2026-01, minimal model presets, parity < 2 % vs Rust
  (golden snapshots in `tests/unit/empreinte.spec.ts`).
- **Prompt detection** on ChatGPT, Claude (claude.ai) and Le Chat
  (chat.mistral.ai) via isolated content scripts (`content-chatgpt.js`,
  `content-claude.js`, `content-le-chat.js`). Shared DOM heuristic
  (`prompt-detector.ts`) + URL → `modelId` mapping.
- **Composer indicator**: circular 36 px badge with progress ring,
  injected to the right of the prompt input. Shows the Sobr.ia score (A-F) on
  mouse hover, the gCO₂eq + Wh + mL estimate after sending.
  No banner at the bottom of the chat (user feedback — *less is more*).
- **Unsupported models**: dedicated badge without a fake estimate (no
  silent fallback to a default preset, in accordance with CLAUDE.md
  §13 « Ne JAMAIS implémenter un calcul scientifique sans source »).
- **Compact popup**: last result, detected model, daily total
  (gCO₂eq + water + energy), with persistence via `chrome.storage.local`.
- **Options page**: methodology choice (AFNOR/EcoLogits), opt-out per
  site (toggles), native bridge opt-in to the desktop app, pairing section
  (see C27.5).
- **Build pipeline**: `scripts/build.js` Vite multi-pass (ES modules for
  background + service worker, IIFE per content script for cross-frame
  isolation). `scripts/package.js` generates Chrome .zip + Firefox .xpi
  with SHA-256 of each archive.
- **Vitest tests**: 55 passing tests (happy-dom), covering `empreinte`
  (24), `prompt-detector` (9) and `badge-injector` (22).
- **Strict CSP**: no `unsafe-eval` or `unsafe-inline`. No
  runtime dependency other than `webextension-polyfill`.

### Added — C27.5 Native bridge + 6-digit pairing + app ingestion

- **New binary `sobria-bridge`** (`crates/sobria-bridge/`): bridge for
  WebExtensions Native Messaging. Reads `stdin` (uint32 LE + JSON UTF-8) from the
  browser, writes `stdout` in the same format. No network port, no
  permanent service — standard OS security.
  - `Ping` → `{ pong: true }` (extension heartbeat).
  - `Estimate{ secret, payload }` → append-only file spool
    `~/.sobria/spool/incoming.jsonl` (10 MB rotation).
  - `Pair{ code }` / `Revoke{ secret }` forwarded in real time to the app
    via **socket forward** (Unix domain socket `/tmp/sobria-bridge.sock`
    on macOS/Linux, named pipe `\\.\pipe\sobria-bridge` on Windows),
    2 s timeout with file spool fallback if the app is not running.
  - Manifest template `manifest/com.sobria.bridge.json.tmpl` + README
    for manual installation (macOS / Linux / Windows × Chrome / Firefox).
  - 8 integration tests (`tests/protocol.rs`) covering read / EOF /
    oversize / write length-prefix / handle ping/pair/estimate / spool
    rotation.
- **`crates/sobria-app::pairing` module**: pure pairing logic via
  6-digit code, 5 min TTL, single-use, constant-time comparison.
  - `PendingCode::new()` — 6 random digits, zero padding (e.g. `042039`).
  - `PairingSecret::new()` — 32 random bytes (OS RNG), **Argon2id** hash
    (PHC string, standard params), stored as is in `secret_hash`
    (the PHC includes the salt — no more separate `salt_hex` column).
    SQLite v3 migration: v2 pairings (SHA-256+salt) are
    automatically revoked at boot, the user re-enters their code.
  - `verify_code(pending, submitted, now)` — constant-time + expire-aware.
  - 14 unit tests.
- **`crates/sobria-app::extension_store` module**: SQLite persistence
  in `referentiel.sqlite`.
  - Tables `device_pairings(id, fingerprint, secret_hash, salt_hex,
    created_at, last_seen_at, revoked_at)` (UNIQUE on fingerprint with
    REPLACE for re-pairing after unpairing) and `extension_events(id,
    pairing_id, ts, method, model_id, tokens_in, tokens_out, gco2eq_p50,
    water_ml, energy_wh, raw_payload_json, ingested_at)` (FK pairing_id).
  - **ULID** as identifier (26 chars Crockford Base32, time-sortable —
    lexicographic order = chronological order, monotonic for B-tree).
  - `drain_spool(store, spool_path)`: reads the file spool, validates the
    secrets, inserts into `extension_events`. Atomic (rename → read →
    remove to avoid losses during the drain).
  - 14 unit tests.
- **`AppState` wiring**: `pending_code: Mutex<Option<PendingCode>>` +
  `extension_store: Mutex<ExtensionStore>` opened from
  `referentiel.sqlite` at boot.
- **7 new Tauri IPC commands** in `crates/sobria-app/src/main.rs`
  + TypeScript mirrors in `web/src/lib/api.ts`:
  - `regenerate_pairing_code()` → `PairingCodeDto`
  - `get_pairing_code_status()` → `Option<PairingCodeDto>`
  - `verify_pairing_code(code, fingerprint)` → `PairingSecretDto`
  - `list_pairings()` → `Vec<PairingDto>`
  - `revoke_pairing(id)` → `()`
  - `list_extension_events(limit, offset)` → `Vec<ExtensionEventDto>`
  - `drain_extension_spool()` → `usize`
- **New UI section** in `/parametres` (between Référentiel and
  Runtime): 2-column grid — 6-digit code in large display format
  (each digit in a lime frame on an ink background, mono font) + TTL
  countdown in `M:SS`, button « Générer / Régénérer un code »; and
  list of active pairings with fingerprint, creation / seen /
  revocation dates, X button on each row to revoke.

### Added — Final patches v0.6.0 (before tag)

- **Auto-install of native manifests by the Tauri app** (new module
  `crates/sobria-app/src/bridge_install.rs`): detection of the browsers
  installed (Chrome, Firefox, Edge, Brave, Chromium) + programmatic
  writing of `com.sobria.bridge.json` manifests at the correct
  OS locations (macOS / Linux / Windows registry key). Svelte dialog
  `aria-modal` with explicit consent, non-blocking toast on first
  post-update launch. IPC `bridge_status`, `install_extension_bridge`,
  `uninstall_extension_bridge`. The scripts `crates/sobria-bridge/scripts/`
  remain as a fallback for custom setups.
- **Real-time socket forward bridge ↔ app** (`crates/sobria-app/src/bridge_server.rs`
  + extension of `crates/sobria-bridge/src/lib.rs`): Unix domain socket
  on macOS/Linux, named pipe on Windows. Pair/Revoke respond in ≤ 2 s
  when the app is running. File spool fallback kept for offline mode.

### Added — Documents

- **ADR-0013** « WebExtension MV3 + native messaging bridge + pairing
  perso/équipe » documents the split between Phase 1 (personal pairing v0.6.0,
  6-digit code + file spool) vs Phase 2 (self-hosted Team Mode
  deferred to C28/v0.7.0) vs Phase 3 (enterprise SSO, multi-device, RBAC —
  v0.8+). Status: **Phase 1 Implemented (v0.6.0, 2026-05-16)**.
- **`crates/sobria-bridge/README.md`**: manual installation guide
  for the native manifest on macOS / Linux / Windows for Chrome / Firefox /
  Chromium / Brave / Edge.
- **`docs/archive/chantiers/C27-extension-navigateur.md`** + **`C27-PROMPT-CLAUDE-CODE.md`**:
  workstream brief + structured prompt used to drive Claude Code.

### Quality gates v0.6.0

- ✅ `cargo test -p sobria-app -p sobria-bridge`: 198 + 8 = 206 passing tests.
- ✅ `cargo clippy -p sobria-app -p sobria-bridge -- -D warnings` clean
  (pedantic enabled).
- ✅ `cargo fmt --check` clean.
- ✅ `cd web && npm run check`: 0 errors, 1 pre-existing warning (tsconfig
  node types).
- ✅ `cd extension && npm run check && npm run lint && npm run test`:
  strict TypeScript + Prettier + 55 passing Vitest tests.
- ✅ Bundles: Chrome 206.7 KB, Firefox 206.8 KB (< 500 KB DoD).
- ⚠️ `npm audit` extension: 6 moderate vulnerabilities in
  **devDependencies** only (vite, vitest, @vitest/mocker) — no
  code shipped to the browser. Accepted for v0.6.0, to be revisited at the next
  major Vite bump (deferred outside C27).

## [0.5.0] — 2026-05-15 — Medallion pipeline activation (C26)

### Added — C26.1 sobria-ingest CLI wiring

- **`LayerRegistry::standard()`** now instantiates `ComparIASource` +
  `RteIrisSource` (Tier 1 of the data.gouv.fr challenge). Before 0.5.0 the
  standard registry was empty (`TODO(sobria-003)`).
- **`sobria-ingest` binary** fully wired: `pipeline run`, `copper`,
  `silver`, `gold`, `validate` call the real registry methods.
  No more `tracing::info!("... (stub)")` stubs.
- **`sobria-ingest::cli` module** (testable) with:
  - `build_context(incremental)` honors `SOBRIA_DATA_ROOT` + `SOBRIA_SEED`.
  - `build_context_with(data_root, seed, incremental)` injectable variant
    for parallel tests.
  - `filter_registry(Option<&str>)` filters the standard registry on a
    source (`--source <id>`) with a clear error if the id is unknown.
  - `standard_source_ids()` introspection of Tier 1 sources.
- **`CopperManifest::verify_files(&self, dir)`**: recomputes the SHA-256 of
  each snapshot file and compares it to the recorded hash. Used by the
  `validate` subcommand to detect corruption.
- **`validate` subcommand** walks `data/copper/<source>/<date>/`,
  loads each `manifest.json`, recomputes the hashes, and reports OK/KO.
  Exit code ≠ 0 if at least one manifest is corrupted.

### Added — C26.2 Versioned Silver schemas + validation on write

- **Module `sobria_ingest::silver_validate`**: validates each Silver entity
  against its versioned JSON Schema 2020-12 schema (`schemas/silver/<entity>-v<n>.json`)
  before returning to the orchestrator. Reads the Arrow schema of the Parquet via
  `polars::LazyFrame::scan_parquet`, checks the presence of the
  `required` columns and the type compatibility (`string` ↔ `Utf8`, `integer` ↔
  `Int*`, etc.).
- **Silver schemas**: 4 schemas embedded via `include_str!`
  (`comparia_conversations`, `comparia_votes`, `comparia_reactions`,
  `rte_iris_consommation`). Each schema requires the systematic lineage columns
  `_copper_sha256` (64-char hex regex) and `_ingested_at` (RFC 3339).
  The RTE IRIS schema additionally requires the `code_iris` geographic unit — unique join key
  with the INSEE reference and the future `datacenter_iris_link` Gold.
- **`CopperSnapshot::from_manifest(snapshot_dir)`** (on `crates/sobria-ingest/src/layer.rs`):
  rebuilds a Copper snapshot from a persistent folder
  (`data/copper/<source>/<YYYY-MM-DD>/`) by loading `manifest.json` and
  verifying the integrity of each file (`verify_files`). Lets the
  `silver` subcommand restart from a frozen Copper without re-downloading.
- **`cli::latest_copper_snapshot(ctx, source_id)`** + **`cli::rehydrate_copper(ctx, registry)`**
  publicly exposed and testable. The latter produces a
  `Vec<StepResult<CopperSnapshot>>` directly consumable by
  `LayerRegistry::run_silver` — functionally equivalent to a `run_copper`
  but read from disk.
- **Tests**: `crates/sobria-ingest/tests/silver_validation.rs` (proptest +
  insta golden snapshots on the 4 schemas) and `tests/copper_rehydrate.rs`
  (round-trip ingest → from_manifest → promote_silver, corruption detection,
  explicit error message when no snapshot exists).

### Added — C26.3 Complete Gold (joins + Gebru datasheet)

- **Tables materialized in `referentiel.sqlite`**:
  - `model_overview(id, name, family, vendor, n_conversations)` — one model
    per row, populated from `comparia_conversations` Silver (distinct
    extraction on `model_id`/`model`/`model_name` + heuristic on
    family/vendor).
  - `scenario_inputs(model_id, country_iso, pue, if_g_per_kwh, wue_l_per_kwh)`
    — denormalized table ready for the M13 simulator.
  - `time_series_mix(region_iso, hour_utc, production_mw)` — placeholder
    for v1, populated in v2 when RTE eco2mix is ingested.
  - `comparison_matrix(model_id, method, co2_g_per_request, computed_at)` —
    empty at init, filled at runtime by the app.
  - `datacenter_iris_link(datacenter_id, code_iris, distance_km, …)` —
    geographic join European datacenter ↔ nearest IRIS unit
    (haversine on IRIS centroids extracted from the RTE Copper GeoJSON).
- **FTS5 index**: virtual table `model_overview_fts(name, family,
  vendor)` for M9 full-text search.
- **Module `sobria_ingest::iris_link`**: IRIS GeoJSON parser, centroid
  computation, WGS84 haversine distance, nearest-neighbor join
  datacenter ↔ IRIS. Tolerant: if the RTE Copper snapshot is missing or
  empty, the table stays empty without breaking the pipeline.
- **Module `sobria_ingest::datasheet`**: Datasheet for Datasets (Gebru
  et al. 2018, doi:10.48550/arXiv.1803.09010) with the 7 mandatory
  sections (Motivation, Composition, Collection, Preprocessing, Uses,
  Distribution, Maintenance) + multi-vocabulary JSON-LD (schema.org +
  DCAT 3 + PROV-O + Sobr.ia vocabulary). Validated on write against
  `schemas/gold/datasheet-v1.json` — any incomplete datasheet makes the
  Gold assembly fail.
- **Gold schema `schemas/gold/datasheet-v1.json`**: JSON Schema 2020-12
  that formalizes the datasheet format (required sections, field types,
  SHA-256 regex on Copper hashes and artifacts).
- **Optional GPG signature**: if the environment variable
  `SOBRIA_GPG_KEY_ID` is set, `MANIFEST.sha256` is signed with a
  detached ASCII signature (`MANIFEST.sha256.asc`) via `gpg --detach-sign`.
  Silently skippable if the variable is not set or if gpg
  is absent from the PATH (CI-compatible without a key).
- **Dependency `sobria-ingest` → `sobria-geoloc`**: added to access
  `all_datacenters()` during the Gold assembly.
- **Tests**: extension of `tests/gold_pipeline.rs` (checks the 5
  new tables + working FTS5 + vendor inference + complete
  Gebru datasheet) + new `tests/datasheet_jsonld.rs` (8 cases covering
  validation against the schema, presence of the 7 sections, Copper lineage
  intact, rejection of a missing section).

### Changed

- `LayerRegistry::standard()` is no longer a simple alias of `new()`: it
  returns the default Tier 1 registry.
- `print_pipeline_report` now displays the paths of the 4 Gold
  artifacts + the timed summary of the pipeline.
- **`promote_silver` ComparIA + RTE IRIS** systematically calls
  `silver_validate::validate_silver` before adding an entity to the result.
  An entity whose Parquet does not comply with its versioned schema makes
  the whole Silver promotion of the source concerned fail.
- **`silver` subcommand**: no longer re-ingests the Copper layer upstream.
  It rehydrates the persistent Copper snapshots via `rehydrate_copper`
  and fails with an explicit message if no snapshot is available
  for a source of the filtered registry.
- **`assemble_gold`**: pre-computes the `datacenter_iris_link` join
  before opening the SQLite transaction, then chains SQLite (with all the
  materialized views + FTS5), Parquet catalog, validated datasheet,
  hashed manifest, optional GPG signature.
- **`GoldArtifacts`** exposes a new field `manifest_signature:
  Option<PathBuf>` that points to `MANIFEST.sha256.asc` when the GPG
  signature succeeded.

### Added — C26.4 DVC orchestration + nightly CI

- **`.dvc/config`**: local remote by default (`.dvc-cache/`) +
  template to switch to S3 (`dvc remote default s3-prod` after
  `dvc remote modify`).
- **`.dvc/.gitignore`**: excludes `cache/`, `tmp/`, `plots/` from the Git repo.
- **`.dvcignore`** extended: ignores `target/`, `node_modules/`, `dist/`,
  `build/`, `.svelte-kit/`, `.vite/`, test fixtures, rendered
  notebooks, etc.
- **`.gitignore`**: adds `.dvc-cache/` (the DVC local remote must
  never be pushed to GitHub).
- **`docs/operations/dvc.md`**: operator guide (~150 lines) with
  quick start, stages table, retention policy referenced in
  ADR-0009, instructions to switch to an S3/HTTP remote, and FAQ
  (difference with Git LFS, role of `dvc.lock`, bit-for-bit reproducibility
  check, `dvc: command not found` troubleshooting).
- **Workflow `.github/workflows/dvc-nightly.yml`**: daily cron job
  03:00 UTC + manual trigger (`workflow_dispatch` with input
  `force`). Steps: checkout, install Rust + Python + DVC + dvc-s3,
  conditional configuration of the S3 remote and GPG key (skip if secrets
  are absent), `dvc pull`, `cargo build --release`, `dvc repro`,
  `validate`, `dvc push`, upload of the Gold artifacts (30 d retention),
  Markdown summary with SHA-256 hashes.

### Changed

- `dvc.yaml`: enriched `desc` annotations for the 3 stages, reproducibility
  guarantee documented. (Stages `copper`, `silver`, `gold`,
  `validate` already present since C01-C04, only the annotations were improved.)

### Added — C26.5 App reconnection to Gold

- **Crate `sobria-referentiel`** now functional (previously an empty
  skeleton):
  - `Referentiel::open(&Path)` opens the Gold SQLite read-only
    (WAL mode `SQLITE_OPEN_READ_ONLY`).
  - `load()` honors `SOBRIA_REFERENTIEL_PATH` (default
    `data/gold/referentiel.sqlite`).
  - `Referentiel::status()` returns a `ReferentielStatus { version,
    snapshot_date, sha256, source_count, model_count, path }` —
    SHA-256 computed in streaming for large files.
  - Tolerant of legacy Gold: if `model_overview` does not exist (Gold
    pre-C26.3), `model_count` returns 0 without error.
  - 5 unit tests + 1 doctest.
- **Tauri IPC** (`get_referentiel_status`, `reload_referentiel`):
  - `get_referentiel_status` never throws an error — encapsulates the
    absence of the file in `available=false` + `message`, so that the
    UI can offer an action rather than crash.
  - `reload_referentiel` invokes `dvc pull` via `std::process::Command`,
    captures stdout/stderr (truncated to 4 KB), returns the resulting status.
    Silently skipped if DVC is absent from the PATH (explicit help message).
- **`crates/sobria-app/src/dto.rs`**: `ReferentielStatusDto` +
  `ReferentielReloadResultDto` (TS mirror in `web/src/lib/api.ts`).
- **Web `web/src/lib/api.ts`**: types + functions
  `getReferentielStatus()` / `reloadReferentiel()`.
- **Settings page `/parametres`**: new "Référentiel" section
  (above "Runtime") with:
  - Status (available/unavailable badge + message).
  - Version, snapshot (FR-formatted), truncated SHA-256 (12 chars + tooltip
    with full hash), number of sources, number of models, SQLite path.
  - "Recharger le référentiel" button that calls `reload_referentiel`,
    displays the result (success / DVC error).
  - Explicit warning callout when Gold is absent.

### Changed — C26.5

- **`crates/sobria-geoloc/data/datacenters.json` → `datacenters_demo.json`**
  to distinguish the **embedded static** data (offline fallback
  M9/M12) from the **dynamic Gold reference** produced by the
  medallion pipeline. The doc-comment of the module makes this explicit. Eventually, a
  Tier 2 source (Cloud Carbon Footprint, Climatiq Datacenters…) will
  dynamically feed this table in Gold; the embedded file will remain
  as a fallback.

### Fixed — workspace clippy

- Several `clippy::pedantic` / `clippy::cast_lossless` /
  `clippy::manual_string_new` / `clippy::format_push_string` /
  `clippy::same_item_push` lints fixed in `sobria-app` and `sobria-export`
  to allow `cargo clippy --workspace --all-targets -- -D warnings`
  to pass cleanly.

---

## [0.4.0] — 2026-05-14 — Multi-methodology catalog (C24 + polish A-H)

### Added — Multi-methodology catalog (workstream C24)

- **Trait `EmpreinteEngine`** in `sobria-estimator`: common interface
  to all the LLM footprint methodologies embedded.
- **Type `EmpreinteMethod`** (sobria-core): stable enum `afnor_sobria` /
  `ecologits` shared by all crates.
- **`EcoLogitsEngine`**: direct port of the official EcoLogits
  2026-01 formulas (Rincé & Banse 2025, doi:10.21105/joss.07471, CC BY-SA 4.0).
  Reproduction validated to ≤ 1 % vs the Python notebook.
- **Page `/methodologies`**: catalog UI letting the user
  choose their default methodology + enable other methodologies as
  reference (« Voir aussi » panel in M1 Atelier).
- **IPC `list_methodologies` + `estimate_for_comparison`**: expose the
  catalog and allow ephemeral comparative computations (not
  logged).
- **Audit ledger migration v1 → v2**: new column `method` on
  `audit_entries`, idempotent. Historical ledgers keep their
  SHA-256 integrity; pre-C24 entries are retroactively labeled
  `afnor_sobria` (the only historical engine).
- **ADR-0012**: complete multi-methodology decision (context, 4
  rejected alternatives, consequences).
- **Notebook `notebook/validation.qmd`**: Python reproduction of the 3
  EcoLogits ReproductionCase, executable end to end.

### Fixed — Audit B (May 2026)

- **`K_DECODE_MJ_PER_TOKEN_PER_B`** recalibrated from `0.025` to `25.0` (missing
  factor 1000). All Sobr.ia estimates produced before 0.4
  were underestimated by a factor of ~1000.
- **RTE eco2mix bug**: `FACTOR = 0.25/1e6` → `0.5/1e6` (the actual step
  is 30 min, not 15 min). Total FR 2023 production goes from 243 TWh
  (wrong) to 487 TWh (within ≈ 2 % of the published RTE 2023 Bilan).
- **Empty `REPRODUCTION_CASES`**: replaced by 3 real cases reproduced to
  ≤ 1 % against EcoLogits Python (Llama 3.1 70B FR/USVA + Mistral Large 2).

### Changed — Multi-methodology consistency (Polish A → H)

- **A** Ledger hygiene: « Voir aussi » panel via `estimate_for_comparison`
  (not logged).
- **B** Discoverability: cross-links `/methodo` ↔ `/methodologies`,
  disambiguated rail labels.
- **C** Visible methodology badges: M1 ResultBlock + Journal column.
- **D** M3 Comparer modèles honors `default_method`.
- **E** Dashboard breakdown by methodology + multi-methodology warning.
- **F** CSRD PDF + PROV-O sidecar trace the methodologies actually used.
- **G** M9 model sheet, M12 datacenter, M13 simulator, M16 forecaster,
  M17 Gebru datasheet → all routed via `engine_for(default_method)`.
  `simulate()` and `forecast_yearly()` now take
  `&dyn EmpreinteEngine`.
- **H** Bump version 0.2.0 → 0.4.0, `/parametres` exposes the
  methodology choice, onboarding mentions the catalog, FR/EN toggle disabled
  (i18n v1.1), cleanup of zombie routes (`/workbench`, `/importer`,
  `/exporter`).

### Removed

- Fake FR/EN toggle in `/parametres` disabled (i18n not implemented,
  will be v1.1).
- Orphan routes `/workbench` (duplicate of `/m9`), `/importer` and
  `/exporter` (ADR-0011 deferred to v1.1+).

### Fixed — UAT bugs I1 → I5 (post-polish)

- **I1 — Dashboard M15 unreadable X axis** (`web/src/routes/m15/+page.svelte`):
  labels « ma18 ma19 ma10 » that overlapped. `shouldShowXLabel` goes
  from a binary threshold to an adaptive stride `ceil(n / 10)`, guaranteeing
  at most ~10 visible labels whatever the window.
- **I2 — Silent « Enregistrer » button in M25 Eco-budget**
  (`web/src/routes/m25/+page.svelte`): Svelte 5 + `<input type="number">`
  + `bind:value` silently coerces to `number`, the later `.trim()`
  made the handler crash. Switched to `type="text" inputmode="decimal"`
  + explicit parsing `parseFloat(value.replace(',', '.'))`.
- **I3 — M20 Territoire FR empty map despite data present**
  (`crates/sobria-geoloc/src/{sankey_fr,territoire_fr}.rs` + UI):
  files present in the repo but absent from the `data_root` at runtime.
  Fix: `const DEFAULT_*_JSON: &str = include_str!(...)` with embedded
  fallback in `load_*()`. Added a « Recharger » button (icon
  `RefreshCw`) on the UI side for manual invalidation.
- **I4 — M12 Datacenters map invisible**
  (`web/package.json` + `web/src/lib/components/m12/DatacenterMap.svelte`
  + `web/src/routes/datacenters/+page.svelte`): root cause = `leaflet`
  and `@types/leaflet` missing from `package.json`. The dynamic import
  failed silently. Fixed dependencies + defensive CSS:
  explicit `.map-wrapper { height: 560px }`, `requestAnimationFrame`
  before `map.invalidateSize()`, `ResizeObserver` on the container,
  `.col-c { display: block }` instead of flex. M20 benefits from the same fix
  (same Leaflet pattern).
- **I5 — Deferred modules invisible in `/parametres`**
  (`web/src/routes/parametres/+page.svelte`): the single « Modules
  disponibles » section split in two — _Modules disponibles_ (can be enabled right
  away) and _À venir en v1.1+_ (lime `v1.1` badge, controls disabled
  for the deferred ADR-0011 modules).

---

## [Unreleased]

### Added — Workstream C10: Onboarding personas + module gating (S6 / S7) — `v0.3.0-onboarding`

> ADR-0010 « Personas et gating modulaire par préférences utilisateur ». Brief `docs/archive/chantiers/C10-onboarding-personas.md`. Tauri + frontend bundle.

#### Rust backend (by Cowork — already merged in `feat(app): C09 Estimer screen + C10 personas/gating IPC`)

- `sobria-core::preferences`: JSON-serializable enums `Persona` (5 values: `student`, `pro_tech`, `enterprise`, `public_sector`, `researcher`) and `ModuleId` (24 values `m1..m25` without reserved `m4`). `Persona::default_modules` mirrors the ADR-0010 bundles. 12 tests guarantee: M1 in all bundles, no duplicates, M4 absent, serde round-trip, bundles ⊆ `ModuleId::all`.
- `sobria-app::dto::AppPreferencesDto` (persona + enabled_modules + onboarded + lang) + `defaults()` = `ProTech` bundle (cf. ADR-0010 §"Onboarding non-bloquant").
- `app_preferences` table created in `referentiel.sqlite` (4 keys: `persona`, `enabled_modules`, `onboarded`, `lang`).
- 2 IPC commands: `get_app_preferences` (returns defaults if empty) and `set_app_preferences` (persona/modules/lang validation + transactional UPSERT).

#### SvelteKit frontend (workstream C10.2)

- `web/src/lib/api.ts`: added `Persona`, `ModuleId`, `AppPreferencesDto` (snake_case types mirroring Rust) + `getAppPreferences()` / `setAppPreferences(prefs)`.
- `web/src/lib/preferences.ts`: strictly typed `writable<PreferencesState>` store with a `loaded` flag, `loadPreferences()` at boot, optimistic `savePreferences()` + IPC rollback, helpers `defaultModulesFor`, `moduleLabel`, `moduleDescription`, `moduleCategory`, `personaLabel`, `personaTagline`, `personaEmoji`. Catalog of the 24 modules + 5 personas, without LocalStorage persistence (CLAUDE.md §13).
- **`/onboarding` route (4 steps, Svelte 5 runes)**:
  1. _Splash_ — logo + italic tagline + mission; auto-advance 3 s or click « Continuer ».
  2. _Persona picker_ — 5 cards (`student`, `pro_tech`, `enterprise`, `public_sector`, `researcher`) + « choisir à la carte » link.
  3. _Bundle_ — checkboxes pre-checked for the persona (8-11 mod.) + collapsible « + Plus de modules disponibles » section (24 - bundle), live counter.
  4. _Premier prompt_ — illustrated preview of the M1 workshop with an animated lime tooltip on the model selector, « Terminer » button (sets `onboarded=true` + `window.location.replace('/')`) + discreet « Passer cette étape » link.
- **Layout guard** in `+layout.svelte`: `onMount` → `loadPreferences()`; if `!onboarded && pathname !== '/onboarding'` → `window.location.replace('/onboarding')`. Outside Tauri (IPC unavailable), no redirect — the rail stays displayed with all modules (shell mode).
- **Rail filtered by `enabled_modules`**: each rail entry carries a `data-module-id` (m1..m25). `visible()` hides the non-enabled entries once the preferences are loaded. Persistent lime « + Ajouter des modules » button at the bottom of the rail → `/parametres`.
- **`/parametres` route rewritten**: 5 sections — _Persona courant_ (with a 5-button selector + `aria-modal` confirmation dialog before replacing the bundle), _Modules activés_ grouped by category (Estimation / Visualisation / Reporting / Pédagogie), unchecked _Modules disponibles_, _Réinitialiser & langue_ (« Refaire l'onboarding » button + FR/EN radio to prepare C12), _Runtime_ (read-only via `meta_info`). All writes go through `savePreferences` (optimistic + rollback).
- **Route guards**  placed on M1 (`/`) and M13 (`/simuler`): `$effect` reactive to the store; if `preferences.loaded && !enabled_modules.includes(MODULE_ID)` → `window.location.replace('/?disabled=mXX')`.
- **« module désactivé » banner** on `/`: `?disabled=mXX` → coral dashed with link « → Activer dans Paramètres ».
- **`/simuler` fix**: `moduleId` updated from `M4` (reserved) to `M13` (Simulateur « Et si...? »), workstream label changed to C11.

#### Checks

- `npm run check`: 0 errors (1 pre-existing warning on `node` types).
- `npm run lint` (Prettier + ESLint): clean.
- 4 Playwright tests `tests/onboarding.spec.ts` (no-mock contract, browser context): splash → persona picker → Étudiant bundle pre-checked at 8 modules, expanding « + Plus de modules » on the Enterprise bundle (M22/CSRD checked), « Terminer » attempt outside Tauri shows the error banner, route guard `/?disabled=m13` shows the coral banner.
- `tests/parametres.spec.ts` updated for the new UI (persona/modules/runtime) + `/simuler` stub now M13 instead of M4.

#### Non-goals (deferred)

- Bundle « partager mon bundle avec un collègue » → C11+.
- Full EN translation → C12.
- Interactive tutorial beyond the step 4 tooltip → v1.1 backlog.
- Multi-user mode → v1.1 backlog.

---

### Added — Workstream C09: Tauri integration + desktop UI v0.2 (S6) — `v0.2.0-estimer`

#### IPC wrapper + design system v2

- `web/src/lib/api.ts`: 6 typed functions mirroring `crates/sobria-app/src/dto.rs` (`metaInfo`, `listModels`, `estimatePrompt`, `verifyAudit`, `listAuditEntries`, `exportAuditNdjson`). `SobriaIpcError extends Error` + strict `isTauriContext()` — no mock, no fallback (CLAUDE.md §13).
- Adoption of design system v2 (`sobr-ia-design-system/` Claude Design): palette `ink #0a0d0b` / `lime #c5f04a` / `ivory #f0ece3`, **Instrument Serif italic** (display) + **Geist** (UI) + **JetBrains Mono** (numbers), 4 px grid, 150-300 ms ease-out animations.
- 8 self-hosted WOFF2 files in `web/static/fonts/` (latin + latin-ext only, ~155 kB total), CSP `default-src 'self'` respected.
- Font licenses documented in `docs/LICENSES-FONTS.md` (SIL OFL 1.1 + SHA-256 per file).
- Design tokens in `web/src/app.css`, atomic components (Composer, ResultBlock, HypothesisBlock, ComingSoon).

#### Delivered screens

- **Estimer (M2)** route `/`: editorial hero, composer (custom model selector + prompt textarea + automatic tokens + datacenter + mix), result with CO₂eq P50 hero metric + **Monte-Carlo SVG distribution from `bins`** (option A1, Gaussian fallback), 3 side indicators (energy + water + metals), meaningful equivalents, hypotheses drawer, clickable ledger signature, cross-screen CTA → Comparer.
- **Journal d'audit (M7)** route `/journal`: paginated table (limit 50), entry detail drawer, integrity verdict, NDJSON export via `@tauri-apps/plugin-dialog`, query param `?focus=N` from Signature.
- **Workbench (M3)** route `/workbench`: multi-select filters (provider / calibration / openness) + client-side full-text search + 5-column sort (`aria-sort`), model sheet drawer with clickable sources, « Estimer avec ce modèle » CTA (`?model=<id>`).
- **Comparer (M5)** route `/comparer`: selection of 2-8 models, parallel `estimatePrompt` fan-out (Promise.allSettled), **M15-style editorial verdict** (« X est Y× plus sobre que Z » + `−N %` delta), per-model cards with normalized CO₂eq / energy / water bars + P5-P95 interval, hypotheses drawer + sources + ledger link. Accepts `?prompt=...&tokensOut=...&model=...` from Estimer.
- **Méthodologie (M8)** route `/methodo`: reference formula, cross-validation (3 cards), FR/EN glossary of 15 terms, normative references (AFNOR SPEC 2314, ISO 21031, ITU-T L.1410, GHG Protocol, ADEME), selective bibliography. Sticky TOC on desktop / horizontal on mobile.
- **Paramètres** route `/parametres`: runtime via `meta_info` IPC (app version, seed, N, audit path, data root) + clipboard copy buttons, « Préférences à venir » section (Thème, Langue, Seed perso, Télémétrie) with their workstream.

#### Documented stubs (4 routes)

- `/simuler` (M4 · C10), `/importer` (M10 · C11), `/territoire` (M9+M12 · C12), `/exporter` (M6 · C10).
- Shared `<ComingSoon>` component: amber « Module Mx · en chantier » eyebrow + dashed status card with an explicit list of the **expected Rust IPCs** + covered CDC functional requirements (EF) + planned workstream.

#### Methodology

- Logged Monte-Carlo distribution: **option A1 adopted** — `IndicatorValue.bins: Option<DistributionBins>` in `sobria-core`, 50 equi-width bins over the 10⁴ draws, present in the audit payload (~600 B / entry). The front end renders the actual log-normal right tail; Gaussian fallback if bins are absent.
- FR tokenizer: `prompt.length / 3.3` (FR median 3.0-3.5) with a tooltip pointing to the v0.3 BPE tokenizer.
- Automatic unit rescale: `pickScale(p50, baseUnit)` picks kg/g/mg/µg/ng depending on the P50 — inter-percentile consistency guaranteed, no more `0,00 g` for small models.
- Number format everywhere with `maximumSignificantDigits: 3` (Intl FR).
- Comparator verdict based on the worst/best CO₂eq P50 ratio (no weighted composite score, mathematically wrong as long as the indicators are collinear in engine v0.2).

#### Tooling

- ESLint: `typescript-eslint` meta-package (replaces the `@typescript-eslint/eslint-plugin` + `parser` splits).
- Prettier: `.prettierignore` + `.svelte-kit` / `build` / `node_modules` / `test-results` / `playwright-report` ignored.
- `npm run clean`: cleans `.vite`, `.svelte-kit`, `build`, `test-results`, `playwright-report` (cross-OS rimraf) — workaround for the dirty Vite cache after a deps change (cf. lucide migration).
- `npm run clean:full`: same + deletes `node_modules` + reruns `npm install`.
- `lucide-svelte` (legacy Svelte 3/4) → `@lucide/svelte` (official Svelte 5 runes).
- Playwright: 12 « no-mock contract » tests, `workers: 1` to avoid races on the shared Vite dev server.

#### Tauri (Rust side, by Cowork)

- Tauri runtime enabled (`tauri::Builder` + `generate_context!()` + icons via `npx tauri icon` + `tauri-plugin-dialog`).
- 6 IPC commands registered (`meta_info`, `list_models`, `estimate_prompt`, `verify_audit`, `list_audit_entries`, `export_audit_ndjson`).
- `IndicatorValue.bins` (option A1) + `bin_samples()` helper with `N >= 10` & `min < max` guard.
- Capabilities `dialog:default`, `dialog:allow-save`, `dialog:allow-open` in `crates/sobria-app/capabilities/default.json`.

#### Checks

- `npm run check`: 0 errors / 3 784 files.
- `npm run lint`: OK (prettier + eslint).
- `npx playwright test`: **12 passed (5 s)** — « no-mock » contract locked on the 6 functional screens + 4 smoke tests on the stubs.

See `docs/archive/chantiers/C09-RETROSPECTIVE.md` for the details of the methodological decisions and the C10+ roadmap.

### Added — Framing and bootstrap (S0-S1)
- Initial framing pack: CDC v1.2, 9 ADRs, 12-week roadmap, S0 brief, source catalog, textual UI mockup.
- Medallion architecture Copper / Silver / Gold (ADR-0009).
- Module M12 — French territory (IRIS mapping, ComparIA × RTE IRIS cross-referencing).
- Technical bootstrap: Cargo workspace, GitHub Actions CI, DVC pipeline, `scripts/bootstrap.sh`.

### Added — Workstream #1: Foundation medallion pipeline
- `sobria-core`: fundamental types (`Datacenter`, `EmissionFactor`, `EstimationRequest`, `EstimationResult`, `Hypothesis`) + strict validation (`UncertaintyInterval::new` guarantees p5 ≤ p50 ≤ p95).
- `sobria-ingest::hash`: streaming SHA-256, file verification, tested RFC 6234 vectors.
- `sobria-ingest::manifest`: `CopperManifest` format v1, versioned JSON Schema, save/load (async).
- `sobria-ingest::download`: streaming HTTP `Downloader` with exponential retry on 5xx, on-the-fly hash verification, cached hit, wiremock mocks.
- `sobria-ingest::lineage`: `CopperRef`/`SilverLineage`/`GoldLineage`, JSON-LD output compatible with PROV-O + schema.org/Dataset.
- `sobria-ingest::layer`: `DataLayer` trait enriched with `health_check` and `depends_on` (default impls), switched to `IngestResult<T>`.
- `sobria-ingest::registry`: `LayerRegistry::run_full_pipeline` actually orchestrated, per-source error handling (no fail-fast), `PipelineReport` with `failed_sources()`.
- Strict JSON schema `schemas/copper/manifest-v1.json` (HTTPS only, SHA-256 64 hex).
- Tests: ≥ 50 unit tests + 2 `proptest` properties (`UncertaintyInterval` invariants, guarantee that the Copper hashes are preserved in `GoldLineage`).

### Added — Workstream #2: ComparIA source
- `ComparIASource`: full implementation of the `DataLayer` trait for Compar:IA (Beta.gouv / Ministry of Culture).
- Download of the 3 ComparIA Parquet files (conversations 682 MB, votes 733 MB, reactions 3.4 GB) from data.gouv.fr.
- `promote_silver` via polars 0.46 (in `spawn_blocking`) with systematic `_copper_sha256` + `_ingested_at` enrichment.
- 3 permissive versioned Silver schemas: `comparia_{conversations,votes,reactions}-v1.json`.
- 3 end-to-end integration tests (wiremock + synthetic Parquet).
- HTTP loopback allowed in the manifest for tests (strict HTTPS in production).

### Added — Workstream #3: RTE / NaTran / Teréga IRIS source
- `RteIrisSource`: second official source of the challenge (territorial).
- Download of the consumption CSV (90 MB) + GeoJSON of the IRIS geometries (91 MB) from ODRÉ.
- CSV promoted to Silver via `polars::LazyCsvReader`, GeoJSON kept in Copper only (direct consumption by the M12 module later).
- Permissive Silver schema `rte_iris_consommation-v1.json`.
- 3 integration tests (registry, end-to-end, GeoJSON preserved).

### Added — Workstream #4: Gold assembly
- `sobria-ingest::gold`: `assemble_gold` orchestrates 4 write functions.
- `referentiel.sqlite` (WAL mode) with 3 tables: `sources`, `silver_entities`, `lineage` (FK + index).
- `analytics.parquet` (tabular catalog of the Silver entities, readable by DuckDB).
- `datasheet.jsonld` (PROV-O + schema.org/Dataset, pretty-printed).
- `MANIFEST.sha256` in standard `sha256sum` format.
- `LayerRegistry::run_full_pipeline` calls `assemble_gold` at the end of the chain; `PipelineReport.gold_artifacts: Option<GoldArtifacts>` exposes the paths.
- Tests: 4 unit (assemble, sqlite, parquet, empty case) + 1 integration end-to-end 
