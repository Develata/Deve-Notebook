# 04_storage.md - 数据存储篇 (Data Storage)

## 三库隔离 (The Trinity Isolation)

实现“绝对数据主权”和“零污染”。本地不再是单体数据库，而是物理隔离的存储结构：

*   **Store A (Vault)**: 本地 Markdown 工作区 ($W_{user}$)。
    *   **Nature**: 投影 ($P$)。由 Ledger 实时投影生成，允许包含脏读状态 (Dirty Read)。
*   **Store B (Local Branch)**: 本地权威分支 ($B_{local}$).
    *   **Physical Path**: `/data/ledger/local/`.
    *   **Content**: 包含多个 **Repo Instances** (e.g., `personal_repo_uuid.redb`).
    *   **Nature**: 唯一真值源。仅本地用户可写。
*   **Store C (Remote Branches)**: 远端影子分支集合 ($\Sigma_{remote}$).
    *   **Physical Path**: `/data/ledger/remotes/<RemoteName>/`.
    *   **Indexing**: 系统依靠 `PeerUUID` 检索对应的 `RemoteName` 文件夹。
    *   **Content**: 包含该 Remote 视角下的多个 **Repo Instances**.
    *   **Nature**: 只读镜像。Editor 不可写，但允许后端 Vector Gossip 协议根据 vector clock 进行覆盖更新。

## Branch Storage Mapping (分支存储结构)

*   **Structure**: `ledger/` 目录包含 `local/` 和 `remotes/` 两个平权子目录。
*   **Equality (平权性)**: `local` 文件夹与 `remotes/<name>` 文件夹在结构上完全等价，均对应一个 **Branch**。
*   **File Layout (文件布局)**:
    *   `/data/ledger/local/my-wiki.redb` (Metadata: `URL=..., UUID=...`)
    *   `/data/ledger/remotes/ipad-pro/my-wiki.redb` (Metadata: `URL=..., UUID=...`)
    *   **Filename Rules**:
        *   文件名 **MUST** 是人类可读的 `repo_name.redb`。
        *   **Conflict Strategy**: 若同个 Branch 下出现同名但不同 URL 的 Repo，必须自动重命名 (e.g., `wiki.redb` -> `wiki-1.redb`)。
*   **Indexing**: 系统使用 Redb 维护多组映射表以实现 `O(1)` 双向查找：
    *   `NODEID_TO_META`: `u128 -> NodeMeta` (统一节点元数据)
    *   `PATH_TO_NODEID`: `&str -> u128` (路径解析)
    *   `INODE_TO_NODEID`: `u128 -> u128` (重命名追踪, 文件节点)
    *   **Op Log (操作日志)**:
        *   `LEDGER_OPS`: `u64 -> &[u8]` (全局有序日志, Key=SeqNo, Value=Bincode Serialized Entry)
        *   `DOC_OPS`: `u128 -> [u64]` (Multimap, 允许快速检索单一文档的所有变更 Seq)
    *   **Atomic Sequence (原子序号)**:
        *   `PEER_DOC_SEQ`: `(DocId, PeerId) -> u64`。用于生成严格单调递增的 `OpSeq`，防止并发冲突。
*   **Virtual Backup**: 系统 MAY 为当前活跃 Repo 自动创建 `.redb` 文件的只读快照。
    *   **Frequency**: 每日自动 (可配) 或手动触发 (`deve backup`).
    *   **Storage**: `ledger/backups/<repo_name>-<timestamp>.redb`.
    *   **Retention**: 默认保留最近 3 份；超出按 FIFO 删除.
*   **Virtual Backup**: 针对每个 Repo Instance (`.redb`) 可存在对应的只读快照。

## Repository Manager (仓库管理器)

