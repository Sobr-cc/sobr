# ADR-0027 — The estimator in the browser (WebAssembly)

- **Status**: Accepted (2026-10-31). Thibault answered yes to the five
  points below (SOB-269).
- **Date**: 2026-10-31
- **Deciders**: Thibault (decision); CTO recommendation
- **Technical context**: Foundations (SOB-262), after the research SOB-95 and
  the Decision SOB-246
- **Amends**: ADR-0021, point 3, §1 (the web-context row) and §3 (the CSP),
  once accepted. See Amendment 1 of ADR-0021.
- **Related**: ADR-0004 (Monte-Carlo, N = 10⁴), ADR-0017 (web demo),
  ADR-0019 (wave 3, data delivery), ADR-0023 (use-case layer; its issue 11
  is SOB-263)

> **Summary.** On October 31, Thibault chose that on the web the Understand
> pages compute their numbers in the browser, with the estimator compiled to
> WebAssembly (WASM) (SOB-246). This ADR says how:
>
> * **A thin wrapper crate** exposes one function, `call(command, args)`,
>   with JSON in and out. It mirrors the Tauri `invoke`.
> * **The build:** `cargo` and `wasm-bindgen-cli`, at the version that
>   `Cargo.lock` holds. No `wasm-pack`.
> * **Loading:** the generated files are not committed. Vite bundles them
>   with a content hash, every build that ships includes them, and the app
>   loads them only in the web context.
> * **Running:** on the main thread.
> * **Two amendments to ADR-0021:**
>   * the CSP adds `'wasm-unsafe-eval'`;
>   * in the web context, the Understand commands are answered by the
>     module instead of being refused.
> * **A parity test** holds the module to the native numbers.
>
> Thibault answered yes to the five points below (SOB-269).

## Decisions of Thibault

On October 31, Thibault answered yes to all five recommendations
(SOB-269). The options he did not choose are kept for the record.

1. **The two amendments to ADR-0021.** Recommended: yes, they follow from
   SOB-246.
   - §3, CSP: `script-src` adds `'wasm-unsafe-eval'`. It allows compiling
     WebAssembly and nothing else: `eval` and `new Function` stay blocked.
   - §1, web context: the Understand commands are answered by the module
     instead of being refused with `not_available_here`. The other commands
     are still refused.
2. **`wasm-bindgen` as a direct dependency, and `wasm-bindgen-cli` as the
   build tool.** Recommended: yes.
   - `wasm-bindgen` 0.2.121 is already in `Cargo.lock` through `chrono`, so
     the lockfile gains no new crate. The build gains one tool and one CI
     job.
   - Alternative 1: `wasm-pack`, a wrapper around the same tool. Since the
     rustwasm organization was archived (August 2025), it is maintained by
     one person outside the `wasm-bindgen` organization.
   - Alternative 2: no `wasm-bindgen` at all. That means changing the
     estimator: inject the clock instead of `Utc::now()`, and a custom
     `getrandom` backend.
3. **The desktop app carries the module without loading it.** Recommended:
   yes.
   - ADR-0021 point 1 keeps one build of `web/` for every context. Tauri
     compresses the assets it embeds, so the cost is about the gzipped size:
     about 100 KB today (§4).
   - Alternative: a second build of `web/` without the module, which means
     two artifacts to test.
4. **The static demo keeps its fixtures for now.** Recommended: yes, and
   revisit once SOB-265 has shipped.
   - Alternative: the demo uses the module now. Visitors then get real
     numbers for any input instead of the closest grid point.
   - Cost of the alternative: ADR-0017 rule 1 changes, and the demo's host
     must send the same CSP.
5. **Where the Estimate page lives on the web, and for whom.** Recommended:
   - **At `/estimer`.** In the web context, `/` keeps the redirect by role
     of ADR-0021 §4 (admin to `/organisation`, member to `/suivi`, otherwise
     `/connexion`). On desktop, `/` stays the Estimate page.
   - **For signed-in people only, both roles.** The organization server
     serves its members, and ADR-0021 §5 has no personal space on the web.
     An estimate stores nothing, so it is not a personal space.
   - Alternative: `/` shows Estimate on the web too, which changes the
     redirect table of ADR-0021 §4.
   - Alternative: no Estimate page on the web; only Compare, models,
     datacenters, territory and methods.

## Context and problem statement

**The research (SOB-95)**

