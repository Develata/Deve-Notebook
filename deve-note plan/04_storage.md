# 04_storage.md - 数据存储篇 (Data Storage)

## 三库隔离 (The Trinity Isolation)

实现“绝对数据主权”和“零污染”。本地不再是单体数据库，而是物理隔离的存储结构：

* **Store A (Vault)**: 本地 Markdown 文件（工作区）。由 Store B 实时投影生成，用户可读写。
* **Store B (Local Master DB)**: 本地核心数据库（`local.redb`）。**只有本地用户的操作能写入此库**。
* **Store C (Remote Shadow DBs)**: 远端影子数据库集合（`/data/ledger/remotes/peer_X.redb`）。
    * 来自 Peer X 的数据**只写入** `remotes/peer_X.redb`，**绝对禁止**自动合并进 Store B。

## Repository Manager (仓库管理器)

* **职责**：管理 `Local Repo` (Store B) 和 `Shadow Repos` (Store C)。
* **Routing**：VFS 根据 UI 上下文路由到对应的 `.redb` 实例。
* **Snapshot**：每个 Repo 独立维护自己的 Snapshot 链。

## Clean File Policy (干净文件策略)

* **Implicit Tracking (隐式追踪)**: `DocId` (UUID) 仅存储在内部数据库 (`Ledger/RepoManager`) 中，以文件路径或 Inode 关联。
* **No Injection (零注入)**: 系统**严禁**自动向 Markdown 文件头注入 `uuid` 或其他 Frontmatter 元数据。Markdown 文件必须保持用户原始书写的状态。
* **Deceptive Metadata**: 如果用户手动书写 Frontmatter，仅用于渲染展示或兼容性，不是系统追踪的真理源 (Source of Truth)。

## Inode/DocId Mapping (映射逻辑)

* **Store A -> Store B (Local Watcher)**：
    *   Debounce -> Inode Check -> Append Ops。
* **约束**：MUST 幂等；MUST 识别重命名 (Inode/UUID)；MUST 防抖。

## 数据完整性与灾备 (Integrity & Recovery)

*   **Append-only**：所有写操作追加账本。
*   **投影策略**：Markdown 投影非真源，从 Ledger 渲染。
*   **恢复场景**：
    *   **Markdown 误删** -> 重建 (Rebuild from Ledger)。
    *   **Ledger 损坏** -> 反向导入 (Reverse Import from Vault)。
    *   **客户端错乱** -> Hard Reset (Nuke Store B & Rebuild)。
*   **Portable Ledger Export (灾难恢复)**: 导出为 JSON Lines 格式，确保在无应用情况下数据可读/可迁移。

## 本章相关命令

* 无。

## 本章相关配置

*   `vault.path`: Store A 的根目录路径 (Default: `/data/vault`).
*   `ledger.path`: Store B/C 的根目录路径 (Default: `/data/ledger`).
