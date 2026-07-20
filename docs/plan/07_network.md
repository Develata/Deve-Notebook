# 07_network.md - P2P、WebLightPeer 与 Sync Protocol 工程蓝图

## Metadata

- `Layer`: `Runtime Protocols`
- `Status`: `Current MUST`
- `Version`: `0.0.1`
- `Last Review`: `2026-07-20`
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

- full-peer sync、Remote Projection transport 与 Remote Import source/manifest **MUST NOT** 传输
  host-local repo alias。跨宿主 logical identity 只使用 exact `RepoId`，并继续验证 genesis / ledger
  identity 与 authenticated source；相同 UUID 本身不构成授权。
- 浏览器控制协议可以返回当前 server host 生成的 display alias，但 alias 只供当前人类界面显示，
  不得进入 sync fact、remote shadow、provider object identity 或 Remote Import receipt。
- relay / proxy / browser runtime 不得依赖“隐式默认 repo”这样的全局状态。

## 3. Topology Contract

### 3.1 P2P Mesh

- Desktop / Mobile / Server 是完整 peer。
- Server 是 always-on relay。
- Browser 不是 full peer，而是 WebLightPeer。

### 3.1.1 Full Peer Mesh v1 {#full-peer-mesh-v1}

Full Peer Mesh v1 是当前多服务端拓扑的最小可运行合同：

- 参与者：Desktop Native Peer、Mobile Native Peer、Server Relay Peer 均可作为 full peer；Browser 仍只作为 WebLightPeer。
- 传输：full peer 之间复用同一 `/ws` endpoint 与 versioned postcard frame，不新增独立 mesh 端口或 wire format。
- 拓扑：v1 仅支持静态配置 peer endpoint；不做自动发现、NAT 穿透、DHT、公共 relay marketplace 或 gossip peer discovery。
- Repo identity：同一逻辑 repo 的 full peer **MUST** 共享同一 `RepoId`；不同 `RepoId` 不得被自动合并为同一 mesh repo。
- Repo display：full peer 不发送 repo alias；每个 host 独立维护自己的 `HostRepoAliasBinding`，缺失时显示完整 RepoId。
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
- `peer_id` 是 expected authenticated peer identity，**不是** display label；当前 identity key 生成的 canonical
  peer id 是启动日志中的 12 位小写十六进制 hash 前缀。静态配置加载必须拒绝 human label 或非 canonical peer id；
  FullPeer connector 收到对端 `SyncHello` 后，返回的 authenticated `peer_id` 必须与静态配置完全一致，否则必须 fail-closed 并记录结构化错误。
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
  - `watcher_health`
- 生产默认入口仍然是单一 origin；该接口只用于诊断、本地开发和 fallback 观测。

`watcher_health` 是 workspace ingestion readiness 的 aggregate，只允许以下 shape：

```json
{
  "status": "healthy | transitioning | degraded | unknown",
  "expected": 0,
  "running": 0,
  "unavailable": 0
}
```

- 先冻结 expected set `E`：只包含需要 watcher 的 `RepoHealth::Healthy` local repo；remote shadow、removed、quarantined、repairing 与 durable degraded repo 不计入。
- `expected = |E|`；`running` 只统计 `E` 中处于 `RepoMountState::Mounted` 的 repo，因此必须满足 `0 <= running <= expected`；`unavailable = expected - running`，包含 transition、failed 与其它未 mounted slot。
- `status` 按固定优先级计算，不允许重叠解释：无法取得可信且完整的 `WatcherRuntimeView` 时为 `unknown`；否则，`E` 中存在 `Failed`、非 transition 的未 mounted slot或计数/slot 不变量破坏时为 `degraded`；否则，`E` 中至少一个 slot 为 `Transitioning` 时为 `transitioning`；其余情况为 `healthy`，即 `E` 全部 Mounted（`E` 为空时聚合本身也为 healthy；server bootstrap 的“零个 Mounted 必须退出”由 host contract 单独执行）。
- aggregate **MUST NOT** 暴露 repo 名、`RepoId`、generation、workspace path、failure phase/kind/detail 或 tracing 内容。

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

