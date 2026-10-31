# ADR-0025 — Session identity and duplicates across sources

- **Status**: Proposed (2026-10-31). Thibault answered the two open points on
  2026-10-31 (SOB-195) and took the recommendation of each. Merging the PR is
  his approval.
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
> A source's report for a session replaces its own previous report. The
> numbers of a session come whole from the most precise source whose report
> is complete (OpenTelemetry, then the transcript, then the browser
> extension), and the other reports are kept as history and context. A
> source that cannot name a session, such as billing, never changes a session
> or a total: it is shown beside them, with the gap. When a more precise
> source arrives later, it replaces the numbers and the older report stays in
> the history. A correction by the person belongs to the session, so it
> survives every change of winner. The merge runs on the device; the server
> stores the winning report of each session and keeps the ones it replaced as
> history.

## Points decided

Thibault answered these two points on October 31 in the Decision issue
SOB-195, and took the recommendation of each. Each rests on the matching
section, which compares the options with their cost, effort and risk.

1. **How billing enters the totals** (section 6). The billed figure is shown
   beside the total Sobr saw, with the gap and the ratio; no session and no
   total changes. Chosen. Alternatives: a signed entry that makes the total
   equal the bill; or the bill replacing the total, with the sessions scaled.
2. **Where OpenTelemetry enters** (section 8). The desktop app receives it on
   the same machine, so the merge stays on the device. Chosen. Alternatives:
   the organization's collector forwarding to the server with Claude Code's
   session ID in the upload; or the same with a keyed hash of that ID. This
   changes the recommendation of the capture note (section 3, option 2).

The other choices are the CTO's recommendation, justified where they are made.
The identity rule of section 3 and the tests of section 11 do not depend on
these two answers.

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

The facts about Claude Code are in the capture note (sections 1 to 4) and link
to the official pages. The billing facts below were read on the official pages
on 2026-10-31.

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
  model, tokens and an estimated cost. It has no session ID, and it does not
  include usage on a Claude subscription plan.

So billing is a total per actor or key, model and bucket. It cannot be tied to
one session, and a person on a subscription, or on no organization, has none.
The decision of October 30 assumed a session-level billing source; this ADR
keeps its order of precision and applies each source at the grain it has.

The extension and Claude Code are two tools. No official source ties a browser
conversation to a Claude Code session, so today they never share a session.

### What is out of scope

- Code, and the sources' own capture rules (SOB-145, SOB-161, SOB-164).
- The entities (ADR-0022), except the amendment that closes its references to
  this ADR.
- Billing per person. It needs the identities of the Organization and
  identity project, and ADR-0016 decides who may see a person's daily figure.
- Whether an estimate made in the Understand pages counts as use (ADR-0022).
- What local context the device keeps for rules (folder, repo, branch):
  ADR-0022 section 6 uses it and the capture note says it is not stored
  (SOB-198).

## Decision

### 1. The session ID and its source references

**A session has one ID: a random UUID (version 4), born on the device when
the first report arrives** (ADR-0022 section 9). It never changes. Next to it,
the device keeps its **source references**: pairs of a source family and the
key that the source gives the session.

| Family | Key | Creates a session? |
|--|--|--|
| `claude-code` | The Claude Code session ID: the transcript file name, and the `session_id` of the status line and the hooks | Yes |
| `claude-code-otel` | The `session.id` attribute. It is the family `claude-code` if SOB-163 finds the two IDs equal | Only if it is the family `claude-code` |
| `extension` | The provider host and the conversation ID found in the page address | Yes |

References are local context. They stay in the device's session store, in the
list of what never leaves a device (ADR-0022 section 10). The server knows a
session only by its Sobr ID.

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — A random Sobr ID, with source references kept on the device (chosen)** | A lookup table on the device, and a merge step when two references turn out to name one session | Medium | Low: nothing a third party issued leaves the device |
| B — The source's own session ID is the Sobr ID | No lookup | Small | A browser conversation has no such ID; two Claude Code sources with different IDs split a session; the server stores an ID that anyone holding Claude's logs can match |
| C — An ID derived by hashing the source key | Same IDs on every machine | Small | A hash of a known ID is still matchable, and two keys for one session still give two IDs |

### 2. What each source maps to

