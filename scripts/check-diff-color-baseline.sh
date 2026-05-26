#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "diff-color-baseline-check: $*" >&2
  exit 1
}

check_contains() {
  local file="$1"
  local pattern="$2"
  rg -q --fixed-strings -- "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

check_absent() {
  local file="$1"
  local pattern="$2"
  if rg -q --fixed-strings -- "$pattern" "$ROOT_DIR/$file"; then
    fail "unexpected '$pattern' in $file"
  fi
}

# DIFF-007: editor gutter diff colors use canonical semantic tokens.
check_contains docs/acceptance-cases/04_diff.md "case_id: DIFF-007"
check_contains docs/acceptance-cases/04_diff.md "scripts/check-diff-color-baseline.sh"
check_contains docs/acceptance-cases/04_diff.md 'gutter_color_added "var(--color-added)"'
check_contains docs/acceptance-cases/04_diff.md 'gutter_color_modified "var(--color-modified)"'
check_contains docs/acceptance-cases/04_diff.md 'gutter_color_deleted "var(--color-deleted)"'
check_contains docs/plan/11_ui_design/index.md '`--color-added`'
check_contains docs/plan/11_ui_design/index.md '`--color-modified`'
check_contains docs/plan/11_ui_design/index.md '`--color-deleted`'
check_contains apps/web/js/extensions/gutter_diff.js 'added: "var(--color-added)"'
check_contains apps/web/js/extensions/gutter_diff.js 'modified: "var(--color-modified)"'
check_contains apps/web/js/extensions/gutter_diff.js 'deleted: "var(--color-deleted)"'
check_contains apps/web/js/extensions/gutter_diff.js "deveGutterDiffKind"
check_contains apps/web/js/extensions/gutter_diff.js "backgroundColor"
check_absent apps/web/js/extensions/gutter_diff.js "#81b88b"
check_absent apps/web/js/extensions/gutter_diff.js "#e2c08d"
check_absent apps/web/js/extensions/gutter_diff.js "#e06c75"

echo "diff-color-baseline-check: ok"
