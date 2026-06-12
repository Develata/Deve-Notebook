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
  MSYS2_ARG_CONV_EXCL="$pattern" rg -q --fixed-strings -- "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

check_absent() {
  local file="$1"
  local pattern="$2"
  if MSYS2_ARG_CONV_EXCL="$pattern" rg -q --fixed-strings -- "$pattern" "$ROOT_DIR/$file"; then
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
check_contains docs/plan/14_commands.md "Source Control / Git-like Workflow"
check_contains docs/plan/14_commands.md 'AI: Switch to PLAN Mode'
check_contains docs/plan/14_commands.md 'AI: Switch to BUILD Mode'
check_contains docs/plan/14_commands.md '不负责切换 `native / trusted-cli` 后端'
check_contains apps/web/src/components/command_palette/registry.rs "merge_peer_command"
check_contains apps/web/src/components/command_palette/registry.rs "establish_branch_command(locale, set_show)"
check_contains apps/web/src/components/command_palette/registry/branch.rs "Command::unavailable"
check_contains apps/web/src/components/command_palette/registry/branch.rs "establish_branch_command_is_unavailable_notice_only"
check_contains apps/web/src/components/command_palette/registry/git/tests.rs "git_status_command_sets_cli_only_notice"
check_contains apps/web/src/components/command_palette/registry/git/tests.rs "git_mirror_command_sets_cli_only_notice"
check_contains apps/web/src/components/command_palette/registry/git/tests.rs "git_export_command_sets_cli_only_notice"
check_contains apps/web/src/components/command_palette/ui.rs "data-deve-command-unavailable"
check_contains apps/web/src/hooks/use_core/source_control_notice.rs "ESTABLISH_BRANCH_UNAVAILABLE_DETAIL"
check_contains apps/web/src/hooks/use_core/source_control_notice.rs "GIT_STATUS_CLI_NOTICE_DETAIL"
check_contains apps/web/src/hooks/use_core/source_control_notice.rs "GIT_MIRROR_CLI_NOTICE_DETAIL"
check_contains apps/web/src/hooks/use_core/source_control_notice.rs "GIT_EXPORT_CLI_NOTICE_DETAIL"
check_contains apps/web/src/components/sidebar/source_control/error_notice_copy.rs "is_establish_branch_unavailable_notice"
check_contains apps/web/src/i18n/command_palette.rs "establish_branch_unavailable_reason"
check_contains apps/web/src/i18n/command_palette.rs "git_status"
check_contains apps/web/src/i18n/command_palette.rs "git_mirror"
check_contains apps/web/src/i18n/command_palette.rs "git_export_mirror"
check_contains docs/acceptance-cases/04_diff.md "cargo test -p deve_web command_sets_cli_only_notice -- --nocapture"
check_contains docs/acceptance-cases/11_commands_settings.md "cargo test -p deve_web establish_branch_command -- --nocapture"
check_contains docs/acceptance-cases/11_commands_settings.md "case_id: CMD-004B"
check_contains docs/acceptance-cases/11_commands_settings.md "cargo test -p deve_web git_status_command -- --nocapture"
check_contains docs/acceptance-cases/11_commands_settings.md "cargo test -p deve_web git_mirror_command -- --nocapture"
check_contains docs/acceptance-cases/11_commands_settings.md "cargo test -p deve_web git_export_command -- --nocapture"
check_absent apps/web/src/components/command_palette/registry.rs "P2P: Establish Branch (Placeholder)"
check_absent docs/acceptance-cases/04_diff.md "deve merge --peer"
check_absent docs/acceptance-cases/04_diff.md "deve dump --doc"
check_absent docs/acceptance-cases/04_diff.md "--field last_op"
check_absent apps/web/src/components/command_palette/registry.rs "Git: Sync"
check_absent apps/web/src/components/command_palette/registry.rs "Git: Commit"
check_absent apps/web/src/i18n/command_palette.rs "Git: Sync"
check_absent apps/web/src/i18n/command_palette.rs "Git: Commit"
check_contains apps/web/src/i18n/command_palette.rs "Git: Status"
check_contains apps/web/src/i18n/command_palette.rs "Git: Mirror"
check_contains apps/web/src/i18n/command_palette.rs "Git: Export Mirror"
check_contains apps/web/src/i18n/command_palette.rs "Git: Push Mirror"
check_contains apps/web/src/components/command_palette/registry/git.rs "git_status_command"
check_contains apps/web/src/components/command_palette/registry/git.rs "git_mirror_command"
check_contains apps/web/src/components/command_palette/registry/git.rs "git_export_command"
check_contains apps/web/src/components/command_palette/registry/git/tests.rs "git_import_command_sets_cli_only_notice"
check_contains apps/web/src/components/command_palette/registry/git/tests.rs "git_push_command_sets_cli_only_notice"
check_contains apps/web/src/components/command_palette/registry/git/tests.rs "git_repair_command_sets_cli_only_notice"

# CMD-004A: unimplemented P2P branch creation remains discoverable as an
# unavailable Command Palette entry and must not masquerade as branch switch.
check_contains docs/acceptance-cases/11_commands_settings.md "case_id: CMD-004A"

# Git mirror Web surfaces are intentionally read-only / CLI-only for this
# stage. Do not let repair review, Command Palette notices, import apply, or
# push CLI be re-described as a Web Git writer or a background executor.
check_contains docs/features/07_diff_logic.md "Git import/push/repair 写操作只允许通过显式 CLI surface 触发"
check_contains docs/features/07_diff_logic.md "当前阶段不实现 Web 后端直接 Git import/push/repair，也不实现后台自动 Git mirror repair"
check_contains docs/plan/17_tech_stack.md "Web/后台不得从只读 status/review surface 隐式升级为 Git writer"
check_contains docs/plan/17_tech_stack.md "任何可执行 Git repair UI 都必须另立设计批次，并要求人工确认"
check_contains docs/report/git-ecosystem-bridge-baseline-2026-05-01.md "Web 只提供 CLI-only notice 与只读 repair review"
check_contains docs/report/git-ecosystem-bridge-baseline-2026-05-01.md "不得后台执行 Git，不得让 Command Palette 直接写 Git"
check_contains apps/cli/src/server/router.rs '"/api/sc/git-mirror/repair-review"'
check_contains apps/web/src/components/sidebar/source_control/error_notice.rs 'data-deve-git-repair-review="readonly"'
check_contains apps/web/src/i18n/source_control_git.rs "Git mirror repair is CLI-only"

# Current Source Control panel operations are explicit and repo-scoped.
check_contains docs/plan/05_diff_logic.md "CommitStaged"
check_contains docs/features/operations/sc_commit.md 'ClientMessage::Commit { scope_nonce }'
check_contains apps/web/src/hooks/use_core/callbacks_sc/write/commit.rs "ClientMessage::Commit"
check_contains apps/web/src/hooks/use_core/callbacks_sc/write/commit.rs "ClientMessage::CommitAndPush"
check_contains apps/web/src/hooks/use_core/callbacks_sc/write/commit.rs "source_control_scope_nonce(scope)"
check_contains apps/cli/src/server/ws/route/source_control.rs "ClientMessage::CommitAndPush { message, .. }"
check_contains apps/cli/src/server/ws/route/source_control.rs "source_control::handle_commit_and_push"
check_contains apps/cli/src/server/ws/route/source_control/tests.rs "source_control_scope_nonce_gate_rejects_missing_scope_before_handler"
check_contains apps/cli/src/server/handlers/source_control/http_scope.rs "source control scope nonce missing"
check_contains apps/cli/src/server/tests/source_control/source_control_http_test/status.rs "test_http_status_rejects_missing_scope_nonce"
check_contains apps/cli/src/server/tests/source_control/source_control_http_test/stage.rs "test_http_stage_rejects_missing_scope_nonce_before_mutation"
check_contains apps/cli/src/server/source_control_proxy/mod.rs "REMOTE_PROXY_SCOPE_NONCE"
check_contains apps/cli/src/server/source_control_proxy/mod.rs "new_with_delegation_secret"
check_contains apps/cli/src/server/source_control_proxy/mod.rs "DELEGATED_SC_HEADER"
check_contains apps/cli/src/server/source_control_proxy/client.rs "pub(super) fn build_client(base_url: &str) -> Result<reqwest::Client>"
check_contains apps/cli/src/commands/serve.rs "RemoteSourceControlApi::new_with_delegation_secret"
check_contains apps/cli/src/server/router.rs "delegated_source_control_middleware"
check_contains apps/cli/src/server/auth/delegated_source_control.rs "delegated source control capability missing"
check_contains docs/acceptance-cases/04_diff.md "cargo test -p deve_cli switch_branch_failure_revokes_source_control_write_grant -- --nocapture"
check_contains docs/acceptance-cases/04_diff.md "cargo test -p deve_cli source_control_scope_cleanup_revokes_write_grant -- --nocapture"
check_contains docs/acceptance-cases/04_diff.md "cargo test -p deve_core source_control_write_gate_rejects_broken_workspace_identity -- --nocapture"
check_contains apps/cli/src/server/tests/source_control/source_control_http_test/stage.rs "delegated_source_control_requires_proxy_capability"
check_contains apps/cli/src/server/tests/source_control/source_control_http_test/stage.rs "http_source_control_write_grant_requires_local_branch"
check_contains apps/cli/src/server/tests/source_control/source_control_scope_binding_test/remote.rs "source_control_scope_cleanup_revokes_write_grant"
check_contains apps/cli/src/server/tests/switcher/switcher_current_scope_binding_test.rs "switch_branch_failure_revokes_source_control_write_grant"
check_absent apps/cli/src/server/source_control_proxy/client.rs 'expect("build source control HTTP client")'
check_absent apps/cli/src/server/handlers/source_control/repo_scope.rs 'expect("checked active branch")'
check_absent apps/cli/src/server/repo_scope/sync.rs 'expect("checked active branch")'
check_absent apps/cli/src/server/handlers/switcher/switcher_scope.rs 'expect("checked active branch")'
check_absent apps/cli/src/commands/git_output/status.rs 'expect("lagging record rendered retry command")'
check_absent crates/core/src/git_bridge/push.rs 'expect("remote resolved")'
check_absent crates/core/src/git_bridge/push.rs 'expect("branch resolved")'
check_contains crates/core/src/git_bridge/push/tests.rs "unresolved_push_target_becomes_blocker_instead_of_panic"
check_contains apps/cli/src/commands/git_output/status.rs 'let retry_command = git_command("export", repo_name, true);'
check_contains apps/web/src/api/git_mirror.rs "scope_nonce={scope_nonce}"
check_contains apps/web/src/api/git_mirror.rs "encode_query_component(repo_id)"
check_contains apps/web/src/api/git_mirror.rs "repair_review_url_encodes_repo_id_query_component"
check_contains apps/web/src/components/sidebar/source_control/error_notice.rs "current_scope_nonce.get_untracked() == scope_nonce"
check_contains apps/web/src/components/sidebar/source_control/commit_actions.rs "on_commit_and_push.run(())"
check_contains apps/web/src/i18n/source_control/actions.rs "Commit & Push"

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
check_contains apps/web/src/components/diff_view/state/compute/mod.rs "start_chunked_diff("
check_contains apps/web/src/components/diff_view/state/compute/mod.rs "ComputePhase::PartialReady"
check_contains apps/web/src/components/diff_view/state/compute/mod.rs "preview_diff(old_content_ref.as_str(), &text)"
check_contains apps/web/src/components/diff_view/state/compute/helpers.rs "pub key: Option<String>"
check_contains apps/web/src/components/diff_view/state/compute/helpers.rs "initial_cached_or_preview("
check_contains apps/web/src/components/diff_view/state/compute/helpers.rs "preview_diff("
check_contains apps/web/src/components/diff_view/state/compute/helpers.rs "pub const INITIAL_DIFF_PREVIEW_BYTES"
check_contains apps/web/src/components/diff_view/state/compute/helpers.rs "fn exceeds_preview_window("
check_contains apps/web/src/components/diff_view/state/compute/helpers/tests.rs "fn diff_first_viewport_initial_cache_miss_uses_preview()"
check_contains apps/web/src/components/diff_view/state/compute/helpers/tests.rs "fn diff_first_viewport_long_initial_defers_cache_key()"
check_contains apps/web/src/components/diff_view/state/compute/helpers/tests.rs "fn diff_first_viewport_short_initial_builds_cache_key()"
check_contains apps/web/src/components/diff_view/state/compute/helpers/tests.rs "fn diff_first_viewport_large_single_line_defers_cache_key()"
check_contains apps/web/src/components/diff_view/state/compute/helpers/tests.rs "fn diff_first_viewport_large_single_line_preview_is_byte_bounded()"
check_contains apps/web/src/components/diff_view/state/compute/helpers/tests.rs "fn diff_first_viewport_preview_respects_utf8_byte_boundary()"
check_contains apps/web/src/components/diff_view/state/compute/helpers/tests.rs "fn diff_first_viewport_initial_short_cache_miss_is_complete()"
check_contains apps/web/src/components/diff_view/viewport.rs "pub const DEFAULT_DIFF_VIEWPORT_HEIGHT_PX: i32 = 600;"
check_contains apps/web/src/components/diff_view/viewport.rs "fn diff_first_viewport_default_height_is_stable()"
check_contains apps/web/src/components/diff_view/unified_pane.rs "data-deve-diff-first-viewport=move ||"
check_contains apps/web/src/components/diff_view/unified_pane.rs "fn diff_first_viewport_marker_requires_ready_visible_rows()"

# UI-DIFF-002..018: extended Diff interactions have semantic acceptance
# bindings and minimal automated guards for the behavior that is already pure
# enough to test outside Chrome.
check_contains docs/acceptance-bindings.tsv "UI-DIFF-002|manual-chrome|docs/features/07_diff_logic.md|mobile diff edit debounce workflow"
check_contains docs/acceptance-bindings.tsv "UI-DIFF-003|manual-chrome|docs/features/07_diff_logic.md|desktop diff chat coexistence workflow"
check_contains docs/acceptance-bindings.tsv "UI-DIFF-004|manual-chrome|docs/features/07_diff_logic.md|diff i18n copy workflow"
check_contains docs/acceptance-bindings.tsv "UI-DIFF-005|manual-chrome|docs/features/07_diff_logic.md|diff compute indicator workflow"
check_contains docs/acceptance-bindings.tsv "UI-DIFF-006|manual-chrome|docs/features/07_diff_logic.md|diff hunk button navigation workflow"
check_contains docs/acceptance-bindings.tsv "UI-DIFF-007|manual-chrome|docs/features/07_diff_logic.md|diff word-level replace highlight workflow"
check_contains docs/acceptance-bindings.tsv "UI-DIFF-008|manual-chrome|docs/features/07_diff_logic.md|diff bracket and alt-key navigation workflow"
check_contains docs/acceptance-bindings.tsv "UI-DIFF-009|manual-chrome|docs/features/07_diff_logic.md|diff header change stats workflow"
check_contains docs/acceptance-bindings.tsv "UI-DIFF-010|manual-chrome|docs/features/07_diff_logic.md|diff unchanged-region folding workflow"
check_contains docs/acceptance-bindings.tsv "UI-DIFF-011|manual-chrome|docs/features/07_diff_logic.md|diff context-lines switch workflow"
check_contains docs/acceptance-bindings.tsv "UI-DIFF-012|manual-chrome|docs/features/07_diff_logic.md|diff semantic anchor restore workflow"
check_contains docs/acceptance-bindings.tsv "UI-DIFF-013|manual-chrome|docs/features/07_diff_logic.md|diff cache badge and compute time workflow"
check_contains docs/acceptance-bindings.tsv "UI-DIFF-014|manual-chrome|docs/features/07_diff_logic.md|diff cache invalidation workflow"
check_contains docs/acceptance-bindings.tsv "UI-DIFF-015|manual-chrome|docs/features/07_diff_logic.md|diff algorithm label workflow"
check_contains docs/acceptance-bindings.tsv "UI-DIFF-016|manual-chrome|docs/features/07_diff_logic.md|diff F7 navigation workflow"
check_contains docs/acceptance-bindings.tsv "UI-DIFF-017|manual-chrome|docs/features/07_diff_logic.md|diff cache ratio workflow"
check_contains docs/acceptance-bindings.tsv "UI-DIFF-018|manual-chrome|docs/features/07_diff_logic.md|diff repo-scope cache isolation workflow"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-002 "run: cargo test -p deve_web diff_edit_debounce -- --nocapture"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-004 "run: scripts/check-source-control-baseline.sh"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-004 "cli_assert: diff_hardcoded_copy_absent true"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-005 "run: cargo test -p deve_web diff_compute_indicator -- --nocapture"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-006 "run: cargo test -p deve_web diff_hunk_navigation -- --nocapture"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-007 "run: cargo test -p deve_web diff_replace_lines -- --nocapture"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-008 "run: cargo test -p deve_web diff_hunk_navigation -- --nocapture"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-009 "run: cargo test -p deve_web diff_header_change_stats -- --nocapture"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-010 "run: cargo test -p deve_web diff_fold_rows -- --nocapture"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-011 "run: cargo test -p deve_web diff_context_lines -- --nocapture"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-012 "run: cargo test -p deve_web diff_semantic_anchor -- --nocapture"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-013 "run: cargo test -p deve_web diff_cache -- --nocapture"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-013 "run: cargo test -p deve_web diff_elapsed_ms -- --nocapture"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-014 "run: cargo test -p deve_web diff_cache -- --nocapture"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-015 "run: cargo test -p deve_web diff_algorithm_label -- --nocapture"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-016 "run: cargo test -p deve_web diff_hunk_navigation -- --nocapture"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-017 "run: cargo test -p deve_web diff_cache_ratio -- --nocapture"
check_case_block docs/acceptance-cases/05_ui.md UI-DIFF-018 "run: cargo test -p deve_web diff_cache_key -- --nocapture"
check_contains apps/web/src/components/diff_view/state/compute/mod.rs "pub const DIFF_EDIT_DEBOUNCE_MS: u32 = 150;"
check_contains apps/web/src/components/diff_view/state/compute/mod.rs "fn diff_edit_debounce_matches_acceptance_contract()"
check_contains apps/web/src/components/diff_view/state/mod.rs "fn diff_compute_indicator_tracks_non_ready_phases()"
check_contains apps/web/src/components/diff_view/navigation.rs "fn diff_hunk_navigation_indices_wrap()"
check_contains apps/web/src/components/diff_view/navigation.rs "fn diff_header_change_stats_count_added_and_deleted_lines()"
check_contains apps/web/src/components/diff_view/model.rs "fn diff_replace_lines_emit_word_level_ranges()"
check_contains apps/web/src/components/diff_view/model/hunk_fold.rs "fn diff_fold_rows_collapse_and_expand_unchanged_region()"
check_contains apps/web/src/components/diff_view/model/hunk_fold.rs "fn diff_context_lines_change_fold_count()"
check_contains apps/web/src/components/diff_view/anchor.rs "fn diff_semantic_anchor_delta_preserves_original_offset()"
check_contains apps/web/src/components/diff_view/cache.rs "fn diff_cache_key_is_scoped_by_repo_path_mode_and_context()"
check_contains apps/web/src/components/diff_view/metrics.rs "fn diff_cache_ratio_updates_from_samples()"
check_contains apps/web/src/components/diff_view/metrics.rs "fn diff_elapsed_ms_saturates_for_clock_skew()"
check_contains apps/web/src/components/diff_view/state/compute/helpers/tests.rs "fn diff_algorithm_label_names_are_stable()"
check_contains apps/web/src/components/diff_view/header.rs "t::diff::cache_state_help"
check_contains apps/web/src/components/diff_view/header.rs "t::diff::cache_ratio_help"
check_contains apps/web/src/components/diff_view/header.rs "t::diff::compute_ms_help"
check_contains apps/web/src/i18n/diff.rs "pub fn cache_state_help"
check_contains apps/web/src/i18n/diff.rs "pub fn cache_ratio_help"
check_contains apps/web/src/i18n/diff.rs "pub fn compute_ms_help"
check_absent apps/web/src/components/diff_view '"Diff:"'
check_absent apps/web/src/components/diff_view '"Read Only"'
check_absent apps/web/src/components/diff_view '"Preview Diff"'
check_absent apps/web/src/components/diff_view '"Close Diff View"'

# CommitAndPush is a publish entry point that currently completes as CommitAck,
# not a separate user-visible SyncPush result flow.
check_contains docs/features/operations/sc_commit_and_push.md "CommitAck"
check_contains apps/cli/src/server/handlers/source_control/commits.rs "Commit & Push"
check_absent apps/cli/src/server/handlers/source_control/commits.rs "SyncPush"

# Commit diff must preserve canonical document identity instead of becoming path-only output.
check_contains docs/plan/05_diff_logic.md "canonical targets"
check_contains crates/core/src/source_control/types.rs "pub doc_id: Option<DocId>"
check_contains crates/core/src/source_control/commit_diff.rs "doc_id: Some(doc_id)"
check_contains crates/core/tests/commit_diff_node_projection_test.rs "assert_eq!(diffs[0].doc_id, Some(doc_id));"

# Live doc diff responses must also preserve canonical document identity.
check_contains docs/plan/05_diff_logic.md "- \`DocDiff\`"
check_contains docs/plan/05_diff_logic.md "  - \`doc_id\`"
check_contains crates/core/src/protocol/server.rs "DocDiff {"
check_contains crates/core/src/protocol/server.rs "#[serde(default)] doc_id: Option<DocId>"
check_contains apps/cli/src/server/handlers/source_control/diff/mod.rs "workdir_diff_payload_for_target_in_local_repo"
check_contains apps/cli/src/server/handlers/source_control/diff/remote.rs "doc_id: Some(doc_id)"
check_contains apps/web/src/runtime/source_control_client/diff_session.rs "pub doc_id: Option<DocId>"
check_contains apps/web/src/hooks/use_core/effects_sc_apply/tests.rs "apply_doc_diff_preserves_doc_identity"

# Doc-id source-control targets are strict: exact path or rename successor only.
check_absent crates/core/src/ledger/manager/source_control_target_lookup.rs "resolve_canonical_path"
check_contains crates/core/src/ledger/manager/source_control_target_resolution/tests.rs "resolve_from_entries_rejects_unrelated_same_doc_live_entry"
check_contains crates/core/src/source_control/pending_fs/target/tests.rs "pending_doc_target_prefers_live_successor_over_exact_deleted_doc_path"
check_contains crates/core/src/source_control/staging/target/tests.rs "staged_doc_target_prefers_live_successor_over_exact_deleted_doc_path"
check_contains crates/core/src/source_control/pending_fs/target/tests.rs "doc_target_rejects_unrelated_same_doc_live_entry"
check_contains crates/core/src/source_control/staging/target/tests.rs "doc_target_rejects_unrelated_same_doc_live_entry"
check_contains crates/core/src/plugin/runtime/host/tests.rs "source_control_write_gate_rejects_broken_workspace_identity"
check_contains crates/core/tests/source_control_target_lookup_canonical_test.rs "workdir_diff_target_rejects_doc_id_when_requested_path_is_not_in_change_set"

# Local repo execution selectors must fail closed on repo_id/repo_name mismatch.
check_absent crates/core/src/ledger/manager/repo_scope_selector_runtime.rs "ignored stale repo_name"
check_contains crates/core/src/ledger/manager/repo_scope_selector_runtime.rs "select_local_repo_name_for_execution(&candidates)"
check_contains crates/core/tests/local_repo_selector_heal_test.rs "resolve_local_repo_name_for_execution_rejects_selector_mismatch"
check_contains apps/cli/src/server/tests/source_control/source_control_http_test/status.rs "test_http_status_rejects_selector_mismatch"
check_contains apps/cli/src/server/tests/source_control/source_control_http_test/status.rs "ServerErrorCode::ScRepoContextInvalid"
check_contains apps/cli/src/server/handlers/source_control/present/resolve.rs "matched multiple doc rename successors"
check_contains apps/cli/src/server/handlers/source_control/present/resolve_extra_tests.rs "resolve_target_fails_closed_when_doc_id_has_multiple_rename_successors"
check_contains apps/cli/src/server/handlers/source_control/present/resolve_extra_tests.rs "resolve_target_fails_closed_when_doc_id_matches_exact_and_successor"
check_contains apps/cli/src/server/handlers/source_control/service/target.rs "current_entry(entries, resolved)"
check_contains apps/cli/src/server/handlers/source_control/service/target/tests/related.rs "related_targets_keep_resolved_doc_id_when_old_path_is_reused"
check_contains crates/core/src/ledger/manager/source_control_workdir_diff.rs "workdir_diff_inputs_for_resolved_target"
check_contains crates/core/tests/source_control_target_lookup_canonical_test.rs "workdir_diff_payload_preserves_doc_id_when_resolved_path_is_reused"
check_contains apps/cli/src/server/handlers/source_control/diff/remote_content.rs "Remote document target path mismatch"
check_contains apps/cli/src/server/handlers/source_control/diff/remote_test_extra.rs "remote_diff_rejects_doc_id_path_mismatch"
check_contains crates/core/src/ledger/manager/commit_preflight.rs "preflight_staged_upsert_identity"
check_contains crates/core/src/ledger/manager/commit_preflight.rs "lacks rename evidence"
check_contains crates/core/tests/source_control_commit_apply_error_test.rs "commit_staged_rejects_upsert_target_when_path_is_bound_to_another_doc"
check_contains crates/core/tests/source_control_commit_apply_error_test.rs "commit_staged_rejects_upsert_move_without_rename_evidence"
check_contains docs/plan/05_diff_logic.md "legacy \`Deleted + doc_id=None\` 的 exact delete selector"
check_contains crates/core/src/ledger/manager/source_control_path_target.rs "has_legacy_docless_exact_delete"
check_contains crates/core/src/ledger/manager/source_control_path_target/tests.rs "path_wrapper_promotes_docless_non_delete_to_tracked_identity"

echo "source-control-baseline-check: ok"
