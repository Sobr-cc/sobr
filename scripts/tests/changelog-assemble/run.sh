#!/usr/bin/env bash
# Golden test of scripts/changelog-assemble.sh: preview, --write, and titles that start with a heading.
# Runs on copies of the fixtures, since --write rewrites the changelog and deletes the fragments.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
script="$here/../../changelog-assemble.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
case="$work/case"
failed=0

setup() {
  rm -rf "$case"
  mkdir -p "$case"
  cp -R "$here/fragments" "$case/fragments"
  cp "$here/$1" "$case/CHANGELOG.md"
}

assemble() {
  bash "$script" --dir "$case/fragments" --changelog "$case/CHANGELOG.md" "$@"
}

pass() { echo "ok   $1"; }
fail() { echo "FAIL $1"; failed=1; }

compare() {
  if diff -u "$here/$2" "$3"; then pass "$1"; else fail "$1"; fi
}

# golden NAME CHANGELOG EXPECTED_PREVIEW EXPECTED_WRITE
golden() {
  setup "$2"
  if assemble > "$work/preview.md" 2> "$work/err"; then
    compare "$1: preview" "$3" "$work/preview.md"
  else
    fail "$1: preview exited non-zero: $(cat "$work/err")"
  fi
  if assemble --version 1.0.0 --title "Test release" --date 2026-11-02 --write 2> "$work/err"; then
    compare "$1: write" "$4" "$case/CHANGELOG.md"
    left=$(ls "$case/fragments")
    if [ "$left" = "README.md" ]; then pass "$1: write deletes the fragments"; else fail "$1: write left $left"; fi
  else
    fail "$1: write exited non-zero: $(cat "$work/err")"
  fi
}

golden unreleased CHANGELOG.md expected-preview.md expected-write.md
golden released CHANGELOG-released.md expected-preview-released.md expected-write-released.md

# refused NAME CONTENT [OPTIONS]: CONTENT is a printf format; the fragment stops the script before anything is written.
refused() {
  local name=$1 content=$2
  shift 2
  setup CHANGELOG.md
  printf "$content" > "$case/fragments/sob-9-heading-title.fixed.md"
  if out=$(assemble "$@" 2>&1); then
    fail "$name: accepted"
  elif [[ $out == *"Title starts with a heading"* ]] && cmp -s "$here/CHANGELOG.md" "$case/CHANGELOG.md" \
    && [ -f "$case/fragments/sob-9-heading-title.fixed.md" ]; then
    pass "$name: refused before writing"
  else
    fail "$name: $out"
  fi
}

refused "heading in the title, --write" '### Fixed — x (2026-11-02, SOB-9)\n\n- y.\n' --version 1.0.0 --write
refused "heading in the title, preview" '## x (2026-11-02, SOB-9)\n\n- y.\n'
refused "heading after a BOM, CRLF" '\357\273\277# x (2026-11-02, SOB-9)\r\n\r\n- y.\r\n' --version 1.0.0 --write

setup CHANGELOG.md
printf '#58 regression fixed (2026-11-02, GH-58)\n\n- y.\n' > "$case/fragments/gh-58-issue-number.fixed.md"
if out=$(assemble 2>&1) && [[ $out == *"### Fixed — #58 regression fixed"* ]]; then
  pass "an issue number at the start of a title is accepted"
else
  fail "an issue number at the start of a title: $out"
fi

exit "$failed"