* No backend that the Understand pages call ever receives the prompt. The
  browser turns it into a token count, and the request type has no prompt
  field (`crates/sobria-app/src/dto.rs:195-216`).
* Thibault chose to keep even the token counts, the model choice and the
  timing in the browser (SOB-246).

**What already compiles for WASM** (checked on `wasm32-unknown-unknown`)

* `sobria-core` and `sobria-geoloc` compile as they are.
* `sobria-estimator` needs only a JavaScript backend for `getrandom`: the
  chain is `rand` 0.8, then `rand_core` 0.6, then `getrandom` 0.2.17.
* The engines draw from a fixed seed (`engines/factory.rs:16-21`) and never
  call OS randomness, so that backend is linked but never used.
* `Utc::now()` reads the JavaScript `Date`, through chrono's default
  `wasmbind` feature.

**Size and speed** (scratch build, `opt-level = "z"`)

| Measure | Value |
|--|--|
| Module after `wasm-bindgen` (estimator, presets, datacenter records, JSON, the `call` export) | 240 KB raw, 91 KB gzipped, plus 8 KB of JavaScript glue (2 KB gzipped) |
| Not in it yet | the DTOs and the territory files (`territoire_fr.json` alone is 114 KB raw) |
| Raw `cargo` output, before `wasm-bindgen` | 585 KB, of which 174 KB is binding metadata that `wasm-bindgen` strips |
| One AFNOR estimate (Monte-Carlo, N = 10⁴) | 3 to 4 ms native, 4 to 5.5 ms in WASM (Node 24, three rounds of 200 runs) |
| One EcoLogits estimate | under 0.1 ms in WASM |

SOB-246 quoted 620 KB raw and 170 KB gzipped. Those figures were `cargo`'s
raw output, measured before `wasm-bindgen` processed it.

**What a scratch build checked (October 31)**

The test used a scratch crate, `wasm-bindgen-cli` 0.2.121 and the
real `web/` build (Vite 5).

* **Bindings.** `wasm-bindgen --target web` produced the bindings, and
  `call` answered from Node with an AFNOR interval. An unknown command came
  back as `not_available_here`.
* **Bundling.** Vite picked the module up through `import.meta.glob` and
  emitted `_app/immutable/assets/sobr_wasm_bg.<hash>.wasm`.
* **Without the generated files,** the same build succeeded, shipped no
  `.wasm`, and the page reported the module missing.
* **CSP, in Chromium through Playwright.** `'unsafe-inline'` stood in for the
  start-up script hash.
  * With `script-src 'self' 'unsafe-inline'`, `WebAssembly.instantiateStreaming()`
    failed with a CSP `CompileError`.
  * With `'wasm-unsafe-eval'` added, the module ran, streamed as
    `application/wasm`.

**Constraints**

* **CLAUDE.md §13:** no new dependency without an ADR.
* **§8, frugality:** prefer the smallest build and the fewest tools.
* **§6, reproducibility:** seed 42 everywhere.
* **ADR-0021:** one build of `web/` for desktop, server and demo (point 1),
  and a strict CSP on the HTML the server sends (§3).

**How the server serves files today**

* It embeds the build with rust-embed, which stores files uncompressed.
  Its include list has no `*.wasm` (`embedded_web.rs:26-34`), so a `.wasm`
  path returns 404 today (`63-69`).
* It sends the content type from `mime_guess`, which maps `.wasm` to
  `application/wasm`.
* It caches `_app/immutable/` for a year (`81-95`).
* It does not compress responses: tower-http is built with `trace` and
  `cors` only.

**How Tauri serves files:** it compresses the assets it embeds with its
default `compression` feature.

## Decision drivers

1. Nothing about an estimate leaves the browser.
2. The same numbers as the desktop app for the same inputs and the same
   release.
3. One build of `web/` (ADR-0021 point 1): what CI tests and measures is
   what ships.
4. Local front-end work that does not need the module does not need the
   Rust toolchain either.
5. The fewest new dependencies, tools and lines of glue.

## Options considered

### A. The interface between JavaScript and the module

| Option | Glue | Types | Verdict |
|--|--|--|--|
| A1. One export, `call(command, args_json) -> result_json` | one hand-written TypeScript function | the hand-written DTOs of `$lib/api` (ADR-0021 §2) | **chosen** |
| A2. One export per command, with generated TypeScript types (`serde-wasm-bindgen`, `tsify`) | generated, per command | generated from Rust, a second source of truth | rejected: a new dependency, and it contradicts ADR-0021 §2 (DTOs stay hand-written) |

