#!/usr/bin/env bash
# Golden test of scripts/changelog-assemble.sh: preview, --write, and a title that starts with '#'.
# Runs on a copy of the fixtures, since --write rewrites the changelog and deletes the fragments.
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
  cp "$here/CHANGELOG.md" "$case/CHANGELOG.md"
}

assemble() {
  bash "$script" --dir "$case/fragments" --changelog "$case/CHANGELOG.md" "$@"
}

pass() { echo "ok   $1"; }
fail() { echo "FAIL $1"; failed=1; }

setup
assemble > "$work/preview.md"
if diff -u "$here/expected-preview.md" "$work/preview.md"; then pass preview; else fail preview; fi

assemble --version 1.0.0 --title "Test release" --date 2026-11-02 --write 2> /dev/null
if diff -u "$here/expected-write.md" "$case/CHANGELOG.md"; then pass write; else fail write; fi
left=$(ls "$case/fragments")
if [ "$left" = "README.md" ]; then pass "write deletes the fragments"; else fail "write left: $left"; fi

setup
printf '### Fixed — Heading in the title (2026-11-02, SOB-9)\n\n- **`docs`**: refused.\n' \
  > "$case/fragments/sob-9-hash-title.fixed.md"
if out=$(assemble --version 1.0.0 --write 2>&1); then
  fail "a title starting with '#' was accepted"
elif [[ $out == *"Title starts with '#'"* ]] && cmp -s "$here/CHANGELOG.md" "$case/CHANGELOG.md" \
  && [ -f "$case/fragments/sob-9-hash-title.fixed.md" ]; then
  pass "a title starting with '#' stops the script before writing"
else
  fail "a title starting with '#': $out"
fi

exit "$failed"