| Source | How it becomes a report |
|--|--|
| `claude-code-log` | Cumulative totals, read at the end of the session or on import. A subagent transcript (`<session>/subagents/`) is read as part of its parent |
| `claude-code-statusline` | No token totals, so it cannot be a report with numbers: it supplies context and the live view. Its cost total is the live display (capture note, section 2) |
| `claude-code-otel` | Counters turned into cumulative totals by the receiver |
| `extension` | The estimate for that conversation |
| `billing` | Not a report. See section 6 |

A **report** is a source's cumulative totals for one session, up to the time
it was read, with one measurement (ADR-0022) per day and model. Its day is the
calendar day, in the space's time zone, of the record it counts. The device
keeps the record times and computes the days when the space is known, and again
if the session is withdrawn to another space; a session whose space is pending
has no days yet and uploads nothing (ADR-0022 section 3). A report is
identified by (session, source). An adapter that receives increments, such as
the OpenTelemetry counters, adds them up before it reports.

### 3. The linking rule

This rule holds whether Claude Code uses one session ID in all its sources or
not. **That fact is to be confirmed by SOB-163**; the rule does not wait for it.

1. A report arrives with one reference, or with two that the source itself
   ties together: the status line and the hooks carry `session_id` and
   `transcript_path`, and the file name of the transcript is its session ID.
   The device looks up all of them before it acts.
2. If it knows one session, the report belongs to it, and the references it
   did not know are recorded for that session.
3. If it knows none, a family that can create a session starts a new one with
   the references. Any other report is a **totals-only report** (section 6)
   and never creates a session.
4. If the references name two sessions, the sessions are merged (section 9).
5. Nothing else links reports. No match on time, model, size or folder.

**Case I — one ID everywhere.** OpenTelemetry joins the family `claude-code`.
Steps 1 and 2 do all the work: the status line, the hooks, the transcript and
OpenTelemetry meet on one reference.

**Case II — different IDs.** The status line, the hooks and the transcript
still meet through step 1. OpenTelemetry stays in its own family and has no
tie to them, so by step 3 its reports are totals-only: their total is a check
kept on the device, and adds nothing. Sobr takes the transcript's numbers, one
rank lower. Whether OpenTelemetry alone may create sessions on a machine with
no transcript reader, and so serve people alone as option A of section 8
intends, needs a mode choice that the amendment following SOB-163 defines.

**A wrong link hides use and a missing link only loses precision,** so the rule
prefers the missing link. A link that was not made never adds a report to a
session that is not its own.

Two cases are the importer's, not the identity rule's, and break it if
unchecked. A resumed session may start a new ID and repeat earlier lines; SOB-163
covers resume and `/clear` (a session after `/clear` is a new session, per the
Claude Code documentation). Whether a subagent's requests carry the parent's ID
in OpenTelemetry is not in the capture note; build issue 4 checks it with a
real run. The tests of section 11 pin both, so a subagent is never added to a
parent that already includes it.

### 4. One report per session and source

A new report for the same (session, source) **replaces the previous one
whole**. This covers the transcript read again after more turns, an extension
estimate that reaches the app twice (once live, once from the spool after the
app was closed), and a replayed upload. Applying a report twice changes
nothing. ADR-0022 section 10 already removes the extension's `both` mode, so
one estimate no longer takes two paths.

Cumulative totals do not shrink. A report whose total tokens are lower than the
stored one for the same source is ignored and flagged, because the source was
cut (for example a transcript cleaned after 30 days) and the stored report is
the better record. A receiver keeps its totals on disk, so a restart does not
lower them. A flagged report that is wrong can be reset by the person, which
deletes that source's report for the session; the next one is then accepted.

### 5. Precedence: which report gives the numbers

**For each session, the numbers come whole from one source: the most precise
source whose report is complete.** The days of the session are the days of that
report.

| Rank | Source | Why |
|--|--|--|
| 1 | `claude-code-otel` | Official counters with cost |
| 2 | `claude-code-log` | Token counts from the transcript, unofficial, best effort |
| 3 | `extension` | An estimate from tokens counted in the browser |
| — | `claude-code-statusline` | No token totals: context and live view only |

The extension has no tie to Claude Code today (see Context), so for now its
numbers are the only ones of its sessions. Its rank is kept because the
decision of October 30 orders it so and a tie may appear; adding one needs an
amendment with an official source.

**A report is complete** when it starts no later than the earliest record of any
report with numbers for the session plus 5 minutes, and ends no earlier than the
latest record minus 5 minutes. The tolerance is fixed here and held in the
crate, so every client gets the same result; changing it needs an amendment.
The times are the device's and never leave it. Completeness depends on the set
of reports, not on the order they arrived in. OpenTelemetry started after the
session began, or an app closed for an hour, gives a partial report: it does not
beat a complete transcript. If no report is complete, the report with the most
tokens wins and the session is flagged partial, so use is never dropped for the
sake of precision. Models are not part of the test: an internal call that one
source records and another lacks is a difference between the sources, not a
gap.

