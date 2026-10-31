# ADR-0023 — The Sobr MCP server contract

- **Status**: Proposed. Thibault answered the eight points on 2026-10-31
  (SOB-152): every recommendation. It becomes Accepted once ADR-0022 (PR #18)
  is merged (ADR-0019 is merged)
- **Date**: 2026-10-31
- **Deciders**: Thibault (answers of 2026-10-31, recorded in SOB-152); CTO
  recommendation
- **Technical context**: Claude Code and MCP project (SOB-144)
- **Settles**: the condition of ADR-0019, decision 4 (where the MCP server
  lives). ADR-0019 is merged (PR #2); if its decision 4 changes, this ADR is
  amended
- **Depends on**: ADR-0022, the data model (SOB-143, PR #18, not merged yet),
  for projects, tasks and sessions

> **Summary.** Claude reads and acts on Sobr through MCP tools, within the
> rights of the person who connected it. Two front doors share one contract:
> a local server started by the client for the personal space, and a route
> of the Sobr server for an organization. Both live in the `sobr` repo, and
> one connection serves one space. A grant that the person creates in Sobr
> gives access (`read`, `write` or `act`, expiring, revocable) and carries no
> role: rights are looked up on every call. An irreversible tool never runs
> on the call that asks for it: the person approves it in Sobr, which also
> works for a client without a screen. Every write and approval goes to an
> append-only audit log, and a few actions never go through the MCP. Eight
> points for Thibault come first, with his answers.

**Terms.**

- *Front door*: one way in to the MCP, a local process (stdio) or an HTTP
  route.
- *Space*: the personal space (on the machine, or on the Sobr server) or an
  organization space.
- *Grant*: the record that gives one client access to one space (decision 3).
  Its *scope* is `read`, `write` or `act`.
- *Class*: 0 reads, 1 reversible write, 2 irreversible (needs approval),
  3 never exposed (decision 5).
- *Pending action*: an irreversible action that waits for the person's
  approval in Sobr (decision 8).
- *Actor*: who is calling, as the use case sees it: person, space, scopes and
  grant (decision 4).
- *Deep cleanup*: the Foundations milestone that an area of the code must
  pass before another project changes it.
- *k-anonymity*: the rule of ADR-0015 that hides an aggregate made of too few
  people.

## Points to decide

Seven decisions and one prerequisite for Thibault; each recommendation is
backed in the decision named. The same eight points, with their options, were
in the Decision issue SOB-152. Thibault answered on October 31 with every
recommendation; the answers are in the table after point 8.

1. **Two front doors, one contract, code in `sobr`** (decision 1).
   Recommended: yes. This also settles ADR-0019, decision 4: the MCP code
   stays in `sobr`, and `claude-code` only holds the plugin that configures
   the connection. Alternatives: the server only (a guest with no account
   gets no MCP), or a separate process in `claude-code` that calls the HTTP
   API (the local space has none).
2. **One connection, one space** (decision 2). Recommended: yes, so that the
   personal space never goes up to the organization (screen A7).
   Alternative: one connection with a space argument on every tool.
3. **Access grants** (decision 3). Recommended: scoped, expiring, revocable
   tokens first (they work for a client without a screen), OAuth 2.1 later.
   Three scopes, `read` by default. Alternatives: OAuth only (Claude Code
   cannot run it in a headless run), or tokens only.
4. **Actions that never go through the MCP** (decision 7). Recommended: keep
   out what creates or widens access to the MCP itself (grants, tokens),
   returns a secret, lowers transport security, loosens the visibility
   policy, sends data to a destination the model picks, or takes a file path.
   Role and member changes (invite, promote, create or edit a role) stay
   available as class 2: a person approves each in Sobr, and the use case
   refuses to give more than the caller has. This is narrower than route 9,
   which says Claude "can do everything". Alternative: expose everything and
   confirm what cannot be undone.
5. **Confirmation of irreversible actions** (decision 8). Recommended: a
   pending action approved by the person in Sobr, never by the client alone,
   and in an organization approved with a recent sign-in, so that a token
   stored by the app cannot approve. Screen C4 gains a Sobr side.
   Alternatives: approval with any Sobr session (a model with a shell can
   read the stored token and approve), or confirmation in Claude only, as the
   screen sheet reads today. Both are cheaper and weaker.
6. **Audit log** (decision 8). Recommended: one append-only, chained log for
   both spaces, keyed on the server for an organization, which records every
   write, every approval and every read of per-person data. Alternative: a
   plain table.
7. **MCP library** (issue 1). Recommended: a two-point spike between the
   official Rust SDK and a hand-written server, before any build. Neither is
   approved here, since CLAUDE.md section 13 requires an ADR for a new
   dependency.
8. **Prerequisite, not a choice.** The organization read tools and exports
   must not ship before SOB-114 and SOB-115: exports, alerts and the user
   list ignore the visibility policy today.

**Answers of Thibault, October 31, 2026 (SOB-152).** He chose option A, the
recommended one, for points 1 to 7 and confirmed point 8.

| Point | Answer |
|-------|--------|
| 1. Placement | A: two front doors, one contract, code in `sobr` |
| 2. Spaces | A: one connection per space |
| 3. Grants | A: scoped tokens first, OAuth 2.1 later |
| 4. Kept out of the MCP | A: class 3 as listed; role and member changes stay class 2 |
| 5. Confirmation | A: pending action approved in Sobr, with a recent sign-in in an organization |
| 6. Audit log | A: the `sobria-audit` chain extended to generic events, keyed for an organization |
| 7. MCP library | A: a two-point spike before any choice |
| 8. Prerequisite | Confirmed: SOB-114 and SOB-115 before the organization read tools and exports |

The design does not change: the decisions below stand as written. What is
left is the merge of ADR-0022 (PR #18), then the status.
## Context and problem statement

### What the design asks for

Route 9 (screens A5, C1 to C4, Z11, A7) of the routes document says:

- Claude can do everything through the Sobr MCP, administration included,
  within the rights of the person; an action that cannot be undone asks
  for confirmation.
- A5 offers the integration last during onboarding and turns the MCP on only
  if the person accepts. C1 links Claude to a personal or an organization
  space and writes an authorization that the person can revoke in the app.
- C2 answers questions: consumption of the person, of a project or of a task,
  and ways to reduce it; it reads what the rights allow. C3 acts: sort a
  session, create a task, invite, set a role, set a budget. C4 summarizes the
  action and waits for agreement.
- Every action goes to the audit log; Z11 lets an admin find each
  administration action, including those done by Claude.
- The Claude account in use chooses the space of a session (A7).

### What the code offers today

Read at `main` (e2ddef3, 2026-10-30; the two later commits change none of
the files cited). Paths are under `crates/`; the server
crate is `sobria-team-aggregator`, and its paths start at `src/`.

| Area | Fact | Where |
|------|------|-------|
| Roles | The server knows `user` and `admin` only. The role is checked by strict equality, so an admin token gets 403 on `/me/*` | `server/auth/jwt.rs:19-22`, `middleware.rs:35,50` |
| Tokens | HS256 JWT, 24 h, role inside, no database lookup per request; refresh token 7 days | `server/auth/jwt.rs:14,35-44`, `middleware.rs:57-73` |
| Visibility | Policy `anonymous`, `opt_in` (default), `identified`; k = max(3, setting), default 5 | `src/policy.rs:20-30,61`, `server/api/admin/analytics.rs:34-36` |
| Policy gaps | Exports ignore the policy; a per-user alert names one person by id; the user list returns identifiers in `anonymous` mode | `server/api/admin/exports.rs:22-34`, `server/api/admin/alerts.rs:52-55`, `storage/users.rs:86-97` (SOB-114, SOB-115) |
| Rate limit | None found | — |
| Server audit | No audit table. Traces are timestamps such as `revoked_at` | `storage/schema.rs:26-142` |
| User removal | No route to delete or deactivate a user, none to revoke an access token (ADR-0021 adds `POST /api/v1/logout`, not built) | — |
| App commands | 55 Tauri commands; no local network port (the only listener is the bridge socket, next row) | `sobria-app/src/main.rs:609-665` |
| Bridge | Named pipe or Unix socket, accepts only `ping`, `pair`, `estimate`, `revoke`; no read | `sobria-bridge/src/lib.rs:26,182-193` |
| Local ledger | `sobria-audit`: SHA-256 chain, unkeyed, records `EstimationResult` only; no settings or pairing event | `ledger.rs:46-54`, `entry.rs:38-46` (ADR-0012) |
| Logic placement | Use cases sit in `sobria-app/src/logic.rs` and in the server's `api/` modules, next to the interface code | — |
| Prompt text | Estimate requests carry counts, not text; the invariant is stated in CLAUDE.md §3, ADR-0014 condition 3, ADR-0015, ADR-0016 | `server/api/estimations.rs:52-60`, `sobria-app/src/dto.rs:195-216` |

### What the MCP specification gives

From the MCP specification, revision 2026-07-28 (the `latest` pages on 2026-10-31).

- **Transports.** stdio and Streamable HTTP. The old HTTP+SSE transport is
  deprecated. This revision has no protocol-level session.
- **Authorization.** Optional. For HTTP: OAuth 2.1, protected resource
  metadata, PKCE and resource indicators; client ID metadata documents are
  preferred, and dynamic registration is deprecated. For stdio: "SHOULD NOT"
  follow it, take credentials from the environment.
- **Tools.** `name`, `title`, `description`, `inputSchema`, optional
  `outputSchema`, `annotations`. A tool with an output schema must return
  `structuredContent` that conforms to it. Errors are of two kinds: protocol
  errors, and tool errors with `isError: true`. The list a server returns
  may vary with the authorization on the request.
- **Annotations are hints.** `readOnlyHint`, `destructiveHint`,
  `idempotentHint`, `openWorldHint`. Clients "MUST consider tool annotations
  to be untrusted unless they come from trusted servers".
- **Human in the loop** is a SHOULD, not a MUST. Servers MUST validate
  inputs, enforce access controls, rate limit and sanitize outputs.
- **Elicitation** (the server asks the person through the client) is
  optional for clients, in `form` or `url` mode.
- **Token passthrough** is forbidden: a server must not accept tokens that
  were not issued to it.

From the Claude Code documentation: servers are added with `claude mcp add`
(HTTP, stdio) at local, project or user scope; remote servers use OAuth or a
static `--header`; a headless run "can't run the OAuth flow"; a tool can set
`_meta["anthropic/requiresUserInteraction"]` to force a prompt on every call,
and only Claude Code documents it.

What follows from this: the spec cannot guarantee that a person confirmed an
action. A client may run without a screen, may auto-approve, and may ignore
annotations. So the guarantee has to live in Sobr.

## Decision

### 1. Placement: two front doors, one contract

| Option | What it is | Cost and effort | Risk |
|--------|-----------|-----------------|------|
| A. Server only | One `/mcp` route in the Sobr server | Lowest: one transport, the data and roles are already there | A person in guest mode (local, no account) gets no MCP, and that is the usual individual. Product mismatch |
| B. Separate process | A process (in `claude-code`) that calls `/api/v1` for both spaces | Medium, but the local space has no HTTP API: a local network listener must be added | New attack surface on the machine; the process holds the organization token; the API has two roles and few routes, so most tools cannot exist |
| C. Two doors, one contract (recommended) | One registry of tools and schemas. A stdio server for the personal space, a Streamable HTTP route in the Sobr server for an organization | Highest: two transports. About 80 % is shared (registry, schemas, errors, tests) | Two places to keep in step; the shared registry and a parity test (decision 5) cover it |

**Decision: C.** Each door sits next to the data it serves and uses the same
rights code as the rest of the product. The code lives in `sobr`: a crate
(`sobr-mcp`) with the registry, the schemas and the handlers; the server
mounts the route; a small binary runs the stdio door. The `claude-code`
repo holds only the plugin that writes the client configuration.

**No business logic in an interface.** The layers are: interface (Tauri
command, HTTP route, MCP tool, CLI) → use case (one function per question or
action, taking an `Actor`) → stores and core crates. An MCP handler
validates the input, calls one use case, and shapes the result. It holds no
rule of its own. Today the use cases are mixed into `logic.rs` and the
server's `api/` modules, so extracting them is the first build step and
waits for the Deep cleanup of those areas (section 9).

**How the stdio door reaches local data.**

| Option | Cost and effort | Risk |
|--------|-----------------|------|
| L1. Through the running app (a new local control channel) | Medium; the logic stays where it is | The MCP works only while the app runs; a new channel on the machine; the pipe has no explicit access control today |
| L2. Directly, through the shared use cases (recommended) | Medium; needs the extraction above | Two processes may write the stores. SQLite serializes writes, but `append` reads the last signature before it opens its transaction, so a deferred transaction does not stop two processes from reading the same one: the append must run under `BEGIN IMMEDIATE` with a `busy_timeout`, or two writers can fork the chain. To verify in the `sobria-audit` cleanup (SOB-22) |

**Decision: L2.** It works with the app closed (a terminal session of Claude
Code, for example) and follows from the layer rule. Writes that need the
person (decision 8) wait in the local store until the app shows them.

### 2. Spaces: one connection, one space

A connection is bound to one person and one space when the grant is made. The
stdio door always serves the personal space on the machine. A `/mcp` route
serves the one organization of its server (or the personal space hosted on
the Sobr server, ADR-0014 Amendment 1). No tool takes a space argument and no
tool reads two spaces. The client lists two entries when a person uses both,
for example `sobr-personal` and `sobr-org`. `get_context` tells the model
which one it is talking to.

| Option | Cost | Risk |
|--------|------|------|
| One connection per space (recommended) | Two entries to configure | None for isolation; the model cannot leak across spaces by mistake |
| One connection with a `space` argument | One entry | A prompt-injected call can read or write the wrong space; each tool needs a second rights check; personal data and organization data share a grant |

### 3. Access grants (C1, A5)

A **grant** is a record created in Sobr by the person, never by the MCP
itself: `{id, person, space, scopes, client_name, created_at, expires_at,
last_used_at, revoked_at}` plus a secret. The scopes group the tool classes
of decision 5:

| Scope | Allows | Default |
|-------|--------|---------|
| `read` | class 0 tools (questions, C2) | yes |
| `write` | `read`, and class 1 tools (reversible writes) | no |
| `act` | `write`, and class 2 tools, as proposals that still need approval (decision 8) | no |

The person picks the scopes and an expiry (30 days by default, 365 at most)
on C1, sees the grant in the app, and revokes it there. A5 only offers the
integration; nothing exists until the person creates a grant.

| Option | Cost and effort | Risk |
|--------|-----------------|------|
| G1. Scoped tokens, created in the app (first) | Low: a table, a hash, a revoke button. Works for stdio (`SOBR_MCP_TOKEN` in the environment) and for a static `--header` | A secret to copy by hand; it can leak into a shell history or a `.mcp.json` |
| G2. OAuth 2.1 in the Sobr server (second) | High: authorize and token endpoints, consent page, metadata, refresh, revoke. About 8 points | Security-critical code; self-hosted servers must run it too |
| G3. Delegate to the organization's SSO | Medium once SSO exists (Organization and identity project) | A self-hosted server without SSO has no path; not before February 2027 |

**Decision: G1 first, G2 second, G3 not used for the MCP.** G1 is the only path
that works for a client without a screen, and the spec allows it (authorization
is optional; stdio takes credentials from the environment). G2 follows, when an
interactive client needs it, and is what the specification expects of an HTTP
server.
Both create the same grant record, so the tools do not know which was used.

Rules for the secret and the token:

- The secret is 256 random bits, shown once, stored hashed. It is opaque,
  not a JWT, so a revoke applies on the next call.
- It is bound to the MCP server: the `/mcp` route refuses the login token of
  `/api/v1`, and `/api/v1` refuses the grant token (the specification
  forbids token passthrough).
- The documentation tells people to keep the secret out of `.mcp.json` and
  in an environment variable.
- Creating, listing and revoking grants is a class 3 action: the MCP cannot
  give itself access. Revoking or expiring a grant cancels its pending
  actions (decision 8).

### 4. Rights: how the server knows them

| Option | Cost and effort | Risk |
|--------|-----------------|------|
| R1. Role in the grant (like today's JWT) | Lowest | A demoted admin keeps the old role until the grant expires; custom roles (Organization and identity project) break it |
| R2. Look up the person on every call (recommended) | One lookup per call, a single joined query on grant, person and permissions | None new: it is what a revoke needs anyway |

**Decision: R2.** The grant holds the person, the space and the scopes, not a
role. On each call the use case receives an `Actor` (person, space, scopes,
grant id) and resolves the current permissions in its own store. **Effective
rights = the person's permissions, cut down by the grant's scopes.** A grant
can narrow rights and never widen them.

Rights use a closed list of **permissions** that the use layer owns, not role
names, so custom roles and per-member matrices (Z4, Z9) later change a
mapping and not the tools:

`usage.read.self`, `usage.read.project`, `usage.read.org`,
`sessions.read.self`, `sessions.classify.self`, `estimates.record`,
`projects.read`, `projects.write`, `tasks.read`, `tasks.write`,
`budget.read`, `budget.set.self`, `budget.set.project`, `budget.set.org`,
`suggestions.read`, `suggestions.manage`, `members.read`, `members.invite`,
`members.remove`, `roles.read`, `roles.assign`, `roles.manage`,
`policy.read`, `policy.set`, `alerts.read`, `alerts.manage`,
`audit.read.self`, `audit.read.org`, `exports.create`, `reference.read`.

Today the mapping is: in the personal space the person holds every
permission that applies there (`usage.read.self`, `sessions.*.self`,
`estimates.record`, the project, task, budget and suggestion permissions,
`audit.read.self`, `reference.read`). In an organization space, `user` holds
`usage.read.self`, `sessions.*.self`, `audit.read.self` and `policy.read`;
`admin` holds the organization ones and `policy.read`, and no `.self`
permission, because an admin has no usage of their own on the server
(ADR-0021). The server has no project
entities, tasks or budgets yet (only a free-text project label on
estimations); who holds `projects.*`, `tasks.*` and `budget.*` there comes
from the data model and roles work. Two consequences:

- An admin has no usage of their own on the server (ADR-0021: `/me/usage`
  answers 403), so `get_usage` with scope `self`, `list_sessions` and `get_session`
  answer `permission_denied` for an admin until the data model ADR (SOB-143) says otherwise.
- `manager` and custom roles do not exist yet (ADR-0021, point 4). The
  tools below name permissions; the roles project decides who holds them.

The visibility policy (ADR-0015, ADR-0016) applies as in the interface. A tool
returns exactly what the matching screen shows to that person: k-anonymity
on aggregates (empty sections and `k_anonymity.blocked`, as in ADR-0015, not
an error; the tool applies k to every group it returns, which can be
stricter than a screen), per-person detail only when the policy allows it, and the
personal space never leaves the person. The server never trusts an argument
for rights: an id the person may not see answers `not_found`, the same as an
id that does not exist.

A tool never grants more than the person has. Role changes also have guards
in the use case, not in the MCP: nobody changes their own role, the last
admin cannot be removed, and nobody can assign, invite with or create a role
that holds more permissions than their own. A role cannot be deleted if its
members would fall back to a default role that holds permissions the deleted
role did not.

### 5. Rules for every tool

- **Four classes (0 to 3), a property of the tool and never of its
  arguments**, so the annotations tell the truth:

  | Class | Meaning | Confirmation | Audit |
  |-------|---------|--------------|-------|
  | 0 | Reads. Changes nothing | none | per-person reads only |
  | 1 | Reversible or additive write. The previous values are kept in full in the audit entry, so a normal edit puts them back (or the call only adds a record and removes nothing); the call itself sends nothing to anyone; no linked data is lost | none | yes |
  | 2 | Irreversible, or reaches other people or outside systems | pending action approved in Sobr | yes, every step |
  | 3 | Never exposed: no tool | — | — |

- **No tool mixes reading and writing**, and no tool has an `action`
  argument that switches between them.
- **No prompt text, ever**, in an input or an output. Estimates take counts.
  Sessions have no title or label. Free text typed by people (project names,
  descriptions, member names, synced task titles) comes back as plain data in
  its own field, cut to 200 characters. No such text can change rights or
  skip a confirmation: only tool calls change state, and they pass through
  rights and decision 8. Class 1 tools still run at once, so a model steered
  by such text can use them within its grant (see Consequences).
- **No secret in a result**: no code, token, password, pairing secret or
  certificate. An invitation is sent by email, and the code is shown in the
  Sobr interface only.
- **Results are structured**: each tool has an `outputSchema` and returns
  `structuredContent` (and the same JSON as text, for older clients).
  Quantities are `{value, p5, p95, unit}` (CLAUDE.md §6) with the `method`
  id of ADR-0012, such as `afnor_sobria`; energy in kWh, emissions in
  gCO₂eq, money in euros, times in RFC 3339 UTC.
  Lists take `cursor` and `limit` (25 by default, 100 at most).
- **Inputs are strict**: `additionalProperties: false`, lengths and ranges
  bounded (names 120 characters, descriptions 2,000), ids opaque strings.
- **Errors** are tool errors with a stable `code`: `permission_denied`,
  `not_found`, `policy_blocked`, `scope_missing`, `grant_expired`,
  `rate_limited`, `proposal_rejected`, `too_many_pending`, `invalid_input`. A
  pending action is a normal result, not an error (decision 8). Protocol errors stay for
  malformed requests.
- **Annotations**: `readOnlyHint: true` on class 0; `destructiveHint: true`
  on class 2 and on the class 1 tools that overwrite (`update_project`,
  `update_task`, `classify_session`, `set_budget`, `dismiss_suggestion`),
  `false` on the additive ones; `idempotentHint` where true;
  `openWorldHint: false` on every tool except `invite_members`,
  `create_alert` and `set_shared_budget`, which make mail leave the system
  (`true`; the specification's default is `true`). They are hints for
  the client; Sobr does not rely on them.
- **Short descriptions** (200 characters at most), because every tool costs
  context in every request. The list a server returns is cut to the grant
  and the person: an organization member with `read` sees about 8 of the 47 tools today,
  more once projects, tasks and budgets exist on the server.
- **Rate limit per grant** (the specification requires it of servers; none
  was found in the Sobr server). Writes have a lower limit than reads. Both doors apply it (issues 6 and
  13); suggested defaults are 60 reads and 20 writes per minute per grant, to
  be set in the build issue.
- **Contract version**: `contract_version` (semver) in `get_context`. New
  tools and optional fields are minor; renaming or removing a tool,
  narrowing an input or changing a class is major and needs an amendment of
  this ADR.
- **Parity test**: for each use case reachable by more than one interface, a
  test calls it through the MCP and through the HTTP route or Tauri command
  with the same Actor and checks the same data comes back.

**Two choices behind these rules.**

| Choice | Option | Cost and effort | Risk |
|--------|--------|-----------------|------|
| Granularity | One tool per question or action (recommended) | 47 definitions in all, though a client lists only what its grant allows | None for rights: class, annotations and permission are set per tool |
| Granularity | A few tools with an `action` argument | Fewer definitions, less context | A read tool can hide a write; the class, the hints and the permission cannot be set per case, and a client prompt cannot tell the cases apart |
| Surface | Tools only (recommended) | Lowest; every client supports tools | The model must call a tool to read; nothing to browse |
| Surface | Tools and resources | Resources suit browsing, such as a usage summary by URI | Client support varies; a second path to the same rights check |

**Decision: one tool per question or action, and tools only in v1.**
Resources and prompts can be added later as minor versions.

### 6. Read tools (C2)

Class 0. A tool needs the `read` scope and the permission shown. "Both"
means personal and organization spaces; "org" the organization only.

| Tool | Inputs | Returns | Permission | Space |
|------|--------|---------|------------|-------|
| `get_context` | — | space (kind, name), person, role, permissions, grant (id, scopes, expiry), visibility level, `contract_version` | any valid grant | both |
| `get_usage` | `scope` (`self`, `project`, `task`), `id`, `period`, `group_by` (`day`, `model`, `project`, `task`, `source`) | totals and groups with P5–P95 and `method`; sessions count; budget status when set | `usage.read.self`; `usage.read.project` for a project or task of the organization | both |
| `get_organization_usage` | `period` (7 days at least), `group_by` (`day`, `project`, `model`, `source`, `person`) | aggregates. k applies to every group returned: a group under k active people is merged into `other`, and if `other` is still under k it is dropped together with the grand total. Under k people overall the sections are empty and `k_anonymity.blocked` is true (ADR-0015). `person` is honored only where the policy allows it | `usage.read.org` | org |
| `list_sessions` | `period`, `source`, `project_id`, `task_id`, `classified`, `cursor`, `limit` | the caller's own sessions: id, start, source, model, links, energy and emissions with intervals. No text | `sessions.read.self` | both |
| `get_session` | `id` | figures per source merged (value, P5–P95, method), links, sources | `sessions.read.self` | both |
| `list_projects` | `status`, `cursor`, `limit` | id, name, description, member count, budget status | `projects.read` | both |
| `get_project` | `id`, `include_datasheet` | the same, with task count, linked tools and, on request, the project datasheet (figures and hypotheses) | `projects.read` | both |
| `list_tasks` | `project_id`, `status`, `cursor`, `limit` | id, title, status, origin (manual or synced) | `tasks.read` | both |
| `get_budget_status` | `scope` (`self`, `project`, `organization`), `id` | amount, unit, period, consumed with interval, thresholds reached (80 %, 100 %) | `budget.read` | both |
| `list_suggestions` | `scope`, `status` (`open`, `dismissed`), `cursor`, `limit` | id, title, gain in kWh and euros with interval, whether Sobr can apply it, how (a tool name or a command) | `suggestions.read` | both |
| `list_members` | `status`, `cursor`, `limit` | id, role, status; name and usage totals only where the policy allows it (ADR-0015, ADR-0016) | `members.read` | org |
| `list_roles` | `cursor`, `limit` | roles and their permissions | `roles.read` | org |
| `list_invitations` | `status`, `cursor`, `limit` | id, created, expires, status, creator. Never the code | `members.invite` | org |
| `get_visibility_policy` | — | policy, `k_anonymity_min`, research sharing (Z10), attestation date, last change | `policy.read` (any member, as screen M3) | org |
| `list_alerts` | `include_triggers`, `cursor`, `limit` | thresholds (scope, metric, value, channel, enabled) and, on request, their history | `alerts.read` | org |
| `get_audit_log` | `period`, `actor_id`, `tool`, `outcome`, `cursor`, `limit` | entries of decision 8. `audit.read.self` returns the caller's own entries; `audit.read.org` adds the organization's administration entries (invitations, roles, budgets, policy, alerts, exports, pending actions), never another person's session or estimate edits | `audit.read.self`, `audit.read.org` | both |
| `verify_audit_chain` | — | `ok`, entries checked, first bad entry | `audit.read.org` in an organization, `audit.read.self` in the personal space | both |
| `get_pending_action` | `id` | state, summary, expiry, result | the person who holds the grant | both |
| `list_pending_actions` | `state`, `cursor`, `limit` | the grant's own pending actions: id, tool, summary, state, expiry | the person who holds the grant | both |
| `list_models` | `family`, `cursor`, `limit` | id, vendor, size class, methodology ids | `reference.read` | personal |
| `get_model` | `id` | parameters, sources, methodologies | `reference.read` | personal |
| `estimate` | `model_ids` (1 to 5), `input_tokens`, `output_tokens`, `method`, `datacenter_id`, `modalities`, `overhead` (the last four optional, as in `EstimationRequestDto`) | per model: energy, emissions, water with P5–P95, hypotheses, seed; ranked when there are several. Not written to the ledger | `reference.read` | personal |

The reference tools (`list_models`, `get_model`, `estimate`) serve the
personal space only: on a server they need the estimator and the territory
data, which SOB-95 decides (ADR-0021, point 3). Reference data for
territories (data centers, industrial sites, flows) follows the same
pattern and is listed in its build issue.

Two limits are on purpose. `list_sessions` and `get_session` return only the
caller's own sessions, and an admin has none on the server: another person's sessions are
per-person data and stay behind the policy. And `get_organization_usage`
with `group_by: person` and `list_members` are the only org reads of
per-person data; both are audited (decision 8). Applying k to each group does
not stop differencing across overlapping queries, and the screens share this
limit; a minimum period of 7 days limits it, and query budgets are left to
the visibility policy work.

### 7. Write tools (C3) and the actions kept out

Classes 1 and 2 (scopes `write` and `act`, decision 3). A class 2 tool
returns a pending action (decision 8).

| Tool | Class | Inputs | Returns | Permission | Space |
|------|-------|--------|---------|------------|-------|
| `classify_session` | 1 | `session_id`, `project_id`, `task_id` (null = personal) | the session | `sessions.classify.self` | both |
| `create_classification_rule` | 1 | `match` (source, model, repository), `project_id`, `task_id` | rule id | `sessions.classify.self` | both |
| `apply_rule_to_history` | 2 | `rule_id` | pending action (then the count of sessions changed) | `sessions.classify.self` | both |
| `record_estimate` | 1 | the `estimate` inputs with one `model_id`, `project_id`, `task_id` | the figures of `estimate` and the ledger entry id. Append-only, loses nothing | `estimates.record` | personal |
| `create_project` | 1 | `name`, `description`, `tags` | project | `projects.write` | both |
| `update_project` | 1 | `id`, `name`, `description`, `tags` | project; the old values go to the audit entry | `projects.write` | both |
| `delete_project` | 2 | `id` | pending action. Today a hard delete; its sessions become unclassified | `projects.write` | both |
| `create_task` | 1 | `project_id`, `title`, `due` (manual tasks only; a synced task is read-only) | task | `tasks.write` | both |
| `update_task` | 1 | `id`, `title`, `status` | task | `tasks.write` | both |
| `delete_task` | 2 | `id` | pending action | `tasks.write` | both |
| `set_budget` | 1 | `amount` (null clears), `unit` (`kWh`, `EUR`), `period`: the person's own budget | budget; the old value goes to the audit entry | `budget.set.self` | both |
| `set_shared_budget` | 2 | `scope` (`project`, `organization`), `id`, `amount` (null clears), `unit`, `period` | pending action; crossing 80 % or 100 % later mails the managers and admins | `budget.set.project`, `budget.set.org` | both |
| `dismiss_suggestion` | 1 | `id` | — | `suggestions.manage` | both |
| `apply_suggestion` | 2 | `id` (changes a model, compacts or cuts a session: work in progress can be lost) | pending action | `suggestions.manage` | both |
| `create_alert` | 2 | `scope` (`organization`, `project`), `metric`, `threshold`, `channel` (`in_app`, or `email` to the managers and admins; no webhook URL) | pending action (the alert mails people when it triggers). A per-person scope waits for SOB-115 | `alerts.manage` | org |
| `disable_alert` | 2 | `id` | pending action (people stop receiving the alert) | `alerts.manage` | org |
| `invite_members` | 2 | `emails` (1 to 50), `role_id` | pending action (mail leaves the system) | `members.invite` | org |
| `revoke_invitation` | 2 | `id` | pending action (the old code is not restored) | `members.invite` | org |
| `set_member_role` | 2 | `member_id`, `role_id` | pending action; guards of decision 4 | `roles.assign` | org |
| `remove_member` | 2 | `member_id` | pending action; names and detail are deleted, usage is anonymized, grants are revoked and pending actions cancelled | `members.remove` | org |
| `create_role` | 2 | `name`, `permissions` | pending action; no role may hold more than the caller | `roles.manage` | org |
| `update_role` | 2 | `id`, `name`, `permissions` | pending action; same guard | `roles.manage` | org |
| `delete_role` | 2 | `id` | pending action; members of the role fall back to the default role | `roles.manage` | org |
| `restrict_visibility_policy` | 2 | `policy` (`anonymous` or `opt_in`); the use case rejects any target that is not stricter than the current policy (the same policy answers `invalid_input`) | pending action | `policy.set` | org |
| `create_export` | 2 | `format` (`csrd`, `prov-o`, `csv`), `period` | pending action, then a download link the person opens; the file never passes through the model. Always anonymized | `exports.create` | org |

Several tools wait on features that do not exist yet: `remove_member` and
the three role tools on the Organization and identity project, `apply_suggestion` and `list_suggestions`
on B1, `create_export` on SOB-114. The registry lists a tool only when its
use case exists. Usage that crosses a budget threshold mails people as it
does in the interface: that mail is the product's behavior, not an effect of
the tool that added the usage.

**Which actions are irreversible.** Class 2 is every tool in the table above
marked 2. The reasons: it deletes data that is not recorded (`delete_project`,
`delete_task`, `remove_member`); it reaches other people or outside systems
(`invite_members`, `create_export`, `create_alert`, `disable_alert`,
`set_shared_budget`); it cannot be put back as it was
(`revoke_invitation`, `apply_suggestion`, `apply_rule_to_history`); or it
changes who can see or do what (`set_member_role`, `create_role`,
`update_role`, `delete_role`, and `restrict_visibility_policy`, which only
tightens but cannot be undone through the MCP).

**Class 3, never exposed:**

| Action | Why |
|--------|-----|
| Create, list or revoke grants and tokens; enroll, sign in, sign out, change a password | The MCP would give itself access |
| Pairing secrets and installing or removing the bridge | A secret in the model's context; changes what runs on the machine |
| Server URL, mode, "accept invalid certificates" | Lowers transport security |
| Loosen the visibility policy (any step), and the works council attestation | More of people's data is exposed: a human decision with a legal statement (ADR-0016) |
| Billing and payment; SSO, SCIM, mail and integration secrets | Money and secrets |
| Delete the organization or an account | Out of proportion for a model call |
| Webhook URLs and any other destination the model picks | Data would leave to a place the model chooses |
| Commands that take a file path (`run_batch_from_csv`, `export_audit_ndjson`, `export_csrd_report`) | The model would choose where files are read or written |
| `reload_referentiel` (runs `dvc pull`) | Network and disk write, and SOB-126 is open |
| Retention, certificate regeneration, `init --force` | Reachable from the command line only today |

| Option | Cost | Risk |
|--------|------|------|
| X1. Expose everything, confirm class 2 (route 9 as written) | Low | A model call could create access, change transport security or write any file. One confirmation protects all of it |
| X2. Keep class 3 out (recommended) | Low: fewer tools | Narrower than "everything"; the person uses the app for these |
| X3. Read-only MCP at first, writes later | Lowest | Misses C3, the point of route 9 |

**Decision: X2.** Everything a person does to their own work and to their
organization's usage data goes through the MCP. What widens access to the MCP
itself, or leaves a secret, does not. Role and member changes stay class 2:
each needs approval in Sobr. Thibault's call, since it is narrower than
route 9.

### 8. Confirmation (C4) and audit log

**Confirmation.**

| Option | Cost and effort | Risk |
|--------|-----------------|------|
| K1. The client asks (a tool prompt in Claude Code, `destructiveHint`, `requiresUserInteraction`) | None, matches the screen sheet | The spec makes it a SHOULD and annotations untrusted. A client in `bypassPermissions`, in auto mode or run with `-p` can skip it. Sobr cannot tell |
| K2. Elicitation through the client | About 3 points | Optional for clients; a headless run has no one to answer; the answer still comes through a client the server cannot check |
| K3. Pending action approved in Sobr (recommended) | About 18 points over the stores, the screens and the recent-sign-in check | Friction on rare actions; the person leaves the chat to approve |

A `confirm: true` argument is not an option: the model fills it.

**Decision: K3, with K1 as an extra layer.** A class 2 tool never runs on
the call that asks for it:

```
Claude ── tools/call delete_project ──▶ Sobr MCP
                                          checks grant, scope, rights;
                                          builds the summary itself
                       pending action + audit "proposed"
Claude ◀── result: pending_confirmation (summary, expiry, link) ──┘
Person ── opens the link in Sobr (C4) ──▶ approve or reject
                                          runs the stored arguments;
                                          audit "approved", "executed"
Claude ── get_pending_action ──▶ result
```

- The server builds the **summary** from the validated arguments and the
  current data (for example: "Delete project Payments API: 14 sessions
  become unclassified, 3 tasks are deleted"), not from the model's words.
- The pending action stores the tool, the arguments, their digest, a digest
  of the state the summary describes, the person, the space, the grant and
  an expiry (24 hours). The same call with the same arguments while one is
  pending returns the same pending action. If the state has changed at
  approval time, nothing runs: the action becomes `stale` and the model must
  propose again. The permission check, the state check and the execution run
  in one transaction. A rejected proposal blocks the same proposal for 24 hours
  (an expired one does not) and answers `proposal_rejected`; a grant holds
  at most 10 open pending actions and answers `too_many_pending`.
- **Approval is a first-party Sobr action** by the same person: a dialog in
  the app (personal space, where guest mode has no account and the person is
  at the machine), the app or the web page (organization space), under a
  recent Sobr sign-in (see below). No tool approves. The person must still hold the
  permission at approval time. The model cannot change the arguments after
  the summary was shown.
- **A client without a screen** (a headless run, a scheduled agent) gets the
  pending action and goes on; a person approves later, from any device, and
  the agent reads the result with `get_pending_action`.
- A client that supports URL-mode elicitation may open the link in the flow.
  This is a shortcut to the same page, not a second path.
- Class 2 tools carry `destructiveHint: true`, so a careful client asks
  before sending the proposal too. Sobr does not set
  `requiresUserInteraction`: a proposal executes nothing, so a forced prompt
  on every call only adds friction.
- Class 1 tools run at once. The old values go to the audit entry in full,
  so a normal edit puts them back.

**Who can approve in an organization.** The approval must come from the
person, not from a program on their machine. The desktop app keeps the
organization access and refresh tokens in its local store today
(`sobria-app/src/team_client.rs`; ADR-0021, section 5), so a model with a
shell can read them.

| Option | Cost and effort | Risk |
|--------|-----------------|------|
| S1. Any Sobr session of the person | None | A model with a shell reads the stored refresh token, gets an access token and approves its own proposal |
| S2. A recent sign-in (recommended) | About 3 points in issue 10, plus a password or SSO prompt at approval | The approval route accepts only a token minted by a sign-in in the last 10 minutes, which a refresh token cannot mint. A person who approves often sees the prompt often |

**Decision: S2** for an organization. The personal space has no account to
sign in to (guest mode), so approval there is the dialog in the app.

**What this protects, and what it does not.** Approval stops a model that
goes through the MCP. In an organization, with S2, a grant holder cannot
approve: its token works only on `/mcp`, where class 2 tools only propose,
and the approval route needs a sign-in that a stored token cannot give. Two
things stay out of reach. On the personal space the data sits in files of
the same OS user as the client. On a machine enrolled in an organization,
the app keeps the organization tokens in its local store, so a model with a
shell (Claude Code with its Bash tool) can call the admin API with them
directly. Only the client's own permission system can stop either, and
keeping tokens in the OS keychain (ADR-0021, section 5) closes the second.
The grant of the stdio door guards the MCP wrapper, not the files.

Screen C4 therefore has two sides: a summary and a wait in Claude (as the
sheet reads), and an approve-or-reject view in Sobr. The design issue for
C1 to C4 (SOB-58) needs this change; it is in the issue list below.

**Audit log.**

| Option | Cost and effort | Risk |
|--------|-----------------|------|
| Au1. Extend the `sobria-audit` chain to generic events, keyed for an organization (recommended) | About 10 points over the crate and the server | The signed entry format must stay readable (ADR-0020: signed data is never rewritten); a new versioned format sits beside the old one |
| Au2. A second, server-only log table with its own chain | About 5 points | Two chain implementations to keep in step; the app cannot reuse it |
| Au3. A plain table | About 2 points | Z11 says "journal chaîné"; edits go unseen |

**Decision: Au1.** One mechanism for both spaces. The local ledger stays an
unkeyed chain (it detects an accidental edit, not a deliberate rewrite by
someone who can write the file; there is no key store on the machine). The
organization log adds a keyed signature from a key derived from the server
secret, so an edit that skips the key shows up. The entry format is
versioned; existing entries are not touched.

An entry holds: `id`, `time`, `space`, `actor` (a `person_ref`), `via` (`mcp`,
`app`, `web`, `cli`), `grant_id`, `client_name` (as the client declares it,
untrusted), `tool`, `class`, `target` (type and id), `state_before` and
`state_after` for class 1 (the changed fields in full, within the input
limits; never prompt text), `args_digest` (keyed in an organization),
`pending_id`, `outcome`
(`proposed`, `approved`, `rejected`, `expired`, `cancelled`, `stale`,
`executed`, `failed`), `approver` (a `person_ref`) and
`approved_via`, plus `prev_sig` and `sig`. It never holds prompt text, a
secret, a person's name or a conversation. A `person_ref` is a random reference kept in its own table. When a member
leaves, that row is deleted: the entries stay in the chain, which is never
rewritten, but no longer point to anyone. Leaving also revokes the member's
grants and cancels their pending actions. Emails are never stored:
`invite_members` records their count and the invitation ids.

What is logged: every class 1 and class 2 write; every step of a pending
action; and the reads that return per-person data (`get_organization_usage`
with `group_by: person`, `list_members`). Other reads are not logged, since
that would turn the log into a record of what an admin looked at. Z11 reads
the organization log; the app shows the personal one.

### 9. Order and issues

The Foundations rule applies: no issue touches an area before its Deep
cleanup. A new crate touches no area. Dates come from the project targets:
Claude Code and MCP on 2027-01-15, Organization and identity on 2027-02-26,
so the personal space and the first organization reads come first, and the
administration tools come with the roles project.

Points are indicative; the official estimate is made at creation. Each
issue is 8 points at most. They are listed here, to be created once this
ADR is Accepted.

| # | Issue | Role | Points | Waits for |
|---|-------|------|--------|-----------|
| 1 | Spike: official Rust MCP SDK or a hand-written server (revision 2026-07-28, stdio and stateless HTTP, binary size, Origin checks) | `role:rust` | 2 | this ADR |
| 2 | `sobr-mcp` crate: tool registry, JSON Schemas, annotations, error codes, contract version, mock use cases, golden `tools/list` per rights profile | `role:rust` | 5 | 1 |
| 3 | Grant model and token format (scopes, expiry, audience), shared by both doors | `role:rust` | 3 | this ADR |
| 4 | Server use-case layer: extract the use cases from `api/` into functions that take an `Actor`, with the permission table (`user` and `admin` today) | `role:rust` | 8 | **SOB-25** |
| 5 | Grants on the server: table, hashed tokens, create, list, revoke | `role:rust` | 5 | **SOB-25**, 3, 4 |
| 6 | `/mcp` route: Streamable HTTP, stateless, audience check, rate limit per grant | `role:rust` | 5 | **SOB-25**, 2, 5 |
| 7 | Organization read tools (C2) | `role:rust` | 5 | **SOB-25**, SOB-114, SOB-115, 2, 4, 9, data model (SOB-143) |
| 8 | Generic audit events in `sobria-audit`: versioned entry, old entries untouched, append under `BEGIN IMMEDIATE` with a `busy_timeout` | `role:rust` | 5 | **SOB-22** |
| 9 | Organization audit log: table, keyed chain, read API for Z11 | `role:rust` | 5 | **SOB-25**, 8 |
| 10 | Pending actions: store, state machine, expiry, approve and reject (recent sign-in in an organization), with `get_pending_action` and `list_pending_actions` | `role:rust` | 8 | **SOB-25**, **SOB-26**, 8, 9 |
| 11 | Extract the app's use cases from `logic.rs` into a layer callable without Tauri | `role:rust` | 8 | **SOB-26**, SOB-19 (estimator), SOB-20 (referentiel), SOB-22 (audit) |
| 12 | Local grants: store and commands for A5 and C1 | `role:rust` | 3 | **SOB-26**, 3 |
| 13 | stdio door `sobr-mcp` binary for the personal space, with the per-grant rate limit | `role:rust` | 5 | 2, 11, 12 |
| 14 | Local read tools (C2) | `role:rust` | 5 | 11, 13, 8, data model (SOB-143) |
| 15 | Write tools, class 1, both spaces | `role:rust` | 5 | **SOB-25**, **SOB-26**, 2, 4, 8, 9, 11 |
| 16 | Write tools, class 2 with pending actions, both spaces | `role:rust` | 8 | 10, 15 |
| 17 | Screens A5, C1 and C4 (grants and approval) in the single app, on desktop and web | `role:frontend` | 8 | **SOB-27**, **SOB-30**, SOB-42 (single app on the web), 5, 12, 10, 22 |
| 18 | Administration tools: invitations, roles, remove member, visibility, alerts, exports | `role:rust` | 5 | Organization and identity project, SOB-114, SOB-115, 16 |
| 19 | OAuth 2.1 in the Sobr server, when an interactive client needs it (metadata, PKCE, resource indicators, consent, refresh, revoke) | `role:rust` | 8 | **SOB-25**, 5, 17 |
| 20 | Shared conformance harness: no prompt text or secret in any result, hostile text in fields, rights narrowing, expired and revoked grants (each tool issue adds its own parity test) | `role:qa` | 3 | 2, 3 |
| 21 | User documentation: connect Claude Code, scopes, allow rules, headless runs | `role:docs` | 2 | 13, 6 |
| 22 | Update the design of C1 to C4 and A5 for the Sobr side of approval (SOB-58 holds the screens) | `role:designer` | not counted | this ADR |

Total: 111 points, not counting the design update, the Claude Code plugin in
the `claude-code` repo and the VS Code extension (their own projects). The
bold numbers are the Deep cleanups that must pass first: SOB-25 (team
server), SOB-26 (desktop app backend), SOB-22 (audit), SOB-27 and SOB-30
(web). Issues 1 and 3 touch no existing area and can start once this ADR is
approved; issue 2 follows the spike. The suggestion tools wait for the B1 issues of the Browser
extension project; they join issues 14 and 15 when those exist.

## Consequences

**Positive**

- **One contract for two spaces.** A tool means the same thing on the
  machine and on a server, and a test proves it.
- **Rights are enforced where the data is**, by the same code as the
  screens, so the MCP cannot see more than the person does.
- **A model cannot approve its own irreversible action** through the MCP,
  whatever the client does, and a client without a screen still works.
- **A revoke or a demotion applies on the next call.**
- **The audit log answers Z11** and also covers the personal space.
- **Frugal**: no local network port, no new service, a tool list cut to the grant.

**Negative**

- **Two doors to build and keep in step**, and an extraction of the use
  cases that waits for the Deep cleanups.
- **Irreversible actions take a second step** outside the chat.
- **Narrower than route 9** (class 3), which Thibault must accept.
- **About 111 points of work**, with the administration tools landing in
  February 2027 at the earliest.
- **Screen C4 changes** and a design issue is needed.
- **Residual risk.** A model steered by hostile text can still run class 1
  tools within its grant, and a model with a shell can bypass Sobr: through the
  local files on the personal space, and through the organization tokens that
  the app stores (a recent sign-in keeps it from approving, not from calling
  the admin API). The `read` default, the full old values in the audit entry
  and a lower write limit reduce it. Aggregates asked over overlapping periods
  can still be differenced; the 7-day minimum limits it.

**Neutral**

- Tools only in v1; resources and prompts can be added later as minor
  versions.
- The Claude Code usage capture (hooks, logs, OpenTelemetry) is another
  project and does not go through this MCP: this one lets Claude ask Sobr,
  that one tells Sobr what Claude used.
- ADR-0019 decision 4 needs no amendment: its condition is settled here.

## Links

- [Routes, screen by screen](https://claude.ai/artifact/Cua7SK84VzMCwYFZRQfUHD) (route 9; A5, A7, Z10, Z11)
- [App and screen architecture](https://claude.ai/artifact/5UaiKHXrKPVyRPEd6gfeyr)
- ADR-0012 (audit ledger), ADR-0013 (bridge, pairing), ADR-0014 (personal
  space, single binary), ADR-0015 and ADR-0016 (privacy, visibility),
  ADR-0019 (repo split, PR #2, decision 4), ADR-0020 (rename; signed data is never rewritten),
  ADR-0021 (single app, roles `user` and `admin`), ADR-0022 (data model,
  PR #18)
- MCP specification, revision 2026-07-28:
  [transports](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports),
  [Streamable HTTP](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http),
  [authorization](https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization),
  [tools](https://modelcontextprotocol.io/specification/2026-07-28/server/tools),
  [elicitation](https://modelcontextprotocol.io/specification/2026-07-28/client/elicitation)
- Claude Code documentation:
  [MCP](https://code.claude.com/docs/en/mcp),
  [permissions](https://code.claude.com/docs/en/permissions)
- Issues: SOB-144 (this ADR), SOB-152 (its eight points), SOB-143 (data model ADR), SOB-58 (C1 to C4
  design), SOB-114 and SOB-115 (policy gaps), SOB-22, SOB-25, SOB-26,
  SOB-27, SOB-30 (Deep cleanups), SOB-65 (repo split)
