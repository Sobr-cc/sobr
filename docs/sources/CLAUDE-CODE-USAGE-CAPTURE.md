# Capturing Claude Code usage without storing prompts

> **Status**: proposal for the "Claude Code and MCP" project (SOB-145). **Checked on 2026-10-31** against the official Claude Code documentation (`code.claude.com/docs`).
> **Related**: ADR-0019 decision 4 (where the import is built), ADR-0022 (the session model and the upload contract), ADR-0015 and CLAUDE.md section 7 (privacy), SOB-146 (session identity and duplicates, not written yet), SOB-67 (the old `sobria-import` crate is removed).
> **Amendment 1 (2026-10-31, SOB-203, decision [SOB-198](https://linear.app/sobrcc/issue/SOB-198), option A):** the device keeps the folder, repo and branch of a session for 24 months, on the device only, so that rules from corrections can run on past sessions. The server never receives them. The previous wording is kept here for the record: "No prompt text, response text, tool input or output, file path or command line is stored, on the device or on the server", and "Never stored: ... file paths, folder, repo and branch names, command lines, ... The project path may be read to choose a project, but it is not stored or uploaded." Both sentences are replaced by the text below ("What we store per session", the table of what Sobr stores, and "Kept on the device, never uploaded").
> Every fact about Claude Code links to the official page it comes from. "No official source" means we looked and found none; it is never filled in by guessing. The documentation is read through a fetch tool that summarises pages, so a link is where to confirm a fact, not a proof by itself (see "How this was checked").

## Summary

Sobr can measure Claude Code three ways. None of them needs to keep prompt text, but **only OpenTelemetry keeps prompt text out by default**; the other two hand the reader prompt text or file content and must be cut down on our side.

| Source | Gives | History? | Prompt text reaches us? | Recommendation |
|---|---|---|---|---|
| Local transcripts | whatever the lines hold, in an **unofficial, unstable** format (no official schema) | yes, 30 days by default | **yes**, in the same files | Import history with an allow-list reader that fails closed |
| Plugin: status line and hooks | live model, a cost estimate (a running total), the **last call's** token counts (not totals), session ID, and the path of the transcript | no | only in a few hook events, which we do not subscribe to | Status line for the live display and the cost; a `SessionEnd` hook hands the transcript to the local reader for the totals; no prompt-bearing hook |
| OpenTelemetry | metrics (model, token totals, cost, session ID) and events | no | **no prompt text** unless `OTEL_LOG_USER_PROMPTS` is set; but it does send the user's email and account IDs, which the receiver must drop | Optional, for organizations; metrics only, through a collector run by the organization |

**Token totals per interactive session come only from the transcript (unofficial) or from OpenTelemetry (official, opt-in).** The status line gives a cost total but no token total. Thibault decided on October 31 (SOB-162, option 1) that Sobr reads the transcripts, with the guard rails of section 1, so a solo user gets token totals from them.

What we store per session is below. **No prompt text, response text, tool input or output, command line, or path of a file that a session reads or writes is stored, on the device or on the server.** The folder, repo and branch of a session are kept on the device only (see "Kept on the device, never uploaded").

## What Sobr stores per session

This restates the upload contract of ADR-0022 for Claude Code sources; it adds no field to it. The last row stays on the device.

| Stored | Where it comes from |
|---|---|
| Session ID (Sobr's own random ID), source kind (`claude-code`), provider host | Sobr |
| Model ID | status line `model.id`, OpenTelemetry `model`, or the transcript |
| Day the session started | the clock time is cut to the day before anything leaves the device |
| Input, output and cache token totals | OpenTelemetry metrics, or the transcript read at the end of the session |
| Cost estimate | status line, OpenTelemetry, or the transcript (an estimate at list price, not a bill) |
| Energy, CO₂ and water with P5, P50, P95, method and version | computed by Sobr from the tokens and the model |
| Folder, repo and branch of the session (device only, never uploaded) | the folder is the `cwd` value (hooks, status line, or the transcript's `cwd` key; never the transcript's project folder name, which loses characters); the branch is the transcript's `gitBranch` key; the repo is read on the device, best effort, from the `origin` remote of `cwd` (none if there is no `origin`), and only a normalized `host/owner/repo` is kept: credentials, query strings and local paths are dropped, and the value read at import time can differ from the one at the time of the session. If `cwd` or the branch changes during a session, all the values are kept: the context of a session is the union of its reports (ADR-0025 section 7) |

**Kept on the device, never uploaded**: the working folder, the repo and the branch of a session, for 24 months (SOB-198, stated in ADR-0025 section 7). Like clock times (ADR-0022 section 10), they never leave the device, and rules from corrections use them, also on past sessions (ADR-0022 section 6). The server never receives them: the upload contract carries only the project or task ID, never these values, so no sync of a personal space carries them either, and no log line holds them. A session seen only through OpenTelemetry has no folder, repo or branch, and a rule that needs them does not match it.

**Never stored**: prompt or response text and anything derived from it (a title, a summary, a first message), tool inputs and outputs, any other path (the transcript path, the paths of the files a session reads or writes, the paths in the dictionaries with free keys), command lines, conversation URLs, and clock times on the server (ADR-0022). Of all the paths, only the session's working folder is kept, on the device.

Each source also hands over fields that must be **dropped at the boundary**, before they reach any variable that outlives the call:

- the hook fields `prompt`, `last_assistant_message`, `delta`, `tool_input`, `tool_response`;
- the status line `session_name` (it can be a title generated from the first prompt), and the Agent SDK `firstPrompt` and `summary`;
- every OpenTelemetry attribute outside the allow-list, such as `user.email`, `user.account_uuid`, `user.account_id`, `organization.id`, `user.id`, `prompt.id` and the file path attributes (the folder is the `cwd` value, the branch the `gitBranch` key and the repo the `origin` remote, all read on the device; see "What Sobr stores per session");
- everything in a transcript line other than the few keys in the allow-list.

## 1. Local logs

**What Claude Code writes**

- Transcripts are JSONL files at `~/.claude/projects/<project>/<session-id>.jsonl`. `<project>` is the working directory with every non-alphanumeric character replaced by `-`; a name over 200 characters is cut and a hash of the full path is appended. [Manage sessions](https://code.claude.com/docs/en/sessions#where-transcripts-are-stored).
- `CLAUDE_CONFIG_DIR` moves the storage; `CLAUDE_CODE_PROJECT_DIR_NAME` fixes the project folder name (only with `CLAUDE_CONFIG_DIR`). Same page. On Windows `~/.claude` is `%USERPROFILE%\.claude` ([The .claude directory](https://code.claude.com/docs/en/claude-directory)).
- Subagent transcripts are in `projects/<project>/<session>/subagents/`, same format ([The .claude directory](https://code.claude.com/docs/en/claude-directory)).
- `~/.claude/stats-cache.json` holds the aggregated token and cost counts shown by `/usage`; `~/.claude/history.jsonl` holds typed prompts with their project path. Same page. We do not read `history.jsonl`.
- Retention: 30 days by default, set by `cleanupPeriodDays` ([Manage sessions](https://code.claude.com/docs/en/sessions#where-transcripts-are-stored)). `CLAUDE_CODE_SKIP_PROMPT_HISTORY=1` and `--no-session-persistence` stop the writes: then there is nothing to read.

**Fields that map to a session**

- Official: the session ID is the file name and the project is the folder name. A line is "a message, a tool use or result, a record of a request Claude Code sent the model, or a metadata entry".
- **Model, tokens, time, working directory and version inside a line: no official source.** The documentation lists no schema and says: "The entry format is internal to Claude Code and changes between versions, so scripts that parse these files directly can break on any release." ([Manage sessions](https://code.claude.com/docs/en/sessions#where-transcripts-are-stored)). The key names must be found in sample files, not guessed here (follow-up issue).
- The documentation says transcripts are plaintext and hold file contents, command output and pasted text ([The .claude directory](https://code.claude.com/docs/en/claude-directory)), and, for the SDK, "your prompt, every tool call the agent made, every tool result" ([Agent SDK sessions](https://code.claude.com/docs/en/agent-sdk/sessions)). Reading a transcript means skipping that content.

**Options**

1. *Parse the transcripts directly.* Only source of history; unofficial format; holds prompt text; breaks on any release; 30 days of history by default.
2. *Official script interfaces only* (`claude -p --output-format json`, the Agent SDK result message, hooks and status line). Stable and documented, but they only see runs that we start or sessions that are running with the plugin. No history.
3. *Allow-list reader (recommended for history).* Option 1 with guard rails: read only the few keys needed (model, token counts, a time, the session ID, and the working folder, repo and branch that the device keeps); never copy a message body into a structure that lives past the line; detect the Claude Code version; **fail closed** (an unknown shape yields "no data", never a guess); pin the shapes with golden files made from real samples; count a message once if it appears twice (the SDK warns that assistant messages sharing an ID must be deduplicated in its stream: [cost tracking](https://code.claude.com/docs/en/agent-sdk/cost-tracking), not re-checked; whether transcripts repeat messages: no official source).

**Recommendation**: option 3 for the history import and for the session totals read at the end of a session (the `SessionEnd` hook gives the transcript path, section 2), labelled as a best-effort unofficial source (`claude-code-log`), plus option 2 where a run is ours. Thibault accepted the risk that the format changes on October 31 (SOB-162, option 1), on these guard rails: an allow-list of keys, never keep message text, check the Claude Code version, fail closed on an unknown shape, golden files.

## 2. Plugin: hooks and status line

**Status line** ([Customize your status line](https://code.claude.com/docs/en/statusline))

- Configured by a `statusLine` object (`{"type":"command","command":"..."}`) in the user or project settings. The command reads JSON on stdin.
- Fields that matter: `model.id`, `cost.total_cost_usd`, `context_window.current_usage` (input, output, cache creation and cache read tokens), `session_id`, `prompt_id`, `transcript_path`, `cwd`, `version`. **The token fields are not totals**: `context_window.total_input_tokens` and `total_output_tokens` are "Token counts currently in the context window, from the most recent API response", and `current_usage` is the last call.
- No field carries prompt text, but `session_name` may be a title generated from the first prompt: drop it.
- `cost.total_cost_usd` is "computed client-side at list price" and "May differ from your actual bill"; it resets to $0 when `/clear` starts a new session, and a session after `/clear` is a new session ([Manage sessions](https://code.claude.com/docs/en/sessions)), so the reset does not split one session in two. `current_usage` is `null` before the first call and after `/compact`.
- It runs locally and "does not consume API tokens". It runs at session start, after each assistant message, after `/compact` and on a few other events, debounced at 300 ms.
- Turn it off: `/statusline delete`, or delete the `statusLine` field.
- **A plugin cannot ship a status line**: the plugin manifest `settings` key: "Only `agent` and `subagentStatusLine` take effect; other keys are dropped at load" ([Plugins reference](https://code.claude.com/docs/en/plugins-reference), read on the raw page). The setting is added by the one-time consent step decided in SOB-164 (see "Decided").

**Hooks** ([Hooks reference](https://code.claude.com/docs/en/hooks))

- 33 events at the time of reading. Common input fields: `session_id`, `prompt_id`, `transcript_path`, `cwd`, `hook_event_name`. None of these carries text.
- Events that carry text: `UserPromptSubmit` (`prompt`), `Stop` and `SubagentStop` (`last_assistant_message`), `MessageDisplay` (`delta`), and the tool events (`tool_input`, `tool_response`).
- Events that carry no prompt or tool data: `SessionStart` (`source`, `model`), `SessionEnd` (`reason`), `PreModelSwitch` and `PostModelSwitch` (`from_model`, `to_model`). Only `SessionStart` receives a field named `model`, and not always; the model-switch events carry `from_model` and `to_model` instead, so a plugin that wants the model after a switch listens to `PostModelSwitch` (hooks page, read on the raw page).
- **No hook carries per-turn usage or cost** (`Stop`, `StopFailure`, `SessionEnd` have none). So hooks cannot measure tokens or cost; the status line gives a cost total but only the last call's token counts.
- `disableAllHooks: true` turns every hook off; "There is no way to disable an individual hook while keeping it in the configuration". `"async": true` runs a command hook without blocking. Command hooks run "with your full user permissions", and an HTTP hook posts the same JSON to a URL.

**Plugin** ([Plugins reference](https://code.claude.com/docs/en/plugins-reference), [Discover plugins](https://code.claude.com/docs/en/discover-plugins), [Plugin marketplaces](https://code.claude.com/docs/en/plugin-marketplaces))

- A plugin has an optional `.claude-plugin/plugin.json` and components at its root (`hooks/hooks.json`, `skills/`, `commands/`, `agents/`, `.mcp.json`). It is distributed through a marketplace (a repo with `.claude-plugin/marketplace.json`) and installed with `/plugin install name@marketplace`.
- It is turned off with `claude plugin disable name@marketplace` or `/plugin disable`; its hooks then stop.
- No official statement on the privacy of hook or status line input: no official source.

**Options**

1. *Status line only.* Live model, cost total and the last call's token counts, no prompt text, no hook. No token totals. The user must add the setting.
2. *Hooks only.* `SessionStart` gives the model, but no usage: they only help to find the transcript (`transcript_path`), which brings back the problems of section 1.
3. *Both.* Status line for the live display and the cost; `SessionStart` and `SessionEnd` hooks (command, async) to tell the app that a session began or ended. `SessionEnd` gives `transcript_path`, so the app reads that one transcript with the allow-list reader of section 1 to get the totals.

**Recommendation**: option 3 with **no hook that carries text**: subscribe only to `SessionStart`, `SessionEnd` and the model-switch events. The live display and the cost come from the status line; the token totals come from the transcript at the end of the session (or from OpenTelemetry). The status line is enabled by the consent step Thibault chose (SOB-164, October 31, option 1): a plugin command, run once and explained on screen, writes the `statusLine` setting. The live display can always be turned off by the user (`/statusline delete`, or disabling the plugin).

## 3. OpenTelemetry (optional, for organizations)

Source: [Monitoring usage](https://code.claude.com/docs/en/monitoring-usage) unless said otherwise.

**Enable it**

- `CLAUDE_CODE_ENABLE_TELEMETRY=1` is required. Exporters: `OTEL_METRICS_EXPORTER` and `OTEL_LOGS_EXPORTER` (`otlp`, `console`, `none`; metrics also `prometheus`). Protocol `OTEL_EXPORTER_OTLP_PROTOCOL` (`grpc`, `http/json`, `http/protobuf`; no default) and `OTEL_EXPORTER_OTLP_ENDPOINT`. Intervals: metrics 60 s, logs 5 s.
- Organizations set these in the managed settings file `env` block: macOS `/Library/Application Support/ClaudeCode/managed-settings.json`, Linux and WSL `/etc/claude-code/managed-settings.json`, Windows `C:\Program Files\ClaudeCode\managed-settings.json` ([Managed settings](https://code.claude.com/docs/en/managed-settings), reported by the research run). Managed settings win over user, project and `--settings` values, and a repository `settings.json` cannot configure the exporters.
- The page says: "OpenTelemetry export to your backend is opt-in and requires explicit configuration."

**Signals**

- Metrics include `claude_code.token.usage` (tokens; `type` is input, output, cacheRead or cacheCreation) and `claude_code.cost.usage` (USD), both with `model`; also `claude_code.session.count` and `claude_code.active_time.total`.
- Events include `claude_code.api_request` (`model`, `cost_usd`, `duration_ms`, input, output and cache token counts) and `claude_code.user_prompt` (`prompt_length`, not the text).
- Attributes on every signal include `session.id`, `organization.id`, `user.id` (a random anonymous ID), `user.account_uuid`, `user.account_id`, `user.email` (and `ccr.session.id` on cloud sessions). `OTEL_METRICS_INCLUDE_SESSION_ID` and `OTEL_METRICS_INCLUDE_ACCOUNT_UUID` default to true. No client setting removes `user.email` or `organization.id`: only the receiver can drop them.

**Privacy of the content**

- `OTEL_LOG_USER_PROMPTS` is "disabled" by default; the prompt is redacted and only its length is sent. Assistant text, tool details, raw tool output and raw API bodies each have their own variable, also off by default (`OTEL_LOG_ASSISTANT_RESPONSES`, `OTEL_LOG_TOOL_DETAILS`, `OTEL_LOG_TOOL_CONTENT`, `OTEL_LOG_RAW_API_BODIES`).
- Tool names and some path attributes are still exported without these flags (reported by the research run, not re-checked). Tracing is a separate, beta path that can carry content.

**Privacy limits of this path**

- Claude Code sends the raw signals, with full timestamps, `session.id`, `user.email`, `organization.id`, `user.account_uuid` and `user.account_id`, to whatever endpoint is configured. They reach the collector before any filter runs, so the collector must run in the organization's own infrastructure, never on a third party's.
- The allow-list is therefore a hard requirement, enforced twice: by the collector, and by the Sobr server contract, which refuses any field outside the ADR-0022 upload contract. A collector that forwards more than the allow-list breaks that contract.
- ADR-0022's rule that no clock time leaves a device binds the Sobr clients. It does not constrain what Claude Code sends to a collector; the collector is what brings the data back inside the rule.

**Receiving**

- A receiver speaks OTLP over gRPC or HTTP (protobuf or JSON). The page shows a collector on ports 4317 (gRPC) and 4318 (HTTP) and paths ending `/v1/metrics`, `/v1/logs`, `/v1/traces`. Whether a base endpoint gets these paths added automatically: no official source. Authentication headers can be static or come from an `otelHeadersHelper`. The page gives a collector `attributes` processor that deletes `prompt` and `prompt_text`.
- Sampling, retries and delivery when the receiver is down: no official source.

**Options**

1. *The Sobr server is an OTLP receiver.* One less piece to run, but the server must handle OTLP and strip identities itself.
2. *An OpenTelemetry Collector in front* (the pattern the documentation shows): it deletes the identity and path attributes and cuts timestamps to the day, then forwards to the Sobr API.
3. *Not in v1.*

**Recommendation**: option 2 first, option 1 later if the collector is a burden. Send **metrics only** (`OTEL_LOGS_EXPORTER=none`): `claude_code.token.usage` and `claude_code.cost.usage` already give model, tokens, cost and session ID, with no per-prompt event at all. Accept only those two names, keep `model`, `type`, `session.id` and the value, drop every other attribute, and cut the time to the day, as ADR-0022 requires. It stays optional: an administrator sets it in the managed settings.

## 4. Double counting

The rules belong to the session duplicates ADR (**SOB-146, not written yet**, in Backlog; ADR-0022 blocks it and leaves it open). This note gives it the facts:

- The session ID appears as the transcript file name, as `session_id` in hooks and in the status line, and as `session.id` in OpenTelemetry. The documentation states that the status line `prompt_id` "Matches the `prompt.id` attribute on OpenTelemetry events"; **it does not state that `session_id` and `session.id` are the same value** (no official source). A test on a real session must check it before any rule relies on it (SOB-163).
- Inside one source: assistant messages that share an ID must be counted once (SDK cost tracking). Subagents write their own transcripts; the SDK `usage` field leaves subagents out while `total_cost_usd` includes them ([cost tracking](https://code.claude.com/docs/en/agent-sdk/cost-tracking), not re-checked). The importer must not add a subagent transcript and its parent's totals.
- Across sources: one measurement per session and source (ADR-0022), then SOB-146 picks which source wins. Suggested input for it: prefer OpenTelemetry (official, token totals and cost), then the transcript (unofficial, token totals), then the status line, which gives the cost total and the last call only and so cannot stand alone for tokens. The status line stays the live display.
- The cost figures are client-side estimates at list price, not billing ([cost tracking](https://code.claude.com/docs/en/agent-sdk/cost-tracking), [status line](https://code.claude.com/docs/en/statusline)); they must not be matched to a bill.

## 5. Where the import is built

ADR-0019 decision 4 (option 4a) puts the **log import in a crate of `sobr`**: it runs in the app and writes to its session store. The plugin goes to the `claude-code` repo, and the MCP server goes to `claude-code` if it is a separate process talking to the HTTP API, or to `sobr` if it is a module of the server. This note confirms it:

- **Log import**: a crate of `sobr` (named with the `sobr-` prefix of ADR-0020), on the device only. It reads the allow-listed keys and writes sessions that follow the ADR-0022 contract. The removed `sobria-import` skeleton (SOB-67) gives nothing to reuse.
- **Status line script and hooks**: in the `claude-code` plugin. The script prints nothing useful to the user by itself; it hands the numbers to the local app. How it reaches the app (native messaging bridge of ADR-0013, a local socket, a file) is part of the plugin design.
- **OpenTelemetry**: a collector configuration plus a small receiver module in the organization server in `sobr`, not in the app.

## Open questions

Each one is a Linear issue.

1. **Research, SOB-161**: find the transcript keys Sobr needs (model, token counts, a time, the session ID, `cwd` and `gitBranch`) from real sample files, and pin them with golden tests. Done in the transcript keys note (SOB-161); the repo has no key and comes from the `origin` remote of `cwd`.
2. **Research, SOB-163**: check on a real session whether the transcript session ID, `session_id` (hooks, status line) and `session.id` (OpenTelemetry) are one value; input for SOB-146.
3. **Research, SOB-166**: build a collector configuration for the metrics-only path and check, with a real Claude Code run, the paths, the headers, the temporality and what happens when the receiver is down.

## Decided

Thibault answered both Decision issues of this note on October 31 (option 1 each), recorded in their comments.

- **SOB-162**: Sobr reads the local transcripts, with guard rails: an allow-list of keys, never keep message text, check the Claude Code version, fail closed on an unknown shape, golden files. SOB-161 (the transcript keys) goes ahead after this note is merged, and the import crate in `sobr` follows from it.
- **SOB-164**: one consent step. A plugin command, run once and explained on screen, writes the `statusLine` setting in the user's settings. It is reversible (`/statusline delete`) and shown in the Sobr settings. The plugin design uses this; how the script reaches the app is part of that design.

## How this was checked

- Three read-only research runs opened the official pages (hooks and status line, OpenTelemetry, local logs). I re-read these key points on the pages myself: the transcript location, retention and the "format is internal" sentence ([Manage sessions](https://code.claude.com/docs/en/sessions)); the status line fields, including that the token counts are from the most recent API response, and its removal ([status line](https://code.claude.com/docs/en/statusline)); the `OTEL_LOG_USER_PROMPTS` default and the metric and event names; the OpenTelemetry "opt-in" sentence ([Monitoring usage](https://code.claude.com/docs/en/monitoring-usage)); on the raw pages, the plugin manifest `settings` limit ([Plugins reference](https://code.claude.com/docs/en/plugins-reference)) and which hook events carry a `model` field ([Hooks](https://code.claude.com/docs/en/hooks)). A first draft said the status line gave token totals; the page shows it does not, and the note was corrected.
- Marked "reported by the research run, not re-checked", or not marked but outside the points above: the managed settings file paths and precedence; the SDK cost tracking rules; which hook events carry which fields (the run read the raw hooks page for these); the status line refresh timing; the facts taken from "The .claude directory" page (plaintext content, `history.jsonl`, `stats-cache.json`, subagent transcripts, the Windows path); the export intervals, the other metric names, the attribute defaults, the content flags other than `OTEL_LOG_USER_PROMPTS`, and the tracing remark.
- The pages are long and change often. Confirm a fact on its page before building on it, and read the version of Claude Code you target.

## Sources

[Manage sessions](https://code.claude.com/docs/en/sessions) · [The .claude directory](https://code.claude.com/docs/en/claude-directory) · [Hooks reference](https://code.claude.com/docs/en/hooks) · [Customize your status line](https://code.claude.com/docs/en/statusline) · [Plugins reference](https://code.claude.com/docs/en/plugins-reference) · [Discover plugins](https://code.claude.com/docs/en/discover-plugins) · [Plugin marketplaces](https://code.claude.com/docs/en/plugin-marketplaces) · [Monitoring usage](https://code.claude.com/docs/en/monitoring-usage) · [Managed settings](https://code.claude.com/docs/en/managed-settings) · [Data usage](https://code.claude.com/docs/en/data-usage) · [Agent SDK sessions](https://code.claude.com/docs/en/agent-sdk/sessions) · [Agent SDK cost tracking](https://code.claude.com/docs/en/agent-sdk/cost-tracking) · [Costs](https://code.claude.com/docs/en/costs) · [Non-interactive mode](https://code.claude.com/docs/en/headless)
