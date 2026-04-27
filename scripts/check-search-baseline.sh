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

check_contains apps/cli/src/server/handlers/search.rs "ServerMessage::SearchResults"
check_contains apps/cli/src/server/handlers/search.rs "repo_id: Some(scope.repo_id)"
check_contains apps/cli/src/server/handlers/search.rs "branch: scope.branch.clone()"
check_contains apps/cli/src/server/handlers/search.rs "scope_nonce"
check_contains apps/cli/src/server/handlers/search.rs "Search feature disabled for current runtime profile"
check_contains apps/cli/src/server/handlers/search.rs "Search feature not enabled"

check_contains apps/web/src/hooks/use_core/effects/message_dispatch_gate.rs "scope_nonce == Some(signals.current_scope_nonce.get_untracked())"
check_contains apps/web/src/hooks/use_core/effects/message_dispatch_gate.rs "repo_id.map(|id| id.to_string()) == signals.current_repo_id.get_untracked()"
check_contains apps/web/src/hooks/use_core/effects/message_dispatch_gate.rs "branch == signals.active_branch.get_untracked()"
check_contains apps/web/src/hooks/use_core/effects/message_dispatch_gate.rs "signals.search_request_id.get_untracked().as_deref() == Some(request_id)"

check_contains apps/web/src/hooks/use_core/effects/message_protocol.rs "signals.set_search_results.set(Vec::new());"
check_contains apps/web/src/i18n/search.rs "Search unavailable"
check_contains docs/acceptance-cases/16_search.md "Tantivy 增量索引仍是 future optimization"

echo "search-baseline-check: ok"