A1 mirrors `invoke(command, args)`. The web branch of `$lib/api` has the
same shape as the desktop one.

### B. The build tool

| Option | Risk | Verdict |
|--|--|--|
| B1. `cargo build --target wasm32-unknown-unknown`, then `wasm-bindgen-cli` at the version in `Cargo.lock` | the CLI refuses a version mismatch, which is a feature | **chosen** |
| B2. `wasm-pack` | one maintainer since August 2025; it wraps B1 and adds nothing we need | rejected |
| B3. No `wasm-bindgen`: raw `extern "C"` exports and hand-written glue | changes the estimator (the clock, a custom `getrandom` backend) | rejected |

No `wasm-opt` (binaryen) for now: one more tool, for a gain not yet
measured on this module.

### C. Where the generated files go, and how the app loads them

| Option | Effect on `web/` | Caching | Verdict |
|--|--|--|--|
| C1. Generated into `web/src/lib/wasm/pkg/` (ignored by git and by the linters), picked up by Vite through `import.meta.glob` | `vite build`, `svelte-check` and the unit tests work with or without the files | Vite emits them with a content hash under `_app/immutable/`, cached for a year | **chosen** |
| C2. Generated into `web/static/wasm/`, loaded by URL at run time | no effect | fixed URL: the server would need ETags | rejected: needs server work for caching |
| C3. Committed to git | no effect | as C1 | rejected: a binary in git at each change |

### D. Where it runs

* **D1. The main thread.** Chosen: about 40 ms for an 8-model comparison.
* **D2. A Web Worker.** Only if a measurement on a slow laptop shows the
  page freezing.

### E. The static demo

* **E1. Keep the fixtures (ADR-0017).** Recommended for now: point 4.
* **E2. Use the module.**

### F. How parity is checked

| Option | Verdict |
|--|--|
| F1. Node runs the module and writes the raw JSON it returns; a Rust test compares that with the native golden file, using the shared comparator | **chosen**: no new dependency and no port of the comparator |
| F2. `wasm-bindgen-test`, running Rust tests inside the module | rejected: a new dev dependency, and the golden files are read from disk |

## Decision

### 1. A thin wrapper crate

* **Position.** A new workspace crate sits on top of the read-only part of
  the use-case layer that SOB-263 (ADR-0023 issue 11) extracts.
  * SOB-263 moves those use cases, their DTOs and `IpcError` into a crate
    free of `tauri`, `rusqlite` and `sobria-audit`.
  * The wrapper depends on that crate and on nothing else of the app.
  * The read-only use cases read no person's data, so they take no `Actor`.
    The MCP door of ADR-0023 checks the `reference.read` scope before it
    calls them.
* **Name.** It follows the crate rename (SOB-37); until then, `sobria-wasm`.
  `crate-type = ["cdylib", "rlib"]`, so it also builds natively for its
  tests.
* **Its direct dependencies,** listed here as CLAUDE.md §13 asks:
  * **`wasm-bindgen`**, with no exact pin in `Cargo.toml`. `js-sys` already
    pins it to one version, and `Cargo.lock` holds that version (0.2.121
    today). When a `chrono` or Tauri update moves it, the build script and
    CI follow, because they read the version from `Cargo.lock` (§3).
  * **`serde_json`.**
  * **`getrandom`, for `wasm32` only,** with its JavaScript backend feature.
    `getrandom`'s documentation advises enabling it in the final crate, not
    in libraries.
    * With `getrandom` 0.2 (what `rand` 0.8 uses today), the feature is
      `js`.
    * If `rand` moves to 0.9 or later (`getrandom` 0.3), the feature
      becomes `wasm_js`. Some early `getrandom` 0.3 releases also need the
      `getrandom_backend="wasm_js"` cfg. `Cargo.lock` already holds `rand`
      0.9 and `getrandom` 0.3 for other crates.
    * None of our other crates enables it.

### 2. One function, JSON in and out

* **The exports.**
  * `call(command: &str, args_json: &str) -> String` returns `{"ok":
    <result>}` or `{"err": {"code", "message", "details"}}`.
    * The result has the JSON shape of the matching Tauri command (the same
      DTOs).
    * The error has the shape of `IpcError`.
  * `version()` returns the app version.
