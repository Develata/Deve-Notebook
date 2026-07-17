## 多仓库与存储

```markdown
- case_id: STORE-001
  goal: Trinity Isolation 与 Projection Locator 结构存在。
  preconditions:
    - CLI init 与 core ledger init 可在临时目录运行
  steps:
    - run: cargo test -p deve_cli init_creates_trinity_workspace_layout -- --nocapture
    - run: cargo test -p deve_cli projection_locator_init_writes_locator_without_vault_path_config -- --nocapture
    - run: cargo test -p deve_core trinity_dir_structure_after_init -- --nocapture
    - run: cargo test -p deve_core projection_locator_toml_roundtrip -- --nocapture
    - run: scripts/check-storage-repo-baseline.sh
  assertions:
    - cli_assert: init_trinity_layout_bound true
    - cli_assert: ledger_local_remotes_bound true

- case_id: STORE-002
  goal: Repo 命名冲突自动重命名。
  preconditions:
    - 两个 Repo 同名不同 URL
  steps:
    - run: cargo test -p deve_core init_allocates_collision_safe_repo_name_for_same_name_different_url -- --nocapture
    - run: scripts/check-storage-repo-baseline.sh
  assertions:
    - cli_assert: collision_safe_local_repo_name true
    - cli_assert: repo_identity_not_changed_by_physical_suffix true

- case_id: STORE-003
  goal: Redb 索引表存在。
  preconditions:
    - 至少一个 .redb 文件
  steps:
    - run: cargo test -p deve_core required_redb_tables_exist_after_init -- --nocapture
    - run: cargo test -p deve_core redb_schema_version -- --nocapture
    - run: scripts/check-storage-repo-baseline.sh
  assertions:
    - cli_assert: required_storage_tables_reachable true
    - cli_assert: redb_schema_version_gate_present true
    - cli_assert: unversioned_redb_schema_fails_closed true

- case_id: STORE-004
  goal: Snapshot 双表与修剪。
  preconditions:
    - snapshot_depth=3
  steps:
    - run: cargo test -p deve_core snapshot_respects_depth_limit -- --nocapture
    - run: cargo test -p deve_core snapshot_rejects_middle_content_mismatch -- --nocapture
    - run: scripts/check-storage-repo-baseline.sh
  assertions:
    - cli_assert: snapshot_tables_reachable true
    - cli_assert: snapshot_count_le_config true
    - cli_assert: snapshot_candidate_matches_full_ledger_rebuild true

- case_id: STORE-005
  goal: Ledger-First 与原子持久化。
  preconditions:
    - 存在一个可编辑文档 <doc>
  steps:
    - run: cargo test -p deve_core edit_round_trip_reconstructs_content -- --nocapture
    - run: cargo test -p deve_core global_seq_increases -- --nocapture
    - run: cargo test -p deve_core ledger_entry_format -- --nocapture
    - run: scripts/check-storage-repo-baseline.sh
  assertions:
    - cli_assert: ledger_seq_increases true
    - cli_assert: reconstructed_content_matches true
    - cli_assert: ledger_entry_format_versioned true
    - cli_assert: unversioned_ledger_entry_rejected true

- case_id: STORE-006
  goal: Clean File Policy。
  preconditions:
    - 文档含 Frontmatter
  steps:
    - run: cargo test -p deve_cli markdown_export_preserves_user_frontmatter_without_system_metadata -- --nocapture
    - run: scripts/check-storage-repo-baseline.sh
  assertions:
    - cli_assert: clean_markdown_contains_user_frontmatter true
    - cli_assert: clean_markdown_has_no_system_metadata_injection true

- case_id: STORE-007
  goal: Watcher 事件映射、owned lifecycle、目录 scan 过滤与内部路径边界。
  preconditions:
    - watch 运行中并监听 local repo: main
    - 已跟踪文件存在: ${DEVE_DATA_DIR}/notes/main/notes/live.md
    - 已跟踪文件存在: ${DEVE_DATA_DIR}/notes/main/notes/delete.md
    - ${DEVE_DATA_DIR}/notes/main/.deveignore 包含: ignored/*.md
  steps:
    - run: cargo test -p deve_core watcher_records_create_modify_delete_candidates -- --nocapture
    - run: cargo test -p deve_core repo_watcher_handle_reports_identity_and_restarts_after_shutdown -- --nocapture
    - run: cargo test -p deve_core --lib dispatch_batch_rescans_removed_directory_after_path_disappears -- --nocapture
    - run: cargo test -p deve_core --lib full_rescan_emits_repo_scoped_refresh -- --nocapture
    - run: cargo test -p deve_core --lib consumer_failure_preserves_primary_and_cleanup -- --nocapture
    - run: cargo test -p deve_core --lib worker_panic_becomes_typed_failure_and_stops_backend -- --nocapture
    - run: cargo test -p deve_core --lib cleanup_panic_becomes_typed_shutdown_failure -- --nocapture
    - run: cargo test -p deve_core --lib terminal_failure_is_visible_before_cleanup_completes -- --nocapture
    - run: cargo test -p deve_core watcher_drop_is_a_synchronous_cleanup_safety_net -- --nocapture
    - run: cargo test -p deve_core --test watcher_platform_fs watcher_directory_removal_rescans_tracked_descendants -- --nocapture
    - run: cargo test -p deve_core --test watcher_platform_fs watcher_stop_prevents_post_stop_delivery -- --nocapture
    - run: cargo test -p deve_web fs_change_refreshes_external_changes_sibling_view -- --nocapture
    - run: cargo test -p deve_core internal_repo_path_uses_segment_semantics -- --nocapture
    - run: cargo test -p deve_core --test watcher_internal_ignore -- --nocapture
    - run: cargo test -p deve_core --test watcher_internal_ignore watcher_respects_deveignore_for_matching_markdown -- --nocapture
    - run: cargo test -p deve_core --test watcher_internal_ignore watcher_startup_scan_respects_deveignore -- --nocapture
    - run: cargo test -p deve_core --test watcher_lifecycle repo_watcher_handle -- --nocapture
    - run: cargo test -p deve_core --lib watcher_capture_first_startup -- --nocapture
    - run: cargo test -p deve_core --test watcher_platform_fs watcher_capture_first_startup_reaches_running_with_preexisting_and_post_start_changes -- --nocapture --test-threads=1
    - run: cargo test -p deve_core --lib watcher_final_state_shutdown -- --nocapture
    - run: cargo test -p deve_cli standalone_watch -- --nocapture
    - run: cargo test -p deve_cli watcher_refresh_adapter_maps_all_domain_fields -- --nocapture
    - run: cargo test -p deve_cli server_shutdown_preserves_background_primary_and_watcher_failure -- --nocapture
    - run: scripts/check-storage-repo-baseline.sh
  assertions:
    - cli_assert: pending_fs_ops_contains_added_modified_deleted true
    - api_assert: removed_directory_rescans_tracked_descendants true
    - ws_assert: directory_rescan_emits_repo_scoped_dir_changed true
    - cli_assert: duplicate_watcher_batch_reservation_fails_before_attach true
    - api_assert: consumer_failure_stops_backend true
    - api_assert: watcher_stop_blocks_post_stop_delivery true
    - web_assert: fs_change_refreshes_external_changes_sibling_view true
    - cli_assert: internal_notegit_and_git_segments_ignored true
    - cli_assert: notegit_backup_sibling_not_rejected_by_prefix true
    - cli_assert: pending_fs_ops_ignores_deveignore true
    - cli_assert: ignored_markdown_not_appended_to_ledger true
    - api_assert: repo_watcher_handle_is_unique_non_clone_owner true
    - api_assert: startup_requires_clean_capture_first_scan_pass true
    - api_assert: startup_scan_window_change_retries_and_converges_pending true
    - api_assert: startup_churn_stops_after_three_dirty_passes true
    - api_assert: startup_scan_failure_preserves_primary_and_cleanup true
    - api_assert: startup_terminal_backend_failure_never_becomes_churn_or_running true
    - api_assert: startup_fixed_root_never_reresolves_locator true
    - api_assert: startup_real_fs_preexisting_and_post_cut_changes_reach_pending true
    - api_assert: worker_failure_is_typed_and_generation_guarded true
    - api_assert: shutdown_reconciles_final_state_before_join true
    - cli_assert: standalone_watch_terminal_failure_closes_handles_in_reverse_and_exits_nonzero true
    - api_assert: watcher_refresh_adapter_maps_all_domain_fields true
    - api_assert: server_shutdown_preserves_background_primary_and_watcher_failure true

- case_id: STORE-008
  goal: 数据恢复策略。
  preconditions:
    - ledger 中存在可重建文档
  steps:
    - run: cargo test -p deve_cli recover_rebuilds_workspace_files_from_ledger -- --nocapture
    - run: cargo test -p deve_core rebuild_projection_recovers_when_node_projection_is_missing -- --nocapture
    - run: scripts/check-storage-repo-baseline.sh
  assertions:
    - cli_assert: projection_workspace_rebuilt_from_ledger true
    - cli_assert: projection_rebuild_uses_authority_facts true

- case_id: STORE-009
  goal: UUID-First Retrieval。
  preconditions:
    - 文档打开与历史读取使用 repo-scoped DocId
  steps:
    - run: cargo test -p deve_cli document_scope_bootstrap -- --nocapture
    - run: cargo test -p deve_cli open_doc_scope -- --nocapture
    - run: cargo test -p deve_cli resolve_target_prefers_doc_id_over_stale_path -- --nocapture
    - run: scripts/check-storage-repo-baseline.sh
  assertions:
    - cli_assert: open_doc_bootstraps_by_doc_id true
    - cli_assert: wrong_repo_doc_id_fails_closed true
    - cli_assert: path_hint_does_not_override_doc_id true

- case_id: STORE-010
  goal: 路径规范化。
  preconditions:
    - Windows 路径输入
  steps:
    - run: cargo test -p deve_core --test path_normalize_structure_test -- --nocapture
    - run: scripts/check-storage-repo-baseline.sh
  assertions:
    - cli_assert: persisted_paths_use_forward_slash true

- case_id: STORE-011
  goal: Browser storage authority boundary。
  preconditions:
    - WebLightPeer 运行在浏览器环境
  steps:
    - run: cargo test -p deve_web storage_capabilities -- --nocapture
    - run: cargo test -p deve_web browser_identity_capability -- --nocapture
    - run: cargo test -p deve_web typed_prefs_roundtrip -- --nocapture
    - run: cargo test -p deve_web shortcut_config_roundtrips -- --nocapture
    - run: cargo test -p deve_web locale_preference_uses_ui_prefs -- --nocapture
    - run: scripts/check-browser-prefs-boundary.sh
    - run: cargo run -p deve_baseline -- browser-prefs-boundary
    - run: cargo test -p deve_web output_write_classification -- --nocapture
  assertions:
    - WebCrypto_Ed25519_key_extractable_false: true
    - IndexedDB_missing_enters_DegradedSyncMode: true
    - WebCrypto_Ed25519_missing_has_typed_blocker: true
    - capability_probe_failure_is_not_UI_parsed_authority: true
    - ui_prefs_use_fallback_layer_only: true
    - ui_prefs_last_scope_stores_repo_name_alias_only: true
    - shortcut_prefs_new_writes_use_structured_json: true
    - degraded_mode_blocks_RegisterWriter_and_SyncPush: true
    - degraded_mode_allows_read_and_snapshot_pull: true

- case_id: STORE-012
  goal: Document structure WS scope gate。
  preconditions:
    - Browser session has current `scope_nonce`
  steps:
    - run: cargo test -p deve_cli docs_scope_nonce_gate -- --nocapture
    - run: scripts/check-repo-file-ops-baseline.sh
  assertions:
    - missing_scope_nonce_rejected_before_handler true
    - stale_scope_nonce_rejected_before_handler true
    - cli_assert: repo_file_ops_baseline_bound true

- case_id: STORE-013
  goal: Degraded projection 与 workspace ingestion readiness 统一阻断相关 local mutation。
  preconditions:
    - local repo 已被标记为 projection degraded
  steps:
    - run: cargo test -p deve_cli degraded_local -- --nocapture
    - run: cargo test -p deve_cli browser_writer_registration_rejects_broken_workspace_identity -- --nocapture
    - run: cargo test -p deve_core source_control_write_gate -- --nocapture
    - run: cargo test -p deve_cli mounted_repo_gate -- --nocapture
    - run: cargo test -p deve_cli watcher_failure -- --nocapture
    - run: cargo test -p deve_cli watcher_server_isolation -- --nocapture
    - run: scripts/check-repo-file-ops-baseline.sh
  assertions:
    - degraded_projection_blocks_docs_create_before_mutation: true
    - degraded_projection_blocks_edit_before_append: true
    - degraded_projection_blocks_RegisterWriter: true
    - degraded_projection_blocks_source_control_mutations: true
    - degraded_projection_blocks_merge_mutations: true
    - degraded_projection_blocks_http_source_control_mutations: true
    - degraded_projection_blocks_plugin_host_source_control_mutations: true
    - broken_workspace_identity_blocks_RegisterWriter: true
    - unmounted_repo_blocks_all_workspace_dependent_writers_with_storage_ingestion_unavailable: true
    - watcher_failure_does_not_write_projection_fault_or_degraded_projection: true
    - readonly_ledger_export_and_remote_shadow_ingest_remain_available: true
    - repo_local_watcher_failure_keeps_other_mounted_repo_writable: true
    - host_fatal_watcher_failure_rolls_back_all_started_handles_and_aborts_server: true
    - bootstrap_with_zero_mounted_repos_cleans_up_and_aborts_server: true
    - runtime_all_watchers_failed_keeps_readonly_and_diagnostics_available_with_degraded_health: true
    - cli_assert: repo_file_ops_baseline_bound true

- case_id: STORE-014
  goal: Ledger JSON Lines export。
  preconditions:
    - ledger 中存在 content 与 structure facts
  steps:
    - run: cargo test -p deve_cli jsonl_roundtrip_is_monotonic_and_line_stable -- --nocapture
    - run: cargo test -p deve_cli includes_dir_structure_fact_in_export -- --nocapture
    - run: scripts/check-storage-repo-baseline.sh
  assertions:
    - cli_assert: jsonl_rows_are_single_line true
    - cli_assert: jsonl_global_seq_monotonic true
    - cli_assert: jsonl_export_includes_structure_facts true

- case_id: STORE-014B
  goal: schema v2 只能通过显式离线只读导出，不得进入正常 runtime 或伪造 v3 来源。
  preconditions:
    - 临时目录中存在格式合法的 v2 repo fixture，server 未运行
  steps:
    - run: cargo test -p deve_cli legacy_v2_export -- --nocapture
    - run: deve export --allow-legacy-v2 --format json --out legacy-v2.jsonl
    - run: deve export --allow-legacy-v2 --format markdown --out legacy-v2-markdown
  assertions:
    - cli_assert: normal_v2_repo_open_fails_closed true
    - cli_assert: legacy_export_requires_explicit_flag true
    - cli_assert: legacy_export_is_read_only true
    - cli_assert: legacy_json_preserves_old_peer_and_seq_without_v3_attribution true
    - cli_assert: legacy_markdown_fails_closed_on_invalid_structure true

- case_id: STORE-014A
  goal: 本地 Repo 新增、重命名、安全移除与 watcher mount partial outcome。
  preconditions:
    - WebLightPeer 已认证并处于 local writable scope
    - 至少存在两个 local repo
  steps:
    - run: cargo test -p deve_cli create_repo -- --nocapture
    - run: cargo test -p deve_cli repo_lifecycle -- --nocapture
    - run: cargo test -p deve_cli repo_lifecycle_watcher_mount -- --nocapture
    - run: cargo test -p deve_web remove_current_fallback_failure_commits_no_scope -- --nocapture
    - run: cargo test -p deve_cli remove_current_invalidates_all_bound_sessions -- --nocapture
    - run: cargo test -p deve_web remove_partial_stage_is_connection_epoch_bounded -- --nocapture
    - run: cargo test -p deve_core --lib remove_local_repo_hides_it_without_deleting_authority_and_projection_workspace -- --nocapture
    - run: scripts/check-storage-repo-baseline.sh
    - chrome_mcp: 展开 repo switcher，点击顶部新增按钮创建 repo
    - chrome_mcp: 点击 repo 行更多菜单并执行重命名
    - chrome_mcp: 点击非当前 repo 行更多菜单并执行移除
    - chrome_mcp: 在 failure fixture 中分别执行 create/rename/remove，观察 mount partial outcome 与最终 scope/list publication
  assertions:
    - ui_assert: repo_switcher_create_button_visible true
    - ui_assert: repo_row_more_menu_contains_rename_and_remove true
    - ui_assert: renamed_repo_visible_and_bound true
    - ui_assert: removed_repo_hidden_from_normal_list true
    - cli_assert: removed_repo_authority_not_physically_deleted true
    - cli_assert: removed_repo_projection_workspace_not_physically_deleted true
    - cli_assert: create_mount_failure_keeps_repo_readonly_and_current_scope_unchanged true
    - cli_assert: rename_mount_failure_keeps_committed_name_and_readonly_scope true
    - cli_assert: rename_non_active_repo_does_not_change_current_scope true
    - cli_assert: remove_final_reconcile_precedes_removed_marker_and_locator_cleanup true
    - cli_assert: remove_precommit_failure_keeps_current_scope_and_restarts_old_watcher true
    - cli_assert: removed_membership_generation_changed_or_failed_fallback_is_not_bound true
    - cli_assert: unrelated_catalog_mutation_does_not_invalidate_fallback_token true
    - ws_assert: invalid_fallback_publishes_final_repo_list_then_protocol_error_sc_repo_not_selected_with_matching_switch_nonce true
    - ws_assert: invalid_fallback_does_not_emit_repo_switched true
    - ws_assert: invalid_fallback_repo_list_and_protocol_error_share_new_scope_nonce_equal_to_switch_nonce true
    - web_assert: pending_remove_buffers_final_repo_list_then_commits_no_scope_and_clears_pending_remove_intent_on_typed_error true
    - web_assert: invalid_fallback_partial_never_auto_selects_another_repo true
    - web_assert: old_repo_scope_messages_are_stale_after_no_scope_epoch_commit true
    - ws_assert: all_observer_sessions_bound_to_removed_repo_receive_distinct_per_connection_no_scope_epochs true
    - ws_assert: invalid_fallback_initiator_and_observers_use_distinct_per_connection_no_scope_epochs true
    - ws_assert: observer_invalidation_uses_protocol_error_without_initiator_switch_nonce true
    - web_assert: observer_invalidation_retires_old_scope_pending_switch_but_preserves_editor_pending_overlay true
    - web_assert: remove_partial_stage_has_one_slot_bound_to_connection_epoch_kind_and_nonce true
    - web_assert: disconnect_mismatch_second_stage_or_10s_timeout_discards_stage_and_retires_connection true
    - web_assert: old_connection_second_frame_cannot_commit_and_recovery_requires_explicit_repo_selection true
    - cli_assert: membership_revocation_cut_is_o1_and_session_fanout_runs_outside_catalog_permit true
    - cli_assert: old_binding_write_admission_fails_immediately_on_membership_generation_mismatch true
    - web_assert: first_repo_list_frame_immediately_closes_writer_ready_without_committing_no_scope true
    - web_assert: every_half_sequence_failure_preserves_editor_pending_overlay true
    - web_assert: remove_partial_deadline_uses_monotonic_clock true
    - cli_assert: mixed_lifecycle_truth_enters_repair_without_guessing_rollback true
    - ui_assert: lifecycle_publication_waits_for_final_mount_outcome true
    - cli_assert: final_e2_refresh_is_deferred_until_lifecycle_finalization true

- case_id: STORE-015
  goal: Writeback failure 后 Ledger Ack 仍成立。
  preconditions:
    - ledger append 已成功
    - workspace projection writeback 失败
  steps:
    - run: cargo test -p deve_cli edit_acknowledges_ledger_commit_when_workspace_writeback_fails -- --nocapture
    - run: cargo test -p deve_core --test durable_projection_fault_test -- --nocapture
    - run: scripts/check-storage-repo-baseline.sh
  assertions:
    - cli_assert: ack_sent_for_committed_ledger_edit true
    - cli_assert: writeback_failure_reported_as_protocol_error true
    - cli_assert: committed_client_op_index_persisted true

- case_id: STORE-016
  goal: Watcher bounded capture、level-triggered reconcile、overflow 与 debounce 边界。
  preconditions:
    - watcher backend 可返回 rescan batch
    - debounce window 可配置
    - W9 Windows producer 可在独立进程中注入或可靠触发 kernel overflow
  steps:
    - run: cargo test -p deve_core notify_backend_error_requests_rescan -- --nocapture
    - run: cargo test -p deve_core notify_rescan_flag_requests_rescan -- --nocapture
    - run: cargo test -p deve_core watcher_rejects_zero_debounce_window -- --nocapture
    - run: cargo test -p deve_core dispatch_batch_collapses_modified_burst_by_content_hash -- --nocapture
    - run: cargo test -p deve_core --lib watcher_bounded_capture -- --nocapture
    - run: cargo test -p deve_core --lib watcher_ignore_change_sets_reconcile_before_filter -- --nocapture
    - run: cargo test -p deve_core --lib watcher_cross_root_rename_sets_reconcile -- --nocapture
    - run: cargo test -p deve_core --test watcher_platform_fs watcher_atomic_replace_records_single_final_candidate -- --nocapture
    - run: cargo run -p deve_baseline -- acceptance-run --producer storage.watcher-windows-overflow
    - run: scripts/check-storage-repo-baseline.sh
  assertions:
    - cli_assert: watcher_backend_error_triggers_rescan true
    - cli_assert: watcher_rescan_flag_triggers_rescan true
    - cli_assert: zero_debounce_rejected true
    - cli_assert: modify_burst_collapses_to_single_pending_entry true
    - api_assert: atomic_replace_has_one_final_pending_candidate true
    - api_assert: notify_debounced_event_does_not_cross_backend_adapter true
    - api_assert: queue_is_16_batches_and_batch_limits_are_256_hints_256_kib_paths true
    - api_assert: queue_full_oversized_backend_rescan_and_cross_root_rename_set_one_level_latch true
    - api_assert: raw_events_are_not_cached_or_replayed_during_capture_or_reconcile true
    - api_assert: reconcile_latch_clears_only_after_clean_full_reconcile true
    - api_assert: deveignore_change_sets_dirty_before_semantic_filter true
    - receipt_assert: windows_overflow_runs_in_three_independent_processes_with_callback_barrier_and_2048_file_burst true
    - receipt_assert: overflow_maps_to_rescan_and_backend_delivers_normal_events_after_rearm true
    - receipt_assert: reconciled_pending_set_matches_independent_expected_hash true
    - receipt_assert: evidence_binds_dependency_source_revision_windows_build_filesystem_and_exact_head true
    - receipt_assert: watcher_windows_linux_real_fs_and_chrome_lifecycle_evidence_bind_same_head true

- case_id: STORE-017
  goal: Repo catalog hard fail / quarantine。
  preconditions:
    - local/remote repo catalog 可被测试夹具破坏
    - repair 命令可运行在临时 ledger
  steps:
    - run: cargo test -p deve_core remote_repo_catalog_calls_fail_closed_when_remotes_dir_is_missing -- --nocapture
    - run: cargo test -p deve_core remote_repo_listing_fails_closed_on_unexpected_non_redb_entry -- --nocapture
    - run: cargo test -p deve_cli quarantines_nil_shadow_repo_into_invalid_peer_dir -- --nocapture
    - run: scripts/check-storage-repo-baseline.sh
  assertions:
    - cli_assert: missing_remote_catalog_fails_closed true
    - cli_assert: unexpected_remote_catalog_entry_fails_closed true
    - cli_assert: nil_shadow_repo_quarantined_not_deleted true

- case_id: STORE-018
  goal: Projection Backup push 只上传 Markdown Projection Workspace files，且 provider 结果不产生 authority effect。
  preconditions:
    - local Projection Workspace 已通过 Projection Locator 与 `.notegit` identity marker gate
    - workspace 中同时存在 Markdown files 与 internal/reserved paths
  steps:
    - run: cargo test -p deve_cli --lib run_webdav_push_uses_webdav_provider_after_workspace_gate -- --nocapture
    - run: cargo test -p deve_cli --lib run_s3_push_uses_s3_provider_after_workspace_gate -- --nocapture
    - run: cargo test -p deve_cli --lib collect_markdown_projection_files_uploads_only_markdown_projection_files -- --nocapture
  assertions:
    - cli_assert: projection_backup_push_uploads_markdown_files_only true
    - cli_assert: projection_backup_push_skips_internal_reserved_paths true
    - cli_assert: projection_backup_push_writes_no_ledger_source_control_or_git_mirror true

- case_id: STORE-019
  goal: Projection Backup pull 只覆盖 Projection Workspace，并经 External Changes admission。
  preconditions:
    - remote provider 返回 Markdown object set
    - target local repo 是当前 writable local repo
  steps:
    - run: cargo test -p deve_cli --lib run_webdav_pull_scans_written_files_into_external_changes -- --nocapture
    - run: cargo test -p deve_cli --lib run_s3_pull_scans_written_files_into_external_changes -- --nocapture
    - run: cargo test -p deve_core --lib fake_adapter_pull_returns_external_changes_candidate_only -- --nocapture
  assertions:
    - cli_assert: projection_backup_pull_overwrites_projection_workspace true
    - cli_assert: projection_backup_pull_surfaces_external_changes true
    - cli_assert: projection_backup_pull_does_not_stage_commit_or_write_ledger true

- case_id: STORE-020
  goal: Projection Backup provider metadata 与 unsafe remote paths 不得成为 authority。
  preconditions:
    - provider may expose ETag/mtime/object version/listing order
    - remote may contain malformed, duplicate, non-Markdown, or unsafe paths
  steps:
    - run: cargo test -p deve_cli --lib run_rejects_provider_authority_effects_before_success_report -- --nocapture
    - run: cargo test -p deve_cli --lib run_rejects_authoritative_provider_metadata_before_success_report -- --nocapture
    - run: cargo test -p deve_cli --lib run_rejects_duplicate_pull_paths_before_workspace_write -- --nocapture
    - run: cargo test -p deve_core --lib provider_request_rejects_duplicate_paths -- --nocapture
  assertions:
    - cli_assert: projection_backup_provider_metadata_diagnostic_only true
    - cli_assert: projection_backup_rejects_provider_authority_effects true
    - cli_assert: projection_backup_rejects_duplicate_remote_paths_before_workspace_write true
    - cli_assert: projection_backup_rejects_unsafe_or_internal_projection_paths true

- case_id: STORE-021
  goal: Projection Backup pull 必须在 workspace 写入前校验 provider contract 与 resource budget；External Changes 用户确认发生在 workspace overwrite + watcher/scan 之后。
  preconditions:
    - provider outcome declares whether pull writes only Projection Workspace and requires later External Changes admission
    - file count, single file size, and aggregate size budgets are configured in provider adapters
  steps:
    - run: cargo test -p deve_cli --lib run_rejects_pull_without_external_changes_confirmation_before_workspace_write -- --nocapture
    - run: cargo test -p deve_cli --lib run_rejects_pull_without_projection_workspace_overwrite_before_workspace_write -- --nocapture
    - run: cargo test -p deve_cli --lib webdav_pull_rejects_oversized_file_before_workspace_write -- --nocapture
    - run: cargo test -p deve_cli --lib s3_pull_rejects_oversized_file_before_workspace_write -- --nocapture
  assertions:
    - cli_assert: projection_backup_pull_declares_later_external_changes_admission true
    - cli_assert: projection_backup_pull_requires_projection_workspace_overwrite_contract true
    - cli_assert: projection_backup_pull_budget_failures_happen_before_workspace_write true

- case_id: STORE-022
  goal: Projection Backup S3-compatible endpoint 只允许显式 Remote Projection profile binding；未绑定或不匹配时 fail-closed。
  preconditions:
    - locator uses `s3+https://` custom endpoint
    - ADR 0008 Remote Projection profile binding is either absent, mismatched, or explicitly supplied by CLI profile handle
  steps:
    - run: cargo test -p deve_cli --lib s3_custom_https_endpoint_requires_explicit_credential_binding -- --nocapture
    - run: cargo test -p deve_cli --lib s3_custom_https_endpoint_fails_before_workspace_file_read -- --nocapture
    - run: cargo test -p deve_cli --lib s3_custom_https_endpoint_direct_push_fails_before_credentials_resolve -- --nocapture
    - run: cargo test -p deve_cli --lib s3_custom_https_endpoint_direct_pull_fails_before_credentials_resolve -- --nocapture
    - run: cargo test -p deve_cli --lib s3_custom_https_endpoint_push_uses_explicit_profile_binding -- --nocapture
    - run: cargo test -p deve_cli --lib s3_custom_https_endpoint_profile_env_ref_is_not_default_aws_fallback -- --nocapture
  assertions:
    - cli_assert: projection_backup_s3_custom_endpoint_requires_profile_binding true
    - cli_assert: projection_backup_s3_custom_endpoint_fails_before_default_credentials true
    - cli_assert: projection_backup_s3_custom_endpoint_fails_before_provider_io true
    - cli_assert: projection_backup_s3_custom_endpoint_explicit_profile_can_enable_cli_io true

