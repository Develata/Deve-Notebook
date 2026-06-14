# 07_network.md - P2P、WebLightPeer 与 Sync Protocol 工程蓝图

## Metadata

- `Layer`: `Runtime Protocols`
- `Status`: `Current MUST`
- `Version`: `0.0.1`
- `Last Review`: `2026-06-14`
- `Counterpart Feature`: `docs/features/05_network.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/06_network.md`
- `Primary Code Areas`: `crates/core/src/protocol/`, `crates/core/src/sync/`, `apps/cli/src/server/ws/`, `apps/cli/src/server/p2p/`, `apps/web/src/hooks/use_core/effects/handshake*.rs`

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
- relay / proxy / browser runtime 不得依赖“隐式默认 repo”这样的全局状态。

## 3. Topology Contract

### 3.1 P2P Mesh

- Desktop / Mobile / Server 是完整 peer。
- Server 是 always-on relay。
- Browser 不是 full peer，而是 WebLightPeer。

### 3.1.1 Full Peer Mesh v1 {#full-peer-mesh-v1}

Full Peer Mesh v1 是当前多服务端拓扑的最小可运行合同：

- 参与者：Desktop Native Peer、Mobile Native Peer、Server Relay Peer 均可作为 full peer；Browser 仍只作为 WebLightPeer。
- 传输：full peer 之间复用同一 `/ws` endpoint 与 versioned bincode frame，不新增独立 mesh 端口或 wire format。
- 拓扑：v1 仅支持静态配置 peer endpoint；不做自动发现、NAT 穿透、DHT、公共 relay marketplace 或 gossip peer discovery。
- Repo identity：同一逻辑 repo 的 full peer **MUST** 共享同一 `RepoId`；不同 `RepoId` 不得被自动合并为同一 mesh repo。
- 写入语义：每个 full peer 只对自己的 local branch 拥有 writer authority；入站远端 facts 只能进入对应 `ledger/remotes/<peer>/<repo>` shadow repo。
- 合并语义：remote shadow merge 到 local branch 必须通过显式 source-control merge flow；mesh 同步成功不得自动污染 local branch。
- 资源策略：low-spec / 服务器环境中 connector 必须可关闭、可限频、可 backoff；不得要求常驻大内存索引或后台全量 replay。

### 3.1.2 Static Peer Configuration {#static-peer-config}

Full Peer Mesh v1 的配置面默认关闭：

```toml
[p2p]
enabled = false
inbound_token_env = "DEVE_P2P_INBOUND_TOKEN"
connect_interval_ms = 5000

[[p2p.peers]]
label = "peer-b"
peer_id = "<expected-peer-id>"
repo_id = "<repo-uuid>"
ws_url = "ws://127.0.0.1:3102/ws"
auth_token_env = "DEVE_P2P_PEER_B_TOKEN"
enabled = true
```

约束：

- `enabled = false` 时 runtime **MUST NOT** 启动 outbound mesh connector。
- `inbound_token_env` / `auth_token_env` 只保存环境变量名；token material **MUST NOT** 写入 `config.toml`、日志、`/api/node/role`、native bootstrap payload 或 browser localStorage。
- `peer_id`、`repo_id` 与 `ws_url` 都必须显式配置；缺失任一项必须 fail-closed。
- `peer_id` 是 expected authenticated peer identity，**不是** display label；FullPeer connector 收到对端 `SyncHello` 后，返回的 authenticated `peer_id`
  必须与静态配置完全一致，否则必须 fail-closed 并记录结构化错误。
- FullPeer connector 接受对端 `SyncHello` 前，必须验证 `peer_id` 可由 `pub_key` 推导，且 `signature` 能验证当前 `SyncHello.vector`
  的 v1 handshake transcript；验证失败不得设置 authenticated peer，不得处理后续 sync frame。
- `ws_url` 的 scheme 必须是 `ws://` 或 `wss://`；Docker/local smoke 可使用 loopback 或 compose service DNS，生产配置应使用受控私网或 TLS 终端。
- connector 必须拒绝连接本机相同 `peer_id + repo_id + ws_url` 的明显 self-loop。
- FullPeer `/ws` admission **MUST** 使用 effective `p2p.inbound_token_env` 读取入站 token
  环境变量；不得只依赖硬编码 env 间接项。`inbound_token_env = null` 或 token 缺失时必须 fail-closed。
