# 05_network.md - 网络架构篇 (Network Architecture)

## 拓扑定义：P2P 三角与 Web 面板 (The P2P Triangle & Web Dashboard)

### P2P Mesh (对等网络)
* **核心节点**：仅包含 **Desktop Native (PC/Mac)**、**Mobile Native (iOS/Android)** 和 **Server (Linux)**。
* **机制**：这三方拥有独立的 `PeerID` 和 **Local Branch** (数据集合)，通过 Gossip 协议交换 Repo Instances。
* **Server 角色**：Server 在 P2P 网络中充当 **Always-on Relay Peer** (全天候中继/备份节点)。

### Mobile P2P Strategy (移动端 P2P 参与策略)

* **Foreground Participation**: iOS/Android 客户端在前台运行时 **MUST** 作为完整 P2P Peer 参与 Gossip、握手与增量同步。
* **Background Degrade**: iOS/Android 进入后台后 **MUST** 立即降级为 **Light Peer**，停止主动 Gossip 轮询与高频广播。
* **Pull-on-Resume**: 移动端回到前台时 **MUST** 触发一次 `SyncHello`，并按 Vector Clock 执行缺失补齐。
* **Write Boundary**: Light Peer 状态下 **MUST** 禁止发起长时合并与大批量重放，仅保留必要心跳/唤醒能力。
* **Durability Guarantee**: 后台窗口期产生的跨端更新 **MUST** 由 Server（Always-on Relay）托管，移动端前台恢复后再增量拉取。
* **Power Policy**: 在系统电量/网络受限场景下，移动端 **MAY** 延迟非关键同步任务，但不得破坏向量时钟一致性。

### WebLightPeer (受限同步端点)
* **定位**：Web 端被定义为 **WebLightPeer**。它不是 Full Peer，而是一个受限的同步端点。
* **数据源**：支持在线增量拉取与受限推送。它拥有 repo-scoped identity 与 vector，但无完整本地 ledger。
* **状态边界**：浏览器 peer 状态严格按 `repo_id` 分桶；切换仓库等价于切换独立的 identity、vector、cache 与连接上下文。
* **存储模型**：从 `localStorage` 升级为分层存储。
    * **UI 偏好**：使用 `localStorage`。
    * **Peer Identity & Metadata**：使用 `IndexedDB` 存储 repo-scoped vector、cache metadata 与身份材料。
    * **私钥材料**：使用 `WebCrypto` 安全存储。
* **连接约束**：必须保持与 Server 的 WebSocket 连接。断连后进入只读模式，禁止离线编辑。
### 术语表 (Terminology)

**WebLightPeer** — 受限同步端点。浏览器作为轻量级 peer 参与同步，但受以下约束：
  - 无完整本地 ledger（仅在线状态下的 repo-scoped cache）
  - 无后台长期 gossip（依赖 Server Always-on Relay）
  - 仅 repo-scoped 同步（每个 repo 独立 identity/vector）

**DashboardSession** — 浏览器用户会话。通过 JWT Cookie 认证，与 peer identity 分离。

**PeerIdentity** — 节点身份。每个 repo 独立的 Ed25519 keypair，存储于 IndexedDB。

**RepoScopedVector** — 仓库作用域版本向量。WebLightPeer 为每个 repo 维护独立 vector。

**OfflineCache** — 离线缓存。IndexedDB 中存储的 repo-scoped metadata 与最近访问文档。

**DegradedSyncMode** — 降级同步模式。当 IndexedDB 不可用时，WebLightPeer 进入只读模式。

### 不变量 (Invariants)

**INV-1: Repo Scope Isolation**
- WebLightPeer 的 identity、vector、cache 必须按 repo_id 隔离
- 不允许跨 repo 共享 peer identity 或 vector state

**INV-2: Online Dependency**
- WebLightPeer 必须保持与 Server 的 WebSocket 连接才能工作
- 断连后进入只读模式，禁止离线编辑（与 Full Peer 不同）

**INV-3: Storage Separation**
- UI 偏好 → localStorage
- Peer identity 私钥 → WebCrypto secure storage
- Repo-scoped cache metadata → IndexedDB
- 业务数据 → Server ledger（WebLightPeer 不持久化文档内容）

**INV-4: Auth Layering**
- User session (JWT Cookie) 与 peer identity (Ed25519 keypair) 是独立的两层认证
- User session 验证用户访问权限，peer identity 验证同步数据来源