* **职责**：管理 `Local Repo` (Store B) 和 `Shadow Repos` (Store C)。
* **Routing**：VFS 根据 UI 上下文路由到对应的 `.redb` 实例。
* **Snapshot Strategy**:
    *   **Dual-Table Structure**: 为了优化性能，快照存储拆分为两个表 (verified in `schema.rs`):
        *   `SNAPSHOT_INDEX`: 索引表 (`DocId -> [SeqNo]`)，用于快速检索历史版本号。
        *   `SNAPSHOT_DATA`: 数据表 (`SeqNo -> ContentBlob`)，存储实际快照内容。
    *   **Pruning**: 每个 Repo 独立维护自己的 Snapshot 链，并根据配置深度 (`snapshot_depth`) 进行自动修剪。

## Synchronization Architecture (同步架构)

实现单向数据流与原子持久化策略：

*   **Core Data Model (核心数据模型)**：
    *   **Definition**: 对任意 Repo，权威状态定义为有序操作日志：
        *   $DB = \mathrm{OrderedLog}\langle LedgerEntry \rangle$
    *   **Interpretation**: `LedgerEntry` 包含 `(Op, PeerId, OpSeq, GlobalSeq)`，全局序号 `GlobalSeq` 决定落盘线性顺序，`PeerId + OpSeq` 用于因果/缺失检测。
    *   **Pending FS Ops**: 系统维护 `pending_fs_ops` 表，存储 Watcher 检测到的但尚未 Commit 的文件系统变更（类比 Git Working Directory）。
    *   **Projection Rule**: Store A ($W_{user}$) 上的任意文件内容 **MUST** 可由 `Replay(DB)` 唯一导出；文件系统仅是投影，不是权威源。
    *   **Ordering Rule**: 同一 Repo 内操作落盘顺序 **MUST** 由 `GlobalSeq` 决定，任何并发写入 **MUST** 在落盘前被线性化。
    *   **Version Control**: Commit 锚定到特定 `ledger_seq`，形成版本历史链（类比 Git commit）。
    *   **Metadata Directory**: 系统使用 `vault/<repo_name>/.notegit/` 目录存储 repo-scoped 运行时元数据（类比 `.git/`，包含 repo keys、迁移归档等）。
        *   **Location**: `.notegit/` 位于对应 Repo 工作区根目录下。
        *   **Watcher Ignore**: `.notegit/` **MUST** 被 Watcher 忽略（不触发变更检测）。
        *   **Backup Policy**: `.notegit/` **SHOULD** 随对应 Repo 一起备份，但 **MUST NOT** 被跨 Repo 复用。
    *   **Host Runtime Directory**: 宿主级身份与配置存储于 `ledger/.host/`。
        *   **Content**: `identity.key`、`mcp.json` 等 host-scoped 状态。
    *   **Definition**: 对任意 Repo，权威状态定义为有序操作集合：
        *   $DB = \mathrm{Set}\langle (Op, Time) \rangle$
    *   **Interpretation**: `Op` 表示最小可重放变更单元，`Time` 表示全序比较键（由 `PeerId + OpSeq` 复合构成）。
    *   **Projection Rule**: Store A ($W_{user}$) 上的任意文件内容 **MUST** 可由 `Replay(DB)` 唯一导出；文件系统仅是投影，不是权威源。
    *   **Ordering Rule**: 同一 Repo 内操作顺序 **MUST** 由 `Time` 决定，任何并发写入 **MUST** 在落盘前被线性化。
    *   **Version Control**: Commit 锚定到特定 `ledger_seq`，形成版本历史链（类比 Git commit）。
    *   **Metadata Directory**: 系统使用 `vault/<repo_name>/.notegit/` 目录存储 repo-scoped 运行时元数据。

*   **Ledger-First Strategy (账本优先策略)**:
    *   前端编辑器生成的变更 **MUST** 直接作为 `Op` 写入 Store B (Ledger)。
    *   Watcher 检测到的后台 Vault 变更 **MUST NOT** 直接入 Ledger；**MUST** 先写入 `pending_fs_ops`，等待用户手动 Commit。
    *   绝不允许绕过 Ledger 直接修改 Store A (Vault) 文件。


*   **Ledger-First Strategy (账本优先策略)**:
    *   所有的变更 (Edit, Discard, etc.) **MUST** 首先作为 `Op` 写入 Store B (Ledger)。
    *   绝不允许绕过 Ledger 直接修改 Store A (Vault) 文件，唯一的例外是外部编辑器的 `Watcher` 触发 Ingestion。
