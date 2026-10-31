Changelog script refuses a fragment title that starts with a heading (2026-10-31, SOB-236)

- **`scripts/changelog-assemble.sh`**: a title such as `### Fixed — x` used to
  become `### Added — ### Fixed — x`. The script now stops before writing when
  a title starts with a heading marker (`#`, `##`… followed by a space, or
  alone); a title such as `#58 …` is still accepted. `changelog.d/README.md`
  states the rule.
- **`docs/methodology/MODEL-PRESETS.md`**: §7 asks for a fragment in
  `changelog.d/` instead of an edit of `CHANGELOG.md`.
