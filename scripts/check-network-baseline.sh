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
check_contains apps/web/src/api/connection.rs ".try_set(signals.set_status, ConnectionStatus::Connecting)"
check_contains apps/web/src/api/connection.rs ".try_set(signals.set_status, ConnectionStatus::Disconnected)"
check_contains apps/web/src/components/disconnect_overlay.rs "ConnectionStatus::Unauthorized | ConnectionStatus::Connected => None"
check_contains apps/web/src/hooks/use_core/write_gate/logic.rs "ConnectionStatus::Disconnected => Some(RepoWriteBlock::Offline)"
check_contains apps/web/src/hooks/use_core/write_gate/logic.rs "ConnectionStatus::Connecting => Some(RepoWriteBlock::Reconnecting)"

# NET-002: production path is same-origin /ws; localhost fallbacks are debug-only.
check_contains apps/web/src/api/connection_urls.rs "format!(\"{}://{}/ws\", ws_scheme, host)"
check_contains apps/web/src/api/connection_urls.rs "cfg!(debug_assertions)"
check_contains apps/web/src/api/connection_urls.rs "include_debug_fallbacks"
check_contains apps/web/src/api/connection_urls.rs "if include_debug_fallbacks"
check_contains apps/web/src/api/connection_urls.rs "ws_port"
check_contains apps/web/src/api/connection_urls.rs "fn parse_ws_port(value: &str) -> Option<u16>"
check_contains apps/web/src/api/connection_urls.rs "query_ws_port_rejects_invalid_or_zero_ports"
check_absent apps/web/src/api/connection_urls.rs "Scanning ports"

# NET-003: role endpoint remains public and exposes the main/proxy route contract.
check_contains apps/cli/src/server/router.rs ".route(\"/api/node/role\", get(node_role_http::role))"
check_contains apps/cli/src/server/router.rs "public = Router::new()"
check_contains apps/cli/src/server/node_role_http.rs "\"role\": r.role"
check_contains apps/cli/src/server/node_role_http.rs "\"ws_port\": r.ws_port"
check_contains apps/cli/src/server/node_role_http.rs "\"main_port\": r.main_port"

# NET-004: frame protocol is versioned binary by default, with all JSON text frames debug-gated.
check_contains crates/core/src/protocol/frame.rs "pub const WS_PROTOCOL_VERSION: u16 = 9;"
check_contains crates/core/src/protocol/frame.rs "pub const MIN_SUPPORTED_WS_PROTOCOL_VERSION: u16 = 9;"
check_contains crates/core/src/protocol/frame.rs "pub const WS_FRAME_MAGIC: &[u8] = b\"DEVEWSF3\";"
check_contains crates/core/src/protocol/frame.rs "missing WS frame magic"
check_contains apps/cli/src/server/ws/receive/mod.rs "MISSING_WS_FRAME_MAGIC"
check_contains apps/cli/src/server/ws/receive/tests/frame_errors.rs "Some(\"missing WS frame magic\")"
check_contains apps/cli/src/server/ws/receive/mod.rs "JSON WS text frames are disabled outside development debug mode"
check_contains apps/cli/src/server/ws/receive/mod.rs "allow_ws_json_text_debug"
check_contains apps/cli/src/server/ws/receive/mod.rs "ServerErrorCode::SyncVersionMismatch"
check_contains apps/cli/src/server/ws/receive/mod.rs "ServerErrorCode::SyncInvalidPayload"
check_contains apps/cli/src/server/ws/receive/mod.rs "DEVE_ALLOW_WS_JSON_TEXT"
check_contains apps/cli/src/server/ws/receive/mod.rs "DEVE_ALLOW_LEGACY_WS_JSON"
check_contains apps/cli/src/server/ws/receive/mod.rs "DEVE_ENV"
check_contains apps/cli/src/server/ws/receive/tests/frame_errors.rs "unsupported_versioned_json_reports_version_mismatch"
check_contains apps/cli/src/server/ws/receive/tests/frame_errors.rs "malformed_versioned_binary_reports_invalid_payload"
check_contains apps/web/src/api/incoming/tests.rs "binary_malformed_versioned_payload_surfaces_protocol_error"
check_contains apps/web/src/api/service.rs "writer_ready_scope_nonce"
check_contains apps/web/src/api/service.rs "writer_ready_for(&self, repo_id: Option<&str>, scope_nonce: Option<u64>)"
check_contains apps/web/src/hooks/use_core/effects/message_dispatch_write.rs "ws.mark_writer_ready(repo_id, scope_nonce, peer_id.as_str())"
check_contains apps/web/src/hooks/use_core/status_summary.rs "PeerNotRegistered"
check_contains apps/web/src/components/bottom_bar/status.rs "data-deve-peer-registration-retry=\"true\""
check_contains apps/web/src/components/mobile_layout/footer_status.rs "data-deve-peer-registration-retry=\"mobile\""
check_contains apps/web/src/hooks/use_core/effects/handshake/mod.rs "handshake_retry_nonce"
check_contains apps/web/src/hooks/use_core/state_build/assemble.rs "build_retry_peer_registration_callback"
check_contains apps/cli/src/server/session/writer.rs "pub scope_nonce: u64"
check_contains apps/cli/src/server/session/scope.rs "writer_peer_id_for(&self, repo_id: &RepoId, scope_nonce: Option<u64>)"
check_contains apps/cli/src/server/handlers/sync/writer/mod.rs "session.set_writer_identity(repo_id, peer_id.clone(), scope_nonce)"
check_contains apps/cli/src/server/handlers/document/edit_checks.rs ".writer_peer_id_for(repo_id, requested_scope_nonce)"
check_contains apps/cli/src/server/handlers/document/edit_apply.rs "with_repo_write_gate(repo_id, || append_client_edit_locked(input))"
check_contains apps/cli/src/server/handlers/document/write_gate.rs "HashMap<RepoId, Arc<Mutex<()>>>"
check_contains apps/cli/src/server/handlers/document/write_gate.rs "fn repo_write_gate_serializes_same_repo()"
check_contains docs/acceptance-cases/14_operation_flow_refs.md "cargo test -p deve_cli repo_write_gate_serializes_same_repo -- --nocapture"
check_contains apps/cli/src/server/ws/route/core_scoped.rs "document::handle_edit("
check_contains apps/cli/src/server/handlers/sync/snapshot.rs "snapshot_kind: Some(\"full\".to_string())"
check_contains crates/core/src/protocol/session_proof.rs "pub struct SessionProof"
check_contains crates/core/src/protocol/client.rs "peer_pubkey: Vec<u8>"
check_contains crates/core/src/protocol/client.rs "session_proof: SessionProof"
check_contains apps/cli/src/server/ws/route/mod.rs "session_proof"

