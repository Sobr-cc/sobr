Characterization tests of the ingest pipeline, CLI and downloader (2026-10-31, SOB-222)

- **`sobria-ingest`**: 20 tests in `tests/` record what the code does today,
  without changing it. Golden files pin the Silver rows of both sources, the
  Gold `referentiel.sqlite` (tables, columns, rows) and the datasheet, with
  the run time, hashes and file sizes masked. The real binary runs on a temp
  folder with no network (its proxy variables point at a closed port) for
  `validate`, `silver`, `gold`, `pipeline run --incremental` and an unknown
  source. Other tests pin the Gold rebuild per source, the same-day Copper
  overwrite, the join of a datacenter outside France, and the downloader's
  retry from byte 0 and its whole-body timeout. No production code changed.
- Pinned quirks are named `quirk`. New: a run where every download fails
  exits 0 and replaces Gold with an empty one (SOB-239). Already known:
  `gold` runs the whole pipeline and `--incremental` does nothing (SOB-123);
  the other audit findings are left to SOB-21.
- Not covered: `sources/territoire_fr.rs` (SOB-189).