- WebSocket 二进制帧 **MUST** 使用 `DEVEWSF4` magic header、`protocol_version` 与 project-owned postcard codec payload。
- 首个公开 wire epoch 固定为 `protocol_version = 4`，兼容窗口为 `4..=4`。v4 同时包含
  workspace ingestion unavailable、nested Remote Import family，以及 nested Repo Control
  alias/lifecycle family。
  历史未发布开发 namespace `DEVEWSF2` / `DEVEWSF3` 与 F4/v0、F4/v1、F4/v2、F4/v13
  和未发布的 F4/v3 全部 fail-closed，不提供 adapter。F4/v4 发布后只允许单调升级，不得再次重置或复用旧
  `(magic, version)` identity。任何后续破坏兼容的 schema 或 codec 变更 **MUST** bump F4
  内的版本并同步更新收发端兼容窗口。
- FullPeer Mesh v1 的发布前策略是 lockstep protocol：在没有真实 version-specific message adapter 与覆盖测试前，`MIN_SUPPORTED_WS_PROTOCOL_VERSION` **MUST** 等于当前 `WS_PROTOCOL_VERSION`。仅把常量下调、仍用当前 enum 解析旧 payload 不构成兼容实现，不得进入 runtime。
- 未来若支持滚动升级，必须为每个仍支持的旧 `protocol_version` 维护显式 decode/upgrade adapter，并在 `MIN_SUPPORTED_WS_PROTOCOL_VERSION..=WS_PROTOCOL_VERSION` 区间内逐版本测试。
- 服务端到服务端、服务端到客户端 **MUST** 默认使用 versioned postcard frame。
- 浏览器生产客户端到服务端 **MUST** 使用 versioned postcard binary frame；收到任意 text frame、损坏
  binary frame 或不支持的 wire identity 时必须退休当前 connection epoch 并重连，不得把错误帧投影成
  普通业务消息继续消费。text-frame versioned JSON 只能由 server 的显式 development/debug 入口解析。
- development/debug JSON **MUST** 显式携带 `protocol_version = 4` 并使用与 postcard frame 相同的
  v4 message schema；无版本 JSON、`LegacyJsonText` 与 `DEVE_ALLOW_LEGACY_WS_JSON` 不属于合同，
  不得解析或回退。所有 sync frame **MUST** 显式携带当前 schema 要求的 vector 字段。
- 旧式 raw codec payload / binary JSON 不属于兼容合同；运行时 **MUST** 拒绝缺失 `DEVEWSF4` magic 的二进制帧。
- 运行时 **MUST** 拒绝 unsupported protocol version，并通过结构化 `ProtocolError` 暴露失败。

当前实现已完成 C1′ 的 F4/v4 lockstep 切换。主 `/ws` 不得恢复 legacy/unversioned JSON
fallback、旧环境开关或旧 version window；显式 development/debug JSON 也必须携带 v4 envelope。
plugin-host 的 loopback
外围消息通道属于 `19_plugins#plugin-runtime-boundary`，不进入主 `/ws` F4 编解码合同。

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
  - `ProjectionRecoveryRequired`
- diff projection:
  - `GetDocDiff` / `DocDiff`
  - `GetCommitDiff` / `CommitDiffResult`
  - `GetCommitFileDiff`
  - `ComputeDiffProjection` / `DiffProjectionResult` / `DiffProjectionError`
- remote import:
  - `RemoteImport(RemoteImportRequest)`
  - `RemoteImport(RemoteImportResponse)`
- repo/runtime:
  - `RepoList`
  - `RepoControl(RepoControlRequest / RepoControlResponse)`
  - `DocList`
  - `TreeUpdate`
  - `ProtocolError`

watcher lifecycle 不新增独立 WS message family；repo create/remove 的 host-owned job 归 nested
Repo Control family。workspace ingestion unavailable 复用既有 response family：

- HTTP mutation：JSON `ServerError`、HTTP `503`；
- editor WS mutation：既有 `EditRejected`；
- 其它 WS mutation：既有 `ProtocolError`。

