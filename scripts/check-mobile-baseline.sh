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
  contains_case_block_in_file "docs/acceptance-cases/05_ui.md" "$1" "$2"
}

contains_case_block_in_file() {
  local file="$1"
  local case_id="$2"
  local pattern="$3"
  awk -v case_id="$case_id" -v pattern="$pattern" '
    $0 == "- case_id: " case_id { in_case = 1; next }
    in_case && $0 ~ /^- case_id: / { in_case = 0 }
    in_case && index($0, pattern) { found = 1 }
    END { exit found ? 0 : 1 }
  ' "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file acceptance case $case_id"
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

# UI-MOB-007: collapsed mobile bottom bar shows one-line key status.
contains docs/acceptance-cases/05_ui.md "case_id: UI-MOB-007"
contains_case_block UI-MOB-007 "run: scripts/check-mobile-baseline.sh"
contains_case_block UI-MOB-007 "run: cargo test -p deve_web mobile_bottom_bar_collapsed -- --nocapture"
contains_case_block UI-MOB-007 "连接状态为 Ready"
contains_case_block UI-MOB-007 "ui_query_text: \"Branch\""
contains_case_block UI-MOB-007 "ui_query_text: \"Ready\""
contains_case_block UI-MOB-007 "ui_query_text: \"Words\""
contains_case_block UI-MOB-007 "ui_query_text: \"Lines\""
contains_case_block UI-MOB-007 "ui_query_text: \"Col\""
contains_case_block UI-MOB-007 "cli_assert: mobile_bottom_bar_collapsed_fields_bound true"
contains_case_block UI-MOB-007 "cli_assert: mobile_bottom_bar_single_line_marker_bound true"
contains_case_block UI-MOB-007 "cli_assert: mobile_bottom_bar_col_placeholder_bound true"
contains_case_block UI-MOB-007 "ui_assert: bottom_bar_collapsed true"
contains_case_block UI-MOB-007 "ui_assert: bottom_bar_single_line true"
contains docs/plan/08_ui_design_03_mobile.md '默认折叠态 **MUST** 仅显示一行：`Branch / Ready / Words / Lines / Col`。'
contains apps/web/src/components/mobile_layout/footer.rs "data-deve-mobile-bottom-bar=move || bottom_bar_state_attrs(expanded.get()).0"
contains apps/web/src/components/mobile_layout/footer.rs "data-deve-mobile-bottom-bar-lines=move || bottom_bar_state_attrs(expanded.get()).1"
contains apps/web/src/components/mobile_layout/footer.rs "fn mobile_bottom_bar_collapsed_state_is_single_line()"
contains apps/web/src/components/mobile_layout/footer_summary.rs "data-deve-mobile-bottom-bar-row=\"summary\""
contains apps/web/src/components/mobile_layout/footer_summary.rs "data-deve-mobile-bottom-bar-single-line=\"true\""
contains apps/web/src/components/mobile_layout/footer_summary.rs "data-deve-mobile-bottom-bar-fields-overflow=\"scroll-x\""
contains apps/web/src/components/mobile_layout/footer_summary.rs "data-deve-mobile-bottom-bar-field=\"branch\""
contains apps/web/src/components/mobile_layout/footer_summary.rs "data-deve-mobile-bottom-bar-field=\"status\""
contains apps/web/src/components/mobile_layout/footer_summary.rs "data-deve-mobile-bottom-bar-field=\"words\""
contains apps/web/src/components/mobile_layout/footer_summary.rs "data-deve-mobile-bottom-bar-field=\"lines\""
contains apps/web/src/components/mobile_layout/footer_summary.rs "data-deve-mobile-bottom-bar-field=\"col\""
contains apps/web/src/components/mobile_layout/footer_summary.rs "data-deve-mobile-bottom-bar-col-source=\"placeholder\""
contains apps/web/src/components/mobile_layout/footer_summary.rs "fn mobile_bottom_bar_collapsed_fields_scroll_horizontally_without_wrapping()"
contains apps/web/src/components/mobile_layout/footer_summary.rs "fn mobile_bottom_bar_collapsed_summary_exposes_required_fields()"
contains apps/web/src/i18n/bottom_bar.rs "pub fn branch(locale: Locale) -> &'static str"
contains apps/web/src/i18n/bottom_bar.rs "pub fn col(locale: Locale) -> &'static str"

# UI-MOB-008: mobile bottom bar expands, collapses by toggle, and dismisses by
# outside overlay click.
contains docs/acceptance-cases/05_ui.md "case_id: UI-MOB-008"
contains_case_block UI-MOB-008 "run: scripts/check-mobile-baseline.sh"
contains_case_block UI-MOB-008 "run: cargo test -p deve_web mobile_bottom_bar_expand -- --nocapture"
contains_case_block UI-MOB-008 "ui_click: \"bottom_bar_toggle\""
contains_case_block UI-MOB-008 "ui_assert: bottom_bar_expanded true"
contains_case_block UI-MOB-008 "ui_click: \"outside_bottom_bar\""
contains_case_block UI-MOB-008 "cli_assert: mobile_bottom_bar_toggle_marker_bound true"
contains_case_block UI-MOB-008 "cli_assert: mobile_bottom_bar_outside_dismiss_marker_bound true"
contains_case_block UI-MOB-008 "cli_assert: mobile_bottom_bar_details_marker_bound true"
contains_case_block UI-MOB-008 "cli_assert: mobile_bottom_bar_expand_state_transition_bound true"
contains_case_block UI-MOB-008 "ui_assert: bottom_bar_collapsed true"
contains docs/plan/08_ui_design_03_mobile.md "通过右侧箭头按钮展开详情；再次点击或点击状态栏外区域自动收起。"
contains apps/web/src/components/mobile_layout/footer.rs "data-deve-mobile-bottom-bar-dismiss=\"outside_bottom_bar\""
contains apps/web/src/components/mobile_layout/footer.rs "pub(super) fn bottom_bar_after_outside_click("
contains apps/web/src/components/mobile_layout/footer.rs "fn mobile_bottom_bar_expand_outside_click_collapses()"
contains apps/web/src/components/mobile_layout/footer_summary.rs "data-deve-mobile-bottom-bar-toggle=\"bottom_bar_toggle\""
contains apps/web/src/components/mobile_layout/footer_summary.rs "pub(super) fn bottom_bar_after_toggle("
contains apps/web/src/components/mobile_layout/footer_summary.rs "fn mobile_bottom_bar_expand_toggle_flips_state()"
contains apps/web/src/components/mobile_layout/footer_details.rs "data-deve-mobile-bottom-bar-details=\"expanded\""

# UI-MOB-009: mobile touch targets expose stable markers and keep a minimum
# 44px tap area.
contains docs/acceptance-cases/05_ui.md "case_id: UI-MOB-009"
contains_case_block UI-MOB-009 "run: scripts/check-mobile-baseline.sh"
contains_case_block UI-MOB-009 "run: cargo test -p deve_web mobile_touch_targets -- --nocapture"
contains_case_block UI-MOB-009 "ui_measure: \"topbar_buttons\""
contains_case_block UI-MOB-009 "ui_open_drawer: \"left\""
contains_case_block UI-MOB-009 "ui_measure: \"drawer_close_buttons\""
contains_case_block UI-MOB-009 "ui_open_drawer: \"right\""
contains_case_block UI-MOB-009 "ui_measure: \"outline_toggle\""
contains_case_block UI-MOB-009 "ui_measure: \"bottom_bar_toggle\""
contains_case_block UI-MOB-009 "ui_click: \"bottom_bar_toggle\""
contains_case_block UI-MOB-009 "ui_click: \"outside_bottom_bar\""
contains_case_block UI-MOB-009 "ui_assert: bottom_bar_collapsed true"
contains_case_block UI-MOB-009 "ui_measure: \"accessory_toolbar_buttons\""
contains_case_block UI-MOB-009 "ui_measure: \"bottom_bar_playback_buttons\""
contains_case_block UI-MOB-009 "ui_focus_editor: true"
contains_case_block UI-MOB-009 "ui_wait_keyboard: true"
contains_case_block UI-MOB-009 "cli_assert: mobile_touch_target_markers_bound true"
contains_case_block UI-MOB-009 "cli_assert: mobile_touch_targets_min_size_bound true"
contains_case_block UI-MOB-009 "ui_assert: all_targets_min_size \"44x44\""
contains apps/web/src/components/mobile_layout/header.rs "data-deve-mobile-touch-target=\"topbar_buttons\""
contains apps/web/src/components/mobile_layout/header.rs "fn mobile_touch_targets_topbar_buttons_are_at_least_44px()"
contains apps/web/src/components/mobile_layout/drawers/left_header.rs "data-deve-mobile-touch-target=\"drawer_close_buttons\""
contains apps/web/src/components/mobile_layout/drawers/left_header.rs "fn mobile_touch_targets_left_drawer_close_button_is_at_least_44px()"
contains apps/web/src/components/mobile_layout/drawers/right.rs "data-deve-mobile-touch-target=\"drawer_close_buttons\""
contains apps/web/src/components/mobile_layout/drawers/right.rs "fn mobile_touch_targets_right_drawer_close_button_is_at_least_44px()"
contains apps/web/src/components/mobile_layout/outline_button.rs "data-deve-mobile-touch-target=\"outline_toggle\""
contains apps/web/src/components/mobile_layout/outline_button.rs "fn mobile_touch_targets_outline_toggle_is_at_least_44px()"
contains apps/web/src/components/mobile_layout/footer_summary.rs "data-deve-mobile-touch-target=\"bottom_bar_toggle\""
contains apps/web/src/components/mobile_layout/footer_summary.rs "fn mobile_touch_targets_bottom_bar_toggle_is_at_least_44px()"
contains apps/web/src/components/mobile_layout/toolbar.rs "data-deve-mobile-touch-target=\"accessory_toolbar_buttons\""
contains apps/web/src/components/mobile_layout/toolbar.rs "fn mobile_touch_targets_accessory_toolbar_buttons_are_at_least_44px()"
contains apps/web/src/components/mobile_layout/footer_playback.rs "data-deve-mobile-touch-target=\"bottom_bar_playback_buttons\""
contains apps/web/src/components/mobile_layout/footer_playback.rs "fn mobile_touch_targets_bottom_bar_playback_buttons_are_at_least_44px()"

# UI-MOB-010: mobile-visible copy must come from the i18n facade, not direct
# component string literals.
contains docs/acceptance-cases/05_ui.md "case_id: UI-MOB-010"
contains_case_block UI-MOB-010 "run: scripts/check-mobile-baseline.sh"
contains_case_block UI-MOB-010 "run: cargo test -p deve_web mobile_i18n -- --nocapture"
contains_case_block UI-MOB-010 "cli_assert: mobile_i18n_facade_keys_bound true"
contains_case_block UI-MOB-010 "cli_assert: mobile_layout_hardcoded_copy_absent true"
contains apps/web/src/i18n/header.rs "fn mobile_i18n_header_action_copy_has_facade_keys()"
contains apps/web/src/i18n/sidebar.rs "fn mobile_i18n_sidebar_drawer_copy_has_facade_keys()"
contains apps/web/src/i18n/bottom_bar.rs "fn mobile_i18n_bottom_bar_toggle_copy_has_facade_key()"
contains apps/web/src/components/mobile_layout/header.rs "t::header::file_tree(locale.get())"
contains apps/web/src/components/mobile_layout/header.rs "t::header::home(locale.get())"
contains apps/web/src/components/mobile_layout/header.rs "t::header::open(locale.get())"
contains apps/web/src/components/mobile_layout/header.rs "t::header::command(locale.get())"
contains apps/web/src/components/mobile_layout/outline_button.rs "t::header::toggle_outline(locale.get())"
contains apps/web/src/components/mobile_layout/drawers/left_header.rs "t::sidebar::close_file_tree(locale.get())"
contains apps/web/src/components/mobile_layout/drawers/right.rs "t::sidebar::close_outline(locale.get())"
contains apps/web/src/components/mobile_layout/footer_summary.rs "t::bottom_bar::toggle_status_details(locale.get())"
contains apps/web/src/components/mobile_layout/drawers/right.rs "t::sidebar::outline_unavailable(locale.get())"
contains apps/web/src/components/mobile_layout/drawers/right.rs "t::sidebar::no_headings_found(locale.get())"
rejects apps/web/src/components/mobile_layout "\"File tree\""
rejects apps/web/src/components/mobile_layout "\"Home\""
rejects apps/web/src/components/mobile_layout "\"Open Index\""
rejects apps/web/src/components/mobile_layout "\"Command Palette\""
rejects apps/web/src/components/mobile_layout "\"Toggle Outline\""
rejects apps/web/src/components/mobile_layout "\"Close file tree\""
rejects apps/web/src/components/mobile_layout "\"Close outline\""
rejects apps/web/src/components/mobile_layout "\"Toggle status details\""
rejects apps/web/src/components/mobile_layout "\"Outline unavailable\""
rejects apps/web/src/components/mobile_layout "\"No headings found\""

# UI-MOB-011: mobile AI Chat opens as a same-page fullscreen surface and closes
# back to the editor surface.
contains docs/acceptance-cases/05_ui.md "case_id: UI-MOB-011"
contains_case_block UI-MOB-011 "run: scripts/check-mobile-baseline.sh"
contains_case_block UI-MOB-011 "run: cargo test -p deve_web mobile_chat_page -- --nocapture"
contains_case_block UI-MOB-011 "ui_click: \"mobile_chat_chip\""
contains_case_block UI-MOB-011 "ui_assert: chat_page_fullscreen true"
contains_case_block UI-MOB-011 "ui_click: \"chat_close_button\""
contains_case_block UI-MOB-011 "cli_assert: mobile_chat_chip_marker_bound true"
contains_case_block UI-MOB-011 "cli_assert: mobile_chat_close_marker_bound true"
contains_case_block UI-MOB-011 "cli_assert: mobile_chat_fullscreen_marker_bound true"
contains_case_block UI-MOB-011 "cli_assert: mobile_chat_page_state_transition_bound true"
contains_case_block UI-MOB-011 "ui_assert: chat_page_fullscreen false"
contains_case_block UI-MOB-011 "ui_assert: editor_visible true"
contains docs/acceptance-cases/13_ui_mobile_chat_regression.md "case_id: UI-MOB-CHAT-REG-001"
contains_case_block_in_file docs/acceptance-cases/13_ui_mobile_chat_regression.md UI-MOB-CHAT-REG-001 "run: scripts/check-mobile-baseline.sh"
contains_case_block_in_file docs/acceptance-cases/13_ui_mobile_chat_regression.md UI-MOB-CHAT-REG-001 "run: cargo test -p deve_web mobile_chat_page -- --nocapture"
contains_case_block_in_file docs/acceptance-cases/13_ui_mobile_chat_regression.md UI-MOB-CHAT-REG-001 "ui_click: \"mobile_chat_chip\""
contains_case_block_in_file docs/acceptance-cases/13_ui_mobile_chat_regression.md UI-MOB-CHAT-REG-001 "ui_assert: chat_page_fullscreen true"
contains_case_block_in_file docs/acceptance-cases/13_ui_mobile_chat_regression.md UI-MOB-CHAT-REG-001 "ui_click: \"chat_close_button\""
contains_case_block_in_file docs/acceptance-cases/13_ui_mobile_chat_regression.md UI-MOB-CHAT-REG-001 "cli_assert: mobile_chat_chip_marker_bound true"
contains_case_block_in_file docs/acceptance-cases/13_ui_mobile_chat_regression.md UI-MOB-CHAT-REG-001 "cli_assert: mobile_chat_close_marker_bound true"
contains_case_block_in_file docs/acceptance-cases/13_ui_mobile_chat_regression.md UI-MOB-CHAT-REG-001 "cli_assert: mobile_chat_fullscreen_marker_bound true"
contains_case_block_in_file docs/acceptance-cases/13_ui_mobile_chat_regression.md UI-MOB-CHAT-REG-001 "cli_assert: mobile_chat_page_state_transition_bound true"
contains_case_block_in_file docs/acceptance-cases/13_ui_mobile_chat_regression.md UI-MOB-CHAT-REG-001 "ui_assert: chat_page_fullscreen false"
contains_case_block_in_file docs/acceptance-cases/13_ui_mobile_chat_regression.md UI-MOB-CHAT-REG-001 "ui_assert: editor_visible true"
contains apps/web/src/components/mobile_layout/chat_sheet.rs "data-deve-mobile-chat-action=\"mobile_chat_chip\""
contains apps/web/src/components/mobile_layout/chat_sheet.rs "data-deve-mobile-chat-page=move || mobile_chat_page_mode(expanded.get())"
contains apps/web/src/components/mobile_layout/chat_sheet.rs "data-deve-mobile-chat-fullscreen=move || expanded.get().to_string()"
contains apps/web/src/components/mobile_layout/chat_sheet.rs "set_expanded.set(mobile_chat_after_open())"
contains apps/web/src/components/mobile_layout/chat_sheet.rs "set_expanded.set(mobile_chat_after_close())"
contains apps/web/src/components/mobile_layout/chat_sheet.rs "fn mobile_chat_page_expands_to_fullscreen()"
contains apps/web/src/components/mobile_layout/chat_sheet.rs "fn mobile_chat_page_close_returns_to_editor_surface()"
contains apps/web/src/components/chat/header.rs "data-deve-mobile-chat-action=\"chat_close_button\""

# MOB-SHOULD-003: the editor text size must stay at 16px so iOS Safari does not
# zoom the page when the CodeMirror content area receives input focus.
contains apps/web/style/_base.css ".cm-content"
contains apps/web/style/_base.css "font-size: 16px;"
contains docs/plan/08_ui_design_03_mobile.md '**Font Size**: 默认字号 **SHOULD** 设为 `16px`'
contains docs/plan/08_ui_design_03_mobile.md 'Font Size：移动端编辑器默认字号 **SHOULD** 设为 `16px` 或更高'
contains_case_block UI-MOB-020 "run: scripts/check-mobile-baseline.sh"
contains_case_block UI-MOB-020 "cli_assert: mobile_editor_font_size_16px true"

echo "mobile-baseline-check: ok"
