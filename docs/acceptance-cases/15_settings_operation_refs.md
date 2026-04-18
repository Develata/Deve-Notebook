## Settings Operation Cases

这些用例为 settings operation flows 提供更细的反链。

```markdown
- case_id: SET-003
  goal: 环境变量未设置时使用默认 profile。
  preconditions:
    - 未设置 DEVE_PROFILE
  steps:
    - run: deve serve --dry-run
  assertions:
    - log_contains: "Standard"

- case_id: SET-004
  goal: config.toml 文件配置在重启后生效。
  preconditions:
    - config.toml 可写
  steps:
    - run: deve init --path /tmp/deve-settings
    - edit_file: /tmp/deve-settings/config.toml
      set: "profile = \"low-spec\""
    - run: deve serve --dry-run
  assertions:
    - log_contains_any: ["LowSpec", "low-spec"]

- case_id: SET-005
  goal: Settings UI 偏好变更有即时反馈。
  preconditions:
    - 应用已运行
    - Settings 已打开
  steps:
    - ui_click: "中文"
    - ui_click: "Manual"
    - ui_click: "Native"
  assertions:
    - ui_assert: locale_eq "zh-CN"
    - ui_assert: sync_mode_eq "manual"
    - ui_assert: ai_backend_eq "native"

- case_id: SET-006
  goal: Settings 中不可用或预留能力必须显示明确反馈。
  preconditions:
    - Trusted CLI 条件未满足
    - Settings 已打开
  steps:
    - ui_query: "Trusted CLI"
    - ui_hover: "Trusted CLI"
  assertions:
    - ui_assert: setting_disabled true
    - ui_assert: disabled_reason_visible true
```