### 主节点 / 代理节点 (Main / Proxy)
* **动机**：Redb 为独占锁模型，同一时间只允许一个进程持锁。
* **策略**：当 `deve_cli serve` 检测到端口被占用或数据库已锁定时，自动降级为 **Proxy 模式**。
    * **Main**：持锁进程，监听配置的主端点，负责真实读写。
    * **Proxy**：不触碰数据库，通过同源 HTTP/WS 转发访问主节点。
* **路由契约**：浏览器 **MUST** 优先连接当前 origin 下的 `relative /ws`；Proxy 模式必须保持该相对路径可用，而不是要求前端感知后端真实端口。
* **端口策略**：本地开发环境 **MAY** 暴露显式端口用于诊断；生产环境 **MUST** 通过单一配置端点或反向代理提供稳定入口。
* **探测接口**：`GET /api/node/role` 返回 `{ role, ws_port, main_port }`。
* **前端行为**：生产环境默认使用 `relative /ws` 或显式配置的单一 WS 端点；端口探测仅允许作为本地开发兜底，不得作为规范默认行为。

## 连接与协议 (Connection & Protocol)

### WebSocket 协议类型 (Protocol Types)
*   **Format (格式)**: 节点间 (Server-to-Server) 与服务端下行 (Server-to-Client) 默认使用 **Bincode**；浏览器上行 (Client-to-Server) 以 Bincode 为优先格式，同时保留 JSON 文本兼容入口用于调试与旧客户端。
*   **ClientMessage (客户端消息)**:
    *   `SyncHello`, `SyncRequest`, `SyncPush`: P2P 同步协议消息；凡进入同步路径的消息 **MUST** 携带可确定路由的 `repo_id`。
    *   `Edit`, `Cursor`, `OpenDoc`, `CreateDoc`: 编辑器操作消息。
    *   `PluginCall`: 远程插件调用请求。
*   **ServerMessage (服务端消息)**:
    *   `TreeUpdate(TreeDelta)`: 文件树增量更新。
    *   `FsChangeDetected`: 文件系统变更通知，提示客户端按当前 session repo 重新拉取列表/状态。
    *   `NewOp`: 实时协作操作事件。
    *   `Snapshot`: OpenDoc 文档快照，绑定于当前 session repo；同步回退快照则使用显式 `repo_id` 的 `SyncSnapshotRequest` / `SyncPushSnapshot`。

### OpenDoc 性能策略 (Snapshot-First + Progressive Prefetch)
*   **Snapshot-First**: 打开文档优先返回最近快照 + 增量 Ops。
*   **Client Prefetch**: 客户端按自适应批次应用增量 Ops。
*   **Search Gate**: 见 [03_rendering.md §大文档渲染策略](./03_rendering.md)。

### WebLightPeer Handshake (身份与握手)

*   **Setup (初始化)**: 用户会话建立后，浏览器为当前 `repo_id` 读取或生成独立的 Ed25519 keypair，并恢复该 repo 的 vector 与 cache metadata。
*   **Repo-Scoped Identity**: `repo_id_a` 与 `repo_id_b` 必须映射到不同的 peer state；切换 repo 时不得复用前一个仓库的 identity、vector 或订阅。
*   **Handshake Flow**:
    1.  WebLightPeer 通过 `relative /ws`（或显式配置端点）建立连接，并声明自身角色为受限同步端点。
    2.  客户端发送 `SyncHello { repo_id, peer_pubkey, vector, session_proof }`；其中 `repo_id` 是服务器路由与权限校验的主键。
    3.  Server 校验用户会话、仓库访问权限与 `repo_id` 对应的路由上下文，随后绑定该连接到单个 repo。
    4.  Server 返回 `ServerMessage::SyncHello { peer_id, pub_key, signature, vector }` 作为握手回执，并按 diff 结果追加 `SyncRequest` / `SyncSnapshotRequest` / `SyncPush`。
    5.  后续 `SyncRequest`、`SyncPush`、`SyncSnapshotRequest`、`SyncPushSnapshot` 与实时同步广播均 **MUST** 沿用同一 `repo_id`，否则服务器必须拒绝或断开连接。
*   **Deterministic Routing (确定性路由)**:
    *   `SyncHello` **MUST** 提供 `repo_id` 与当前 vector，确保 Server 能决定是走增量同步还是快照回退。
    *   `SyncRequest` **MUST** 至少携带 `{ repo_id, known_vector }`；禁止依赖连接外的隐式默认 repo。
    *   `Snapshot` **MUST** 携带其所属 `repo_id` 与生成时的 server vector；协议示例不得使用空 repo 占位符。
