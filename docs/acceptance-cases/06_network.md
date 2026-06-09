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
    - run: cargo test -p deve_web message_refresh -- --nocapture
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
    - versioned_packet_protocol_version_eq: 9
    - min_supported_packet_protocol_version_eq: 9
    - text_legacy_json_debug_only: true
    - production_rejects_text_legacy_json: true
    - reject_binary_without_magic: true
    - unsupported_protocol_version_error_code: "SYNC_VERSION_MISMATCH"
    - malformed_versioned_payload_error_code: "SYNC_INVALID_PAYLOAD"

- case_id: NET-005
  goal: WebLightPeer repo-scoped 握手。
  preconditions:
    - 用户打开 Repo A，服务端已返回当前 repo 的 switch_nonce/scope_nonce
  steps:
    - ws_connect: "relative /ws"
    - ws_send: { type: "SwitchRepoExact", name: "notes", repo_id: "11111111-1111-1111-1111-111111111111", switch_nonce: 1 }
    - ws_send: { type: "SyncHello", peer_id: "web-light-peer", peer_pubkey: "signed_pubkey_bytes", session_proof: { signature: "signature_bytes" }, vector: { peer: "web-light-peer", seq: 7 }, repo_id: "11111111-1111-1111-1111-111111111111", scope_nonce: 1 }
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
    - sync_push_source_peer_id_eq_requested_source: true
    - new_sync_request_frames_include_known_vector: true
    - wrong_or_unbound_repo_returns_structured_protocol_error: true

- case_id: NET-008
  goal: Snapshot fallback 必须保留 repo_id。
  preconditions:
    - 增量同步不可继续，当前 peer/source 已由握手或 offer 流程授权
  steps:
    - ws_send: { type: "SyncSnapshotRequest", source_peer_id: "peer-a", repo_id: "11111111-1111-1111-1111-111111111111", known_vector: { peer: "web-light-peer", seq: 3 } }
    - run: cargo test -p deve_cli non_browser_snapshot_request_uses_bound_sync_scope_nonce_for_push -- --nocapture
    - run: cargo test -p deve_cli snapshot_request_exports_requested_shadow_source -- --nocapture
    - run: cargo test -p deve_cli snapshot_request_rejects_unoffered_source -- --nocapture
  assertions:
    - ws_receive_contains: { type: "SyncPushSnapshot", repo_id: "11111111-1111-1111-1111-111111111111", source_peer_id: "peer-a", scope_nonce: 1, server_vector: "present", snapshot_kind: "full" }
    - sync_push_snapshot_source_peer_id_eq_requested_source: true
    - unoffered_source_returns_structured_protocol_error: true

- case_id: NET-009
  goal: 多仓库切换必须重新握手并隔离状态。
  preconditions:
    - 浏览器已先后打开 Repo A 与 Repo B
  steps:
    - ws_send: { type: "SwitchRepoExact", name: "repo-a", repo_id: "11111111-1111-1111-1111-111111111111", switch_nonce: 1 }
    - ws_send: { type: "SyncHello", peer_id: "web-light-peer", peer_pubkey: "signed_pubkey_bytes", session_proof: { signature: "signature_bytes" }, vector: { peer: "web-light-peer", seq: 7 }, repo_id: "11111111-1111-1111-1111-111111111111", scope_nonce: 1 }
    - ws_send: { type: "SwitchRepoExact", name: "repo-b", repo_id: "22222222-2222-2222-2222-222222222222", switch_nonce: 2 }
    - ws_send: { type: "SyncHello", peer_id: "web-light-peer", peer_pubkey: "signed_pubkey_bytes", session_proof: { signature: "signature_bytes" }, vector: { peer: "web-light-peer", seq: 1 }, repo_id: "22222222-2222-2222-2222-222222222222", scope_nonce: 2 }
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
    - ws_send: { type: "SyncPush", source_peer_id: "malicious-source", repo_id: "11111111-1111-1111-1111-111111111111", header: { repo_id: "11111111-1111-1111-1111-111111111111", peer_id: "malicious-source", vector: {}, payload_kind: "diff" }, encrypted_payload: ["encrypted_ledger_facts"] }
    - run: cargo test -p deve_cli sync_push_does_not_pollute_transport_or_local_ledger -- --nocapture
    - run: cargo test -p deve_cli sync_push_uses_message_source_peer_for_shadow_write -- --nocapture
    - run: cargo test -p deve_cli sync_push_snapshot_uses_message_source_peer_for_shadow_replace -- --nocapture
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
    - ws_send: { type: "SyncPush", source_peer_id: "unrequested-source", repo_id: "11111111-1111-1111-1111-111111111111", header: { repo_id: "11111111-1111-1111-1111-111111111111", peer_id: "unrequested-source", vector: {}, payload_kind: "diff" }, encrypted_payload: [] }
    - ws_send: { type: "SyncSnapshotRequest", source_peer_id: "unoffered-source", repo_id: "11111111-1111-1111-1111-111111111111", known_vector: {}, reason: "source-boundary-check" }
    - run: cargo test -p deve_cli ws_sync_push_rejects_unrequested_source -- --nocapture
    - run: cargo test -p deve_cli sync_push_rejects_unrequested_relay_source -- --nocapture
    - run: cargo test -p deve_cli sync_push_rejects_relay_forged_source_proof -- --nocapture
    - run: cargo test -p deve_cli sync_push_snapshot_rejects_relay_forged_source_proof -- --nocapture
    - run: cargo test -p deve_core source_proof -- --nocapture
    - run: cargo test -p deve_cli snapshot_request_rejects_unoffered_source -- --nocapture
  assertions:
    - ws_receive_contains: { type: "ProtocolError", code: "SyncPeerUnauthenticated", scope_nonce: 1 }
    - unrequested_source_not_written: true
    - forged_source_proof_returns_invalid_payload: true
    - relay_cannot_force_receive: true
    - note: "GossipOffer/FetchRequest relay offer protocol is not implemented in the current protocol surface; this case validates the current fail-closed source authorization boundary."

