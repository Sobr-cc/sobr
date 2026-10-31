# ADR-0025 — Session identity and duplicates across sources

- **Status**: Proposed (2026-10-31). Two points wait for Thibault's answers
  (SOB-195); the rest rests on the recommendations below. Merging the PR
  is his approval.
- **Date**: 2026-10-31
- **Deciders**: Thibault (decision); CTO recommendation
- **Technical context**: Data model and sessions project (SOB-146)
- **Related**: ADR-0015 and ADR-0016 (privacy, visibility), ADR-0019
  (repos), ADR-0022 (the session model, which this ADR completes), the
  Claude Code capture note (`docs/sources/CLAUDE-CODE-USAGE-CAPTURE.md`,
  SOB-145)

> **Summary.** A session has one Sobr ID, a random UUID born on the device,
> and a set of **source references**: the IDs that each source uses for it.
> The references never leave the device. Two reports belong to one session
> only when a reference proves it; Sobr never guesses from times or sizes.
> A source's report for a session and a day replaces its own previous
> report, so a replay changes nothing. The numbers of a session come whole
> from the most precise source that has numbers (OpenTelemetry, then the
> transcript, then the browser extension), and the other reports are kept as
> history and as context. A source that cannot name a session, such as
> billing, never changes a session or a total: it is shown beside them, with
> the gap. When a more precise source arrives later, it replaces the numbers
> and the older report stays in the history. A correction by the person
> belongs to the session, so it survives every change of winner. The merge
> runs on the device; the server receives one winning report per session and
> day.

## Points to decide

Thibault answers these two points in the Decision issue SOB-195. Each
choice rests on the matching part of the Decision section, which compares the
options with their cost, effort and risk.

1. **How billing enters the totals** (§6). Billing reports cover an actor or
   a key per model and per hour or day, never a session. Recommended: show
   the billed figure beside the total Sobr saw, with the gap and the ratio;
   change no session and no total. Alternatives: let billing set the total
   through a signed entry; or let billing replace the total and scale the
   sessions.
2. **Where OpenTelemetry enters** (§8). Recommended: a receiver in the
   desktop app on the same machine, so the merge happens on the device.
   Alternatives: the organization's collector forwarding to the server with
   Claude Code's session ID in the upload; or the same with a keyed hash of
   that ID. This changes the recommendation of the capture note (SOB-145,
   section 3, option 2).

The other choices are mine to recommend and are justified where they are made.
Whatever Thibault answers, the identity rule of §3 and the tests of §11 stay.

## Context and problem statement

ADR-0022 left two things open: how reports from several sources form one
session, and what happens to billing, which "covers a workspace or a key per
day rather than one person's use". Without one rule, the same use is counted
twice and every total people see is wrong. The decision of October 30
(*Routes, screen by screen*, "Doublons de mesure") gives the outline: "Un
identifiant par session ; la source la plus précise donne les chiffres
(facturation, puis Claude Code, puis extension) ; le contexte des sources est
fusionné". Screens T1 to T3 show the result, and T3 offers "chiffres et
contexte fusionnés des sources".

### What each source can say

The facts about Claude Code are in the capture note and link to the official
pages. The billing facts below were read on the official pages on 2026-10-31.

| Source | Who can read it | What one report is | Names a session? |
|--|--|--|--|
| Claude Code transcript (`claude-code-log`) | The person's device | Turns of one session, in an unofficial format | Yes: the file name is the session ID |
| Claude Code status line and hooks (`claude-code-statusline`) | The person's device | A cost total and the last call's token counts, plus context | Yes: `session_id`, and `transcript_path` |
| Claude Code OpenTelemetry (`claude-code-otel`) | Whoever receives the export | Token and cost counters with the `session.id` attribute | Yes, but the documentation does not say that `session.id` equals `session_id` (SOB-163) |
| Browser extension (`extension`) | The person's browser | An estimate from the tokens it counts in the browser | Its own conversation, found in the page address, which stays local |
| Billing (`billing`) | An organization admin with an Admin credential | Tokens and cost per model, actor or key, and bucket | No |

What the three Anthropic pages say about billing:

