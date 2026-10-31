Characterization specs of the web Understand pages (2026-10-31, SOB-224)

- **`web/tests`**: Playwright specs pin what the Understand pages do today, including
  defects found by the audit (tests named `quirk`). In demo mode: `/comparer?models=`
  opens 7 or 8 models because 4 defaults are added; the `/modeles` CO₂ sort ranks only
  the first 8 models of the catalogue; `?id=` and `?focus=` are ignored; the home page
  says the estimate is logged as entry #0 with "Chaîne intègre"; the "100 % local"
  claims. On the desktop path, through a new reusable IPC stub
  (`tests/helpers/tauri-stub.ts`, answers taken from the demo fixtures or small
  hand-written ones): `/methodologies` saves the settings read at page load and
  overwrites a change made meanwhile, the "Voir aussi" panel says all calculations are
  audited next to "not logged" cards, `/territoire` loads 200 sites once with no
  paging or notice, and `/datasheets` reports "Fichier écrit" even if the download is
  cancelled. No production code changed.