- case_id: NET-012
  goal: WebSocket 错误必须走结构化 ProtocolError。
  preconditions:
    - 连接已建立
    - 触发 source control 错误（如暂存不存在的 pending）
  steps:
    - run: scripts/check-ws-structured-errors.sh
    - run: cargo test -p deve_cli core_scoped_scope_nonce_gate -- --nocapture
  assertions:
    - stdout_contains: "ws-structured-errors-check: ok"
    - core_scoped_missing_or_stale_scope_returns_protocol_error: true
    - malformed_ws_payload_returns_structured_error: true

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

- case_id: NET-014
  goal: server-to-server FullPeer `/ws` admission 必须独立于 Browser session admission。
  preconditions:
    - 两个服务端使用静态 P2P peer 配置
    - 入站服务端设置 `DEVE_P2P_INBOUND_TOKEN`
  steps:
    - ws_connect: { role: "FullPeer", authorization: "Bearer <token>", path: "/ws" }
    - ws_send: { type: "SyncHello", peer_id: "peer-a", repo_id: "11111111-1111-1111-1111-111111111111", vector: {}, source_proof: "signed" }
    - run: cargo test -p deve_cli ws_acceptance -- --nocapture
    - run: cargo test -p deve_cli p2p_mesh -- --nocapture
    - run: cargo test -p deve_cli p2p_node_role_summary -- --nocapture
    - run: cargo test -p deve_cli p2p_exchange_rejects_frame_limit_without_sync_hello -- --nocapture
    - run: cargo test -p deve_cli p2p_exchange_rejects_authenticated_self_loop -- --nocapture
    - run: cargo test -p deve_cli p2p_connector_error_classifier_keeps_auth_separate -- --nocapture
    - run: cargo test -p deve_cli p2p_exchange_rejects_forged_sync_push_source -- --nocapture
    - run: cargo test -p deve_cli p2p_exchange_rejects_forged_snapshot_source -- --nocapture
    - run: scripts/check-network-baseline.sh
  assertions:
    - full_peer_session_browser_flag_eq: false
    - bad_or_missing_full_peer_token_rejected_before_upgrade: true
    - browser_cookie_admission_not_accepted_as_full_peer: true
    - sync_hello_signature_and_repo_scope_still_required: true
    - writer_registration_not_granted_by_full_peer_admission: true
    - api_assert: p2p_node_role_summary_omits_token_material true
    - api_assert: full_peer_exchange_requires_sync_hello true
    - api_assert: authenticated_self_loop_rejected true
    - api_assert: self_loop_status_is_not_reconnecting true
    - api_assert: p2p_inbound_sync_push_source_attribution_checked true
    - api_assert: p2p_inbound_snapshot_source_attribution_checked true

- case_id: NET-015
  goal: FullPeer mesh 入站远端 facts 只写 shadow repo，不自动污染本地 branch。
  preconditions:
    - peer-a 与 peer-b 使用相同 `RepoId`
    - 两端 ledger volume 独立
    - P2P static peer 配置启用
  steps:
    - run: DEVE_DOCKER_P2P_MESH_REQUIRED=1 bash scripts/smoke-docker-p2p-mesh.sh
    - run: cargo test -p deve_cli sync_hello_pushes_source_control_commit_to_full_peer -- --nocapture
    - run: cargo test -p deve_cli sync -- --nocapture
    - run: cargo test -p deve_cli p2p_mesh -- --nocapture
  assertions:
    - peer_b_shadow_contains_peer_a_write: true
    - peer_b_local_branch_unchanged_before_explicit_merge: true
    - explicit_merge_makes_remote_content_local_visible: true
    - source_attribution_uses_origin_peer_not_transport_peer: true

- case_id: NET-016
  goal: FullPeer mesh 断线重连后必须重新握手并对齐 vector。
  preconditions:
    - peer-a 与 peer-b 已完成一次 mesh 同步
  steps:
    - docker_network_disconnect: "peer-b"
    - docker_network_reconnect: "peer-b"
    - run: DEVE_DOCKER_P2P_MESH_REQUIRED=1 bash scripts/smoke-docker-p2p-mesh.sh
  assertions:
    - reconnect_uses_backoff_not_busy_loop: true
    - reconnect_sends_fresh_sync_hello: true
    - vector_aligned_after_reconnect: true
    - no_automatic_local_merge_after_reconnect: true
```
