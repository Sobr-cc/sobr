# ADR-0021 — One Svelte app, shipped on desktop and on the web

- **Status**: Accepted (2026-10-30)
- **Date**: 2026-10-30
- **Deciders**: Thibault (decision); CTO recommendation
- **Technical context**: Foundations, Structure milestone (SOB-14)
- **Amends**: ADR-0017, rule 2 (demo activation), once accepted
- **Amendment 1** (2026-10-31, Proposed with ADR-0027): in the web context,
  the Understand commands are answered by the estimator compiled to
  WebAssembly, and the CSP allows WebAssembly. See the section below.

> **Summary.** `web/` becomes the only interface. One build of it is embedded
> both by the desktop app (Tauri) and by the organization server, in place of
> `web-team/`. At start-up the app detects one of three contexts: desktop, web
> served by the server, or the static demo of ADR-0017. On the web, the merge
> covers the organization space only: sign-in, the member home, the
> Organisation section and alerts. The Understand pages stay on desktop until
> a later decision on how they can run on a server without the prompt leaving
> the machine. Organization data goes through one client, `$lib/org`. It
> fetches the server directly in a browser and goes through a Rust proxy on
> desktop, so tokens never reach the webview. The seven web-team pages land
> on screens M2, Z6, Z7, Z9, B2 and A6. The Organisation section is
> admin-only until roles arrive. Six points for Thibault come first.
>
> *Amendment 1 (proposed with ADR-0027): the Understand pages move to the
> web, computed in the browser. See the section of that name.*

## Points to decide

Six questions for Thibault; the recommendations rest on the Decision section.

1. **One build of `web/` for desktop, server and demo**, with the context
   detected at runtime. Recommended: yes. One artifact to test, like the
   single server binary of ADR-0014. Alternative: one build per context, at
   the cost of three artifacts that drift apart.
2. **On desktop, the organization space goes through Rust.** Recommended:
   yes. A Tauri command relays calls to the server, and tokens stay in Rust.
   The webview never sees a token, and its CSP does not have to open to the
   server URL. Alternative: the webview calls the server directly. That
   means opening `connect-src` to any organization URL and keeping tokens in
   JavaScript, as `team_enroll` does today, which the SOB-7 audit flags.
3. **On the web, the merge covers the organization space only**: sign-in,
   member home, Organisation section, alerts. Recommended: yes. The
   Understand section moves to the web only after a separate decision
   (§6, research issue), for four reasons:
   - the prompt travels in the `/comparer` URL today, so it would reach the
     server through the `Referer` header and the logs;
   - the Estimate and Compare pages write to the local ledger;
   - a shared logic crate would be needed;
   - the territory data files are not delivered to the server.

   Alternative: move everything now. The merge would then wait for these
   four pieces of work.

   *Amendment 1 (proposed with ADR-0027) answers the four reasons. See the
   section of that name.*
4. **The Organisation section is admin-only, based on the token's role,
   until roles arrive.** Recommended: yes. The server only knows `user` and
   `admin` (`server/auth/jwt.rs`). The manager role, what a manager sees,
   and the k-anonymity scope that applies to them (ADR-0015, ADR-0016)
   belong to the Organization and identity project.
5. **Web session.** Recommended: keep web-team's current scheme for the merge
   (access token in memory, refresh token in `sessionStorage`), isolated in
   `$lib/org`, and add two things:
   - a sign-out that revokes the token on the server, through the merge's
     only new route, `POST /api/v1/logout`;
   - a refresh coordinated across tabs.

   Move to an `HttpOnly` cookie with SSO. Alternative: the cookie right away,
   which means issuing the cookie and checking the `Origin` header on the
   server.
6. **Merge texts in French only.** Recommended: yes. `web/` has no i18n
   (`svelte-i18n` is installed but unused). SOB-47, SOB-62 and SOB-63 ask for
   French and English, so each would have to set up i18n on its own. Remove
   that criterion from the three issues, and let one dedicated issue set up
   i18n once for the whole app (§6). Alternative: make i18n a prerequisite of
   the merge.

## Context and problem statement

The "App and screen architecture" document (October 30) sets one interface
for everyone. It ships as a desktop app (Tauri) or as a web app served by the
organization server, in the cloud or self-hosted. Administration is part of
it: there is no separate web-team any more.

