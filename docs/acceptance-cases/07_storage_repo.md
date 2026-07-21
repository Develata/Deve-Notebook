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
  goal: 重复 host-local alias 不改变 RepoId 或 workspace identity，歧义 alias selector fail-closed。
  preconditions:
    - 两个 Repo 同名不同 URL
  steps:
    - run: cargo test -p deve_core init_keeps_duplicate_display_name_for_same_name_different_url -- --nocapture
    - run: cargo test -p deve_cli --lib select_target_repo_fails_closed_on_ambiguous_local_alias -- --nocapture
    - run: scripts/check-storage-repo-baseline.sh
  assertions:
    - cli_assert: collision_safe_workspace_segment true
    - cli_assert: repo_identity_not_changed_by_physical_suffix true
    - cli_assert: ambiguous_host_alias_selector_fails_closed true

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
    - run: cargo test -p deve_core --test watcher_platform_fs watcher_final_state_shutdown_captures_unflushed_change_and_stops_callbacks -- --nocapture --test-threads=1
    - api_assert: final_state_shutdown_orders_stop_discard_reconcile_refresh true
    - api_assert: worker_failure_primary_survives_stop_and_final_scan_cleanup true
    - api_assert: final_state_shutdown_revalidates_exact_repo_id_and_root true
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
    - ui_prefs_last_scope_stores_repo_id_recovery_hint_without_alias_or_writer_grant: true
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
  goal: 本地 Repo 新增、host-local alias、安全移除与 watcher mount partial outcome。
  preconditions:
    - WebLightPeer 已认证
    - fixture 可分别启动 zero/one/multi local repo host
  steps:
    - run: cargo test -p deve_cli create_repo -- --nocapture
    - run: cargo test -p deve_cli repo_lifecycle -- --nocapture
    - run: cargo test -p deve_cli repo_lifecycle_watcher_mount -- --nocapture
    - run: cargo test -p deve_cli repo_lifecycle_remove_e2_failure_restarts_old_watcher -- --nocapture
    - run: cargo test -p deve_core --lib authority_storage_runtime::local_authority::tests:: -- --nocapture
    - run: cargo test -p deve_core --test local_repo_routing_test -- --nocapture
    - gap: R1 per-RepoId authority ownership and R2 zero-repo NoScope/configured first Create are implemented; dedicated REPO_CREATION_PROJECTION_BASE_REQUIRED wire projection and R3-R6 F4/v5 preview-token removal, owned cleanup/repair, thin UI and fresh cross-process evidence are not implemented yet
    - run: cargo test -p deve_core --lib local_authority_runtime_retires_bootstrap_and_secondary_with_identical_semantics -- --nocapture
    - run: cargo test -p deve_cli --test zero_repo_server_runtime_test -- --nocapture
    - run: cargo test -p deve_cli --lib zero_repo_host_starts_no_scope_and_creates_from_configured_base -- --nocapture
    - run: cargo test -p deve_cli --lib zero_repo_create_without_projection_base_is_typed_before_ws_v5_projection -- --nocapture
    - run: cargo test -p deve_cli --lib remove_last_then_create_uses_new_durable_membership_for_default_reads -- --nocapture
    - run: cargo test -p deve_cli --lib queued_create_rejects_projection_base -- --nocapture
    - run: cargo test -p deve_cli --lib prepare_local_repo_removal_reissues_and_invalidates_confirmation_token -- --nocapture
    - run: cargo test -p deve_cli --lib execute_local_repo_removal_rejects_expired_stale_and_wrong_issuer_token -- --nocapture
    - run: cargo test -p deve_cli --lib execute_local_repo_removal_retry_returns_existing_job_or_result -- --nocapture
    - run: cargo test -p deve_cli --lib execute_local_repo_removal_atomically_persists_admission_before_worker -- --nocapture
    - run: cargo test -p deve_cli --lib web_removal_token_binds_principal_connection_and_server_incarnation -- --nocapture
    - run: cargo test -p deve_cli --lib offline_removal_token_survives_two_cli_invocations_only_for_exact_authority_identity -- --nocapture
    - run: cargo test -p deve_core --lib remove_local_repo_deletes_owned_state_and_preserves_workspace_content -- --nocapture
    - run: cargo test -p deve_cli --lib remove_local_repo_cleanup_failure_is_exactly_repairable -- --nocapture
    - run: cargo test -p deve_core --lib remove_local_repo_retires_bootstrap_and_secondary_db_leases -- --nocapture
    - run: cargo test -p deve_core --lib remove_local_repo_fails_closed_while_another_process_holds_authority -- --nocapture
    - run: cargo test -p deve_core --lib persistent_authority_lock_path_survives_removal_and_serializes_readmission -- --nocapture
    - run: cargo test -p deve_core --lib readmitted_repo_id_replaces_retired_slot_with_new_generation -- --nocapture
    - run: cargo test -p deve_core --lib concurrent_same_repo_id_readmission_uses_one_reopening_reservation -- --nocapture
    - run: cargo test -p deve_core --lib remove_local_repo_rejects_notegit_symlink_and_reparse_escape -- --nocapture
    - run: cargo test -p deve_core --lib remove_local_repo_no_follow_walker_never_enters_child_link_targets -- --nocapture
    - run: cargo test -p deve_cli --lib remove_local_repo_restart_classifies_each_durable_cut_from_exact_manifest -- --nocapture
    - run: cargo test -p deve_cli --lib remove_local_repo_manifest_precedes_tombstone_and_cleanup_complete_precedes_retirement -- --nocapture
    - run: cargo test -p deve_cli --lib remove_local_repo_old_request_cannot_remove_readmitted_membership -- --nocapture
    - run: cargo test -p deve_cli --lib remove_local_repo_remote_import_owner_state_matrix -- --nocapture
    - run: cargo test -p deve_cli --lib removal_repair_dry_run_and_explicit_apply_reauthorize_only_proven_drift -- --nocapture
    - run: cargo test -p deve_cli --lib removal_repair_rejects_replaced_notegit_and_redb_even_with_matching_repo_id_text -- --nocapture
    - run: cargo test -p deve_cli --lib removal_prepare_blocks_overlapping_operator_recovery_input -- --nocapture
    - run: cargo test -p deve_cli --lib removal_pre_cut_failure_compensates_provider_watcher_authority_and_transition -- --nocapture
    - run: cargo test -p deve_cli --lib removal_provider_resume_failure_remains_typed_readonly_repair -- --nocapture
    - run: cargo test -p deve_cli --lib removal_terminal_result_releases_lock_before_publication_delivery -- --nocapture
    - run: scripts/check-storage-repo-baseline.sh
    - chrome_mcp: 展开 repo switcher，点击顶部新增按钮创建 repo
    - chrome_mcp: 点击 repo 行更多菜单并修改本机 alias
    - chrome_mcp: 点击 repo 行更多菜单，检查 backend preview 后确认移除
    - chrome_mcp: 移除最后一个 repo，验证 NoScope 空状态，重启后创建首个 repo
    - chrome_mcp: 在 failure fixture 中分别执行 create/remove 与 alias store failure，观察最终 scope/list/display publication
  assertions:
    - ui_assert: repo_switcher_create_button_visible true
    - ui_assert: repo_row_more_menu_contains_alias_and_remove true
    - ui_assert: alias_change_preserves_repo_id_scope_and_workspace true
    - ui_assert: removed_repo_hidden_from_normal_list true
    - cli_assert: removed_repo_local_redb_not_present true
    - cli_assert: removed_repo_notegit_not_present true
    - cli_assert: remote_import_owner_returns_typed_removal_plan_and_performs_cleanup true
    - cli_assert: removed_repo_locator_and_alias_not_present true
    - cli_assert: removed_repo_workspace_markdown_attachments_and_git_preserved true
    - cli_assert: removed_repo_remote_shadows_and_nonoverlapping_operator_recovery_input_preserved true
    - cli_assert: removed_repo_catalog_tombstone_absent_after_successful_cleanup true
    - cli_assert: cleanup_failure_retains_exact_manifest_tombstone_and_repair_debt true
    - cli_assert: cleanup_debt_receipt_is_not_pruned_before_tombstone_retirement true
    - cli_assert: alias_locator_and_catalog_conditional_delete_preserve_other_repo_rows true
    - cli_assert: bootstrap_and_secondary_repo_handle_retirement_have_identical_semantics true
    - cli_assert: external_process_authority_holder_blocks_redb_delete_on_windows_and_unix true
    - cli_assert: notegit_top_level_symlink_junction_reparse_or_identity_replacement_fails_closed true
    - cli_assert: notegit_child_link_entry_is_deleted_without_following_or_touching_external_target true
    - cli_assert: manifest_parent_identity_and_containment_drift_fail_closed true
    - cli_assert: replaced_redb_requires_original_file_and_membership_genesis_identity_and_is_never_auto_applied true
    - cli_assert: overlapping_operator_recovery_input_blocks_prepare true
    - cli_assert: remote_import_pending_and_degraded_are_blocked_while_safe_states_are_owner_cleaned true
    - cli_assert: old_remove_request_cannot_affect_readmitted_same_repo_id true
    - cli_assert: confirmation_token_is_256_bit_hashed_five_minute_single_use_and_membership_bound true
    - cli_assert: execute_admission_atomically_consumes_token_and_persists_request_job_before_worker true
    - cli_assert: online_token_is_principal_connection_and_server_incarnation_bound_and_memory_only true
    - cli_assert: offline_token_is_authority_root_lock_identity_and_preparation_bound_not_process_bound true
    - cli_assert: repeated_prepare_invalidates_previous_token true
    - cli_assert: exact_execute_retry_is_idempotent_after_lost_response true
    - ws_assert: websocket_protocol_is_f4_v5_and_direct_remove_intent_is_absent true
    - ui_assert: remove_confirmation_states_irreversible_no_ledger_restore_and_workspace_git_preserved true
    - ui_assert: removal_preview_uses_backend_categories_and_never_exposes_path_digest_or_manifest true
    - ui_assert: last_repo_removal_commits_no_scope_without_error true
    - ui_assert: zero_repo_restart_keeps_login_diagnostic_and_create_available true
    - ui_assert: zero_repo_create_without_base_renders_repo_creation_projection_base_required true
    - cli_assert: create_mount_failure_keeps_repo_readonly_and_current_scope_unchanged true
    - cli_assert: alias_set_never_stops_watcher_or_moves_workspace true
    - cli_assert: alias_import_unknown_invalid_duplicate_entries_warn_skip_and_summarize true
    - cli_assert: alias_import_valid_entries_commit_as_one_atomic_batch true
    - cli_assert: alias_store_commit_failure_is_global_error true
    - cli_assert: peer_sync_and_remote_import_never_transmit_alias true
    - cli_assert: remove_final_reconcile_precedes_removed_marker_and_locator_cleanup true
    - cli_assert: remove_precommit_failure_keeps_current_scope_and_restarts_old_watcher true
    - cli_assert: stale_optional_fallback_never_binds_and_does_not_downgrade_successful_removal true
    - web_assert: no_fallback_commits_no_scope_and_clears_pending_remove_intent_on_typed_success true
    - web_assert: stale_optional_fallback_never_auto_selects_another_repo true
    - web_assert: old_repo_scope_messages_are_stale_after_no_scope_epoch_commit true
    - ws_assert: all_observer_sessions_bound_to_removed_repo_receive_distinct_per_connection_no_scope_epochs true
    - ws_assert: invalid_fallback_initiator_and_observers_use_distinct_per_connection_no_scope_epochs true
    - ws_assert: observer_invalidation_uses_typed_repo_control_finalization_without_initiator_request_id true
    - web_assert: observer_invalidation_retires_old_scope_pending_switch_but_preserves_editor_pending_overlay true
    - web_assert: prepare_and_execute_responses_are_bound_to_connection_epoch_request_and_scope true
    - web_assert: single_typed_finalization_atomically_carries_final_repo_list_and_scope true
    - web_assert: removal_success_is_never_inferred_from_sc_repo_not_selected_or_two_frame_order true
    - ws_assert: optional_fallback_is_user_selected_prepare_bound_opaque_and_never_backend_auto_selected true
    - web_assert: disconnect_or_mismatch_discards_preview_and_token_without_canceling_admitted_job true
    - web_assert: old_connection_response_cannot_commit_ui_state_and_recovery_requires_fresh_prepare true
    - cli_assert: membership_revocation_cut_is_o1_and_session_fanout_runs_outside_catalog_permit true
    - cli_assert: old_binding_write_admission_fails_immediately_on_membership_generation_mismatch true
    - web_assert: removal_publication_closes_writer_ready_and_commits_per_connection_no_scope_epoch true
    - web_assert: every_publication_failure_preserves_editor_pending_overlay true
    - web_assert: frontend_does_not_compute_confirmation_ttl_or_blockers true
    - cli_assert: mixed_lifecycle_truth_enters_repair_without_guessing_rollback true
    - ui_assert: lifecycle_publication_waits_for_final_mount_outcome true
    - cli_assert: final_e2_refresh_is_deferred_until_lifecycle_finalization true
    - cli_assert: provider_quiesce_and_watcher_e2_precede_authority_quiescing_and_removed_cut true
    - cli_assert: persistent_authority_lock_path_is_not_a_cleanup_target true
    - cli_assert: readmitted_same_repo_id_uses_new_slot_generation_and_rejects_old_capabilities true
    - cli_assert: remote_import_cleanup_is_sealed_before_quiescing_and_artifact_only_after_removed_cut true
    - cli_assert: destructive_manifest_has_no_repo_runtime_catch_all true
    - cli_assert: pre_cut_compensation_restores_provider_generation_invalidates_owner_plan_and_releases_transition true
    - cli_assert: failed_pre_cut_compensation_remains_readonly_repair_not_active true
    - cli_assert: durable_terminal_result_fsync_precedes_lock_release_and_best_effort_publication true
    - cli_assert: concurrent_same_repo_id_readmission_has_one_reopening_reservation_and_generation true

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
    - core_assert: durable_projection_fault_is_repo_local_redb_v4_side_table true
    - core_assert: host_wide_projection_fault_toml_is_absent true
    - core_assert: durable_projection_fault_survives_restart_and_clears_only_after_repair true

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
  goal: Remote Projection push 只上传 Markdown Projection Workspace files，且 provider 结果不产生 authority effect。
  preconditions:
    - local Projection Workspace 已通过 Projection Locator 与 `.notegit` identity marker gate
    - workspace 中同时存在 Markdown files 与 internal/reserved paths
  steps:
    - run: cargo test -p deve_cli --lib run_webdav_push_uses_webdav_provider_after_workspace_gate -- --nocapture
    - run: cargo test -p deve_cli --lib run_s3_push_uses_s3_provider_after_workspace_gate -- --nocapture
    - run: cargo test -p deve_cli --lib collect_markdown_projection_files_uploads_only_markdown_projection_files -- --nocapture
  assertions:
    - cli_assert: remote_projection_push_uploads_markdown_files_only true
    - cli_assert: remote_projection_push_skips_internal_reserved_paths true
    - cli_assert: remote_projection_push_writes_no_ledger_source_control_or_git_mirror true

