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

- case_id: AI-009
  goal: AI Chat 消息体支持 TeX 展示且不误渲染代码块。
  preconditions:
    - 聊天面板可用
    - KaTeX 静态资源已加载
  steps:
    - ui_open: ai_chat
    - assistant_message: |
        inline $a^2+b^2=c^2$

        $$\int_0^1 x^2 dx$$

        ```text
        $not_math$
        ```
    - run: cargo test -p deve_web chat_math -- --nocapture
    - run: cargo test -p deve_web chat_message_ui_identity -- --nocapture
  assertions:
    - ui_assert: chat_inline_math_rendered true
    - ui_assert: chat_block_math_rendered true
    - ui_assert: chat_code_block_contains_literal "$not_math$"
    - ui_assert: chat_streaming_not_blocked_by_math_render_error true
    - ui_assert: chat_streaming_reuses_same_message_row true
    - ui_assert: identical_non_request_messages_render_as_distinct_rows true

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
    - run: cargo test -p deve_core skill_manager -- --nocapture
    - run: cargo test -p deve_core --lib skill_host_file_error -- --nocapture
    - run: cargo test -p deve_web message_dispatch_runtime -- --nocapture
  assertions:
    - server_assert: request_tools_rejected_before_provider_call true
    - server_assert: provider_tool_calls_rejected true
    - ui_assert: partial_stream_error_code_copy_visible true
    - ui_assert: chat_streaming_stopped_after_plugin_error true
    - log_not_contains_any: ["mcp", "skill", "spawn subprocess", "shell"]
    - server_assert: skill_lookup_rejects_path_traversal_and_symlink_escape true
    - server_assert: skill_file_failures_keep_fixed_public_category_and_typed_io_source true

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
    - http_assert: native_available_matches_config_and_runtime_registration true
    - http_assert: trusted_cli_available_matches_policy true
    - http_assert: effective_backend_in ["native", "trusted-cli", "none"]
    - ui_assert: unavailable_backend_disabled true
    - ui_assert: source_control_commit_ai_uses_backend_gate true
    - server_assert: native_disabled_blocks_ai_chat_rpc true
    - ui_assert: backend_fallback_reason_visible true
    - ui_assert: chat_streaming_stopped_after_plugin_error true

- case_id: AI-010
  goal: Native AI 作为编译期内建 runtime 注册，不依赖 Android/Desktop 运行目录中的插件文件。
  preconditions:
    - `ai.native_enabled = true`
    - 外部 plugins 目录不存在或为空
  steps:
    - run: cargo test -p deve_cli native_ai_builtin -- --nocapture
    - run: cargo test -p deve_cli serve_loader_registers_builtin_ai_without_external_plugins -- --nocapture
    - run: cargo test -p deve_cli serve_loader_rejects_explicit_external_ai_chat_duplicate -- --nocapture
    - run: cargo test -p deve_cli native_ai_disabled_omits_builtin_runtime -- --nocapture
    - run: cargo test -p deve_cli proxy_host_owns_builtin_ai_registration_for_its_lifetime -- --nocapture
    - run: cargo test -p deve_cli backend_capabilities_http -- --nocapture
    - http_get: "/api/ai/backend-capabilities"
    - server_call: "ai-chat.chat"
  assertions:
    - cli_assert: builtin_ai_registered_without_plugin_directory true
    - cli_assert: ordinary_serve_uses_shared_builtin_assembly true
    - cli_assert: proxy_host_retains_builtin_runtime_registration true
    - cli_assert: builtin_ai_assets_contain_no_secret true
    - cli_assert: duplicate_external_ai_plugin_fails_closed true
    - http_assert: native_available true
    - server_assert: ai_chat_plugin_not_found_absent true
    - server_assert: native_disabled_still_blocks_ai_chat_rpc true

- case_id: AI-011
  goal: Native AI 三种 provider protocol 各自使用精确 request 与 SSE adapter。
  preconditions:
    - Native AI runtime 已注册
    - provider HTTP fixture 可观测 request 且不记录 secret
  steps:
    - run: cargo test -p deve_cli ai_chat -- --nocapture
    - run: cargo test -p deve_core --test ai_chat_plugin_test -- --nocapture
    - run: scripts/check-ai-baseline.sh
  assertions:
    - server_assert: openai_chat_completions_request_and_stream_exact true
    - server_assert: openai_responses_request_and_stream_exact true
    - server_assert: anthropic_messages_request_and_stream_exact true
    - server_assert: provider_tool_or_refusal_events_fail_closed true
    - server_assert: truncated_provider_streams_fail_before_success_projection true
    - server_assert: done_without_protocol_terminal_fails_closed true
    - server_assert: external_rhai_plugin_has_no_native_ai_stream_authority true
    - server_assert: embedded_backend_reinitializes_same_stream_handler true
    - server_assert: rhai_receives_no_provider_secret_or_network_authority true
    - log_not_contains_any: ["fixture-secret", "Authorization: Bearer", "x-api-key"]

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
    - doc_read: "docs/plan/19_plugins.md"
    - run: cargo test -p deve_core plugin::runtime::module_resolver -- --nocapture
    - run: cargo test -p deve_core source_control_write_gate_missing_dependencies_fail_closed -- --nocapture
  assertions:
    - doc_contains: "<projection_base>/<workspace_segment>/**/*.md"
    - doc_contains: ".notegit"
    - doc_contains: "ledger-aware host functions"
    - plugin_assert: source_control_writer_gate_fail_closed_without_local_gate true
```
