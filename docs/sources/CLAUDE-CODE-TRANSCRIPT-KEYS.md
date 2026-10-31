# Claude Code transcripts: the keys Sobr needs (WIP, SOB-161)

> **Status**: work in progress, not reviewed. Findings made on 2026-10-31 with Claude Code **2.1.314** (the installed version); the files surveyed were written by several versions, among them 2.1.270, 2.1.310 and 2.1.314. The format is internal to Claude Code and changes between versions ([Manage sessions](https://code.claude.com/docs/en/sessions#where-transcripts-are-stored)), so every key below must be pinned by golden files and read with the fail-closed reader of `CLAUDE-CODE-USAGE-CAPTURE.md` (section 1).
> **Method**: a script read 40 session files and 70 subagent files (of 40 and 718 on disk, about 84,000 lines) and printed only key names, value types and, for a short allow-list (model, token counts, timestamps, IDs, version), example values. No text value was printed or copied. Counts below come from that run.

## Where the files are

- Main session: `~/.claude/projects/<project>/<sessionId>.jsonl`. The `sessionId` of every line (69,315 of 69,315) equals the file name.
- Subagents: `~/.claude/projects/<project>/<sessionId>/subagents/agent-<agentId>.jsonl`. The `sessionId` of every line (59,921 of 59,921) is the **parent** session, the `agentId` is in the file name, and `isSidechain` is `true` (it is `false` in main files).
- One JSON object per line, no invalid line in the sample.

## Line types seen (`type`)

`assistant`, `user`, `attachment`, `system`, metadata lines (`ai-title`, `last-prompt`, `mode`, `permission-mode`, `agent-setting`, `pr-link`, `queue-operation`, `tools-used`), request records (`api-request`, `api-request-shape`, `api-request-blob`), and `cost-state`, `file-history-snapshot`, `file-history-delta`, `relocated`, `frame-link`, `bridge-session`. Only `assistant` lines carry per-call usage. **Not surveyed yet**: the keys of `user` and `attachment` lines, and the exact content of `api-request` lines.

## Keys of an `assistant` line

| Need | Key | Type | Example (no prompt text) |
|---|---|---|---|
| Session ID | `sessionId` | string | a UUID |
| Time | `timestamp` | string | ISO 8601 UTC, `2026-..-..T..:..:..Z` |
| Claude Code version | `version` | string | `2.1.314` |
| Working directory | `cwd` | string | a path (**do not store**) |
| Branch | `gitBranch` | string | a branch name (**do not store**; see SOB-203) |
| Message ID (dedupe key) | `message.id` | string | `msg_<id>`; a UUID on some lines (not checked which) |
| Request ID | `requestId` | string | on 33,029 assistant lines of the main files |
| Model | `message.model` | string | `claude-opus-5-5`, `claude-sonnet-5-5`, `claude-opus-5`, and `<synthetic>` (28 lines: not a real call, skip) |
| Input tokens | `message.usage.input_tokens` | int | `2`, `4` |
| Output tokens | `message.usage.output_tokens` | int | `161`, `41` |
| Cache read | `message.usage.cache_read_input_tokens` | int | `0`, `27366` |
| Cache creation | `message.usage.cache_creation_input_tokens` | int | `8229` |
| Cache creation, 5 min / 1 h | `message.usage.cache_creation.ephemeral_5m_input_tokens`, `...ephemeral_1h_input_tokens` | int | `0`, `8229` |
| Thinking tokens | `message.usage.output_tokens_details.thinking_tokens` | int or null | `103` |
| Other | `message.usage.service_tier`, `speed`, `server_tool_use.web_search_requests`, `server_tool_use.web_fetch_requests`, `inference_geo` | string, int, null | `standard`, `0` |
| Sidechain | `isSidechain` | bool | `false` in main files, `true` in subagents |
| Entry point | `entrypoint` | string | `cli`, `claude-vscode` |

`message.usage.iterations[]` repeats the token counts of the same call (`input_tokens`, `output_tokens`, the cache counts, `type`). **Never add it to the totals of `message.usage`.**

## Keys that hold text (skip them)

- `user` lines: `message.content` (the prompt, or `tool_result` items).
- `assistant` lines: `message.content[]` items of type `text` (`text`), `thinking` (`thinking`, `signature`), `tool_use` (`input.*`, `name`, `id`, `caller`).
- `tool_result` items: `content`, `tool_use_id`, `is_error`.
- Line types that are text by themselves: `last-prompt` (`lastPrompt`), `ai-title` (`aiTitle`), `queue-operation` (`content`), `system` (`commandRun.*`).
- Dictionaries with free keys: `wireToolInputs.<toolu id>.*` (the inputs of tool calls, including prompts, commands and paths) and `wireIngestContext.<toolu id>.cwd`.
- `api-request` lines (`params.*`, bodies) and `file-history-*` lines can hold file content and paths.

## One message on several lines

Of 10,130 assistant message IDs in the sample, **7,920 appear on 2 to 32 lines** (one line per content block of the same API response), and the `usage` is **identical on every line of the same ID** (7,920 of 7,920). A reader that adds the usage of every line counts such a call two to three times. Dedupe by `message.id` inside a file and keep one `usage`. No `message.id` was found in two different files, and none is shared between a subagent file and its parent (0 of 10,826).

## Subagents

A subagent writes its own file with its own calls; its messages do not appear in the parent file. The usage of a session is therefore the deduplicated usage of the main file **plus** the deduplicated usage of the files in its `subagents/` folder.

## `cost-state` lines (26 of the 40 main files have one)

Cumulative per session: `modelUsage.<model>.{inputTokens, outputTokens, cacheReadInputTokens, cacheCreationInputTokens, cacheCreationEphemeral5mInputTokens, cacheCreationEphemeral1hInputTokens, thinkingTokens, webSearchRequests, costUSD}`, plus `totalCostUSD`, `totalAPIDuration`, `startTime`, `sessionId`. Compared with the deduplicated sums (main file plus subagents) on 12 sessions: the cache counts match in 11 of 12; one session with 14 subagent messages matches on all four counts; in most others `inputTokens` and `outputTokens` of `cost-state` are larger by about 1,200 and a few hundred, so it also counts requests that have no `assistant` line. **To check**: what those extra requests are, and whether `cost-state` is written at every exit.

## Left to do

- Survey the keys of `user`, `attachment` and `api-request` lines.
- Look at a resumed session and a compacted session (`system` lines with `compactMetadata`) for repeated messages across files.
- Write the synthetic sample files (same shapes, made-up text) and check that every key above appears in one.