*   **Atomic Persistence (原子持久化)**:
    *   **Component**: `SyncManager` 负责协调 Op 应用与文件写入。
    *   **Method**: `apply_local_op_and_persist`.
        1.  **Append**: 调用 `RepoManager` 将 Op 写入 Redb。
        2.  **Reconstruct**: 基于 Ledger 计算最新文档快照。
        3.  **Persist**: 立即将快照写入 Vault 文件系统 ($W_{user}$)。
    *   **Logic**: `Op -> Ledger -> Snapshot -> Disk`. 确保文件系统总是 Ledger 的最新投影。
*   **Batch Optimization (批量优化)**:
    *   对于批量操作 (如 `Discard` 重置整个文件)，系统 **SHOULD** 批量应用 Ops (仅写入 DB)，并在最后执行一次 `persist_doc`，以减少 I/O 开销。

## Clean File Policy (纯净文件策略)

*   **Implicit Tracking (隐式追踪)**: 系统 **MUST** 使用 `NodeId` (UUID) 作为内部追踪标识。
    *   **Storage Location**: `NodeId <-> Path/Inode` 的映射表 **MUST** 存储在 **Store B (Local Repo)** 的专用 Table/Bucket 中，严禁存储在 Markdown 文件内。
*   **Zero Injection (零注入原则)**: 系统 **MUST NOT** 向用户创建的 Markdown 文件中注入任何元数据（如 YAML Frontmatter 中的 UUID）。
*   **Metadata Source (元数据溯源)**: 即使文件中存在用户手写的 Frontmatter，系统也 **MUST** 视其为普通文本内容 (Payload)。
    *   **No Impact**: 投影中的 Frontmatter 修改 **MUST NOT** 反向影响 Ledger 中的系统元数据（如 Creation Time, UUID等）。Ledger 的元数据仅由 Authoritative Ops 变更。

## Inode/DocId Mapping & Watcher Service (映射与监听)

*   **Store A -> Store B (Ingestion Flow)**:
    *   **Watcher Service**: 系统核心 **MUST** 运行一个文件系统监听服务 (Watcher)，实时监控 Vault 目录。
        *   **Create / Modify**: 监测到 Markdown 文件的新增或内容变更 -> 触发 Ledger **写入/更新**操作 (Append Ops)。
        *   **Delete**: 监测到 Markdown 文件被移除 -> 触发 Ledger **标记删除**操作 (Mark Deleted)。
        *   **Rename / Move**: 监测到重命名或移动 -> 更新 **Path Mapping**，保持 `NodeId` 不变。
    *   **External Tools Support**: 必须兼容 VS Code, Vim, Nano 等外部编辑器的原子写入 (Atomic Write) 和重命名行为。
    *   **Mechanism**: Watcher Event -> Debouncer -> Inode Tracker -> Op Generator.
    *   **Constraints**:
        *   **Idempotency (幂等性)**: 重复的信号触发 **MUST** 产生相同的结果状态。
        *   **Rename Detection**: 系统 **MUST** 利用 OS 提供的 Inode (或 FileID) 追踪文件重命名，避免 DocId 丢失或重建。

## Data Integrity & Recovery (数据完整性与灾备)

*   **Append-Only Log**: 所有写操作 **MUST** 以日志追加形式 (Append Only) 记录，**MUST NOT** 执行原地修改 (In-Place Mutation)。
*   **Projection Strategy**: Markdown 文件仅为 Ledger 的投影 ($P$)。系统 **SHOULD** 优先信任 Ledger 数据。
*   **Recovery Scenarios (恢复场景)**:
    *   **Vault Corruption (误删/篡改)** -> **Rebuild**: 从 Ledger 重放 (Replay) 并强制覆盖文件系统。
    *   **Ledger Corruption (损坏)** -> **Reverse Import**: 从 Vault 文件反向生成新的 Ledger (Reset History)。
    *   **State Deviation (状态错乱)** -> **Hard Reset**: 清空 Store B 并从头重建。
