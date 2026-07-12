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
    - run: cargo test -p deve_web writer_ready -- --nocapture
    - run: cargo test -p deve_web message_refresh -- --nocapture
  assertions:
    - ui_assert: overlay_text "Reconnecting..."
    - ui_assert: editing_disabled true
    - writer_ready_cleared_on_disconnected: true

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
    - packet_format_eq: ["server", "versioned-postcard"]
    - packet_format_any_of: ["client", "versioned-postcard", "text-versioned-json-debug"]
    - binary_packet_magic_eq: "DEVEWSF3"
    - versioned_packet_protocol_version_eq: 12
    - min_supported_packet_protocol_version_eq: 12
    - p2p_v1_protocol_policy_eq: "lockstep_until_version_adapter_exists"
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
    - ws_receive_contains: { type: "SyncPush", repo_id: "11111111-1111-1111-1111-111111111111", source_peer_id: "peer-a", range_start: 4, range_end: 7, scope_nonce: 1 }
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
    - ws_receive_contains: { type: "SyncPushSnapshot", repo_id: "11111111-1111-1111-1111-111111111111", source_peer_id: "peer-a", scope_nonce: 1, server_vector: "present", waterline: "present", snapshot_kind: "full" }
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
    - source_proof_signing_rejects_wrong_source_key: true
    - relay_cannot_force_receive: true
    - note: "GossipOffer/FetchRequest relay offer protocol is not implemented in the current protocol surface; this case validates the current fail-closed source authorization boundary."

- case_id: NET-012
  goal: WebSocket 错误必须走结构化 ProtocolError。
  preconditions:
    - 连接已建立
    - 触发 source control 错误（如暂存不存在的 pending）
  steps:
    - run: scripts/check-ws-structured-errors.sh
    - run: cargo run -p deve_baseline -- ws-structured-errors
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
    - run: cargo run -p deve_baseline -- auth-unauthorized-state
    - run: cargo test -p deve_web auth_probe -- --nocapture
    - run: cargo test -p deve_web writer_ready -- --nocapture
    - run: cargo test -p deve_web status_summary -- --nocapture
  assertions:
    - ui_assert: login_screen_visible true
    - ui_assert: overlay_text_not_eq "Reconnecting..."
    - auth_probe_401_403_or_auth_error_eq_invalid: true
    - websocket_send_failure_does_not_force_disconnected_before_auth_probe: true
    - unauthorized_status_triggers_session_expired: true
    - writer_ready_cleared_on_unauthorized: true

