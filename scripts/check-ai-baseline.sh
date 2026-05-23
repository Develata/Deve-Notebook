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
  MSYS2_ARG_CONV_EXCL="$pattern" rg -q --fixed-strings "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

check_absent() {
  local file="$1"
  local pattern="$2"
  if MSYS2_ARG_CONV_EXCL="$pattern" rg -q --fixed-strings "$pattern" "$ROOT_DIR/$file"; then
    fail "unexpected '$pattern' in $file"
  fi
}

# Command Palette exposes only the implemented AI entry point. Retry/backend/native-mode
# palette commands remain Planned / Optional until explicitly wired and accepted.
check_contains docs/plan/12_commands.md "Future UI Surface"
check_contains apps/web/src/components/command_palette/registry.rs "toggle_ai_chat"
check_absent apps/web/src/components/command_palette/registry.rs "Retry Last Request"
check_absent apps/web/src/components/command_palette/registry.rs "Switch Backend"
check_absent apps/web/src/components/command_palette/registry.rs "Switch to PLAN"
check_absent apps/web/src/components/command_palette/registry.rs "Switch to BUILD"

# AI-004/CMD-005: Slash commands are local Native PLAN / BUILD session-mode switches.
# They must not switch native/trusted-cli backend or send a plugin call by themselves.
check_contains docs/acceptance-cases/10_plugins.md "case_id: AI-004"
check_contains docs/acceptance-cases/11_commands_settings.md "case_id: CMD-005"
check_contains docs/acceptance-cases/11_commands_settings.md "ui_type: \"/plan\""
check_contains docs/acceptance-cases/11_commands_settings.md "ui_type: \"/build\""
check_contains docs/acceptance-cases/11_commands_settings.md "ui_type: \"/agents\""
check_contains docs/acceptance-cases/10_plugins.md "cargo test -p deve_web slash_commands_preserve_backend_mode -- --nocapture"
check_contains docs/plan/10_ai_agent.md "用作后端切换命令"
check_contains docs/plan/12_commands.md "\`/agents\`: 在原生 \`PLAN ↔ BUILD\` 之间顺序切换。"
check_contains docs/features/operations/ai_chat.md "不切换 backend，不发起 plugin call"
check_contains apps/web/src/components/chat/slash_commands.rs "\"/plan\" => Some(SlashCommand::Plan)"
check_contains apps/web/src/components/chat/slash_commands.rs "\"/build\" => Some(SlashCommand::Build)"
check_contains apps/web/src/components/chat/slash_commands.rs "\"/agents\" => Some(SlashCommand::Agents)"
check_contains apps/web/src/components/chat/slash_commands.rs "agents_toggles_only_native_session_modes"
check_contains apps/web/src/components/chat/slash_commands.rs "slash_commands_are_consumed_without_plugin_call"
check_contains apps/web/src/components/chat/slash_commands.rs "slash_commands_preserve_backend_mode"
check_contains apps/web/src/components/chat/slash_commands.rs "change_backend: false"
check_absent apps/web/src/components/chat/slash_commands.rs "agent-bridge"
check_absent apps/web/src/components/chat/slash_commands.rs "trusted-cli"
check_contains apps/web/src/api/ai_backend.rs "AI_BACKEND_NATIVE"
check_contains apps/web/src/api/ai_backend.rs "AI_BACKEND_TRUSTED_CLI"
check_contains apps/web/src/api/ai_backend.rs "ai_backend_to_plugin_id"
check_contains apps/web/src/components/chat/actions/send.rs "if let Some(command) = consume_slash_command(&msg"
check_contains apps/web/src/components/chat/actions/send_backend.rs "ai_backend_to_plugin_id"
check_contains apps/web/src/components/chat/actions/send.rs "build_chat_context"
check_contains apps/web/src/components/chat/actions/send.rs "bounded_chat_history"
check_contains plugins/ai-chat/main.rhai "system_prompt_base"
check_contains plugins/ai-chat/main.rhai "append_prior_history"

