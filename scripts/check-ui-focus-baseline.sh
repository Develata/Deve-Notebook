#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "ui-focus-baseline-check: $*" >&2
  exit 1
}

contains() {
  local file="$1"
  local pattern="$2"
  rg -q --fixed-strings -- "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

# UI-GEN-003: command palette traps focus and restores the previous surface.
contains docs/acceptance-cases/05_ui.md "case_id: UI-GEN-003"
contains docs/acceptance-cases/05_ui.md "scripts/check-ui-focus-baseline.sh"
contains docs/acceptance-cases/05_ui.md "command_palette_focus_trap_bound true"
contains docs/acceptance-cases/05_ui.md "command_palette_focus_restore_bound true"
contains docs/acceptance-cases/05_ui.md "settings_modal_focus_trap_bound true"
contains docs/acceptance-cases/05_ui.md "pending_navigation_modal_focus_trap_bound true"
contains docs/acceptance-cases/05_ui.md "merge_modal_focus_trap_bound true"
contains docs/acceptance-cases/05_ui.md "shared_modal_focus_restore_bound true"
contains docs/plan/08_ui_design.md "Modal MUST trap focus"
contains docs/plan/08_ui_design.md "restore previous focus target"

contains apps/web/src/components/mod.rs "pub(crate) mod focus_scope;"
contains apps/web/src/components/focus_scope.rs "FOCUSABLE_SELECTOR"
contains apps/web/src/components/focus_scope.rs "resolve_tab_target"
contains apps/web/src/components/focus_scope.rs "restore_focus_next_frame"
contains apps/web/src/components/focus_scope.rs "attach_modal_focus_restore_effect"
contains apps/web/src/components/focus_scope.rs ".cm-content"
contains apps/web/src/components/focus_scope.rs "focus_scope_tab_from_last_wraps_to_first"
contains apps/web/src/components/focus_scope.rs "focus_scope_shift_tab_from_first_wraps_to_last"

contains apps/web/src/components/search_box/effects.rs "previous_focus.set_value(focus_scope::active_element())"
contains apps/web/src/components/search_box/effects.rs "focus_scope::restore_focus_next_frame(previous)"
contains apps/web/src/components/search_box/ui.rs "role=\"dialog\""
contains apps/web/src/components/search_box/ui.rs "aria-modal=\"true\""
contains apps/web/src/components/search_box/ui.rs "focus_scope::handle_focus_trap_keydown(&ev, panel_ref)"
contains apps/web/src/components/search_box/ui_sections.rs "node_ref=input_ref"

contains apps/web/src/components/command_palette/mod.rs "logic::attach_focus_restore_effect(show, input_ref)"
contains apps/web/src/components/command_palette/logic.rs "previous_focus.set_value(focus_scope::active_element())"
contains apps/web/src/components/command_palette/logic.rs "focus_scope::restore_focus_next_frame(previous)"
contains apps/web/src/components/command_palette/ui.rs "role=\"dialog\""
contains apps/web/src/components/command_palette/ui.rs "aria-modal=\"true\""
contains apps/web/src/components/command_palette/ui.rs "focus_scope::handle_focus_trap_keydown(&ev, panel_ref)"
contains apps/web/src/components/command_palette/ui.rs "node_ref=input_ref"

contains apps/web/src/components/settings.rs "focus_scope::attach_modal_focus_restore_effect"
contains apps/web/src/components/settings.rs "role=\"dialog\""
contains apps/web/src/components/settings.rs "aria-modal=\"true\""
contains apps/web/src/components/settings.rs "focus_scope::handle_focus_trap_keydown(&ev, panel_ref)"
contains apps/web/src/components/settings.rs "node_ref=close_button_ref"

contains apps/web/src/components/pending_navigation_modal.rs "focus_scope::attach_modal_focus_restore_effect"
contains apps/web/src/components/pending_navigation_modal.rs "role=\"dialog\""
contains apps/web/src/components/pending_navigation_modal.rs "aria-modal=\"true\""
contains apps/web/src/components/pending_navigation_modal.rs "focus_scope::handle_focus_trap_keydown(&ev, panel_ref)"
contains apps/web/src/components/pending_navigation_modal.rs "node_ref=cancel_button_ref"

contains apps/web/src/components/merge_modal.rs "focus_scope::attach_modal_focus_restore_effect"
contains apps/web/src/components/merge_modal.rs "role=\"dialog\""
contains apps/web/src/components/merge_modal.rs "aria-modal=\"true\""
contains apps/web/src/components/merge_modal.rs "focus_scope::handle_focus_trap_keydown(&ev, panel_ref)"
contains apps/web/src/components/merge_modal.rs "node_ref=close_button_ref"

echo "ui-focus-baseline-check: ok"
