# 04_storage.md - 数据存储篇 (Data Storage)

## 三库隔离 (The Trinity Isolation)

实现“绝对数据主权”和“零污染”。本地不再是单体数据库，而是物理隔离的存储结构：

*   **Store A (Vault)**: 本地 Markdown 工作区 ($W_{user}$)。
    *   **Nature**: Repo-scoped Workspace Projection 的物理容器。
    *   **Authority Rule**: Vault 不是权威源；其“规范状态”来自 Ledger 投影。
    *   **Dirty Workspace Rule**: Vault 允许临时包含外部编辑器产生的未提交差异，但该偏差 **MUST** 由 repo-scoped `pending_fs_ops` / `staging` 显式跟踪；未被跟踪的偏差视为实现错误。
*   **Store B (Local Branch)**: 本地权威分支 ($B_{local}$)。
    *   **Physical Path**: `/data/ledger/local/`.
    *   **Content**: 包含多个 **Repo Instances** (e.g., `personal_repo_uuid.redb`)。
    *   **Nature**: 本地唯一权威真值源，仅本地用户可写。
*   **Store C (Remote Branches)**: 远端影子分支集合 ($\Sigma_{remote}$)。
    *   **Physical Path**: `/data/ledger/remotes/<RemoteName>/`.
    *   **Indexing**: 系统依靠 `PeerUUID` 检索对应的 `RemoteName` 文件夹。
    *   **Content**: 包含该 Remote 视角下的多个 **Repo Instances**。
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
    *   **Ledger Log (账本事实日志)**:
        *   `LEDGER_OPS`: `u64 -> &[u8]` (全局有序日志, Key=SeqNo, Value=Bincode Serialized Entry)
        *   `DOC_OPS`: `u128 -> [u64]` (Multimap, 允许快速检索单一文档的所有变更 Seq)
    *   **Atomic Sequence (原子序号)**:
        *   `PEER_DOC_SEQ`: `(DocId, PeerId) -> u64`。用于生成严格单调递增的 `LedgerSeq`，防止并发冲突。
*   **Repo Runtime Directory**: 系统使用 `vault/<repo_name>/.notegit/` 存储 repo-scoped 运行时元数据（repo keys、pending/staged side tables、迁移归档等）。
    *   **Location**: `.notegit/` 位于对应 Repo 工作区根目录下。
    *   **Watcher Ignore**: `.notegit/` **MUST** 被 Watcher 忽略（不触发变更检测）。
    *   **Backup Policy**: `.notegit/` **SHOULD** 随对应 Repo 一起备份，但 **MUST NOT** 被跨 Repo 复用。
*   **Host Runtime Directory**: 宿主级身份与配置存储于 `ledger/.host/`。
    *   **Content**: `identity.key`、`mcp.json` 等 host-scoped 状态。
*   **Virtual Backup**: 系统 MAY 为当前活跃 Repo 自动创建 `.redb` 文件的只读快照。
    *   **Frequency**: 每日自动 (可配) 或手动触发 (`deve backup`)。
    *   **Storage**: `ledger/backups/<repo_name>-<timestamp>.redb`.
    *   **Retention**: 默认保留最近 3 份；超出按 FIFO 删除。

## Repository Manager (仓库管理器)

*   **职责**: 管理 `Local Repo` (Store B) 和 `Shadow Repos` (Store C)。
*   **Routing**: VFS 根据 UI 上下文路由到对应的 `.redb` 实例。
*   **Snapshot Strategy**:
    *   **Dual-Table Structure**: 为了优化性能，快照存储拆分为两个表：
        *   `SNAPSHOT_INDEX`: 索引表 (`DocId -> [SeqNo]`)，用于快速检索历史版本号。
        *   `SNAPSHOT_DATA`: 数据表 (`SeqNo -> ContentBlob`)，存储实际快照内容。
    *   **Pruning**: 每个 Repo 独立维护自己的 Snapshot 链，并根据配置深度 (`snapshot_depth`) 进行自动修剪。

## Single Authoritative Model (唯一权威模型)

对任意 Repo $r$，系统的权威状态 **MUST** 定义为单一有序账本事实日志：

```text
L_r = OrderedLog<LedgerEntry>
S_r(t) = Fold(L_r[1..t])
P_r = Project(S_r(head(L_r)))
```

其中：

*   `L_r`: Repo `r` 的唯一权威状态。
*   `S_r(t)`: 在 `ledger_seq = t` 处的逻辑状态快照。
*   `P_r`: 从当前 Ledger Head 推导出的规范投影 (Canonical Projection)。

