# 05_network.md - P2P、WebLightPeer 与 Sync Protocol 工程蓝图

## Metadata

- `Layer`: `Runtime Protocols`
- `Status`: `Current MUST`
- `Counterpart Feature`: `docs/features/05_network.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/06_network.md`
- `Primary Code Areas`: `crates/core/src/protocol/`, `crates/core/src/sync/`, `apps/cli/src/server/ws/`, `apps/web/src/hooks/use_core/effects/handshake*.rs`

## 1. Scope

本章定义：

- peer topology
- WebLightPeer / Main / Proxy 角色边界
- ws/http/sync protocol
- handshake、reconnect、snapshot fallback、trust boundary

本章不定义用户可见状态文案与操作示例；这些属于 `docs/features/05_network.md`。

## 2. Authoritative Entities

### 2.1 Peer Roles

- `Desktop Native Peer`
- `Mobile Native Peer`
- `Server Relay Peer`
- `WebLightPeer`
- `Main`
- `Proxy`

### 2.2 Network Identity

- `PeerId`
- `RepoId`
- `VersionVector`
- `ScopeNonce`
- `SessionProof`
- `RepoKey`

### 2.3 Route Authority

- repo-scoped network routing 必须以 `repo_id` 为主绑定键。
- `relative /ws` 是浏览器默认连接契约。
- `Proxy` 只转发，不拥有 ledger authority。

补充：

- `repo_name` 只能作为兼容提示或 selector 输入，进入同步链后必须解析成 `repo_id`。
- relay / proxy / browser runtime 不得依赖“当前默认 repo”这样的隐式全局状态。

## 3. Topology Contract

### 3.1 P2P Mesh

- Desktop / Mobile / Server 是完整 peer。
- Server 是 always-on relay。
- Browser 不是 full peer，而是 WebLightPeer。

### 3.2 Mobile Participation

- foreground = full peer
- background = light peer
- resume = 强制 `SyncHello`

移动端额外合同：

- `Write Boundary`
  - background / suspended 状态 MUST 禁止长时 merge、全量 replay、批量 writeback。
  - 仅允许必要心跳、session 保活与唤醒后增量恢复准备。
- `Durability Guarantee`
  - 后台窗口产生的跨端更新 MUST 由 server relay 托管。
  - 移动端恢复前台后再通过 vector 对齐增量拉取。
- `Power Policy`
  - 在低电量/弱网场景下，移动端 MAY 延迟非关键同步任务。
  - 但不得破坏 vector monotonicity、repo route correctness、scope gate correctness。

### 3.3 WebLightPeer

- repo-scoped identity/vector/cache
- 无完整本地 ledger
- 必须在线连接 server 才能工作
- 断连进入只读

### 3.4 Main / Proxy

- Main：持锁、真实读写
- Proxy：同源 HTTP/WS 转发
- 前端不得感知真实 ledger 进程端口变化

探测与入口契约：

- `GET /api/node/role` 返回至少：
  - `role`
  - `ws_port`
  - `main_port`
- 生产默认入口仍然是单一 origin；该接口只用于诊断、本地开发和 fallback 观测。

## 4. Transport and Message Contract

### 4.1 Transport

- ws 是主实时协议
- http 用于 auth / bootstrapping / role probing / 部分代理接口
- browser 默认连接 `relative /ws`

### 4.2 Serialization

- WebSocket 二进制帧必须使用 `DEVEWSF2` magic header + `protocol_version` + bincode payload。
- 当前 `protocol_version = 2`；任何 breaking schema change 必须 bump 该版本，并同步更新收发端兼容窗口。
- server-to-server 与 server-to-client 默认 versioned bincode frame。
- browser client-to-server 优先 versioned bincode frame，保留 text-frame versioned JSON / legacy JSON 调试兼容入口。
- legacy raw bincode / binary JSON 不属于当前兼容合同，runtime 必须拒绝缺失 `DEVEWSF2` magic 的二进制帧。
- runtime 必须能拒绝 unsupported protocol version，并通过结构化 `ProtocolError` 暴露失败。