- case_id: NET-014
  goal: server-to-server FullPeer `/ws` admission 必须独立于 Browser session admission。
  preconditions:
    - 两个服务端使用静态 P2P peer 配置
    - 入站服务端设置 `DEVE_P2P_INBOUND_TOKEN`
  steps:
    - ws_connect: { role: "FullPeer", authorization: "Bearer <token>", path: "/ws" }
    - ws_send: { type: "SyncHello", peer_id: "peer-a", repo_id: "11111111-1111-1111-1111-111111111111", vector: {}, source_proof: "signed" }
    - run: cargo test -p deve_cli bearer_token_admits_full_peer_without_browser_session -- --nocapture
    - run: cargo test -p deve_cli bearer_token_uses_configured_inbound_token_env -- --nocapture
    - run: cargo test -p deve_cli invalid_bearer_token_rejects_full_peer -- --nocapture
    - run: cargo test -p deve_cli p2p_mesh -- --nocapture
    - run: cargo test -p deve_cli p2p_node_role_summary -- --nocapture
    - run: cargo test -p deve_cli p2p_status_marks_peers_disabled_when_mesh_disabled -- --nocapture
    - run: cargo test -p deve_cli p2p_status_duplicate_labels_do_not_share_state -- --nocapture
    - run: cargo test -p deve_cli p2p_connector_jitter_uses_peer_identity_not_label -- --nocapture
    - run: cargo test -p deve_core --lib load_checked_fails_closed_on_invalid_p2p_peer_id_identity -- --nocapture
    - run: cargo test -p deve_core --lib load_checked_fails_closed_on_duplicate_p2p_peer_identity_tuple -- --nocapture
    - run: cargo test -p deve_cli p2p_status_retry_preserves_last_error_until_success -- --nocapture
    - run: cargo test -p deve_cli p2p_exchange_rejects_frame_limit_without_sync_hello -- --nocapture
    - run: cargo test -p deve_cli p2p_exchange_responds_to_ping_without_aborting_handshake -- --nocapture
    - run: cargo test -p deve_cli p2p_exchange_rejects_request_before_sync_hello -- --nocapture
    - run: cargo test -p deve_cli p2p_exchange_rejects_duplicate_sync_hello -- --nocapture
    - run: cargo test -p deve_cli p2p_exchange_rejects_configured_peer_id_mismatch -- --nocapture
    - run: cargo test -p deve_cli p2p_exchange_rejects_invalid_sync_hello_signature -- --nocapture
    - run: cargo test -p deve_cli p2p_exchange_rejects_sync_hello_pubkey_peer_id_mismatch -- --nocapture
    - run: cargo test -p deve_cli p2p_exchange_rejects_repo_mismatch_after_sync_hello -- --nocapture
    - run: cargo test -p deve_cli p2p_exchange_rejects_authenticated_self_loop -- --nocapture
    - run: cargo test -p deve_cli p2p_connector_error_classifier_keeps_auth_separate -- --nocapture
    - run: cargo test -p deve_cli p2p_connector_retry_backoff_starts_at_one_second -- --nocapture
    - run: cargo test -p deve_cli p2p_connector_classifies_structured_protocol_errors -- --nocapture
    - run: cargo test -p deve_cli p2p_connector_identity_mismatch_is_terminal -- --nocapture
    - run: cargo test -p deve_cli p2p_connector_repo_mismatch_is_terminal -- --nocapture
    - run: cargo test -p deve_cli p2p_connector_static_config_errors_are_terminal -- --nocapture
    - run: cargo test -p deve_cli p2p_connector_static_token_header_errors_are_terminal -- --nocapture
    - run: cargo test -p deve_cli p2p_connector_unoffered_source_is_terminal -- --nocapture
    - run: cargo test -p deve_cli p2p_connector_unrequested_source_is_terminal -- --nocapture
    - run: cargo test -p deve_cli p2p_connector_source_proof_rejection_is_terminal -- --nocapture
    - run: cargo test -p deve_cli p2p_connector_duplicate_sync_hello_is_terminal -- --nocapture
    - run: cargo test -p deve_cli p2p_exchange_rejects_unoffered_sync_request_source -- --nocapture
    - run: cargo test -p deve_cli p2p_exchange_rejects_unoffered_snapshot_request_source -- --nocapture
    - run: cargo test -p deve_cli p2p_exchange_rejects_forged_sync_push_source -- --nocapture
    - run: cargo test -p deve_cli p2p_exchange_rejects_forged_snapshot_source -- --nocapture
    - run: cargo test -p deve_cli p2p_exchange_rejects_unrequested_direct_sync_push_source -- --nocapture
    - run: cargo test -p deve_cli p2p_exchange_rejects_unrequested_direct_snapshot_source -- --nocapture
    - run: cargo test -p deve_cli p2p_exchange_rejects_snapshot_missing_source_proof -- --nocapture
    - run: cargo test -p deve_cli p2p_fullpeer_offer_set_excludes_third_party_shadow_sources -- --nocapture
    - run: cargo test -p deve_cli sync_hello_fullpeer_offer_set_excludes_third_party_shadow_sources -- --nocapture
    - run: cargo test -p deve_core relay_proxy -- --nocapture
    - run: scripts/check-network-baseline.sh
  assertions:
    - full_peer_session_browser_flag_eq: false
    - bad_or_missing_full_peer_token_rejected_before_upgrade: true
    - browser_cookie_admission_not_accepted_as_full_peer: true
    - sync_hello_signature_and_repo_scope_still_required: true
    - writer_registration_not_granted_by_full_peer_admission: true
    - api_assert: p2p_node_role_summary_omits_token_material true
    - api_assert: p2p_status_global_disabled_marks_peers_disabled true
    - api_assert: p2p_status_keyed_by_peer_identity_not_label true
    - api_assert: p2p_connector_jitter_keyed_by_peer_identity_not_label true
    - api_assert: static_p2p_peer_id_human_label_rejected true
    - api_assert: duplicate_static_p2p_peer_identity_tuple_rejected true
    - api_assert: p2p_status_retry_preserves_last_error_until_success true
    - api_assert: full_peer_control_frames_do_not_abort_handshake true
    - api_assert: full_peer_exchange_requires_sync_hello true
    - api_assert: pre_hello_sync_request_rejected true
    - api_assert: duplicate_sync_hello_does_not_reset_p2p_source_sets true
    - api_assert: configured_peer_id_is_expected_authenticated_identity true
    - api_assert: full_peer_connector_verifies_sync_hello_proof true
    - api_assert: p2p_connector_retry_backoff_starts_at_one_second true
    - api_assert: sync_hello_pubkey_peer_id_mismatch_rejected true
    - api_assert: configured_peer_id_mismatch_not_reconnecting true
    - api_assert: static_p2p_config_errors_not_reconnecting true
    - api_assert: static_p2p_invalid_token_header_not_reconnecting true
    - api_assert: post_hello_repo_mismatch_rejected true
    - api_assert: authenticated_self_loop_rejected true
    - api_assert: self_loop_status_is_not_reconnecting true
    - api_assert: p2p_sync_request_source_must_be_offered true
    - api_assert: p2p_snapshot_request_source_must_be_offered true
    - api_assert: p2p_unoffered_source_status_is_not_reconnecting true
    - api_assert: p2p_unrequested_source_status_is_not_reconnecting true
    - api_assert: p2p_structured_source_boundary_error_not_masked_as_handshake_failure true
    - api_assert: p2p_inbound_sync_push_source_attribution_checked true
    - api_assert: p2p_inbound_snapshot_source_attribution_checked true
    - api_assert: p2p_inbound_sync_push_source_must_be_requested true
    - api_assert: p2p_inbound_snapshot_source_must_be_requested true
    - api_assert: p2p_inbound_snapshot_source_proof_required true
    - api_assert: p2p_fullpeer_does_not_offer_unprovable_third_party_shadow_source true
    - api_assert: server_sync_hello_does_not_offer_unprovable_third_party_shadow_source true
    - api_assert: p2p_source_attribution_helper_shared_by_server_and_connector true
    - api_assert: p2p_source_proof_rejected_status_is_not_reconnecting true
    - api_assert: p2p_duplicate_sync_hello_status_is_not_reconnecting true