三条路径均携带 `13_i18n#i18n-error-code-catalog` 唯一定义的
`STORAGE_WORKSPACE_INGESTION_UNAVAILABLE`。Rust 公开枚举 variant 为
`ServerErrorCode::StorageWorkspaceIngestionUnavailable`，wire code 为同名 SCREAMING_SNAKE_CASE；
前端只按 code/i18n 分支。产品 detail 必须是固定泛化文案，repo identity、path、generation 与
failure 原因只进入 tracing。

### 4.3.1 Remote Import Wire Contract {#remote-import-wire-contract}

Remote Import 使用 nested message family，不为每个动作扩张顶层 `ClientMessage` / `ServerMessage`
variant：

```text
ClientMessage::RemoteImport(RemoteImportRequest)
ServerMessage::RemoteImport(RemoteImportResponse)

RemoteImportRequest =
  Prepare | List | Show | Page | Diff | Refresh | Apply | Discard
RemoteImportResponse =
  Prepared | Listed | Shown | Paged | Diffed | Refreshed | Applied | Discarded | Error
```

所有 request **MUST** 携带 `request_id + repo_id + branch + scope_nonce`。除 Prepare/List 外，
请求还必须按动作携带精确 `session_id + revision`；唯一例外是从未发布 candidate 的
pre-candidate `Failed` record：Show/Discard 使用 `revision=None` 表示对“candidate revision 精确缺失”的
匹配，backend 必须拒绝把 `None` 用于任何已有 candidate 的 record。Prepare 由 backend 解析 provider/profile，List
只查询当前 repo 可见 session。所有 response 必须先回显并精确匹配
`request_id + repo_id + branch + scope_nonce`。`Prepared` 响应携带 backend 新生成的
`session_id + revision`，只有通过该 request/scope gate 后才能安装为当前 session；`Listed`
响应按 request/scope 关联，且每个 summary 自带其 session/revision。Show/Page/Diff/Refresh/Apply/Discard
等 session-bound 响应还必须与当前 `session_id + revision`（含上述 exact absence）精确匹配，否则 Web 丢弃。

- Page 默认 100 entries，硬上限 200；cursor 是 opaque token，且必须绑定 candidate revision。
- Diff 只接受 opaque strong `entry_id`；显示 label 由 backend 生成，不能由 Web 从内部路径重建。
- wire/UI 只暴露 `entry_id`、backend-generated display label、typed change kind、typed blocker 与
  必要的分页/状态字段；不得暴露 locator、provider/host path、blob path、digest、credential、
  source manifest 或原始失败 detail。
- `RemoteImportChangeKind` 首版只包含 `Added / Modified / Unchanged`；远端缺失文件不投影 Delete。
- blocker 与 change kind 正交；任意 blocker 禁止整个 session Apply，前端不得自行合并或降级 blocker。
- Prepare/List/Show/Page/Diff/Refresh/Discard 不以 Mounted 为前置条件；Apply 未 Mounted 时复用
  `STORAGE_WORKSPACE_INGESTION_UNAVAILABLE`。
- Apply 响应携带 durable `RemoteImportApplyReceipt`，其 typed Projection outcome 为
  `Pending / Written / Degraded`。`Pending` 明确表示 Ledger 已提交但 recovery 尚未完成；writeback
  失败则返回 `Degraded`。两者都不得伪装成未提交或要求重试写 Ledger。

本 family 的 Rust message、server handler 与 CLI proxy 已由 B4 激活；B5 负责独立
`remote_import_client` 与完整 review UI。首发前不得以旧 Pull 或 Source Control notice 充当其实现。

### 4.3.2 Repo Control Wire Contract {#repo-control-wire-contract}

F4/v4 删除旧 `SwitchRepo` name selector、`CreateRepo`、`RenameRepo` 与 `RemoveRepo` 顶层 variants，
不保留 adapter。repo scope switch 只保留 exact `SwitchRepoExact { repo_id, switch_nonce }`；display
alias 不回传为 selector。host-local alias 与 A1 lifecycle 使用单个 nested family：

