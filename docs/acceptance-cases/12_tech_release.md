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
    - run: rg -n "tags: \\['v\\*'\\]|type=semver,pattern=\\{\\{version\\}\\}|type=raw,value=latest|ghcr.io/\\$\\{\\{ github.repository \\}\\}" .github/workflows/release.yml
  assertions:
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
    - run: cargo test --locked
    - run: DEVE_RELEASE_AUDIT_REQUIRED=1 scripts/check-release-audit-gate.sh
  assertions:
    - exit_code_all_eq: 0

- case_id: REL-004
  goal: 当前运行与测试入口文档和实现边界保持一致。
  preconditions:
    - docs/dev-runbook.md 可读
  steps:
    - run: scripts/check-dev-runbook-baseline.sh
  assertions:
    - exit_code_eq: 0

- case_id: REL-005
  goal: Docker、Compose 与 release workflow 保持当前 embedded frontend 单二进制发布边界。
  preconditions:
    - Dockerfile、docker-compose.yml、.github/workflows/release.yml 可读
  steps:
    - run: scripts/check-release-baseline.sh
    - run: scripts/check-native-track-boundary.sh
    - run: scripts/check-native-packaging-gate.sh
    - run: scripts/check-native-process-adapter-gate.sh
    - run: scripts/check-native-target-host-evidence.sh
    - run: scripts/install-native-target-host-tools.sh
    - run: scripts/check-desktop-package-preflight.sh
    - run: scripts/check-desktop-platform-package-build.sh
    - run: scripts/check-desktop-package-startup-smoke.sh
    - run: scripts/check-desktop-target-host-preflight.sh
    - run: scripts/check-mobile-platform-package-preflight.sh
    - run: scripts/check-mobile-android-shell-package-build.sh
    - run: scripts/check-graph-baseline.sh
    - run: cargo test -p deve_cli graph -- --nocapture
  assertions:
    - exit_code_eq: 0
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
    - chrome_mcp: open dashboard
  assertions:
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
```