* **The web branch of `$lib/api`** calls the module for these commands:

| Command | Answered by | Note |
|--|--|--|
| `list_models`, `list_vendor_comparison`, `list_methodologies` | module | |
| `get_model_detail` | module | `$lib/api` adds the method from the preferences, which the Tauri command reads from SQLite |
| `list_datacenters`, `aggregate_datacenters_by_country` | module | |
| `get_datacenter_detail` | module | method added as above |
| `estimate_prompt` | module, stateless | method added as above; `audit_id` 0, nothing written (SOB-95, question 6) |
| `estimate_for_comparison` | module | already stateless |
| `list_industrial_sites_fr`, `aggregate_industrial_sites_by_region`, `sankey_fr_data` | module | the embedded copy of the files (SOB-248) |
| `get_app_preferences`, `set_app_preferences` | `$lib/api`, in `localStorage` | same DTO, never sent to the server |
| every other command | refused with `not_available_here` | the ledger, projects, datasheets, journal, batch, forecast, scenarios, bridge, team |

`get_industrial_site_fr` (`main.rs:629`) is registered but no page calls it,
so it is not listed.

**No map in the web context.** `/datacenters` and `/territoire` show
Leaflet maps that load CARTO tiles (`DatacenterMap.svelte:147`,
`TerritoireMap.svelte:120`).

* The web CSP (`img-src 'self' data:`, ADR-0021 §3) blocks those tiles.
* Widening it would send the person's IP address and map area to CARTO.

So in the web context these two pages show their tables, figures and the
Sankey, with no map. Self-hosted tiles would be a separate decision.

### 3. Build

* **The script.** `scripts/build-wasm.sh`:
  1. reads the `wasm-bindgen` version from `Cargo.lock`, and stops with the
     install command when the installed `wasm-bindgen` differs;
  2. `cargo build -p sobria-wasm --release --target wasm32-unknown-unknown`,
     with the workspace release profile (`opt-level = "z"`, `lto`,
     `strip`, `panic = "abort"`);
  3. `wasm-bindgen --target web --out-dir web/src/lib/wasm/pkg`;
  4. print the raw and gzipped sizes.
* **Ignore lists.** `web/src/lib/wasm/pkg/` goes in all three lists:
  `.gitignore`, the ESLint ignores (`web/eslint.config.js`) and
  `web/.prettierignore`.
* **The toolchain.** `rust-toolchain.toml` already lists the
  `wasm32-unknown-unknown` target. A Docker stage that does not copy it
  adds the target itself.
* **Local use.** Run the script once before working on the web context
  (`VITE_SOBR_RUNTIME=server`). Other front-end work does not need it.
* **CI and releases, so that what is tested and measured is what ships:**
  * **A reusable workflow, `wasm-build.yml`** (`workflow_call`).
    * It installs `wasm-bindgen-cli` at the `Cargo.lock` version (`cargo
      install --locked`, cached by version).
    * It runs the script and the parity test (§7), and uploads the
      generated files as an artifact of the calling run.
    * An artifact is reachable only from the run that uploaded it, so every
      workflow that builds `web/` calls `wasm-build.yml` first, and its web
      jobs download the files before `vite build`.
  * **`ci.yml`** calls it first. "web — lint + check + build",
    "web — Playwright", "Frugality — binary sizes" and the desktop build (the
    reusable `app-build.yml`, whose Tauri `beforeBuildCommand` runs `vite
    build`) download from that same run.
  * **`app-release.yml`** calls it before `app-build.yml`, and
    **`team-aggregator-release.yml`** calls it before its web build. Their web
    stage moves from `web-team/` to `web/` with SOB-109.
  * **The Docker image** (`team-docker.yml`, `deploy/team/Dockerfile`) cannot
    take a run artifact: its web stage builds inside Docker, and its
    `Dockerfile.dockerignore` keeps `web/` outside the build context.
    * So the Dockerfile gets a `wasm-builder` stage (`rust:1-slim`, the
      `wasm32-unknown-unknown` target, `wasm-bindgen-cli` at the
      `Cargo.lock` version), which runs `scripts/build-wasm.sh`.
    * The web stage copies the generated files from it before `vite build`.
    * SOB-109 makes this change, when it moves the image from `web-team/`
      to `web/`.
  * **Every release build fails** when `build/_app/immutable/assets/` holds
    no `.wasm`: `app-build.yml` on a tag, `team-aggregator-release.yml`, and
    the Docker web stage.