Repo 运行时还允许存在若干 **辅助状态**，但这些状态 **MUST NOT** 升格为权威真相：

*   `pending_fs_ops_r`: Watcher 检测到、尚未进入 Stage/Commit 的工作区偏差。
*   `staging_r`: 用户显式确认、准备进入 Commit 的候选集合。
*   `commit_index_r`: 锚定到特定 `ledger_seq` 的 Commit 视图与历史索引。

因此，工作区磁盘状态应理解为：

```text
Workspace_r = P_r ⊕ D_r
```

其中 `D_r` 表示已被系统跟踪的工作区偏差（由 `pending_fs_ops_r` / `staging_r` 物化），它只影响“当前工作区表现”，不影响“已确认业务事实”。

### Ledger Facts Partition（账本事实分层）

对任意 Repo `r`，`L_r` 中的权威事实 **MUST** 至少分为两层：

*   **Content Facts**：针对 `DocId` 的文本内容变化（例如 Insert / Delete）。
*   **Structure Facts**：针对 `NodeId` / `DocId` 的结构变化（例如 CreateFile、CreateDir、RenameNode、MoveNode、DeleteNode）。

因此，`Path Mapping`、`NodeMeta`、`Tree` 与 `Vault Projection` 的权威来源都不是“直接写表”，而是：

```text
Ledger Facts -> State Fold -> Structure/Content Projection -> Vault / Tree / Path Cache
```

其中：

*   `metadata` / `PATH_TO_DOCID` / `DOCID_TO_PATH` / `NODEID_TO_META` 是 projection storage 或 projection cache。
*   业务层 **MUST NOT** 把 `metadata::rename_doc / set_doc_path / delete_doc` 当作主写路径。
*   这些 API 仅允许被 projection rebuild、repair 或 migration 过程调用。

### Non-Negotiable Invariants (硬不变量)

1. 只有向 `L_r` 追加 `LedgerEntry` 才能改变 Repo 的权威状态。
2. `Snapshot`、`Vault Projection`、`Path Mapping`、`Commit View` 都 **MUST** 由 `L_r` 或锚定到 `L_r` 的辅助表唯一导出。
3. `pending_fs_ops_r` 与 `staging_r` 是 workflow side table，不是第二真值源。
4. 同一 Repo 内所有落账本写入 **MUST** 在追加前被线性化，并由 `GlobalSeq` 决定落盘顺序。
5. 任意时刻若 `Workspace_r != P_r`，系统 **MUST** 能通过 `pending_fs_ops_r` / `staging_r` 解释这种偏差；无法解释则视为状态漂移故障。
6. 路径与树结构相关的业务事实 **MUST NOT** 通过 metadata table 直写完成；必须先形成 Structure Facts，再由 projection 导出。

## Ledger-First Write Paths (账本优先写路径)

### Path A - Editor Direct Write (前端/本地编辑直写)

*   **Scope**: Deve-Note 内置编辑器、受控 CLI 写操作。
*   **Flow**:
    1. 产生写入意图 (`Edit Intent`)。
    2. 校验 user auth、repo binding、writer identity。
    3. 直接将变更编码为 `LedgerEntry` 追加到 `L_r`。
        *   文本编辑 -> `Content Facts`
        *   `Create / Rename / Move / Delete` -> `Structure Facts`
    4. 更新快照或重放尾部 Ledger Facts，得到新的 `P_r`。
    5. 将新的规范投影写回 Vault，并返回确认消息给调用方。
*   **Invariant**: 内置编辑路径 **MUST NOT** 先改 Vault 再“回填” Ledger。
*   **Web Note**: Web Thin Client 的 `pending overlay -> Ack -> confirmed` 确认链详见 [16_web_thin_client_ledger.md](./16_web_thin_client_ledger.md)。

### Path B - Watcher / External Edit Ingestion (外部编辑摄取)

*   **Scope**: VS Code、Vim、Nano、批处理脚本、用户直接修改 Vault 文件。
*   **Flow**:
    1. Watcher 监听 Vault 中的文件系统事件。
    2. 事件经 Debouncer、路径规范化与 Inode / FileID 关联后，写入或更新 `pending_fs_ops_r`。
    3. 系统向 UI 暴露 Working Directory 差异，但不改变已确认业务状态。