- case_id: NET-015
  goal: FullPeer mesh 入站远端 facts 只写 shadow repo，不自动污染本地 branch。
  preconditions:
    - peer-a 与 peer-b 使用相同 `RepoId`
    - 两端 ledger volume 独立
    - P2P static peer 配置启用
  steps:
    - run: DEVE_DOCKER_P2P_MESH_REQUIRED=1 bash scripts/smoke-docker-p2p-mesh.sh
    - run: cargo test -p deve_cli merge_peer_local_branch_contract_writes_local_only -- --nocapture
    - run: cargo test -p deve_cli sync_hello_pushes_source_control_commit_to_full_peer -- --nocapture
    - run: cargo test -p deve_cli sync -- --nocapture
    - run: cargo test -p deve_cli p2p_mesh -- --nocapture
  assertions:
    - peer_b_shadow_contains_peer_a_write: true
    - peer_b_local_branch_unchanged_before_explicit_merge: true
    - explicit_merge_makes_remote_content_local_visible: true
    - source_attribution_uses_origin_peer_not_transport_peer: true

- case_id: NET-016
  goal: FullPeer peer 重启后必须重新完成 authenticated mesh handshake。
  preconditions:
    - peer-a 与 peer-b 已完成一次 mesh 同步
  steps:
    - docker_service_stop: "peer-b"
    - docker_service_start: "peer-b"
    - run: DEVE_DOCKER_P2P_MESH_REQUIRED=1 bash scripts/smoke-docker-p2p-mesh.sh
  assertions:
    - reconnect_sends_fresh_authenticated_sync_hello: true
    - evidence_gap: live post-reconnect vector equality is not exposed by the current diagnostic surface; NET-017 covers apply-side vector monotonicity

