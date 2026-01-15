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
    *   **Indexing**: 系统维护 `RepoURL -> List<InstanceUUID>` 索引。前端使用 Name -> 解析为 UUID -> 执行操作。
*   **Virtual Backup**: 针对每个 Repo Instance (`.redb`) 可存在对应的只读快照。

## Repository Manager (仓库管理器)

* **职责**：管理 `Local Repo` (Store B) 和 `Shadow Repos` (Store C)。
* **Routing**：VFS 根据 UI 上下文路由到对应的 `.redb` 实例。
* **Snapshot**：每个 Repo 独立维护自己的 Snapshot 链。

## Clean File Policy (纯净文件策略)

*   **Implicit Tracking (隐式追踪)**: 系统 **MUST** 使用 `DocId` (UUID) 作为内部追踪标识。
    *   **Storage Location**: `DocId <-> Inode/Path` 的映射表 **MUST** 存储在 **Store B (Local Repo)** 的专用 Table/Bucket 中，严禁存储在 Markdown 文件内。
*   **Zero Injection (零注入原则)**: 系统 **MUST NOT** 向用户创建的 Markdown 文件中注入任何元数据（如 YAML Frontmatter 中的 UUID）。
*   **Metadata Source (元数据溯源)**: 即使文件中存在用户手写的 Frontmatter，系统也 **MUST** 视其为普通文本内容 (Payload)。
    *   **No Impact**: 投影中的 Frontmatter 修改 **MUST NOT** 反向影响 Ledger 中的系统元数据（如 Creation Time, UUID等）。Ledger 的元数据仅由 Authoritative Ops 变更。

## Inode/DocId Mapping & Watcher Service (映射与监听)

*   **Store A -> Store B (Ingestion Flow)**:
    *   **Watcher Service**: 系统核心 **MUST** 运行一个文件系统监听服务 (Watcher)，实时监控 Vault 目录。
        *   **Create / Modify**: 监测到 Markdown 文件的新增或内容变更 -> 触发 Ledger **写入/更新**操作 (Append Ops)。
        *   **Delete**: 监测到 Markdown 文件被移除 -> 触发 Ledger **标记删除**操作 (Mark Deleted)。
        *   **Rename / Move**: 监测到重命名或移动 -> 更新 **Path Mapping**，保持 `DocId` 不变。
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

## Core Interaction Constraint (核心交互约束)

*   **UUID-First Retrieval (UUID 优先检索)**:
    *   **Rule**: 后端对于任意 File/Folder/Repo 的检索与操作，**MUST** 仅通过 UUID 完成，严禁直接使用 File Path 作为主键。
    *   **Resolution Flow**:
        1.  **Frontend**: 允许传递用户可读的 `Name` 或 `Path`。
        2.  **Resolution**: 后端接收到 Name 后，**MUST** 先查询映射表 (`Name` -> `UUID`) 获取唯一标识。
        3.  **Execution**: 所有的业务逻辑执行 (Execution) **MUST** 仅针对 UUID 进行。
    *   **Rationale**: 确保在文件重命名或移动（Path 变更）时，正在进行的后台任务（如 Embeddings, Sync）不中断，且路径不一致时以 UUID 指向的实体为准。

## 本章相关命令

* 无。

## 本章相关配置

*   `vault.path`: Store A 的根目录路径 (Default: `/data/vault`).
*   `ledger.path`: Store B/C 的根目录路径 (Default: `/data/ledger`).