The order of precision is a table in the model crate, changed only by an
amendment of this ADR. The energy, CO₂ and water of a session are computed from
the winning report's tokens and model with the method and its version
(ADR-0004), so a change of winner recomputes them and nothing else.

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — The numbers come whole from the best complete source (chosen)** | A source with a gap loses to a lower one that has none | Small | Low: one definition of every figure of a session, and no day or model counted from two sources |
| B — Each field comes from the best source that has it (tokens from one, cost from another) | Fewer holes | Medium | A session mixes definitions, and a turn near midnight can land on two days |

While a session runs, the status line may show a live figure. It is marked
live, and the session's numbers replace it when a report with numbers arrives.

### 6. Sources that cannot name a session: billing

A **totals-only report** gives a total for a scope, a model and a day, with no
session. Billing is one, at the scope of an organization or a workspace.
OpenTelemetry in case II is another.

**Rule: a totals-only report never creates a session, never changes a session
and never changes a total of sessions.** It is shown beside them, with the gap.

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

Context is what a report knows beyond its numbers: source account, model,
Claude Code version, terminal, provider host, the conversation, and, while the
device holds them, folder, repo and branch. **The context of a session is the
union of the context of all its reports.**

- When two reports give the same field, the more precise source wins.
- A missing value never erases a known one.
- Context stays on the device except what the upload contract already holds:
  the model, the provider host and the day (ADR-0022 section 10).
- How long the device keeps folder, repo and branch is open (SOB-198). This
  ADR merges what is kept and adds nothing to it.

The merge is repeated when a report arrives, so a session gains context from
every source that sees it, and T3 shows the merged numbers and context.

### 8. Where the merge runs, and what is uploaded

**The merge runs on the device.** The device sees every source of the person
on that machine. It uploads, in its daily batch, **the winning report of each
session**, under the contract of ADR-0022 section 10. It uploads no reference
and no losing report.

**The device decides the winner; the server does not rank.** Completeness uses
times that never leave the device, so the server could not judge it. The server
stores the report it receives for a session in place of the one it held, and
keeps the replaced one as history (section 10). An upload that repeats the
stored report changes nothing, so a retry cannot count twice.

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
accordingly (SOB-195).

### 9. Corrections by the person, and merges

An attachment (project, task, "no project") belongs to the **session**, not to a
report. It does not change when the winner changes or when a report arrives.

- **The person's choice is never overridden** by a rule, a project link or a
  source. Between automatic attachments, ADR-0022 sections 6 and 8 apply as
  written.
- **Rules run on the merged context** (ADR-0022 section 6). A rule is run again
  when a session gains context, only on sessions still "to classify" and on
  attachments that a rule or a source made.
- **A correction made in T3 can become a rule applied to history**
  (ADR-0022 section 6). Its conditions read the merged context, so a session
  that gained its folder from a later report is matched from then on.

**Merging two sessions.** When step 4 of section 3 shows that two sessions are
one, the older session survives. This can happen when a source ties two
references after each already has a session, for example a resumed session
whose new ID a later report ties to the old one; SOB-163 tells whether that
exists. A merge only happens inside one space, or between two sessions whose
space is still pending. Two sessions in different spaces are not merged and
the tie is flagged, because a session never moves from the personal space to an
organization (ADR-0022 section 3). The newer ID is retired and recorded as
merged into the survivor, and its references and reports move to it. If both
held a report from the same source, the one with more total tokens is kept and
the other becomes history; reports are never added together, because a resumed
session may repeat earlier lines. For the
attachment, the person's choice wins; if both were chosen by the person, the
more recent choice wins. A merge before the session is uploaded changes nothing
on the server. After the upload, the device sends the merge in its daily batch,
and the server moves the reports and deletes the retired session. Like any
attachment change, it can move a day's total, and the group of a member's use.

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — Merge sessions, the older ID survives, the person's choice wins (chosen)** | A merge change in the contract | Medium | Low: IDs are random, so the choice of survivor is arbitrary but fixed |
| B — Keep both sessions and hide one | No change in the contract | Small | Two rows for one session, the totals depend on which one is hidden, and a correction applies to one only |

### 10. When a more precise source arrives later

