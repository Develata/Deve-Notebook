# 06_repository.md - 仓库与分支篇 (Repository & Branching)

## 仓库管理 (Repository Manager)

*   **Repository Identification (仓库标识)**:
    *   **Basis**: 基于 **Characteristic Parameter** (默认为 URL，可配置) 唯一标识。
    *   **Logical Identity**: URL `https://my-wiki` 是区分 Repo 的唯一标准。
        *   **Conflict Rule**: 若 `Name` 相同但 `URL` 不同，视为**完全不同的 Repo**。
    *   **Physical Storage (Repo Instance)**:
        *   **Filename**: `branch_path/<repo_name>.redb` (Human Readable).
        *   **Collision Handling**: 若同一 Branch 下存在同名文件但 URL 不同，系统 **MUST** 自动重命名新文件 (e.g., `wiki.redb` -> `wiki-1.redb`) 以避免覆盖。
    *   **Retrieval Constraint (Resolution First)**:
        *   **Input Acceptance**: 后端接口 **MAY** 接受 `Url` `Name` 或 `UUID` 作为输入参数。
        *   **Execution Safety**: 但在执行具体业务逻辑前 (Before Execution)，系统 **MUST** 将非 UUID 参数解析为 `InstanceUUID`。
        *   **Rule**: 任何文件读写、合并、同步操作的底层算子，**MUST** 操作 `UUID`。名字解析必须在算子调用前完成。
        *   **Note**: 严禁直接复用 UUID，必须保证每个物理 `.redb` 文件拥有全局唯一的 FileId。
*   **P2P Connection Strategy (连接策略)**:
    *   **Match**: URL 相同 -> 同一仓库协作 (显示 Shadow Branches)。
    *   **Mismatch**: URL 不同 -> **Multi-Root Workspace** (侧边栏分列显示 Local Repos + Peer Repos)。
    *   **Access Control**: Peer-only Repos (URL 不匹配) 强制为 **Read-Only** (仅允许 Copy/Diff)。
    *   **Retrieval Constraint**:
        *   **UUID-First**: 所有 Repo 和 Branch 的后端操作 **MUST** 使用 UUID 检索。
        *   **Name Resolution**: 前端可传递 `RepoName` 或 `PeerName`，但后端必须先解析为 `RepoUUID` 或 `PeerUUID` 再执行文件系统操作。
    *   **Repo Instance Selection**: 当用户切换 Branch (Peer Identity) 时，前端展示该 Branch 下所有可用的 **Repo Instances**。

## 树状态管理器 (Tree State Manager)

*   **Implementation**: `crates/core/src/tree/manager.rs`
*   **Data Structure**:
    *   **Core**: `HashMap<String, NodeInfo>` (Flat Map with Path Keys).
    *   **NodeInfo**:
        ```rust
        struct NodeInfo {
            name: String,
            doc_id: Option<DocId>, // None for pure folders
            parent_path: String,
            children_paths: Vec<String>,
        }
        ```
    *   **Advantages**: 扁平化存储使得 `Rename` 操作需递归更新子节点路径，但在 `Lookup` 时达到 O(1) 效率。
*   **TreeDelta Generation**:
    *   **Add**: `TreeDelta::add_file` / `add_folder`.
    *   **Remove**: `TreeDelta::remove`. 自动递归删除子节点。
    *   **Rename**: `TreeDelta::rename`. 自动处理子树路径重写。
*   **Sorting Logic**:
    *   构建树视图 (`build_tree_from_root`) 时，严格遵循：**Folder First** > **Alphabetical (Case-Insensitive)**。
*   **Initialization**: 服务启动时，通过 `RepoManager::list_docs` 遍历 `docid_to_path` 表进行全量加载。

## 严格分支策略 (Strict Branching Policy)

*   **Logic**: "Branch" 对应 "Writer Identity"，即 Peer 的数据集合 (Folder)。
*   **Establishment**:
    *   **Local Branch**: 初始化时自动创建 `ledger/local/` 文件夹。
    *   **Remote Branch**: 通过 P2P 发现新 Peer 后，在 `ledger/remotes/` 下自动创建对应 UUID 的文件夹。
*   **Repo Mapping**: 每个 `.redb` 文件是 Branch 文件夹下的一个逻辑单元。
*   **No Arbitrary Creation**: ❌ 禁止类似 `git checkout -b feature` 的操作。分支由 Peer 身份唯一确定。
*   **Deletion**: 允许删除某个 Remote Branch 文件夹（即移除该 Peer 的所有数据）。但不建议删除 Local Branch (除非重置应用)。

## 分支切换与交互 (Branch Switching)

* **分支切换器**：UI 提供类似 VS Code 左下角的分支切换功能。
    *   **Local Branch**: Default View. 读写 `ledger/local/*.redb`。
    *   **Remote Branches (Peer Instances)**:
        *   **Display Logic**: 侧边栏显示已发现的 Peers (对应 `ledger/remotes/<PeerUUID>/` 文件夹)。
        *   **Spectator Mode**: 选中某 Peer 后，VFS 切换挂载点至该 Peer 的对应目录。
        *   **Content**: 显示该 Peer 下拥有的所有 Repos (redb files)。
    * **Virtual Backup Branch**:
        *   提供一个虚拟远程分支（只读），作为当前版本数据库文件的实时/快照备份。

## 本章相关命令

*   `P2P: Switch to Peer`: 切换到指定 Peer 的影子分支 (进入 Spectator Mode)。
*   `P2P: Establish Branch`: 从当前查看的 Peer 分支创建本地分支。

## 本章相关配置

* 无。