- case_id: NET-017
  goal: 入站 remote facts 落库满足 apply 端单调性、连续性与 confirmed-prefix equality（plan 07_network 7.1）——陈旧或乱序到达的 snapshot 不回退持久化 peer waterline、不 reset 更新的 shadow；增量 ops 不越过 seq 空洞，冲突重复整批拒绝。
  preconditions:
    - 一个配置了 RepoKey 的 sync engine，shadow ledger 已持有该 peer 的若干 ops
  steps:
    - run: cargo test -p deve_core sync::engine::manual -- --nocapture
    - run: cargo test -p deve_core sync::buffer::tests -- --nocapture
  assertions:
    - stale_snapshot_does_not_regress_peer_vector: true
    - stale_snapshot_does_not_wipe_newer_shadow_ops: true
    - incremental_apply_rejects_seq_gap: true
    - replayed_remote_ops_skip_duplicate_shadow_append: true
    - snapshot_base_allows_newer_contiguous_ops: true
    - run: cargo test -p deve_core --lib auto_conflicting_prefix_rejects_entire_incremental_batch -- --nocapture
    - run: cargo test -p deve_core --lib auto_newer_snapshot_cannot_rewrite_confirmed_prefix -- --nocapture
    - run: cargo test -p deve_core --lib manual_equal_waterline_snapshots_must_be_identical -- --nocapture
    - run: cargo test -p deve_core --lib persisted_shadow_waterline_blocks_stale_snapshot_from_another_engine -- --nocapture
    - api_assert: untrusted_version_vector_zero_unsorted_duplicate_rejected_before_diff true
    - api_assert: huge_or_over_budget_peer_range_fails_before_allocation true
    - api_assert: manual_pending_payload_fact_and_encoded_bytes_are_cumulative_and_bounded true
    - api_assert: failed_manual_merge_restores_payloads_and_all_resource_counters true
    - api_assert: transport_clone_does_not_copy_manual_pending_queue true

- case_id: NET-018
  goal: 同一 repo 的多个 browser session 即使拥有不同连接内 scope_nonce，也能接收彼此已经通过 writer gate 的实时广播。
  preconditions:
    - client A 与 client B 已登录同一 repo/branch
    - client B 曾断线重连，因此 B 的 scope_nonce 与 A 不同
  steps:
    - run: cargo test -p deve_cli recipient_scope_nonce_overrides_producer_nonce_for_runtime_broadcast -- --nocapture
    - run: DEVE_DOCKER_MULTI_REQUIRED=1 bash scripts/smoke-docker-multiclient.sh
  assertions:
    - producer_write_still_uses_producer_writer_gate: true
    - delivered_new_op_uses_recipient_scope_nonce: true
    - client_a_receives_client_b_post_reconnect_edit: true

- case_id: NET-019
  goal: 同一 repo 内物理 peer 的 content/structure facts 共享严格连续 PeerFactSeq；任意缺口阻塞后续事实，直到补齐或明确失败。
  preconditions:
    - peer-a 与 peer-b 使用相同 RepoId、独立 local ledger 与认证 FullPeer session
    - 测试入口可以遗漏 peer-a 的序号 N 并先投递 N+1
  steps:
    - run: cargo test -p deve_core peer_fact_seq -- --nocapture
    - run: cargo test -p deve_core sync::engine::manual -- --nocapture
    - run: cargo test -p deve_cli p2p_sequence_gap -- --nocapture
    - run: cargo test -p deve_core --test merge_checkpoint_test checkpoint_survives_reopen_and_anchor_is_in_peer_range -- --nocapture
    - run: DEVE_DOCKER_P2P_MESH_REQUIRED=1 DEVE_DOCKER_P2P_INJECT_SEQUENCE_GAP=1 bash scripts/smoke-docker-p2p-mesh.sh
  assertions:
    - content_and_structure_share_peer_sequence: true
    - failed_write_does_not_consume_peer_sequence: true
    - global_seq_not_used_as_sync_waterline: true
    - missing_range_entry_returns_sequence_gap: true
    - inbound_gap_keeps_shadow_and_vector_unchanged: true
    - smoke_waits_for_receiver_expected_n_observed_n_plus_one_rejection: true
    - gap_hold_uses_exact_shadow_content_not_substring_match: true
    - connector_request_and_full_peer_hello_push_fault_paths_are_labeled: true
    - restored_missing_fact_allows_contiguous_recovery: true
    - snapshot_requires_exact_source_range_1_through_waterline: true
    - merge_anchor_consumes_peer_fact_seq_and_roundtrips_in_full_log_snapshot: true
    - merge_anchor_does_not_mutate_content_or_structure_projection: true
```