Sources arrive in any order, and some arrive late by nature: OpenTelemetry is
turned on after the first sessions, a transcript is imported days later,
billing refreshes up to a day later.

**The numbers are derived, never overwritten.** Each report is kept, the
winner is the best-ranked complete one, and the session shows the winner's
numbers. When a better report arrives, it becomes the winner and the previous
winner is kept as **superseded**. When a report of lower rank arrives, nothing
visible changes: it is kept as history and its context is merged.

**On the device,** the history keeps every report: the source, its rows per day
and model, the method and its version, the record times, and, for a superseded
report, the day it was replaced. **On the server,** it keeps each winner that a
later upload replaced, with the day of the replacement and no clock time.
History follows the retention of ADR-0022 section 10 (24 months). T3 shows, on
the device, "estimated 12,000 tokens from the extension, measured 15,300 by the
transcript on Tuesday".

| Option | Cost | Effort | Risk |
|--|--|--|--|
| A — Overwrite the numbers and keep nothing | The least storage | Small | A total changes with no explanation, and a wrong replacement cannot be undone |
| **B — Keep every report, derive the winner, mark replaced ones (chosen)** | One report per source and session, at most three | Small | Low |
| C — Freeze the first report | Totals never move | Small | The least precise number stays for good |

A replacement changes past days' totals, as a late batch does (ADR-0022
section 10). It can also move a record across midnight, because two sources may
date the same record to different days, so a day group can gain or lose a
contributor. That is the same trade-off as a late batch or a withdrawal, and the
same remedy: organization views show a day after the batches and the billing
refresh of that day. Within one report the days never overlap, so a replacement
cannot count a record on two days.

### 11. Tests that prove no double counting

The model crate publishes these cases with its release, as ADR-0022 section 3
does for the space rule, so the extension and every other client run them.
Property tests (`proptest`) cover the order of arrival.

1. A report applied twice gives the numbers of applying it once.
2. Two sources for one session, in both orders: the numbers are the winner's,
   and the day totals are the winner's, never the sum.
3. Three reports (OpenTelemetry, transcript, status line), in all six orders:
   the same numbers and the same history.
4. A better source arrives later: the numbers change, the older report is
   superseded, and the day total moves by the difference only.
5. A worse source arrives later: nothing visible changes; the context is
   merged.
6. A report with fewer total tokens than the stored one from the same source
   is ignored and flagged; a receiver restart does not lower its totals.
7. A status line alone adds nothing to any total.
8. A subagent transcript and its parent give the parent's totals once.
9. A partial report (it starts late or ends early) does not beat a complete
   lower one. With no complete report, the one with the most tokens wins and the
   session is flagged partial. A model that one source lacks does not make it
   partial.
10. A record near midnight dated to two different days by two sources is counted
    once, on the day of the winner's report.
11. Two sessions merged after a late link: in one space, one session, the
    person's attachment kept, the totals counted once, before and after the
    upload. Two reports from one source: the larger is kept, never summed. In
    two spaces, no merge and a flag.
12. Case I and case II of section 3, with the same inputs: equal totals, except
    that case II uses rank 2 numbers where case I used rank 1.
13. A totals-only report adds nothing to any total.
14. Billing: a day with a gap, a day where Sobr saw more than the bill, a day
    with no billing, and a day not yet refreshed. No session and no total
    changes in any of them.
15. The extension alone: an estimate that reaches the app twice counts once.
16. A correction and a rule: the person's attachment survives a change of
    winner and a re-run of the rules.

## Examples

**Three reports for one session, in any order.** Alice uses the app in her
personal space. On Monday, a Claude Code session S sends the status line
(cost $0.80, last call 2,100 tokens: context only), then the transcript at the
end (118,000 input and 31,000 output tokens: 149,000), then OpenTelemetry
through the app's receiver (120,500 and 31,200: 151,700). All three start and
end together, so each is complete.

1. The status line adds nothing. The transcript, rank 2, gives S 149,000
   tokens.
2. OpenTelemetry, rank 1, arrives and wins: S is 151,700. The transcript
   report is kept, superseded.
3. Alice's day total counts 151,700, not 300,700. Had OpenTelemetry arrived
   first, the transcript would have been kept as history and nothing would have
   changed. Her correction of S to the task "Fix the RSS feed" stays the same
   through all three steps.
4. Had the app been closed for the first hour, OpenTelemetry would start an
   hour after the transcript. It would be partial, so the transcript would stay
   the winner and the partial report would be kept as history.

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

