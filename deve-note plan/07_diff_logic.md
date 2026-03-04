# 07_diff_logic.md - "Git" Diff 篇 (Diff Logic)

> [!IMPORTANT]
> **Scope Constraint (作用域约束)**: 本章所述的 Diff 与 Merge 逻辑 **仅适用于同一逻辑仓库 (Same Logical Repo) 下的不同分支**。
> *   **Identity Check**: 系统判定两个分支是否属于同一 Repo 的唯一标准是 **RepoUUID** (或 Logical URL Hash)，**绝非** 文件名 (RepoName)。
> *   **Strict Prohibition**: 系统 **严禁** 跨仓库 (Cross-Repo) 的自动化合并 (e.g., `wiki.redb` merge into `blog.redb` where UUIDs differ is undefined behavior)。

## 核心算法 (Core Algorithms)

*   **Text Diff**: 采用 **Myers Algorithm** (implemented via `similar` crate).
    *   **Index Standard**: 全链路统一为 **UTF-16 code unit** 索引（与 JS/CodeMirror 一致）。
    *   **Atomicity**: `Op::Insert` 和 `Op::Delete` 均基于 UTF-16 位置而非字节位置 (Byte Pos)。
*   **Structural Merge**: 采用 **3-Way Merge** 策略。
    *   **Base**: 两个分支的最近共同祖先 (LCA - Lowest Common Ancestor)。
    *   **Left**: 本地当前状态 (Local Branch)。
    *   **Right**: 远端传入状态 (Remote Branch)。

## Two Diff Domains (两层 Diff 域)

*   **Domain 2 — Working Directory（工作区域）**：
    *   Watcher 监控 Vault (Store A) 的 markdown 文件变化。
    *   检测到变更后，**MUST** 写入 `pending_fs_ops` 表（存储于 `.notegit/pending`），**MUST NOT** 直接生成 Ops 入 Ledger。
    *   通过 WebSocket `FsChangeDetected` 消息实时通知前端，前端显示在 "Changes" 列表。
    *   用户可执行 Stage → 变更进入 "Staged Changes"（写入 `.notegit/staged` 表）。

*   **Domain 1 — Staging & Commit（暂存与提交）**：
    *   用户点击 Commit 后，系统：
        1. 将 Staged 文件与 Ledger 最新快照对比，生成 Ops（Insert/Delete）。
        2. 将 Ops **追加到 Ledger**（唯一真值源），分配 `GlobalSeq`。
        3. 创建 Commit 记录，锚定到当前 `ledger_seq`，形成版本历史。
        4. 清空 Staging 区和 `pending_fs_ops` 中已处理的条目。
    *   此时变更从 Domain 2 正式转入 Domain 1（Committed）。

*   **手动确认原则（Git-like Workflow）**：
    *   **Watcher Invariant**: Watcher 检测到的后台 Vault 变更 **MUST NOT** 自动入 Ledger；必须等待用户 Stage → Commit。
    *   **Frontend Exception**: 前端编辑器生成的变更 **MAY** 直接入 Ledger（绕过 Working Directory），或遵循相同的 Stage → Commit 流程（用户可配置）。
    *   此设计严格类比 Git 的三阶段：Working Directory (`pending_fs_ops`) → Staging Area (`.notegit/staged`) → Commit (Ledger + `.notegit/commits`)。

*   **Conflict Detection**: 
    *   若 `pending_fs_ops` 与 Ledger 已存在变更冲突（如同一位置被前端和后台同时修改），系统 **MUST** 提示用户选择 "Keep File System" 或 "Keep Ledger"。


## 合并流程 (Merging Flow)

### 1. The 3-Way Merge Process
当用户执行 "Merge Peer-B into Local" 时：
1.  **LCA Calculation**: 系统根据 Vector Clock 回溯找到 Base Snapshot。
2.  **Diff Generation**:
    *   $Diff_{local} = Base \to Local$
    *   $Diff_{remote} = Base \to Remote$
3.  **Conflict Detection**:
    *   若 $Diff_{local}$ 和 $Diff_{remote}$ 修改了不重叠的区域 -> **Auto Merge**。
    *   若修改了同一区域 -> **Conflict State** -> 暂停并弹出 UI。

### 2. Conflict Resolution UI (冲突解决界面)
*   **Layout**: **Side-by-Side** (Visual Studio Code 风格)。
    *   **Left Pane**: Current (Local).
    *   **Right Pane**: Incoming (Remote).
    *   **Bottom Pane**: Result (Preview).
*   **Actions**:
    *   `Accept Current` (保留本地)。
    *   `Accept Incoming` (采用远端)。
    *   `Accept Both` (同时保留，上下排列)。
*   **Scrubbing**: 支持逐行/逐块 (Hunk) 处理。

## 差异可视化 (Diff Visualization)
*   前端需提供 **Diff View**，用于展示 Local 与 Peer 之间的变更，支持 Side-by-Side 对比。
*   **Gutter Indicators**: 编辑器左侧槽显示变更状态 (相对于 Base)。
    *   **Green**: Added.
    *   **Red**: Deleted (Triangles).
    *   **Blue**: Modified.
*   **Inline Diff**: 编辑时即时计算与已提交状态的差异。

## 长文档打开策略 (Large Doc Open Strategy)

> 目标：首屏 < 200ms 可见，完整可编辑时间最短化。

*   **Snapshot-First**: 打开文档时优先读取最新快照，再仅重放快照之后的 Ops。
*   **UTF-16 Index Cache**: 为 UTF-16 索引引入断点缓存，降低定位成本。
*   **Progressive Prefetch**: 先渲染首屏 + 缓冲区，其余内容后台分批预加载。
*   **Search Gate**: 见 [03_rendering.md §大文档渲染策略](./03_rendering.md)。

## 本章相关命令

*   `P2P: Merge Peer`: 将当前 Spectator Mode 查看的 Peer 分支合并入本地。

## 本章相关配置

*   `diff.merge_strategy`: `manual` (Default, 推荐) | `auto` (CRDT优先)。
    *   **manual**: 总是弹出 Diff View 供用户确认，除非差异微小且确信无冲突。
    *   **auto**: 仅在检测到 Structural Conflict 时才弹出，其余自动通过。