- case_id: STORE-019
  goal: Remote Import Prepare 将远端输入封存为 immutable manifest/blob/candidate，不预写 workspace 或 authority。
  preconditions:
    - B1 已实现 durable session store 与 host-only artifact publication
    - B4 已接入 provider-bound product Prepare 与 bounded sink contract
  steps:
    - run: cargo test -p deve_core --lib remote_import -- --nocapture
    - run: cargo test -p deve_cli --lib source_acquisition_delivers_normalized_paths_in_order -- --nocapture
    - gap: B6 provider-bound fresh Remote Import prepare receipt is not sealed yet
  assertions:
    - api_assert: remote_import_prepare_captures_deterministic_manifest_v1 true
    - api_assert: remote_import_prepare_writes_no_workspace_ledger_or_external_changes true
    - api_assert: remote_import_prepare_enforces_single_active_session_and_resource_budgets true

- case_id: STORE-020
  goal: Remote Import Review 只投影 backend-generated change kind、typed blocker、entry_id 与 display label。
  preconditions:
    - STORE-019 已产生 immutable session candidate
    - B4 backend review/blocker/diff 与 CLI/WS product API 已实现
  steps:
    - run: cargo test -p deve_core --lib remote_import::facade::tests -- --nocapture
    - gap: B5 thin client and B6 fresh Remote Import review receipt are not sealed yet
  assertions:
    - api_assert: remote_import_review_is_backend_owned true
    - api_assert: remote_import_review_exposes_no_locator_blob_digest_credential_or_raw_failure_detail true
    - api_assert: remote_import_any_blocker_disables_whole_session_apply true