**A late tie and a replay.** Erin finds, with Claude Code, that the status line
and the transcript use different IDs (case II). She works on Monday.

1. The transcript is imported first: session B is born with the reference "log
   B". She attaches B to the project "Checkout".
2. The status line later reports `session_id` A with the transcript path of B.
   Step 2 of section 3 finds B's session and records A as one more reference of
   it; nothing is merged, because A had no session of its own.
3. Had A already had a session in the same space, the older would survive,
   Erin's attachment would win, the two sets of reports would be joined, and the
   totals would count the session once.
4. Her browser extension sends an estimate for a conversation to the app while
   it is closed, so the estimate waits in the spool. When the app opens, the
   estimate arrives once from the spool and once live. Both carry the same
   conversation reference, so they are one report; the second replaces the first
   and nothing is counted twice.

## Consequences

**Positive:**

- One rule answers "is this the same use?" for every source, with a proof or no
  link at all.
- Totals cannot be doubled by a source, an order of arrival, a replay or a
  record near midnight.
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
- One machine merges only the sources it sees. The same transcript read on two
  machines of one personal space counts twice until the merge at first sync
  compares references (ADR-0022, issue 15).
- A past day's total can change when a better report arrives, and a record can
  move to the next day.
- The organization view has two figures to read.
- The precedence function runs on the device only, and the server stores what
  it receives, so a wrong choice on the device reaches the totals. The
  published cases are the guard.
- Option A of section 8 serves people alone in case I. In case II it needs the
  mode choice that the amendment following SOB-163 defines.
- SOB-166 is reframed, and the capture note (section 3, option 2) is updated
  by build issue 9, because OpenTelemetry enters through the app.

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
| 1 | Add source references, the report identity, completeness, the precedence function, the merge and the published cases to the model crate, with property tests | Model crate | 5 | ADR-0022 issue 1 |
| 2 | Turn the Claude Code transcript import into cumulative reports with subagent folding, and pin the resumed-session case | Import crate, desktop backend | 5 | #1; SOB-161; **SOB-26** |
| 3 | Make the status line and hooks report context and the live view only, with the two references of step 1 | Plugin (`claude-code` repo) | 3 | #1; SOB-164 |
| 4 | Receive OpenTelemetry in the desktop app (local endpoint, allow-list, increments turned into totals), check with a real run whether a subagent's requests carry the parent's ID, and ship the export settings | Desktop backend, plugin | 5 | #1; SOB-166 (reframed); **SOB-26** |
| 5 | Make the extension send one report per conversation and keep its conversation reference local (with ADR-0022 issue 9) | Browser extension, bridge | 3 | #1; ADR-0022 issue 9; **SOB-31**, **SOB-24**, **SOB-26** |
| 6 | Store the winning report of each session on the server with the replaced ones as history, and accept a session merge | Server | 5 | #1; ADR-0022 issues 6 and 7; **SOB-25** |
| 7 | Apply rules again when a session gains context, and merge two sessions with the person's attachment winning | Model crate, desktop backend | 3 | #1; ADR-0022 issue 5; **SOB-26** |
| 8 | Fetch billing for an organization (Z2 connection, hourly buckets cut to the space's day, provisional until refreshed) and show the billed figure and the gap | Server, integrations | 8 | ADR-0022 issue 6; **SOB-25** |
| 9 | Update the capture note with the result of SOB-163 and this ADR (OpenTelemetry through the app, section 3 option 2) | Docs | 1 | SOB-163 |

## Links

- [Routes, screen by screen](https://claude.ai/artifact/Cua7SK84VzMCwYFZRQfUHD):
  "Doublons de mesure"; screens T1 to T3
- ADR-0004, ADR-0015, ADR-0016, ADR-0019, ADR-0022 (Amendment 1 closes its
  references to this ADR)
- `docs/sources/CLAUDE-CODE-USAGE-CAPTURE.md` (SOB-145), SOB-161, SOB-163,
  SOB-166, SOB-195 (the two points), SOB-198 (local context), SOB-118 (the
  `both` mode double count), SOB-87 (amendment by PR)
- Official pages read on 2026-10-31: [Usage and Cost API](https://platform.claude.com/docs/en/manage-claude/usage-cost-api),
  [Usage and Cost Analytics API](https://platform.claude.com/docs/en/manage-claude/usage-cost-analytics-api),
  [Claude Code Analytics API](https://platform.claude.com/docs/en/manage-claude/claude-code-analytics-api)
