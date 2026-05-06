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

# UI-MOB-004: visualViewport keyboard offset keeps the accessory toolbar above keyboard.
contains docs/acceptance-cases/05_ui.md "case_id: UI-MOB-004"
contains_case_block UI-MOB-004 "run: scripts/check-mobile-baseline.sh"
contains_case_block UI-MOB-004 "run: cargo test -p deve_web mobile_toolbar_keyboard -- --nocapture"
contains_case_block UI-MOB-004 "cli_assert: mobile_toolbar_visual_viewport_offset_bound true"
contains_case_block UI-MOB-004 "cli_assert: mobile_toolbar_dom_marker_bound true"
contains_case_block UI-MOB-004 "ui_assert: toolbar_visible true"
contains_case_block UI-MOB-004 "ui_assert: toolbar_not_overlapped_by_keyboard true"
contains docs/plan/08_ui_design_03_mobile.md '必须使用 `visualViewport` API 监听键盘高度变化'
contains apps/web/src/components/mobile_layout/effects.rs "visual_viewport_keyboard_offset("
contains apps/web/src/components/mobile_layout/effects.rs "fn mobile_toolbar_keyboard_offset_uses_visual_viewport_overlap()"
contains apps/web/src/components/mobile_layout/effects.rs "fn mobile_toolbar_keyboard_offset_clamps_to_zero_without_overlap()"
contains apps/web/src/components/mobile_layout/toolbar.rs "data-deve-mobile-toolbar=\"accessory\""
contains apps/web/src/components/mobile_layout/toolbar.rs "data-deve-keyboard-offset=move || keyboard_offset.get().to_string()"
contains apps/web/src/components/mobile_layout/toolbar.rs "fn mobile_toolbar_keyboard_style_places_toolbar_above_keyboard()"

# UI-MOB-005: mobile search opens as a top sheet and closes from handle upward swipe.
contains docs/acceptance-cases/05_ui.md "case_id: UI-MOB-005"
contains_case_block UI-MOB-005 "run: scripts/check-mobile-baseline.sh"
contains_case_block UI-MOB-005 "run: cargo test -p deve_web mobile_search_sheet -- --nocapture"
contains_case_block UI-MOB-005 "cli_assert: mobile_search_top_sheet_bound true"
contains_case_block UI-MOB-005 "cli_assert: mobile_search_sheet_swipe_close_bound true"
contains_case_block UI-MOB-005 "cli_assert: mobile_search_sheet_handle_marker_bound true"
contains_case_block UI-MOB-005 "ui_assert: search_sheet_position \"top\""
contains_case_block UI-MOB-005 "ui_swipe: \"search_sheet_handle\" (direction: \"up\", distance: 90)"
contains_case_block UI-MOB-005 "ui_assert: search_sheet_closed true"
contains docs/plan/08_ui_design_03_mobile.md "点击 Search -> Top Sheet 自上而下展开。"
contains docs/plan/08_ui_design_03_mobile.md "关闭手势以顶部拖拽上滑为主"
contains apps/web/src/components/main_layout_overlays.rs "SearchUiMode::Sheet"
contains apps/web/src/components/search_box/ui.rs "data-deve-search-sheet-position=move || ui_sheet::sheet_position(ui_mode.get())"
rejects apps/web/src/components/search_box/ui.rs "data-deve-search-sheet-open"
contains apps/web/src/components/search_box/ui_sheet_style.rs "data-deve-search-sheet-handle=\"top\""
contains apps/web/src/components/search_box/ui_sheet_style.rs "SearchUiMode::Overlay => None"
contains apps/web/src/components/search_box/ui_sheet_style.rs "fn mobile_search_sheet_is_positioned_at_top()"
contains apps/web/src/components/search_box/sheet_gesture.rs "pub(super) fn should_close_by_drag("
contains apps/web/src/components/search_box/sheet_gesture.rs "fn mobile_search_sheet_upward_handle_swipe_closes()"

# UI-MOB-006: search results scroll must not trigger top sheet dismissal.
contains docs/acceptance-cases/05_ui.md "case_id: UI-MOB-006"
contains_case_block UI-MOB-006 "run: scripts/check-mobile-baseline.sh"
contains_case_block UI-MOB-006 "run: cargo test -p deve_web mobile_search_results_scroll -- --nocapture"
contains_case_block UI-MOB-006 "cli_assert: mobile_search_results_scroll_isolated_bound true"
contains_case_block UI-MOB-006 "cli_assert: mobile_search_results_swipe_does_not_close_bound true"
contains_case_block UI-MOB-006 "ui_scroll: \"search_results\" (delta: 200)"
contains_case_block UI-MOB-006 "ui_swipe: \"search_results\" (direction: \"up\", distance: 80)"
contains_case_block UI-MOB-006 "ui_assert: search_sheet_closed false"
contains docs/plan/08_ui_design_03_mobile.md "避免与结果列表滚动冲突"
contains apps/web/src/components/search_box/ui_sections.rs "data-deve-search-results-scroll=move || search_results_scroll_marker(ui_mode.get())"
contains apps/web/src/components/search_box/ui_sections.rs "SearchUiMode::Sheet => Some(\"isolated\")"
contains apps/web/src/components/search_box/ui_sections.rs "SearchUiMode::Overlay => None"
contains apps/web/src/components/search_box/ui_sections.rs "fn mobile_search_results_scroll_marker_is_sheet_only()"
contains apps/web/src/components/search_box/sheet_gesture.rs "pub(super) fn can_start_dismiss_by_zone("
contains apps/web/src/components/search_box/sheet_gesture.rs "fn mobile_search_results_scroll_does_not_start_dismiss()"
contains apps/web/src/components/search_box/sheet_gesture.rs "fn mobile_search_results_scroll_swipe_cannot_close_sheet()"

# MOB-SHOULD-003: the editor text size must stay at 16px so iOS Safari does not
# zoom the page when the CodeMirror content area receives input focus.
contains apps/web/style/_base.css ".cm-content"
contains apps/web/style/_base.css "font-size: 16px;"
contains docs/plan/08_ui_design_03_mobile.md '**Font Size**: 默认字号 **SHOULD** 设为 `16px`'
contains docs/plan/08_ui_design_03_mobile.md 'Font Size：移动端编辑器默认字号 **SHOULD** 设为 `16px` 或更高'
contains_case_block UI-MOB-020 "run: scripts/check-mobile-baseline.sh"
contains_case_block UI-MOB-020 "cli_assert: mobile_editor_font_size_16px true"

echo "mobile-baseline-check: ok"