*   **Strict Rule**: Watcher 检测到的 `Create / Modify / Delete / Rename` **MUST NOT** 直接生成 Ledger Append。
*   **Delete Rule**: 删除只能先生成 pending delete 候选；最终删除必须体现在显式删除结构事实（如 `DeleteNode` 或等价 tombstone fact）中。
*   **Rename Rule**: 重命名/移动必须先保持 `NodeId` 稳定并记录 pending rename 候选；最终路径变更以 Ledger 中的显式 Structure Facts 为准，而不是 metadata 直写。

### Path C - Stage -> Commit (手动确认入账本)

*   **Stage**:
    *   用户执行 Stage 时，系统将对应条目从 `pending_fs_ops_r` 移入 `staging_r`。
    *   Stage 是 repo-scoped 的真实迁移，不是单纯 UI 隐藏或布尔标记。
*   **Commit**:
    1. 以当前 `ledger_head` 为基准读取 `P_r`。
    2. 将 `staging_r` 中的文件内容与 `P_r` 对比，生成显式 Ledger Facts。
        *   文本变化 -> `Content Facts`
        *   rename / move / create / delete -> `Structure Facts`
    3. 将生成的 Ledger Facts 追加到 `L_r`，分配新的 `GlobalSeq`。
    4. 创建 Commit 记录，并锚定到结果 `ledger_seq`。
    5. 清空本次已消费的 `staging_r` 与 `pending_fs_ops_r` 条目。
    6. 重建或增量更新 `P_r`，再持久化回 Vault。
*   **Discard**:
    *   Discard 的语义是“放弃工作区偏差并恢复到规范投影”，而不是直接篡改 Ledger 历史。

## Projection & Persistence (投影与持久化)

*   **Canonical System-Owned Path**: 所有系统自发写盘路径 **MUST** 满足：

```text
Intent -> Ledger Facts -> Snapshot / Tree Projection -> Vault
```

*   **Projection Authority**: Vault 中由系统写入的内容必须来自 `P_r`，不得直接依据临时 UI 状态或 side table 拼接结果。
*   **Projection Write Rule**: `metadata`、`path mapping`、`tree cache` 与 `NodeMeta` **MUST** 由 projection builder 写入；业务 handler 不得将其当作最终真值直接修改。
*   **Atomic Persistence**:
    *   `SyncManager` / Projection Manager 负责协调 Ledger Append 与投影写入。
    *   当 Ledger Append 成功而投影写入失败时，系统 **MUST** 记录可恢复故障，并支持从 Ledger 重新构建 Vault。
*   **Batch Optimization**:
    *   对批量操作，系统 **SHOULD** 先批量追加 Ledger Facts，再执行一次投影持久化，以减少 I/O 放大。

## Clean File Policy (纯净文件策略)

*   **Implicit Tracking (隐式追踪)**: 系统 **MUST** 使用 `NodeId` (UUID) 作为内部追踪标识。
    *   **Storage Location**: `NodeId <-> Path/Inode` 的映射表 **MUST** 存储在 **Store B (Local Repo)** 的专用 Table/Bucket 中，严禁存储在 Markdown 文件内。
*   **Zero Injection (零注入原则)**: 系统 **MUST NOT** 向用户创建的 Markdown 文件中注入任何元数据（如 YAML Frontmatter 中的 UUID）。
*   **Metadata Source (元数据溯源)**: 即使文件中存在用户手写的 Frontmatter，系统也 **MUST** 视其为普通文本内容 (Payload)。
    *   **No Impact**: 投影中的 Frontmatter 修改 **MUST NOT** 反向影响 Ledger 中的系统元数据（如 Creation Time, UUID 等）。Ledger 的元数据仅由权威 Ledger Facts 变更。

## Inode/DocId Mapping & Watcher Service (映射与监听)

*   **Watcher Service**: 系统核心 **MUST** 运行一个文件系统监听服务，实时监控 Vault 目录。
*   **Mechanism**: `Watcher Event -> Debouncer -> Path Normalize -> Inode/FileID Tracker -> Pending Recorder`.
*   **Event Semantics**:
    *   **Create / Modify**: 监测到 Markdown 文件新增或内容变更 -> 生成 pending create/modify 候选，写入 `pending_fs_ops_r`。
    *   **Delete**: 监测到文件被移除 -> 生成 pending delete 候选，等待用户 Stage / Commit 或 Discard。
    *   **Rename / Move**: 利用 OS 提供的 Inode / FileID 追踪重命名，保持 `NodeId` 不变，并记录 pending rename 候选。
