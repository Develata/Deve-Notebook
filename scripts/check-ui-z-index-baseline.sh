#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "ui-z-index-baseline-check: $*" >&2
  exit 1
}

contains() {
  local file="$1"
  local pattern="$2"
  rg -q --fixed-strings -- "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

token_value() {
  local token="$1"
  local line
  line="$(rg -n --fixed-strings -- "$token:" "$ROOT_DIR/apps/web/style/_variables.css" | head -n 1 || true)"
  [[ -n "$line" ]] || fail "missing token $token"
  printf '%s\n' "$line" | sed -E 's/.*: ([0-9]+);.*/\1/'
}

assert_lt() {
  local left="$1"
  local right="$2"
  local left_value
  local right_value
  left_value="$(token_value "$left")"
  right_value="$(token_value "$right")"
  (( left_value < right_value )) || fail "$left must be below $right"
}

assert_no_private_z_levels() {
  local targets=(
    "$ROOT_DIR/apps/web/index.html"
    "$ROOT_DIR/apps/web/js"
    "$ROOT_DIR/apps/web/src"
    "$ROOT_DIR/apps/web/style"
  )
  local rg_common=(
    --glob '!editor.bundle.js'
    --glob '!editor.bundle.js.map'
  )

  if rg -n "${rg_common[@]}" 'z-index[[:space:]]*:[[:space:]]*[0-9]' "${targets[@]}"; then
    fail "raw numeric z-index must use shell registry tokens"
  fi
  if rg -n "${rg_common[@]}" 'zIndex[[:space:]]*=[[:space:]]*["'"'"']?[0-9]' "${targets[@]}"; then
    fail "raw numeric style.zIndex must use shell registry tokens"
  fi
  if rg -n "${rg_common[@]}" 'z-\[[[:space:]]*[0-9]' "${targets[@]}"; then
    fail "Tailwind arbitrary numeric z-index must use shell registry tokens"
  fi
  if rg -n "${rg_common[@]}" '\bz-[0-9]+\b' "${targets[@]}"; then
    fail "Tailwind numeric z-index utilities must use shell registry tokens"
  fi
}

assert_editor_bundle_is_current() {
  local bundle_targets=()
  local bundle_file
  for bundle_file in \
    "$ROOT_DIR/apps/web/js/editor.bundle.js" \
    "$ROOT_DIR/apps/web/js/editor.bundle.js.map"; do
    [[ -f "$bundle_file" ]] && bundle_targets+=("$bundle_file")
  done
  ((${#bundle_targets[@]} > 0)) || return

  if rg -n --fixed-strings 'z-50 min-w-[120px]' "${bundle_targets[@]}"; then
    fail "editor bundle still contains stale code menu z-50"
  fi
  if rg -n --fixed-strings 'z-20 p-0.5' "${bundle_targets[@]}"; then
    fail "editor bundle still contains stale code toolbar z-20"
  fi
}

# UI-GEN-002: shell layers use the canonical z-index registry.
contains docs/acceptance-cases/05_ui.md "case_id: UI-GEN-002"
contains docs/acceptance-cases/05_ui.md "scripts/check-ui-z-index-baseline.sh"
contains docs/plan/11_ui_design/index.md "shell 分层至少保留以下 z-index registry"
contains docs/plan/11_ui_design/index.md "--z-editor"
contains docs/plan/11_ui_design/index.md "--z-toast"
contains apps/web/style/_variables.css "--z-editor: 0;"
contains apps/web/style/_variables.css "--z-toast: 120;"
assert_lt "--z-editor" "--z-chrome"
assert_lt "--z-chrome" "--z-panels"
assert_lt "--z-panels" "--z-floating"
assert_lt "--z-floating" "--z-overlay"
assert_lt "--z-overlay" "--z-modal"
assert_lt "--z-modal" "--z-toast"

contains apps/web/src/components/search_box/ui_sheet/style.rs "z-[var(--z-overlay)]"
contains apps/web/src/components/command_palette/ui.rs "z-[var(--z-modal)]"
contains apps/web/src/components/disconnect_overlay.rs "z-[var(--z-toast)]"
contains apps/web/src/components/dropdown.rs "z-[calc(var(--z-floating)_+_1)]"
contains apps/web/src/components/dropdown/position.rs "fn viewport_height() -> f64"
contains apps/web/src/components/dropdown/position.rs ".and_then(|window| window.inner_height().ok())"
if rg -q --fixed-strings 'expect("window")' "$ROOT_DIR/apps/web/src/components/dropdown/position.rs"; then
  fail "dropdown viewport height must not panic when window is unavailable"
fi
contains apps/web/src/components/mobile_layout/drawers/left/tabs/more_menu.rs "z-[calc(var(--z-floating)_+_1)]"
contains apps/web/src/components/mobile_layout/drawers/mod.rs "z-[calc(var(--z-overlay)_+_1)]"
contains apps/web/index.html "z-[var(--z-toast)]"
contains apps/web/js/extensions/code_menu.js "menu.style.zIndex = \"var(--z-floating)\""
contains apps/web/js/extensions/code_toolbar.js "container.style.zIndex = \"calc(var(--z-editor) + 1)\""
contains apps/web/style/tailwind.css "z-index: var(--z-modal);"
contains apps/web/style/_base.css "z-index: var(--z-toast);"
assert_no_private_z_levels
assert_editor_bundle_is_current

echo "ui-z-index-baseline-check: ok"
