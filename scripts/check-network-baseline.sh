#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "network-baseline-check: $*" >&2
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
    fail "forbidden '$pattern' in $file"
  fi
}

# NET-001: reconnect UI and write gate must distinguish network states.
check_contains apps/web/src/api/connection.rs "set_status.set(ConnectionStatus::Connecting);"
check_contains apps/web/src/api/connection.rs "set_status.set(ConnectionStatus::Disconnected);"
check_contains apps/web/src/components/disconnect_overlay.rs "ConnectionStatus::Unauthorized | ConnectionStatus::Connected => None"
check_contains apps/web/src/hooks/use_core/write_gate_logic.rs "ConnectionStatus::Disconnected => Some(RepoWriteBlock::Offline)"
check_contains apps/web/src/hooks/use_core/write_gate_logic.rs "ConnectionStatus::Connecting => Some(RepoWriteBlock::Reconnecting)"

# NET-002: production path is same-origin /ws; localhost fallbacks are debug-only.
check_contains apps/web/src/api/connection_urls.rs "format!(\"{}://{}/ws\", ws_scheme, host)"
check_contains apps/web/src/api/connection_urls.rs "if cfg!(debug_assertions)"
check_contains apps/web/src/api/connection_urls.rs "ws_port"
check_absent apps/web/src/api/connection_urls.rs "Scanning ports"

# NET-003: role endpoint remains public and exposes the main/proxy route contract.
check_contains apps/cli/src/server/router.rs ".route(\"/api/node/role\", get(node_role_http::role))"
check_contains apps/cli/src/server/router.rs "public = Router::new()"
check_contains apps/cli/src/server/node_role_http.rs "\"role\": r.role"
check_contains apps/cli/src/server/node_role_http.rs "\"ws_port\": r.ws_port"
check_contains apps/cli/src/server/node_role_http.rs "\"main_port\": r.main_port"

# NET-004: frame protocol is versioned binary by default, with legacy JSON debug-gated.
check_contains crates/core/src/protocol/frame.rs "pub const WS_PROTOCOL_VERSION: u16 = 3;"
check_contains crates/core/src/protocol/frame.rs "pub const WS_FRAME_MAGIC: &[u8] = b\"DEVEWSF3\";"
check_contains crates/core/src/protocol/frame.rs "missing WS frame magic"
check_contains apps/cli/src/server/ws/receive.rs "MISSING_WS_FRAME_MAGIC"
check_contains apps/cli/src/server/ws/receive_frame_test.rs "Some(\"missing WS frame magic\")"
check_contains apps/cli/src/server/ws/receive.rs "WsFrameFormat::LegacyJsonText"
check_contains apps/cli/src/server/ws/receive.rs "DEVE_ALLOW_LEGACY_WS_JSON"
check_contains apps/cli/src/server/ws/receive.rs "DEVE_ENV"
check_contains apps/web/src/api/service.rs "writer_ready_scope_nonce"
check_contains apps/web/src/api/service.rs "writer_ready_for(&self, repo_id: Option<&str>, scope_nonce: Option<u64>)"
check_contains apps/web/src/hooks/use_core/effects/message_dispatch_write.rs "ws.mark_writer_ready(repo_id, scope_nonce, peer_id.as_str())"
check_contains apps/cli/src/server/session_writer.rs "pub scope_nonce: u64"
check_contains apps/cli/src/server/session_scope.rs "writer_peer_id_for(&self, repo_id: &RepoId, scope_nonce: Option<u64>)"
check_contains apps/cli/src/server/handlers/sync/writer.rs "session.set_writer_identity(repo_id, peer_id.clone(), scope_nonce)"
check_contains apps/cli/src/server/handlers/document/edit_checks.rs ".writer_peer_id_for(repo_id, scope_nonce)"
check_contains apps/cli/src/server/ws/route/core_scoped.rs "document::handle_edit("

echo "network-baseline-check: ok"