- connector 必须维护 peer-local runtime state：`configured/connecting/connected/reconnecting/unauthorized/error/self_loop/disabled`、
  attempts、handshakes、last_error_code 与已发送/已应用统计；单 peer 状态不得阻塞其他 peer。
- peer-local runtime state 的更新键必须来自静态 peer identity tuple（至少包含 `peer_id + repo_id + ws_url`），不得只用 display `label`；
  重复 label 只能影响显示，不得导致 attempts、handshake、last_error_code 或统计串扰。
- connector retry jitter 也必须由静态 peer identity tuple 派生，不得只由 display `label` 或 label 长度决定；
  重复或同长度 label 的不同 peer 仍必须具备不同的 retry 抖动输入。
- 静态配置中重复的 `peer_id + repo_id + ws_url` identity tuple 必须在配置加载时 fail-closed；否则 connector 与
  `/api/node/role` 无法把 attempt、handshake、last_error_code 和统计唯一归属到一个 peer entry。
- `last_error_code` 是最近一次失败的诊断事实；普通重连 attempt **MUST NOT** 提前清空它，只有成功 handshake/apply 周期或重新初始化静态配置时才能清空。
- `/api/node/role` 可暴露不含 token material 的只读 `p2p` 摘要：只允许 label、peer_id、repo_id、
  state、attempt/handshake 计数、push/snapshot 计数与 last_error_code。

### 3.2 Mobile Participation

- foreground = full peer
- background = light peer
- resume = 强制 `SyncHello`

移动端额外合同：

- `Write Boundary`
  - background / suspended 状态 **MUST** 禁止长时 merge、全量 replay、批量 writeback。
  - 仅允许必要心跳、session 保活与唤醒后增量恢复准备。
- `Durability Guarantee`
  - 后台窗口产生的跨端更新 **MUST** 由 server relay 托管。
  - 移动端恢复前台后再通过 vector 对齐增量拉取。
- `Power Policy`
  - 在低电量/弱网场景下，移动端 **MAY** 延迟非关键同步任务。
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
- full peer 默认连接静态配置的 peer `/ws`

### 4.1.1 Full Peer `/ws` Admission {#full-peer-ws-admission}

`/ws` admission 必须区分两类 session：

- `Browser`：沿用 cookie/JWT 与 localhost dev policy；升级后标记为 WebLightPeer browser session。
- `FullPeer`：使用 `Authorization: Bearer <token>` 或等价受控 header 与 inbound token env 比对；升级后不得标记为 browser session。

规则：

- FullPeer admission 只证明 transport 可进入 P2P handshake；真正 repo/peer authority 仍由 `SyncHello` 的 peer signature、`repo_id`、scope 与 source proof 决定。
- token 比对失败必须在 WebSocket upgrade 前返回结构化 unauthorized response，不能进入普通 sync handler。
- FullPeer connector 必须把 WebSocket `Ping` / `Pong` 作为 transport control frame 处理：`Ping` 需回应 `Pong`，
  `Pong` 需忽略；二者不得被解释为 sync protocol frame，也不得中断 `SyncHello` handshake。
- FullPeer connector 在同一个 WebSocket exchange 中只能接受一次 `SyncHello`；重复 `SyncHello` 必须 fail-closed，
  不得重置已认证 peer、repo route 或本次 handshake 的 source offer/request 集合；诊断必须暴露 `duplicate_sync_hello`
  并停止把该状态机错误当作普通断线持续重连。断线重连必须建立新的 WebSocket。
- FullPeer session 不允许走 browser writer registration shortcut；writer gate 仍只对当前 local authority branch 生效。
- Browser 与 FullPeer 复用 `ClientMessage` / `ServerMessage` schema；除非 enum/wire shape 改变，否则不得 bump `WS_PROTOCOL_VERSION`。

### 4.2 Serialization