# AI-001: Native AI Chat must include current markdown context and finish the
# matching chat placeholder on text responses.
check_contains docs/acceptance-cases/10_plugins.md "case_id: AI-001"
check_contains docs/acceptance-cases/10_plugins.md "cargo test -p deve_core --test ai_chat_plugin_test -- --nocapture"
check_contains docs/acceptance-cases/10_plugins.md "cargo test -p deve_web chat_context -- --nocapture"
check_contains docs/acceptance-cases/10_plugins.md "cargo test -p deve_web plugin_text_response -- --nocapture"
check_contains crates/core/tests/ai_chat_plugin_test.rs "test_chat_with_api_key_reaches_stream_bridge"
check_contains apps/web/src/components/chat/actions/send/tests.rs "chat_context_includes_current_doc_markdown_selection_and_mode"
check_contains apps/web/src/components/chat/actions/send.rs "core_for_send.on_plugin_call"
check_contains apps/web/src/hooks/use_core/effects/message_dispatch_gate/tests.rs "plugin_text_response_finishes_matching_chat_placeholder"
check_contains apps/web/src/hooks/use_core/effects/message_dispatch_gate/tests.rs "plugin_text_response_does_not_duplicate_streamed_chat_content"
check_contains apps/web/src/components/chat/actions/send_backend/tests.rs "trusted_cli_untrusted_send_uses_native_plugin_and_visible_notice"
check_contains apps/web/src/components/chat/actions/apply.rs "chat_apply_append_markdown_op_uses_utf16_end_position"
check_contains apps/web/src/components/chat/actions/apply.rs "chat_apply_edit_message_carries_current_scope_nonce"
check_contains apps/web/src/components/chat/actions/apply.rs "apply_local_programmatic_op"
check_contains apps/web/src/components/chat/actions/apply.rs "sync_editor_state_to_rust"
check_contains apps/web/js/editor_adapter.js "window.syncEditorStateToRust = syncEditorStateToRust"
check_contains apps/web/src/components/chat/message_item.rs "chat_apply_label_is_build_only_for_assistant_messages"
check_contains apps/web/src/components/chat/message_list.rs "chat_apply_click_is_consumed_only_in_build_mode"
check_contains apps/web/src/components/chat/drop_handler.rs "file reader is unavailable"
check_contains apps/web/src/components/chat/drop_handler.rs "file read failed"
check_contains apps/web/src/components/chat/drop_handler.rs "attach_file_errors_are_visible_banner_copy"
check_absent apps/web/src/components/chat/drop_handler.rs "FileReader::new().unwrap()"
check_absent apps/web/src/components/chat/drop_handler.rs "let _ = reader.read_as_text(&file);"

# Native AI remains read-first. BUILD mode may expose controlled apply, not tools/shell/MCP.
check_contains docs/features/operations/ai_chat.md "Native AI 默认拒绝请求侧"
check_contains docs/features/operations/ai_chat.md "/api/ai/backend-capabilities"
check_contains docs/acceptance-cases/10_plugins.md "AI-007"
check_contains apps/web/src/hooks/use_core/effects/message_dispatch_runtime/mod.rs "finish_chat_request_from_plugin_response"
check_contains apps/cli/src/server/ai_chat/mod.rs "Native AI Chat tools are disabled by default"
check_contains apps/cli/src/server/ai_chat/mod.rs "native_ai_rejects_request_tools_before_provider_call"
check_contains apps/cli/src/server/ai_chat/stream.rs "Native AI Chat provider tool calls are disabled by default"
check_contains apps/cli/src/server/ai_chat/stream.rs "pub fn get_http_client() -> Result<&'static reqwest::Client>"
check_contains apps/cli/src/server/ai_chat/stream.rs "let client = get_http_client()?"
check_absent apps/cli/src/server/ai_chat/stream.rs "expect(\"Failed to create HTTP client\")"
check_contains apps/cli/src/server/ai_chat/stream/tests.rs "native_ai_http_client_creation_is_result_based"
check_contains apps/cli/src/server/ai_chat/stream/tests.rs "finalize_stream_response_rejects_provider_tool_calls"
check_contains apps/cli/src/server/ai_chat/stream/tests.rs "provider_tool_call_rejection_does_not_send_finish_chunk"
check_contains apps/cli/src/server/ai_chat/stream/tests.rs "provider_tool_call_delta_is_rejected_immediately"
check_contains apps/cli/src/server/ai_chat/stream/tests.rs "provider_tool_call_payload_is_rejected_before_content_chunk"

