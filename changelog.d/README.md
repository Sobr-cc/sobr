# Changelog fragments

Each pull request adds **one file** here instead of editing `CHANGELOG.md`.
Two pull requests never touch the same file, so their changelog entries
never conflict. At each release, `scripts/changelog-assemble.sh` moves the
fragments into `CHANGELOG.md`.

## File name

`<issue>-<slug>.<category>.md`, in lower case:

- `<issue>`: the Linear issue, such as `sob-123`. A contributor without a
  Linear issue uses `gh-` and the number of their pull request, such as
  `gh-58`;
- `<slug>`: a few words joined by hyphens;
- `<category>`: one of `added`, `changed`, `deprecated`, `removed`,
  `fixed`, `security`, in that order in the release.

Any other file in this folder, except this README, makes the release script
stop, so a typo in a name is caught.

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

The script adds the heading `### Fixed — ` in front of the title, so the
title itself must not start with `#` and a space: the script stops on it.

## Release

```sh
# Print the section built from the fragments, without changing anything.
scripts/changelog-assemble.sh

# Write it into CHANGELOG.md as release 1.0.0 and delete the fragments.
scripts/changelog-assemble.sh --version 1.0.0 --title "Public V1" --write
```

The heading follows the file's style: `## [1.0.0] — 2026-03-31 — Public V1`.
`--date` sets the date (today by default) and `--title` is optional.

`--write` looks only at the first release heading of `CHANGELOG.md`:

- if it is `## [Non publié]` (or `## [Unreleased]`), it becomes the release
  heading. The fragments' entries come first, grouped by category, followed
  by the entries written there before fragments existed. That second group
  is not regrouped; it happens only at the first release after the switch;
- otherwise, the section goes before it.
