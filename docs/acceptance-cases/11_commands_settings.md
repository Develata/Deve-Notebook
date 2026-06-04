## Commands 与 Settings

```markdown
- case_id: CMD-001
  goal: CLI 命令可执行。
  preconditions:
    - CLI 可用
  steps:
    - run: deve init --path ${DEVE_DATA_DIR} --repo default --projection-base ${DEVE_DATA_DIR}/notes
    - run: deve scan
    - run: deve watch --dry-run
    - run: deve serve --dry-run
    - run: deve dump --help
    - run: deve export --help
    - run: deve graph --help
    - run: deve verify-p2p --help
    - run: deve seed --help
    - run: deve node-check --help
    - run: deve sc-status --help
    - run: deve git status --help
    - run: deve git mirror --help
    - run: deve git export --help
    - run: deve git import --help
    - run: deve git push --help
    - run: scripts/check-cli-settings-baseline.sh
    - run: cargo test -p deve_cli main -- --nocapture
  assertions:
    - exit_code_all_eq: 0

- case_id: CMD-002
  goal: Command Palette 快捷键。
  preconditions:
    - 应用已运行
  steps:
    - ui_keypress: "Ctrl+Shift+P"
    - run: scripts/check-cli-settings-baseline.sh
  assertions:
    - ui_assert: command_palette_visible true

- case_id: CMD-003
  goal: Quick Open 快捷键。
  preconditions:
    - 应用已运行
  steps:
    - ui_keypress: "Ctrl+P"
    - run: scripts/check-cli-settings-baseline.sh
  assertions:
    - ui_assert: unified_search_visible true
    - ui_assert: mode_eq "file"

- case_id: CMD-004
  goal: Branch Switcher 快捷键。
  preconditions:
    - 应用已运行
  steps:
    - ui_keypress: "Ctrl+Shift+K"
    - run: scripts/check-cli-settings-baseline.sh
  assertions:
    - ui_assert: branch_switcher_visible true

- case_id: CMD-004D
  goal: Settings 全局快捷键。
  preconditions:
    - 应用已运行
  steps:
    - ui_keypress: "Ctrl+L"
    - ui_keypress: "Ctrl+Shift+O"
    - ui_keypress: "Ctrl+B"
    - run: scripts/check-cli-settings-baseline.sh
    - run: cargo test -p deve_web global_shortcut -- --nocapture
    - run: cargo test -p deve_web static_commands_include_sidebar_toggle -- --nocapture
  assertions:
    - ui_assert: locale_toggled true
    - ui_assert: outline_visibility_toggled true
    - ui_assert: sidebar_visibility_toggled true

- case_id: CMD-004A
  goal: 未绑定后端的 P2P branch 创建入口必须显示 unavailable 状态。
  preconditions:
    - Command Palette 可用
  steps:
    - ui_keypress: "Ctrl+Shift+P"
    - ui_command: "P2P: Establish Branch"
    - run: scripts/check-source-control-baseline.sh
    - run: cargo test -p deve_web establish_branch_command -- --nocapture
  assertions:
    - ui_assert: command_unavailable "establish_branch"
    - ui_assert: source_control_notice_eq "establish-branch-unavailable"

- case_id: CMD-004B
  goal: Git mirror Command Palette 入口只能作为 CLI-only notice，不得执行 Web Git writer。
  preconditions:
    - Command Palette 可用
  steps:
    - ui_keypress: "Ctrl+Shift+P"
    - ui_command: "Git: Status"
    - ui_command: "Git: Mirror"
    - ui_command: "Git: Export Mirror"
    - run: scripts/check-source-control-baseline.sh
    - run: cargo test -p deve_web git_status_command -- --nocapture
    - run: cargo test -p deve_web git_mirror_command -- --nocapture
    - run: cargo test -p deve_web git_export_command -- --nocapture
  assertions:
    - ui_assert: command_unavailable "git_status"
    - ui_assert: command_unavailable "git_mirror"
    - ui_assert: command_unavailable "git_export_mirror"
    - ui_assert: source_control_notice_eq "git-*-cli-only"
    - ui_assert: web_git_writer_absent true

- case_id: CMD-004C
  goal: Source Control 与 AI 的未接线命令入口必须明确显示 unavailable 状态。
  preconditions:
    - Command Palette 可用
  steps:
    - ui_keypress: "Ctrl+Shift+P"
    - ui_command: "Source Control: Sync"
    - ui_command: "Source Control: Commit"
    - ui_command: "Source Control: Push"
    - ui_command: "AI: Switch to PLAN Mode"
    - ui_command: "AI: Switch to BUILD Mode"
    - run: scripts/check-cli-settings-baseline.sh
    - run: cargo test -p deve_web reserved_commands -- --nocapture
    - run: cargo test -p deve_web static_commands_partition_reserved_surfaces -- --nocapture
  assertions:
    - ui_assert: command_unavailable "source_control_sync"
    - ui_assert: command_unavailable "source_control_commit"
    - ui_assert: command_unavailable "source_control_push"
    - ui_assert: command_unavailable "ai_switch_plan"
    - ui_assert: command_unavailable "ai_switch_build"

- case_id: CMD-005
  goal: AI 模式与斜杠命令。
  preconditions:
    - 聊天面板可用
  steps:
    - ui_type: "/plan"
    - ui_submit: true
    - ui_assert: ai_mode_eq "plan"
    - ui_type: "/build"
    - ui_submit: true
    - ui_assert: ai_mode_eq "build"
    - ui_type: "/agents"
    - ui_submit: true
    - run: scripts/check-cli-settings-baseline.sh
  assertions:
    - ui_assert: ai_mode_eq "plan"

- case_id: CMD-006
  goal: CLI Projection Workspace lifecycle commands expose safe parse surfaces.
  preconditions:
    - CLI 可用
  steps:
    - run: deve init --help
    - run: deve repo projection set --help
    - run: deve repo projection list --help
    - run: deve repo projection check --help
    - run: deve scan --help
    - run: deve watch --dry-run
    - run: scripts/check-cli-settings-baseline.sh
    - run: cargo test -p deve_cli projection_locator -- --nocapture
  assertions:
    - exit_code_all_eq: 0

- case_id: CMD-007
  goal: CLI server runtime options are explicit.
  preconditions:
    - CLI 可用
  steps:
    - run: deve serve --help
    - run: deve serve --dev --dry-run --port 3001
    - run: scripts/check-cli-settings-baseline.sh
    - run: cargo test -p deve_cli commands::serve::tests -- --nocapture
  assertions:
    - exit_code_all_eq: 0

- case_id: CMD-007A
  goal: Embedded browser runtime is available from the CLI server.
  preconditions:
    - 已运行 `scripts/smoke-web-release-build.sh`
    - 已重新构建 CLI，使 `apps/web/dist` 被编译进二进制
    - 后端通过 `deve serve --dev --port 3001` 运行，且未设置 `DEVE_STATIC_DIR`
  steps:
    - run: scripts/smoke-web-runtime-paths.sh
    - browser_open: "http://127.0.0.1:3001/"
  assertions:
    - ui_contains_any: ["Ready", "Login"]
    - network_contains: "/api/auth/status"
    - network_not_contains_status: ["/api/auth/me", 401]
    - network_contains: "/api/node/role"

- case_id: CMD-007B
  goal: Trunk browser dev runtime fallback is explicit.
  preconditions:
    - 后端通过 `deve serve --dev --port 3001` 运行
    - 前端通过 `NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080` 从 `apps/web` 运行
  steps:
    - run: scripts/smoke-web-runtime-paths.sh
    - browser_open: "http://127.0.0.1:8080/"
  assertions:
    - ui_contains_any: ["Ready", "Login"]
    - network_contains: "/api/node/role"

- case_id: CMD-008
  goal: CLI export and dump inspection options are discoverable.
  preconditions:
    - CLI 可用
  steps:
    - run: deve dump --help
    - run: deve export --help
    - run: deve export --format markdown --allow-degraded-projection --help
    - run: scripts/check-cli-settings-baseline.sh
    - run: cargo test -p deve_cli export -- --nocapture
  assertions:
    - exit_code_all_eq: 0

- case_id: CMD-009
  goal: CLI repair and admin commands are discoverable.
  preconditions:
    - CLI 可用
  steps:
    - run: deve verify-p2p --help
    - run: deve seed --help
    - run: deve node-check --help
    - run: deve node-check --projection --help
    - run: deve recover --help
    - run: deve repair --help
    - run: deve repair --check --help
    - run: scripts/check-dev-data-health-baseline.sh
    - run: scripts/check-cli-settings-baseline.sh
    - run: cargo test -p deve_cli node_check -- --nocapture
    - run: cargo test -p deve_cli repair_check -- --nocapture
  assertions:
    - exit_code_all_eq: 0

- case_id: SET-001
  goal: 环境变量默认值。
  preconditions:
    - 未设置 DEVE_PROFILE
  steps:
    - run: deve config print
    - run: scripts/check-settings-local-feedback-baseline.sh
    - run: scripts/check-cli-settings-baseline.sh
    - run: cargo test -p deve_cli config -- --nocapture
  assertions:
    - stdout_contains: 'profile = "standard"'

- case_id: SET-002
  goal: config.toml 配置可由 CLI 更新。
  preconditions:
    - config.toml 可写
  steps:
    - run: deve config set ui.sidebar_width 300
    - run: scripts/check-settings-local-feedback-baseline.sh
    - run: scripts/check-cli-settings-baseline.sh
    - run: cargo test -p deve_cli config -- --nocapture
  assertions:
    - file_contains: config.toml "sidebar_width = 300"

- case_id: SET-007
  goal: Server-backed Settings API 仍按 future 边界处理。
  preconditions:
    - docs/plan/15_settings.md 仍标记 Settings API 为 future work
  steps:
    - run: scripts/check-settings-local-feedback-baseline.sh
    - run: scripts/check-cli-settings-baseline.sh
    - run: cargo test -p deve_cli config -- --nocapture
  assertions:
    - route_absent: "/api/settings"
    - unsupported_key_rejected: "server.settings.api_enabled"
```
