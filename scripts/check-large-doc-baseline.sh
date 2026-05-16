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
contains docs/acceptance-cases/03_rendering.md "snapshot_first true"
contains docs/acceptance-cases/03_rendering.md "progressive_replay_enabled true"
contains docs/acceptance-cases/03_rendering.md "search_disabled_until_prefetch_complete true"
contains docs/acceptance-cases/03_rendering.md "case_id: RENDER-LARGE-002"
contains docs/acceptance-cases/03_rendering.md "cargo test -p deve_web snapshot_apply_failure -- --nocapture"
contains docs/acceptance-cases/03_rendering.md "remote_batch_apply_returns_failure true"
contains docs/acceptance-cases/03_rendering.md "failed_batch_does_not_advance_version_or_history true"
contains docs/acceptance-cases/03_rendering.md "full_snapshot_fallback_requested true"
if rg --quiet --fixed-strings -- "virtual_render_enabled true" "$ROOT_DIR/docs/acceptance-cases/03_rendering.md"; then
  fail "RENDER-LARGE-001 must not claim complete virtual rendering"
fi
contains docs/acceptance-cases/04_diff.md "snapshot_first true"
contains docs/acceptance-cases/04_diff.md "search_disabled_until_prefetch_complete true"
contains docs/features/operations/rendering_large_doc_search_gate.md "apps/web/src/hooks/use_core/callbacks/misc.rs"
contains docs/features/operations/rendering_large_doc_prefetch.md "RENDER-LARGE-002"
contains docs/features/operations/rendering_large_doc_prefetch.md "op.render.large-doc.delta-fallback"
contains docs/features/operations/rendering_large_doc_prefetch.md "Failed delta replay must not advance local version or history."
contains docs/features/operations/rendering_large_doc_prefetch.md "snapshot reopen is only a last-resort fallback"
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

# Failed remote op batches must be observable and must recover through full-content fallback.
contains apps/web/src/editor/sync/snapshot.rs "build_delta_failure_fallback("
contains apps/web/src/editor/sync/snapshot.rs "try_apply_content_ops"
contains apps/web/src/editor/sync/snapshot_finish.rs "complete_with_content"
contains apps/web/src/editor/sync/snapshot.rs "ClientMessage::OpenDoc"
contains apps/web/src/editor/sync/snapshot_apply.rs "snapshot_apply_failure_does_not_advance_version_or_history"
contains apps/web/src/editor/sync/snapshot/tests.rs "snapshot_delta_fallback_reconstructs_full_content"
contains apps/web/src/editor/sync/history_replay.rs "Pending overlay replay batch failed"
contains apps/web/src/editor/ffi.rs "pub fn applyRemoteOpsBatch(ops_json: &str) -> bool;"
contains apps/web/index.html "return false;"
contains apps/web/index.html "Queued editor ops batch replay failed"
contains apps/web/js/editor_remote_ops.js "return false;"
contains apps/web/js/editor_remote_ops.js "ensureValidRange"
contains apps/web/js/editor_remote_ops.js "Unsupported remote op"
contains apps/web/builder.js "entryPoints: ['js/editor_adapter.js']"
contains apps/web/builder.js "outfile: 'js/editor.bundle.js'"

echo "large-doc-baseline-check: ok"
