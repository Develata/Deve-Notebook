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
  assertions:
    - ui_assert: chat_response_visible true
    - ui_assert: chat_response_mentions_current_doc true

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
  assertions:
    - ui_assert: ai_mode_eq "plan"
    - ui_assert: plugin_call_not_sent_for_slash_command true
    - ui_assert: chat_apply_buttons_hidden true
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
  assertions:
    - ui_assert: ai_mode_eq "build"
    - ui_assert: plugin_call_not_sent_for_slash_command true
    - ui_assert: chat_apply_buttons_visible_for_assistant_code_blocks true
    - ws_assert: ClientMessage.Edit_sent true
    - ws_assert: edit_scope_nonce_eq_current true
    - ui_assert: current_markdown_changed_by_controlled_apply true
    - log_not_contains_any: ["mcp", "skill", "spawn subprocess"]

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
  assertions:
    - ui_assert: chat_error_visible true
    - ui_assert: ai_backend_eq "native"
    - ui_assert: chat_message_contains_trusted_cli_fallback_reason true
    - stdout_contains_any: ["trusted mode required", "external agent disabled"]
    - log_not_contains: "spawn subprocess"

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
  assertions:
    - doc_contains: "vault/<repo>/**/*.md"
    - doc_contains: ".notegit"
    - doc_contains: "ledger-aware host functions"
```
