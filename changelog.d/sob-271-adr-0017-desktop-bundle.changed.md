ADR-0017 rule 2 states what the desktop binary contains (2026-10-31, SOB-271)

- **`docs/adr/ADR-0017-contrat-demo-web.md`**: Amendment 1 (Proposed)
  corrects rule 2. The demo module and its fixtures are in `web/build`, so
  Tauri embeds them in the desktop binary; the lazy import only keeps them
  from being loaded there. A `vite build` puts the fixtures in one 467 KB
  chunk. No rule changes, and the original text is kept.
