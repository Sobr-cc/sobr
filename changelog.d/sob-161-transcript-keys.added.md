Claude Code transcript keys and synthetic samples (2026-10-31, SOB-161)

- **`docs/sources/CLAUDE-CODE-TRANSCRIPT-KEYS.md`**: the keys the history import needs
  (session ID, time, version, `message.id`, model, the token counts), the keys that
  hold text or personal data (to skip), how one API response is written on several
  lines with the same usage (7,920 of 10,130 IDs in the sample; dedupe by
  `message.id`), how subagent files relate to their session, and the `cost-state`
  lines. Made with Claude Code 2.1.314 by a script that prints key names only.
- **`docs/sources/samples/claude-code/`**: three synthetic files with the same shapes
  and made-up values, for the golden tests of the future reader (one session, one
  subagent, and malformed lines for the fail-closed cases). No real prompt, path,
  name or ID.
