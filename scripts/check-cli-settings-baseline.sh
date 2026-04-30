#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# SET-007: server-backed Settings API remains a future boundary; current
# settings mutation is config.toml-only.

fail() {
  echo "cli-settings-baseline-check: $*" >&2
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

# CLI command surface.
check_contains apps/cli/src/main.rs "Init {"
check_contains apps/cli/src/main.rs "Scan,"
check_contains apps/cli/src/main.rs "Watch {"
check_contains apps/cli/src/main.rs "Serve {"
check_contains apps/cli/src/main.rs "Dump {"
check_contains apps/cli/src/main.rs "Export {"
check_contains apps/cli/src/main.rs "Recover {"
check_contains apps/cli/src/main.rs "Repair {"
check_contains apps/cli/src/main.rs "Config {"
check_contains apps/cli/src/dispatch.rs "ConfigAction::Print => commands::config::print(config)?"
check_contains apps/cli/src/dispatch.rs "ConfigAction::Set { key, value } => commands::config::set(&key, &value)?"

# config.toml remains the current authoritative runtime settings file.
check_contains apps/cli/src/commands/config.rs "const CONFIG_FILE: &str = \"config.toml\";"
check_contains apps/cli/src/commands/config.rs "toml::to_string_pretty(config)"
check_contains apps/cli/src/commands/config.rs "parse_whitelisted_value"
check_contains apps/cli/src/commands/config.rs "supported_config_keys_match_settings_plan_tables"
check_contains apps/cli/src/commands/init.rs "init_config_template_matches_current_settings_schema"
check_contains apps/cli/src/commands/init.rs "[ai.agent_bridge]"
check_contains apps/cli/src/commands/config.rs "\"ui.sidebar_width\""
check_contains apps/cli/src/commands/config.rs "\"ai.mode\""
check_contains apps/cli/src/commands/config.rs "Updated config.toml is not compatible with runtime config"
check_contains crates/core/src/config_test.rs "trusted_cli_requested_mode_is_preserved_when_agent_cli_path_is_missing"
check_contains crates/core/src/config_test.rs "trusted_cli_requested_mode_is_preserved_when_agent_cli_path_is_relative"
check_contains crates/core/src/config_test.rs "trusted_cli_requested_mode_is_preserved_when_agent_cli_path_is_not_executable"
check_contains crates/core/src/config_test.rs "trusted_cli_mode_is_kept_when_policy_conditions_are_satisfied"
check_contains crates/core/src/config_test.rs "trusted_cli_mode_honors_agent_bridge_env_aliases"
check_contains docs/plan/13_settings.md "DEVE_AI_AGENT_BRIDGE_ENABLED"
check_contains docs/plan/13_settings.md "DEVE_AI_AGENT_BRIDGE_TRUSTED"
check_contains docs/plan/13_settings.md '运行模式预设: `standard` (默认), `low-spec` (低配).'
check_absent docs/plan/13_settings.md '`debug` (调试)'
check_contains docs/plan/13_settings.md "server-backed Settings API 与统一 GUI"
check_contains docs/plan/13_settings.md "仍是 future work"
check_absent apps/cli/src/server/router.rs "/api/settings"

# UI command surfaces remain reachable through the shortcut layer.
check_contains apps/web/src/shortcuts/global.rs "Ctrl+P"
check_contains apps/web/src/shortcuts/global.rs "Ctrl+Shift+P"
check_contains apps/web/src/shortcuts/global.rs "Ctrl+Shift+K"
check_contains apps/web/src/shortcuts/global_handlers.rs "key == \"k\""
check_contains apps/web/src/shortcuts/global_handlers.rs "set_search_mode.set(\"@\".to_string())"
check_contains apps/web/src/components/command_palette/mod.rs "CommandPalette"
check_contains apps/web/src/components/branch_switcher/mod.rs "BranchSwitcher"
check_contains apps/web/src/i18n/sidebar.rs "Switch Branch (Ctrl+Shift+K)"
check_contains apps/web/src/components/settings.rs "current_boundary_desc"
check_contains apps/web/src/components/settings.rs "language_button_state"
check_contains apps/web/src/components/settings.rs "reserved_setting_state"
check_contains apps/web/src/components/settings.rs "data-deve-setting-disabled"
check_contains apps/web/src/i18n/settings.rs "deve config set"
check_contains apps/web/src/i18n/settings.rs "config.toml"
check_contains apps/web/src/i18n/settings.rs "Future setting: not available in the current release"
check_contains apps/web/src/i18n/settings.rs "reserved_setting_copy_marks_future_boundary"
check_absent apps/web/src/i18n/settings.rs "Coming in Phase 6"
check_contains apps/web/src/components/settings_sections_policy.rs "language_buttons_reflect_current_locale"
check_contains apps/web/src/components/settings_sections_policy.rs "reserved_setting_state_exposes_disabled_reason"
check_contains apps/web/src/components/settings_sections.rs "sync_mode_button_state"
check_contains apps/web/src/components/settings_sections_policy.rs "sync_mode_buttons_reflect_current_mode"
check_contains apps/web/src/components/settings_sections_policy.rs "sync_mode_buttons_treat_unknown_mode_as_auto_safe_default"

echo "cli-settings-baseline-check: ok"
