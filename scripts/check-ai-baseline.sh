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
check_contains docs/plan/12_commands.md "PLAN / BUILD 面板命令仍属于"
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
check_contains apps/web/src/api/ai_backend.rs "AI_BACKEND_NATIVE"
check_contains apps/web/src/api/ai_backend.rs "AI_BACKEND_TRUSTED_CLI"
check_contains apps/web/src/api/ai_backend.rs "ai_backend_to_plugin_id"
check_contains apps/web/src/components/chat/actions_send.rs "if let Some(command) = consume_slash_command(&msg"
check_contains apps/web/src/components/chat/actions_send.rs "ai_backend_to_plugin_id"
check_contains apps/web/src/components/chat/actions_send.rs "bounded_chat_history"
check_contains plugins/ai-chat/main.rhai "append_prior_history"
check_contains apps/web/src/components/chat/actions_send.rs "core.on_plugin_call"
check_contains apps/web/src/components/chat/actions_apply.rs "append_markdown_op_uses_utf16_end_position"
check_contains apps/web/src/components/chat/actions_apply.rs "apply_edit_message_carries_current_scope_nonce"
check_contains apps/web/src/components/chat/message_item.rs "apply_label_is_build_only_for_assistant_messages"
check_contains apps/web/src/components/chat/message_list.rs "apply_click_is_consumed_only_in_build_mode"

# Native AI remains read-first. BUILD mode may expose controlled apply, not tools/shell/MCP.
check_contains docs/features/operations/ai_chat.md "Native AI 默认拒绝请求侧"
check_contains docs/features/operations/ai_chat.md "/api/ai/backend-capabilities"
check_contains docs/acceptance-cases/10_plugins.md "AI-007"
check_contains apps/web/src/hooks/use_core/effects/message_dispatch_runtime.rs "finish_chat_request_from_plugin_response"
check_contains apps/web/src/hooks/use_core/effects/message_dispatch_gate_test.rs "plugin_text_response_finishes_matching_chat_placeholder"
check_contains apps/web/src/hooks/use_core/effects/message_dispatch_gate_test.rs "plugin_text_response_does_not_duplicate_streamed_chat_content"
check_contains apps/cli/src/server/ai_chat/mod.rs "Native AI Chat tools are disabled by default"
check_contains apps/cli/src/server/ai_chat/mod.rs "native_ai_rejects_request_tools_before_provider_call"
check_contains apps/cli/src/server/ai_chat/stream.rs "Native AI Chat provider tool calls are disabled by default"
check_contains apps/cli/src/server/ai_chat/stream.rs "finalize_stream_response_rejects_provider_tool_calls"
check_contains apps/cli/src/server/ai_chat/stream.rs "provider_tool_call_rejection_does_not_send_finish_chunk"

# Trusted CLI is default-off and policy-gated at both server and UI boundaries.
check_contains docs/plan/10_ai_agent.md "default-off、policy-gated 的 Trusted CLI path"
check_contains docs/plan/10_ai_agent.md "DEVE_AI_AGENT_BRIDGE_ENABLED"
check_contains docs/plan/10_ai_agent.md "DEVE_AI_AGENT_BRIDGE_TRUSTED"
check_contains docs/plan/plugins/agent_bridge/01_agent_bridge.md "DEVE_AI_AGENT_BRIDGE_ENABLED"
check_contains docs/plan/plugins/agent_bridge/01_agent_bridge.md "DEVE_AI_AGENT_BRIDGE_TRUSTED"
check_contains apps/cli/src/server/agent_bridge/policy.rs "external agent disabled"
check_contains apps/cli/src/server/agent_bridge/policy.rs "trusted mode required"
check_contains apps/cli/src/server/agent_bridge/policy.rs "AGENT_CLI_PATH required"
check_contains apps/cli/src/server/agent_bridge/policy.rs "AGENT_CLI_PATH must be absolute"
check_contains apps/cli/src/server/agent_bridge/policy.rs "AGENT_CLI_PATH must point to an executable file"
check_contains apps/cli/src/server/ai_chat/mod.rs "Native AI Chat disabled by config"
check_contains apps/cli/src/server/handlers/plugin.rs "NATIVE_AI_DISABLED_ERROR"
check_contains apps/cli/src/server/handlers/plugin_test.rs "native_ai_disabled_blocks_ai_chat_rpc_and_finishes_chat"
check_contains apps/web/src/api/ai_backend.rs "native_available"
check_contains apps/web/src/components/ai_backend_guard.rs "select_backend_fallback"
check_contains apps/web/src/components/ai_backend_guard.rs "native_does_not_auto_promote_to_trusted_cli_when_native_is_disabled"
check_contains apps/cli/src/server/agent_bridge/policy_test.rs "capabilities_do_not_promote_native_mode_to_trusted_cli_when_native_is_disabled"
check_contains apps/cli/src/server/agent_bridge/policy_test.rs "capabilities_keep_requested_trusted_cli_reason_when_policy_blocks_it"
check_contains apps/web/src/i18n/extensions.rs "ai_backend_fallback"
check_contains apps/cli/src/server/agent_bridge/stream.rs "Agent CLI exited with status"
check_contains apps/cli/src/server/agent_bridge/stream.rs "Check AGENT_CLI_PATH points to an existing absolute executable path"
check_contains apps/cli/src/server/agent_bridge/AGENTS.md "default-off"
check_contains crates/core/src/plugin/runtime/chat_stream.rs "Native AI Chat currently rejects"
check_contains apps/web/src/components/settings_sections.rs "disabled=move || !native_available.get()"
check_contains apps/web/src/components/settings_sections.rs "disabled=move || !trusted_available.get()"
check_contains apps/web/src/components/settings_sections.rs "if trusted_available.get_untracked()"
check_contains apps/web/src/components/settings_sections.rs "ai_backend_button_state"
check_contains apps/web/src/components/settings_sections.rs "ai_backend_buttons_disable_only_unavailable_backends"
check_contains apps/web/src/components/settings_sections.rs "ai_backend_buttons_show_disabled_reason_only_for_disabled_native"
check_contains apps/web/src/components/ai_backend_guard.rs "backend: AI_BACKEND_NATIVE"

echo "ai-baseline-check: ok"
