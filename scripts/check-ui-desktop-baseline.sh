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
contains docs/acceptance-cases/05_ui.md "run: scripts/check-ui-desktop-baseline.sh"
contains docs/acceptance-cases/05_ui.md "run: cargo test -p deve_web desktop_diff_scroll -- --nocapture"
contains docs/acceptance-cases/05_ui.md "cli_assert: desktop_canonical_column_markers_bound true"
contains docs/acceptance-cases/05_ui.md "cli_assert: desktop_diff_scroll_sync_ratio_bound true"

contains apps/web/src/components/desktop_layout_sidebar.rs "data-deve-desktop-col=\"1-sidebar\""
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
contains apps/web/src/hooks/use_layout_resize.rs "fn resized_width_for_target("
contains apps/web/src/hooks/use_layout_resize.rs "fn desktop_layout_resize_sidebar_clamps_to_bounds()"
contains apps/web/src/hooks/use_layout_resize.rs "fn desktop_layout_resize_right_panel_uses_inverse_delta_and_clamps()"
contains apps/web/src/hooks/use_layout_resize.rs "fn desktop_layout_resize_outer_gutter_uses_side_direction_and_clamps()"

echo "ui-desktop-baseline-check: ok"