* **`scripts/build-all.sh`.** Its last step,
  `cargo build … -p sobria-core || echo "⚠ Wasm build non disponible"`, is
  a placeholder. SOB-264 replaces it with `build-wasm.sh`, run before the
  web build.

### 4. Loading, and which builds carry it

* **Loading.** A loader in `$lib/wasm/` imports the module lazily through
  `import.meta.glob`, only in the web context. Vite emits the `.js` and the
  `.wasm` with a content hash under `_app/immutable/`.
* **The server.**
  * SOB-109, which makes the server embed `web/`, adds `*.wasm` to the
    rust-embed include list (`embedded_web.rs:26-34`). The server then
    sends it as `application/wasm`, cached for a year.
  * Responses are not compressed today: about 250 KB per version per
    browser. Compressing the server's responses is a separate issue, and
    benefits every asset.
* **Every shipped build of `web/` includes the module (§3),** so the one
  artifact of ADR-0021 point 1 carries it:
  * **the server** embeds it raw, about 250 KB in the binary, more once the
    DTOs and territory files are in;
  * **the desktop app** embeds it compressed by Tauri and never loads it.
    The 20 MB size check (SOB-240) measures that build.
  * The same holds for the demo fixtures today: they are in `web/build`, so
    in the desktop binary, and the lazy import only keeps them from being
    loaded. ADR-0017 rule 2 says they are not in the Tauri bundle; a
    separate docs fix corrects that wording.
* **The real size** is printed by the job of §3 and written in the PR of
  SOB-264. There is no size gate until a first real figure exists.

### 5. Running

* **The thread.** The module runs on the main thread.
* **A panic.** The release profile has `panic = "abort"`, so a panic in the
  module is a WebAssembly trap, outside the `{"err"}` contract.
  * After a trap, the instance is in an unknown state, and the
    `wasm-bindgen` glue keeps that same instance in a module variable:
    calling its init again returns it unchanged.
  * So the loader catches the trap and marks the module broken. That call,
    and every later one, returns `{"code": "internal"}`.
  * The page then asks the person to reload it. The reload creates a fresh
    instance.
  * A panic is a bug, so this path should be rare. If it is not, running
    the module in a Web Worker that is replaced after a trap is the next
    step (below).
* **A Web Worker** is added only if a measurement on a slow laptop shows a
  freeze. It is then a same-origin file, never `blob:` or `data:`. The
  server sends the CSP of §6 on its response too: a worker follows the CSP
  of its own script response, not the page's.

### 6. Security

* **CSP (Amendment 1 of ADR-0021, §3).** `script-src 'self'
  'wasm-unsafe-eval'` plus the hash of the start-up script.
  `'wasm-unsafe-eval'` allows compiling WebAssembly and nothing else.
* **Browser floor for the Understand pages on the web:** Chrome 97,
  Firefox 102, Safari 16. These are the first versions that support
  `'wasm-unsafe-eval'`, per MDN's browser compatibility data. An older
  browser shows the module error. The rest of the app keeps Vite's build
  target.
* **Unchanged:** `connect-src 'self'`, and the module makes no network
  call.
* **The desktop CSP** (`tauri.conf.json`) does not change: the desktop
  never loads the module.
* **The prompt** never enters the module: `$lib/api` passes token counts,
  as today. Before the pages reach the web, SOB-247 takes the prompt out of
  the `/comparer` URL.

### 7. Parity test

**The cases**

* `estimate_for_comparison` and `estimate_prompt`:
  * each method, three models and two sizes of token counts;
  * one datacenter override, one input modality other than text, and one
    context overhead.
* `get_model_detail` and `get_datacenter_detail`, for each method.
* The complete output of the list commands and of the territory and Sankey
  commands, so that the catalog compiled into each build cannot drift.
* The two errors: unknown model, and both token counts at zero.

**The mechanics**

* **The native side.** A Rust test of the use-case layer writes and checks
  the native golden file, with the existing update mode.
  * After SOB-263, the Tauri commands are thin wrappers over the same
    functions, so this golden file also covers the desktop path.
  * The desktop-only part, which reads the method from the SQLite
    preferences, stays covered by the app's characterization tests
    (SOB-188).
* **The WASM side.** Node runs the module on the same cases and writes the
  raw JSON strings it returns, unparsed, so integers and floats keep their
  form.
