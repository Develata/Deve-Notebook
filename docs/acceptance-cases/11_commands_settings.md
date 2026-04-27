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
    - ui_assert: quick_open_visible true

- case_id: CMD-004
  goal: Branch Switcher 快捷键。
  preconditions:
    - 应用已运行
  steps:
    - ui_keypress: "Ctrl+Shift+K"
    - run: scripts/check-cli-settings-baseline.sh
  assertions:
    - ui_assert: branch_switcher_visible true

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
  goal: CLI vault lifecycle commands expose safe parse surfaces.
  preconditions:
    - CLI 可用
  steps:
    - run: deve init --help
    - run: deve scan --help
    - run: deve watch --dry-run
    - run: scripts/check-cli-settings-baseline.sh
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
    - 已运行 `NO_COLOR=true trunk build --release`
    - 已重新构建 CLI，使 `apps/web/dist` 被编译进二进制
    - 后端通过 `deve serve --dev --port 3001` 运行，且未设置 `DEVE_STATIC_DIR`
  steps:
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
    - run: scripts/check-dev-data-health-baseline.sh
    - run: scripts/check-cli-settings-baseline.sh
    - run: cargo test -p deve_cli node_check -- --nocapture
  assertions:
    - exit_code_all_eq: 0

- case_id: SET-001
  goal: 环境变量默认值。
  preconditions:
    - 未设置 DEVE_PROFILE
  steps:
    - run: deve config print
    - run: scripts/check-cli-settings-baseline.sh
    - run: cargo test -p deve_cli config -- --nocapture
  assertions:
    - stdout_contains: "profile = 'standard'"

- case_id: SET-002
  goal: config.toml 配置可由 CLI 更新。
  preconditions:
    - config.toml 可写
  steps:
    - run: deve config set ui.sidebar_width 300
    - run: scripts/check-cli-settings-baseline.sh
    - run: cargo test -p deve_cli config -- --nocapture
  assertions:
    - file_contains: config.toml "sidebar_width = 300"

- case_id: SET-007
  goal: Server-backed Settings API 仍按 future 边界处理。
  preconditions:
    - docs/plan/13_settings.md 仍标记 Settings API 为 future work
  steps:
    - run: scripts/check-cli-settings-baseline.sh
    - run: cargo test -p deve_cli config -- --nocapture
  assertions:
    - route_absent: "/api/settings"
    - unsupported_key_rejected: "server.settings.api_enabled"
```