*   **Disaster Recovery (灾难恢复)**: 系统 **MUST** 提供将 Ledger 导出为 JSON Lines 格式的能力，确保数据的可移植性 (Portability)。
*   **Schema Stability & Migration (架构稳定性与迁移)**:
    *   **Fixed Schema**: 本地数据存储路径与核心 Schema 结构已固定，**SHOULD NOT** 发生变更。
    *   **Manual Migration (手动迁移)**:
        *   若发生不可避免的 Breaking Change，系统 **MUST NOT** 尝试执行复杂的原地 Migration 脚本（风险过高）。
        *   **Strategy**: 采用 "Copy & Rebuild" 策略。用户只需保留 `vault` 中的 Markdown 原文件（Source of Truth 的物理投影），在新版本中重新 `init` 并导入即可。后端程序更新不会破坏本地数据，仅需复制数据文件即可完成环境迁移。

## Core Interaction Constraint (核心交互约束)

*   **UUID-First Retrieval (UUID 优先检索)**:
    *   **Rule**: 后端对于任意 File/Folder/Repo 的检索与操作，**MUST** 仅通过 `NodeId/RepoUUID` 完成，严禁直接使用 File Path 作为主键。
    *   **Resolution Flow**:
        1.  **Frontend**: 允许传递用户可读的 `Name` 或 `Path`。
        2.  **Resolution**: 后端接收到 Name 后，**MUST** 先查询映射表 (`Name/Path` -> `NodeId`) 获取唯一标识。
        3.  **Execution**: 所有的业务逻辑执行 (Execution) **MUST** 仅针对 UUID 进行。
    *   **Rationale**: 确保在文件/目录重命名或移动（Path 变更）时，正在进行的后台任务（如 Embeddings, Sync）不中断，且路径不一致时以 UUID 指向的实体为准。

## Node Entity Unification (统一节点实体)

> 目标: 为未来自研 VFS 提前建立同构语义层。

*   **Definition**: 目录与文件统一为 `Node`，以 `NodeId` 作为唯一标识。
*   **NodeMeta**:
    *   `kind`: `File | Dir`
    *   `name`: 当前节点名
    *   `parent_id`: 父节点 `NodeId`
*   **Path Strategy**: `path` 为可重建缓存，不作为主键。
*   **Invariants**:
    *   任何节点均有且仅有一个 `NodeId`。
    *   树结构由 `parent_id` 关系完全决定。
    *   文件内容仅对 `kind=File` 生效。

## Cross-Platform Path Strategy (跨平台路径策略)

为解决 Windows/Linux/macOS 路径分隔符不一致的问题，系统实施严格的路径规范化策略：

*   **Canonical Internal Format (内部权威格式)**:
    *   **Rule**: 所有存储在 Ledger、KV Database、Memory Cache 中的路径字符串，**MUST** 统一使用 Linux 风格的正斜杠 (`/`) 分隔符。
    *   **Scope**: `DocId <-> Path` 映射表、Op Logs、Protocol Messages (Sync/Gossip)。
    *   **Example**: `folder/subfolder/file.md` (Valid), `folder\subfolder\file.md` (Invalid).
*   **Normalization Boundary (规范化边界)**:
    *   **Ingestion (输入)**: 当从 OS 文件系统读取路径时 (e.g. Watcher events, File Dialogs)，**MUST** 立即调用规范化函数 (`to_forward_slash`) 转换为内部格式。
    *   **Interaction (输出)**: 仅在直接调用 OS 文件系统 API (e.g. `std::fs`, `open_file`) 的瞬间，**SHOULD** 调用转换函数 (`to_native`) 还原为系统原生格式。
*   **Implementation**:
    *   核心库提供 `crates/core/src/utils/path.rs` 标准模块，包含 `to_forward_slash` 和 `to_native` 方法，所有路径操作 **MUST** 通过此模块进行，严禁手动通过字符串替换 (`replace`) 处理。

## Browser Storage Layering (浏览器存储分层)

