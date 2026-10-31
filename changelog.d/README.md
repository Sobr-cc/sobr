# Changelog fragments

Each pull request adds **one file** here instead of editing `CHANGELOG.md`.
Two pull requests never touch the same file, so their changelog entries
never conflict. At each release, `scripts/changelog-assemble.sh` moves the
fragments into `CHANGELOG.md`.

## File name

`<issue>-<slug>.<category>.md`, in lower case:

- `<issue>`: the Linear issue, such as `sob-123`;
- `<slug>`: a few words joined by hyphens;
- `<category>`: one of `added`, `changed`, `fixed`, `removed`, `security`.

Example: `sob-123-retry-the-upload.fixed.md`. A pull request that fits two
categories adds one fragment per category.

## Content

English, in the style of the entries already in `CHANGELOG.md`:

- the first line is the title, followed by the date and the issue in
  parentheses;
- then a blank line, and the entry as a Markdown list.

Example, `changelog.d/sob-123-retry-the-upload.fixed.md`:

```markdown
Upload retried after a timeout (2026-11-02, SOB-123)

- **`sobria-app`**: an upload that times out is retried once, then queued.
```

The script adds the heading `### Fixed — ` in front of the title.

## Release

```sh
# Print the section built from the fragments, without changing anything.
scripts/changelog-assemble.sh

# Write it into CHANGELOG.md as release 1.0.0 and delete the fragments.
scripts/changelog-assemble.sh --version 1.0.0 --write
```

`--write` turns the `## [Non publié]` (or `## [Unreleased]`) heading into the
release heading, puts the fragments' entries first, and keeps the entries
already under it. `--date` sets the release date (today by default).