### 4.3 Core Message Families

- sync:
  - `SyncHello`
  - `SyncRequest`
  - `SyncPush`
  - `SyncSnapshotRequest`
  - `SyncPushSnapshot`
- document:
  - `OpenDoc`
  - `Snapshot`
  - `History`
  - `NewOp`
  - `Ack`
  - `EditRejected`
- repo/runtime:
  - `RepoList`
  - `DocList`
  - `TreeUpdate`
  - `ProtocolError`

### 4.4 Routing Rule

- 进入同步路径的消息 MUST 带 `repo_id`。
- 跨 repo 复用 connection-local 默认值是禁止的。

### 4.5 Message Field Matrix

- `SyncHello`
  - required:
    - `repo_id`
    - `peer_pubkey`
    - `vector`
    - `session_proof`
  - optional:
    - `peer_label`
    - `client_capabilities`
- `SyncRequest`
  - required:
    - `repo_id`
    - `known_vector`
- `SyncPush`
  - required:
    - `repo_id`
    - `source_peer_id`
    - `header`
    - `encrypted_payload`
- `SyncSnapshotRequest`
  - required:
    - `repo_id`
    - `known_vector`
    - `reason`
- `SyncPushSnapshot`
  - required:
    - `repo_id`
    - `source_peer_id`
    - `server_vector`
    - `payload`
    - `snapshot_kind`
- 所有 sync message 的 `repo_id` 都是 routing 主键；缺失时必须结构化拒绝。

## 5. State Machines

### 5.1 Browser Connection Lifecycle

```text
Disconnected
  -> Connecting
  -> WsOpen
  -> HandshakePending(repo_id)
  -> RepoBound(repo_id, scope_nonce)
  -> Synced | Resyncing
  -> Disconnected | Unauthorized
```

### 5.2 Main / Proxy Lifecycle

```text
Start
  -> Main
  -> Proxy
```

约束：

- Proxy 只负责转发。
- `relative /ws` 和同源 API 在 Proxy 模式下仍然必须可用。

### 5.3 Snapshot Fallback Lifecycle

```text
VectorCompared
  -> IncrementalSync
  -> SnapshotFallback
  -> SnapshotApplied
  -> VectorUpdated
```

## 6. Handshake Contract

### 6.1 Repo-Scoped Handshake

1. 客户端建立 ws。
2. 发送 `SyncHello { repo_id, peer_pubkey, vector, session_proof }`
3. Server 验证会话、repo 权限、repo route。
4. Server 返回 `ServerMessage::SyncHello { repo_id, peer_id, pub_key, signature, vector, scope_nonce }`
5. 后续 sync traffic 必须沿用同一 `repo_id`

### 6.2 Handshake Invariants

- `repo_id_a` 与 `repo_id_b` 必须映射到不同 peer state。
- 旧 repo 的延迟握手不得激活新 repo 的写入闸门。
- `scope_nonce` 必须参与 repo-scoped message gating。

### 6.3 Keystore Contract

- WebLightPeer 的 peer identity 与 keystore 是 repo-scoped。
- 没有 repo key 的 peer 即使拿到消息体也无法解密内容。

### 6.4 Browser Peer Registration

1. 浏览器先验证 user session 已成立。
2. 针对当前 `repo_id` 读取 IndexedDB 中的 peer metadata。
3. 若不存在，则调用 `WebCrypto` 生成 repo-scoped keypair，私钥必须 `extractable: false`。
4. 发送 `SyncHello` 注册 browser peer。
5. 服务端校验 session + repo access + handshake material 后，返回已绑定 `repo_id` 的 `ServerMessage::SyncHello`。
6. 浏览器只有在收到与当前 `repo_id` 匹配的回执后，才能进入可写/可同步状态。

## 7. Sync Contract

### 7.1 Vector Gossip

同步必须按以下流程进行：

