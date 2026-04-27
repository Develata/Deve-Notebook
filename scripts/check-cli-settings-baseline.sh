#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

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
check_contains apps/cli/src/commands/config.rs "\"ui.sidebar_width\""
check_contains apps/cli/src/commands/config.rs "\"ai.mode\""
check_contains apps/cli/src/commands/config.rs "Updated config.toml is not compatible with runtime config"

# UI command surfaces remain reachable through the shortcut layer.
check_contains apps/web/src/shortcuts/global.rs "Ctrl+P"
check_contains apps/web/src/shortcuts/global.rs "Ctrl+Shift+P"
check_contains apps/web/src/shortcuts/global.rs "Ctrl+Shift+K"
check_contains apps/web/src/shortcuts/global_handlers.rs "key == \"k\""
check_contains apps/web/src/shortcuts/global_handlers.rs "set_search_mode.set(\"@\".to_string())"
check_contains apps/web/src/components/command_palette/mod.rs "CommandPalette"
check_contains apps/web/src/components/branch_switcher/mod.rs "BranchSwitcher"
check_contains apps/web/src/i18n/sidebar.rs "Switch Branch (Ctrl+Shift+K)"

echo "cli-settings-baseline-check: ok"