- The [Usage and Cost API](https://platform.claude.com/docs/en/manage-claude/usage-cost-api)
  reports organization usage by model, workspace and API key, in buckets of
  one minute, one hour or one day. It needs an Admin credential and "is
  unavailable for individual accounts". It is the legacy API.
- The [Usage and Cost Analytics API](https://platform.claude.com/docs/en/manage-claude/usage-cost-analytics-api)
  (beta) adds the actor behind each request, a user or a service account.
  Buckets are in UTC. Data "typically refreshes within 4 hours and can take up
  to 24 hours", and each answer carries `data_refreshed_at`. The answer lists
  a `session_id` dimension; the page does not say what it identifies, so Sobr
  does not rely on it.
- The [Claude Code Analytics API](https://platform.claude.com/docs/en/manage-claude/claude-code-analytics-api)
  gives one record per actor and UTC day, with a count of sessions and, per
  model, tokens and an estimated cost. It has no session ID, and "does not
  include" usage on a Claude subscription plan.

So billing is a total per actor or key, model and bucket. It cannot be tied to
one session, and a person on a subscription, or on no organization, has none.
The decision of October 30 assumed a session-level billing source; this ADR
keeps its order of precision and applies each source at the grain it has.

### What is out of scope

- Code, and the sources' own capture rules (SOB-145, SOB-161, SOB-164).
- The entities (ADR-0022), except the amendment that closes its references to
  this ADR.
- Billing per person. It needs the identities of the Organization and
  identity project, and ADR-0016 decides who may see a person's daily figure.
- Whether an estimate made in the Understand pages counts as use (ADR-0022).

## Decision

### 1. The session ID and its source references

**A session has one ID: a random UUID (version 4), born on the device when
the first report arrives** (ADR-0022 section 9). It never changes. Next to it,
the device keeps a set of **source references**: pairs of a source family and
the key that the source gives the session.

- Family `claude-code`: the Claude Code session ID, as the transcript file
  name and as the `session_id` of the status line and the hooks.
- Family `claude-code-otel`: the `session.id` attribute of OpenTelemetry. It
  is the same family as `claude-code` if SOB-163 finds the two IDs equal, and
  a family of its own if not (section 3).
- Family `extension`: the provider host and the conversation ID found in the
  page address.

Only the families `claude-code` and `extension` can create a session.

References are local context. They stay in the device's session store, in the
list of what never leaves a device (ADR-0022 section 10). The server knows a
session only by its Sobr ID.

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — A random Sobr ID, with source references kept on the device (chosen)** | A lookup table on the device, and a merge step when two references turn out to name one session | Medium | Low: nothing a third party issued leaves the device |
| B — The source's own session ID is the Sobr ID | No lookup | Small | A browser conversation has no such ID; two Claude Code sources with different IDs split a session; the server stores an ID that anyone holding Claude's logs can match |
| C — An ID derived by hashing the source key | Same IDs on every machine | Small | A hash of a known ID is still matchable, and two keys for one session still give two IDs |

### 2. What each source maps to

| Source | Reference it gives | How the report becomes a measurement |
|--|--|--|
| `claude-code-log` | Claude Code session ID, from the file name. A subagent transcript (`<session>/subagents/`) carries its parent's ID | Cumulative totals per day and model, read at the end of the session or on import |
| `claude-code-statusline` | `session_id`, and the transcript file name from `transcript_path` | No numbers: it supplies context and the live view. Its cost total and last-call tokens cannot stand alone (capture note, section 2) |
| `claude-code-otel` | `session.id`: family `claude-code` if SOB-163 finds it equal to the other IDs, otherwise its own family, which creates no session | Counters turned into cumulative totals per day and model by the receiver |
| `extension` | Provider host and conversation ID | The estimate for that conversation and day |
| `billing` | None | Not a measurement. See section 6 |

A report is always a **cumulative total for a session, a source and a day,
with one row per model**. An adapter that receives increments, such as the
OpenTelemetry counters, adds them up before it reports.

### 3. The linking rule

This rule holds whether Claude Code uses one session ID in all its sources or
not. **That fact is to be confirmed by SOB-163**; the rule does not wait for it.

1. A report arrives with a reference. If the device knows it, the report
   belongs to that session.
2. If the device does not know it and its family can create a session
   (`claude-code` or `extension`), a new session is born with the reference.
3. If the device does not know it and its family cannot create a session, the
   report is a **totals-only report** (section 6). It never creates a session.
4. A report may carry two references that the source itself ties together. The
   status line and the hooks carry `session_id` and `transcript_path`, and the
   file name of the transcript is its session ID. Both references are
   recorded for one session. If they belong to two sessions, the sessions
   are merged (section 9).
5. Nothing else links reports. No match on time, model, size or folder.

**If SOB-163 finds one ID everywhere,** OpenTelemetry joins the family
`claude-code`, and step 1 does all the work: the status line, the hooks, the
transcript and OpenTelemetry meet on one reference.

**If SOB-163 finds different IDs,** the status line, the hooks and the
transcript still meet through step 4. OpenTelemetry stays in its own family and
has no tie to them, so its reports name a session the device cannot match. By
step 3 they are totals-only at the person's scope: their day total is a check
against the sessions' sum and adds nothing. Sobr takes the transcript's
numbers, one rank lower, and loses no use. Whether OpenTelemetry alone may
create sessions on a machine that has no transcript reader is left to the
amendment that follows SOB-163.

**A wrong link hides use and a missing link only loses precision,** so the rule
prefers the missing link. Nothing in the totals can be counted twice by a link
that was not made, because a report that cannot be linked adds nothing.

Two more facts wait for SOB-163 and SOB-161: whether a resumed session keeps
its ID or starts a new one that repeats earlier lines, and whether a subagent's
requests carry the parent's ID in OpenTelemetry. The tests of section 11 pin
both, so the importer never adds a subagent to a parent that already includes
it. A session after `/clear` is a new session, per the Claude Code
documentation.

### 4. One report per session, source and day

A source's report for a session and a day, with its rows per model, is
identified by (session, source, day). A new report for the same identity
**replaces the previous one whole**. This covers three cases at once: the
transcript read again after more turns, the extension's estimate sent to both
the app and the server, and a replayed upload. Applying a report twice changes
nothing.

Cumulative totals do not shrink. A report lower than the stored one for the
same identity is ignored and flagged, because the source was cut (for example a
transcript cleaned after 30 days) and the stored report is the better record.

### 5. Precedence: which report gives the numbers

**For each session and day, the numbers come whole from one source: the
most precise one that has numbers.** The order is:

| Rank | Source | Why |
|--|--|--|
| 1 | `claude-code-otel` | Official counters, per request, with cost |
| 2 | `claude-code-log` | Token counts from the transcript, unofficial, best effort |
| 3 | `extension` | An estimate from tokens counted in the browser |
| — | `claude-code-statusline` | No token totals: context and live view only |

Billing sits above this table in precision but at another grain (section 6).
The order is a table in the model crate, changed only by an amendment of this
ADR. The energy, CO₂ and water of a session are computed from the winning
report's tokens and model with the method and its version (ADR-0004), so a
change of winner recomputes them and nothing else.

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — The numbers come whole from the best source that has numbers (chosen)** | A source with a gap in its fields loses to a lower one only when it has no report at all | Small | Low: one definition of every figure of a session |
| B — Each field comes from the best source that has it (tokens from one, cost from another) | Fewer holes | Medium | A session mixes definitions: an official counter beside an estimate at list price, and totals that no source ever stated |

While a session runs, the status line may show a live figure. It is marked
live, and the session's numbers replace it when a report with numbers
arrives.

### 6. Sources that cannot name a session: billing

A **totals-only source** reports a total for a scope, a model and a day, with
no session. Billing is one, at the scope of an organization or a workspace.
An unlinked OpenTelemetry stream is another, at the scope of a person
(section 3).

**Rule: a totals-only source never creates a session, never changes a session
and never changes a total of sessions.** It supplies its own figure for its
scope and day, and the gap to what Sobr saw.

For billing, the server fetches the hourly buckets with the organization's
Admin credential (the Z2 connection), cuts them into the calendar day of the
space's time zone (ADR-0022 section 10), and stores one billed figure per
space, model and day. A figure is provisional until `data_refreshed_at` is
later than the end of that day. The organization view then shows three
numbers for a day: what Sobr saw, what was billed, and the gap. When Sobr saw
more than the bill, the gap is shown as a ratio ("estimates 8 % above the
bill") and nothing is subtracted.

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — The billed figure is shown beside the total Sobr saw, with the gap (chosen)** | Two figures to read; the gap mixes members who do not use Sobr, service keys and Sobr's own errors | Small | Low: no session or total is rewritten; the choice can move to B later because the billed figure is stored either way |
| B — Billing sets the day total, and a signed entry holds the difference | One total; a negative entry needs explaining | Medium | Low, but the total then includes use by non-members, which changes what "organization total" means for projects and k |
| C — Billing replaces the day total and the sessions are scaled to it | One exact total | Medium | Per-session numbers that nobody measured: the invented precision that ADR-0022 refused for splitting a session across tasks |

The billed figure has no member and no project, so it adds no contributor to
any k count and appears only in the organization total of ADR-0015 and
ADR-0016, not in a breakdown.

A zone whose offset is not a whole number of hours cannot be cut from hourly
buckets. Its day is cut from one-minute buckets, which the API limits to 24
hours per request.

### 7. Merging context

Context is what a report knows beyond its numbers: folder, repo, branch,
source account, model, Claude Code version, terminal, provider host, and the
conversation. **The context of a session is the union of the context of all its
reports.**

- When two reports give the same field, the more precise source wins.
- A missing value never erases a known one.
- Most context stays on the device (ADR-0022 section 10). What can leave is
  already in the upload contract: the model, the provider host and the day.

The merge is repeated when a report arrives. A session therefore gains
context from every source that sees it, and T3 shows the merged numbers and
context.

### 8. Where the merge runs, and what is uploaded

**The merge runs on the device.** The device sees every source of the person.
It uploads, in its daily batch, **the winning report of each session and
day**, under the contract of ADR-0022 section 10. It uploads no reference and
no losing report.

The server applies the same precedence function from the model crate, so a
late or repeated upload cannot count twice: a report replaces one of lower or
equal rank for the same session and day, and is ignored if lower. The
replaced report stays in the history (section 10).

This ADR adds one item to the contract: **a session merge**, carrying the
retired session ID and the surviving one (section 9). Both are random IDs, so
it holds no text and no time.

OpenTelemetry follows from this:

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — The desktop app receives OpenTelemetry on the same machine, so it merges like any other source (chosen)** | The user, or the managed settings, point `OTEL_EXPORTER_OTLP_ENDPOINT` at the local app; a member without the app is not measured | Medium | Low: no new field leaves the device, and OpenTelemetry works on any plan, also for people alone |
| B — The organization's collector forwards to the server, and the upload carries Claude Code's session ID so that the server can merge | One collector for the organization, members need no receiver | Medium | The contract gains a third-party ID that anyone holding Claude's logs can match; the server must merge too, so two merge points |
| C — As B, with a keyed hash of the ID, the key held by the organization | Same as B, with the key to distribute to collector and devices | Large | A shared secret to rotate; still two merge points |

The capture note (SOB-145, section 3) recommended a collector for
organizations. Under A it becomes a way to deliver the export settings, not a
path to the server. SOB-166, which researches the collector, is reframed
accordingly if Thibault takes A.

### 9. Corrections by the person, and merges

An attachment (project, task, "no project") belongs to the **session**, not to a
report. It does not change when the winner changes or when a report arrives.

- **Order of authority:** the person, then a rule, then a project link, then
  the source (ADR-0022 sections 4 and 6). A later decision of a lower authority
  never overrides a higher one.
- **Rules run on the merged context** (ADR-0022 section 6). A rule is run again
  when a session gains context, only on sessions still "to classify" and on
  attachments that a rule or a source made. It never touches an attachment the
  person chose.
- **A correction made in T3 can become a rule applied to history.** The rule's
  conditions (folder, repo, branch, source account, model) read the merged
  context, so a session that gained its folder from a later report is matched
  from then on.

**Merging two sessions.** When step 2 of section 3 shows that two sessions are
one, the older session survives. The newer ID is retired and recorded as
merged into it, and its references and reports move to the survivor. For the
attachment, the person's choice wins; if both were chosen by the person, the
more recent choice wins. A merge before the session is uploaded changes
nothing on the server. After the upload, the device sends the merge in its
daily batch, and the server moves the reports and deletes the retired
session. Totals count the session once on the next day's view.

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — Merge sessions, the older ID survives, the person's choice wins (chosen)** | A merge change in the contract | Medium | Low: IDs are random, so the choice of survivor is arbitrary but fixed |
| B — Keep both sessions and hide one | No change in the contract | Small | Two rows for one session, the totals depend on which one is hidden, and a correction applies to one only |

### 10. When a more precise source arrives later

Sources arrive in any order, and some arrive late by nature: OpenTelemetry is
turned on after the first sessions, a transcript is imported days later,
billing refreshes up to a day later.

**The numbers are derived, never overwritten.** Each report is kept, the
winner is the best-ranked one, and the session shows the winner's numbers. When
a better report arrives, it becomes the winner and the previous winner is kept
as **superseded**. When a report of lower rank arrives, nothing visible
changes: it is kept as history and its context is merged.

The history keeps for each report: the source, the day, the rows per model,
the method and its version, and, for a superseded report, the day it was
replaced. It keeps no clock time on the server. The device keeps its own clock
times for the person's views. History follows the retention of ADR-0022
section 10 (24 months). T3 shows "estimated 12,000 tokens from the extension,
measured 15,300 by the transcript on Tuesday".

| Option | Cost | Effort | Risk |
|--|--|--|--|
| A — Overwrite the numbers and keep nothing | The least storage | Small | A total changes with no explanation, and a wrong replacement cannot be undone |
| **B — Keep every report, derive the winner, mark replaced ones (chosen)** | One row per source and day, at most three per session and day | Small | Low |
| C — Freeze the first report | Totals never move | Small | The least precise number stays for good |

A replacement changes a past day's total. ADR-0022 already accepts this for a
late batch. A replacement never changes **who** contributed to a group on a
day, so it can never make a group pass or fail the k check (ADR-0022 section 10).
It can reveal the difference between two values to someone who saw both; the
same trade-off as a withdrawal, and the same remedy: organization views show a
day after the batches and the billing refresh of that day.

### 11. Tests that prove no double counting

The model crate publishes these cases with its release, as ADR-0022 section 3
does for the space rule, so the extension and every other client run them.
Property tests (`proptest`) cover the order of arrival.

1. A report applied twice gives the numbers of applying it once.
2. Two sources for one session, in both orders: the numbers are the winner's,
   and the day total is the winner's, never the sum.
3. Three sources, in all six orders: the same numbers and the same history.
4. A better source arrives later: the numbers change, the older report is
   superseded, and the day total moves by the difference only.
5. A worse source arrives later: nothing visible changes; the context is
   merged.
6. A lower cumulative report than the stored one is ignored and flagged.
7. A status line alone adds nothing to any total.
8. A subagent transcript and its parent give the parent's totals once.
9. Two sessions merged after a late link: one session, the person's
   attachment kept, the totals counted once, before and after the upload.
10. Case I and case II of section 3, with the same inputs: totals equal in
    both, except that case II uses rank 2 numbers where case I used rank 1.
11. An unlinked totals-only stream adds nothing to the totals.
12. Billing: a day with a gap, a day where Sobr saw more than the bill, a day
    with no billing, and a day not yet refreshed. No session and no total
    changes in any of them.
13. The extension alone (no app): its one source follows rules 1 and 4 only.
14. A correction and a rule: the person's attachment survives a change of
    winner and a re-run of the rules.

## Examples

**Three sources for one session, in any order.** Alice uses the app in her
personal space. On Monday, a Claude Code session S sends the status line
(cost $0.80, last call 2,100 tokens: context only), then the transcript at the
end (118,000 input and 31,000 output tokens: 149,000), then OpenTelemetry
through the app's receiver (120,500 and 31,200: 151,700).

1. The status line adds nothing. The transcript, rank 2, gives S 149,000
   tokens.
2. OpenTelemetry, rank 1, arrives and wins: S is 151,700. The transcript
   report is kept, superseded.
3. Alice's day total counts 151,700, not 300,700. Had OpenTelemetry arrived
   first, the transcript would have been kept as history and nothing would have
   changed. Her correction of S to the task "Fix the RSS feed" stays the same
   through all three steps.

**Billing beside the total, in an organization.** Acme's space uses Europe/Paris
as its time zone. For Monday, its members' winning reports sum to 4,200,000
tokens for the model M. The server cuts the hourly UTC buckets into the Paris
day: billed 4,650,000.

1. The organization view shows: seen 4,200,000, billed 4,650,000, gap 450,000.
   The gap holds service keys, members who do not use Sobr, and Sobr's own
   misses. No session and no project total moves.
2. For Tuesday, Sobr saw 3,000,000 and the bill says 2,700,000. The view shows
   the ratio "estimates 11 % above the bill". Nothing is subtracted.
3. For Wednesday, `data_refreshed_at` is still before the end of the day. The
   billed figure is provisional and no gap is shown.

**A late link and a replay.** Erin finds, with Claude Code, that the status line
and the transcript use different IDs (case II). She works on Monday.

1. The transcript is imported first: session B is born with the reference "log
   B". She attaches B to the project "Checkout".
2. The status line later reports `session_id` A with the transcript path of B.
   Step 2 of section 3 records A and B as references of one session; nothing is
   merged, because A had no session of its own yet.
3. Had A already had a session, the older would survive, Erin's attachment
   would win, the two sets of reports would be joined, and the totals would
   count the session once.
4. Her browser extension, still in the old `both` mode, sends the same
   conversation estimate to the app and to the server. The report has one
   identity, (session, `extension`, day), so the second copy replaces the first
   and nothing is counted twice.

## Consequences

**Positive:**

- One rule answers "is this the same use?" for every source, with a proof or no
  link at all.
- Totals can never be doubled by a source, an order of arrival or a replay.
- Billing keeps its authority at the grain it has, without inventing a number
  for a session.
- Nothing new leaves the device except a session merge, which holds two random
  IDs.
- The history explains every change of a number.

**Negative:**

- A report that cannot be linked is left out of the totals, so Sobr can
  undercount in case II until the IDs are known. The gap against billing shows
  it.
- The merge needs the device: a member without the app and without the
  extension is not measured, except through billing.
- A past day's total can change when a better report arrives.
- The organization view has two figures to read.
- The server and the device both run the precedence function. The crate holds
  one implementation and the same published cases.
- SOB-166 and part of the capture note (section 3, option 2) are reframed if
  Thibault takes option A for OpenTelemetry.

**Neutral:**

- The order of precision is a table in the crate and changes by amendment.
- Billing per person waits for the Organization and identity project and for
  ADR-0016.

## Issues to create

The orchestrator creates these issues once this ADR is accepted. "Waits for"
names the Deep cleanup of every existing area an issue touches, under the
Foundations rule. The screens (T1 to T3) wait for Design V7 and are not listed.
The numbers refer to the issues of ADR-0022.

| # | Issue | Area | Estimate | Waits for |
|--|--|--|--|--|
| 1 | Add source references, the report identity, the precedence function, the merge and the published cases to the model crate, with property tests | Model crate | 5 | ADR-0022 issue 1 |
| 2 | Turn the Claude Code transcript import into cumulative reports with subagent folding, and pin the resumed-session case | Import crate, desktop backend | 5 | #1; SOB-161; **SOB-26** |
| 3 | Make the status line and hooks report context and the live view only, with the two references of step 2 | Plugin (`claude-code` repo) | 3 | #1; SOB-164 |
| 4 | Receive OpenTelemetry in the desktop app (local endpoint, allow-list, increments turned into totals) and ship the export settings | Desktop backend, plugin | 5 | #1; the answer to point 2; SOB-166 reframed; **SOB-26** |
| 5 | Make the extension send one report per conversation and day, and keep its conversation reference local (with ADR-0022 issue 9) | Browser extension, bridge | 3 | #1; ADR-0022 issue 9; **SOB-31**, **SOB-24**, **SOB-26** |
| 6 | Keep winning reports and their history on the server, apply the precedence on upload, and accept a session merge | Server | 5 | #1; ADR-0022 issues 6 and 7; **SOB-25** |
| 7 | Apply rules again when a session gains context, and merge two sessions with the person's attachment winning | Model crate, desktop backend | 3 | #1; ADR-0022 issue 5; **SOB-26** |
| 8 | Fetch billing for an organization (Z2 connection, hourly buckets cut to the space's day, provisional until refreshed) and show the billed figure and the gap | Server, integrations | 8 | The answer to point 1; ADR-0022 issue 6; **SOB-25** |
| 9 | Update the capture note with the result of SOB-163 and this ADR | Docs | 1 | SOB-163 |

## Links

- [Routes, screen by screen](https://claude.ai/artifact/Cua7SK84VzMCwYFZRQfUHD):
  "Doublons de mesure"; screens T1 to T3
- ADR-0004, ADR-0015, ADR-0016, ADR-0019, ADR-0022 (Amendment 1 closes its
  references to this ADR)
- `docs/sources/CLAUDE-CODE-USAGE-CAPTURE.md` (SOB-145), SOB-161, SOB-163,
  SOB-166, SOB-118 (the `both` mode double count), SOB-87 (amendment by PR)
- Official pages read on 2026-10-31: [Usage and Cost API](https://platform.claude.com/docs/en/manage-claude/usage-cost-api),
  [Usage and Cost Analytics API](https://platform.claude.com/docs/en/manage-claude/usage-cost-analytics-api),
  [Claude Code Analytics API](https://platform.claude.com/docs/en/manage-claude/claude-code-analytics-api)