1. 比较当前 repo 的 vector。
2. 决定增量还是 snapshot fallback。
3. 增量路径只发送缺失 ledger facts。
4. 镜像端仅把收到的远端事实写入 remote branch。
5. 成功后更新该 repo 的 vector。

Vector authority:

- `SyncHello.vector` 必须由当前 `repo_id` 的 ledger heads 重建或刷新，不得只信任进程内缓存。
- local branch 水位来自本地 repo ledger head；remote branch 水位来自对应 `ledger/remotes/<peer>/<repo>` shadow ledger head。
- 服务重启、engine lazy-load、或已有 engine 之后发生本地写入时，下次 strict sync 访问必须先刷新 vector，再计算 diff 或签名回包。

### 7.2 Envelope Pattern

- plaintext header：
  - `VectorClock`
  - `PeerId`
  - `RepoId`
- encrypted body：
  - diff/snapshot payload

Relay 节点不得依赖解密 payload 才能完成路由。

额外约束：

- plaintext header 至少包含：
  - `repo_id`
  - `peer_id`
  - `vector`
  - `payload_kind`
- encrypted body 才允许携带 doc/content/structure 事实。
- relay 不得修改 header 中的来源归属字段。

### 7.3 OpenDoc Performance

- `Snapshot-First`
- progressive prefetch
- search gate

该链必须与 `03_rendering` 和 `16_web_thin_client_ledger` 保持一致。

## 8. Reconnection Contract

- exponential backoff with jitter
- `1s -> 2s -> 4s -> 8s -> 16s -> 30s cap`
- 普通断连可以无限重连
- `401/403/AUTH_*` 不得进入普通重连循环

重连成功后：

1. 必须重新发送当前 repo 的 `SyncHello`
2. 若 repo 已切换，必须按新 `repo_id` 建立新的 handshake context

细则：

- backoff 序列：
  - `1s -> 2s -> 4s -> 8s -> 16s -> 30s(cap)`
- 每次重连尝试 SHOULD 记录结构化日志和 UI retry counter。
- `Unauthorized`、`repo route mismatch`、`malformed session proof` 不得继续普通无限重连。

## 9. Failure Modes

- ws disconnected
- proxy/main role change
- stale scope
- repo route mismatch
- snapshot gap too large
- invalid sync payload
- missing repo key
- unauthorized session

错误码 / 处理矩阵：

- `AUTH_*`
  - stop reconnect
  - enter unauthorized surface
- `SC_STALE_SCOPE`
  - discard delayed scope write/read message
  - request fresh handshake/bootstrap
- `SYNC_REPO_ROUTE_MISMATCH`
  - clear stale repo binding
  - re-bootstrap repo list / scope
- `SYNC_SNAPSHOT_REQUIRED`
  - switch to snapshot fallback
- `SYNC_INVALID_PAYLOAD`
  - reject peer payload
  - keep local authority untouched

## 10. Recovery / Safety

### 10.1 Snapshot Fallback

- vector 差距过大时允许 snapshot fallback
- fallback 必须绑定到明确 repo route
- 禁止空 repo 占位符或跨 repo 复用 snapshot

### 10.2 Trust Boundary

- relay 只是传输管道，不改变来源归属
- 间接同步时，数据写入路径必须由签名来源决定

### 10.3 Malicious / Broken Peers

- 远端恶意数据只能污染对应 remote mirror，不得自动污染 local ledger
- merge 到 local 必须是显式用户动作

### 10.4 Remote Shadow Apply Atomicity

- `SyncPush` / `SyncPushSnapshot` 必须先完成 payload 解密，再进入 storage 写入。
- 增量 payload 的 ledger append 与 tree projection 必须在同一个 shadow repo 写事务内完成；中途校验或 projection 失败时不得留下前序 op。
- Snapshot fallback 的 shadow reset 与 replay 必须在同一个 shadow repo 写事务内完成；replay 失败时旧 shadow 内容必须保留。
- Manual 模式确认合并时，同一次确认只允许一个 `peer_id + repo_id` 目标以保持原子性；混合目标必须 fail-closed 并保留 pending payload。

