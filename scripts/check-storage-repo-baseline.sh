#!/usr/bin/env bash
set -euo pipefail

# STORE-001..010 keep storage/repo acceptance bound to the current CLI and
# test surface. Do not resurrect pseudo commands such as `deve repo create`.

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
    || fail "missing '$text' in ${file#$ROOT_DIR/}"
}

not_contains() {
  local file="$1"
  local text="$2"
  if rg --fixed-strings --quiet "$text" "$file"; then
    fail "unexpected '$text' in ${file#$ROOT_DIR/}"
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
case_contains STORE-001 "cargo test -p deve_core trinity_dir_structure_after_init -- --nocapture"
case_contains STORE-002 "cargo test -p deve_core init_allocates_collision_safe_repo_name_for_same_name_different_url -- --nocapture"
case_contains STORE-003 "cargo test -p deve_core required_redb_tables_exist_after_init -- --nocapture"
case_contains STORE-004 "cargo test -p deve_core snapshot_respects_depth_limit -- --nocapture"
case_contains STORE-005 "cargo test -p deve_core edit_round_trip_reconstructs_content -- --nocapture"
case_contains STORE-005 "cargo test -p deve_core global_seq_increases -- --nocapture"
case_contains STORE-006 "cargo test -p deve_cli markdown_export_preserves_user_frontmatter_without_system_metadata -- --nocapture"
case_contains STORE-007 "cargo test -p deve_core watcher_records_create_modify_delete_candidates -- --nocapture"
case_contains STORE-007 "cargo test -p deve_core watcher_respects_deveignore_for_matching_markdown -- --nocapture"
case_contains STORE-007 "cargo test -p deve_core watcher_startup_scan_respects_deveignore -- --nocapture"
case_contains STORE-008 "cargo test -p deve_cli recover_rebuilds_workspace_files_from_ledger -- --nocapture"
case_contains STORE-008 "cargo test -p deve_core rebuild_projection_recovers_when_node_projection_is_missing -- --nocapture"
case_contains STORE-009 "cargo test -p deve_cli document_scope_bootstrap -- --nocapture"
case_contains STORE-009 "cargo test -p deve_cli open_doc_scope -- --nocapture"
case_contains STORE-009 "cargo test -p deve_cli resolve_target_prefers_doc_id_over_stale_path -- --nocapture"
case_contains STORE-010 "cargo test -p deve_core path_normalize_structure -- --nocapture"

for case_id in STORE-001 STORE-002 STORE-003 STORE-004 STORE-005 STORE-006 STORE-007 STORE-008 STORE-009 STORE-010; do
  case_contains "$case_id" "run: scripts/check-storage-repo-baseline.sh"
done

contains "$ROOT_DIR/apps/cli/src/commands/init.rs" "fn init_creates_trinity_workspace_layout()"
contains "$ROOT_DIR/apps/cli/src/commands/recover.rs" "fn recover_rebuilds_workspace_files_from_ledger()"
contains "$ROOT_DIR/apps/cli/src/commands/export/tests.rs" "fn markdown_export_preserves_user_frontmatter_without_system_metadata()"
contains "$ROOT_DIR/crates/core/tests/local_repo_metadata_repair_test.rs" "fn init_allocates_collision_safe_repo_name_for_same_name_different_url()"
contains "$ROOT_DIR/crates/core/tests/store_acceptance_test.rs" "SNAPSHOT_DATA"

not_contains "$ACCEPTANCE" "deve repo create"
not_contains "$ACCEPTANCE" "deve db inspect"
not_contains "$ACCEPTANCE" "deve doc edit"
not_contains "$ACCEPTANCE" "deve dump --doc"
not_contains "$ACCEPTANCE" "deve api call"
not_contains "$ACCEPTANCE" "deve path normalize"
not_contains "$ACCEPTANCE" "deve recover --from-ledger"
not_contains "$ACCEPTANCE" "powershell -Command"
not_contains "$ACCEPTANCE" 'dir "${DEVE_DATA_DIR}"'
not_contains "$ACCEPTANCE" 'type ${DEVE_DATA_DIR}'

echo "storage-repo-baseline-check: ok"