- case_id: STORE-021
  goal: Remote Import Apply 以 whole-session sealed transaction exactly-once 写 Ledger，提交后才 writeback Projection。
  preconditions:
    - Ready session 无 blocker，repo health healthy 且 watcher Mounted
    - B4 已接入 Mounted product gate、current locator/ignore producer 与 post-commit materialization/startup recovery orchestration
  steps:
    - run: cargo test -p deve_core --lib remote_import::tests::apply -- --nocapture
    - run: cargo test -p deve_core --lib remote_import::facade::tests -- --nocapture
    - gap: B5 browser journey and B6 fresh Remote Import apply receipt are not sealed yet
  assertions:
    - api_assert: remote_import_apply_revalidates_session_revision_head_scope_and_overlap_in_one_transaction true
    - api_assert: remote_import_apply_failure_leaves_no_fact_prefix true
    - api_assert: remote_import_apply_lost_response_returns_stored_receipt true
    - api_assert: remote_import_apply_replay_survives_cleanup_and_new_active_session true
    - api_assert: remote_import_apply_tamper_transitions_to_failed_repair true
    - api_assert: remote_import_apply_pending_projection_outcome_recovers_without_reappend true
    - api_assert: remote_import_writeback_failure_does_not_roll_back_ledger true
    - api_assert: remote_import_fault_origin_binds_session_revision_and_request true
    - api_assert: remote_import_fault_and_degraded_receipt_share_one_transaction true
    - api_assert: failed_projection_settlement_keeps_pending_without_fault true
    - api_assert: projection_settlement_retry_does_not_reappend_ledger true