```text
ClientMessage::RepoControl(RepoControlRequest)
ServerMessage::RepoControl(RepoControlResponse)

RepoControlRequest =
  SetAlias { request_id, repo_id, alias, expected_alias_revision }
  | SubmitLifecycle { request_id, lifecycle_intent }
  | GetLifecycle { request_id }

RepoLifecycleIntent =
  Create { initial_alias, current_scope_nonce, switch_nonce }
  | Remove { repo_id, current_scope_nonce, switch_nonce }

RepoControlResponse =
  AliasSet { request_id, binding }
  | LifecycleAccepted { request_id, job_id, target_repo_id }
  | LifecycleStatus { request_id, job_id, state, outcome }
  | Error { request_id, error }
```

- `request_id` / `job_id` 是 UUID。alias request 是 authenticated host control operation，精确按
  RepoId/CAS revision 线性化但不绑定 branch/scope；stale editor scope 不能阻止合法 alias 修改。
- lifecycle submit 的 session observer 绑定当前 connection epoch 与
  `(current_scope_nonce, switch_nonce)`；job ownership 不绑定 connection。observer 消失只取消该
  connection 的可选 auto-switch，不取消 job、settlement、repo-list publication 或 completion。
- Create 的 target RepoId 由 backend admission 生成并在 `LifecycleAccepted` 返回；同一 request_id
  retry 必须返回同一 job/target。Remove 始终携带 exact RepoId，不能按 alias 查找。
- `GetLifecycle` 允许重连后按 request_id 取得既有 Accepted/Running/terminal/repair outcome；前端不得
  通过自然语言 detail 或 repo-list 差分猜测 job 是否提交。
- `RepoListEntry` 只暴露 `repo_id + display_alias + alias_revision + readiness`；不得暴露
  execution filename、workspace segment/path、locator generation 或历史 creation label。
- `RepoSwitched` 只回显 exact `repo_id + display_alias + branch + switch_nonce/scope_nonce`。alias
  变化不产生 `RepoSwitched`，只产生 `AliasSet` ack 与 backend-produced `RepoList` projection。
- alias/lifecycle failures 使用 `13_i18n#i18n-error-code-catalog` 的 `REPO_ALIAS_*` /
  `REPO_LIFECYCLE_*`；Web 只按 typed code/state/outcome 渲染，不解析 detail。

### 4.3.3 Projection Recovery Wire Contract {#projection-recovery-contract}

后端统一使用以下 typed wire 表示“authority 已变化或消息完整性已无法证明，客户端必须按指定
范围刷新 projection”：

```text
ProjectionRecoveryRequired {
  repo_id,
  branch,
  scope_nonce,
  cause: ProjectionRecoveryCause,
  plan: ProjectionRecoveryPlan,
}

DocumentRecoveryScope = None | Exact(Vec<DocId>) | CurrentDocument
ProjectionRecoveryPlan {
  documents,
  refresh_doc_list,
  refresh_source_control,
  refresh_external_changes,
}
```

`ProjectionRecoveryCause` 至少区分 ExternalApply、RemoteImportApply、DocumentMutation、
SourceControlCommit、Merge、PluginMutation 与 `BroadcastGap { skipped }`。plan 是 server authority
的投影刷新决定；Web 只能
执行 typed refresh，不得根据 cause、路径或正文推断业务恢复范围。

`ClientMessage::ApplyExternalChanges` 必须携带 `request_id`；成功返回
`ExternalApplyAck { request_id, receipt, repo_id, branch, scope_nonce }`。该 Ack 与 recovery signal
用途不同：Ack 关联请求，recovery 使同 repo 各 session 收敛。二者都不得携带逐 fact content
广播替代物。

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
    - `range_start`
    - `range_end`
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
    - `waterline`
    - `payload`
  - 可选:
    - `snapshot_kind`
- 所有 sync message 的 `repo_id` 都是 routing 主键；缺失时必须结构化拒绝。