# NET-005: WebLightPeer repo-scoped handshake must bind SyncHello, WriteReady,
# and ShadowList to the active repo scope.
check_contains scripts/smoke-runtime-happy-path.sh "run_test deve_cli ws_endpoint_sync_hello_uses_switched_repo_scope"
check_contains scripts/smoke-runtime-happy-path.sh "run_test deve_cli ws_endpoint_register_writer_after_sync_hello_returns_write_ready"
check_contains apps/cli/src/server/tests/ws_acceptance/ws_sync_hello_acceptance_test.rs "async fn ws_endpoint_sync_hello_uses_switched_repo_scope"
check_contains apps/cli/src/server/tests/ws_acceptance/ws_register_writer_acceptance_test.rs "async fn ws_endpoint_register_writer_after_sync_hello_returns_write_ready"

# NET-006: OpenDoc must remain snapshot-first and reject wrong or deleted docs
# without fabricating an empty snapshot.
check_contains scripts/smoke-runtime-happy-path.sh "run_test deve_cli ws_open_doc_and_history_read_back_registered_edit"
check_contains apps/cli/src/server/tests/ws_acceptance/ws_edit_readback_acceptance_test.rs "async fn ws_open_doc_and_history_read_back_registered_edit"
check_contains apps/cli/src/server/test_modules.rs "mod open_doc_scope_test;"
check_contains scripts/check-storage-repo-baseline.sh "case_contains STORE-009 \"cargo test -p deve_cli open_doc_scope -- --nocapture\""

# NET-007: missing-fact transfer must stay repo-scoped and preserve requested
# source peer identity.
check_contains apps/cli/src/server/tests/sync/sync_transfer_scope_test.rs "async fn non_browser_sync_request_uses_bound_sync_scope_nonce_for_push"
check_contains apps/cli/src/server/tests/sync/sync_transfer_scope_test.rs "async fn sync_request_preserves_requested_source_peer_in_push"
check_contains apps/cli/src/server/tests/ws_acceptance/ws_sync_transfer_reject_acceptance_test.rs "async fn ws_sync_request_requires_sync_hello_scope"
check_contains apps/cli/src/server/tests/ws_acceptance/ws_sync_transfer_reject_acceptance_test.rs "async fn ws_sync_request_rejects_wrong_repo_after_sync_hello"

# NET-008: snapshot fallback must preserve repo scope, requested source peer,
# and structured rejection for unoffered sources.
check_contains apps/cli/src/server/tests/sync/sync_transfer_scope_test.rs "async fn non_browser_snapshot_request_uses_bound_sync_scope_nonce_for_push"
check_contains apps/cli/src/server/tests/sync/sync_transfer_snapshot_test.rs "async fn snapshot_request_exports_requested_shadow_source"
check_contains apps/cli/src/server/tests/sync/sync_transfer_snapshot_test.rs "async fn snapshot_request_rejects_unoffered_source"
check_contains apps/cli/src/server/handlers/sync/snapshot.rs "snapshot_kind: Some(\"full\".to_string())"

