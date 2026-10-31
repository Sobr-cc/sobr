ADR-0025 amended: who owns a message ID that a fork repeats (2026-10-31, SOB-243)

- **ADR-0025, Amendment 1** (Proposed): `claude --resume --fork-session` writes a new
  transcript that repeats all the lines of its parent (SOB-163). Each `message.id` now has
  one owning session on the device, the one whose transcript file is the oldest, with a
  table of owners that survives the deletion of old transcripts. A session's report counts
  only the message IDs it owns, so the person's totals count a shared line once and the
  fork shows only its own new lines.
- Five test cases on forks (both orders of reading, a deleted parent, a restored older file,
  several forks, the order of file times) and the changes to build issue 2.
