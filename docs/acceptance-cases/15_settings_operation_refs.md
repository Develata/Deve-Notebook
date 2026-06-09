## Settings Operation Cases

这些用例为 settings operation flows 提供更细的反链。

```markdown
- case_id: SET-003
  goal: `ai.mode=trusted-cli` 条件不满足时 effective backend 回退到 `native`，但保留用户请求配置。
  preconditions:
    - config.toml 中设置 `ai.mode = "trusted-cli"`
    - `ai.agent_bridge.enabled = true`
    - `ai.agent_bridge.trusted = false` 或未设置绝对路径 `AGENT_CLI_PATH`
  steps:
    - run: deve config print
    - http_get: "/api/ai/backend-capabilities"
    - run: scripts/check-settings-local-feedback-baseline.sh
  assertions:
    - stdout_contains: 'mode = "trusted-cli"'
    - http_assert: effective_backend_eq "native"
    - http_assert: effective_backend_reason_visible true

- case_id: SET-004
  goal: config.toml 文件配置在重启后生效。
  preconditions:
    - config.toml 可写
  steps:
    - run: deve init --path /tmp/deve-settings
    - edit_file: /tmp/deve-settings/config.toml
      set: "profile = \"low-spec\""
    - run: cd /tmp/deve-settings && deve serve --dry-run
    - run: scripts/check-settings-local-feedback-baseline.sh
  assertions:
    - log_contains_any: ["LowSpec", "low-spec"]

- case_id: SET-005
  goal: Settings UI 偏好变更有即时反馈。
  preconditions:
    - 应用已运行
    - Settings 已打开
  steps:
    - ui_click: "中文"
    - ui_click: "Dark"
    - ui_click: "Off"
    - ui_click: "Compact"
    - ui_click: "Manual"
    - ui_click: "Native"
    - ui_click: "Hide AI Chat"
    - ui_click: "Show AI Chat"
    - ui_type_number_draft: "max_document_tabs" (value: 12)
    - ui_assert: max_document_tabs_pref_unchanged_before_submit true
    - ui_submit_number: "max_document_tabs" (value: 8)
    - manual_chrome: docs/dev-runbook.md#settings--command-ui-smoke
    - run: scripts/check-settings-local-feedback-baseline.sh
  assertions:
    - ui_assert: locale_eq "zh-CN"
    - ui_assert: theme_pref_eq "dark"
    - ui_assert: editor_wrap_eq "off"
    - ui_assert: editor_density_eq "compact"
    - ui_assert: sync_mode_eq "manual"
    - ui_assert: ai_backend_eq "native"
    - ui_assert: ai_chat_panel_visible true
    - ui_assert: max_document_tabs_eq 8
    - ui_assert: max_document_tabs_pref_applies_after_submit true
    - ui_assert: settings_primary_close_absent true
    - ui_assert: settings_icon_close_visible true
    - ui_assert: ai_chat_divider_hidden_when_setting_hidden true
    - ui_assert: display_editor_visible_when_ai_chat_setting_hidden true
    - ui_assert: hidden_ai_chat_preserves_layout_width_pref true

- case_id: SET-006
  goal: Settings 中不可用或预留能力必须显示明确、可访问的反馈。
  preconditions:
    - Trusted CLI 条件未满足
    - Settings 已打开
  steps:
    - ui_query: "Trusted CLI"
    - ui_hover: "Trusted CLI"
    - ui_query: "Hybrid Editing"
    - manual_chrome: docs/dev-runbook.md#settings--command-ui-smoke
    - run: cargo test -p deve_web settings -- --nocapture
    - run: scripts/check-settings-local-feedback-baseline.sh
    - run: scripts/check-cli-settings-baseline.sh
  assertions:
    - ui_assert: setting_disabled true
    - ui_assert: disabled_reason_visible true
    - ui_assert: reserved_setting_has_marker "data-deve-setting-disabled"
    - ui_assert: reserved_setting_has_aria_disabled true
    - ui_assert: reserved_setting_copy_contains "current release"
```
