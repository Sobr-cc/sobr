#!/usr/bin/env bash
# Assemble the changelog fragments of changelog.d/ into a release section.
# See changelog.d/README.md for the fragment format.
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/changelog-assemble.sh [--version X.Y.Z] [--title TEXT]
                                     [--date YYYY-MM-DD] [--write]
                                     [--dir DIR] [--changelog FILE]

Prints the release section built from the fragments in DIR (default: the
repo's changelog.d), grouped by category.

  --write      writes the section into FILE (default: the repo's
               CHANGELOG.md) and deletes the fragments; needs --version
  --version    release number used in the heading
  --title      release title added to the heading
  --date       release date (default: today)
EOF
}

root=$(cd "$(dirname "$0")/.." && pwd)
dir="$root/changelog.d"
changelog="$root/CHANGELOG.md"
version=""
title=""
release_date=$(date +%Y-%m-%d)
write=0

while [ $# -gt 0 ]; do
  case "$1" in
    --version) version=${2:?--version needs a value}; shift 2 ;;
    --title) title=${2:?--title needs a value}; shift 2 ;;
    --date) release_date=${2:?--date needs a value}; shift 2 ;;
    --dir) dir=${2:?--dir needs a value}; shift 2 ;;
    --changelog) changelog=${2:?--changelog needs a value}; shift 2 ;;
    --write) write=1; shift ;;
    -h | --help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

if [ "$write" -eq 1 ] && [ -z "$version" ]; then
  echo "--write needs --version." >&2
  exit 2
fi

categories="added changed deprecated fixed removed security"
name_pattern='^[a-z]+-[0-9]+-[a-z0-9-]+\.(added|changed|deprecated|fixed|removed|security)\.md$'

# Every file except the README must be a well-named fragment, so a typo
# fails here instead of being skipped.
fragments=()
for f in "$dir"/*; do
  [ -f "$f" ] || continue
  base=$(basename "$f")
  [ "$base" = "README.md" ] && continue
  if ! [[ $base =~ $name_pattern ]]; then
    echo "Bad fragment name: $base (expected <issue>-<slug>.<category>.md)" >&2
    exit 1
  fi
  fragments+=("$f")
done

if [ "${#fragments[@]}" -eq 0 ]; then
  echo "No fragments in $dir." >&2
  exit 1
fi

title_case() {
  printf '%s' "$1" | awk '{ print toupper(substr($0, 1, 1)) substr($0, 2) }'
}

# Prints the entries of every fragment, grouped by category, in name order.
# Each body loses its leading and trailing blank lines, and each entry ends
# with exactly one blank line.
entries() {
  local category label f entry_title
  for category in $categories; do
    label=$(title_case "$category")
    for f in "${fragments[@]}"; do
      case "$f" in *".$category.md") ;; *) continue ;; esac
      entry_title=$(head -n 1 "$f" | tr -d '\r')
      if [ -z "$entry_title" ]; then
        echo "Empty title in $f." >&2
        exit 1
      fi
      printf '### %s — %s\n\n' "$label" "$entry_title"
      tail -n +2 "$f" | tr -d '\r' | awk '
        NF { if (seen) for (i = 0; i < blank; i++) print ""; blank = 0; seen = 1; print; next }
        { blank++ }
      '
      printf '\n'
    done
  done
}

if [ -n "$version" ]; then
  heading="## [$version] — $release_date"
  [ -n "$title" ] && heading="$heading — $title"
else
  heading="## [Unreleased]"
fi

section=$(mktemp)
trap 'rm -f "$section" "$section.out"' EXIT
entries > "$section"

if [ "$write" -eq 0 ]; then
  printf '%s\n\n' "$heading"
  cat "$section"
  exit 0
fi

# Only the first release heading is looked at. If it is the unreleased one,
# it becomes the release heading and the fragments' entries go first;
# otherwise the section goes before it.
awk -v heading="$heading" -v section="$section" '
  function print_section(   line) {
    print heading
    print ""
    while ((getline line < section) > 0) print line
    close(section)
  }
  { sub(/\r$/, "") }
  !done && /^## \[(Non publié|Unreleased)\]/ { print_section(); done = 1; skip_blank = 1; next }
  !done && /^## / { print_section(); done = 1 }
  skip_blank && /^$/ { skip_blank = 0; next }
  { skip_blank = 0; print }
  END { if (!done) print_section() }
' "$changelog" > "$section.out"
mv "$section.out" "$changelog"

rm -f "${fragments[@]}"
echo "Wrote \"$heading\" into $changelog and deleted ${#fragments[@]} fragment(s)." >&2
