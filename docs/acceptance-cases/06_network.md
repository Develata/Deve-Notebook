## 网络连通与同步

```markdown
- case_id: NET-001
  goal: Web 端网络断连时锁屏并进入重连态。
  preconditions:
    - 已连接 WS
  steps:
    - net_block_ws: true
    - run: scripts/check-network-baseline.sh
    - run: cargo test -p deve_web write_gate -- --nocapture
  assertions:
    - ui_assert: overlay_text "Reconnecting..."
    - ui_assert: editing_disabled true

- case_id: NET-002
  goal: 生产连接必须走 relative /ws 或单一配置端点。
  preconditions:
    - 生产环境部署
  steps:
    - browser_open: "/"
    - run: scripts/check-network-baseline.sh
  assertions:
    - ws_url_eq: "/ws"
    - log_not_contains: "Scanning ports"

- case_id: NET-003
  goal: Main/Proxy 角色切换不改变浏览器路由契约。
  preconditions:
    - 同源服务暴露 `/ws`
  steps:
    - ws_connect: "relative /ws"
    - run: curl http://127.0.0.1/api/node/role
    - run: scripts/check-network-baseline.sh
  assertions:
    - ws_connect_success: true
    - stdout_contains: "role"

- case_id: NET-004
  goal: 协议格式区分。
  preconditions:
    - Server-to-Server 与 Client-Server 连接已建立
  steps:
    - net_capture: true
    - run: scripts/check-network-baseline.sh
    - run: cargo test -p deve_core frame -- --nocapture
    - run: cargo test -p deve_cli receive -- --nocapture
  assertions:
    - packet_format_eq: ["server", "versioned-bincode"]
    - packet_format_any_of: ["client", "versioned-bincode", "text-versioned-json-debug"]
    - binary_packet_magic_eq: "DEVEWSF3"
    - versioned_packet_protocol_version_eq: 3
    - text_legacy_json_debug_only: true
    - production_rejects_text_legacy_json: true
    - reject_binary_without_magic: true

- case_id: NET-005
  goal: WebLightPeer repo-scoped 握手。
  preconditions:
    - 用户打开 Repo A，服务端已返回当前 repo 的 switch_nonce/scope_nonce
  steps:
    - ws_connect: "relative /ws"
    - ws_send: { type: "SwitchRepoExact", name: "notes", repo_id: "11111111-1111-1111-1111-111111111111", switch_nonce: 1 }
    - ws_send: { type: "SyncHello", peer_id: "web-light-peer", pub_key: "signed_pubkey_bytes", signature: "signature_bytes", vector: { peer: "web-light-peer", seq: 7 }, repo_id: "11111111-1111-1111-1111-111111111111", scope_nonce: 1 }
    - ws_send: { type: "RegisterWriter", peer_id: "web-light-peer", repo_id: "11111111-1111-1111-1111-111111111111", scope_nonce: 1 }
    - run: cargo test -p deve_cli ws_endpoint_sync_hello_uses_switched_repo_scope -- --nocapture
    - run: cargo test -p deve_cli ws_endpoint_register_writer_after_sync_hello_returns_write_ready -- --nocapture
  assertions:
    - ws_receive_contains: { type: "SyncHello", repo_id: "11111111-1111-1111-1111-111111111111", scope_nonce: 1 }
    - ws_receive_contains: { type: "WriteReady", repo_id: "11111111-1111-1111-1111-111111111111", scope_nonce: 1 }
    - ws_receive_contains: { type: "ShadowList", scope_nonce: 1 }

- case_id: NET-006
  goal: OpenDoc Snapshot-First。
  preconditions:
    - 文档有快照与增量 Content Facts，浏览器处于当前 repo scope
  steps:
    - ws_send: { type: "OpenDoc", doc_id: "doc_uuid", request_id: 42, scope_nonce: 1 }
    - run: cargo test -p deve_cli ws_open_doc_and_history_read_back_registered_edit -- --nocapture
    - run: cargo test -p deve_cli open_doc_scope -- --nocapture
  assertions:
    - ws_receive_order: ["Snapshot", "History"]
    - ws_receive_contains: { type: "Snapshot", repo_id: "active_repo_id", doc_id: "doc_uuid", request_id: 42, scope_nonce: 1 }
    - wrong_or_deleted_doc_returns_protocol_error: true
    - no_empty_snapshot_for_wrong_doc: true

- case_id: NET-007
  goal: Vector Gossip 缺失 Ledger Facts 必须 repo-scoped。
  preconditions:
    - Repo A 中本地 VC 大于远端，SyncHello 已绑定当前 sync scope
  steps:
    - ws_send: { type: "SyncRequest", repo_id: "11111111-1111-1111-1111-111111111111", known_vector: { peer: "web-light-peer", seq: 3 }, requests: [["peer-a", [4, 7]]] }
    - run: cargo test -p deve_cli non_browser_sync_request_uses_bound_sync_scope_nonce_for_push -- --nocapture
    - run: cargo test -p deve_cli sync_request_preserves_requested_source_peer_in_push -- --nocapture
    - run: cargo test -p deve_cli ws_sync_request -- --nocapture
  assertions:
    - ws_receive_contains: { type: "SyncPush", repo_id: "11111111-1111-1111-1111-111111111111", scope_nonce: 1 }
    - sync_push_peer_id_eq_requested_source: true
    - new_sync_request_frames_include_known_vector: true
    - wrong_or_unbound_repo_returns_structured_protocol_error: true

- case_id: NET-008
  goal: Snapshot fallback 必须保留 repo_id。
  preconditions:
    - 增量同步不可继续，当前 peer/source 已由握手或 offer 流程授权
  steps:
    - ws_send: { type: "SyncSnapshotRequest", peer_id: "peer-a", repo_id: "11111111-1111-1111-1111-111111111111", known_vector: { peer: "web-light-peer", seq: 3 } }
    - run: cargo test -p deve_cli non_browser_snapshot_request_uses_bound_sync_scope_nonce_for_push -- --nocapture
    - run: cargo test -p deve_cli snapshot_request_exports_requested_shadow_source -- --nocapture
    - run: cargo test -p deve_cli snapshot_request_rejects_unoffered_source -- --nocapture
  assertions:
    - ws_receive_contains: { type: "SyncPushSnapshot", repo_id: "11111111-1111-1111-1111-111111111111", peer_id: "peer-a", scope_nonce: 1, server_vector: "present" }
    - sync_push_snapshot_peer_id_eq_requested_source: true
    - unoffered_source_returns_structured_protocol_error: true

- case_id: NET-009
  goal: 多仓库切换必须重新握手并隔离状态。
  preconditions:
    - 浏览器已先后打开 Repo A 与 Repo B
  steps:
    - ws_send: { type: "SwitchRepoExact", name: "repo-a", repo_id: "11111111-1111-1111-1111-111111111111", switch_nonce: 1 }
    - ws_send: { type: "SyncHello", peer_id: "web-light-peer", pub_key: "signed_pubkey_bytes", signature: "signature_bytes", vector: { peer: "web-light-peer", seq: 7 }, repo_id: "11111111-1111-1111-1111-111111111111", scope_nonce: 1 }
    - ws_send: { type: "SwitchRepoExact", name: "repo-b", repo_id: "22222222-2222-2222-2222-222222222222", switch_nonce: 2 }
    - ws_send: { type: "SyncHello", peer_id: "web-light-peer", pub_key: "signed_pubkey_bytes", signature: "signature_bytes", vector: { peer: "web-light-peer", seq: 1 }, repo_id: "22222222-2222-2222-2222-222222222222", scope_nonce: 2 }
    - run: cargo test -p deve_cli browser_sync_hello_rejects_stale_scope_nonce -- --nocapture
    - run: cargo test -p deve_cli browser_sync_hello_rejects_stale_active_db_binding -- --nocapture
    - run: cargo test -p deve_cli browser_sync_hello_rejects_stale_bound_repo_and_writer_identity -- --nocapture
  assertions:
    - ws_receive_contains: { type: "SyncHello", repo_id: "22222222-2222-2222-2222-222222222222", scope_nonce: 2 }
    - stale_repo_a_scope_returns_structured_protocol_error: true
    - stale_runtime_binding_cleared: true

- case_id: NET-010
  goal: 恶意数据隔离。
  preconditions:
    - 入站 SyncPush 已通过当前 sync scope 授权，但 payload source 是远端分支
  steps:
    - ws_send: { type: "SyncPush", peer_id: "malicious-source", repo_id: "11111111-1111-1111-1111-111111111111", ops: ["encrypted_ledger_facts"] }
    - run: cargo test -p deve_cli sync_push_does_not_pollute_transport_or_local_ledger -- --nocapture
    - run: cargo test -p deve_cli manual_sync_push_buffers_without_applying_remote_ops -- --nocapture
  assertions:
    - shadow_written_under_source_peer: "ledger/remotes/malicious-source"
    - transport_peer_shadow_not_written: true
    - local_ledger_not_modified_by_inbound_push: true
    - merge_to_local_requires_explicit_user_action: true

- case_id: NET-011
  goal: 间接同步信任边界。
  preconditions:
    - relay transport 已认证，但 source peer 未被当前 SyncHello diff 请求或授权
  steps:
    - ws_send: { type: "SyncPush", peer_id: "unrequested-source", repo_id: "11111111-1111-1111-1111-111111111111", ops: [] }
    - ws_send: { type: "SyncSnapshotRequest", peer_id: "unoffered-source", repo_id: "11111111-1111-1111-1111-111111111111" }
    - run: cargo test -p deve_cli ws_sync_push_rejects_unrequested_source -- --nocapture
    - run: cargo test -p deve_cli sync_push_rejects_unrequested_relay_source -- --nocapture
    - run: cargo test -p deve_cli snapshot_request_rejects_unoffered_source -- --nocapture
  assertions:
    - ws_receive_contains: { type: "ProtocolError", code: "SyncPeerUnauthenticated", scope_nonce: 1 }
    - unrequested_source_not_written: true
    - relay_cannot_force_receive: true
    - note: "GossipOffer/FetchRequest relay offer protocol is not implemented in the current protocol surface; this case validates the current fail-closed source authorization boundary."

- case_id: NET-012
  goal: WebSocket 错误必须走结构化 ProtocolError。
  preconditions:
    - 连接已建立
    - 触发 source control 错误（如暂存不存在的 pending）
  steps:
    - run: scripts/check-ws-structured-errors.sh
  assertions:
    - stdout_contains: "ws-structured-errors-check: ok"

- case_id: NET-013
  goal: 认证失效必须进入 Unauthorized，而不是普通重连。
  preconditions:
    - 已建立登录态与 WS 连接
  steps:
    - run: curl -s -X POST http://127.0.0.1:3000/api/auth/logout
    - browser_wait_ws_event: true
    - run: scripts/check-auth-unauthorized-state.sh
    - run: cargo test -p deve_web auth_probe -- --nocapture
    - run: cargo test -p deve_web status_summary -- --nocapture
  assertions:
    - ui_assert: login_screen_visible true
    - ui_assert: overlay_text_not_eq "Reconnecting..."
    - auth_probe_401_403_or_auth_error_eq_invalid: true
    - websocket_send_failure_does_not_force_disconnected_before_auth_probe: true
    - unauthorized_status_triggers_session_expired: true
```