- WebSocket 二进制帧 **MUST** 使用 `DEVEWSF3` magic header、`protocol_version` 与 bincode payload。
- `protocol_version` 当前固定为 `9`；当前兼容窗口为 `9..=9`；任何破坏兼容的 schema 变更 **MUST** bump 版本，并同步更新收发端兼容窗口。
- 服务端到服务端、服务端到客户端 **MUST** 默认使用 versioned bincode frame。
- 浏览器客户端到服务端 **SHOULD** 优先使用 versioned bincode frame；text-frame versioned JSON 只能作为调试入口保留。
- 旧式 JSON text frame **MAY** 在显式 development/debug 兼容开关下解析，**MUST NOT** 成为生产默认运行时合同。
- 旧式 JSON debug frame 缺少 `known_vector` / `server_vector` 时，只能按空向量兼容解析；新发送的 sync frame **MUST** 显式携带这些字段。
- 旧式 raw bincode / binary JSON 不属于兼容合同；运行时 **MUST** 拒绝缺失 `DEVEWSF3` magic 的二进制帧。
- 运行时 **MUST** 拒绝 unsupported protocol version，并通过结构化 `ProtocolError` 暴露失败。

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

- 进入同步路径的消息 **MUST** 带 `repo_id`。
- 跨 repo 复用 connection-local 默认值是禁止的。

### 4.5 Message Field Matrix

- `SyncHello`
  - 必填:
    - `peer_id`
    - `repo_id`
    - `scope_nonce`
    - `peer_pubkey`
    - `vector`
    - `session_proof`
  - 可选:
    - `peer_label`
    - `client_capabilities`
- `SyncRequest`
  - 必填:
    - `repo_id`
    - `known_vector`
    - `requests`
- `SyncPush`
  - 必填:
    - `repo_id`
    - `source_peer_id`
    - `header`
    - `encrypted_payload`
- `SyncSnapshotRequest`
  - 必填:
    - `source_peer_id`
    - `repo_id`
    - `known_vector`
  - 可选:
    - `reason`
- `SyncPushSnapshot`
  - 必填:
    - `repo_id`
    - `source_peer_id`
    - `server_vector`
    - `payload`
  - 可选:
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

### 6.1 Repo-Scoped Handshake {#repo-scoped-handshake}

1. 客户端建立 ws。
2. 发送 `SyncHello { peer_id, repo_id, scope_nonce, peer_pubkey, vector, session_proof }`
3. Server 验证会话、repo 权限、repo route。
4. Server 返回 `ServerMessage::SyncHello { repo_id, peer_id, pub_key, signature, vector, scope_nonce }`
5. 后续 sync traffic 必须沿用同一 `repo_id`

`SyncHello` proof v1 的 transcript 由 core sync 层唯一生成与验证：`"deve-handshake" || peer_id || canonical_json(vector)`，
其中 `canonical_json(vector)` 是按 peer id 排序后的 version vector map。client `session_proof`、server `signature` 与 FullPeer
connector 验证必须共用同一 helper，禁止在 server handler、connector 或测试 helper 中各自手写 transcript。当前 v1 不把
`repo_id` / `scope_nonce` 纳入签名 transcript；二者仍必须按独立字段在 repo route / scope gate 中 fail-closed 校验。仅改变
enum 或 wire shape 时才需要 bump `WS_PROTOCOL_VERSION`。

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

Vector authority 规则：

- `SyncHello.vector` 必须由当前 `repo_id` 的 ledger heads 重建或刷新，不得只信任进程内缓存。
- local branch 水位来自本地 repo ledger head；remote branch 水位来自对应 `ledger/remotes/<peer>/<repo>` shadow ledger head。
- 服务重启、engine lazy-load、或已有 engine 之后发生本地写入时，下次 strict sync 访问必须先刷新 vector，再计算 diff 或签名回包。
- Deve-authorized local writes, including Source Control stage/commit flows, MUST be visible to the next FullPeer
  `SyncHello` diff. A committed local Projection Workspace change on peer A must produce either an incremental
  `SyncPush` or snapshot response for peer B, and peer B must write it only to peer A's shadow repo.

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

该链必须与 `10_rendering` 和 `09_web_thin_client_ledger` 保持一致。

## 8. Reconnection Contract

- exponential backoff with jitter
- `1s -> 2s -> 4s -> 8s -> 16s -> 30s cap`
- 普通断连可以无限重连
- `401/403/AUTH_*` 不得进入普通重连循环

重连成功后：

1. 必须重新发送当前 repo 的 `SyncHello`
2. 若 repo 已切换，必须按新 `repo_id` 建立新的 handshake context
3. FullPeer connector 必须刷新当前 repo ledger heads 后再计算 diff / snapshot fallback

细则：

- backoff 序列：
  - `1s -> 2s -> 4s -> 8s -> 16s -> 30s(cap)`