*   **Secure Keystore (安全信任列表)**:
    *   所谓 "Trusted List" 实质上是 **Verified Peer Keystore**。
    *   **Content**: 包含 `{ repo_id, PeerID, PubKey, SharedRepoKeys, HandshakeSignature }`。
    *   **Tamper-Proof**: 若 B 本地篡改列表添加了 A 的 ID，但 B **缺失** A 在握手时加密传输的 `SharedRepoKeys`，则 B 无法解密 A 的数据。

### Sync Process (同步流程)

*   **Security (E2EE)**:
    *   **Encryption**: 所有 Repo 数据在传输前 **MUST** 使用 `RepoKey` 加密 (AES-256-GCM)。
    *   **Defense**: 即使 C (Relay) 恶意投递了 A 的数据给未授权的 B，或 B 篡改了本地信任列表强制接收，由于 B 没有 **RepoKey**，数据对 B 而言通过是乱码 (Garbage)。

*   **Performance Optimization (Envelope Pattern)**:
    *   为了不影响 Gossip 运算性能，系统采用 **信封模式**:
        *   **Header (Plaintext)**: 包含 `VectorClock`, `PeerID`, `RepoID`。Relay 节点仅需读取 Header 即可完成 Gossip 差异计算与路由 (Zero Decrypt Overhead)。
        *   **Body (Encrypted)**: 仅实际的 Diff/Snapshot 数据被加密。
    *   **Impact**: AES-NI 硬件加速下，Payload 加密对 CPU 开销几乎可忽略，且不阻塞 Gossip 逻辑运算。

*   **Logic**: **Vector Gossip**。
    *   **Trigger**: 同步仅在 **Vector Clock Comparison** 发现差异时触发 (e.g., $VC_A > VC_B$)。这确保了包含操作序列数的 Header 是决定传输的唯一依据。
    *   **Mechanism (Operation-Based)**:
        1.  **Compare**: Server 在 `repo_id` 作用域内比较 $VC_B$ (B's State) vs $VC_A$ (A's State)。
        2.  **Calculate**: A 计算出 B 在该 repo 中缺失的操作序列 (Missing Ops = $Ops[VC_B.Seq+1 ... VC_A.Seq]$)。
        3.  **Send**: A 仅发送这些缺失的 **Operations** (Payload)，而非整个文件或文件 Diff。
        4.  **Apply**: B 接收 Ops 并追加到本地的 Remote Branch 中。
        5.  **Update VC**: B 成功写入后，**MUST** 更新本地记录的 $VC_{PeerA}$ 至最新 Seq。这将作为下一次比对的基准。
    *   **Direct Write**: B 作为镜像端，**MUST** 直接接受来自 A 的已校验数据（无需本地冲突消解，因为 B 是只读的）。
*   **Web Request Contract**: WebLightPeer 发起的 `SyncRequest`/`SyncPush` 必须显式带上 `repo_id`，以便 Proxy/Main/Relay 在零解密前提下完成确定性路由。

*   **Flow Control**: 支持断点续传与背压。

### WebSocket Reconnection (重连策略)

*   **Strategy**: Exponential Backoff with Jitter。
*   **Intervals**: 1s → 2s → 4s → 8s → 16s → 30s (cap)。
*   **Max Retries**: 无限 (用户手动关闭才停止)。
*   **Endpoint Rule**: 重连目标优先保持为 `relative /ws`；仅在显式配置了单一外部 WS 地址时才覆盖同源路径。
*   **UI Feedback**:
    *   断连后立即显示 "Reconnecting..." 遮罩。
    *   每次重连尝试更新计数器 "Retry #N..."。
    *   重连成功后自动请求增量同步 (SyncHello)。
*   **State Recovery**: 重连成功后 MUST 重新发送当前 repo 的 `SyncHello` 获取离线期间的变更；若用户已切换 repo，则必须以新 `repo_id` 重建连接上下文。

### Edge Cases & Safety Strategy (边界与安全)

*   **Snapshot Sync (Fast Forward)**:
    *   **Scenario**: 当 OpSeq 差异过大 (e.g., GAP > 1000) 或 Peer 首次接入时。
    *   **Strategy**: 自动切换为 **Direct Overwrite** 模式。
    *   **Action**: A 发送当前 `repo_id` 状态的完整快照 (`Snapshot { repo_id, server_vector, payload }`)，B 直接覆盖对应的 Remote Branch。这比重放 100 万条日志更高效 (解决算力/带宽平衡问题)。
    *   **Guardrail**: Snapshot 回退只允许在已知 repo 路由上发生；禁止使用空 repo 占位符或跨 repo 复用快照。

