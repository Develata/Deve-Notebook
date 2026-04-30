#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "mobile-baseline-check: $*" >&2
  exit 1
}

contains() {
  local file="$1"
  local pattern="$2"
  rg -q --fixed-strings -- "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

# MOB-SHOULD-003: the editor text size must stay at 16px so iOS Safari does not
# zoom the page when the CodeMirror content area receives input focus.
contains apps/web/style/_base.css ".cm-content"
contains apps/web/style/_base.css "font-size: 16px;"
contains docs/plan/08_ui_design_03_mobile.md '| MOB-SHOULD-003 | Font Size: 默认字号 SHOULD 设为 16px | `apps/web/style/_base.css` | 已实现 |'
contains docs/acceptance-cases/05_ui.md "scripts/check-mobile-baseline.sh"
contains docs/acceptance-cases/05_ui.md "mobile_editor_font_size_16px true"

echo "mobile-baseline-check: ok"
