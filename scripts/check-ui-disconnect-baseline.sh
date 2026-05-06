#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "ui-disconnect-baseline-check: $*" >&2
  exit 1
}

check_contains() {
  local file="$1"
  local pattern="$2"
  rg -q --fixed-strings "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

# UI-WEB-002: visible disconnect lockdown and edit-disabled state.
check_contains docs/acceptance-cases/05_ui.md "case_id: UI-WEB-002"
check_contains docs/acceptance-cases/05_ui.md "run: scripts/check-ui-disconnect-baseline.sh"
check_contains docs/acceptance-cases/05_ui.md "run: cargo test -p deve_web disconnected_lockdown -- --nocapture"
check_contains docs/acceptance-cases/05_ui.md "ui_assert: overlay_text \"Reconnecting...\""
check_contains docs/acceptance-cases/05_ui.md "ui_assert: editing_disabled true"

check_contains apps/web/src/components/disconnect_overlay.rs "data-deve-disconnect-overlay=\"lockdown\""
check_contains apps/web/src/components/disconnect_overlay.rs "data-deve-editing-disabled=\"true\""
check_contains apps/web/src/components/disconnect_overlay.rs "fn overlay_copy(locale: Locale, status: ConnectionStatus) -> Option"
check_contains apps/web/src/components/disconnect_overlay.rs "ConnectionStatus::Unauthorized | ConnectionStatus::Connected => None"
check_contains apps/web/src/i18n/common.rs "Locale::En => \"Reconnecting...\""

check_contains apps/web/src/hooks/use_core/state_init/build_spectator.rs "connection_status.get() != ConnectionStatus::Connected"
check_contains apps/web/src/hooks/use_core/state_init.rs "fn disconnected_lockdown_marks_core_as_spectator()"
check_contains apps/web/src/hooks/use_core/state_init.rs "fn disconnected_lockdown_releases_when_connection_is_ready()"
check_contains apps/web/src/hooks/use_core/write_gate.rs "is_read_only: signals.is_spectator.get() || signals.active_branch.get().is_some()"
check_contains apps/web/src/hooks/use_core/write_gate_tests.rs "fn repo_write_gate_blocks_remote_branches_as_read_only()"
check_contains apps/web/src/editor/mod.rs "fn should_editor_be_read_only("
check_contains apps/web/src/editor/mod.rs "fn editor_read_only_gate_blocks_native_runtime_write_gate()"
check_contains apps/web/src/editor/mod.rs "ffi::set_read_only(should_readonly);"

echo "ui-disconnect-baseline-check: ok"