- 每次重连尝试 **SHOULD** 记录结构化日志和 UI retry counter。
- `Unauthorized`、`repo route mismatch`、configured `peer_id` mismatch、invalid static `repo_id` / `ws_url`、missing / empty / header-invalid outbound token env、`malformed session proof` 不得继续普通无限重连。
- full peer connector backoff 与 browser reconnect backoff 可以共用节奏，但必须按 peer endpoint 独立计数；单个 peer 失败不得阻塞其他 peer connector。

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

### 10.2 Trust Boundary {#trust-boundary}

- relay 只是传输管道，不改变来源归属
- 间接同步时，数据写入路径必须由签名来源决定

### 10.3 Malicious / Broken Peers

- 远端恶意数据只能污染对应 remote mirror，不得自动污染 local ledger
- merge 到 local 必须是显式用户动作

### 10.4 Remote Shadow Apply Atomicity {#remote-shadow-apply-atomicity}

- `SyncPush` / `SyncPushSnapshot` 必须先完成 payload 解密，再进入 storage 写入。
- 增量 payload 的 ledger append 与 tree projection 必须在同一个 shadow repo 写事务内完成；中途校验或 projection 失败时不得留下前序 op。
- Snapshot fallback 的 shadow reset 与 replay 必须在同一个 shadow repo 写事务内完成；replay 失败时旧 shadow 内容必须保留。
- Manual 模式确认合并时，同一次确认只允许一个 `peer_id + repo_id` 目标以保持原子性；混合目标必须 fail-closed 并保留 pending payload。
- Shadow apply 失败域 **MUST NOT** 回滚 local branch 已确认 ledger write。
- Local write 的 projection / workspace writeback fault **MUST NOT** 改写 remote shadow apply 事务结果。

### 10.5 Indirect Sync and Attribution {#relay-proxy-attribution-contract}

- A 经 C 中继传给 B 时，B 的落盘归属必须由 A 的签名来源决定，而不是由 C 的传输通道决定。
- `SyncPush` 与 `SyncPushSnapshot` 必须携带 source peer / branch id；该字段决定 shadow 写入目标。
- `LedgerEntry.peer_id` 表示 op author，不等于 source branch id，不能替代 payload source peer。
- authenticated transport peer 只用于会话、repo、scope 校验，不得替代 payload source peer。
- 同一个 push payload 只能包含一个 source peer 的 ledger facts；不同 source peer 必须拆成多个 push。
- Snapshot request 若请求 shadow source，响应必须导出对应 shadow，而不能回退到本地 ledger。
  Static FullPeer connector v1 例外：在没有持久化 origin `source_proof` 或 canonical ledger proof
  之前，connector 的 `SyncHello` offer set 只能包含当前节点 identity 可签名的 local source；不得
  advertise 本机保存的 third-party shadow source。
- 入站 push / snapshot push 的 source 必须来自本端在当前 SyncHello diff 中请求过的 peer；未请求的 inbound source 属于确定性
  source-boundary 错误，FullPeer connector 诊断必须暴露 `unrequested_source`，并停止把该错误当作普通断线持续重连。
- 入站 request / snapshot request 的 source 必须来自本端在当前 SyncHello diff 中声明可发送的 peer。
- request / snapshot request 请求未 offer source 属于确定性协议/source-boundary 错误；FullPeer connector 诊断必须暴露 `unoffered_source`，并停止把该错误当作普通断线持续重连。
- push / snapshot push 的 source proof 拒绝属于确定性 source-attribution 错误；FullPeer connector 诊断必须暴露 `source_proof_rejected`，并停止把该错误当作普通断线持续重连。
- 远端 `ProtocolError` 若同时携带通用 peer unauthenticated code 与 source-boundary detail，connector 诊断必须优先保留
  `unoffered_source` / `unrequested_source` / `source_proof_rejected`，不得泛化成 `malformed_session_proof`。
- FullPeer connector 接收到 `ServerMessage::SyncPush` 时，必须在 shadow apply 之前校验
  `source_peer_id`、`repo_id`、`SyncPushHeader.repo_id`、`SyncPushHeader.peer_id` 与
  `SyncPushHeader.payload_kind` 一致，并按 direct / indirect route 规则验证 `source_proof`。
- FullPeer connector 接收到 `ServerMessage::SyncPushSnapshot` 时，必须在 shadow reset / replay
  之前校验 source、repo、server vector 与 snapshot `source_proof`；静态 FullPeer v1 不实现多跳 relay
  时，声明的 `source_peer_id` 必须等于已完成 `SyncHello` 的 authenticated peer。
