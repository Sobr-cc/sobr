# Capturing Claude Code usage without storing prompts

> **Status**: proposal for the "Claude Code and MCP" project (SOB-145). **Checked on 2026-10-31** against the official Claude Code documentation (`code.claude.com/docs`).
> **Related**: ADR-0019 decision 4 (where the import is built), ADR-0022 (the session model and the upload contract), ADR-0015 and CLAUDE.md section 7 (privacy), SOB-146 (session identity and duplicates, not written yet), SOB-67 (the old `sobria-import` crate is removed).
> Every fact about Claude Code links to the official page it comes from. "No official source" means we looked and found none; it is never filled in by guessing. The documentation is read through a fetch tool that summarises pages, so a link is where to confirm a fact, not a proof by itself (see "How this was checked").

## Summary

Sobr can measure Claude Code three ways. None of them needs to keep prompt text, but **only OpenTelemetry keeps prompt text out by default**; the other two hand the reader prompt text or file content and must be cut down on our side.

| Source | Gives | History? | Prompt text reaches us? | Recommendation |
|---|---|---|---|---|
| Local transcripts | whatever the lines hold, in an **unofficial, unstable** format (no official schema) | yes, 30 days by default | **yes**, in the same files | Import history with an allow-list reader that fails closed |
| Plugin: status line and hooks | model, cost, tokens, session ID, live | no | only in a few hook events, which we do not subscribe to | Live capture through the status line; no prompt-bearing hook |
| OpenTelemetry | metrics (model, tokens, cost, session ID) and events | no | **no**, unless `OTEL_LOG_USER_PROMPTS` is set | Optional, for organizations; metrics only |

What we store per session is below. **No prompt text, response text, tool input or output, file path or command line is stored, on the device or on the server.**

## What Sobr stores per session

This restates the upload contract of ADR-0022 for Claude Code sources; it adds no field.

