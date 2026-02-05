## 网络连通与同步

```markdown
- case_id: NET-001
  goal: Web 端断连锁屏。
  preconditions:
    - 已连接 WS
  steps:
    - net_block_ws: true
  assertions:
    - ui_assert: overlay_text "Reconnecting..."
    - ui_assert: editing_disabled true

- case_id: NET-002
  goal: Main/Proxy 角色切换与端口策略。
  preconditions:
    - 3001 端口已占用
  steps:
    - run: deve serve
    - run: curl http://127.0.0.1:3002/api/node/role
  assertions:
    - stdout_contains: "role"
    - stdout_contains: "proxy"

- case_id: NET-003
  goal: 协议格式区分。
  preconditions:
    - Server-to-Server 与 Client-Server 连接已建立
  steps:
    - net_capture: true
  assertions:
    - packet_format_eq: ["server", "bincode"]
    - packet_format_eq: ["client", "json"]

- case_id: NET-004
  goal: OpenDoc Snapshot-First。
  preconditions:
    - 文档有快照与增量 Ops
  steps:
    - ws_send: { type: "OpenDoc", id: "doc_id" }
  assertions:
    - ws_receive_order: ["Snapshot", "NewOp"]

- case_id: NET-005
  goal: TOFU 握手。
  preconditions:
    - 两个全新 Peer
  steps:
    - ws_connect: "peer_a"
    - ws_connect: "peer_b"
  assertions:
    - log_contains: "Key Pair"
    - log_contains: "Handshake"

- case_id: NET-006
  goal: E2EE + Envelope Pattern。
  preconditions:
    - 触发同步
  steps:
    - net_capture: true
  assertions:
    - packet_header_contains: ["VectorClock", "PeerID", "RepoID"]
    - packet_body_encrypted: true

- case_id: NET-007
  goal: Vector Gossip 缺失 Ops。
  preconditions:
    - A 的 VC 大于 B
  steps:
    - ws_send: { type: "SyncRequest", from: "B", to: "A" }
  assertions:
    - ws_payload_contains_only_missing_ops true

- case_id: NET-008
  goal: Snapshot Sync 覆盖模式。
  preconditions:
    - OpSeq 差异超过阈值
  steps:
    - ws_send: { type: "SyncRequest" }
  assertions:
    - ws_receive_contains: "Snapshot"
    - ws_receive_not_contains: "OpsRange"

- case_id: NET-009
  goal: 恶意数据隔离。
  preconditions:
    - Remote 分支有破坏性 Ops
  steps:
    - ws_send: { type: "SyncPush", peer: "malicious" }
  assertions:
    - fs_changes_only_under: "ledger/remotes/malicious"

- case_id: NET-010
  goal: 间接同步信任边界。
  preconditions:
    - B 未与 A 握手
  steps:
    - ws_send: { type: "GossipOffer", from: "C", about: "A" }
  assertions:
    - ws_receive_not_contains: "FetchRequest"
```
