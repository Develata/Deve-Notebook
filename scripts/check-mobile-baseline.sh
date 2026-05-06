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

rejects() {
  local path="$1"
  local pattern="$2"
  if rg -q --fixed-strings -- "$pattern" "$ROOT_DIR/$path"; then
    fail "unexpected '$pattern' in $path"
  fi
}

contains_case_block() {
  local case_id="$1"
  local pattern="$2"
  awk -v case_id="$case_id" -v pattern="$pattern" '
    $0 == "- case_id: " case_id { in_case = 1; next }
    in_case && $0 ~ /^- case_id: / { in_case = 0 }
    in_case && index($0, pattern) { found = 1 }
    END { exit found ? 0 : 1 }
  ' "$ROOT_DIR/docs/acceptance-cases/05_ui.md" \
    || fail "missing '$pattern' in acceptance case $case_id"
}

# UI-MOB-001: narrow Web viewport maps to Mobile shell.
contains docs/acceptance-cases/05_ui.md "case_id: UI-MOB-001"
contains_case_block UI-MOB-001 "run: scripts/check-mobile-baseline.sh"
contains_case_block UI-MOB-001 "run: cargo test -p deve_web mobile_viewport_mapping -- --nocapture"
contains_case_block UI-MOB-001 "cli_assert: mobile_viewport_mapping_bound true"
contains_case_block UI-MOB-001 "cli_assert: mobile_layout_mode_marker_bound true"
contains_case_block UI-MOB-001 "ui_assert: layout_mode_eq \"mobile\""
contains apps/web/src/components/main_layout_contexts.rs "pub(crate) const MOBILE_BREAKPOINT_WIDTH: f64 = 768.0;"
contains apps/web/src/components/main_layout_contexts.rs "pub(crate) fn viewport_width_maps_to_mobile(width: f64) -> bool"
contains apps/web/src/components/main_layout_contexts.rs "fn mobile_viewport_mapping_uses_inclusive_768px_boundary()"
contains apps/web/src/components/mobile_layout/layout_frame.rs "data-deve-layout-mode=\"mobile\""

# UI-MOB-002: edge swipes open mobile side drawers and expose stable drawer markers.
contains docs/acceptance-cases/05_ui.md "case_id: UI-MOB-002"
contains_case_block UI-MOB-002 "run: scripts/check-mobile-baseline.sh"
contains_case_block UI-MOB-002 "run: cargo test -p deve_web mobile_drawer_edge_swipe -- --nocapture"
contains_case_block UI-MOB-002 "cli_assert: mobile_drawer_edge_swipe_threshold_bound true"
contains_case_block UI-MOB-002 "cli_assert: mobile_drawer_dom_marker_bound true"
contains_case_block UI-MOB-002 "ui_assert: left_drawer_open true"
contains_case_block UI-MOB-002 "ui_assert: right_drawer_open true"
contains apps/web/src/components/mobile_layout/gesture.rs "pub(super) fn resolve_swipe_outcome("
contains apps/web/src/components/mobile_layout/gesture_test.rs "fn mobile_drawer_edge_swipe_opens_left_from_left_edge()"
contains apps/web/src/components/mobile_layout/gesture_test.rs "fn mobile_drawer_edge_swipe_opens_right_from_right_edge()"
contains apps/web/src/components/mobile_layout/gesture_test.rs "fn mobile_drawer_edge_swipe_closes_open_drawers()"
contains apps/web/src/components/mobile_layout/gesture_test.rs "fn mobile_drawer_edge_swipe_ignores_short_drags()"
contains apps/web/src/components/mobile_layout/drawers/left.rs "data-deve-mobile-drawer=\"left\""
contains apps/web/src/components/mobile_layout/drawers/right.rs "data-deve-mobile-drawer=\"right\""
contains apps/web/src/components/mobile_layout/drawers/left.rs "data-deve-mobile-drawer-open=move || open.get().to_string()"
contains apps/web/src/components/mobile_layout/drawers/right.rs "data-deve-mobile-drawer-open=move || open.get().to_string()"

# UI-MOB-003: Mobile shell must not render desktop resize handles.
contains docs/acceptance-cases/05_ui.md "case_id: UI-MOB-003"
contains_case_block UI-MOB-003 "run: scripts/check-mobile-baseline.sh"
contains_case_block UI-MOB-003 "ui_query_dom: \".resizer-handle\""
contains_case_block UI-MOB-003 "cli_assert: mobile_resizer_handles_absent true"
contains_case_block UI-MOB-003 "ui_dom_count_eq: 0"
contains apps/web/src/components/desktop_layout_handles.rs "class=\"resizer-handle"
contains apps/web/src/components/desktop_chat_panel.rs "class=\"resizer-handle"
rejects apps/web/src/components/mobile_layout "resizer-handle"

# MOB-SHOULD-003: the editor text size must stay at 16px so iOS Safari does not
# zoom the page when the CodeMirror content area receives input focus.
contains apps/web/style/_base.css ".cm-content"
contains apps/web/style/_base.css "font-size: 16px;"
contains docs/plan/08_ui_design_03_mobile.md '**Font Size**: 默认字号 **SHOULD** 设为 `16px`'
contains docs/plan/08_ui_design_03_mobile.md 'Font Size：移动端编辑器默认字号 **SHOULD** 设为 `16px` 或更高'
contains_case_block UI-MOB-020 "run: scripts/check-mobile-baseline.sh"
contains_case_block UI-MOB-020 "cli_assert: mobile_editor_font_size_16px true"

echo "mobile-baseline-check: ok"
