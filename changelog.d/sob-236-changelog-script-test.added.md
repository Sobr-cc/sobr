Golden test of the changelog assembly script (2026-10-31, SOB-236)

- **`scripts/tests/changelog-assemble/`**: sample fragments (one per
  category, plus a CRLF file, a file with a BOM and a `gh-` name), two sample
  changelogs (with and without an unreleased section) and the expected
  outputs of the preview and of `--write`. `run.sh` compares them, checks that
  `--write` deletes the fragments, and checks that a title starting with a
  heading is refused before anything is written.
- **CI**: the job `Changelog script — golden test` runs it on Linux and macOS,
  whose `awk` and `bash` differ from Git Bash.