Diff projection request/response 同样必须绑定当前 `repo_id`、`branch` 与
`scope_nonce`。`ComputeDiffProjection` 还必须携带 request id 与 session 单调 revision；
服务器每个 WS session 只保留一个活跃计算，新 revision 取消旧计算。结果发送前必须
再次核对 session generation、revision 与 scope；stale 结果不得进入 Web state。

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
- server outbound fanout 中的 `scope_nonce` 是**接收连接作用域版本**，不是事件生产者的
  连接版本。repo/branch 已匹配的 runtime broadcast 在进入某个接收连接的 unicast queue
  前，**MUST** 覆盖为该接收连接当前 `scope_nonce`；不得因生产者重连后 nonce 更大而让
  仍停留在同一 repo/branch 的其他客户端丢弃已确认 `NewOp`。
- 覆盖接收者 nonce 不得放宽 writer gate：生产者请求在产生 ledger fact 前仍必须按其自身
  当前 `scope_nonce` fail-closed 校验；fanout 只投影已经成立的 server/runtime event。

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

- `VersionVector` 是 repo-scoped `PeerId -> PeerFactSeq`；同一物理 peer 的 Content Facts 与 Structure Facts 共享一个连续水位。
- `SyncHello.vector` 必须由当前 `repo_id` 的 ledger heads 重建或刷新，不得只信任进程内缓存。
- local branch 水位来自本地 repo 的 `PEER_FACT_SEQ[local physical PeerId]`；remote branch 水位来自对应 `ledger/remotes/<peer>/<repo>` shadow 的 source peer waterline。两者不得使用 `GlobalSeq` 代替。
- 服务重启、engine lazy-load、或已有 engine 之后发生本地写入时，下次 strict sync 访问必须先刷新 vector，再计算 diff 或签名回包。
- Deve-authorized local writes, including Source Control stage/commit flows, MUST be visible to the next FullPeer
  `SyncHello` diff. A committed local Projection Workspace change on peer A must produce either an incremental
  `SyncPush` or snapshot response for peer B, and peer B must write it only to peer A's shadow repo.

Apply 端单调性与连续性规则（与第 5 节「不得破坏 vector monotonicity」一致，定义入站 facts 落库的合法性）：

- **Snapshot 单调性**：远端 snapshot 是单一 source peer 从 `1..=waterline` 的完整连续事实日志。持久化 shadow waterline 是最终 gate，不能只信任某个进程内 `VersionVector`。任何 snapshot 都必须先逐条比对与已确认历史重叠的前缀；完全相同且 `waterline <= stored` 时才幂等跳过，冲突重复必须整批拒绝。只有 `waterline > stored`、confirmed prefix 相同且完整验证通过的 snapshot 才能在同一 shadow write transaction 中替换影子库并推进持久化水位。
- **增量连续性**：增量 ops apply **MUST** 从 `持久化 shadow waterline + 1` 起严格连续地推进 peer 水位。`seq <= stored waterline`
  的已应用区间只有在完整事实逐字段相同时才幂等跳过，不得二次 append 到 shadow；遇到序号空洞、冲突重复或重复 seq **MUST** fail-closed 并保持
  状态不变，留待重连/重新请求，**MUST NOT** 把 vector 推过未接收的 op 造成静默丢失。
