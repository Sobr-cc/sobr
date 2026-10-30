---
name: linear-issue-writing
description: Use when creating, rewriting or splitting Linear issues or project updates for Sobr.cc, so each task is clear, estimated, routed to the right role and model, and written in plain English a human can act on.
---

# Writing Linear issues for Sobr.cc

Every issue must be understandable by a person who has never seen the conversation that created it. Write in plain English: short sentences, common words, no jargon without a one-line explanation. If a newcomer cannot tell what "done" looks like after reading the issue once, rewrite it.

All agents working on Sobr.cc are Claude agents (Claude Code and its `.claude/agents/`, Claude Design). Linear's own AI features stay off: do not delegate to Linear Agent and do not use the `agent:linear` label.

## Before writing

1. Check the issue does not already exist (search the team SOB by key words).
2. Find the project it belongs to. If none fits, put it in Triage and say so.
3. Find the screen IDs it touches (E1, A6, Z2, V3, T1, B2...) in the routes document, and the components in the architecture document. Never invent a screen or a decision that is not in those documents. If something is undecided, write a Decision issue (Thibault must choose) or a Research issue (facts are missing).

## Title

- Start with a verb, describe the outcome, under 70 characters.
- Good: "Show live session energy in the VS Code status bar".
- Bad: "VS Code stuff", "Fix bug", "Status bar".

## Description template

Use exactly these headings, in this order. Leave a heading out only if it truly does not apply.

```
## Why
One or two sentences: the problem or the user need, and who has it.

## What
What changes, in plain words. Mention the screens (e.g. V1, V3) and the files or components if known.

## Acceptance criteria
- [ ] Observable result 1 (something a person can check)
- [ ] Observable result 2
- [ ] Works in French and English (if user-facing)

## Out of scope
What this issue must NOT change.

## How to verify
The exact command, test or manual steps that prove it works (e.g. `cargo clippy -- -D warnings`, `cargo test -p sobria-estimator`, a Playwright test name, a screenshot of screen V1).

## References
Links: routes document, architecture document, ADRs, design link, related issues.
```

## Decision issues

Use Type = Decision when only Thibault can choose. Assign the issue to Thibault, label it `human-only`, and write:

```
## Question
One sentence.

## Options
1. Option A: what it means, cost, risk.
2. Option B: ...

## Recommendation
Which option and why, in two sentences.

## Blocks
Issues that wait for this answer.
```

Thibault answers in a comment. The orchestrator reads the answer, records it, and closes the loop.

## Estimate (Fibonacci points)

Points measure size, including review time.

| Points | Meaning | Rough time for one person |
| --- | --- | --- |
| 1 | Trivial, one obvious change | under 1 hour |
| 2 | Small, clear path | half a day |
| 3 | Medium, a few files, known approach | about 1 day |
| 5 | Large, several parts or some unknowns | 2 to 3 days |
| 8 | Very large, maximum allowed | about a week |

- Never use more than 8. Split anything bigger into sub-issues, each with its own outcome.
- If you cannot estimate because something is unknown, create a Research issue (3 points or less) to remove the unknown first.
- Design issues are estimated the same way (screens produced and validated).

## Labels (set one of each group)

- Type: Feature, Bug, Chore, Design, Docs, Research, Decision
- Area: App, Browser extension, VS Code, Claude Code, MCP, Server, Site, Data
- Agent (who executes): agent:claude-code, agent:claude-design, human-only
- Role (which hat): role:orchestrator, role:po-individuals, role:po-organizations, role:po-research, role:designer, role:ux-writer, role:cto, role:rust, role:frontend, role:extensions, role:integrations, role:devops, role:qa, role:security, role:data, role:docs
- Model (which Claude model): model:haiku, model:sonnet, model:opus

Use `human-only` for anything touching accounts, payments, app stores, secrets or legal matters.

## Choosing the model

The model follows the size and risk of the task, not the role.

| Task | Model |
| --- | --- |
| 1 point, mechanical, no decision (labels, renames following a plan, changelog, weekly catalog collection, running an existing test suite) | model:haiku |
| 2 to 5 points, known path (most features, cleaning one area with /simplify, tests, front end, extensions, specs) | model:sonnet |
| 8 points, or high risk (ADRs, architecture, data model, cross-crate refactor, SSO and SCIM, privacy and security, estimator method, review of risky PRs) | model:opus |

Fable is never a default. Use it only if Opus failed twice on a hard problem and Thibault agreed.

Move up one level only after two blocked attempts: the agent writes why in a comment and the orchestrator changes the Model label.

## Priority

- Urgent: blocks users or the release now.
- High: needed for the current project milestone.
- Medium: planned for this project, not blocking.
- Low: nice to have.

## Definition of Ready (required before moving to Todo)

- Title follows the rules above.
- Why, What, Acceptance criteria and How to verify are filled.
- Estimate set, 8 or less.
- Labels set (Type, Area, Agent, Role, Model), project set, priority set.
- No open question left in the text. If there is one, the issue stays in Backlog and a Decision or Research issue is linked.

## Definition of Done (for code issues)

- `cargo clippy -- -D warnings`, tests, `tsc --noEmit` and lint pass.
- `/simplify` and `/code-review` run on the diff, findings handled.
- One real check: `/verify`, a Playwright test, or a curl call on the changed endpoint.
- CHANGELOG updated. PR description contains "Fixes SOB-123".

## Splitting a large issue

Split by user-visible result, not by technical layer. Each sub-issue must be shippable and testable alone.
- Good split: "Show sessions list (T1)", "Add 'to classify' inbox (T2)", "Turn a correction into a rule (T3)".
- Bad split: "Backend for sessions", "Frontend for sessions", "Tests for sessions".

## Comments written by agents

When an agent moves an issue to In Review or stops working on it, it adds one comment with three parts, signed with its role:

```
Done: what changed, in one to three sentences.
Verified: the command or steps run, and the result.
Left: anything not done, follow-up issue links, or questions for Thibault.
-- role:rust (model:sonnet)
```

Any extra work discovered along the way becomes a new issue in Triage, never a silent addition.

## Weekly project update

Every Friday, for each active project, post a project update with a health value (On track, At risk, Off track):

```
Progress: what was finished this week (issue links).
Risks: what could make the target date slip, and why.
Next: the two or three issues planned next.
```

Keep it under 120 words. State facts, not feelings.

## Example of a ready issue

Title: Show live session energy in the VS Code status bar

```
## Why
Developers want to see the cost of their current AI session without leaving the editor.

## What
Add a status bar item to the Sobr VS Code extension (screen V1) that shows the current session's energy (kWh, P5-P95 range) and cost. Clicking it opens the Sobr tab.

## Acceptance criteria
- [ ] The status bar shows energy first, then cost, for the active session.
- [ ] The value updates within 5 seconds of a new Claude Code turn.
- [ ] The item can be turned off in the extension settings.
- [ ] Text is available in French and English.

## Out of scope
The Sobr tab content (V1 list) and project views (V3).

## How to verify
Run the extension in the VS Code test host, start a Claude Code session, check the value changes; run `npm test` in the vscode package.

## References
Routes document: route 5 (V1). Architecture document: surfaces. Decision: live tracking is everywhere you code and can be turned off.
```

Estimate: 3. Labels: Feature, VS Code, agent:claude-code, role:extensions, model:sonnet. Priority: High.