* **The comparison.** A Rust test compares those strings with the native
  golden file, using the shared comparator (`test-support/golden.rs`).

**The rule**

* Integers and text are compared exactly.
* Floats are compared with a relative tolerance of 1e-9, because `ln` and
  `exp` differ in the last bit between platforms, and WASM brings its own
  math library.
* **The time fields** are masked in this test only: `computed_at` (already
  masked by the comparator) and the request `timestamp` (`Utc::now()`).
  The comparator's global list does not change, so the golden files of
  other crates stay the same.
* **Histogram bin counts** are integers, compared exactly, as the golden
  files already compare them on Linux, macOS and Windows. A sample that
  lands exactly on a bin edge could in theory move one count. If that ever
  happens, the test compares bins with ±1 per bin and an equal total.

**Scope.** Parity holds within one release. The desktop app and the server
are released separately and can run different versions, as for every other
feature. The methods page shows `version()`.

### 8. Data and catalog

* **The model presets and datacenter records** are compiled into both
  builds. The territory and RTE mix files are too, after SOB-248.
* **When the wave 3 delivery ADR (ADR-0019)** changes how the desktop gets
  the model catalog, the module follows the same path in the same change:
  * if the catalog then arrives at run time, the web fetches it as a static
    file from the same origin and passes it to the module;
  * the module itself never fetches anything.

### 9. Issues

* **SOB-263:** the use-case layer, with the WASM-ready constraint.
* **SOB-264:** this crate, the script, the CI job, the loader, the ignore
  lists, the `build-all.sh` fix and the parity test.
* **SOB-265:** the pages in the web context, including `/estimer` per
  point 5. Its Playwright tests run against the built artifact served by
  the server, with its CSP.
* **SOB-42:** detecting the web context. Today `call()` sends every call
  made outside Tauri to the demo (`web/src/lib/api.ts:643-656`), and SOB-42
  adds the web branch, `not_available_here`, and the first Playwright test
  in web mode.
* **SOB-109:** the server embeds `web/`, with `*.wasm` in its include list.
* **SOB-247:** the prompt out of the URL.
* **SOB-248:** the embedded territory files.
* **Filed in Triage with this ADR:**
  * compressing the server's responses;
  * the wording of ADR-0017 rule 2.

## Consequences

**Positive**

* **Privacy:** the Understand pages work on the web, and nothing about an
  estimate leaves the browser.
* **No server work:** no new route and no server CPU (CLAUDE.md §8).
* **Same numbers on desktop and web** for the same release, checked in CI.
* **What CI tests is what ships:** one `web/` build, with the module.

**Negative**

* **New tools:** one direct dependency (`wasm-bindgen`, plus `getrandom`
  for its feature), one build tool (`wasm-bindgen-cli`) and one CI job that
  the web jobs wait for.
* **The same code built twice,** native and WASM, held together by a test.
* **Bigger binaries:** the server binary grows by the module's raw size,
  and the desktop app carries a module it never loads.
* **A CSP exception:** the web CSP gains `'wasm-unsafe-eval'`.
* **A browser floor:** the Understand pages on the web need Chrome 97,
  Firefox 102 or Safari 16.

**Neutral**

* The demo keeps its fixtures until point 4 is revisited.
* The extension keeps its hand-ported TypeScript estimator. Replacing it
  with the module is a separate decision.

## Links

* [SOB-95 proposal: Understand pages on the web without the prompt](https://linear.app/sobrcc/document/understand-pages-on-the-web-without-the-prompt-sob-95-38e15adca854)
* Decision SOB-246. Issues SOB-262 (this ADR), SOB-263, SOB-264, SOB-265,
  SOB-42, SOB-109, SOB-247, SOB-248, SOB-240, SOB-188.
* ADR-0004, ADR-0017, ADR-0019, ADR-0021 (Amendment 1), ADR-0023.
* [Sunsetting the rustwasm GitHub org](https://blog.rust-lang.org/inside-rust/2025/07/21/sunsetting-the-rustwasm-github-org)
  (Inside Rust blog, July 2025)
* [`getrandom`: WebAssembly support](https://docs.rs/getrandom/#webassembly-support)
* [MDN browser compatibility data for `'wasm-unsafe-eval'`](https://github.com/mdn/browser-compat-data/blob/main/http/headers/Content-Security-Policy.json)
