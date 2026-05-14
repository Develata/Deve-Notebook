#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "search-baseline-check: $*" >&2
  exit 1
}

check_contains() {
  local file="$1"
  local pattern="$2"
  rg -q --fixed-strings "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

check_absent() {
  local file="$1"
  local pattern="$2"
  if rg -q --fixed-strings "$pattern" "$ROOT_DIR/$file"; then
    fail "unexpected '$pattern' in $file"
  fi
}

check_contains apps/cli/src/server/handlers/search.rs "ServerMessage::SearchResults"
check_contains apps/cli/src/server/handlers/search.rs "repo_id: Some(scope.repo_id)"
check_contains apps/cli/src/server/handlers/search.rs "branch: scope.branch.clone()"
check_contains apps/cli/src/server/handlers/search.rs "scope_nonce"
check_contains apps/cli/src/server/handlers/search.rs "Search feature disabled for current runtime profile"
check_contains apps/cli/src/server/handlers/search.rs "Search feature not enabled"
check_contains apps/cli/src/server/handlers/search/tests/feature_enabled.rs "handler_returns_scoped_empty_results_for_blank_query_and_zero_limit"
check_contains apps/cli/src/server/handlers/search/tests/feature_enabled.rs "scope_search_orders_by_score_then_path_before_limit"
check_contains apps/cli/src/server/handlers/search/tests/feature_enabled.rs "scope_search_scans_remote_branch_documents"
check_contains apps/cli/src/server/ws/route/core/tests.rs "browser_search_rejects_stale_scope_before_handler"
check_contains apps/cli/src/server/state.rs "pub search_available: bool"
check_contains apps/cli/src/server/start.rs "Search baseline scan enabled"
check_absent apps/cli/src/server/start.rs "load_search_service"
check_absent apps/cli/src/server/state.rs "SearchService"

check_contains apps/web/src/hooks/use_core/effects/message_dispatch_gate/mod.rs "scope_nonce == Some(signals.current_scope_nonce.get_untracked())"
check_contains apps/web/src/hooks/use_core/effects/message_dispatch_gate/mod.rs "repo_id.map(|id| id.to_string()) == signals.current_repo_id.get_untracked()"
check_contains apps/web/src/hooks/use_core/effects/message_dispatch_gate/mod.rs "branch == signals.active_branch.get_untracked()"
check_contains apps/web/src/hooks/use_core/effects/message_dispatch_gate/mod.rs "signals.search_request_id.get_untracked().as_deref() == Some(request_id)"
check_contains apps/web/src/hooks/use_core/effects/message_dispatch_gate/tests.rs "rejects_search_results_while_scope_switch_is_pending"

check_contains apps/web/src/hooks/use_core/effects/message_protocol/mod.rs "signals.set_search_results.set(Vec::new());"
check_contains apps/web/src/i18n/search.rs "Search unavailable"
check_absent apps/web/src/components/search_box/result_item/sections.rs "detail_text.clone().unwrap()"
check_contains apps/web/src/components/search_box/result_item/sections.rs "let detail_view = detail_text"
check_contains docs/acceptance-cases/16_search.md "增量索引优化不作为本文件阻塞项"
check_contains docs/acceptance-cases/16_search.md "no_heavy_index_startup_for_baseline true"
check_contains docs/features/operations/search_query.md "不依赖常驻重型索引"

echo "search-baseline-check: ok"
