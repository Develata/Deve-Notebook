## Diff 与合并

```markdown
- case_id: DIFF-001
  goal: UTF-16 索引一致。
  preconditions:
    - 文档包含 emoji: "A😀B"
  steps:
    - ui_place_cursor_after: "A😀"
    - ui_type: "X"
    - run: deve dump --doc current --field last_op
  assertions:
    - stdout_contains: "utf16_index"

- case_id: DIFF-002
  goal: 3-Way Merge 使用 LCA。
  preconditions:
    - Local 与 Remote 均基于同一 Base 修改
  steps:
    - run: deve merge --peer <peer_id>
    - run: cargo test -p deve_cli merge_scope_nonce_gate -- --nocapture
  assertions:
    - log_contains: "LCA"

- case_id: DIFF-003
  goal: 冲突检测按 Hunk 触发。
  preconditions:
    - Local 与 Remote 修改同一段落
  steps:
    - run: deve merge --peer <peer_id>
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
  assertions:
    - ws_message: "ResolveMergeConflict"
    - ws_field_contains: "doc_id action result_content scope_nonce"
    - ui_assert: result_matches_strategy true

- case_id: DIFF-005
  goal: 合并中断可续。
  preconditions:
    - 合并进行中
  steps:
    - run: taskkill /F /IM deve_cli.exe
    - run: deve merge --peer <peer_id>
  assertions:
    - log_contains: "resume"
    - ledger_ops_not_lost true

- case_id: DIFF-006
  goal: Watcher 防抖与 Hash 校验。
  preconditions:
    - `deve watch` 运行中
  steps:
    - run: powershell -Command "1..5 | % { 'x' | Add-Content ${DEVE_DATA_DIR}/vault/debounce.md }"
  assertions:
    - ledger_op_count_increases_by: 1

- case_id: DIFF-007
  goal: Diff 颜色语义。
  preconditions:
    - 文档包含新增/修改/删除
  steps:
    - ui_open_diff: true
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
    - run: cargo test -p deve_web fs_refresh -- --nocapture
    - run: cargo test -p deve_web read_list_dispatch -- --nocapture
    - run: cargo test -p deve_web doc_diff_read_gate -- --nocapture
    - run: cargo test -p deve_web doc_diff_dispatch -- --nocapture
    - run: cargo test -p deve_web commit_diff_read_gate -- --nocapture
    - run: cargo test -p deve_web commit_diff_dispatch -- --nocapture
    - run: cargo test -p deve_cli source_control_scope_nonce_gate -- --nocapture
    - run: cargo test -p deve_cli readonly_remote_doc_diff_uses_shadow_projection -- --nocapture
    - run: cargo test -p deve_cli readonly_remote_doc_diff_missing_target -- --nocapture
    - run: cargo test -p deve_cli readonly_remote_doc_diff_path_mismatch -- --nocapture
    - run: cargo test -p deve_cli readonly_remote_source_control_writes -- --nocapture
    - run: cargo test -p deve_cli readonly_remote_commit_diff_is_allowed -- --nocapture
    - run: cargo test -p deve_cli readonly_remote_commit_diff_reports_rename_projection -- --nocapture
    - run: cargo test -p deve_cli status_lines_include_git_mirror_failure_metadata -- --nocapture
    - run: cargo test -p deve_cli status_lines_include_cli_only_repair_action -- --nocapture
    - run: cargo test -p deve_cli status_lines_include_guidance_for_all_repair_actions -- --nocapture
    - run: cargo test -p deve_cli test_git_mirror_repair_review_is_readonly_record_source -- --nocapture
    - run: cargo test -p deve_cli git_import_apply_resolved_commit_exports_roundtrip -- --nocapture
    - run: cargo test -p deve_cli git_import_export_push_resolved_publish_roundtrip -- --nocapture
    - run: cargo test -p deve_web local_git_repair_notice -- --nocapture
  assertions:
    - ui_assert: source_control_commit_available true
    - ui_assert: source_control_commit_and_push_available true
    - ui_assert: command_palette_git_sync_absent true
    - cli_assert: git_import_apply_pending_only true
    - cli_assert: git_push_dirty_worktree_blocker true
    - cli_assert: git_push_unexported_queue_blocker true
    - ui_assert: command_palette_git_commit_absent true
    - ui_assert: command_palette_git_import_cli_notice_available true
    - ui_assert: command_palette_git_push_cli_notice_available true
    - ui_assert: source_control_git_push_blocker_details_available true
    - api_assert: readonly_remote_source_control_writes_rejected true
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

- case_id: DIFF-010
  goal: Source Control smoke 不依赖 checked-in dev ledger 处于 clean 状态。
  preconditions:
    - CLI 可用
  steps:
    - run: deve sc-status --repo default
    - run: scripts/check-source-control-smoke-hygiene.sh
    - run: cargo test -p deve_cli sc_status -- --nocapture
    - run: cargo test -p deve_cli clean_source_control_smoke_fixture -- --nocapture
  assertions:
    - stdout_contains: "sc_status[default]: staged="
    - exit_code_all_eq: 0
```
