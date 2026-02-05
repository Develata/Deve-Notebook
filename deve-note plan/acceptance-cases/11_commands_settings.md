## Commands 与 Settings

```markdown
- case_id: CMD-001
  goal: CLI 命令可执行。
  preconditions:
    - CLI 可用
  steps:
    - run: deve init
    - run: deve scan
    - run: deve watch --dry-run
    - run: deve serve --dry-run
    - run: deve dump --help
    - run: deve export --help
    - run: deve verify-p2p --help
    - run: deve seed --help
  assertions:
    - exit_code_all_eq: 0

- case_id: CMD-002
  goal: Command Palette 快捷键。
  preconditions:
    - 应用已运行
  steps:
    - ui_keypress: "Ctrl+Shift+P"
  assertions:
    - ui_assert: command_palette_visible true

- case_id: CMD-003
  goal: Quick Open 快捷键。
  preconditions:
    - 应用已运行
  steps:
    - ui_keypress: "Ctrl+P"
  assertions:
    - ui_assert: quick_open_visible true

- case_id: CMD-004
  goal: Branch Switcher 快捷键。
  preconditions:
    - 应用已运行
  steps:
    - ui_keypress: "Ctrl+Shift+K"
  assertions:
    - ui_assert: branch_switcher_visible true

- case_id: CMD-005
  goal: AI 模式与斜杠命令。
  preconditions:
    - 聊天面板可用
  steps:
    - ui_type: "/plan"
    - ui_submit: true
    - ui_type: "/build"
    - ui_submit: true
  assertions:
    - ui_assert: ai_mode_eq "plan"
    - ui_assert: ai_mode_eq "build"

- case_id: SET-001
  goal: 环境变量默认值。
  preconditions:
    - 未设置 DEVE_PROFILE
  steps:
    - run: deve config print
  assertions:
    - stdout_contains: "profile = standard"

- case_id: SET-002
  goal: settings.toml 配置生效。
  preconditions:
    - settings.toml 可写
  steps:
    - run: deve config set ui.sidebar_width 300
    - run: deve restart
  assertions:
    - ui_assert: sidebar_width_eq 300
```
