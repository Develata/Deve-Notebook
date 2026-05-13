## AI Agent / Plugins & Runtime

```markdown
- case_id: AI-001
  goal: Native AI Chat 可读取当前 Markdown 并返回基础回答。
  preconditions:
    - 已打开一篇 Markdown 文档
    - Native AI Chat 已启用
  steps:
    - ui_open: ai_chat
    - ui_type: "Summarize this markdown file"
    - ui_submit: true
    - run: scripts/check-ai-baseline.sh
    - run: cargo test -p deve_core --test ai_chat_plugin_test -- --nocapture
    - run: cargo test -p deve_web chat_context -- --nocapture
    - run: cargo test -p deve_web plugin_text_response -- --nocapture
  assertions:
    - ui_assert: chat_response_visible true
    - ui_assert: chat_response_mentions_current_doc true
    - ui_assert: plugin_text_response_stops_loading true

- case_id: AI-002
  goal: `/plan` 进入原生 PLAN 模式，且 slash command 本身不会调用任何工具。
  preconditions:
    - 聊天面板可用
    - 当前文档已打开
  steps:
    - ui_type: "/plan"
    - ui_submit: true
    - ui_type: "How should we restructure this markdown?"
    - ui_submit: true
    - run: scripts/check-ai-baseline.sh
    - run: cargo test -p deve_web slash_commands -- --nocapture
    - run: cargo test -p deve_web chat_apply -- --nocapture
    - run: cargo test -p deve_cli ai_chat -- --nocapture
  assertions:
    - ui_assert: ai_mode_eq "plan"
    - ui_assert: plugin_call_not_sent_for_slash_command true
    - ui_assert: chat_apply_buttons_hidden true
    - server_assert: native_ai_rejects_tools_payload true
    - log_not_contains_any: ["tool call", "mcp", "skill", "spawn subprocess"]
    - ui_assert: markdown_unchanged true

- case_id: AI-003
  goal: `/build` 进入原生 BUILD 模式，并仅通过受控 Apply 修改当前 Markdown。
  preconditions:
    - 聊天面板可用
    - 当前 Markdown 文档可写
  steps:
    - ui_type: "/build"
    - ui_submit: true
    - assistant_message_contains_code_block: true
    - ui_click: "Apply"
    - run: scripts/check-ai-baseline.sh
    - run: cargo test -p deve_web chat_apply -- --nocapture
    - run: cargo test -p deve_cli ai_chat -- --nocapture
  assertions:
    - ui_assert: ai_mode_eq "build"
    - ui_assert: plugin_call_not_sent_for_slash_command true
    - ui_assert: chat_apply_buttons_visible_for_assistant_code_blocks true
    - ws_assert: ClientMessage.Edit_sent true
    - ws_assert: edit_scope_nonce_eq_current true
    - ui_assert: current_markdown_changed_by_controlled_apply true
    - server_assert: native_ai_rejects_tools_payload true
    - log_not_contains_any: ["mcp", "skill", "spawn subprocess"]

- case_id: AI-008
  goal: Native BUILD 程序执行边界默认 fail-closed，不等价于通用 tools/shell。
  preconditions:
    - Native AI Chat 后端可用
    - 请求或 provider 响应尝试携带 tool calls
  steps:
    - server_call: native_ai_chat_stream_with_tools
    - server_receive: provider_tool_call_delta
    - run: scripts/check-ai-baseline.sh
    - run: cargo test -p deve_cli ai_chat -- --nocapture
    - run: cargo test -p deve_web message_dispatch_runtime -- --nocapture
  assertions:
    - server_assert: request_tools_rejected_before_provider_call true
    - server_assert: provider_tool_calls_rejected true
    - ui_assert: partial_stream_error_detail_visible true
    - ui_assert: chat_streaming_stopped_after_plugin_error true
    - log_not_contains_any: ["mcp", "skill", "spawn subprocess", "shell"]

- case_id: AI-004
  goal: `/agents` 在原生 `PLAN ↔ BUILD` 间顺序切换。
  preconditions:
    - 聊天面板可用
  steps:
    - ui_type: "/plan"
    - ui_submit: true
    - ui_type: "/agents"
    - ui_submit: true
    - ui_type: "/agents"
    - ui_submit: true
    - run: scripts/check-ai-baseline.sh
    - run: cargo test -p deve_web slash_commands -- --nocapture
    - run: cargo test -p deve_web slash_commands_preserve_backend_mode -- --nocapture
  assertions:
    - ui_assert_sequence:
        - ai_mode_eq "plan"
        - ai_mode_eq "build"
        - ai_mode_eq "plan"
    - ui_assert: backend_mode_unchanged true
    - ui_assert: plugin_call_not_sent_for_slash_command true

- case_id: AI-005
  goal: Trusted External Agent 默认关闭。
  preconditions:
    - 未显式启用 trusted-cli
  steps:
    - ui_open: settings
    - run: scripts/check-ai-baseline.sh
    - run: cargo test -p deve_web trusted_cli_default_off -- --nocapture
    - run: cargo test -p deve_cli trusted_cli_default_off -- --nocapture
    - run: cargo test -p deve_cli agent_bridge -- --nocapture
  assertions:
    - ui_assert: ai_backend_option_visible "native"
    - ui_assert: ai_backend_option_disabled "trusted-cli"

- case_id: AI-006
  goal: 未满足 trusted 条件时不得启动外部 CLI。
  preconditions:
    - `AGENT_CLI_PATH` 已设置
    - `ai.agent_bridge.enabled = true`
    - `ai.agent_bridge.trusted = false`
  steps:
    - ui_set: "ai.mode" = "trusted-cli"
    - ui_type: "hello"
    - ui_submit: true
    - run: scripts/check-ai-baseline.sh
    - run: cargo test -p deve_web trusted_cli_untrusted -- --nocapture
    - run: cargo test -p deve_cli trusted_cli_untrusted -- --nocapture
    - run: cargo test -p deve_cli agent_bridge -- --nocapture
  assertions:
    - ui_assert: chat_error_visible true
    - ui_assert: ai_backend_eq "native"
    - ui_assert: chat_message_contains_trusted_cli_fallback_reason true
    - ui_assert: chat_streaming_stopped_after_plugin_error true
    - stdout_contains_any: ["trusted mode required", "external agent disabled"]
    - log_not_contains: "spawn subprocess"

- case_id: AI-007
  goal: AI backend 能力必须反映 `ai.mode` / `ai.native_enabled` 的有效运行时决策。
  preconditions:
    - `ai.native_enabled = false` 或 `ai.mode = trusted-cli` 但 trusted 条件不满足
  steps:
    - http_get: "/api/ai/backend-capabilities"
    - ui_open: settings
    - run: scripts/check-ai-baseline.sh
    - run: cargo test -p deve_core config -- --nocapture
    - run: cargo test -p deve_cli agent_bridge -- --nocapture
    - run: cargo test -p deve_cli backend_capabilities_http -- --nocapture
    - run: cargo test -p deve_cli native_ai_disabled_blocks_ai_chat_rpc -- --nocapture
    - run: cargo test -p deve_web ai_backend -- --nocapture
    - run: cargo test -p deve_web backend_for_send -- --nocapture
    - run: cargo test -p deve_web chat_send -- --nocapture
    - run: cargo test -p deve_web message_dispatch_runtime -- --nocapture
    - run: cargo test -p deve_web source_control_commit_ai -- --nocapture
  assertions:
    - http_assert: native_available_matches_config true
    - http_assert: trusted_cli_available_matches_policy true
    - http_assert: effective_backend_in ["native", "trusted-cli", "none"]
    - ui_assert: unavailable_backend_disabled true
    - ui_assert: source_control_commit_ai_uses_backend_gate true
    - server_assert: native_disabled_blocks_ai_chat_rpc true
    - ui_assert: backend_fallback_reason_visible true
    - ui_assert: chat_streaming_stopped_after_plugin_error true

- case_id: PLUG-001
  goal: Trusted External Agent 仅保留接口位，不要求当前 release 完整实现。
  preconditions:
    - 打开 Extensions / Settings
  steps:
    - ui_open: extensions
  assertions:
    - ui_assert: text_visible "Plugin Runtime"
    - ui_assert: text_visible "Trusted"
    - ui_assert: text_visible "default off"

- case_id: PLUG-002
  goal: Calculation Runtime 仅保留接口位，不要求当前 release 可执行代码。
  preconditions:
    - 打开 Extensions / Settings
  steps:
    - ui_open: extensions
  assertions:
    - ui_assert: text_visible "Calculation Runtime"
    - ui_assert: text_visible "planned"
    - ui_assert: code_execution_entry_hidden_or_disabled true

- case_id: PLUG-003
  goal: Ledger-Managed Boundary 仍然是未来插件运行时的硬约束。
  preconditions:
    - 存在未来 host api / plugin capability 文档
  steps:
    - doc_read: "docs/plan/17_plugins.md"
    - run: cargo test -p deve_core plugin::runtime::module_resolver -- --nocapture
  assertions:
    - doc_contains: "vault/<repo>/**/*.md"
    - doc_contains: ".notegit"
    - doc_contains: "ledger-aware host functions"
```