*   **Strategy Selection (策略选择 - Why Ops?)**:
    *   **Q: 对于小文件，直接覆盖是否更优？**
    *   **A**: 对于 **低频同步**，直接覆盖可行。但对于 **实时协作 (Real-time)**，Ops 依然占优：
        *   **Bandwidth**: 修改一个字符，Ops 仅需几十字节；Snapshot 需传输整个文件 (e.g., 10KB)。Ops 带宽占用低 2-3 个数量级。
        *   **Granularity**: Ops 保留了 "操作意图" (Insert/Delete)，这是后续实现自动合并 (CRDT) 和历史回溯 (Time Travel) 的基础。直接覆盖会丢失这些上下文。

*   **Malicious Defense & Rollback (恶意防御与回滚)**:
    *   **Isolation**: 远端传来的恶意 Ops (e.g., "Delete All") 仅会影响 `ledger/remotes/peer_a/`，**绝不会** 自动污染用户的 `ledger/local/`。
    *   **Undo Capability**: 若用户不小心合并了恶意分支，Local Ledger 本身支持 **Time Travel (Undo/Redo)**。用户可随时回滚 Local Branch 到任意历史状态。
    *   **OpSeq Limitation**: OpSeq 为 64-bit 整数，即使每秒写入 100万次，也需 58 万年才会溢出。

### Indirect Sync & Trust Boundary (间接同步与信任边界)

*   **Scenario**: A (Offline) -> C (Relay) -> B (Online).
*   **Case 1: A & B are Trusted (Has Handshake)**
    1.  C 发送 Gossip Offer (`I have updates for A, C`).
    2.  B 检查本地 Trusted List，发现 `A` 在列表中 (已握手)。
    3.  B 向 C 发送 Fetch Request (`Get A's updates`).
    4.  C 传输 A 的数据给 B。
*   **Case 2: A & B are Strangers (No Handshake)**
    1.  C 发送 Gossip Offer (`I have updates for A, C`).
    2.  B 检查本地 Trusted List，发现 **不认识 A** (未握手)。
    3.  B **MUST Ignore** A's offer (Strict Filtering).
    4.  C **MUST NOT** 传输 A 的数据给 B (Payload Blocking).
    *   **Result**: B 仅接收已建立信任关系 Peer 的数据。C 虽持有 A 的数据，但不会泄露给陌生人 B。

*   **Storage Attribution (存储归属)**:
    *   **Rule**: 数据存储路径主要由 **Data Source Signature** 决定，而非传输管道。
    *   **Behavior**: 即使数据包由 C (Relay) 转交，只要签名验证通过显示来源为 A，B **MUST** 将其写入 `ledger/remotes/peer_a/`，绝不可写入 `peer_c`。C 仅作为透明管道 (Transparent Pipe)。

### Data Integrity Analysis (数据一致性分析)

*   **Q: vector gossip 会导致数据丢失吗?**
*   **A: NO (不会)。设计保证了零数据丢失 (Zero Data Loss)。**
    *   **Reason 1 (Separation)**: 我们传输的是 **Replication Log** (A -> Mirror A)，而不是 Merge Result。网络层只负责搬运 A 的日志到 B 的镜像区，不发生任何合并冲突。
    *   **Reason 2 (Source Reliability)**: 既然 A 是 Source of Truth，只要 A 本地不炸，数据就永远存在。
    *   **Reason 3 (Receiver Recovery)**: 即使 B 恶意篡改本地 Vector Clock (例如谎称已由 Seq 100)，导致 A 停止发送。这只会导致 B 自己的镜像 **"停更" (Stale)**，而不会导致 A 的数据丢失。一旦 B 恢复诚实汇报真实 Seq，A 会立即补发缺失数据。

## 本章相关命令

* 无。

## 本章相关配置

*   `SYNC_MODE`: `auto` (Default, 后台自动拉取与合并) | `manual` (StrictMode, 仅交换 Vector，需显式 Fetch/Merge)。
    *   **Mobile Override**: 移动端（iOS/Android）后台状态下，`SYNC_MODE` 设置 **MUST** 被强制覆盖为 `light-peer` 模式：仅被动接收 Server Relay 推送的更新，禁止主动 Gossip/合并。