| Stored | Where it comes from |
|---|---|
| Session ID (Sobr's own random ID), source kind (`claude-code`), provider host | Sobr |
| Model ID | status line `model.id`, OpenTelemetry `model`, or the transcript |
| Day the session started | the clock time is cut to the day before anything leaves the device |
| Input, output and cache tokens, cost estimate | status line, OpenTelemetry, or the transcript |
| Energy, CO₂ and water with P5, P50, P95, method and version | computed by Sobr from the tokens and the model |

**Never stored**: prompt or response text and anything derived from it (a title, a summary, a first message), tool inputs and outputs, file paths, folder, repo and branch names, command lines, conversation URLs, clock times on the server (ADR-0022). The project path may be read to choose a project, but it is not stored or uploaded.

Each source also hands over fields that must be **dropped at the boundary**, before they reach any variable that outlives the call:

- the hook fields `prompt`, `last_assistant_message`, `delta`, `tool_input`, `tool_response`;
- the status line `session_name` (it can be a title generated from the first prompt), and the Agent SDK `firstPrompt` and `summary`;
- the OpenTelemetry `user.email`, `user.account_uuid`, `prompt.id` and file path attributes;
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
3. *Allow-list reader (recommended for history).* Option 1 with guard rails: read only the few keys needed (model, token counts, a time, the session ID); never copy a message body into a structure that lives past the line; detect the Claude Code version; **fail closed** (an unknown shape yields "no data", never a guess); pin the shapes with golden files made from real samples; count a message once if it appears twice (the SDK warns that assistant messages sharing an ID must be deduplicated in its stream: [cost tracking](https://code.claude.com/docs/en/agent-sdk/cost-tracking), not re-checked; whether transcripts repeat messages: no official source).

**Recommendation**: option 3 for the history import, labelled as a best-effort unofficial source (`claude-code-log`), plus option 2 where a run is ours. The risk that the format changes is accepted only if Thibault agrees (open question 2).

## 2. Plugin: hooks and status line

**Status line** ([Customize your status line](https://code.claude.com/docs/en/statusline))

- Configured by a `statusLine` object (`{"type":"command","command":"..."}`) in the user or project settings. The command reads JSON on stdin.
- Fields that matter: `model.id`, `cost.total_cost_usd`, `context_window.current_usage` (input, output, cache creation and cache read tokens), `session_id`, `prompt_id`, `transcript_path`, `cwd`, `version`.
- No field carries prompt text, but `session_name` may be a title generated from the first prompt: drop it.
- `cost.total_cost_usd` is "computed client-side at list price" and "May differ from your actual bill"; it resets to $0 when `/clear` starts a new session. `current_usage` is the last API call, not a running total, and is `null` before the first call and after `/compact`.
- It runs locally and "does not consume API tokens". It runs at session start, after each assistant message, after `/compact` and on a few other events, debounced at 300 ms.
- Turn it off: `/statusline delete`, or delete the `statusLine` field.
- **A plugin cannot ship a status line**: the plugin manifest `settings` key honours only `agent` and `subagentStatusLine` ([Plugins reference](https://code.claude.com/docs/en/plugins-reference), reported by the research run, not re-checked). The user must add the `statusLine` setting.

**Hooks** ([Hooks reference](https://code.claude.com/docs/en/hooks))

- 33 events at the time of reading. Common input fields: `session_id`, `prompt_id`, `transcript_path`, `cwd`, `hook_event_name`. None of these carries text.
- Events that carry text: `UserPromptSubmit` (`prompt`), `Stop` and `SubagentStop` (`last_assistant_message`), `MessageDisplay` (`delta`), and the tool events (`tool_input`, `tool_response`).
- Events that carry no prompt or tool data: `SessionStart` (`source`, `model`), `SessionEnd` (`reason`), `PreModelSwitch` and `PostModelSwitch` (`from_model`, `to_model`). Only `SessionStart` can receive a model field.
- **No hook carries per-turn usage or cost** (`Stop`, `StopFailure`, `SessionEnd` have none). So hooks cannot measure tokens; the status line can.
- `disableAllHooks: true` turns every hook off; "There is no way to disable an individual hook while keeping it in the configuration". `"async": true` runs a command hook without blocking. Command hooks run "with your full user permissions", and an HTTP hook posts the same JSON to a URL.

**Plugin** ([Plugins reference](https://code.claude.com/docs/en/plugins-reference), [Discover plugins](https://code.claude.com/docs/en/discover-plugins), [Plugin marketplaces](https://code.claude.com/docs/en/plugin-marketplaces))

- A plugin has an optional `.claude-plugin/plugin.json` and components at its root (`hooks/hooks.json`, `skills/`, `commands/`, `agents/`, `.mcp.json`). It is distributed through a marketplace (a repo with `.claude-plugin/marketplace.json`) and installed with `/plugin install name@marketplace`.
- It is turned off with `claude plugin disable name@marketplace` or `/plugin disable`; its hooks then stop.
- No official statement on the privacy of hook or status line input: no official source.

**Options**

1. *Status line only.* Live model, cost and tokens, no prompt text, no hook. The user must add the setting.
2. *Hooks only.* `SessionStart` gives the model, but no usage: they only help to find the transcript (`transcript_path`), which brings back the problems of section 1.
3. *Both.* Status line for numbers, `SessionStart` or `SessionEnd` hooks (command, async) to tell the app that a session began or ended.

**Recommendation**: option 3 with **no hook that carries text**: subscribe only to `SessionStart`, `SessionEnd` and the model-switch events. The numbers come from the status line. The live display can always be turned off by the user (`/statusline delete`, or disabling the plugin).

## 3. OpenTelemetry (optional, for organizations)

Source: [Monitoring usage](https://code.claude.com/docs/en/monitoring-usage) unless said otherwise.

**Enable it**

- `CLAUDE_CODE_ENABLE_TELEMETRY=1` is required. Exporters: `OTEL_METRICS_EXPORTER` and `OTEL_LOGS_EXPORTER` (`otlp`, `console`, `none`; metrics also `prometheus`). Protocol `OTEL_EXPORTER_OTLP_PROTOCOL` (`grpc`, `http/json`, `http/protobuf`; no default) and `OTEL_EXPORTER_OTLP_ENDPOINT`. Intervals: metrics 60 s, logs 5 s.
- Organizations set these in the managed settings file `env` block: macOS `/Library/Application Support/ClaudeCode/managed-settings.json`, Linux and WSL `/etc/claude-code/managed-settings.json`, Windows `C:\Program Files\ClaudeCode\managed-settings.json` ([Managed settings](https://code.claude.com/docs/en/managed-settings), reported by the research run). Managed settings win over user, project and `--settings` values, and a repository `settings.json` cannot configure the exporters.
- The page says: "OpenTelemetry export to your backend is opt-in and requires explicit configuration."

**Signals**

- Metrics include `claude_code.token.usage` (tokens; `type` is input, output, cacheRead or cacheCreation) and `claude_code.cost.usage` (USD), both with `model`; also `claude_code.session.count` and `claude_code.active_time.total`.
- Events include `claude_code.api_request` (`model`, `cost_usd`, `duration_ms`, input, output and cache token counts) and `claude_code.user_prompt` (`prompt_length`, not the text).
- Attributes on every signal include `session.id`, `organization.id`, `user.id` (a random anonymous ID), `user.account_uuid`, `user.email`. `OTEL_METRICS_INCLUDE_SESSION_ID` and `OTEL_METRICS_INCLUDE_ACCOUNT_UUID` default to true. No client setting removes `user.email` or `organization.id`: only the receiver can drop them.

**Privacy of the content**

- `OTEL_LOG_USER_PROMPTS` is "disabled" by default; the prompt is redacted and only its length is sent. Assistant text, tool details, raw tool output and raw API bodies each have their own variable, also off by default (`OTEL_LOG_ASSISTANT_RESPONSES`, `OTEL_LOG_TOOL_DETAILS`, `OTEL_LOG_TOOL_CONTENT`, `OTEL_LOG_RAW_API_BODIES`).
- Tool names and some path attributes are still exported without these flags (reported by the research run, not re-checked). Tracing is a separate, beta path that can carry content.

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

- The session ID appears as the transcript file name, as `session_id` in hooks and in the status line, and as `session.id` in OpenTelemetry. The documentation states that the status line `prompt_id` "Matches the `prompt.id` attribute on OpenTelemetry events"; **it does not state that `session_id` and `session.id` are the same value** (no official source). A test on a real session must check it before any rule relies on it (open question 3).
- Inside one source: assistant messages that share an ID must be counted once (SDK cost tracking). Subagents write their own transcripts; the SDK `usage` field leaves subagents out while `total_cost_usd` includes them ([cost tracking](https://code.claude.com/docs/en/agent-sdk/cost-tracking), not re-checked). The importer must not add a subagent transcript and its parent's totals.
- Across sources: one measurement per session and source (ADR-0022), then SOB-146 picks which source wins. Suggested input for it: prefer OpenTelemetry, then the status line, then the transcript, because the first two are official interfaces and the last one is not.
- The cost figures are client-side estimates at list price, not billing ([cost tracking](https://code.claude.com/docs/en/agent-sdk/cost-tracking), [status line](https://code.claude.com/docs/en/statusline)); they must not be matched to a bill.

## 5. Where the import is built

ADR-0019 decision 4 (option 4a) puts the **log import in a crate of `sobr`**: it runs in the app and writes to its session store. The plugin goes to the `claude-code` repo, and the MCP server goes to `claude-code` if it is a separate process talking to the HTTP API, or to `sobr` if it is a module of the server. This note confirms it:

- **Log import**: a crate of `sobr` (named with the `sobr-` prefix of ADR-0020), on the device only. It reads the allow-listed keys and writes sessions that follow the ADR-0022 contract. The removed `sobria-import` skeleton (SOB-67) gives nothing to reuse.
- **Status line script and hooks**: in the `claude-code` plugin. The script prints nothing useful to the user by itself; it hands the numbers to the local app. How it reaches the app (native messaging bridge of ADR-0013, a local socket, a file) is part of the plugin design, after open question 4.
- **OpenTelemetry**: a collector configuration plus a small receiver module in the organization server in `sobr`, not in the app.

## Open questions

Each one is an issue in Triage.

1. **Research, SOB-161**: find the transcript keys Sobr needs (model, token counts, a time, the session ID) from real sample files, and pin them with golden tests.
2. **Decision, SOB-162**: accept reading an unofficial, unstable transcript format to import history, with the guard rails above? Without it, there is no history.
3. **Research, SOB-163**: check on a real session whether the transcript session ID, `session_id` (hooks, status line) and `session.id` (OpenTelemetry) are one value; input for SOB-146.
4. **Decision, SOB-164**: how the live status line is enabled, given that a plugin cannot ship it (a one-time consent step that edits the user settings, or a documented manual step). How the script reaches the app is part of the plugin design.
5. **Research, SOB-166**: build a collector configuration for the metrics-only path and check, with a real Claude Code run, the paths, the headers, the temporality and what happens when the receiver is down.

## How this was checked

- Three read-only research runs opened the official pages (hooks and status line, OpenTelemetry, local logs). I re-read four key points on the pages myself: the transcript location, retention and the "format is internal" sentence ([Manage sessions](https://code.claude.com/docs/en/sessions)); the status line fields and its removal ([status line](https://code.claude.com/docs/en/statusline)); the `OTEL_LOG_USER_PROMPTS` default and the metric and event names; the OpenTelemetry "opt-in" sentence ([Monitoring usage](https://code.claude.com/docs/en/monitoring-usage)).
- Marked "reported by the research run, not re-checked", or not marked but outside the four points above: the plugin manifest limit on `statusLine`; the managed settings file paths and precedence; the SDK cost tracking rules; which hook events carry which fields (the run read the raw hooks page for these); the status line refresh timing; the facts taken from "The .claude directory" page (plaintext content, `history.jsonl`, `stats-cache.json`, subagent transcripts, the Windows path); the export intervals, the other metric names, the attribute defaults, the content flags other than `OTEL_LOG_USER_PROMPTS`, and the tracing remark.
- The pages are long and change often. Confirm a fact on its page before building on it, and read the version of Claude Code you target.

## Sources

[Manage sessions](https://code.claude.com/docs/en/sessions) · [The .claude directory](https://code.claude.com/docs/en/claude-directory) · [Hooks reference](https://code.claude.com/docs/en/hooks) · [Customize your status line](https://code.claude.com/docs/en/statusline) · [Plugins reference](https://code.claude.com/docs/en/plugins-reference) · [Discover plugins](https://code.claude.com/docs/en/discover-plugins) · [Plugin marketplaces](https://code.claude.com/docs/en/plugin-marketplaces) · [Monitoring usage](https://code.claude.com/docs/en/monitoring-usage) · [Managed settings](https://code.claude.com/docs/en/managed-settings) · [Data usage](https://code.claude.com/docs/en/data-usage) · [Agent SDK sessions](https://code.claude.com/docs/en/agent-sdk/sessions) · [Agent SDK cost tracking](https://code.claude.com/docs/en/agent-sdk/cost-tracking) · [Costs](https://code.claude.com/docs/en/costs) · [Non-interactive mode](https://code.claude.com/docs/en/headless)
