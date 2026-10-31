Transcript keys note: a forked session copies its parent's lines (2026-10-31, SOB-242)

- **`docs/sources/CLAUDE-CODE-TRANSCRIPT-KEYS.md`**: the note said a resumed session does
  not copy messages. That holds for `--resume`, not for `--resume --fork-session`, which
  writes a new file with all the lines of its parent (same `uuid` and `message.id`, new
  `sessionId`). Deduplicate by `message.id` across files. Tested on Claude Code 2.1.314
  (SOB-163).
- **`docs/sources/samples/claude-code/session-forked.jsonl`**: a synthetic forked session
  for the golden tests of the future reader.
