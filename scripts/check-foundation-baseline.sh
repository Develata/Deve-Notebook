#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TERM_ACCEPTANCE="$ROOT_DIR/docs/acceptance-cases/01_terminology.md"
POS_ACCEPTANCE="$ROOT_DIR/docs/acceptance-cases/02_positioning.md"

fail() {
  echo "foundation-baseline-check: $*" >&2
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
  local file="$1"
  local case_id="$2"
  awk -v id="$case_id" '
    $0 ~ "case_id: " id { in_case = 1 }
    in_case && $0 ~ "^- case_id: " && $0 !~ "case_id: " id { exit }
    in_case { print }
  ' "$file"
}

case_contains() {
  local file="$1"
  local case_id="$2"
  local text="$3"
  case_block "$file" "$case_id" | rg --fixed-strings --quiet "$text" \
    || fail "missing '$text' in $case_id"
}

# TERM-001..003: terminology and source-of-truth wording remain explicit.
case_contains "$TERM_ACCEPTANCE" TERM-001 "MUST / 必须"
case_contains "$TERM_ACCEPTANCE" TERM-001 "SHOULD / 应"
case_contains "$TERM_ACCEPTANCE" TERM-001 "MAY / 可选"
case_contains "$TERM_ACCEPTANCE" TERM-002 "Ledger"
case_contains "$TERM_ACCEPTANCE" TERM-002 "Projection"
case_contains "$TERM_ACCEPTANCE" TERM-002 "Vector Clock"
case_contains "$TERM_ACCEPTANCE" TERM-003 "唯一真值源"
contains "$ROOT_DIR/docs/plan/01_terminology.md" "**MUST / 必须**"
contains "$ROOT_DIR/docs/plan/01_terminology.md" "**SHOULD / 应**"
contains "$ROOT_DIR/docs/plan/01_terminology.md" "**MAY / 可选**"
contains "$ROOT_DIR/docs/plan/01_terminology.md" "**Ledger (账本)**"
contains "$ROOT_DIR/docs/plan/01_terminology.md" "**Snapshot (快照)**"
contains "$ROOT_DIR/docs/plan/01_terminology.md" "**Projection (投影)**"
contains "$ROOT_DIR/docs/plan/01_terminology.md" "**Vault (投影仓)**"
contains "$ROOT_DIR/docs/plan/01_terminology.md" "**DocId**"
contains "$ROOT_DIR/docs/plan/01_terminology.md" "**Path Mapping (路径映射)**"
contains "$ROOT_DIR/docs/plan/01_terminology.md" "**Peer (节点)**"
contains "$ROOT_DIR/docs/plan/01_terminology.md" "**Vector Clock (向量时钟)**"
contains "$ROOT_DIR/docs/plan/01_terminology.md" "系统唯一真值源（Source of Truth）"

# POS-001: init creates the trinity workspace layout.
case_contains "$POS_ACCEPTANCE" POS-001 "deve init"
case_contains "$POS_ACCEPTANCE" POS-001 "fs_exists:"
case_contains "$POS_ACCEPTANCE" POS-001 "ledger/local"
case_contains "$POS_ACCEPTANCE" POS-001 "ledger/remotes"
contains "$ROOT_DIR/apps/cli/src/commands/init.rs" "fn init_creates_trinity_workspace_layout()"

# POS-002/POS-003: external writes enter pending_fs_ops and Deve writeback is suppressed.
case_contains "$POS_ACCEPTANCE" POS-002 'pending_fs_ops_contains: "test.md"'
case_contains "$POS_ACCEPTANCE" POS-002 'ledger_op_not_appended: "test.md"'
case_contains "$POS_ACCEPTANCE" POS-003 'pending_fs_ops_count_increases_by: 1'
contains "$ROOT_DIR/crates/core/tests/watcher_create_modify_delete.rs" "fn watcher_records_create_modify_delete_candidates()"
contains "$ROOT_DIR/crates/core/tests/watcher_create_modify_delete.rs" "h.wait_pending(\"main\", \"notes/live.md\", ChangeStatus::Modified)?"
contains "$ROOT_DIR/crates/core/tests/watcher_writeback_loop.rs" "fn projection_writeback_events_are_suppressed()"
contains "$ROOT_DIR/crates/core/tests/watcher_writeback_loop.rs" "list_pending_fs_in_local_repo(\"main\")?.is_empty()"

# POS-004: rename pairing preserves DocId.
case_contains "$POS_ACCEPTANCE" POS-004 "watcher_pairs_rename_and_preserves_doc_identity"
contains "$ROOT_DIR/crates/core/tests/watcher_rename_pairing.rs" "fn watcher_pairs_rename_and_preserves_doc_identity()"
contains "$ROOT_DIR/crates/core/tests/watcher_rename_pairing.rs" "assert_eq!(added.doc_id, Some(doc_id));"
contains "$ROOT_DIR/crates/core/tests/watcher_rename_pairing.rs" "assert_eq!(deleted.doc_id, Some(doc_id));"

# POS-005: `.deveignore` filters watcher and startup scan ingress.
case_contains "$POS_ACCEPTANCE" POS-005 "pending_fs_ops_not_contains"
contains "$ROOT_DIR/crates/core/tests/watcher_internal_ignore.rs" "fn watcher_respects_deveignore_for_matching_markdown()"
contains "$ROOT_DIR/crates/core/tests/watcher_internal_ignore.rs" "fn watcher_startup_scan_respects_deveignore()"

# POS-006: heavyweight defaults remain outside the core path.
case_contains "$POS_ACCEPTANCE" POS-006 "Core MUST NOT"
contains "$ROOT_DIR/docs/plan/02_positioning.md" "Core MUST NOT"
contains "$ROOT_DIR/docs/plan/02_positioning.md" "AI 推理"
contains "$ROOT_DIR/docs/plan/02_positioning.md" "Full-Text Search"
contains "$ROOT_DIR/docs/plan/02_positioning.md" "Code Execution"
contains "$ROOT_DIR/docs/features/02_positioning.md" "默认不把 AI、全文索引、代码执行、复杂媒体处理作为核心必选能力"
not_contains "$ROOT_DIR/docs/features/02_positioning.md" "AI 是当前核心必选能力"
not_contains "$ROOT_DIR/docs/features/02_positioning.md" "代码执行是当前核心必选能力"

echo "foundation-baseline-check: ok"
