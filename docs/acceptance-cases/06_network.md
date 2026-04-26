## 网络连通与同步

```markdown
- case_id: NET-001
  goal: Web 端网络断连时锁屏并进入重连态。
  preconditions:
    - 已连接 WS
  steps:
    - net_block_ws: true
  assertions:
    - ui_assert: overlay_text "Reconnecting..."
    - ui_assert: editing_disabled true

- case_id: NET-002
  goal: 生产连接必须走 relative /ws 或单一配置端点。
  preconditions:
    - 生产环境部署
  steps:
    - browser_open: "/"
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
  assertions:
    - ws_connect_success: true
    - stdout_contains: "role"

- case_id: NET-004
  goal: 协议格式区分。
  preconditions:
    - Server-to-Server 与 Client-Server 连接已建立
  steps:
    - net_capture: true
  assertions:
    - packet_format_eq: ["server", "versioned-bincode"]
    - packet_format_any_of: ["client", "versioned-bincode", "text-versioned-json-debug"]
    - binary_packet_magic_eq: "DEVEWSF2"
    - versioned_packet_protocol_version_eq: 2
    - text_legacy_json_debug_only: true
    - production_rejects_text_legacy_json: true
    - reject_binary_without_magic: true

- case_id: NET-005
  goal: WebLightPeer repo-scoped 握手。
  preconditions:
    - 用户打开 Repo A
  steps:
    - ws_connect: "relative /ws"
    - ws_send: { type: "SyncHello", repo_id: "11111111-1111-1111-1111-111111111111", peer_pubkey: "pub_a", vector: { seq: 7 } }
  assertions:
    - ws_receive_contains: "SyncHello"
    - ws_receive_contains: "11111111-1111-1111-1111-111111111111"

- case_id: NET-006
  goal: OpenDoc Snapshot-First。
  preconditions:
    - 文档有快照与增量 Content Facts
  steps:
    - ws_send: { type: "OpenDoc", id: "doc_id" }
  assertions:
    - ws_receive_order: ["Snapshot", "NewOp"]

- case_id: NET-007
  goal: Vector Gossip 缺失 Ledger Facts 必须 repo-scoped。
  preconditions:
    - Repo A 中 A 的 VC 大于 B
  steps:
    - ws_send: { type: "SyncRequest", repo_id: "11111111-1111-1111-1111-111111111111", known_vector: { seq: 3 } }
  assertions:
    - ws_payload_contains_only_missing_facts true
    - ws_payload_contains: "11111111-1111-1111-1111-111111111111"

- case_id: NET-008
  goal: Snapshot fallback 必须保留 repo_id。
  preconditions:
    - LedgerSeq 差异超过阈值
  steps:
    - ws_send: { type: "SyncRequest", repo_id: "11111111-1111-1111-1111-111111111111", known_vector: { seq: 0 } }
  assertions:
    - ws_receive_contains: "Snapshot"
    - ws_receive_contains: "11111111-1111-1111-1111-111111111111"

- case_id: NET-009
  goal: 多仓库切换必须重新握手并隔离状态。
  preconditions:
    - 浏览器已先后打开 Repo A 与 Repo B
  steps:
    - ws_send: { type: "SyncHello", repo_id: "11111111-1111-1111-1111-111111111111", peer_pubkey: "pub_a", vector: { seq: 7 } }
    - ws_send: { type: "SyncHello", repo_id: "22222222-2222-2222-2222-222222222222", peer_pubkey: "pub_b", vector: { seq: 1 } }
  assertions:
    - ws_receive_contains: "22222222-2222-2222-2222-222222222222"
    - ws_receive_not_contains: "reuse_repo_a_vector"

- case_id: NET-010
  goal: 恶意数据隔离。
  preconditions:
    - Remote 分支有破坏性 Ledger Facts
  steps:
    - ws_send: { type: "SyncPush", peer: "malicious", repo_id: "11111111-1111-1111-1111-111111111111" }
  assertions:
    - fs_changes_only_under: "ledger/remotes/malicious"

- case_id: NET-011
  goal: 间接同步信任边界。
  preconditions:
    - B 未与 A 握手
  steps:
    - ws_send: { type: "GossipOffer", from: "C", about: "A", repo_id: "11111111-1111-1111-1111-111111111111" }
  assertions:
    - ws_receive_not_contains: "FetchRequest"

- case_id: NET-012
  goal: WebSocket 错误必须走结构化 ProtocolError。
  preconditions:
    - 连接已建立
    - 触发 source control 错误（如暂存不存在的 pending）
  steps:
    - run: rg -n "Error\\(String\\)" "crates/core/src/protocol/server.rs" "apps/web/src/hooks/use_core/effects/message.rs"
  assertions:
    - stdout_not_contains: "Error(String)"

- case_id: NET-013
  goal: 认证失效必须进入 Unauthorized，而不是普通重连。
  preconditions:
    - 已建立登录态与 WS 连接
  steps:
    - run: curl -s -X POST http://127.0.0.1:3000/api/auth/logout
    - browser_wait_ws_event: true
  assertions:
    - ui_assert: login_screen_visible true
    - ui_assert: overlay_text_not_eq "Reconnecting..."
```
