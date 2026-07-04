## Diff 与合并

```markdown
- case_id: DIFF-001
  goal: UTF-16 索引一致。
  preconditions:
    - 文档包含 emoji: "A😀B"
  steps:
    - ui_place_cursor_after: "A😀"
    - ui_type: "X"
    - run: cargo test -p deve_core compute_diff_uses_utf16_positions -- --nocapture
  assertions:
    - api_assert: utf16_positions_match_editor_offsets true

- case_id: DIFF-002
  goal: 3-Way Merge 使用 LCA。
  preconditions:
    - Local 与 Remote 均基于同一 Base 修改
    - 用户当前处于 Local Branch，并显式选择 peer force-mirror 作为只读 source
  steps:
    - ui_command: "P2P: Merge Peer"
    - run: cargo test -p deve_cli merge_scope_nonce_gate -- --nocapture
    - run: cargo test -p deve_cli merge_manual_write_readonly_gate -- --nocapture
    - run: cargo test -p deve_cli merge_peer_local_branch_contract -- --nocapture
  assertions:
    - log_contains: "LCA"
    - api_assert: merge_manual_writes_reject_remote_readonly true
    - api_assert: merge_peer_writes_local_branch_only true
    - api_assert: merge_peer_rejects_remote_branch_scope true

- case_id: DIFF-003
  goal: 冲突检测按 Hunk 触发。
  preconditions:
    - Local 与 Remote 修改同一段落
  steps:
    - ui_command: "P2P: Merge Peer"
    - run: cargo test -p deve_cli resolve_merge_conflict -- --nocapture
  assertions:
    - ws_message: "MergeConflict"
    - ws_field_contains: "actions AcceptCurrent AcceptIncoming AcceptBoth"
    - ui_assert: conflict_view_open true

- case_id: DIFF-004
  goal: 冲突 UI 三种策略。
  preconditions:
    - 已进入冲突界面
  steps:
    - ui_click: "[data-deve-merge-action='accept-current']"
    - ui_click: "[data-deve-merge-action='accept-incoming']"
    - ui_click: "[data-deve-merge-action='accept-both']"
    - run: cargo test -p deve_cli resolve_merge_conflict_accept_current -- --nocapture
    - run: cargo test -p deve_cli resolve_merge_conflict_local_branch_contract -- --nocapture
    - run: cargo test -p deve_cli resolve_merge_conflict_accept_both -- --nocapture
  assertions:
    - ws_message: "ResolveMergeConflict"
    - ws_field_contains: "doc_id action result_content scope_nonce"
    - ui_assert: result_matches_strategy true
    - api_assert: resolve_merge_conflict_accept_current_local_only true
    - api_assert: resolve_merge_conflict_writes_local_branch_only true
    - api_assert: resolve_merge_conflict_accept_both_result_local_only true

- case_id: DIFF-005
  goal: 合并 state 重开可重放。
  preconditions:
    - Peer merge conflict 已产生但未确认
  steps:
    - run: cargo test -p deve_cli merge_peer_conflict_replays_after_state_reopen -- --nocapture
  assertions:
    - api_assert: merge_peer_local_remote_ledger_entries_unchanged_after_state_reopen true
    - api_assert: merge_peer_conflict_replay_after_state_reopen true

- case_id: DIFF-006
  goal: Watcher 防抖与 Hash 校验。
  preconditions:
    - `deve watch` 运行中
  steps:
    - run: powershell -Command "1..5 | % { 'x' | Add-Content ${DEVE_DATA_DIR}/notes/default/debounce.md }"
    - run: cargo test -p deve_core dispatch_batch_collapses_modified_burst_by_content_hash -- --nocapture
  assertions:
    - api_assert: watcher_burst_pending_fs_ops_single_entry true
    - api_assert: watcher_burst_pending_hash_matches_final_content true
    - api_assert: watcher_burst_does_not_append_ledger_ops true

- case_id: DIFF-007
  goal: Diff 颜色语义。
  preconditions:
    - 文档包含新增/修改/删除
  steps:
    - ui_open_diff: true
    - run: scripts/check-diff-color-baseline.sh
  assertions:
    - ui_assert: gutter_color_added "var(--color-added)"
    - ui_assert: gutter_color_modified "var(--color-modified)"
    - ui_assert: gutter_color_deleted "var(--color-deleted)"

- case_id: DIFF-008
  goal: 长文档打开策略。
  preconditions:
    - 文档含快照与大量 Ops
  steps:
    - ui_open_doc: "large.md"
    - run: scripts/check-large-doc-baseline.sh
    - run: cargo test -p deve_web large_doc_search_gate -- --nocapture
  assertions:
    - ui_assert: snapshot_first true
    - ui_assert: search_disabled_until_prefetch_complete true

- case_id: DIFF-009
  goal: Source Control 当前入口与 Git Command Palette CLI-only notice 保持分层。
  preconditions:
    - Source Control 面板可用
  steps:
    - run: scripts/check-source-control-baseline.sh
    - run: cargo test -p deve_cli source_control -- --nocapture
    - run: cargo test -p deve_web commit_write_block -- --nocapture
    - run: cargo test -p deve_web commit_refresh -- --nocapture
    - run: cargo test -p deve_web commit_ack_dispatch -- --nocapture
    - run: cargo test -p deve_web commit_shortcut -- --nocapture
    - run: cargo test -p deve_web fs_refresh -- --nocapture
    - run: cargo test -p deve_web read_list_dispatch -- --nocapture
    - run: cargo test -p deve_web doc_diff_read_gate -- --nocapture
    - run: cargo test -p deve_web doc_diff_dispatch -- --nocapture
    - run: cargo test -p deve_web commit_diff_read_gate -- --nocapture
    - run: cargo test -p deve_web commit_diff_dispatch -- --nocapture
    - run: cargo test -p deve_cli source_control_scope_nonce_gate -- --nocapture
    - run: cargo test -p deve_cli commit_and_push_ws_returns_cli_only_blocker_without_commit -- --nocapture
    - run: cargo test -p deve_cli readonly_remote_doc_diff_uses_shadow_projection -- --nocapture
    - run: cargo test -p deve_cli readonly_remote_doc_diff_missing_target -- --nocapture
    - run: cargo test -p deve_cli readonly_remote_doc_diff_path_mismatch -- --nocapture
    - run: cargo test -p deve_cli readonly_remote_source_control_writes -- --nocapture
    - run: cargo test -p deve_cli readonly_remote_commit_diff_is_allowed -- --nocapture
    - run: cargo test -p deve_cli readonly_remote_commit_diff_reports_rename_projection -- --nocapture
    - run: cargo test -p deve_web change_item_read_gate -- --nocapture
    - run: cargo test -p deve_core discard_docless_added_on_tracked_path -- --nocapture
    - run: cargo test -p deve_core --test source_control_commit_apply_error_test apply_external_changes_rejects_docless_upsert_on_tracked_path -- --nocapture
    - run: cargo test -p deve_core stage_pending_rejects_unresolved_conflict -- --nocapture
    - run: cargo test -p deve_cli sc_stage_all_rejects_unresolved_conflict -- --nocapture
    - run: cargo test -p deve_cli sc_stage_all_rejects_broken_workspace_identity -- --nocapture
    - run: cargo test -p deve_cli sc_commit_rejects_broken_workspace_identity -- --nocapture
    - run: cargo test -p deve_cli status_lines_include_git_mirror_failure_metadata -- --nocapture
    - run: cargo test -p deve_cli status_lines_include_cli_only_repair_action -- --nocapture
    - run: cargo test -p deve_cli status_lines_include_guidance_for_all_repair_actions -- --nocapture
    - run: cargo test -p deve_cli test_git_mirror_repair_review_is_readonly_record_source -- --nocapture
    - run: cargo test -p deve_cli ngit_import_apply_resolved_commit_exports_roundtrip -- --nocapture
    - run: cargo test -p deve_cli ngit_import_export_push_resolved_publish_roundtrip -- --nocapture
    - run: cargo test -p deve_cli ngit_import_apply_rejects_broken_workspace_identity -- --nocapture
    - run: cargo test -p deve_core source_control_ngit_only -- --nocapture
    - run: cargo test -p deve_core run_pending_mirror_commits_terminal_projection_for_multiple_queued_records -- --nocapture
    - run: cargo test -p deve_core run_pending_mirror_rejects_terminal_projection_workspace_content_mismatch -- --nocapture
    - run: cargo test -p deve_core run_pending_mirror_creates_terminal_commit_instead_of_reusing_unmapped_head -- --nocapture
    - run: cargo test -p deve_cli http_source_control_commit_always_queues_git_main_mirror -- --nocapture
    - run: cargo test -p deve_cli http_source_control_mutations_require_browser_write_grant -- --nocapture
    - run: cargo test -p deve_cli http_source_control_write_grant_revoked_on_ws_disconnect -- --nocapture
    - run: cargo test -p deve_cli source_control_write_grant_creation_fails_closed -- --nocapture
    - run: cargo test -p deve_cli switch_branch_failure_revokes_source_control_write_grant -- --nocapture
    - run: cargo test -p deve_cli source_control_scope_cleanup_revokes_write_grant -- --nocapture
    - run: cargo test -p deve_cli repo_scope_runtime_cleanup_revokes_source_control_write_grant -- --nocapture
    - run: cargo test -p deve_cli sync_guard_scope_cleanup_revokes_source_control_write_grant -- --nocapture
    - run: cargo test -p deve_cli browser_sync_hello_failure_revokes_source_control_write_grant -- --nocapture
    - run: cargo test -p deve_cli browser_writer_registration_rejects_degraded_local_projection -- --nocapture
    - run: cargo test -p deve_cli http_source_control_write_grant_requires_local_branch -- --nocapture
    - run: cargo test -p deve_cli anonymous_localhost_source_control_grant_is_not_dev_wide -- --nocapture
    - run: cargo test -p deve_cli anonymous_localhost_source_control_write_grant_roundtrips_status_ws_and_http -- --nocapture
    - run: cargo test -p deve_cli http_source_control_jwt_grant_is_not_shadowed_by_dev_session_cookie -- --nocapture
    - run: cargo test -p deve_cli delegated_source_control_requires_proxy_capability -- --nocapture
    - run: cargo test -p deve_cli delegated_source_control_proxy_api_is_type_marked -- --nocapture
    - run: cargo test -p deve_cli remote_source_control_proxy_reads_use_delegated_capability -- --nocapture
    - run: cargo test -p deve_core --test plugin_source_control_gate_test plugin_sc_commit_uses_ngit_authority -- --nocapture
    - run: cargo check -p deve_core --tests
    - run: cargo test -p deve_core --lib source_control_write_gate_missing_dependencies_fail_closed -- --nocapture
    - run: cargo test -p deve_core --lib source_control_write_gate_rejects_broken_workspace_identity -- --nocapture
    - run: cargo test -p deve_cli proxy_node_role_reports_ngit_authority -- --nocapture
    - run: cargo test -p deve_cli role_payload_exposes_runtime_release_shape -- --nocapture
    - run: cargo test -p deve_core pending_doc_target_prefers_live_successor_over_exact_deleted_doc_path -- --nocapture
    - run: cargo test -p deve_core staged_doc_target_prefers_live_successor_over_exact_deleted_doc_path -- --nocapture
    - run: cargo test -p deve_core stage_wrapper_stages_tracked_rename_pair_from_old_path -- --nocapture
    - run: cargo test -p deve_core commit_diff_rejects_reversed_commit_order -- --nocapture
    - run: cargo test -p deve_web command_palette_ngit_commands_do_not_read_bridge_mode -- --nocapture
    - run: cargo test -p deve_web command_palette_source_control_authority_updates_after_node_role_probe -- --nocapture
    - run: cargo test -p deve_web source_control_header_ngit_authority_badge -- --nocapture
    - run: cargo test -p deve_web remote_projection_commands -- --nocapture
    - run: cargo test -p deve_core remote_projection_file_accepts_only_markdown_projection_paths -- --nocapture
    - run: cargo test -p deve_core provider_request_reuses_transport_admission_validator -- --nocapture
    - run: cargo test -p deve_core fake_adapter_push_stores_projection_files_without_authority_effects -- --nocapture
    - run: cargo test -p deve_core fake_adapter_pull_returns_external_changes_candidate_only -- --nocapture
    - run: cargo test -p deve_cli collect_markdown_projection_files_uploads_only_markdown_projection_files -- --nocapture
    - run: cargo test -p deve_cli collect_markdown_projection_files_skips_ignored_directories -- --nocapture
    - run: cargo test -p deve_cli webdav_push_puts_projection_files_without_authority_effects -- --nocapture
    - run: cargo test -p deve_cli webdav_streaming_push_reads_files_one_at_a_time_without_authority_effects -- --nocapture
    - run: cargo test -p deve_cli webdav_push_rejects_failed_put -- --nocapture
    - run: cargo test -p deve_cli run_webdav_push_uses_webdav_provider_after_workspace_gate -- --nocapture
    - run: cargo test -p deve_cli run_webdav_push_returns_provider_error_before_success_report -- --nocapture
    - run: cargo test -p deve_cli run_reports_provider_io_fail_closed_after_workspace_gate -- --nocapture
    - run: cargo test -p deve_cli run_checks_workspace_identity_before_provider_io -- --nocapture
    - run: cargo test -p deve_web commit_message_placeholder -- --nocapture
    - run: cargo test -p deve_web command_sets_cli_only_notice -- --nocapture
    - run: cargo test -p deve_web local_git_repair_notice -- --nocapture
    - run: cargo test -p deve_core source_control_confirmed_ledger -- --nocapture
    - run: cargo test -p deve_cli confirmed_ledger_changes -- --nocapture
    - run: cargo test -p deve_web confirmed_ledger_changes -- --nocapture
  assertions:
    - ui_assert: source_control_commit_available true
    - ui_assert: source_control_commit_shortcut_prevents_textarea_default true
    - ui_assert: source_control_commit_and_push_cli_only_notice true
    - ui_assert: command_palette_git_sync_absent true
    - cli_assert: ngit_import_apply_pending_only true
    - cli_assert: ngit_writes_reject_broken_workspace_identity true
    - cli_assert: git_push_dirty_worktree_blocker true
    - cli_assert: git_push_unexported_queue_blocker true
    - ui_assert: command_palette_git_commit_absent true
    - ui_assert: command_palette_ngit_import_notice_available true
    - ui_assert: command_palette_ngit_push_notice_available true
    - ui_assert: command_palette_remote_projection_commands_visible true
    - ui_assert: source_control_git_push_blocker_details_available true
    - api_assert: readonly_remote_source_control_writes_rejected true
    - ui_assert: readonly_remote_change_diff_uses_read_gate true
    - ui_assert: readonly_remote_change_write_actions_disabled true
    - ui_assert: command_palette_git_direct_executor_absent true
    - cli_assert: git_mirror_failure_metadata_available true
    - cli_assert: git_mirror_repair_action_cli_only true
    - cli_assert: git_mirror_repair_guidance_manual_only true
    - ui_assert: git_mirror_clickable_repair_ui_future_requires_manual_confirmation true
    - ui_assert: git_mirror_repair_ui_background_git_writer_absent true
    - ui_assert: git_mirror_readonly_repair_review_available true
    - ui_assert: git_mirror_repair_retry_command_copyable_text_only true
    - api_assert: git_mirror_repair_review_data_source protected_http_readonly_endpoint
    - api_assert: git_mirror_repair_review_endpoint_no_git_writer true
    - ui_assert: git_mirror_repair_review_multi_record_available true
    - ui_assert: git_mirror_repair_review_loading_error_empty_fallback true
    - ui_assert: git_mirror_executable_repair_ui_deferred true
    - ui_assert: git_mirror_web_repair_writer_absent true
    - cli_assert: source_control_ngit_commit_queues_git_main_mirror true
    - cli_assert: git_bridge_mode_config_absent true
    - api_assert: http_source_control_commit_always_queues_git_main_mirror true
    - api_assert: browser_http_source_control_requires_session_bound_grant true
    - api_assert: source_control_write_grant_creation_fails_closed true
    - api_assert: source_control_write_grant_revoked_on_failed_scope_cleanup true
    - api_assert: source_control_scope_cleanup_revokes_write_grant true
    - api_assert: repo_scope_runtime_cleanup_revokes_source_control_write_grant true
    - api_assert: sync_guard_scope_cleanup_revokes_source_control_write_grant true
    - api_assert: browser_sync_hello_failure_revokes_source_control_write_grant true
    - api_assert: degraded_writer_registration_revokes_source_control_write_grant true
    - api_assert: anonymous_localhost_source_control_grant_is_session_cookie_bound true
    - api_assert: anonymous_localhost_source_control_grant_roundtrips_status_ws_and_http true
    - api_assert: http_source_control_jwt_grant_is_not_shadowed_by_dev_session_cookie true
    - api_assert: delegated_remote_proxy_scope_nonce_not_accepted_by_main_http_mutation true
    - api_assert: delegated_source_control_requires_proxy_capability true
    - api_assert: delegated_source_control_proxy_api_is_type_marked true
    - api_assert: remote_source_control_proxy_reads_use_delegated_capability true
    - api_assert: discard_docless_added_on_tracked_path_fails_closed true
    - api_assert: apply_external_docless_upsert_on_tracked_path_fails_closed true
    - api_assert: stage_unresolved_conflict_fails_closed true
    - cli_assert: sc_stage_all_unresolved_conflict_fails_closed true
    - cli_assert: sc_write_rejects_broken_workspace_identity true
    - plugin_assert: sc_commit_uses_ngit_authority true
    - api_assert: source_control_commit_writer_uses_ngit_authority true
    - api_assert: commit_and_push_ws_legacy_frame_has_no_write_side_effect true
    - plugin_assert: missing_local_write_gate_fails_closed true
    - plugin_assert: broken_workspace_identity_write_gate_fails_closed true
    - api_assert: proxy_node_role_reports_ngit_authority true
    - api_assert: node_role_omits_source_control_git_bridge true
    - api_assert: doc_id_source_control_targets_prefer_live_rename_successor true
    - api_assert: source_control_rename_pair_stage_is_atomic_and_idempotent true
    - api_assert: commit_diff_reversed_order_fails_closed true
    - ui_assert: command_palette_legacy_bridge_mode_absent true
    - ui_assert: command_palette_source_control_authority_reactive_after_node_role_probe true
    - ui_assert: source_control_legacy_bridge_mode_absent true
    - ui_assert: source_control_ngit_authority_badge_present true
    - api_assert: remote_projection_fake_adapter_markdown_only true
    - api_assert: remote_projection_pull_candidate_external_changes_only true
    - api_assert: remote_projection_provider_metadata_diagnostic_only true
    - api_assert: webdav_push_uploads_markdown_projection_only true
    - api_assert: webdav_push_skips_deveignore_directories true
    - api_assert: webdav_push_streams_projection_files true
    - api_assert: webdav_push_authority_effects_absent true
    - cli_assert: webdav_push_failure_does_not_report_provider_io_ready true
    - cli_assert: webdav_push_provider_io_ready true
    - ui_assert: source_control_commit_empty_state_disabled_reason true
    - api_assert: confirmed_ledger_changes_are_not_pending_fs_ops true
    - api_assert: confirmed_only_commit_creates_anchor_without_duplicate_facts true
    - api_assert: confirmed_only_commit_advances_committed_snapshot_base true
    - ui_assert: source_control_repo_context_confirmed_dirty_marker true
    - ui_assert: confirmed_ledger_changes_section_visible true
    - ui_assert: confirmed_ledger_rows_open_diff_action_present true
    - ui_assert: confirmed_ledger_rows_open_diff_action_title true
    - ui_assert: confirmed_ledger_rows_open_diff_action_visible true
    - ui_assert: confirmed_ledger_rows_stage_discard_absent true
    - ui_assert: source_control_working_rows_absent true
    - ui_assert: external_changes_working_rows_open_discard_stage_actions_present true
    - ui_assert: external_changes_staged_rows_unstage_action_present true
    - ui_assert: source_control_row_actions_desktop_hover_focus_visible true
    - ui_assert: source_control_row_actions_desktop_hidden_tray_does_not_reserve_width true
    - ui_assert: source_control_status_badge_action_tray_separated true
    - ui_assert: confirmed_ledger_section_hint_present true
    - ui_assert: source_control_resource_group_headers_are_buttons true
    - ui_assert: source_control_section_actions_do_not_toggle_group true
    - ui_assert: source_control_secondary_panel_headers_are_buttons true
    - ui_assert: source_control_collapsible_headers_control_stable_panels true
    - ui_assert: source_control_header_section_menu_exposes_checked_state true

- case_id: DIFF-010
  goal: Source Control smoke 不依赖 checked-in dev ledger 处于 clean 状态。
  preconditions:
    - CLI 可用
  steps:
    - run: deve sc-status --repo default
    - run: scripts/check-source-control-smoke-hygiene.sh
    - run: cargo run -p deve_baseline -- source-control-smoke-hygiene
    - run: cargo test -p deve_cli sc_status -- --nocapture
    - run: cargo test -p deve_cli clean_source_control_smoke_fixture -- --nocapture
  assertions:
    - stdout_contains: "sc_status[default]: staged="
    - exit_code_all_eq: 0

- case_id: DIFF-011
  goal: External Changes 与 Source Control 分离。
  preconditions:
    - 当前 repo 可写
    - Projection Workspace 中存在一个外部文件修改
    - 同一文档可构造 confirmed ledger dirty
  steps:
    - run: cargo test -p deve_core external_file_changes_enter_external_changes_not_ledger -- --nocapture
    - run: cargo test -p deve_core external_scan_modified_enters_external_changes_not_ledger -- --nocapture
    - run: cargo test -p deve_core external_scan_deleted_enters_external_changes_not_ledger -- --nocapture
    - run: cargo test -p deve_core external_scan_renamed_enters_external_changes_not_ledger -- --nocapture
    - run: cargo test -p deve_core external_stage_unstage_only_moves_external_staging -- --nocapture
    - run: cargo test -p deve_core apply_external_changes_to_ledger -- --nocapture
    - run: cargo test -p deve_core source_control_confirmed_ledger_changes_visible_after_apply -- --nocapture
    - run: cargo test -p deve_web external_changes -- --nocapture
    - run: cargo test -p deve_web source_control_confirmed_only_view -- --nocapture
    - ui_open: "External Changes"
    - ui_click: "external_change_stage"
    - ui_click: "external_changes_apply_to_ledger"
    - ui_open: "Source Control"
  assertions:
    - api_assert: external_file_changes_enter_external_changes_not_ledger true
    - api_assert: external_scan_modified_enters_external_changes_not_ledger true
    - api_assert: external_scan_deleted_enters_external_changes_not_ledger true
    - api_assert: external_scan_renamed_enters_external_changes_not_ledger true
    - api_assert: external_stage_unstage_only_moves_external_staging true
    - api_assert: apply_external_changes_writes_ledger_facts true
    - api_assert: apply_external_changes_does_not_create_commit_anchor true
    - ui_assert: external_changes_sibling_entry_visible true
    - ui_assert: external_changes_minimal_actions_visible true
    - ui_assert: external_changes_history_graph_absent true
    - ui_assert: external_changes_apply_label_not_commit true
    - ui_assert: source_control_external_working_groups_absent true
    - ui_assert: source_control_confirmed_ledger_changes_visible_after_apply true
    - ui_assert: external_confirmed_overlap_disables_stage true
    - ui_assert: external_confirmed_overlap_allows_diff_and_discard true
```