Today the repo holds two SvelteKit apps (`adapter-static`, SPA mode):

| | `web/` | `web-team/` |
|--|--|--|
| Size | ~33,000 lines, 20 pages | ~2,500 lines, 7 pages and a redirect |
| Embedded by | `sobria-app` (Tauri, `frontendDist: ../../web/build`) | `sobria-team-aggregator` (`rust-embed`, folder `web-team/build`) |
| Data access | `call(cmd, args)` in `src/lib/api.ts`: Tauri `invoke` (55 commands), otherwise the demo fixtures (ADR-0017) | `apiGet` and `apiPost` on `/api/v1/*`: Bearer token, automatic refresh |
| Session | none (local) | access token in memory, refresh token in `sessionStorage` |

Four facts weigh on the decision:

- **The desktop app already talks to the server from Rust.**
  `crates/sobria-app/src/team_client.rs` does the ping, the enrollment and
  the estimate push, with weaknesses (SOB-7 audit):
  - `team_enroll` returns both tokens to the webview;
  - a single token pair is kept, in plain text in `referentiel.sqlite`;
  - changing the server URL does not clear the session.
- **The server is a single binary** in the cloud and self-hosted
  (ADR-0014). It already serves a dashboard (ADR-0013, stage 2) and applies
  the privacy rules on the server side (ADR-0015, ADR-0016). Its errors have
  the shape `{error, code}`, and its roles are `user` and `admin`.
- **The app and the server stay together.** The desktop app, the server and
  `web/` stay in the `sobr` repo, and `web-team/` disappears after the
  merge (ADR-0019).
- **The web demo stays** (ADR-0017): fixtures from the real engine, a
  permanent banner, never in the desktop app.

## Options considered

- **A — Keep two apps.** Ruled out by the architecture decision of October
  30; it doubles the upkeep (color tokens, formatters and API client are
  already copied, SOB-7 audit).
- **B — One codebase, one build per context.** A build variable picks
  desktop, server or demo. The build is simple, but three artifacts must be
  tested.
- **C — One codebase, one build, three contexts detected at runtime, one
  data access layer per context.** The same `web/build` goes into Tauri,
  into the server and onto the demo's static hosting.

## Decision

**Option C.**

### 1. One build, three contexts

| Context | Detection at start-up | App commands (`$lib/api`) | Organization data (`$lib/org`) | Pages offered |
|--|--|--|--|--|
| Desktop | `window.__TAURI_INTERNALS__` present (unchanged) | Tauri `invoke` | Tauri command `org_request`, Rust relay (§5) | all |
| Web | tag `<meta name="sobr-runtime" content="server">` in the HTML the server sends | none: refused with code `not_available_here` | same-origin `fetch`, `/api/v1/...` | organization space (§4) |
| Demo | neither (static hosting) | fixtures (ADR-0017) | none | those of ADR-0017 |

*Amendment 1 (proposed with ADR-0027) changes the "Web" row: the Understand
commands are answered by the WebAssembly module, and the Understand pages
are offered to signed-in people.*

- **Detection is synchronous** and read once at start-up. There is no
  network probe and no race.
- **The server puts the tag in every HTML response**: `/`, `/index.html` and
  the SPA fallback. A page served by the server therefore never falls back
  to the demo, even if the API does not answer: it shows an error.
- **In development**, `VITE_SOBR_RUNTIME=server` forces the web context, and
  Vite's `/api` proxy points to a local server, as for web-team today. Vite
  bakes this variable into the bundle at build time, so the app reads it
  only when `import.meta.env.DEV` is true. The production artifact is tested
  with the tag injected by the server.
- **Amendment to ADR-0017, rule 2.** The demo turns on when the app is
  neither in Tauri nor served by the server; no longer simply "outside
  Tauri". The other rules of ADR-0017 do not change.

### 2. Data access layer

- **Pages never call `invoke` or `fetch` themselves.**
  - `$lib/api` covers app commands, as today.
  - `$lib/org` covers the organization. It replaces `web-team/src/lib/api.ts`;
    its API types come from web-team.
  - DTOs stay hand-written (ADR-0019). Generating them from Rust is a
    separate decision.
