#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STYLE_DIR="$ROOT_DIR/apps/web/style"

fail() {
  echo "ui-token-baseline-check: $*" >&2
  exit 1
}

contains() {
  local file="$1"
  local pattern="$2"
  rg -q --fixed-strings -- "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

assert_no_hex_outside_token_files() {
  local found=0
  while IFS= read -r -d '' file; do
    case "$(basename "$file")" in
      _variables.css|_variables-dark.css) continue ;;
    esac
    if rg -n '#([0-9a-fA-F]{3,6})' "$file"; then
      found=1
    fi
  done < <(find "$STYLE_DIR" -type f -name '*.css' -print0)
  (( found == 0 )) || fail "hex color literals must stay inside style token files"
}

# UI-GEN-001: literal color values are owned by design-token files.
contains docs/acceptance-cases/05_ui.md "case_id: UI-GEN-001"
contains docs/acceptance-cases/05_ui.md "scripts/check-ui-token-baseline.sh"
contains docs/plan/08_ui_design.md "design tokens MUST 通过 CSS variables 暴露"
contains apps/web/style/_variables.css "_variables.css — Design Tokens"
contains apps/web/style/_variables-dark.css "_variables-dark.css — Design Tokens"
contains apps/web/style/_variables.css "--color-added"
contains apps/web/style/_variables.css "--color-modified"
contains apps/web/style/_variables.css "--color-deleted"
contains apps/web/style/codemirror.css '@import "./_variables.css";'
assert_no_hex_outside_token_files

echo "ui-token-baseline-check: ok"