- **发送端完整性**：wire 增量范围使用闭区间 `[range_start, range_end]`，必须通过 `(PeerId, PeerFactSeq) -> GlobalSeq` 索引完整解析；缺少任一事实必须返回结构化 sequence-gap 错误，不得发送部分成功。`VersionVector::diff` 内部可使用 Rust 半开 `Range`，但组装 `SyncRequest` 时必须显式转换为闭区间，禁止把半开 end 直接发上 wire。
- **批次原子性**：envelope `peer_seq`、解密 entry `peer_seq`、entry `origin_peer_id` 与认证 source 必须逐条一致；gap、乱序、冲突重复、来源不符或解密失败时整批不写 shadow、不推进 vector、不刷新投影。
- **Wire vector canonicality**：反序列化后的 `VersionVector` 只能包含正 `PeerFactSeq`，并必须按 `PeerId` 严格升序且无重复；zero、乱序或重复键在 diff 算术之前 fail-closed，不允许由 `normalize()` 修补不可信 wire 输入。
- **Transfer resource gate**：在按请求范围分配 `Vec` 或收集完整 snapshot 前，发送端必须先验证 `end <= source waterline`、checked range width、最多 16384 个事实及最多 16 MiB 编码 fact bytes；超过限制返回 `sync_resource_limit`，不得尝试巨额分配，也不得伪装为成功。加密 snapshot 构造过程中仍须累计 payload budget；分块/压缩另列后续能力。
- **Manual receive resource gate**：Manual 模式的待确认缓冲必须跨全部已排队 frame 累计 payload 数、fact 数及每个 `EncryptedOp` 的 postcard 编码字节；三者分别不得超过 16384 payload、16384 facts 与 16 MiB。任何 checked overflow 或超限必须在解密和入队前返回 `sync_resource_limit`，且队列与全部计数保持不变；空 frame 仍计入 payload 上限。
- **Manual merge memory/rollback**：确认合并必须先 `take` 整个缓冲，失败时原样恢复 payload 与三类计数；不得通过 clone 整队列实现试应用。解密后应先证明 snapshot prefix，再移动最高 waterline snapshot 与增量 entries 形成 canonical batch，及时释放其余解密 payload；解密、连续性、origin、ledger apply 任一步失败都不得丢失待确认缓冲、写入部分 shadow 或推进 vector。SyncHello/transfer 需要脱离 registry lock 生成响应时，只能 clone identity/repo/key/vector 等 transport state，Manual pending queue 必须留在 registry engine，禁止随 outbound engine 复制。
- 完整 source 日志中的 `MergeAnchor` 与 content/structure facts 使用同一 `PeerFactSeq`；接收端必须保存并推进连续水位，但不得把 anchor 直接解释为 Markdown/tree projection mutation。

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

### 8.1 Ordered Delivery and Gap Recovery

- 同一个 server broadcast forwarder 对关键消息必须沿同一 sender 有序 `send().await`；队列满时
  禁止创建 detached send task。sender 关闭时应退休该 forwarder。
- 直接关键 unicast 队列满时不得后台补发或重排；必须退休当前 session，依靠新 connection epoch
  的 Snapshot/History 收敛。非关键消息可以按受控策略丢弃，但必须有指标。
- broadcast receiver `Lagged(skipped)` 必须发送 scoped `ProjectionRecoveryRequired`，不得伪装成
  普通 `RequestFailed`。
- Web incoming ring 可以保持有界，但 sequence cursor 检测到本地 gap 时必须返回 typed gap，且
  两个消费者都不得继续处理缺口后的消息。runtime 立即撤销 writer-ready，并按 connection epoch
  合并为一次 `ReconnectForResync`。

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
- 当前 `SyncPushSnapshot` 是 Full-Fact Replay Payload，必须恰好包含认证 source peer 的 `1..=waterline` 完整连续事实；它不是 storage/authority 状态压缩 Snapshot。验证全部成功后才能原子替换 shadow。
- stale snapshot 仅在其重叠前缀与已确认事实完全相同时幂等跳过；冲突重复、缺失序号、混入其他 origin、越过 waterline 或 envelope/entry 不一致时必须保留旧 shadow 与 vector。
- 当前单响应 snapshot 超过 frame/resource 上限时必须明确失败；分块或压缩 snapshot 不属于 v1 合同。

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
- `LedgerEntry.origin_peer_id` 表示事实的物理 origin；Static FullPeer v1 的单-source payload 中它必须等于 `source_peer_id`。`FactActor` 只表示本地执行路径，不能替代 payload source peer。
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
- source proof 生成端必须先验证 signing key 推导出的 peer id 等于声明的 `source_peer_id`；错 key 不得生成“等待接收端拒绝”的自失效 proof。
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
- native in-process runtime **MUST** separate the process-scoped authority runtime from listener/session transport generations. RepoManager、SyncManager、plugin host APIs、watchers、metrics、prewarm 与 P2P connector 只能由一个 owned `EmbeddedServerRuntime` 初始化一次；随机 loopback listener 或 native session 重建不得重复安装或替换这些 authority objects。
- `EmbeddedServerRuntime` **MUST** own cancellation and join handles for every background task it starts. Axum listener shutdown alone is not a complete runtime shutdown；normal native app exit 必须先停止 active transport generation，再在有界时间内 cancel/join metrics、prewarm、P2P connector，并释放 watcher ownership。
- transport generation 可以针对新端口重建 node-role、native-session bridge、allowed-origin router 与 port hint，但必须复用同一 `AppState`。process-scoped runtime fatal failure 必须 fail-closed 并要求 app restart；不得在同一进程中创建第二套 authority runtime。
- 每个 transport generation 必须拥有其升级后的 WebSocket session cancellation 与 join 边界。listener shutdown 必须先拒绝新 upgrade、通知该 generation 的全部已升级 session 退出，并可抢占当前 in-flight handler future，然后撤销 browser writer grant 并等待 session idle；旧 generation 的 sender/broadcast task、认证状态或 `AppState` 引用不得跨到新 endpoint。transport error 只有在 runtime 明确携带 `sessions_retired=true` 证明时才允许 replacement；任何 shutdown/join/idle 证明失败都必须熔断到 app restart。Mobile 为满足有界 app exit 可以关闭可选 prewarm，但不得 detach 或跳过其他 runtime task 的 cancellation/join；一般 runtime 的 prewarm 也必须协作取消，取消后不得保存部分 snapshot。
- `AppState` 必须持有 process-scoped `RepoMutationPublicationGate`，并把 local authority mutation 与
  对应 publication enqueue 作为同一个 repo permit 内的有序阶段；不得把 socket delivery、网络或
  Git mirror 放入临界区。