- **Each route declares the contexts where it appears** (desktop, web,
  demo). The menu and the route guard follow that declaration.
  - Outside its contexts, a route is hidden, not shown as a disabled
    button, per the architecture document's rule.
  - The demo keeps its banner and its disabled actions with an explanation
    (ADR-0017, rules 3 and 4).
- **`$lib/org`**:
  - maps server errors (`{error, code}` and the HTTP status) to
    `SobriaIpcError`, so the UI keeps a single error type;
  - runs one token refresh at a time, within a tab and across tabs (Web
    Locks), and sends the new pair to the other tabs (`BroadcastChannel`).
    Otherwise a duplicated tab keeps a copy of the old, already revoked
    token. This fixes the sign-out the audit found in web-team; the
    extension has its own client and its own issue.

### 3. The server serves the app

- `sobria-team-aggregator` embeds `web/build` in place of `web-team/build`.
  It is the same artifact as the desktop one.
- **Routing**, in this order:
  1. `/api/*` goes to the API. An unknown API path returns a JSON 404; today
     it returns `index.html` with a 200 (SOB-7 audit).
  2. A file present in the bundle is served. If it is HTML, the server adds
     the `sobr-runtime` tag.
  3. A path that looks like a missing file returns a 404.
  4. Otherwise the server returns `index.html` with the tag.
- **Security headers on HTML responses**:
  - a CSP:
    - `default-src 'self'`;
    - `script-src 'self'` plus the hash of SvelteKit's inline start-up
      script. The server computes it from the embedded `index.html`
      (`build.rs` or at start-up), never by hand. *Amendment 1 (proposed
      with ADR-0027) adds `'wasm-unsafe-eval'`;*
    - `style-src 'self' 'unsafe-inline'` and `font-src 'self' data:`, like
      the desktop CSP (`web/` uses inline styles);
    - `connect-src 'self'`, `img-src 'self' data:`;
    - `frame-ancestors 'none'`;
  - `Referrer-Policy: no-referrer` and `X-Content-Type-Options: nosniff`;
  - `Strict-Transport-Security` when the server serves TLS.

  The web context shows no map, so no third-party tile is loaded.
  Playwright tests in the web context fail on any CSP violation.
- **Build order.** CI builds `web/` before building and testing the server;
  otherwise the bundle is empty and the embedded UI test fails (SOB-7
  audit).
- **One new API route for the merge: `POST /api/v1/logout`**, which revokes
  the refresh token it receives. Otherwise the web pages use the existing
  `/api/v1` API (`/login`, `/enroll`, `/refresh`, `/me/*`, `/admin/*`). The
  CORS rule reserved for the extension stays as it is; the web app is on the
  same origin as the API and does not need it.

### 4. Where each web-team page goes

| web-team page | Target screen | Proposed route in the app | Visible to | Issue |
|--|--|--|--|--|
| `/login`, admin tab | M2 Sign-in (before SSO) | `/connexion` | signed-out person (web) | SOB-47 |
| `/login`, employee tab | M2 Sign-in (before SSO) | `/connexion` | signed-out person (web) | SOB-47 |
| `/login`, enrollment with a 12-digit code | M1 then M2 (the code stands in for an invitation until Z5 and Z6 send a link) | `/connexion#code=…`: the fragment is never sent to the server, and the app clears it from the URL after reading | signed-out person (web) | SOB-47 |
| `/admin/dashboard` | Z7 Tracking dashboard | `/organisation` | admin | SOB-62 |
| `/admin/codes` | Z6 Invitations | `/organisation/invitations` | admin | SOB-62 |
| `/admin/users` | Z9 User management | `/organisation/utilisateurs` | admin | SOB-62 |
| `/admin/users/[id]` | Z9 User management (detail) | `/organisation/utilisateurs/[id]` | admin | SOB-62 |
| `/admin/alerts` | B2 Budgets and alerts, organization scope | `/organisation/alertes` until the V7 design, then inside B2 with the budgets | admin | SOB-62 |
| `/user/dashboard` (usage and sharing) | A6 Home, organization space | `/suivi` (the home), fed by `/api/v1/me/usage` and `/api/v1/me/sharing` | member (role `user`) | SOB-63 |
| `/` (redirect) | none | in the web context, `/` leads to the role's first screen: admin → `/organisation`, member → `/suivi`, otherwise `/connexion`. On desktop, `/` stays the Estimate page. | everyone | SOB-47 |