*   **Authoritative Mapping Rule**: `DocId <-> Path` 的权威映射仍位于 Ledger / Store B 中；Watcher 只能帮助识别候选变更，不能直接宣告权威路径已改变。映射表本身视为 Structure Facts 的 projection 结果，而不是可独立写入的第二真值。
*   **External Tools Support**: 必须兼容 VS Code、Vim、Nano 等外部编辑器的原子写入 (Atomic Write) 和重命名行为。
*   **Constraints**:
    *   **Idempotency (幂等性)**: 重复的信号触发 **MUST** 产生相同的候选结果状态。
    *   **Stable Identity**: 只要文件逻辑实体未被用户确认删除，`NodeId` **MUST** 保持稳定。
    *   **Repo Isolation**: Watcher 生成的 side table 数据 **MUST** 严格绑定当前 Repo，严禁跨 Repo 污染。

## Data Integrity & Recovery (数据完整性与灾备)

*   **Append-Only Log**: 所有权威写操作 **MUST** 以日志追加形式记录，**MUST NOT** 执行原地修改 (In-Place Mutation)。
*   **Projection Strategy**: `P_r` 是规范投影；Vault 工作区如有偏差，必须经 `pending_fs_ops_r` / `staging_r` 明确建模。
*   **Recovery Scenarios (恢复场景)**:
    *   **Vault Corruption (误删/篡改)** -> **Rebuild Projection**: 从 `L_r` 与最新快照重放并强制覆盖 Vault。
    *   **Ledger Corruption (损坏)** -> **Reverse Import**: 仅允许通过显式 repair / reset 流程，从 Vault 反向生成新的 Ledger（等价于重置历史）。
    *   **State Deviation (状态错乱)** -> **Reconcile or Reset**: 比较 `P_r` 与工作区偏差集合；若 side table 可解释则继续和解，否则执行硬重建。
*   **Disaster Recovery (灾难恢复)**: 系统 **MUST** 提供将 Ledger 导出为 JSON Lines 格式的能力，确保数据可移植性。
*   **Schema Stability & Migration (架构稳定性与迁移)**:
    *   **Fixed Schema**: 本地数据存储路径与核心 Schema 结构已固定，**SHOULD NOT** 发生变更。
    *   **Manual Migration (手动迁移)**:
        *   若发生不可避免的 Breaking Change，系统 **MUST NOT** 尝试执行复杂的原地 Migration 脚本（风险过高）。
        *   **Strategy**: 采用 `Copy & Rebuild`。用户保留 `vault` 中的 Markdown 原文件与必要的 `.notegit` 元数据，在新版本中重新导入即可。

## Core Interaction Constraint (核心交互约束)

*   **UUID-First Retrieval (UUID 优先检索)**:
    *   **Rule**: 后端对于任意 File / Folder / Repo 的检索与操作，**MUST** 仅通过 `NodeId / RepoUUID` 完成，严禁直接使用 File Path 作为主键。
    *   **Resolution Flow**:
        1.  **Frontend**: 允许传递用户可读的 `Name` 或 `Path`。
        2.  **Resolution**: 后端接收到 Name 后，**MUST** 先查询映射表 (`Name/Path` -> `NodeId`) 获取唯一标识。
        3.  **Execution**: 所有业务逻辑执行 **MUST** 仅针对 UUID 进行。
    *   **Rationale**: 确保在文件 / 目录重命名或移动时，后台任务（如 Embeddings, Sync）不中断，且路径不一致时以 UUID 指向的实体为准。

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
    *   对 `Node` 的重命名、移动、创建、删除必须先形成 Structure Facts，再由 projection 更新 `path` 与 `path_cache`。

## Cross-Platform Path Strategy (跨平台路径策略)

为解决 Windows / Linux / macOS 路径分隔符不一致的问题，系统实施严格的路径规范化策略：

*   **Canonical Internal Format (内部权威格式)**:
    *   **Rule**: 所有存储在 Ledger、KV Database、Memory Cache 中的路径字符串，**MUST** 统一使用 Linux 风格的正斜杠 (`/`) 分隔符。
    *   **Scope**: `DocId <-> Path` 映射表、Ledger Logs、Protocol Messages (Sync / Gossip)。
    *   **Example**: `folder/subfolder/file.md` (Valid), `folder\subfolder\file.md` (Invalid).
*   **Normalization Boundary (规范化边界)**:
    *   **Ingestion (输入)**: 当从 OS 文件系统读取路径时（Watcher events, File Dialogs），**MUST** 立即调用规范化函数 (`to_forward_slash`) 转换为内部格式。
    *   **Interaction (输出)**: 仅在直接调用 OS 文件系统 API (`std::fs`, `open_file`) 的瞬间，**SHOULD** 调用转换函数 (`to_native`) 还原为系统原生格式。
