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

# DIFF-009: Command Palette Git actions remain Planned / Optional. Source Control panel
# owns the current stage/commit/publish entry points.
check_contains docs/plan/12_commands.md "Git Sync / Commit / Push 与 AI Retry / Backend / PLAN / BUILD 面板命令仍属于"
check_contains apps/web/src/components/command_palette/registry.rs "merge_peer_command"
check_absent apps/web/src/components/command_palette/registry.rs "Git: Sync"
check_absent apps/web/src/components/command_palette/registry.rs "Git: Commit"
check_absent apps/web/src/components/command_palette/registry.rs "Git: Push"
check_absent apps/web/src/i18n/command_palette.rs "Git: Sync"
check_absent apps/web/src/i18n/command_palette.rs "Git: Commit"
check_absent apps/web/src/i18n/command_palette.rs "Git: Push"

# Current Source Control panel operations are explicit and repo-scoped.
check_contains docs/plan/07_diff_logic.md "CommitStaged"
check_contains docs/features/operations/sc_commit.md 'ClientMessage::Commit { scope_nonce }'
check_contains apps/web/src/hooks/use_core/callbacks_sc_write_commit.rs "ClientMessage::Commit"
check_contains apps/web/src/hooks/use_core/callbacks_sc_write_commit.rs "ClientMessage::CommitAndPush"
check_contains apps/web/src/hooks/use_core/callbacks_sc_write_commit.rs "source_control_scope_nonce(scope)"
check_contains apps/cli/src/server/ws/route/source_control.rs "ClientMessage::CommitAndPush { message, .. }"
check_contains apps/cli/src/server/ws/route/source_control.rs "source_control::handle_commit_and_push"
check_contains apps/web/src/components/sidebar/source_control/commit_actions.rs "on_commit_and_push.run(())"
check_contains apps/web/src/i18n/source_control.rs "Commit & Push"

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

echo "source-control-baseline-check: ok"
