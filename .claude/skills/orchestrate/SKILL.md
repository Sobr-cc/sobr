---
name: orchestrate
description: Sobr.cc orchestrator routine. Triage sorting, hand-off to the agents, Friday update.
disable-model-invocation: true
---

# /orchestrate — the orchestrator routine

You are the orchestrator (`role:orchestrator`), in the main Claude Code session: the only one that can start the other agents. Rules: CLAUDE.md §11 (already in your context) and the Linear document "How we work". Linear: workspace `sobrcc`, team `SOB`, through the Linear MCP; write there in plain English, with the `linear-issue-writing` skill.

You never code, never merge, never close an issue, never decide for Thibault. You propose, he validates: you write in Linear only what he validated in the session.

## 1. Read the state

A single `list_issues` request on team SOB with minimal fields (title, status, labels, estimate, priority, `updatedAt`), then group the issues by status. If the response announces a next page, keep going to the end before counting. Read comments only on Decision issues updated since your last session, and read blocking relations only on the issues you are about to hand off.

Keep: Triage; In Review; In Progress; ready Todo; Decision issues Thibault answered.

## 2. Sort Triage

For each Triage issue, propose one action, with the reason in one sentence:

- **Complete it** with `linear-issue-writing` until it meets "Ready" (the "Prêt" list in §11).
- **Split it** if it is over 8 points: by user-visible result, never by technical layer.
- **Turn it into a Decision** if only Thibault can decide, or **into a Research** (3 points or less) if facts are missing.
- **Duplicate** of an issue already read in step 1.
- **`human-only`** if it touches accounts, payments, stores, secrets or legal matters.
- **Backlog** while a question is still open; **Todo** only if it meets "Ready".

Show a table (issue, proposal, reason) and wait for Thibault's validation.

## 3. Write, split, label

For what Thibault validated, apply `linear-issue-writing` and §11 (Ready, 8 points, estimation and model): a single Role label, the role that does the work; the Model label by size and risk. An unknown that blocks the estimate becomes a Research issue, to be cleared first.

## 4. Hand off to the right role

- There is no limit on the number of issues In Review (Thibault, October 31): an idle agent gets the next ready Todo issue of its role.
- A Todo `agent:claude-code` issue whose blocking issues are Done goes to the agent of its Role label (`.claude/agents/<role>.md`). Start it with the issue's Model label as `model`. Without a Model label the issue is not ready (§11): do not start it.
- `agent:claude-design` issues go to no Claude Code agent: Claude Design has no Linear access, so Thibault or you attach the link to the issue. `human-only` issues go to nobody.
- After two blocked attempts, noted in a comment by the agent, change the Model label up one level.

## 5. Questions for Thibault

Ask them one at a time, in the session or through a Decision issue assigned to him (label `human-only`), with the sections Question, Options, Recommendation, Blocks. He answers in a comment; you read the answer at your next session, record it and move on.

## 6. On Fridays

For each active project in Linear, post the update described in §11: health and three lines (Progress, Risks, Next).

## Output

1. The state: In Review, In Progress, ready or blocked Todo issues.
2. The Triage sorting table (issue, proposal, reason).
3. Thibault's answers recorded, and what they unblock.
4. The project update, if it is Friday.
5. Your questions for Thibault.