Rules:

- **The Organisation section shows only if the token carries the `admin`
  role.** It is never shown disabled. A member who opens an
  `/organisation/*` route goes back to `/suivi`.
- **An admin has no usage of their own on the server** (separate table,
  `/me/usage` answers 403). Their first screen is therefore Z7.
- **On desktop, the home of an enrolled member** shows their local summary
  plus an organization block read through `org_request`. This belongs to
  the desktop organization access issue (§6).
- **Routes are proposals for the merge**; the V7 design can rename them
  without reopening this ADR. Route paths are user-facing and stay in
  French, like the app's current routes.

### 5. Sign-in per context

| Context | Personal space | Organization space |
|--|--|--|
| Desktop | no sign-in: local guest, as today (ADR-0013, ADR-0014). The personal account belongs to SOB-35 | enrollment (12-digit code) or sign-in done by Rust; pages go through `org_request` (rules below) |
| Web | no personal space on an organization server (it comes with the personal account, SOB-35) | `/connexion` page (M2): admin (identifier and password), employee (identifier and password), enrollment (code, password, display name), using the current routes `/api/v1/login`, `/enroll` and `/refresh`; sign-out through `POST /api/v1/logout` (point 5) |
| Demo | no sign-in | does not exist |

**Rules of the Rust relay on desktop:**

- `org_request` is never an open proxy:
  - it accepts a relative path only;
  - it joins it to the configured URL, which may contain a sub-path, as
    relative segments;
  - on the resulting, normalized path, it checks that the host is unchanged
    and that the path stays under `<base>/api/v1/me/` or
    `<base>/api/v1/admin/`. So `me/../admin/x` is judged as `admin/x`.
- Each prefix has its token: `/me/*` uses the member's, `/admin/*` the admin
  session's. Without the right token, the call is refused before it leaves.
- Changing the server URL or the certificate setting clears the session.
- An admin session opened on desktop stays in memory and does not replace
  the member's token pair, which is used to push estimates.
- Only one token refresh at a time.
- No command returns a token to the webview, `team_enroll` included.
- The member's tokens stay stored in plain text in `referentiel.sqlite` for
  the merge, against CLAUDE.md §7. The finding is posted on SOB-26 (SOB-7
  audit) and its handling follows SOB-83: an OS keychain needs a dependency
  to be justified.

**Known limits until SSO:**

- An employee's identifier is their browser's fingerprint, which the SOB-7
  audit rates High (collisions between identical machines). It is kept for
  the merge. Fixing it before SSO belongs to SOB-83.
- A member enrolled from the desktop app has an identifier of the form
  `app-desktop-<uuid>`. To sign in on the web, they read that identifier in
  the app's Settings.

SSO and SCIM (final M2 screen, Organization and identity project) replace
password sign-in and these two limits.

### 6. Order and issues

1. **SOB-42 (adjusted)**, serve the app from the server:
   - embed `web/build`, with the `sobr-runtime` tag and the security
     headers;
   - detection of the three contexts, the `$lib/org` client and contexts per
     route;
   - JSON 404 under `/api`, CI build order.

   The Understand pages are no longer "also reachable in a browser". The
   criterion "real data from the server" moves to SOB-47 and SOB-63.
   Instead: the app starts in the web context, the tag and the headers are
   present, and `/` leads to `/connexion`.
2. **SOB-47 (adjusted)**, `/connexion`, `POST /api/v1/logout` on the server,
   web session shared across tabs.
3. **SOB-62 (adjusted)**, Organisation section for admins only (Z7, Z6, Z9,
   alerts).
4. **SOB-63 (adjusted)**, the member's web home.
5. **SOB-64**, remove `web-team/`, unchanged.
6. **New issue SOB-93: open the organization space from the desktop app**:
   `org_request` and its rules (§5), sign-in through Rust, home with an
   organization block. Blocked by SOB-42 and SOB-47.
7. **New issue SOB-94: set up French and English texts in the app** (i18n).
   It carries the criterion removed from SOB-47, SOB-62 and SOB-63 (point 6).