### 10.5 Indirect Sync and Attribution

- A 经 C 中继传给 B 时，B 的落盘归属必须由 A 的签名来源决定，而不是由 C 的传输通道决定。
- `SyncPush` 与 `SyncPushSnapshot` 必须携带 source peer / branch id；该字段决定 shadow 写入目标。
- `LedgerEntry.peer_id` 表示 op author，不等于 source branch id，不能替代 payload source peer。
- authenticated transport peer 只用于会话、repo、scope 校验，不得替代 payload source peer。
- 同一个 push payload 只能包含一个 source peer 的 ledger facts；不同 source peer 必须拆成多个 push。
- Snapshot request 若请求 shadow source，响应必须导出对应 shadow，而不能回退到本地 ledger。
- 入站 push / snapshot push 的 source 必须来自本端在当前 SyncHello diff 中请求过的 peer；入站 request / snapshot request 的 source 必须来自本端在当前 SyncHello diff 中声明可发送的 peer。
- 若 B 未与 A 建立信任或缺少 repo key，则 B 必须丢弃 A 的 payload。
- relay 可以转发 offer，但不得越权强制接收。

## 11. Forbidden Patterns

- 依赖 connection 外的隐式默认 repo。
- 让浏览器在断连后继续可写。
- 把 `Unauthorized` 包装成普通 `Disconnected`。
- 让 Proxy 伪装自己拥有 ledger authority。
- 使用空 repo 占位符完成 snapshot / sync 路由。

## 12. Module Boundary

### 12.1 Protocol Layer

- `crates/core/src/protocol/client.rs`
- `crates/core/src/protocol/server.rs`

### 12.2 Sync Engine Layer

- `crates/core/src/sync/engine/handshake.rs`
- `crates/core/src/sync/materialize.rs`

### 12.3 Server WS Runtime {#server-ws-runtime}

- `apps/cli/src/server/ws/`
- `apps/cli/src/server/handlers/sync/`
- `apps/cli/src/server/repo_scope*.rs`

### 12.4 Web Runtime

- `apps/web/src/hooks/use_core/effects/handshake*.rs`
- `apps/web/src/hooks/use_core/effects/message_sync*.rs`
- `apps/web/src/hooks/use_core/storage_runtime*.rs`

## 13. Code Mapping

- message protocol:
  - `crates/core/src/protocol/client.rs`
  - `crates/core/src/protocol/server.rs`
- sync engine:
  - `crates/core/src/sync/engine/handshake.rs`
  - `crates/core/src/sync/materialize.rs`
- server ws:
  - `apps/cli/src/server/ws/mod.rs`
  - `apps/cli/src/server/ws/route/core.rs`
  - `apps/cli/src/server/ws/route/docs.rs`
  - `apps/cli/src/server/ws/route/source_control.rs`
  - `apps/cli/src/server/ws/route/scope_guard.rs`
  - `apps/cli/src/server/handlers/sync/hello.rs`
  - `apps/cli/src/server/handlers/sync/hello_scope.rs`
- browser runtime:
  - `apps/web/src/hooks/use_core/effects/handshake.rs`
  - `apps/web/src/hooks/use_core/effects/handshake_bootstrap.rs`
  - `apps/web/src/hooks/use_core/effects/handshake_send.rs`
  - `apps/web/src/hooks/use_core/effects/message_sync.rs`
  - `apps/web/src/hooks/use_core/storage_runtime.rs`
  - `apps/web/src/hooks/use_core/storage_runtime_bootstrap.rs`

## 14. Refactor Target

长期应显式收敛成：

- `transport_runtime`
- `repo_scope_sync_runtime`
- `browser_peer_runtime`
- `relay_proxy_runtime`

当前网络实现仍散在 ws route、sync handlers、repo scope cleanup 和前端 handshake effects 中。未来重构必须围绕这四个 runtime 收束。

## 本章相关命令

- 无

## 本章相关配置

- `SYNC_MODE`
- `ws endpoint / public endpoint`
- relay / proxy 相关端口配置
