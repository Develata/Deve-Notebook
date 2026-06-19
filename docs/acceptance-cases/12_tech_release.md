## 技术栈、性能预算与发布

```markdown
- case_id: TECH-001
  goal: 技术栈版本匹配计划。
  preconditions:
    - Cargo.toml/package.json 可读
  steps:
    - run: rg -n "leptos|redb|argon2|ed25519" Cargo.toml
  assertions:
    - stdout_contains: "leptos"
    - stdout_contains: "redb"

- case_id: TECH-002
  goal: Markdown 导出遵循 CommonMark + GFM。
  preconditions:
    - 准备含多种语法的文档
  steps:
    - run: deve export --format markdown --doc <doc_id> --out /tmp/export.md
    - run: rg -n "==highlight==|<div>" /tmp/export.md
  assertions:
    - stdout_empty true

- case_id: PERF-001
  goal: Low-Spec 配置禁用重能力。
  preconditions:
    - DEVE_PROFILE=low-spec
  steps:
    - run: deve config print
  assertions:
    - stdout_contains: "profile = 'low-spec'"

- case_id: REL-001
  goal: 当前 release channel 由 tag-triggered GHCR/Docker surface 表达。
  preconditions:
    - `.github/workflows/release.yml` 可读
  steps:
    - run: scripts/check-release-baseline.sh
    - run: cargo run -p deve_baseline -- release
    - run: rg -n "tags: \\['v\\*'\\]|type=semver,pattern=\\{\\{version\\}\\}|type=raw,value=latest|ghcr.io/\\$\\{\\{ github.repository \\}\\}" .github/workflows/release.yml
  assertions:
    - stdout_contains: "release-baseline-check: ok"
    - stdout_contains: "tags: ['v*']"
    - stdout_contains: "type=semver,pattern={{version}}"
    - stdout_contains: "type=raw,value=latest"
    - stdout_contains: "ghcr.io/${{ github.repository }}"

- case_id: REL-002
  goal: Docker 部署可用。
  preconditions:
    - Docker 可用
    - AUTH_SECRET 已设置为 32 字节以上随机字符串
    - AUTH_PASS 已设置为 Argon2 PHC 密码哈希
  steps:
    - run: DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh
    - note: use `DEVE_DOCKER_BIN=/path/to/docker DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh` when Docker is not named `docker`
  assertions:
    - http_status_eq: 200
    - stderr_contains_on_docker_unavailable: "docker-release-smoke: docker_bin="

- case_id: REL-003
  goal: 发布前检查项可验证。
  preconditions:
    - CI 环境
  steps:
    - run: rustup target add wasm32-unknown-unknown
    - run: cargo check --locked -p deve_web --target wasm32-unknown-unknown
    - run: cargo run -p deve_baseline -- all
    - run: cargo test -p deve_core ledger_entry_format -- --nocapture
    - run: cargo test -p deve_core redb_schema_version -- --nocapture
    - run: cargo test --locked
    - run: DEVE_RELEASE_AUDIT_REQUIRED=1 scripts/check-release-audit-gate.sh
  assertions:
    - exit_code_all_eq: 0
    - stdout_contains: "storage-repo-baseline-check: ok"
    - stdout_contains: "network-baseline-check: ok"
    - stdout_contains: "release-baseline-check: ok"
    - release_assert: stable_data_format_v1_gates_present true

- case_id: REL-004
  goal: 当前运行与测试入口文档和实现边界保持一致。
  preconditions:
    - docs/dev-runbook.md 可读
  steps:
    - run: scripts/check-dev-runbook-baseline.sh
  assertions:
    - exit_code_eq: 0

- case_id: REL-005
  goal: Docker、Compose、release workflow 与 target-host platform evidence 保持当前 embedded frontend / shell-only 发布边界。
  preconditions:
    - Dockerfile、docker-compose.yml、.github/workflows/release.yml 与 .github/workflows/native-target-host.yml 可读
    - platform evidence 只声明 target-host shell package、startup、install smoke
    - platform evidence 不声明 signed release、store distribution、physical-device readiness、native process runtime 或 native authority writes
  steps:
    - run: scripts/check-release-baseline.sh
    - run: cargo run -p deve_baseline -- release
    - run: scripts/check-native-track-boundary.sh
    - run: scripts/check-native-packaging-gate.sh
    - run: scripts/check-native-process-adapter-gate.sh
    - run: scripts/check-native-target-host-evidence.sh
    - run: scripts/install-native-target-host-tools.sh
    - run: scripts/check-desktop-package-preflight.sh
    - run: scripts/check-desktop-platform-package-build.sh
    - run: scripts/check-desktop-package-startup-smoke.sh
    - run: scripts/check-desktop-installer-smoke.sh
    - run: scripts/check-desktop-target-host-preflight.sh
    - run: scripts/check-mobile-platform-package-preflight.sh
    - run: scripts/check-mobile-android-shell-package-build.sh
    - run: scripts/check-mobile-android-emulator-install-startup-smoke.sh
    - run: scripts/check-mobile-ios-shell-package-build.sh
    - run: scripts/check-mobile-android-install-startup-smoke.sh
    - run: scripts/check-mobile-ios-install-startup-smoke.sh
    - run: scripts/check-desktop-signing-preflight.sh
    - run: scripts/check-mobile-android-release-preflight.sh
    - run: scripts/check-graph-baseline.sh
    - run: cargo test -p deve_cli graph -- --nocapture
  assertions:
    - exit_code_eq: 0
    - stdout_contains: "release-baseline-check: ok"
    - release_assert: embedded_frontend_single_binary_boundary true
    - release_assert: target_host_platform_evidence_shell_only true
    - release_assert: signed_release_readiness_not_claimed true
    - release_assert: store_distribution_readiness_not_claimed true
    - release_assert: physical_device_readiness_not_claimed true
    - release_assert: signing_and_physical_device_preflight_diagnostic_only true
    - release_assert: native_process_runtime_closed true
    - release_assert: native_authority_writes_closed true
    - api_assert: graph_projection_http_endpoint_protected_readonly true
    - api_assert: graph_projection_degraded_failure_code_eq "GRAPH_DEGRADED_PROJECTION_REQUIRED"
    - cli_assert: graph_projection_cli_and_http_share_adapter true
    - ui_assert: graph_projection_panel_summary_counts_available true
    - ui_assert: graph_projection_panel_renderer_future_only true
    - ui_assert: graph_renderer_gate_closed_current_batch true
    - ui_assert: graph_projection_panel_local_only_state_available true
    - ui_assert: graph_projection_panel_blocked_state_available true
    - ui_assert: graph_projection_panel_degraded_state_available true
    - ui_assert: graph_projection_panel_empty_state_available true
    - ui_assert: graph_projection_panel_error_state_available true
    - dependency_assert: graph_renderer_dependency_not_declared_for_current_ui true

- case_id: REL-006
  goal: 当前运行实例暴露可理解的版本、profile、环境与交付形态。
  preconditions:
    - 后端 `/api/node/role` 可访问
    - Web dashboard 可打开
  steps:
    - run: DEVE_RUNTIME_SMOKE_REQUIRED=1 scripts/smoke-runtime-release-info.sh
    - run: scripts/check-release-baseline.sh
    - run: cargo run -p deve_baseline -- release
    - chrome_mcp: open dashboard
  assertions:
    - stdout_contains: "release-baseline-check: ok"
    - json_fields_present: ["version", "profile", "delivery", "environment"]
    - json_fields_present: ["repo_health.status", "repo_health.local_total", "repo_health.degraded"]
    - ui_text_visible_any_of: ["embedded-frontend", "static-dir", "api-only", "plugin-host-proxy"]

- case_id: REL-007
  goal: 当前运行写读主链路可在临时 repo 中自动验收。
  preconditions:
    - Rust workspace 可构建
  steps:
    - run: scripts/smoke-runtime-happy-path.sh
  assertions:
    - ws_assert: repo_switch_sync_hello_register_writer_ok true
    - ws_assert: create_edit_ack_new_op_ok true
    - ws_assert: open_doc_history_readback_ok true
    - ui_assert: reconnect_bootstrap_restore_contract_ok true

- case_id: REL-008
  goal: 当前运行恢复链路可自动验收。
  preconditions:
    - Rust workspace 可构建
  steps:
    - run: scripts/smoke-runtime-recovery-path.sh
  assertions:
    - server_assert: degraded_local_writes_blocked_before_mutation true
    - server_assert: stale_sync_scope_cleanup_ok true
    - ui_assert: reconnect_and_read_only_gates_ok true
    - ui_assert: auth_failure_status_not_conflated_with_reconnect true

- case_id: REL-009
  goal: Docker 容器运行时可通过多浏览器 WebLightPeer 实际 smoke。
  preconditions:
    - Docker 与 Docker Compose 可用
    - Node/npm 可用
    - Playwright 可通过 npm 获取或已缓存
  steps:
    - run: DEVE_DOCKER_MULTI_REQUIRED=1 bash scripts/smoke-docker-multiclient.sh
    - note: use `DEVE_DOCKER_MULTI_KEEP=1` to keep the compose project running for Chrome MCP visual validation
  assertions:
    - docker_assert: one_containerized_server_ready true
    - browser_assert: isolated_browser_contexts_login_independently true
    - ws_assert: browser_clients_connect_relative_ws true
    - ui_assert: client_a_create_edit_visible_on_client_b true
    - ui_assert: client_b_offline_readonly_then_reconnect_ready true
    - ui_assert: no_blank_page_or_framework_overlay true
    - ui_assert: no_relevant_console_errors true

- case_id: REL-010
  goal: Docker 双服务端 FullPeer mesh 可通过隔离 volume smoke。
  preconditions:
    - Docker 与 Docker Compose 可用
    - 两个服务端使用相同 `RepoId`
    - 两端 ledger/data/notes volume 隔离
    - P2P token 只通过环境变量注入
  steps:
    - run: DEVE_DOCKER_P2P_MESH_REQUIRED=1 bash scripts/smoke-docker-p2p-mesh.sh
    - note: use `DEVE_DOCKER_P2P_MESH_KEEP=1` to keep both peers running for manual diagnostics
  assertions:
    - docker_assert: peer_a_and_peer_b_ready true
    - p2p_assert: full_peer_ws_admission_uses_bearer_token true
    - p2p_assert: peer_b_shadow_contains_peer_a_write true
    - p2p_assert: peer_b_local_branch_unchanged_before_explicit_merge true
    - p2p_assert: explicit_merge_makes_remote_content_local_visible true
    - p2p_assert: reconnect_vector_aligned true
    - security_assert: p2p_token_material_not_logged_or_written_to_config true

- case_id: REL-011
  goal: Desktop/Android/Mobile native 双模式可验收，native shell 不直接拥有业务 authority。
  preconditions:
    - native-packaging 构建可用
    - 默认 native-packaging 启动使用 LocalBackend
    - RemoteBrowser 使用显式 HTTPS origin
  steps:
    - run: scripts/check-native-process-adapter-gate.sh
    - run: scripts/check-native-packaging-gate.sh
    - run: cargo test -p deve_core native_adapter -- --nocapture
    - run: cargo test -p deve_desktop --features native-packaging -- --nocapture
    - run: cargo test -p deve_mobile --features native-packaging -- --nocapture
  assertions:
    - native_assert: desktop_local_backend_default_starts_controlled_loopback_service true
    - native_assert: mobile_local_backend_default_uses_embedded_loopback_service true
    - native_assert: mobile_embedded_backend_uses_typed_runtime_auth_material true
    - native_assert: remote_browser_accepts_https_origin_only true
    - native_assert: remote_browser_does_not_start_local_backend_or_inject_native_bootstrap true
    - native_assert: native_shell_has_no_direct_ledger_source_control_search_writes true
    - release_assert: signed_release_readiness_not_claimed true
    - release_assert: store_distribution_readiness_not_claimed true
    - release_assert: physical_device_readiness_not_claimed true
```