8. **New research issue SOB-95: the Understand section on the web without
   sending the prompt.** It covers:
   - no prompt in URLs;
   - a server route that stores and logs nothing, with bounded compute;
   - a shared logic crate;
   - delivery of the territory files (ADR-0019, wave 3);
   - whether the API is internal or public (ADR-0019 §3).

## Amendment 1 — The Understand pages on the web (2026-10-31)

**Status.** Proposed with ADR-0027, and it applies once ADR-0027 is
Accepted. On October 31, Thibault chose that on the web the Understand pages
compute their numbers in the browser, with the estimator compiled to
WebAssembly (SOB-246). This is the separate decision that point 3 announced.
The text above stays as written; the changes below apply on top of it.

1. **§1, the "Web" row of the table.**
   * **The "App commands" column** said "none: refused with code
     `not_available_here`".
     * The Understand commands listed in ADR-0027 §2 are now answered by
       the WebAssembly module, in the browser.
     * The preferences are kept in the browser.
     * Every other command is still refused with `not_available_here`.
   * **The "Pages offered" column** adds the Understand pages to the
     organization space once SOB-265 ships, for signed-in people.
     * The pages are `/estimer`, `/comparer`, `/modeles`, `/datacenters`,
       `/territoire`, `/methodo` and `/methodologies`.
     * `/estimer` is ADR-0027 point 5: in the web context, `/` keeps the
       redirect by role of §4.
     * `/datasheets` and `/journal` stay on desktop.
2. **Point 3.** The Understand section moves to the web through ADR-0027.
   The four reasons listed there are answered:
   * the prompt leaves the URL (SOB-247);
   * the web pages write no ledger (ADR-0027 §2);
   * the shared logic is the use-case layer of ADR-0023 (SOB-263);
   * the territory files are embedded (SOB-248).
3. **§3, the CSP.** `script-src 'self'` plus the hash of the start-up script
   becomes `script-src 'self' 'wasm-unsafe-eval'` plus that hash.
   * `'wasm-unsafe-eval'` allows compiling WebAssembly and nothing else.
   * If a Web Worker is added, the server sends the same CSP on the
     worker's script response (ADR-0027 §5).

**Unchanged:** the web context shows no map, `connect-src` stays `'self'`,
and the DTOs stay hand-written.

## Consequences

**Positive**

- **One artifact for three contexts.** Administration is usable on desktop
  and on the web, as the architecture document requires, with the same
  server binary in the cloud and self-hosted (ADR-0014).
- **Organization tokens leave the webview** on desktop. The Rust relay
  closes three weaknesses found by the audit: changed URL, single pair,
  concurrent refreshes. Plain-text token storage remains to be handled (§5).
- **The merge adds only one API route (sign-out)** and sends no more data to
  the server.
- **Pages served on the web get security headers**, which they lack today.

**Negative**

- **On the web, the app first offers only the organization space.** The
  Understand section waits for its own decision.
- **Three more issues** (desktop organization access, i18n, research on
  Understand).
- **The server rewrites HTML** for the context tag, and the CSP must follow
  the start-up script's hash at every build.
- Fingerprint sign-in and the refresh token in `sessionStorage` remain until
  SSO.

**Neutral**

- The demo keeps its rules, except ADR-0017 rule 2, made more precise.
- App routes stay in French, as today.
- DTOs stay hand-written.

## Links

- [App and screen architecture](https://claude.ai/artifact/5UaiKHXrKPVyRPEd6gfeyr) (screen tree, access by role, four rules)
- [Routes, screen by screen](https://claude.ai/artifact/Cua7SK84VzMCwYFZRQfUHD) (M1, M2, Z6, Z7, Z9, B2, A6)
- ADR-0013 (pairing and team mode), ADR-0014 (single binary), ADR-0015 and
  ADR-0016 (privacy), ADR-0017 (web demo, rule 2 amended), ADR-0019 (repo
  split, PR #2), ADR-0020 (rename to Sobr.cc; crate and file names here are
  today's)
- [Codebase audit (Nov 2026)](https://linear.app/sobrcc/document/codebase-audit-nov-2026-6f4f34265a5e)
- Issues: SOB-14 (this ADR), SOB-42, SOB-47, SOB-62, SOB-63, SOB-64, SOB-93,
  SOB-94, SOB-95, SOB-35, SOB-83