WebLightPeer 的浏览器侧存储必须按安全等级与恢复语义分层，禁止再以单一 `localStorage` 承担身份、会话与缓存职责。

*   **UI prefs (`localStorage`)**:
    *   **Allowed**: 主题、侧栏宽度、语言、最近展开面板等纯前端偏好。
    *   **Forbidden**: `peer identity`、`session token`、repo vector、离线缓存内容。
    *   **Recovery**: 用户清理浏览器站点数据后可直接重建，不影响 Server ledger 真值。
*   **User Session (`JWT Cookie`)**:
    *   **Primitive**: `HttpOnly + Secure + SameSite=Strict` Cookie。
    *   **Allowed**: 用户访问权限、登录态续存、服务端 API / WebSocket 握手鉴权。
    *   **Forbidden**: 作为 `peer identity` 使用；不得承载 repo-scoped vector 或离线缓存。
    *   **Recovery**: Cookie 过期、登出或密码修改后，客户端必须重新登录获取新的 session token。
*   **Peer Identity (`WebCrypto + IndexedDB`)**:
    *   **Primitive**: 浏览器首次进入某个 `repo_id` 时，调用 `WebCrypto.subtle.generateKey(...)` 生成 repo-scoped Ed25519 keypair，私钥 **MUST** 设为 `extractable: false`。
    *   **Allowed**: `peer identity` 公钥、key handle、注册状态、最后一次握手元数据持久化在 IndexedDB `peer_identity` store。
    *   **Forbidden**: 私钥原始字节、seed 或可导出密钥材料写入 `localStorage`、URL、Cookie、日志。
    *   **Recovery**: 若 IndexedDB 中仍存在对应 `CryptoKey`/metadata，则恢复原 browser peer；若 identity 丢失，则必须重新生成 keypair 并重新执行 trust registration。
*   **Offline Cache (`IndexedDB`)**:
    *   **Allowed**: repo-scoped vector clock、最近访问文档摘要、只读缓存、同步检查点与 UI 预热数据。
    *   **Forbidden**: 认证 cookie、私钥材料、跨 repo 共用缓存桶。
    *   **Recovery**: 配额清理或用户手动删除后可从 Server 增量重建；缓存缺失不得改变权威账本。

### Trust Registration Flow (信任注册流程)

1. 浏览器已具备有效 user session，但首次打开某个 repo 时仍视为“未注册 peer identity”。
2. WebLightPeer 查询 IndexedDB 的 `peer_identity` store；若不存在该 `repo_id` 记录，则调用 `WebCrypto` 生成新的 repo-scoped keypair。
3. 私钥保持为不可导出的 `CryptoKey`；公钥、`peer_id`、注册时间与握手状态写入 IndexedDB。
4. 浏览器发送 `SyncHello { repo_id, peer_pubkey, vector }` 到 Server，请求将该 browser peer 纳入 repo trust set。
5. Server 以 session token 验证“谁在访问”，再以 `peer identity` 公钥记录“哪个节点在同步”，两者缺一不可但职责不同。
6. 注册成功后，后续 `SyncRequest`/`SyncPush` 仅使用该 repo-scoped peer identity 参与签名与验证。

### Recovery Semantics (恢复语义)

*   **IndexedDB + WebCrypto 可用**: 进入正常 WebLightPeer 模式，允许 repo-scoped 拉取与受限推送。
*   **仅 Cookie 可用，IndexedDB 不可用**: 进入 `DegradedSyncMode`。用户仍可保留登录态，但因无法持久化 peer identity 与 offline cache，UI **MUST** 切换为只读并禁止 `SyncPush`。
*   **Cookie 失效但 IndexedDB identity 仍在**: 必须重新登录；既有 peer identity 不自动授予 API 访问权限。
*   **用户清除站点数据**: `UI prefs`、peer identity、offline cache 全部丢失；下次访问必须重新登录并重新执行 browser peer 注册。

## 本章相关命令

* 无。

## 本章相关配置

*   `vault.path`: Store A 的根目录路径 (Default: `/data/vault`).
*   `ledger.path`: Store B/C 的根目录路径 (Default: `/data/ledger`).
