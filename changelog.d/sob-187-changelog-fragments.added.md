Changelog fragments, one file per pull request (2026-10-31, SOB-187)

- **`changelog.d/`**: each pull request adds one fragment file,
  `<issue>-<slug>.<category>.md`, instead of an entry in `CHANGELOG.md`, so
  two pull requests never conflict on the changelog. `changelog.d/README.md`
  gives the name, the categories and an example.
- **`scripts/changelog-assemble.sh`**: prints the release section built from
  the fragments, grouped by category. With `--write --version X.Y.Z` it
  writes the section into `CHANGELOG.md` and deletes the fragments. Plain
  Bash, no new dependency.
- **Rules:** CLAUDE.md §11 and §14, `CONTRIBUTING.md` (now in English) and
  the issue skill's Definition of Done say "add a fragment".