*   **Implementation**:
    *   核心库提供 `crates/core/src/utils/path.rs` 标准模块，包含 `to_forward_slash` 和 `to_native` 方法，所有路径操作 **MUST** 通过此模块进行，严禁手动通过字符串替换 (`replace`) 处理。

## Browser Storage Layering (浏览器存储分层)

WebLightPeer 的浏览器侧存储必须按安全等级与恢复语义分层，禁止再以单一 `localStorage` 承担身份、会话与缓存职责。

*   **UI prefs (`localStorage`)**:
    *   **Allowed**: 主题、侧栏宽度、语言、最近展开面板等纯前端偏好。
    *   **Forbidden**: `peer identity`、`session token`、repo vector、离线缓存内容。
    *   **Recovery**: 用户清理浏览器站点数据后可直接重建，不影响 Server Ledger 真值。
*   **User Session (`JWT Cookie`)**:
    *   **Primitive**: `HttpOnly + Secure + SameSite=Strict` Cookie。
    *   **Allowed**: 用户访问权限、登录态续存、服务端 API / WebSocket 握手鉴权。
    *   **Forbidden**: 作为 `peer identity` 使用；不得承载 repo-scoped vector 或离线缓存。
    *   **Recovery**: Cookie 过期、登出或密码修改后，客户端必须重新登录获取新的 session token。
*   **Peer Identity (`WebCrypto + IndexedDB`)**:
    *   **Primitive**: 浏览器首次进入某个 `repo_id` 时，调用 `WebCrypto.subtle.generateKey(...)` 生成 repo-scoped Ed25519 keypair，私钥 **MUST** 设为 `extractable: false`。
    *   **Allowed**: `peer identity` 公钥、key handle、注册状态、最后一次握手元数据持久化在 IndexedDB `peer_identity` store。
    *   **Forbidden**: 私钥原始字节、seed 或可导出密钥材料写入 `localStorage`、URL、Cookie、日志。
    *   **Recovery**: 若 IndexedDB 中仍存在对应 `CryptoKey` / metadata，则恢复原 browser peer；若 identity 丢失，则必须重新生成 keypair 并重新执行 trust registration。
*   **Offline Cache (`IndexedDB`)**:
    *   **Allowed**: repo-scoped vector clock、最近访问文档摘要、只读缓存、同步检查点与 UI 预热数据。
    *   **Forbidden**: 认证 cookie、私钥材料、跨 Repo 共用缓存桶。
    *   **Recovery**: 配额清理或用户手动删除后可从 Server 增量重建；缓存缺失不得改变权威账本。

### Trust Registration Flow (信任注册流程)

1. 浏览器已具备有效 user session，但首次打开某个 Repo 时仍视为“未注册 peer identity”。
2. WebLightPeer 查询 IndexedDB 的 `peer_identity` store；若不存在该 `repo_id` 记录，则调用 `WebCrypto` 生成新的 repo-scoped keypair。
3. 私钥保持为不可导出的 `CryptoKey`；公钥、`peer_id`、注册时间与握手状态写入 IndexedDB。
4. 浏览器发送 `SyncHello { repo_id, peer_pubkey, vector }` 到 Server，请求将该 browser peer 纳入 Repo trust set。
5. Server 以 session token 验证“谁在访问”，再以 `peer identity` 公钥记录“哪个节点在同步”，两者缺一不可但职责不同。
6. 注册成功后，后续 `SyncRequest` / `SyncPush` 仅使用该 repo-scoped peer identity 参与签名与验证。

### Recovery Semantics (恢复语义)

*   **IndexedDB + WebCrypto 可用**: 进入正常 WebLightPeer 模式，允许 repo-scoped 拉取与受限推送。
*   **仅 Cookie 可用，IndexedDB 不可用**: 进入 `DegradedSyncMode`。用户仍可保留登录态，但因无法持久化 peer identity 与 offline cache，UI **MUST** 切换为只读并禁止 `SyncPush`。
*   **Cookie 失效但 IndexedDB identity 仍在**: 必须重新登录；既有 peer identity 不自动授予 API 访问权限。
*   **用户清除站点数据**: `UI prefs`、peer identity、offline cache 全部丢失；下次访问必须重新登录并重新执行 browser peer 注册。

## 本章相关命令

*   无。

## 本章相关配置

*   `vault.path`: Store A 的根目录路径 (Default: `/data/vault`).
*   `ledger.path`: Store B/C 的根目录路径 (Default: `/data/ledger`).
