#!/usr/bin/env bash
set -euo pipefail

# Storage/repo acceptance cases stay bound to the current CLI and test surface.
# Do not resurrect pseudo commands such as `deve repo create`.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ACCEPTANCE="$ROOT_DIR/docs/acceptance-cases/07_storage_repo.md"

fail() {
  echo "storage-repo-baseline-check: $*" >&2
  exit 1
}

contains() {
  local file="$1"
  local text="$2"
  rg --fixed-strings --quiet "$text" "$file" \
    || fail "missing '$text' in ${file#"$ROOT_DIR"/}"
}

not_contains() {
  local file="$1"
  local text="$2"
  if rg --fixed-strings --quiet "$text" "$file"; then
    fail "unexpected '$text' in ${file#"$ROOT_DIR"/}"
  fi
}

case_block() {
  local case_id="$1"
  awk -v id="$case_id" '
    $0 ~ "case_id: " id { in_case = 1 }
    in_case && $0 ~ "^- case_id: " && $0 !~ "case_id: " id { exit }
    in_case { print }
  ' "$ACCEPTANCE"
}

case_contains() {
  local case_id="$1"
  local text="$2"
  case_block "$case_id" | rg --fixed-strings --quiet "$text" \
    || fail "missing '$text' in $case_id"
}

case_contains STORE-001 "cargo test -p deve_cli init_creates_trinity_workspace_layout -- --nocapture"
case_contains STORE-001 "cargo test -p deve_cli projection_locator_init_writes_locator_without_vault_path_config -- --nocapture"
case_contains STORE-001 "cargo test -p deve_core trinity_dir_structure_after_init -- --nocapture"
case_contains STORE-001 "cargo test -p deve_core projection_locator_toml_roundtrip -- --nocapture"
case_contains STORE-002 "cargo test -p deve_core init_allocates_collision_safe_repo_name_for_same_name_different_url -- --nocapture"
case_contains STORE-003 "cargo test -p deve_core required_redb_tables_exist_after_init -- --nocapture"
case_contains STORE-004 "cargo test -p deve_core snapshot_respects_depth_limit -- --nocapture"
case_contains STORE-005 "cargo test -p deve_core edit_round_trip_reconstructs_content -- --nocapture"
case_contains STORE-005 "cargo test -p deve_core global_seq_increases -- --nocapture"
case_contains STORE-006 "cargo test -p deve_cli markdown_export_preserves_user_frontmatter_without_system_metadata -- --nocapture"
case_contains STORE-007 "cargo test -p deve_core watcher_records_create_modify_delete_candidates -- --nocapture"
case_contains STORE-007 "cargo test -p deve_core watcher_duplicate_start_fails_and_can_restart_after_stop -- --nocapture"
case_contains STORE-007 "cargo test -p deve_core internal_repo_path_uses_segment_semantics -- --nocapture"
case_contains STORE-007 "cargo test -p deve_core --test watcher_internal_ignore -- --nocapture"
case_contains STORE-007 "cargo test -p deve_core --test watcher_internal_ignore watcher_respects_deveignore_for_matching_markdown -- --nocapture"
case_contains STORE-007 "cargo test -p deve_core --test watcher_internal_ignore watcher_startup_scan_respects_deveignore -- --nocapture"
case_contains STORE-008 "cargo test -p deve_cli recover_rebuilds_workspace_files_from_ledger -- --nocapture"
case_contains STORE-008 "cargo test -p deve_core rebuild_projection_recovers_when_node_projection_is_missing -- --nocapture"
case_contains STORE-009 "cargo test -p deve_cli document_scope_bootstrap -- --nocapture"
case_contains STORE-009 "cargo test -p deve_cli open_doc_scope -- --nocapture"
case_contains STORE-009 "cargo test -p deve_cli resolve_target_prefers_doc_id_over_stale_path -- --nocapture"
case_contains STORE-010 "cargo test -p deve_core --test path_normalize_structure_test -- --nocapture"
case_contains STORE-012 "cargo test -p deve_cli docs_scope_nonce_gate -- --nocapture"
case_contains STORE-012 "run: scripts/check-repo-file-ops-baseline.sh"
case_contains STORE-013 "cargo test -p deve_cli degraded_local -- --nocapture"
case_contains STORE-013 "cargo test -p deve_cli browser_writer_registration_rejects_broken_workspace_identity -- --nocapture"
case_contains STORE-013 "cargo test -p deve_core source_control_write_gate -- --nocapture"
case_contains STORE-013 "run: scripts/check-repo-file-ops-baseline.sh"
case_contains STORE-014 "cargo test -p deve_cli jsonl_roundtrip_is_monotonic_and_line_stable -- --nocapture"
case_contains STORE-014 "cargo test -p deve_cli includes_dir_structure_fact_in_export -- --nocapture"
case_contains STORE-015 "cargo test -p deve_cli edit_acknowledges_ledger_commit_when_workspace_writeback_fails -- --nocapture"
case_contains STORE-015 "cargo test -p deve_core --test durable_projection_fault_test -- --nocapture"
case_contains STORE-016 "cargo test -p deve_core notify_backend_error_requests_rescan -- --nocapture"
case_contains STORE-016 "cargo test -p deve_core notify_rescan_flag_requests_rescan -- --nocapture"
case_contains STORE-016 "cargo test -p deve_core watcher_rejects_zero_debounce_window -- --nocapture"
case_contains STORE-016 "cargo test -p deve_core dispatch_batch_collapses_modified_burst_by_content_hash -- --nocapture"
case_contains STORE-017 "cargo test -p deve_core remote_repo_catalog_calls_fail_closed_when_remotes_dir_is_missing -- --nocapture"
case_contains STORE-017 "cargo test -p deve_core remote_repo_listing_fails_closed_on_unexpected_non_redb_entry -- --nocapture"
case_contains STORE-017 "cargo test -p deve_cli quarantines_nil_shadow_repo_into_invalid_peer_dir -- --nocapture"

