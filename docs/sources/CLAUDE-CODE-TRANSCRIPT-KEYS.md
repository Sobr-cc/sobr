# Claude Code transcripts: the keys Sobr needs (SOB-161)

> **Status**: proposal for review. Findings made on 2026-10-31 with Claude Code **2.1.314** (the installed version); the files surveyed were written by several versions, among them 2.1.270, 2.1.310 and 2.1.314. The format is internal to Claude Code and changes between versions ([Manage sessions](https://code.claude.com/docs/en/sessions#where-transcripts-are-stored)), so every key below must be pinned by golden files and read with the fail-closed reader of `CLAUDE-CODE-USAGE-CAPTURE.md` (section 1).
> **Method**: a script read 40 session files and 70 subagent files (of 40 and 718 on disk, about 84,000 lines) and printed only key names, value types and, for a short allow-list (model, token counts, timestamps, IDs, version), example values. No text value was printed or copied. Counts below come from that run.

## Where the files are

- Main session: `~/.claude/projects/<project>/<sessionId>.jsonl`. The `sessionId` of every line (69,315 of 69,315) equals the file name.
- Subagents: `~/.claude/projects/<project>/<sessionId>/subagents/agent-<agentId>.jsonl`. The `sessionId` of every line (59,921 of 59,921) is the **parent** session, the `agentId` is in the file name, and `isSidechain` is `true` (it is `false` in main files).
- One JSON object per line, no invalid line in the sample.

## Line types seen (`type`)

`assistant`, `user`, `attachment`, `system`, metadata lines (`ai-title`, `last-prompt`, `mode`, `permission-mode`, `agent-setting`, `pr-link`, `queue-operation`, `tools-used`), request records (`api-request`, `api-request-shape`, `api-request-blob`), and `cost-state`, `file-history-snapshot`, `file-history-delta`, `relocated`, `frame-link`, `bridge-session`. Only `assistant` lines carry per-call usage. The request records (`api-request`) hold the request parameters (`params.model`, `params.max_tokens`, `params.thinking`, `querySource`, hashes) but **no usage and no cost**, so they cannot double count; `querySource` is `sdk` on all 14,331 of them. `user` lines carry the prompt or the tool results, and `attachment` lines carry context (including a `context.userEmail`, `context.gitStatus` and file contents): a reader never opens either.

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
- `user` lines also carry `toolUseResult` (the payload of a tool), `origin.*` (a message handed over by another session, with a `body`), `serverClassifierContext.*` (a working directory and git state) and, after a compaction, the summary text (`isCompactSummary: true`).
- `attachment` lines: `attachment.context.userEmail`, `attachment.context.gitStatus`, `attachment.files[].content` and `.path`, `attachment.content`, and more. Skip the whole line type.
- Dictionaries with free keys: `wireToolInputs.<toolu id>.*` (the inputs of tool calls, including prompts, commands and paths) and `wireIngestContext.<toolu id>.cwd`.
- `api-request-blob` lines hold messages of a request, and `file-history-*` lines can hold file content and paths.

## One message on several lines

Of 10,130 assistant message IDs in the sample, **7,920 appear on 2 to 32 lines** (one line per content block of the same API response), and the `usage` is **identical on every line of the same ID** (7,920 of 7,920). A reader that adds the usage of every line counts such a call two to three times. Dedupe by `message.id` and keep one `usage`. **The same `message.id` can also be in two files**: a session forked with `claude --resume <id> --fork-session` gets a new session ID and a new file that contains **all the lines of its parent** (same `uuid`, same `message.id`, same `usage`), with the new `sessionId` on every line (a test run on 2.1.314, on one short session: all 19 `uuid` values and both assistant `message.id` values of the parent were in the fork; SOB-163). Whether a fork of a long or compacted session, or on another version, copies everything is not known, so the reader deduplicates by `message.id` whatever the fork holds. So the deduplication must work across all the files of a user, not only inside one file. Without `--fork-session`, `--resume` keeps the same session ID and appends to the same file (same test, 2.1.314). On the files already on this machine (40 main files, 13,521 `msg_` IDs) no `message.id` is in two files, which only means that none of them was forked; and none is shared between a subagent file and its parent (0 of 10,826). A compaction adds one `system` line with `compactMetadata` and one `user` summary line (`isCompactSummary`, text, skipped) and does not repeat earlier messages (30 of each in the 40 files).

## Subagents

A subagent writes its own file with its own calls; its messages do not appear in the parent file. The usage of a session is therefore the deduplicated usage of the main file **plus** the deduplicated usage of the files in its `subagents/` folder.

## `cost-state` lines (26 of the 40 main files have one)

Cumulative per session: `modelUsage.<model>.{inputTokens, outputTokens, cacheReadInputTokens, cacheCreationInputTokens, cacheCreationEphemeral5mInputTokens, cacheCreationEphemeral1hInputTokens, thinkingTokens, webSearchRequests, costUSD}`, plus `totalCostUSD`, `totalAPIDuration`, `startTime`, `sessionId`. Compared with the deduplicated sums (main file plus subagents) on 12 sessions: the cache counts match in 11 of 12; one session with 14 subagent messages matches on all four counts; in most others `inputTokens` and `outputTokens` of `cost-state` are larger by about 1,200 and a few hundred, so it also counts requests that have no `assistant` line. **To check**: what those extra requests are, and whether `cost-state` is written at every exit.

## Synthetic samples

`docs/sources/samples/claude-code/` holds four files with the shapes above and made-up values only (no real prompt, path, name, email, URL or ID; IDs are `00000000-0000-4000-8000-...` and `msg_DEMO...`):

- `session-main.jsonl` (17 lines): a prompt, one API response written on three lines with the same `message.id` and `usage` (thinking, text, tool use), a tool result, a second response, an `attachment` line, the metadata lines, a compaction pair, a `<synthetic>` error message, an `api-request` and a `cost-state` line.
- `session-forked.jsonl` (17 lines): the lines of `session-main.jsonl` copied as a fork writes them, with another `sessionId` and the same `uuid` and `message.id` values, to test the deduplication across files. Its `cost-state` line is copied from the parent as is; what a real fork writes there is not known.
- `subagent-agent-demo01.jsonl` (3 lines): a subagent (`isSidechain: true`, an `agentId`, the `sessionId` of the main sample) with one response on two lines.
- `session-unknown-shapes.jsonl` (4 lines): a line type that does not exist, a `usage` where a count is a string, an `assistant` line without `usage`, and one without `model`. They are invented to test that the reader fails closed.

A script checked that every line is valid JSON, that each of the 34 keys named in the tables above appears in a sample, and that no real name, email, URL or non-made-up ID is in them.

## Open points

- What the extra requests counted by `cost-state` are (no `assistant` line, `querySource` is always `sdk`), and whether `cost-state` is written at every exit.
- Whether keys differ between Claude Code versions: the survey mixed versions 2.1.270 to 2.1.314 and did not compare them. The golden files must be refreshed for each version Sobr supports.
- Forks: what happens to usage that the parent already holds is a rule for SOB-146 and ADR-0025 (which session owns a `message.id` that is in two files); this note only gives the fact. Only `--resume` and `--resume --fork-session` were tried; other ways to start from an old session are not covered.
