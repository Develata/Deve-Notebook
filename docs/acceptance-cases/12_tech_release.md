## 技术栈、性能预算与发布

```markdown
- case_id: TECH-001
  goal: 技术栈版本匹配计划。
  preconditions:
    - Cargo.toml、rust-toolchain.toml 与 apps/web/package.json 可读
  steps:
    - run: rg -n "leptos|redb|argon2|ed25519" Cargo.toml
    - run: cargo run --locked --quiet -p deve_baseline -- release
  assertions:
    - stdout_contains: "leptos"
    - stdout_contains: "redb"
    - stdout_contains: "release-baseline-check: ok"

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
  goal: 最小性能预算入口覆盖 open-doc、edit-ack、cold-mount 与 RSS baseline，并绑定 low-spec 配置检查。
  preconditions:
    - `docs/plan/21_perf_budget.md` 可读
    - Rust baseline checker 可运行
    - DEVE_PROFILE=low-spec
  steps:
    - run: scripts/plan-coverage.sh --check-perf-budget
    - run: scripts/check-perf-budget-baseline.sh
    - run: cargo run -p deve_baseline -- perf-budget
    - run: deve config print
  assertions:
    - stdout_contains: "check-perf-budget: OK"
    - stdout_contains: "perf-budget-baseline-check: ok"
    - contract_assert: open_doc_budget_entry_present true
    - contract_assert: edit_ack_budget_entry_present true
    - contract_assert: cold_mount_budget_entry_present true
    - contract_assert: rss_baseline_entry_present true
    - stdout_contains: "profile = 'low-spec'"

- case_id: REL-001
  goal: 当前 release channel 在 tag 前构建、验收、哈希并 attest 一个 exact-HEAD candidate；单一 tag orchestrator 只提升 sealed bytes。
  preconditions:
    - `.github/workflows/release-candidate.yml` 可读
    - `.github/workflows/acceptance-aggregate.yml` 可读
    - `.github/workflows/release.yml` 可读
    - `.github/workflows/release-native.yml` 可读
  steps:
    - run: scripts/check-release-baseline.sh
    - run: cargo run -p deve_baseline -- release
    - run: scripts/check-release-version-match.test.sh
    - run: scripts/validate-release-image-tags.test.sh
    - run: cargo test --locked -p deve_baseline release_candidate -- --nocapture
    - run: rg -n "tags: \\['v\\*'\\]|Deve-Acceptance-Aggregate-Run|docker load --input|gh release upload" .github/workflows/release.yml
  assertions:
    - stdout_contains: "release-baseline-check: ok"
    - stdout_contains: "tags: ['v*']"
    - stdout_contains: "Deve-Acceptance-Aggregate-Run"
    - stdout_contains: "docker load --input"
    - stdout_contains: "gh release upload"
    - release_assert: release_yml_is_only_direct_v_tag_entry true
    - release_assert: non_semver_v_tag_rejected_before_checkout_or_promotion true
    - release_assert: checked_out_workspace_desktop_mobile_versions_exact_match_tag true
    - release_assert: prerelease_and_build_metadata_preserved_in_manifest_and_git_release true
    - release_assert: docker_build_metadata_uses_injective_safe_tag_mapping true
    - release_assert: prerelease_does_not_update_registry_latest true
    - release_assert: candidate_version_and_all_jobs_bound_to_exact_head true
    - release_assert: candidate_builds_current_head_web_dist_before_native_preflight true
    - release_assert: annotated_tag_binds_exactly_one_aggregate_run true
    - release_assert: docker_candidate_built_once_smoked_archived_and_attested_before_tag true
    - release_assert: windows_macos_signed_android_artifacts_built_before_tag true
    - release_assert: candidate_manifest_rejects_path_escape_symlink_reparse_duplicate_extra_or_corrupt_files true
    - release_assert: candidate_and_aggregate_rerun_rejected_in_favor_of_fresh_dispatch true
    - release_assert: source_and_docker_spdx_subjects_are_distinct true
    - release_assert: sealed_provenance_and_spdx_attestation_bundles_verified_offline true
    - release_assert: sealed_android_apk_signer_reextracted_and_exactly_one true
    - release_assert: aggregate_recomputes_hashes_and_verifies_attestation_before_tag_ready true
    - release_assert: tag_orchestrator_loads_sealed_docker_archive_without_rebuild true
    - release_assert: stable_version_and_latest_remote_manifest_digests_match true
    - release_assert: latest_requires_strict_semver_and_git_history_progression true
    - release_assert: native_delivery_is_build_only_candidate_workflow true
    - release_assert: github_release_created_once_from_sealed_native_assets true
    - release_assert: native_failure_creates_no_github_release true
    - release_assert: native_manifest_requires_exact_allowlisted_asset_set true
    - release_assert: native_manifest_rejects_extra_downloaded_files true
    - release_assert: existing_public_release_rejected_before_asset_upload true
    - release_assert: github_release_remains_draft_until_remote_asset_manifest_matches true
    - security_assert: reusable_native_receives_only_android_signing_secrets true
    - evidence_boundary: promotion_is_not_cross_registry_atomic

- case_id: REL-002
  goal: Docker release 镜像 boot/auth 与 embedded frontend metadata preflight 可用。
  preconditions:
    - Docker 可用
    - AUTH_SECRET 已设置为 32 字节以上随机字符串
    - AUTH_PASS 已设置为 Argon2 PHC 密码哈希
  steps:
    - run: DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh
    - run: DEVE_DOCKER_SMOKE_REQUIRED=1 DEVE_DOCKER_SMOKE_SKIP_BUILD=1 DEVE_DOCKER_SMOKE_IMAGE=deve-notebook:existing scripts/smoke-docker-release.sh
    - run: DEVE_DOCKER_MULTI_REQUIRED=1 DEVE_DOCKER_MULTI_SKIP_BUILD=1 DEVE_DOCKER_MULTI_IMAGE=deve-notebook:existing scripts/smoke-docker-multiclient.sh
    - run: node --test scripts/smoke-docker-existing-image.test.mjs
    - note: use `DEVE_DOCKER_BIN=/path/to/docker DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh` when Docker is not named `docker`
  assertions:
    - http_status_eq: 200
    - runtime_assert: delivery_eq_embedded_frontend true
    - runtime_assert: local_repo_count_at_least_one true
    - auth_assert: production_login_succeeds true
    - release_assert: existing_image_mode_rejects_missing_image true
    - release_assert: existing_image_mode_does_not_build true
    - stderr_contains_on_docker_unavailable: "docker-release-smoke: docker_bin="

- case_id: REL-003
  goal: 发布前检查项可验证。
  preconditions:
    - CI 环境
    - Redb v4 与 immutable Remote Import backend/product cutover 已实现；F4/v5 ownership-aware Repo Control、B5/B6 与其余 release gates 仍阻塞 tag
    - release baseline 必须同时验证 approved target 与 current implementation，不得把 current 当作首发完成态
  steps:
    - run: rustup target add wasm32-unknown-unknown
    - run: cargo check --locked -p deve_web --target wasm32-unknown-unknown
    - run: cargo run -p deve_baseline -- release
    - run: cargo run -p deve_baseline -- all
    - run: cargo run -p deve_baseline -- full
    - run: cargo run -p deve_baseline -- local-quick-gate
    - run: cargo run -p deve_baseline -- deep-audit-gate
    - run: cargo test -p deve_core ledger_entry_format -- --nocapture
    - run: cargo test -p deve_core redb_schema_version -- --nocapture
    - run: cargo test -p deve_cli static_files -- --nocapture
    - run: cargo test --locked
    - run: cargo run -p deve_baseline -- release-audit-gate
    - run: DEVE_RELEASE_AUDIT_REQUIRED=1 scripts/check-release-audit-gate.sh
    - run: rg -n "LEDGER_ENTRY_FORMAT_VERSION = 3" docs/registry/first-tag-format-matrix.md
    - run: rg -n "REDB_SCHEMA_VERSION = 4|Redb schema v4" docs/registry/first-tag-format-matrix.md
    - run: rg -n "WS_PROTOCOL_VERSION = 5" docs/registry/first-tag-format-matrix.md
    - run: rg -n "当前代码已切换F4/v5" docs/registry/first-tag-format-matrix.md
    - run: rg -n "MIN_SUPPORTED_WS_PROTOCOL_VERSION = 5" docs/registry/first-tag-format-matrix.md
    - run: rg -n "magic `DEVEWSF4`" docs/registry/first-tag-format-matrix.md
  assertions:
    - exit_code_all_eq: 0
    - stdout_contains: "storage-repo-baseline-check: ok"
    - stdout_contains: "network-baseline-check: ok"
    - stdout_contains: "release-baseline-check: ok"
    - stdout_contains: "repo-file-ops-baseline: ok"
    - release_assert: first_tag_data_format_postcard_v3_gates_present true
    - release_assert: legacy_binary_codec_dependency_absent true
    - release_assert: first_tag_format_matrix_bound_to_plan_and_code true
    - release_assert: first_tag_target_current_drift_blocks_tag true
    - release_assert: redb_v4_local_profile_includes_projection_faults true
    - release_assert: validation_script_ownership_policy_classified true
    - release_assert: cargo_audit_warnings_match_registry true
    - release_assert: audit_warning_registry_has_rationale_or_replacement_route true
    - release_assert: yanked_warnings_without_advisory_use_synthetic_registry_key true
    - release_assert: trunk_dev_index_rejected_by_static_delivery true

- case_id: REL-003A
  goal: 首个公开 tag 前，release audit readiness 必须反映 ADR 0006 Route 2：Linux GTK3 native artifacts 不进入 release set。
  preconditions:
    - 当前 release audit warning registry 已登记 GTK3/glib warning
    - `docs/adr/0006-native-linux-gtk3-first-tag-route.md` 为 Accepted
  steps:
    - run: cargo run -p deve_baseline -- release-audit-gate tag-ready
    - run: DEVE_RELEASE_TAG_READY_REQUIRED=1 cargo run -p deve_baseline -- release-audit-gate
  assertions:
    - exit_code_all_eq: 0
    - stdout_contains: "release-audit-gate-check: ok"
    - release_assert: linux_gtk3_native_artifacts_excluded_from_first_tag true
    - release_assert: gtk3_glib_warnings_registered_but_not_tag_blockers true

- case_id: REL-004
  goal: 当前运行与测试入口文档和实现边界保持一致。
  preconditions:
    - docs/dev-runbook.md 可读
  steps:
    - run: scripts/check-dev-runbook-baseline.sh
  assertions:
    - exit_code_eq: 0

- case_id: REL-005
  goal: Docker、Compose、release tag orchestrator、reusable native workflow 与 target-host platform evidence 保持当前 embedded frontend / native runtime 发布边界。
  preconditions:
    - Dockerfile、docker-compose.yml、.github/workflows/release.yml、.github/workflows/release-native.yml 与 .github/workflows/native-target-host.yml 可读
    - platform evidence 只声明 target-host package、startup、install 与 native runtime smoke
    - platform evidence 不声明 signed release、store distribution、physical-device readiness 或 native authority writes
    - process runtime evidence 只表达默认 no-Tauri closed、Desktop LocalBackend controlled child-process 与 Mobile child-process closed
  steps:
    - run: scripts/check-release-baseline.sh
    - run: cargo run -p deve_baseline -- release
    - run: scripts/check-native-track-boundary.sh
    - run: scripts/check-native-packaging-gate.sh
    - run: scripts/check-native-process-adapter-gate.sh
    - run: scripts/check-native-track-boundary.sh
    - run: scripts/check-native-target-host-evidence.sh
    - run: scripts/install-native-target-host-tools.sh
    - run: scripts/check-desktop-package-preflight.sh
    - slurm_run: scripts/check-desktop-linux-apptainer-slurm.sh
    - run: cargo run -p deve_baseline -- desktop-platform-package-build
    - run: scripts/check-desktop-platform-package-build.sh
    - run: cargo run -p deve_baseline -- desktop-package-startup-smoke
    - run: scripts/check-desktop-package-startup-smoke.sh
    - run: cargo run -p deve_baseline -- desktop-native-session-package-smoke
    - run: scripts/check-desktop-native-session-package-smoke.sh
    - run: cargo run -p deve_baseline -- desktop-installer-smoke
    - run: scripts/check-desktop-installer-smoke.sh
    - run: scripts/check-desktop-target-host-preflight.sh
    - run: cargo run -p deve_baseline -- desktop-target-host-preflight
    - run: scripts/check-mobile-platform-package-preflight.sh
    - run: scripts/check-mobile-android-shell-package-build.sh
    - run: cargo run -p deve_baseline -- mobile-android-emulator-install-startup-smoke
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
    - release_assert: trunk_dev_index_not_served_as_release_frontend true
    - release_assert: api_only_does_not_prove_embedded_frontend_health true
    - release_assert: target_host_platform_evidence_runtime_boundary_current true
    - release_assert: signed_release_readiness_not_claimed true
    - release_assert: store_distribution_readiness_not_claimed true
    - release_assert: physical_device_readiness_not_claimed true
    - release_assert: signing_and_physical_device_preflight_diagnostic_only true
    - release_assert: default_no_tauri_process_runtime_closed true
    - release_assert: desktop_localbackend_process_runtime_controlled true
    - release_assert: mobile_child_process_runtime_closed true
    - release_assert: native_authority_writes_closed true
    - release_assert: first_tag_native_artifacts_windows_macos_android_only true
    - release_assert: linux_desktop_and_ios_artifacts_excluded_from_first_tag true
    - release_assert: native_public_preview_signing_boundaries_explicit true
    - release_assert: sealed_native_artifacts_attach_to_github_release_only_during_tag_promotion true
    - release_assert: release_native_has_no_independent_tag_trigger true
    - release_assert: release_native_is_build_and_target_host_smoke_only true
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
    - run: cargo test -p deve_cli cgroup_ -- --nocapture
    - run: cargo test -p deve_cli node_role_watcher_health -- --nocapture
    - chrome_mcp: open dashboard
  assertions:
    - stdout_contains: "release-baseline-check: ok"
    - json_fields_present: ["version", "profile", "delivery", "environment"]
    - json_fields_present: ["repo_health.status", "repo_health.local_total", "repo_health.degraded"]
    - json_fields_present: ["watcher_health.status", "watcher_health.expected", "watcher_health.running", "watcher_health.unavailable"]
    - json_fields_absent: ["watcher_health.repos", "watcher_health.repo_id", "watcher_health.generation", "watcher_health.path", "watcher_health.failure"]
    - ui_text_visible_any_of: ["embedded-frontend", "static-dir", "api-only", "plugin-host-proxy"]
    - metrics_assert: container_memory_uses_current_cgroup_usage true
    - metrics_assert: container_cpu_uses_cgroup_usage_and_effective_capacity true

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
    - ui_assert: client_a_create_edit_visible_on_client_b true
    - ui_assert: client_b_offline_readonly_then_reconnect_ready true
    - ui_assert: client_b_offline_input_does_not_change_content_pending_or_client_a true
    - ui_assert: client_b_post_reconnect_edit_visible_on_client_a true
    - ws_assert: browser_clients_connect_expected_container_origin true
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
    - p2p_assert: full_peer_ws_admission_succeeds_with_configured_bearer_token true
    - p2p_assert: peer_b_shadow_contains_peer_a_write true
    - p2p_assert: peer_b_local_branch_unchanged_before_explicit_merge true
    - p2p_assert: peer_b_reconnect_performs_fresh_authenticated_handshake true
    - security_assert: p2p_token_material_not_logged_or_written_to_persisted_data_or_projection_files true
    - evidence_boundary: explicit merge is covered by NET-015 targeted server tests, not this Docker smoke
    - evidence_gap: live post-reconnect vector equality is not exposed by the current diagnostic surface

- case_id: REL-011
  goal: Desktop/Android/Mobile native 双模式可验收，native shell 不直接拥有业务 authority。
  preconditions:
    - native-packaging 构建可用
    - 默认 native-packaging 启动使用 LocalBackend
    - RemoteBrowser 使用显式 HTTPS origin
  steps:
    - run: scripts/check-native-process-adapter-gate.sh
    - run: scripts/check-native-packaging-gate.sh
    - run: scripts/check-desktop-native-session-package-smoke.sh
    - run: scripts/check-desktop-installer-smoke.sh
    - run: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-desktop-local-backend-lifecycle.ps1 -ForceGitUnavailable
    - run: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-desktop-packaged-ui-smoke.ps1 -DesktopBinary <installed_deve_desktop.exe> -WorkRoot <temp_root>
    - run: node --test scripts/smoke-desktop-packaged-ui.test.mjs
    - run: node --test scripts/smoke-desktop-remote-browser.test.mjs
    - run: pwsh -NoProfile -File scripts/desktop-install-root.test.ps1
    - run: pwsh -NoProfile -File scripts/remote-browser-fixture.test.ps1
    - run: bash scripts/remote-browser-fixture.test.sh
    - run: cargo test -p deve_core native_adapter -- --nocapture
    - run: cargo test -p deve_desktop --features native-packaging -- --nocapture
    - run: cargo test -p deve_mobile --features native-packaging -- --nocapture
  assertions:
    - native_assert: desktop_local_backend_default_starts_controlled_loopback_service true
    - native_assert: desktop_native_session_smoke_stops_local_service_after_probe true
    - native_assert: desktop_native_session_smoke_uses_temporary_data_root true
    - native_assert: mobile_local_backend_default_uses_embedded_loopback_service true
    - native_assert: mobile_embedded_backend_uses_typed_runtime_auth_material true
    - dependency_assert: mobile_android_ios_bridge_dependencies_are_target_scoped_optional_and_native_packaging_only true
    - native_assert: remote_browser_accepts_https_origin_only true
    - native_assert: remote_browser_does_not_start_local_backend_or_inject_native_bootstrap true
    - native_assert: native_shell_has_no_direct_ledger_source_control_search_writes true
    - native_assert: desktop_installer_smoke_uses_local_bare_git_remote true
    - native_assert: desktop_installer_smoke_covers_notegit_commit_mirror_import_export_push true
    - native_assert: desktop_git_unavailable_does_not_rollback_notegit_commit true
    - native_assert: desktop_packaged_ui_uses_installed_binary_and_real_webview true
    - native_assert: desktop_packaged_ui_covers_create_edit_commit_history_and_settings_focus_trap true
    - native_assert: desktop_packaged_ui_exit_leaves_no_orphan_sidecar true
    - evidence_boundary: startup_marker_probe_is_not_packaged_ui_readiness
    - release_assert: signed_release_readiness_not_claimed true
    - release_assert: store_distribution_readiness_not_claimed true
    - release_assert: physical_device_readiness_not_claimed true

- case_id: REL-012
  goal: branch push / pull request CI 只做检查，不执行 package、publish 或 production 动作。
  preconditions:
    - `.github/workflows/check.yml` 可读
  steps:
    - run: rg -n "branches: \\[main\\]|pull_request:|cargo fmt --check|cargo clippy --locked --all-targets|cargo test --locked|deve_baseline -- all" .github/workflows/check.yml
    - run: "! rg -n \"packages: write|docker/(login|metadata|build-push)-action|actions/upload-artifact|push: true|ghcr\\.io|tags: \\['v\\*'\\]\" .github/workflows/check.yml"
  assertions:
    - stdout_contains: "branches: [main]"
    - stdout_contains: "pull_request:"
    - stdout_contains: "cargo fmt --check"
    - stdout_contains: "cargo clippy --locked --all-targets"
    - stdout_contains: "cargo test --locked"
    - stdout_contains: "deve_baseline -- all"
    - release_assert: push_ci_check_only true
    - release_assert: push_ci_no_package_publish_or_production true

- case_id: REL-013
  goal: Reliability / Observability 治理合同可由发布前基线验证。
  preconditions:
    - `docs/plan/22_reliability_observability.md` 可读
    - Rust baseline checker 可运行
  steps:
    - run: scripts/check-reliability-observability-baseline.sh
    - run: cargo run -p deve_baseline -- reliability-observability
  assertions:
    - stdout_contains: "reliability-observability-baseline-check: ok"
    - contract_assert: slo_sli_catalog_bound true
    - contract_assert: telemetry_schema_required_fields_bound true
    - contract_assert: metrics_taxonomy_bound true
    - contract_assert: high_cardinality_metric_labels_forbidden true
    - contract_assert: tracing_span_boundary_bound true
    - contract_assert: observation_health_mapping_bound true
    - contract_assert: watcher_failure_maps_to_repo_mount_state_not_repo_health_or_projection_fault true
    - contract_assert: alerting_tier_bound true
    - contract_assert: repo_local_ingestion_unavailable_is_t2_and_zero_mounted_or_host_fatal_is_t1 true
    - contract_assert: resilience_playbook_index_bound true
    - contract_assert: watcher_detail_is_tracing_only_and_public_health_is_aggregate true

- case_id: REL-014
  goal: 分层验收矩阵与 Rust producer runner 对 case、operation flow、first-tag journey、实际 smoke 和 evidence receipt fail-closed。
  preconditions:
    - `docs/registry/acceptance-matrix.tsv` 可读
    - `docs/registry/acceptance-producers.json` 可读
    - Rust 1.97.0 toolchain 可用
  steps:
    - run: cargo run --locked -p deve_baseline -- acceptance-matrix
    - run: cargo run --locked -p deve_baseline -- acceptance-run --tier ci --plan
    - run: cargo run --locked -p deve_baseline -- acceptance-run --tier ci
    - run: cargo run --locked -p deve_baseline -- acceptance-run --tier tag-ready --plan
    - run: cargo test --locked -p deve_baseline acceptance_matrix -- --nocapture
    - run: receipt_root="$(mktemp -d)" && ! cargo run --locked -p deve_baseline -- acceptance-matrix --tag-ready "$receipt_root"
  assertions:
    - contract_assert: all_acceptance_cases_bound true
    - contract_assert: operation_flow_case_relations_exact true
    - contract_assert: first_tag_journey_surface_complete true
    - contract_assert: store_011_and_store_024_not_soft_unbound true
    - contract_assert: generated_acceptance_matrix_drift_rejected true
    - contract_assert: dirty_failed_stale_or_missing_receipt_rejected true
    - contract_assert: receipt_head_surface_mode_platform_and_locator_bound true
    - contract_assert: receipt_producer_contract_and_execution_group_bound true
    - contract_assert: every_required_receipt_has_exactly_one_typed_producer true
    - contract_assert: every_required_ci_test_or_script_has_exactly_one_typed_producer true
    - contract_assert: ci_tier_executes_nonempty_producer_set_without_receipts true
    - contract_assert: producer_shell_command_strings_forbidden true
    - contract_assert: producer_bound_environment_is_public_non_secret true
    - contract_assert: producer_timeout_failure_writes_failed_receipts true
    - contract_assert: producer_timeout_cleanup_preserves_runner_process_group true
    - contract_assert: producer_timeout_cleanup_is_child_group_bound true
    - contract_assert: evidence_filter_selects_complete_atomic_producer_group true
    - contract_assert: receipt_collection_is_root_pinned_and_resource_bounded true
    - contract_assert: receipt_collection_rejects_excessive_directory_depth true
    - contract_assert: receipt_collection_rejects_inconsistent_execution_fields true
    - contract_assert: producer_finally_cleanup_is_execution_scoped_and_bounded true
    - contract_assert: one_producer_execution_atomically_emits_multiple_bound_receipts true
    - contract_assert: explicit_cross_workflow_run_ids_are_head_bound_before_collection true
    - contract_assert: candidate_and_receipt_source_runs_are_explicit_head_version_bound true
    - contract_assert: sealed_candidate_artifact_expiry_requires_full_regeneration true
    - release_assert: explicit_p0_gap_blocks_tag_ready true
```
