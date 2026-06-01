#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "ui-desktop-baseline-check: $*" >&2
  exit 1
}

contains() {
  local file="$1"
  local pattern="$2"
  rg -q --fixed-strings -- "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
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

# UI-DESK-001: desktop workbench exposes canonical column markers and ratio-based split diff scroll sync.
contains docs/acceptance-cases/05_ui.md "case_id: UI-DESK-001"
contains_case_block UI-DESK-001 "run: scripts/check-ui-desktop-baseline.sh"
contains_case_block UI-DESK-001 "run: cargo test -p deve_web desktop_diff_scroll -- --nocapture"
contains_case_block UI-DESK-001 "cli_assert: desktop_canonical_column_markers_bound true"
contains_case_block UI-DESK-001 "cli_assert: desktop_diff_scroll_sync_ratio_bound true"

contains apps/web/src/components/desktop_layout/sidebar.rs "data-deve-desktop-col=\"1-sidebar\""
contains apps/web/src/components/diff_view/split_pane.rs "data-deve-desktop-col=\"2-diff-old\""
contains apps/web/src/components/diff_view/split_pane.rs "data-deve-desktop-col=\"3-editor\""
contains apps/web/src/editor/mod.rs "data-deve-desktop-col=\"4-outline\""
contains apps/web/src/components/desktop_chat_panel.rs "data-deve-desktop-col=\"5-chat\""

contains apps/web/src/components/diff_view/split_pane.rs "synced_scroll_top("
contains apps/web/src/components/diff_view/split_pane.rs "source_top as f64 / source_max as f64"
contains apps/web/src/components/diff_view/split_pane.rs "right.set_scroll_top(target_top);"
contains apps/web/src/components/diff_view/split_pane.rs "left.set_scroll_top(target_top);"
contains apps/web/src/components/diff_view/split_pane.rs "set_syncing_left.set(false);"
contains apps/web/src/components/diff_view/split_pane.rs "set_syncing_right.set(false);"
contains apps/web/src/components/diff_view/split_pane.rs "fn desktop_diff_scroll_syncs_col3_to_col2_by_ratio()"
contains apps/web/src/components/diff_view/split_pane.rs "fn desktop_diff_scroll_sync_skips_noop_target_update()"

# UI-DESK-002: desktop resize bounds and persisted layout width keys stay bound.
contains docs/acceptance-cases/05_ui.md "case_id: UI-DESK-002"
contains_case_block UI-DESK-002 "run: scripts/check-ui-desktop-baseline.sh"
contains_case_block UI-DESK-002 "run: cargo test -p deve_web desktop_layout_resize -- --nocapture"
contains_case_block UI-DESK-002 "cli_assert: desktop_layout_resize_bounds_bound true"
contains_case_block UI-DESK-002 "cli_assert: desktop_layout_width_persistence_keys_bound true"

contains apps/web/src/hooks/use_layout.rs "const SIDEBAR_MIN: i32 = 180;"
contains apps/web/src/hooks/use_layout.rs "const SIDEBAR_MAX: i32 = 500;"
contains apps/web/src/hooks/use_layout.rs "const RIGHT_MIN: i32 = 240;"
contains apps/web/src/hooks/use_layout.rs "const RIGHT_MAX: i32 = 520;"
contains apps/web/src/hooks/use_layout.rs "const OUTER_MIN: i32 = 0;"
contains apps/web/src/hooks/use_layout.rs "const OUTER_MAX: i32 = 120;"
contains apps/web/src/hooks/use_layout.rs "read_width(\"ui_sidebar_width\")"
contains apps/web/src/hooks/use_layout.rs "read_width(\"ui_right_panel_width\")"
contains apps/web/src/hooks/use_layout.rs "read_width(\"ui_outer_gutter\")"
contains apps/web/src/hooks/use_layout.rs "write_width(\"ui_sidebar_width\","
contains apps/web/src/hooks/use_layout.rs "write_width(\"ui_right_panel_width\","
contains apps/web/src/hooks/use_layout.rs "write_width(\"ui_outer_gutter\","
contains apps/web/src/hooks/use_layout/resize.rs "fn resized_width_for_target("
contains apps/web/src/hooks/use_layout/resize.rs "fn desktop_layout_resize_sidebar_clamps_to_bounds()"
contains apps/web/src/hooks/use_layout/resize.rs "fn desktop_layout_resize_right_panel_uses_inverse_delta_and_clamps()"
contains apps/web/src/hooks/use_layout/resize.rs "fn desktop_layout_resize_outer_gutter_uses_side_direction_and_clamps()"

# UI-DESK-003: Unified Search prefixes route to stable modes and expose a DOM marker.
contains docs/acceptance-cases/05_ui.md "case_id: UI-DESK-003"
contains_case_block UI-DESK-003 "run: scripts/check-ui-desktop-baseline.sh"
contains_case_block UI-DESK-003 "run: cargo test -p deve_web unified_search_mode -- --nocapture"
contains_case_block UI-DESK-003 "run: scripts/check-repo-file-ops-baseline.sh"
contains_case_block UI-DESK-003 "cli_assert: unified_search_mode_prefix_routing_bound true"
contains_case_block UI-DESK-003 "cli_assert: unified_search_mode_dom_marker_bound true"
contains_case_block UI-DESK-003 "cli_assert: repo_file_ops_baseline_bound true"
contains_case_block UI-DESK-003 "ui_assert: mode_eq \"command\""
contains_case_block UI-DESK-003 "ui_assert: mode_eq \"file-op\""
contains_case_block UI-DESK-003 "ui_assert: mode_eq \"branch\""
contains_case_block UI-DESK-003 "ui_assert: mode_eq \"file\""

contains apps/web/src/components/search_box/logic/providers.rs "pub(crate) enum SearchSurfaceMode"
contains apps/web/src/components/search_box/logic/providers.rs "pub(crate) fn search_surface_mode(query: &str) -> SearchSurfaceMode"
contains apps/web/src/components/search_box/logic/providers.rs "SearchSurfaceMode::Command"
contains apps/web/src/components/search_box/logic/providers.rs "SearchSurfaceMode::Branch"
contains apps/web/src/components/search_box/logic/providers.rs "SearchSurfaceMode::File"
contains apps/web/src/components/search_box/logic/providers/tests.rs "fn unified_search_mode_routes_command_branch_file_prefixes()"
contains apps/web/src/components/search_box/logic/providers/tests.rs "fn unified_search_mode_routes_extended_prefixes()"
contains apps/web/src/components/search_box/logic/providers/tests.rs "fn unified_search_mode_exposes_stable_dom_values()"
contains apps/web/src/components/search_box/logic/providers/tests.rs "SearchSurfaceMode::FileOp"
contains apps/web/src/components/search_box/ui_sections.rs "data-deve-search-mode=move || search_surface_mode(&query.get()).as_str()"

# UI-WEB-004: Activity Bar More row selection and Pin/Unpin stay separate.
contains docs/acceptance-cases/05_ui.md "case_id: UI-WEB-004"
contains_case_block UI-WEB-004 "run: scripts/check-ui-desktop-baseline.sh"
contains_case_block UI-WEB-004 "run: cargo test -p deve_web activity_more -- --nocapture"
contains_case_block UI-WEB-004 "ui_click: \"activity-more-button\""
contains_case_block UI-WEB-004 "ui_click: \"activity_more_item_explorer\""
contains_case_block UI-WEB-004 "ui_click: \"activity_more_pin_search\""
contains_case_block UI-WEB-004 "cli_assert: activity_more_button_marker_bound true"
contains_case_block UI-WEB-004 "cli_assert: activity_more_item_marker_bound true"
contains_case_block UI-WEB-004 "cli_assert: activity_more_pin_action_marker_bound true"
contains_case_block UI-WEB-004 "cli_assert: activity_more_item_close_bound true"
contains_case_block UI-WEB-004 "cli_assert: activity_more_pin_keeps_menu_open_bound true"
contains_case_block UI-WEB-004 "ui_assert: more_menu_visible false"
contains_case_block UI-WEB-004 "ui_assert: pinned_state_updated true"

contains docs/report/web-activity-more-browser-smoke-2026-05-17.md "Browser smoke result: pass."
contains docs/report/web-activity-more-browser-smoke-2026-05-17.md "menuStillOpen=true"
contains docs/report/web-activity-more-browser-smoke-2026-05-17.md "Row click and Pin/Unpin are separate in the actual DOM event path."
contains apps/web/src/components/activity_bar/mod.rs "data-deve-activity-more-button=activity_more_button_marker()"
contains apps/web/src/components/activity_bar/mod.rs "fn activity_more_button_marker_is_stable()"
contains apps/web/src/components/activity_bar/popup_menu.rs "data-deve-activity-more-item=activity_more_item_marker(item)"
contains apps/web/src/components/activity_bar/popup_menu.rs "data-deve-activity-more-pin-action=activity_more_pin_action_marker(item)"
contains apps/web/src/components/activity_bar/popup_menu.rs "pub(super) fn activity_more_after_item_click() -> bool"
contains apps/web/src/components/activity_bar/popup_menu.rs "pub(super) fn activity_more_after_pin_click(open: bool) -> bool"
contains apps/web/src/components/activity_bar/popup_menu.rs "pub(super) fn toggle_activity_more_pin("
contains apps/web/src/components/activity_bar/popup_menu/tests.rs "fn activity_more_menu_items_cover_sidebar_entries()"
contains apps/web/src/components/activity_bar/popup_menu/tests.rs "fn activity_more_pin_actions_cover_sidebar_entries()"
contains apps/web/src/components/activity_bar/popup_menu/tests.rs "fn activity_more_item_click_closes_menu()"
contains apps/web/src/components/activity_bar/popup_menu/tests.rs "fn activity_more_pin_click_keeps_menu_state()"
contains apps/web/src/components/activity_bar/popup_menu/tests.rs "fn activity_more_pin_toggle_adds_unpinned_view()"
contains apps/web/src/components/activity_bar/popup_menu/tests.rs "fn activity_more_pin_toggle_removes_pinned_view()"

# UI-WEB-005: Repo Switcher uses button semantics and outside-click dismissal.
contains docs/acceptance-cases/05_ui.md "case_id: UI-WEB-005"
contains_case_block UI-WEB-005 "run: scripts/check-ui-desktop-baseline.sh"
contains_case_block UI-WEB-005 "run: cargo test -p deve_web repo_switcher -- --nocapture"
contains_case_block UI-WEB-005 "ui_click: \"repo-switcher-trigger\""
contains_case_block UI-WEB-005 "ui_click: \"repo-switcher-outside\""
contains_case_block UI-WEB-005 "cli_assert: repo_switcher_trigger_button_bound true"
contains_case_block UI-WEB-005 "cli_assert: repo_switcher_menu_marker_bound true"
contains_case_block UI-WEB-005 "cli_assert: repo_switcher_item_button_bound true"
contains_case_block UI-WEB-005 "cli_assert: repo_switcher_outside_dismiss_bound true"
contains_case_block UI-WEB-005 "ui_assert: repo_switcher_menu_visible true"
contains_case_block UI-WEB-005 "ui_assert: repo_switcher_menu_visible false"

contains docs/report/web-repo-switcher-browser-smoke-2026-05-17.md "Browser smoke result: pass."
contains docs/report/web-repo-switcher-browser-smoke-2026-05-17.md "Repo Switcher trigger/menu/item/outside-click behavior matches the Web shell contract in the actual DOM event path."
contains docs/report/web-repo-switcher-browser-smoke-2026-05-17.md 'Browser console `error` / `warn` count after the final reload was `0`.'
contains docs/report/web-repo-switcher-browser-smoke-2026-05-17.md "focus_scope::attach_modal_focus_restore_effect"
contains apps/web/src/components/sidebar/repo_switcher.rs "11_ui_design/01_web#web-layout-persistence"
contains apps/web/src/components/sidebar/repo_switcher.rs "data-deve-repo-switcher-trigger=repo_switcher_trigger_marker()"
contains apps/web/src/components/sidebar/repo_switcher.rs "data-deve-repo-switcher-backdrop=repo_switcher_backdrop_marker()"
contains apps/web/src/components/sidebar/repo_switcher.rs "data-deve-repo-switcher-menu=move || repo_switcher_menu_marker(show_menu.get())"
contains apps/web/src/components/sidebar/repo_switcher.rs "data-deve-repo-switcher-item=repo_switcher_item_marker()"
contains apps/web/src/components/sidebar/repo_switcher.rs "type=\"button\""
contains apps/web/src/components/sidebar/repo_switcher.rs "role=\"menu\""
contains apps/web/src/components/sidebar/repo_switcher.rs "role=\"menuitem\""
contains apps/web/src/components/sidebar/repo_switcher.rs "fn repo_switcher_trigger_click_toggles_menu()"
contains apps/web/src/components/sidebar/repo_switcher.rs "fn repo_switcher_outside_click_closes_menu()"
contains apps/web/src/components/sidebar/repo_switcher.rs "fn repo_switcher_item_click_closes_menu()"

# UI-WEB-006: Web PWA manifest surface stays standalone and self-hosted.
contains docs/acceptance-cases/05_ui.md "case_id: UI-WEB-006"
contains_case_block UI-WEB-006 "run: scripts/check-ui-desktop-baseline.sh"
contains_case_block UI-WEB-006 "run: cd apps/web && NO_COLOR=true BROWSERSLIST_IGNORE_OLD_DATA=true trunk build"
contains_case_block UI-WEB-006 "cli_assert: pwa_manifest_link_bound true"
contains_case_block UI-WEB-006 "cli_assert: pwa_manifest_copied_to_dist true"
contains_case_block UI-WEB-006 "cli_assert: pwa_manifest_standalone_bound true"
contains_case_block UI-WEB-006 "cli_assert: pwa_theme_color_bound true"

contains apps/web/index.html '<meta name="theme-color" content="#1e1e1e" />'
contains apps/web/index.html '<link rel="manifest" href="/manifest.json" />'
contains apps/web/index.html '<link data-trunk rel="copy-file" href="public/manifest.json" />'
contains apps/web/public/manifest.json '"display": "standalone"'
contains apps/web/public/manifest.json '"theme_color": "#1e1e1e"'
contains apps/web/public/manifest.json '"start_url": "/"'
contains apps/web/public/manifest.json '"scope": "/"'
contains apps/web/public/manifest.json '"src": "/favicon.svg"'
contains docs/report/web-pwa-manifest-browser-smoke-2026-05-17.md "Browser smoke result: pass."
contains docs/report/web-pwa-manifest-browser-smoke-2026-05-17.md "PWA manifest metadata is available through the actual static-file serving path."
contains docs/report/web-pwa-manifest-browser-smoke-2026-05-17.md 'Browser console `error` / `warn` count was `0`.'
contains docs/report/web-pwa-manifest-browser-smoke-2026-05-17.md "content-type: application/json"

echo "ui-desktop-baseline-check: ok"
