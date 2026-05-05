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

echo "ui-desktop-baseline-check: ok"