# Trusted CLI is default-off and policy-gated at both server and UI boundaries.
check_contains docs/plan/10_ai_agent.md "default-off、policy-gated 的 Trusted CLI path"
check_contains docs/plan/10_ai_agent.md "DEVE_AI_AGENT_BRIDGE_ENABLED"
check_contains docs/plan/10_ai_agent.md "DEVE_AI_AGENT_BRIDGE_TRUSTED"
check_contains docs/plan/14_tech_stack.md "Native Baseline"
check_contains docs/plan/14_tech_stack.md "Optional Trusted Path"
check_contains docs/plan/14_tech_stack.md "Compatibility Host + Interface Reserved"
check_contains docs/plan/plugins/agent_bridge/01_agent_bridge.md "enabled + trusted + explicit executable path"
check_contains docs/plan/plugins/agent_bridge/01_agent_bridge.md "MCP 不作为产品运行时方向"
check_contains apps/cli/src/server/agent_bridge/policy.rs "external agent disabled"
check_contains apps/cli/src/server/agent_bridge/policy.rs "trusted mode required"
check_contains apps/cli/src/server/agent_bridge/policy.rs "AGENT_CLI_PATH required"
check_contains apps/cli/src/server/agent_bridge/policy.rs "AGENT_CLI_PATH must be absolute"
check_contains apps/cli/src/server/agent_bridge/policy.rs "AGENT_CLI_PATH must point to an executable file"
check_contains apps/cli/src/server/agent_bridge/policy/tests.rs "trusted_cli_default_off_policy_fails_closed"
check_contains apps/cli/src/server/agent_bridge/policy/tests.rs "trusted_cli_untrusted_policy_falls_back_to_native_without_spawn_path"
check_contains apps/cli/src/server/ai_chat/mod.rs "Native AI Chat disabled by config"
check_contains apps/cli/src/server/handlers/plugin.rs "NATIVE_AI_DISABLED_ERROR"
check_contains apps/cli/src/server/handlers/plugin/tests.rs "native_ai_disabled_blocks_ai_chat_rpc_and_finishes_chat"
check_contains apps/web/src/api/ai_backend.rs "native_available"
check_contains apps/web/src/api/ai_backend/tests.rs "trusted_cli_default_off_capabilities_default_to_native_backend"
check_contains apps/web/src/hooks/use_ai_backend.rs "select_backend_fallback"
check_contains apps/web/src/hooks/use_ai_backend.rs "native_does_not_auto_promote_to_trusted_cli_when_native_is_disabled"
check_contains apps/cli/src/server/agent_bridge/policy/tests.rs "capabilities_do_not_promote_native_mode_to_trusted_cli_when_native_is_disabled"
check_contains apps/cli/src/server/agent_bridge/policy/tests.rs "capabilities_keep_requested_trusted_cli_reason_when_policy_blocks_it"
check_contains apps/web/src/i18n/extensions.rs "ai_backend_fallback"
check_contains apps/cli/src/server/agent_bridge/stream.rs "Agent CLI exited with status"
check_contains apps/cli/src/server/agent_bridge/stream.rs "Check AGENT_CLI_PATH points to an existing absolute executable path"
check_contains apps/cli/src/server/agent_bridge/AGENTS.md "default-off"
check_contains crates/core/src/plugin/runtime/module_resolver.rs "GuardedFileModuleResolver"
check_contains crates/core/src/plugin/runtime/module_resolver.rs "parent_traversal_import_is_rejected"
check_contains crates/core/src/plugin/runtime/module_resolver.rs "symlinked_module_escape_is_rejected"
check_contains crates/core/src/plugin/runtime/rhai_v1.rs "GuardedFileModuleResolver"
check_contains crates/core/src/plugin/runtime/chat_stream.rs "Native AI Chat currently rejects"

# PLUG-002: Calculation Runtime remains interface-only and visibly disabled.
check_contains docs/acceptance-cases/10_plugins.md "case_id: PLUG-002"
check_contains docs/acceptance-cases/10_plugins.md "text_visible \"Calculation Runtime\""
check_contains docs/plan/17_plugins.md "Calculation Runtime 仍然是长期能力，但本章**不要求代码实现**"
check_contains apps/web/src/components/sidebar/extensions.rs "calculation_runtime_title"
check_contains apps/web/src/components/sidebar/extensions.rs "code_execution_disabled"
check_contains apps/web/src/components/sidebar/extensions.rs "data-deve-extension-reserved"
check_contains apps/web/src/components/sidebar/extensions.rs "aria-disabled=\"true\""
check_contains apps/web/src/i18n/extensions.rs "Calculation Runtime"
check_contains apps/web/src/i18n/extensions.rs "default off"
check_contains apps/web/src/i18n/extensions.rs "Code execution disabled"

# PLUG-003: ledger-managed plugin boundaries remain future-hard constraints,
# and Rhai module imports stay sandboxed by guarded resolver tests.
check_contains docs/acceptance-cases/10_plugins.md "case_id: PLUG-003"
check_contains docs/acceptance-cases/10_plugins.md "cargo test -p deve_core plugin::runtime::module_resolver -- --nocapture"
check_contains docs/plan/17_plugins.md "<projection_base>/<repo_name>/**/*.md"
check_contains docs/plan/17_plugins.md ".notegit"
check_contains docs/plan/17_plugins.md "ledger-aware host functions"
check_contains crates/core/src/plugin/runtime/module_resolver.rs "fn parent_traversal_import_is_rejected()"
check_contains crates/core/src/plugin/runtime/module_resolver.rs "fn symlinked_module_escape_is_rejected()"

check_contains apps/web/src/components/settings_sections.rs "disabled=move || button_state.get().native_disabled"
check_contains apps/web/src/components/settings_sections.rs "disabled=move || button_state.get().trusted_disabled"
check_contains apps/web/src/components/settings_sections.rs "if !button_state.get_untracked().native_disabled"
check_contains apps/web/src/components/settings_sections.rs "if !button_state.get_untracked().trusted_disabled"
check_contains apps/web/src/components/settings_sections_policy.rs "ai_backend_button_state"
check_contains apps/web/src/components/settings_sections_policy.rs "native_disabled"
check_contains apps/web/src/components/settings_sections_policy.rs "trusted_disabled"
check_contains apps/web/src/components/settings_sections_policy/tests.rs "trusted_cli_default_off_keeps_native_visible_and_disables_trusted_backend"
check_contains apps/web/src/components/settings_sections_policy/tests.rs "ai_backend_buttons_disable_only_unavailable_backends"
check_contains apps/web/src/components/settings_sections_policy/tests.rs "ai_backend_buttons_show_disabled_reason_only_for_disabled_native"
check_contains apps/web/src/hooks/use_ai_backend.rs "backend: AI_BACKEND_NATIVE"

echo "ai-baseline-check: ok"
