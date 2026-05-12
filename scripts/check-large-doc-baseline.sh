#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "large-doc-baseline-check: $*" >&2
  exit 1
}

contains() {
  local file="$1"
  local pattern="$2"
  rg -q --fixed-strings -- "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

line_no() {
  local file="$1"
  local pattern="$2"
  local line
  line="$(rg -n --fixed-strings -- "$pattern" "$ROOT_DIR/$file" | head -n 1 | cut -d: -f1 || true)"
  [[ -n "$line" ]] || fail "missing '$pattern' in $file"
  printf '%s\n' "$line"
}

before() {
  local file="$1"
  local first="$2"
  local second="$3"
  local first_line
  local second_line
  first_line="$(line_no "$file" "$first")"
  second_line="$(line_no "$file" "$second")"
  (( first_line < second_line )) \
    || fail "'$first' must appear before '$second' in $file"
}

# DIFF-008: large documents show snapshot first, then replay delta ops in batches.
contains docs/acceptance-cases/04_diff.md "case_id: DIFF-008"
contains docs/acceptance-cases/04_diff.md "scripts/check-large-doc-baseline.sh"
contains docs/acceptance-cases/04_diff.md "cargo test -p deve_web large_doc_search_gate -- --nocapture"
contains docs/acceptance-cases/03_rendering.md "case_id: RENDER-LARGE-001"
contains docs/acceptance-cases/03_rendering.md "scripts/check-large-doc-baseline.sh"
contains docs/acceptance-cases/03_rendering.md "cargo test -p deve_web large_doc_search_gate -- --nocapture"
contains docs/acceptance-cases/04_diff.md "snapshot_first true"
contains docs/acceptance-cases/04_diff.md "search_disabled_until_prefetch_complete true"
contains docs/features/operations/rendering_large_doc_search_gate.md "apps/web/src/hooks/use_core/callbacks/misc.rs"
contains apps/web/src/editor/sync/snapshot.rs "applyRemoteContent(&message.new_content);"
contains apps/web/src/editor/sync/snapshot.rs 'ctx.set_load_state.set("partial".to_string())'
contains apps/web/src/editor/sync/snapshot.rs "apply_ops_in_batches("
before apps/web/src/editor/sync/snapshot.rs "applyRemoteContent(&message.new_content);" "apply_ops_in_batches("

# Prefetch must remain incremental instead of blocking the open path.
contains apps/web/src/editor/prefetch.rs "Timeout::new(0, task).forget();"
contains apps/web/src/editor/sync/snapshot.rs "initial_batch: 16"
contains apps/web/src/editor/sync/snapshot.rs "max_batch: 256"

# Search is a runtime gate: non-ready load states must not send ClientMessage::Search.
contains apps/web/src/hooks/use_core/callbacks/misc.rs 'load_state.get_untracked() != "ready"'
contains apps/web/src/hooks/use_core/callbacks/misc.rs 'show_search_block(set_sync_banner, "snapshot loading")'
contains apps/web/src/hooks/use_core/callbacks/misc.rs "ClientMessage::Search"
contains apps/web/src/hooks/use_core/callbacks/misc/tests.rs "large_doc_search_gate_blocks_until_prefetch_ready"
contains apps/web/src/hooks/use_core/callbacks/misc/tests.rs "drain_sent_for_test().is_empty()"
contains apps/web/src/hooks/use_core/callbacks/misc/tests.rs "large_doc_search_gate_sends_after_ready"

echo "large-doc-baseline-check: ok"