- case_id: STORE-023
  goal: Projection Backup 首版 gate 聚合 Remote Projection transport 文档与测试证据。
  preconditions:
    - STORE-018..STORE-022 已列出 Projection Backup 自动化验证命令
    - ledger backup pack / RestoreCandidate / explicit-import / explicit-merge 不属于首版 Backup contract
  steps:
    - run: cargo run -p deve_baseline -- backup
  assertions:
    - cli_assert: projection_backup_baseline_aggregates_remote_projection_transport_checks true
    - cli_assert: projection_backup_baseline_excludes_ledger_pack_restore_checks true

- case_id: STORE-024
  goal: Projection Workspace child path 与 existing ancestor containment fail-closed。
  preconditions:
    - local repo 已绑定 Projection Locator
    - 测试夹具可在 Projection Workspace 内创建目录与符号链接；Windows 权限不足时只跳过 symlink 分支
  steps:
    - run: cargo test -p deve_core --lib projection_workspace_child_path -- --nocapture
    - run: cargo test -p deve_core --lib projection_workspace_existing_ancestor -- --nocapture
    - run: cargo test -p deve_core --test materialize_projection_test -- --nocapture
    - run: scripts/check-storage-repo-baseline.sh
  assertions:
    - cli_assert: projection_workspace_accepts_canonical_forward_slash_relative_path true
    - cli_assert: projection_workspace_rejects_absolute_traversal_empty_internal_and_noncanonical_paths true
    - cli_assert: projection_workspace_existing_ancestor_stays_within_canonical_root true
    - cli_assert: projection_workspace_external_or_dangling_symlink_fails_closed true
    - cli_assert: projection_materialize_and_rematerialize_remain_ledger_derived true

```
