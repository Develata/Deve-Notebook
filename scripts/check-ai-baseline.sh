#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "ai-baseline-check: $*" >&2
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

# Command Palette exposes only the implemented AI entry point. Retry/backend/native-mode
# palette commands remain Planned / Optional until explicitly wired and accepted.
check_contains docs/plan/12_commands.md "AI Retry / Backend / PLAN / BUILD 面板命令仍属于"
check_contains apps/web/src/components/command_palette/registry.rs "toggle_ai_chat"
check_absent apps/web/src/components/command_palette/registry.rs "Retry Last Request"
check_absent apps/web/src/components/command_palette/registry.rs "Switch Backend"
check_absent apps/web/src/components/command_palette/registry.rs "Switch to PLAN"
check_absent apps/web/src/components/command_palette/registry.rs "Switch to BUILD"

# Slash commands are local Native PLAN / BUILD session-mode switches. They must not
# switch native/trusted-cli backend or send a plugin call by themselves.
check_contains docs/plan/10_ai_agent.md "用作后端切换命令"
check_contains docs/features/operations/ai_chat.md "不切换 backend，不发起 plugin call"
check_contains apps/web/src/components/chat/slash_commands.rs "\"/plan\" => Some(SlashCommand::Plan)"
check_contains apps/web/src/components/chat/slash_commands.rs "\"/build\" => Some(SlashCommand::Build)"
check_contains apps/web/src/components/chat/slash_commands.rs "\"/agents\" => Some(SlashCommand::Agents)"
check_contains apps/web/src/components/chat/slash_commands.rs "agents_toggles_only_native_session_modes"
check_contains apps/web/src/components/chat/slash_commands.rs "slash_commands_are_consumed_without_plugin_call"
check_absent apps/web/src/components/chat/slash_commands.rs "agent-bridge"
check_absent apps/web/src/components/chat/slash_commands.rs "trusted-cli"
check_contains apps/web/src/components/chat/actions_send.rs "if let Some(command) = consume_slash_command(&msg"
check_contains apps/web/src/components/chat/actions_send.rs "core.on_plugin_call"

# Native AI remains read-first. BUILD mode may expose controlled apply, not tools/shell/MCP.
check_contains docs/features/operations/ai_chat.md "Native AI 默认拒绝请求侧"
check_contains apps/cli/src/server/ai_chat/mod.rs "Native AI Chat tools are disabled by default"
check_contains apps/cli/src/server/ai_chat/mod.rs "native_ai_rejects_request_tools_before_provider_call"
check_contains apps/cli/src/server/ai_chat/stream.rs "Native AI Chat provider tool calls are disabled by default"
check_contains apps/cli/src/server/ai_chat/stream.rs "finalize_stream_response_rejects_provider_tool_calls"

# Trusted CLI is default-off and policy-gated at both server and UI boundaries.
check_contains docs/plan/10_ai_agent.md "default-off、policy-gated 的 Trusted CLI path"
check_contains apps/cli/src/server/agent_bridge/policy.rs "external agent disabled"
check_contains apps/cli/src/server/agent_bridge/policy.rs "trusted mode required"
check_contains apps/cli/src/server/agent_bridge/policy.rs "AGENT_CLI_PATH required"
check_contains apps/cli/src/server/agent_bridge/policy.rs "AGENT_CLI_PATH must be absolute"
check_contains apps/web/src/components/settings_sections.rs "disabled=move || !trusted_available.get()"
check_contains apps/web/src/components/settings_sections.rs "if trusted_available.get_untracked()"
check_contains apps/web/src/components/ai_backend_guard.rs "chat.set_ai_mode.set(\"ai-chat\".to_string())"

echo "ai-baseline-check: ok"
