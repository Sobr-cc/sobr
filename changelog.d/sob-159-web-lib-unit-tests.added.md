Unit tests of the shared web code (2026-10-31, SOB-159)

- **`web/src/lib`**: Vitest tests pin what four modules do today:
  `demo/index.ts` (command table, models, estimates and grid-point choice,
  model detail, datacenters, preferences, dashboard periods), `preferences.ts`
  (store with optimistic save and rollback, persona and module catalogues),
  `team-store.ts` (load, optimistic field saves, rollback) and the IPC error
  normalization of `api.ts`. The IPC layer is mocked; no production code
  changed. Three defects found are pinned as they are today and tracked in
  SOB-183 (empty distribution values in the demo model detail), SOB-184
  (demo dashboard day labels one day early east of UTC, and a daily point
  count that changes at noon) and SOB-204 (the unknown-model message hardcodes
  a stale catalogue size).
