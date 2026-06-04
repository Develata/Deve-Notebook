#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "settings-local-feedback-baseline-check: $*" >&2
  exit 1
}

check_contains() {
  local file="$1"
  local pattern="$2"
  rg -q --fixed-strings "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

check_absent() {
  local file="$1"
  local pattern="$2"
  if rg -q --fixed-strings "$pattern" "$ROOT_DIR/$file"; then
    fail "unexpected '$pattern' in $file"
  fi
}

cd "$ROOT_DIR"

# SET-003/004: file-backed runtime config and effective backend fallback.
check_contains docs/acceptance-cases/15_settings_operation_refs.md "case_id: SET-003"
check_contains docs/acceptance-cases/15_settings_operation_refs.md "case_id: SET-004"
check_contains docs/features/operations/settings_persistence_apply.md "Runtime persistence uses \`config.toml\`"
check_contains apps/cli/src/commands/config.rs "toml::to_string_pretty(config)"
check_contains apps/cli/src/commands/config/tests.rs "set_core_key_writes_runtime_config"
check_contains apps/cli/src/commands/config/tests.rs "set_rejects_unknown_key"
check_contains apps/cli/src/server/agent_bridge/policy/tests.rs "trusted_cli_untrusted_policy_falls_back_to_native"
check_contains apps/cli/src/server/agent_bridge/http_tests.rs "backend_capabilities_http"

# SET-005/006: visible Settings feedback and reserved/disabled feedback.
check_contains docs/acceptance-cases/15_settings_operation_refs.md "case_id: SET-005"
check_contains docs/acceptance-cases/15_settings_operation_refs.md "case_id: SET-006"
check_contains docs/features/operations/settings_feedback_render.md "aria-disabled"
check_contains docs/features/operations/settings_ui_preferences.md "op.settings.ui.select-theme"
check_contains docs/features/operations/settings_ui_preferences.md "op.settings.ui.select-editor-preference"
check_contains docs/features/operations/settings_runtime_feedback.md "Reserved UI feedback is still non-authoritative"
check_contains apps/web/src/components/settings_sections_policy/tests.rs "language_buttons_reflect_current_locale"
check_contains apps/web/src/components/settings_sections_policy/tests.rs "theme_buttons_reflect_browser_local_preference"
check_contains apps/web/src/components/settings_sections_policy/tests.rs "editor_preference_buttons_reflect_local_feedback_state"
check_contains apps/web/src/components/settings_sections_policy/tests.rs "sync_mode_buttons_reflect_current_mode"
check_contains apps/web/src/components/settings_sections_policy/tests.rs "reserved_setting_state_exposes_disabled_reason"
check_contains apps/web/src/components/settings_sections_policy/tests.rs "trusted_cli_default_off_keeps_native_visible_and_disables_trusted_backend"
check_contains apps/web/src/components/settings_prefs.rs "theme_preference_defaults_to_auto_and_roundtrips"
check_contains apps/web/src/components/settings_prefs.rs "editor_preferences_default_to_safe_values_and_roundtrip"
check_contains apps/web/src/components/settings_sections/local_prefs.rs "data-deve-settings-editor-wrap"
check_contains apps/web/src/components/settings_sections/local_prefs.rs "data-deve-runtime-smoke=\"embedded\""
check_contains apps/web/style/tailwind.css "html[data-deve-editor-wrap=\"off\"] .cm-content"
check_contains docs/acceptance-cases/15_settings_operation_refs.md "ui_click: \"Dark\""
check_contains docs/features/13_settings.md "主题、自动换行、编辑器密度"

# SET-007: server-backed Settings API remains future-only.
check_contains docs/acceptance-cases/11_commands_settings.md "case_id: SET-007"
check_contains docs/acceptance-cases/11_commands_settings.md 'unsupported_key_rejected: "server.settings.api_enabled"'
check_contains apps/cli/src/commands/config/tests.rs "\"server.settings.api_enabled\""
check_absent apps/cli/src/server/router.rs "/api/settings"

cargo test -p deve_cli config -- --nocapture
cargo test -p deve_cli trusted_cli_untrusted -- --nocapture
cargo test -p deve_cli backend_capabilities_http -- --nocapture
cargo test -p deve_web settings -- --nocapture
cargo test -p deve_web ai_backend -- --nocapture
cargo test -p deve_web backend_for_send -- --nocapture

echo "settings-local-feedback-baseline-check: ok"
