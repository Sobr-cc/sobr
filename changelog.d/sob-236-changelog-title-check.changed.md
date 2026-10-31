Changelog script refuses a fragment title that starts with `#` (2026-10-31, SOB-236)

- **`scripts/changelog-assemble.sh`**: a title such as `### Fixed — x` used to
  become `### Added — ### Fixed — x`. The script now stops before writing and
  says the title must not carry the heading.
- **`docs/methodology/MODEL-PRESETS.md`**: §7 asks for a fragment in
  `changelog.d/` instead of an edit of `CHANGELOG.md`.