- `source_peer_id`、authenticated transport peer、repo route、payload kind 与 `source_proof` 的组合校验
  必须由 core protocol 层提供共享 helper；Server WS sync handler 与 FullPeer connector 不得各自维护
  可漂移的 repo/source/proof 判定矩阵。
- 若 B 未与 A 建立信任或缺少 repo key，则 B 必须丢弃 A 的 payload。
- relay 可以转发 offer，但不得越权强制接收。

## 11. Forbidden Patterns

- 依赖 connection 外的隐式默认 repo。
- 让浏览器在断连后继续可写。
- 把 `Unauthorized` 包装成普通 `Disconnected`。
- 让 Proxy 伪装自己拥有 ledger authority。
- 使用空 repo 占位符完成 snapshot / sync 路由。
- 把静态 mesh token 写入 config、日志、browser storage、native bootstrap payload 或 URL。
- 把 mesh sync 成功解释为自动 merge 到 local branch。
- 用 browser `/ws` cookie admission 伪装 full peer server-to-server admission。

## 12. Runtime Boundary

### 12.1 Protocol Layer

- 定义 client/server message schema、frame serialization、structured protocol errors 与 version compatibility window。
- 不得依赖 UI 组件状态或隐式默认 repo。

### 12.2 Sync Engine Layer

- 负责 vector diff、snapshot fallback、materialize 与 trust boundary enforcement。
- 所有 repo-scoped sync 输入必须先完成 `repo_id` / `branch` / `scope_nonce` 验证。

### 12.3 Server WS Runtime {#server-ws-runtime}

- 负责 ws upgrade、session gate、scope guard、server outbound fanout 与 sync handler 编排。
- unauthorized、protocol error、stale scope 与 disconnected 必须走不同结构化错误路径。
- 负责 Browser / FullPeer admission 分类；FullPeer admission 通过后仍必须走 peer signature、repo scope 与 source attribution 校验。
- 负责按静态 peer 配置启动 outbound connector；connector 可以复用独立的 P2P handler，但该 handler 必须执行与普通 sync handler 等价的 repo/source attribution 校验，不得在校验前直接写 shadow repo；等价校验必须复用 core protocol 共享 helper。
- P2P handler 在收到并接受 `SyncHello` 前，不得处理 `SyncRequest`、`SyncSnapshotRequest`、`SyncPush` 或 `SyncPushSnapshot`；
  握手后的所有 sync frame 必须沿用同一 configured `repo_id`。

### 12.4 Web Runtime {#web-ws-runtime}

- 负责 browser peer identity、repo-scoped handshake、client-side durable state probe 与 stale message discard。
- Web runtime 不得在 disconnected、unauthorized 或 peer identity 缺失时保持可写。

### 12.5 Native Full Peer Runtime {#native-full-peer-runtime}

- Desktop / Mobile native full peer runtime 只在 native packaging + 显式 opt-in 后打开。
- Native shell 不拥有 ledger/source-control/search authority；它只能启动或绑定本机 service，并把 endpoint/session/readiness 交给共享 Web shell。
- Desktop full peer v1 使用受控 child-process local service；Mobile full peer v1 使用 in-process embedded loopback service。
- 两者必须通过本机 server/core writer gate 完成业务写入；shell lifecycle、foreground、network online 事件不得直接授予可写状态。

## 13. Refactor Target

长期应显式收敛成：

- `transport_runtime`
- `repo_scope_sync_runtime`
- `browser_peer_runtime`
- `relay_proxy_runtime`

实现必须围绕这四个 runtime 收束；ws route、sync handler、scope cleanup 与前端 handshake 只能作为对应 runtime 的内部实现细节。

## 本章相关命令

- 无

## 本章相关配置

- `SYNC_MODE`
- `ws endpoint / public endpoint`
- relay / proxy 相关端口配置
- `[p2p] enabled`, `connect_interval_ms`, `inbound_token_env`
- `[[p2p.peers]] label`, `peer_id`, `repo_id`, `ws_url`, `auth_token_env`, `enabled`
- Native opt-in env：`DEVE_NATIVE_AUTHORITY=1`，Desktop 还要求 `DEVE_DESKTOP_LOCAL_SERVICE=1`，Mobile 还要求 `DEVE_MOBILE_EMBEDDED_SERVICE=1`