# NET-009: multi-repo switching must reject stale scope and stale runtime
# bindings after repo changes.
check_contains apps/cli/src/server/tests/sync/sync_hello_browser_test.rs "async fn browser_sync_hello_rejects_stale_scope_nonce"
check_contains apps/cli/src/server/tests/sync/sync_hello_browser_scope_test.rs "async fn browser_sync_hello_rejects_stale_active_db_binding"
check_contains apps/cli/src/server/tests/sync/sync_hello_browser_scope_test.rs "async fn browser_sync_hello_rejects_stale_bound_repo_and_writer_identity"

# NET-010: inbound remote data must be buffered under the source peer shadow,
# never under transport peer or local ledger authority.
check_contains apps/cli/src/server/tests/sync/sync_transfer_push_test.rs "async fn manual_sync_push_buffers_without_applying_remote_ops"
check_contains apps/cli/src/server/tests/sync/sync_transfer_push_test.rs "async fn sync_push_uses_message_source_peer_for_shadow_write"
check_contains apps/cli/src/server/tests/sync/sync_transfer_push_test.rs "async fn sync_push_does_not_pollute_transport_or_local_ledger"
check_contains apps/cli/src/server/tests/sync/sync_transfer_snapshot_test.rs "async fn sync_push_snapshot_uses_message_source_peer_for_shadow_replace"

# NET-011: indirect sync must reject unrequested sources and forged source
# proofs before writing shadow data.
check_contains apps/cli/src/server/tests/ws_acceptance/ws_sync_transfer_reject_acceptance_test.rs "async fn ws_sync_push_rejects_unrequested_source"
check_contains apps/cli/src/server/tests/sync/sync_transfer_push_test.rs "async fn sync_push_rejects_unrequested_relay_source"
check_contains apps/cli/src/server/tests/sync/sync_transfer_push_test.rs "async fn sync_push_rejects_relay_forged_source_proof"
check_contains apps/cli/src/server/tests/sync/sync_transfer_snapshot_test.rs "async fn sync_push_snapshot_rejects_relay_forged_source_proof"
check_contains crates/core/src/protocol/sync_push_header/tests.rs "fn source_proof_rejects_payload_tamper"

# NET-012: WebSocket failures must surface as structured ProtocolError values.
check_contains scripts/check-ws-structured-errors.sh "ws-structured-errors-check: ok"
check_contains apps/cli/src/server/ws/route/core_scoped/tests.rs "core_scoped_scope_nonce_gate_rejects_missing_scope_before_handler"
check_contains apps/cli/src/server/ws/route/core_scoped/tests.rs "core_scoped_scope_nonce_gate_rejects_stale_scope_before_handler"

# NET-013: auth failure must enter Unauthorized/session-expired state rather
# than being flattened into ordinary reconnect.
check_contains scripts/check-auth-unauthorized-state.sh "auth-unauthorized-check: ok"
check_contains scripts/smoke-runtime-recovery-path.sh "deve_web \\"
check_contains scripts/smoke-runtime-recovery-path.sh "status_summary \\"
check_contains scripts/smoke-runtime-recovery-path.sh "auth_probe \\"

# NET-014/015/016: FullPeer mesh v1 must be explicitly documented before the
# runtime connector surface is opened.
check_contains docs/plan/07_network.md "Full Peer Mesh v1"
check_contains docs/plan/07_network.md "FullPeer"
check_contains docs/plan/07_network.md "Admission"
check_contains docs/plan/07_network.md "shadow repo"
check_contains docs/features/05_network.md "Full Peer Mesh v1"
check_contains docs/features/05_network.md "Shadow 与显式合并边界"
check_contains docs/acceptance-cases/06_network.md "case_id: NET-014"
check_contains docs/acceptance-cases/06_network.md "case_id: NET-015"
check_contains docs/acceptance-cases/06_network.md "case_id: NET-016"
check_contains docs/acceptance-bindings.tsv "NET-014|manual-network|docs/features/05_network.md"
check_contains docs/acceptance-bindings.tsv "NET-015|manual-network|docs/features/05_network.md"
check_contains docs/acceptance-bindings.tsv "NET-016|manual-network|docs/features/05_network.md"
check_contains apps/cli/src/server/ws/auth.rs "pub(super) enum WsAdmission"
check_contains apps/cli/src/server/ws/auth.rs "FullPeer"
check_contains apps/cli/src/server/ws/auth.rs "DEVE_P2P_INBOUND_TOKEN"
check_contains apps/cli/src/server/ws/mod.rs "admission.browser_auth_session().cloned()"
check_contains apps/cli/src/server/ws/mod.rs "session.mark_browser_session()"
check_contains apps/cli/src/server/ws/mod.rs "session.bind_auth_session(auth_session_id)"
check_contains apps/cli/src/server/router.rs ".route(\"/ws\", get(ws::ws_handler))"
check_contains apps/cli/src/server/p2p.rs "spawn_mesh_connectors"
check_contains apps/cli/src/server/p2p.rs "P2P mesh connector handshake completed"
check_contains apps/cli/src/server/p2p.rs "with_strict_engine(repo_id"
check_contains apps/cli/src/server/p2p.rs "encode_client_binary(&hello)"

echo "network-baseline-check: ok"
