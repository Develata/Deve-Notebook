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
check_contains apps/cli/src/server/state.rs "pub search_available: bool"
check_contains apps/cli/src/server/start.rs "Search baseline scan enabled"
check_absent apps/cli/src/server/start.rs "load_search_service"
check_absent apps/cli/src/server/state.rs "SearchService"

check_contains apps/web/src/hooks/use_core/effects/message_dispatch_gate.rs "scope_nonce == Some(signals.current_scope_nonce.get_untracked())"
check_contains apps/web/src/hooks/use_core/effects/message_dispatch_gate.rs "repo_id.map(|id| id.to_string()) == signals.current_repo_id.get_untracked()"
check_contains apps/web/src/hooks/use_core/effects/message_dispatch_gate.rs "branch == signals.active_branch.get_untracked()"
check_contains apps/web/src/hooks/use_core/effects/message_dispatch_gate.rs "signals.search_request_id.get_untracked().as_deref() == Some(request_id)"

check_contains apps/web/src/hooks/use_core/effects/message_protocol.rs "signals.set_search_results.set(Vec::new());"
check_contains apps/web/src/i18n/search.rs "Search unavailable"
check_contains docs/acceptance-cases/16_search.md "Tantivy 增量索引仍是 future optimization"
check_contains docs/acceptance-cases/16_search.md "no_tantivy_index_startup_for_baseline true"
check_contains docs/features/operations/search_query.md "不在启动路径初始化 Tantivy 索引"

echo "search-baseline-check: ok"
