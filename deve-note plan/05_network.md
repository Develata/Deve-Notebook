# 05_network.md - 网络架构篇 (Network Architecture)

## 拓扑定义：P2P 三角与 Web 面板 (The P2P Triangle & Web Dashboard)

### P2P Mesh (对等网络)
* **核心节点**：仅包含 **Desktop Native (PC/Mac)**、**Mobile Native (iOS/Android)** 和 **Server (Linux)**。
* **机制**：这三方拥有独立的 `PeerID` 和 **Local Branch** (数据集合)，通过 Gossip 协议交换 Repo Instances。
* **Server 角色**：Server 在 P2P 网络中充当 **Always-on Relay Peer** (全天候中继/备份节点)。

### Web Client (服务器面板)
* **定位**：Web 端**不是**一个独立的 P2P 节点，而是 **Server 节点的远程操作面板 (Remote Dashboard)**。
* **数据源**：Web 端直接通过 WebSocket 读写 Server 的内存/数据库。Web 端显示的 "Local" 即为 Server 的 `Local Branch` (ledger/local/)。
* **存储**：**Stateless / RAM-Only (纯内存)**。Web 端严禁使用 IndexedDB/LocalStorage 存储业务数据。它只是 Server 状态的“易失性投影”。
* **连接约束**：Web 端必须保持与 Server 的 WebSocket 连接才能工作。**断连即锁屏**，严禁离线编辑。

### 主节点 / 代理节点 (Main / Proxy)
* **动机**：Redb 为独占锁模型，同一时间只允许一个进程持锁。
* **策略**：当 `deve_cli serve` 检测到端口被占用或数据库已锁定时，自动降级为 **Proxy 模式**。
    * **Main**：持锁进程，监听主端口 (默认 3001)，负责真实读写。
    * **Proxy**：不触碰数据库，通过 HTTP 转发访问主节点。
* **端口策略**：Proxy 自动选择主端口 + 1 的空闲端口 (如 3002)。
* **探测接口**：`GET /api/node/role` 返回 `{ role, ws_port, main_port }`。
* **前端行为**：默认尝试连接 3001..3005；也支持 `?ws_port=xxxx` 强制指定。

## 连接与协议 (Connection & Protocol)

### WebSocket 协议类型 (Protocol Types)
*   **Format (格式)**: 节点间 (Server-to-Server) 使用 **Bincode** 以确保性能；客户端与服务端 (Client-Server) 使用 **JSON** 以便调试。
*   **ClientMessage (客户端消息)**:
    *   `SyncHello`, `SyncRequest`, `SyncPush`: P2P 同步协议消息。
    *   `Edit`, `Cursor`, `OpenDoc`, `CreateDoc`: 编辑器操作消息。
    *   `PluginCall`: 远程插件调用请求。
*   **ServerMessage (服务端消息)**:
    *   `TreeDelta`: 文件树增量更新。
    *   `NewOp`: 实时协作操作事件。
    *   `Snapshot`: 完整文档内容快照。

### Peer Identity & Handshake (身份与握手)

*   **Setup (初始化)**: 用户在设置中配置 `Name` (e.g., "My iPad")。
*   **First Handshake (TOFU)**:
    1.  Peer A 连接 Peer B。
    2.  双方自动生成并交换 **Key Pair**。
    3.  **Binding**: 此后，Key Pair 成为识别对象的唯一凭证 (True Name)。
    4.  **ID Assignment**: 系统为每个验证通过的 Key Pair 分配唯一的 **IdSeq (PeerUUID)**，内部以此指代 (e.g., `peer_a`, `peer_b`)。
*   **Reconnection**: 后续连接通过 PubKey 验证，无需人工干预。
*   **Secure Keystore (安全信任列表)**:
    *   所谓 "Trusted List" 实质上是 **Verified Peer Keystore**。
    *   **Content**: 包含 `{ PeerID, PubKey, SharedRepoKeys, HandshakeSignature }`。
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
        1.  **Compare**: $VC_B$ (B's State) vs $VC_A$ (A's State).
        2.  **Calculate**: A 计算出 B 缺失的操作序列 (Missing Ops = $Ops[VC_B.Seq+1 ... VC_A.Seq]$)。
        3.  **Send**: A 仅发送这些缺失的 **Operations** (Payload)，而非整个文件或文件 Diff。
        4.  **Apply**: B 接收 Ops 并追加到本地的 Remote Branch 中。
        5.  **Update VC**: B 成功写入后，**MUST** 更新本地记录的 $VC_{PeerA}$ 至最新 Seq。这将作为下一次比对的基准。
    *   **Direct Write**: B 作为镜像端，**MUST** 直接接受来自 A 的已校验数据（无需本地冲突消解，因为 B 是只读的）。

*   **Flow Control**: 支持断点续传与背压。

### Edge Cases & Safety Strategy (边界与安全)

*   **Snapshot Sync (Fast Forward)**:
    *   **Scenario**: 当 OpSeq 差异过大 (e.g., GAP > 1000) 或 Peer 首次接入时。
    *   **Strategy**: 自动切换为 **Direct Overwrite** 模式。
    *   **Action**: A 发送当前状态的完整快照 (Snapshot)，B 直接覆盖对应的 Remote Branch。这比重放 100 万条日志更高效 (解决算力/带宽平衡问题)。

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