- `AppState` 只持有 process-scoped `WatcherRuntimeView`，供 mounted mutation admission 与
  `/api/node/role` aggregate 只读查询；不得把 `WatcherSupervisor` 或 handle 的 start/stop/restart
  authority 暴露给 HTTP/WS handler。
- server composition root 唯一拥有 `WatcherSupervisor` 与 `RepoLifecycleCoordinator`。普通 mutation
  handler 进入 `execute_mounted_repo`，repo create/remove handler 只提交 host-owned lifecycle job typed intent；host-local alias handler 只调用 alias runtime，不获得 watcher/lifecycle authority；
  二者均不得依据 tracing detail、路径存在性或 UI 状态推断 watcher readiness。

### 12.4 Web Runtime {#web-ws-runtime}

- 负责 browser peer identity、repo-scoped handshake、client-side durable state probe 与 stale message discard。
- Web runtime 不得在 disconnected、unauthorized 或 peer identity 缺失时保持可写。
- Web runtime 必须把 incoming queue gap 当成 connection integrity failure：停止消费该 gap 后的
  所有消息、撤销 writer-ready、退休当前 connection epoch，再由 fresh handshake/snapshot 恢复。
- typed projection refresh 必须绑定 connection epoch、repo/branch/scope nonce 与单调 flight id；任一 refresh
  request 失败或在有界超时内未完成时，Web 必须退休该 flight 并通过 fresh connection resync，不能永久
  卡在只读 loading，也不能接受迟到 response 恢复 readiness。

### 12.5 Native Full Peer Runtime {#native-full-peer-runtime}

- Desktop / Android / Mobile native full peer runtime 在 native-packaging `LocalBackend` 模式默认打开。
- `RemoteBrowser` 模式等价于浏览器连接远端 Docker/Web HTTPS origin，不拥有本地 FullPeer runtime。
- Native shell 不拥有 ledger/source-control/search authority；它只能启动或绑定本机 service，并把 endpoint/session/readiness 交给共享 Web shell。
- Desktop full peer v1 使用受控 child-process local service；Android/Mobile full peer v1 使用 in-process embedded loopback service。
- 两者必须通过本机 server/core writer gate 完成业务写入；shell lifecycle、foreground、network online 事件不得直接授予可写状态。
- Android/Mobile 的 session generation 只拥有 listener、port-specific router、native session 与 Web bootstrap；process-scoped embedded runtime 继续拥有唯一 RepoManager/SyncManager/AppState，generation restart 不得重开数据库或重装全局 host API。

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
- Native shell mode：默认 `LocalBackend`；显式 `RemoteBrowser` 使用 Desktop `--remote-url https://...` 或诊断/脚本环境变量 `DEVE_NATIVE_REMOTE_URL=https://...`
