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
    - run: deve ngit status --help
    - run: deve ngit mirror --help
    - run: deve ngit export --help
    - run: deve ngit import --help
    - run: deve ngit push --help
    - run: deve projection-remote webdav push --help
    - run: deve projection-remote webdav pull --help
    - run: deve projection-remote s3 push --help
    - run: deve projection-remote s3 pull --help
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
  goal: ngit / remote projection Command Palette 入口不得执行前端 writer。
  preconditions:
    - Command Palette 可用
  steps:
    - ui_keypress: "Ctrl+Shift+P"
    - ui_command: "ngit:status"
    - ui_command: "ngit:mirror"
    - ui_command: "ngit:export"
    - ui_command: "webdav:pull"
    - ui_command: "s3:pull"
    - cli_projection_remote_s3_pull_provider_io_ready: false when current repo_url is not an S3 locator
    - run: scripts/check-source-control-baseline.sh
    - run: cargo test -p deve_web ngit_commands -- --nocapture
    - run: cargo test -p deve_web remote_projection_commands -- --nocapture
  assertions:
    - ui_assert: command_available "ngit_status"
    - ui_assert: command_available "ngit_mirror"
    - ui_assert: command_available "ngit_export_mirror"
    - ui_assert: command_available "webdav_pull"
    - ui_assert: command_available "s3_pull"
    - ui_assert: web_git_writer_absent true
    - ui_assert: web_remote_projection_io_absent true
    - cli_assert: s3_pull_missing_transport_url_fails_closed true

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
    - 当前数据根已通过 `deve init --path <data-root> --repo default --projection-base <projection-base>` 或等价流程具备 Projection Locator
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
    - 当前 smoke 数据根已通过 `deve init --path <data-root> --repo default --projection-base <projection-base>` 初始化 Projection Locator
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
    - 当前 smoke 数据根已通过 `deve init --path <data-root> --repo default --projection-base <projection-base>` 初始化 Projection Locator
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
    - run: cargo test -p deve_core source_control_ngit_only -- --nocapture
    - run: cargo test -p deve_cli set_rejects_empty_env_reference_without_rewriting_config -- --nocapture
    - run: cargo test -p deve_cli set_rejects_zero_p2p_connect_interval_without_rewriting_config -- --nocapture
    - run: cargo test -p deve_cli set_rejects_existing_invalid_runtime_config_without_rewriting -- --nocapture
    - run: cargo test -p deve_cli config -- --nocapture
  assertions:
    - file_contains: config.toml "sidebar_width = 300"
    - unsupported_key_rejected: "source_control.git_bridge"
    - file_not_contains: .env.example "DEVE_SOURCE_CONTROL__GIT_BRIDGE"
    - config_assert: empty_env_reference_rejected_without_rewrite true
    - config_assert: zero_p2p_connect_interval_rejected_without_rewrite true
    - config_assert: existing_invalid_runtime_config_rejected_without_rewrite true

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

- case_id: SET-007A
  goal: Native backend preference 是 host-local shell 配置，不是 browser 或 server-backed Settings。
  preconditions:
    - Settings 已打开
    - 普通浏览器与 native shell 都可执行 Settings smoke
  steps:
    - run: scripts/check-settings-local-feedback-baseline.sh
    - run: cargo test -p deve_core native_adapter -- --nocapture
    - run: cargo test -p deve_web native_backend -- --nocapture
    - run: cargo test -p deve_desktop --features native-packaging native_backend -- --nocapture
    - run: cargo test -p deve_mobile --features native-packaging native_backend -- --nocapture
    - browser_open: "Settings"
    - ui_assert: native_backend_section_unavailable_in_browser true
    - native_validate_remote_backend: "https://example.invalid"
    - desktop_native_menu_switch_backend: "local"
  assertions:
    - route_absent: "/api/settings/native-backend"
    - browser_storage_absent: "deve.native.backend"
    - config_assert: native_backend_preference_not_written_to_config_toml true
    - native_assert: remote_backend_save_requires_node_role_probe true
    - native_assert: desktop_use_local_backend_saves_host_local_preference true
    - browser_assert: remote_browser_exposes_no_backend_preference_ipc true
    - gap_assert: mobile_use_local_backend_native_control_required true

- case_id: SET-008
  goal: 静态 P2P peer 配置必须把 peer_id 表达为 expected authenticated identity，而不是显示 label。
  preconditions:
    - FullPeer Mesh v1 使用静态 peer 配置
  steps:
    - run: scripts/check-cli-settings-baseline.sh
    - run: cargo test -p deve_core p2p_mesh_env_aliases_load_static_peer_config -- --nocapture
    - run: cargo test -p deve_core load_checked_fails_closed_on_zero_p2p_connect_interval_ms -- --nocapture
    - run: cargo test -p deve_core --lib load_checked_rejects_sparse_p2p_peer_env_alias_indices -- --nocapture
    - run: cargo test -p deve_cli init_config_template_matches_current_settings_schema -- --nocapture
  assertions:
    - config_example_peer_id_placeholder_not_label: true
    - init_template_peer_id_placeholder_not_label: true
    - p2p_env_peer_id_preserved_as_expected_identity: true
    - p2p_connect_interval_zero_rejected: true
    - p2p_env_peer_indices_contiguous_from_zero: true
```