- case_id: STORE-022
  goal: Remote Projection transport 的 push/source acquisition 只允许显式 S3-compatible profile binding；未绑定或不匹配时 fail-closed。
  preconditions:
    - locator uses `s3+https://` custom endpoint
    - ADR 0008 Remote Projection profile binding is either absent, mismatched, or explicitly supplied by CLI profile handle
  steps:
    - run: cargo test -p deve_cli --lib s3_custom_https_endpoint_requires_explicit_credential_binding -- --nocapture
    - run: cargo test -p deve_cli --lib s3_custom_https_endpoint_fails_before_workspace_file_read -- --nocapture
    - run: cargo test -p deve_cli --lib s3_custom_https_endpoint_direct_push_fails_before_credentials_resolve -- --nocapture
    - run: cargo test -p deve_cli --lib s3_custom_https_endpoint_source_acquisition_fails_before_credentials_resolve -- --nocapture
    - run: cargo test -p deve_cli --lib s3_custom_https_endpoint_push_uses_explicit_profile_binding -- --nocapture
    - run: cargo test -p deve_cli --lib s3_custom_https_endpoint_profile_env_ref_is_not_default_aws_fallback -- --nocapture
  assertions:
    - cli_assert: projection_backup_s3_custom_endpoint_requires_profile_binding true
    - cli_assert: projection_backup_s3_custom_endpoint_fails_before_default_credentials true
    - cli_assert: projection_backup_s3_custom_endpoint_fails_before_provider_io true
    - cli_assert: projection_backup_s3_custom_endpoint_explicit_profile_can_enable_cli_io true

- case_id: STORE-023
  goal: Remote Import Refresh/Discard/Repair/retention/cleanup 生命周期具有真实 producer 证据。
  preconditions:
    - B1 已实现 durable recovery、retention 与 dry-run repair inventory
    - B4已接入产品Refresh/Discard/Repair；R4 removal owner-plan与B6 fresh cross-platform evidence仍待实现
  steps:
    - run: cargo test -p deve_core --lib remote_import -- --nocapture
    - gap: R4 removal owner-plan and B6 fresh Remote Import manage receipt are not sealed yet
  assertions:
    - cli_assert: remote_import_refresh_uses_sealed_blobs_only true
    - cli_assert: remote_import_discard_and_repair_are_explicit true
    - cli_assert: remote_import_repair_defaults_to_dry_run true
    - api_assert: cleanup_pending_is_never_auto_pruned true

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
