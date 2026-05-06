#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "source-control-baseline-check: $*" >&2
  exit 1
}

check_contains() {
  local file="$1"
  local pattern="$2"
  rg -q --fixed-strings -- "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

check_absent() {
  local file="$1"
  local pattern="$2"
  if rg -q --fixed-strings -- "$pattern" "$ROOT_DIR/$file"; then
    fail "unexpected '$pattern' in $file"
  fi
}

check_case_block() {
  local file="$1"
  local case_id="$2"
  local pattern="$3"
  awk -v case_id="$case_id" -v pattern="$pattern" '
    $0 == "- case_id: " case_id { in_case = 1; next }
    in_case && $0 ~ /^- case_id: / { in_case = 0 }
    in_case && index($0, pattern) { found = 1 }
    END { exit found ? 0 : 1 }
  ' "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file acceptance case $case_id"
}

# DIFF-009: Command Palette Git actions remain Planned / Optional. Source Control panel
# owns the current stage/commit/publish entry points.
check_contains docs/plan/12_commands.md "Source Control / Git-like Workflow"
check_contains docs/plan/12_commands.md 'AI: Switch to PLAN Mode'
check_contains docs/plan/12_commands.md 'AI: Switch to BUILD Mode'
check_contains docs/plan/12_commands.md '不负责切换 `native / trusted-cli` 后端'
check_contains apps/web/src/components/command_palette/registry.rs "merge_peer_command"
check_contains docs/acceptance-cases/04_diff.md "cargo test -p deve_web command_sets_cli_only_notice -- --nocapture"
check_absent apps/web/src/components/command_palette/registry.rs "Git: Sync"
check_absent apps/web/src/components/command_palette/registry.rs "Git: Commit"
check_absent apps/web/src/i18n/command_palette.rs "Git: Sync"
check_absent apps/web/src/i18n/command_palette.rs "Git: Commit"
check_contains apps/web/src/i18n/command_palette.rs "Git: Push Mirror"
check_contains apps/web/src/components/command_palette/registry_git.rs "git_import_command_sets_cli_only_notice"
check_contains apps/web/src/components/command_palette/registry_git.rs "git_push_command_sets_cli_only_notice"
check_contains apps/web/src/components/command_palette/registry_git.rs "git_repair_command_sets_cli_only_notice"

# Git mirror Web surfaces are intentionally read-only / CLI-only for this
# stage. Do not let repair review, Command Palette notices, import apply, or
# push CLI be re-described as a Web Git writer or a background executor.
check_contains docs/features/07_diff_logic.md "Git import/push/repair 写操作只允许通过显式 CLI surface 触发"
check_contains docs/features/07_diff_logic.md "当前阶段不实现 Web 后端直接 Git import/push/repair，也不实现后台自动 Git mirror repair"
check_contains docs/plan/14_tech_stack.md "Web/后台不得从只读 status/review surface 隐式升级为 Git writer"
check_contains docs/plan/14_tech_stack.md "任何可执行 Git repair UI 都必须另立设计批次，并要求人工确认"
check_contains docs/report/git-ecosystem-bridge-baseline-2026-05-01.md "Web 只提供 CLI-only notice 与只读 repair review"
check_contains docs/report/git-ecosystem-bridge-baseline-2026-05-01.md "不得后台执行 Git，不得让 Command Palette 直接写 Git"
check_contains apps/cli/src/server/router.rs '"/api/sc/git-mirror/repair-review"'
check_contains apps/web/src/components/sidebar/source_control/error_notice.rs 'data-deve-git-repair-review="readonly"'
check_contains apps/web/src/i18n/source_control_git.rs "Git mirror repair is CLI-only"

# Current Source Control panel operations are explicit and repo-scoped.
check_contains docs/plan/07_diff_logic.md "CommitStaged"
check_contains docs/features/operations/sc_commit.md 'ClientMessage::Commit { scope_nonce }'
check_contains apps/web/src/hooks/use_core/callbacks_sc_write_commit.rs "ClientMessage::Commit"
check_contains apps/web/src/hooks/use_core/callbacks_sc_write_commit.rs "ClientMessage::CommitAndPush"
check_contains apps/web/src/hooks/use_core/callbacks_sc_write_commit.rs "source_control_scope_nonce(scope)"
check_contains apps/cli/src/server/ws/route/source_control.rs "ClientMessage::CommitAndPush { message, .. }"
check_contains apps/cli/src/server/ws/route/source_control.rs "source_control::handle_commit_and_push"
check_contains apps/cli/src/server/ws/route/source_control.rs "source_control_scope_nonce_gate_rejects_missing_scope_before_handler"
check_contains apps/web/src/components/sidebar/source_control/commit_actions.rs "on_commit_and_push.run(())"
check_contains apps/web/src/i18n/source_control.rs "Commit & Push"

# UI-DIFF-001: long Diff renders a bounded first viewport before mounting the
# full document surface.
check_contains docs/acceptance-cases/05_ui.md "case_id: UI-DIFF-001"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-001 "run: scripts/check-source-control-baseline.sh"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-001 "run: cargo test -p deve_web diff_first_viewport -- --nocapture"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-001 "cli_assert: diff_first_viewport_window_bound true"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-001 "cli_assert: diff_first_viewport_spacer_bound true"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-001 "cli_assert: diff_first_viewport_marker_bound true"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-001 "cli_assert: diff_first_viewport_initial_compute_deferred true"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-001 "cli_assert: diff_first_viewport_full_compute_chunked true"
check_contains docs/acceptance-bindings.tsv "UI-DIFF-001|manual-chrome|docs/features/07_diff_logic.md|diff first viewport workflow"
check_contains apps/web/src/components/diff_view/unified.rs "pub const DIFF_VIEWPORT_CHUNK_SIZE: usize = 80;"
check_contains apps/web/src/components/diff_view/unified.rs "fn diff_first_viewport_initial_window_is_bounded_for_long_doc()"
check_contains apps/web/src/components/diff_view/unified.rs "fn diff_first_viewport_spacers_preserve_scroll_extent()"
check_contains apps/web/src/components/diff_view/unified.rs "fn diff_first_viewport_clamps_stale_scroll_after_shorter_doc()"
check_contains apps/web/src/components/diff_view/model/model_chunk.rs "pub struct DiffChunkJob"
check_contains apps/web/src/components/diff_view/model/model_chunk.rs "fn diff_first_viewport_preview_bounds_initial_compute_rows()"
check_contains apps/web/src/components/diff_view/model/model_chunk.rs "fn diff_first_viewport_chunk_job_matches_sync_chunked_output()"
check_contains apps/web/src/components/diff_view/model/model_chunk.rs "fn diff_first_viewport_chunk_job_handles_insert_only_document()"
check_contains apps/web/src/components/diff_view/model/model_chunk.rs "fn diff_first_viewport_chunk_job_handles_delete_only_document()"
check_contains apps/web/src/components/diff_view/state_compute.rs "start_chunked_diff("
check_contains apps/web/src/components/diff_view/state_compute.rs "ComputePhase::PartialReady"
check_contains apps/web/src/components/diff_view/state_compute.rs "preview_diff(old_content_ref.as_str(), &text)"
check_contains apps/web/src/components/diff_view/state_compute.rs "key: Option<String>"
check_contains apps/web/src/components/diff_view/state_compute_helpers.rs "initial_cached_or_preview("
check_contains apps/web/src/components/diff_view/state_compute_helpers.rs "preview_diff("
check_contains apps/web/src/components/diff_view/state_compute_helpers.rs "fn exceeds_preview_window("
check_contains apps/web/src/components/diff_view/state_compute_helpers.rs "fn diff_first_viewport_initial_cache_miss_uses_preview()"
check_contains apps/web/src/components/diff_view/state_compute_helpers.rs "fn diff_first_viewport_long_initial_defers_cache_key()"
check_contains apps/web/src/components/diff_view/state_compute_helpers.rs "fn diff_first_viewport_short_initial_builds_cache_key()"
check_contains apps/web/src/components/diff_view/state_compute_helpers.rs "fn diff_first_viewport_initial_short_cache_miss_is_complete()"
check_contains apps/web/src/components/diff_view/viewport.rs "pub const DEFAULT_DIFF_VIEWPORT_HEIGHT_PX: i32 = 600;"
check_contains apps/web/src/components/diff_view/viewport.rs "fn diff_first_viewport_default_height_is_stable()"
check_contains apps/web/src/components/diff_view/unified_pane.rs "data-deve-diff-first-viewport=move ||"
check_contains apps/web/src/components/diff_view/unified_pane.rs "fn diff_first_viewport_marker_requires_ready_visible_rows()"

# CommitAndPush is a publish entry point that currently completes as CommitAck,
# not a separate user-visible SyncPush result flow.
check_contains docs/features/operations/sc_commit_and_push.md "CommitAck"
check_contains apps/cli/src/server/handlers/source_control/commits.rs "Commit & Push"
check_absent apps/cli/src/server/handlers/source_control/commits.rs "SyncPush"

# Commit diff must preserve canonical document identity instead of becoming path-only output.
check_contains docs/plan/07_diff_logic.md "canonical targets"
check_contains crates/core/src/source_control/types.rs "pub doc_id: Option<DocId>"
check_contains crates/core/src/source_control/commit_diff.rs "doc_id: Some(doc_id)"
check_contains crates/core/tests/commit_diff_node_projection_test.rs "assert_eq!(diffs[0].doc_id, Some(doc_id));"

# Live doc diff responses must also preserve canonical document identity.
check_contains docs/plan/07_diff_logic.md "- \`DocDiff\`"
check_contains docs/plan/07_diff_logic.md "  - \`doc_id\`"
check_contains crates/core/src/protocol/server.rs "DocDiff {"
check_contains crates/core/src/protocol/server.rs "#[serde(default)] doc_id: Option<DocId>"
check_contains apps/cli/src/server/handlers/source_control/diff.rs "workdir_diff_payload_for_target_in_local_repo"
check_contains apps/cli/src/server/handlers/source_control/diff_remote.rs "doc_id: Some(doc_id)"
check_contains apps/web/src/hooks/use_core/diff_session.rs "pub doc_id: Option<DocId>"
check_contains apps/web/src/hooks/use_core/effects_sc_apply.rs "apply_doc_diff_preserves_doc_identity"

# Doc-id source-control targets are strict: exact path or rename successor only.
check_absent crates/core/src/ledger/manager/source_control_target_lookup.rs "resolve_canonical_path"
check_contains crates/core/src/ledger/manager/source_control_target_lookup_test.rs "resolve_from_entries_rejects_unrelated_same_doc_live_entry"
check_contains crates/core/src/source_control/pending_fs_target_test.rs "doc_target_rejects_unrelated_same_doc_live_entry"
check_contains crates/core/src/source_control/staging_target_test.rs "doc_target_rejects_unrelated_same_doc_live_entry"
check_contains crates/core/tests/source_control_target_lookup_canonical_test.rs "workdir_diff_target_rejects_doc_id_when_requested_path_is_not_in_change_set"

# Local repo execution selectors must fail closed on repo_id/repo_name mismatch.
check_absent crates/core/src/ledger/manager/locator.rs "ignored stale repo_name"
check_contains crates/core/src/ledger/manager/locator.rs "select_local_repo_name_for_execution(&candidates)"
check_contains crates/core/tests/local_repo_selector_heal_test.rs "resolve_local_repo_name_for_execution_rejects_selector_mismatch"
check_contains apps/cli/src/server/source_control_http_status_test.rs "test_http_status_rejects_selector_mismatch"
check_contains apps/cli/src/server/source_control_http_status_test.rs "ServerErrorCode::ScRepoContextInvalid"
check_contains apps/cli/src/server/handlers/source_control/present_resolve.rs "matched multiple doc rename successors"
check_contains apps/cli/src/server/handlers/source_control/present_resolve_extra_test.rs "resolve_target_fails_closed_when_doc_id_has_multiple_rename_successors"
check_contains apps/cli/src/server/handlers/source_control/present_resolve_extra_test.rs "resolve_target_fails_closed_when_doc_id_matches_exact_and_successor"
check_contains apps/cli/src/server/handlers/source_control/service/target.rs "current_entry(entries, resolved)"
check_contains apps/cli/src/server/handlers/source_control/service/target_related_test.rs "related_targets_keep_resolved_doc_id_when_old_path_is_reused"
check_contains crates/core/src/ledger/manager/source_control_workdir.rs "workdir_diff_inputs_for_resolved_target"
check_contains crates/core/tests/source_control_target_lookup_canonical_test.rs "workdir_diff_payload_preserves_doc_id_when_resolved_path_is_reused"
check_contains apps/cli/src/server/handlers/source_control/diff_remote_content.rs "Remote document target path mismatch"
check_contains apps/cli/src/server/handlers/source_control/diff_remote_test_extra.rs "remote_diff_rejects_doc_id_path_mismatch"
check_contains crates/core/src/ledger/manager/commit_ops.rs "preflight_staged_upsert_identity"
check_contains crates/core/src/ledger/manager/commit_ops.rs "lacks rename evidence"
check_contains crates/core/tests/source_control_commit_apply_error_test.rs "commit_staged_rejects_upsert_target_when_path_is_bound_to_another_doc"
check_contains crates/core/tests/source_control_commit_apply_error_test.rs "commit_staged_rejects_upsert_move_without_rename_evidence"
check_contains docs/plan/07_diff_logic.md "legacy \`Deleted + doc_id=None\` 的 exact delete selector"
check_contains crates/core/src/ledger/manager/source_control_path_target.rs "has_legacy_docless_exact_delete"
check_contains crates/core/src/ledger/manager/source_control_path_target.rs "path_wrapper_promotes_docless_non_delete_to_tracked_identity"

echo "source-control-baseline-check: ok"
