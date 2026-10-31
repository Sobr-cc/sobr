Golden test of the changelog assembly script (2026-10-31, SOB-236)

- **`scripts/tests/changelog-assemble/`**: sample fragments (one per
  category, plus a CRLF file, a file with a BOM and a `gh-` name), a sample
  changelog and the expected outputs of the preview and of `--write`.
  `run.sh` compares them and checks that `--write` deletes the fragments.
- **CI**: the job `Changelog script — golden test` runs it on Linux and macOS,
  whose `awk` and `bash` differ from Git Bash.