for case_id in STORE-001 STORE-002 STORE-003 STORE-004 STORE-005 STORE-006 STORE-007 STORE-008 STORE-009 STORE-010 STORE-014 STORE-015 STORE-016 STORE-017; do
  case_contains "$case_id" "run: scripts/check-storage-repo-baseline.sh"
done

contains "$ROOT_DIR/apps/cli/src/commands/init.rs" "fn init_creates_trinity_workspace_layout()"
contains "$ROOT_DIR/apps/cli/src/commands/repo_projection.rs" "fn projection_locator_set_list_check_roundtrip()"
contains "$ROOT_DIR/apps/cli/src/commands/recover.rs" "fn recover_rebuilds_workspace_files_from_ledger()"
contains "$ROOT_DIR/apps/cli/src/commands/export/tests.rs" "fn markdown_export_preserves_user_frontmatter_without_system_metadata()"
contains "$ROOT_DIR/crates/core/tests/local_repo_metadata_repair_test.rs" "fn init_allocates_collision_safe_repo_name_for_same_name_different_url()"
contains "$ROOT_DIR/crates/core/tests/store_acceptance_test.rs" "SNAPSHOT_DATA"
contains "$ROOT_DIR/crates/core/tests/watcher_lifecycle.rs" "fn watcher_duplicate_start_fails_and_can_restart_after_stop()"
contains "$ROOT_DIR/crates/core/tests/watcher_lifecycle.rs" "fn watcher_rejects_zero_debounce_window()"
contains "$ROOT_DIR/crates/core/src/sync/watcher/backend/notify_impl.rs" "fn notify_backend_error_requests_rescan()"
contains "$ROOT_DIR/crates/core/src/sync/watcher/backend/notify_impl.rs" "fn notify_rescan_flag_requests_rescan()"
contains "$ROOT_DIR/crates/core/src/sync/watcher/dispatch_burst_test.rs" "fn dispatch_batch_collapses_modified_burst_by_content_hash()"
contains "$ROOT_DIR/crates/core/src/utils/notegit.rs" "fn internal_repo_path_uses_segment_semantics()"
contains "$ROOT_DIR/crates/core/src/utils/notegit.rs" ".notegit-backup/state.json"
contains "$ROOT_DIR/crates/core/src/utils/notegit.rs" ".git-backup/config"
contains "$ROOT_DIR/crates/core/tests/watcher_internal_ignore.rs" "fn watcher_ignores_internal_notegit_paths()"
contains "$ROOT_DIR/crates/core/tests/watcher_internal_ignore.rs" "fn watcher_ignores_internal_git_paths()"
contains "$ROOT_DIR/crates/core/tests/watcher_internal_ignore.rs" "fn watcher_allows_notegit_backup_sibling_path()"
contains "$ROOT_DIR/crates/core/src/sync/watcher/filter.rs" "!is_internal_repo_path(normalized)"
contains "$ROOT_DIR/crates/core/src/sync/watcher/mod.rs" "registry::is_running(info.uuid)"
contains "$ROOT_DIR/crates/core/src/sync/watcher/mod.rs" "registry::begin_stop(repo_id)"
contains "$ROOT_DIR/crates/core/src/sync/watcher/mod.rs" "registry::finish_stop(repo_id)"
contains "$ROOT_DIR/crates/core/src/sync/watcher/mod.rs" "stop_handle(rejected)?"
contains "$ROOT_DIR/crates/core/src/sync/watcher/registry.rs" "WatcherSlot::Stopping"
contains "$ROOT_DIR/crates/core/src/ledger/manager/remote_repo_select.rs" "let Some(info) = entry.info.as_ref() else"
contains "$ROOT_DIR/apps/cli/src/export_entries.rs" "fn jsonl_roundtrip_is_monotonic_and_line_stable()"
contains "$ROOT_DIR/apps/cli/src/export_entries.rs" "fn includes_dir_structure_fact_in_export()"
contains "$ROOT_DIR/apps/cli/src/server/tests/edit/edit_projection_ack_test.rs" "fn edit_acknowledges_ledger_commit_when_workspace_writeback_fails()"
contains "$ROOT_DIR/apps/cli/src/server/handlers/document/edit_apply.rs" "CommitOutcome::WritebackFailed"
contains "$ROOT_DIR/apps/cli/src/server/handlers/document/edit_apply.rs" "emit_commit_outcome("
contains "$ROOT_DIR/apps/cli/src/server/handlers/document/write_confirmation.rs" "broadcast_and_ack_committed_edit("
contains "$ROOT_DIR/apps/cli/src/server/handlers/document/write_confirmation.rs" "report_projection_writeback_fault("
contains "$ROOT_DIR/crates/core/src/sync/projection_fault_journal.rs" "struct DurableProjectionFault"
contains "$ROOT_DIR/crates/core/tests/durable_projection_fault_test.rs" "fn durable_projection_fault_survives_sync_manager_restart()"
contains "$ROOT_DIR/crates/core/tests/remote_repo_catalog_missing_test.rs" "fn remote_repo_catalog_calls_fail_closed_when_remotes_dir_is_missing()"
contains "$ROOT_DIR/crates/core/tests/repo_catalog_entry_fail_closed_test.rs" "fn remote_repo_listing_fails_closed_on_unexpected_non_redb_entry()"
contains "$ROOT_DIR/apps/cli/src/commands/repair/shadow.rs" "fn quarantines_nil_shadow_repo_into_invalid_peer_dir()"
contains "$ROOT_DIR/scripts/check-repo-file-ops-baseline.sh" "run_filter deve_web file_ops"
contains "$ROOT_DIR/scripts/check-repo-file-ops-baseline.sh" "run_filter deve_web file_provider"
contains "$ROOT_DIR/scripts/check-repo-file-ops-baseline.sh" "run_filter deve_cli docs_scope_nonce_gate"
contains "$ROOT_DIR/scripts/check-repo-file-ops-baseline.sh" "run_filter deve_cli docs_create_test"
contains "$ROOT_DIR/scripts/check-repo-file-ops-baseline.sh" "run_filter deve_cli docs_copy_contract"
contains "$ROOT_DIR/scripts/check-repo-file-ops-baseline.sh" "run_filter deve_cli docs_dir_copy"
contains "$ROOT_DIR/scripts/check-repo-file-ops-baseline.sh" "run_filter deve_cli docs_projection_repair"
contains "$ROOT_DIR/scripts/check-repo-file-ops-baseline.sh" "run_filter deve_cli degraded_local"
contains "$ROOT_DIR/scripts/check-repo-file-ops-baseline.sh" "run_filter deve_cli browser_writer_registration_rejects_broken_workspace_identity"
contains "$ROOT_DIR/scripts/check-repo-file-ops-baseline.sh" "run_filter deve_core source_control_write_gate"
not_contains "$ROOT_DIR/crates/core/src/ledger/manager/remote_repo_select.rs" "expect(\"validated readable\")"

not_contains "$ACCEPTANCE" "deve repo create"
not_contains "$ACCEPTANCE" "deve db inspect"
not_contains "$ACCEPTANCE" "deve doc edit"
not_contains "$ACCEPTANCE" "deve dump --doc"
not_contains "$ACCEPTANCE" "deve api call"
not_contains "$ACCEPTANCE" "cargo test -p deve_core path_normalize_structure -- --nocapture"
not_contains "$ACCEPTANCE" "deve path normalize"
not_contains "$ACCEPTANCE" "deve recover --from-ledger"
not_contains "$ACCEPTANCE" "powershell -Command"
not_contains "$ACCEPTANCE" "dir \"\${DEVE_DATA_DIR}\""
not_contains "$ACCEPTANCE" "type \${DEVE_DATA_DIR}"

echo "storage-repo-baseline-check: ok"
