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

## 和解策略 (Reconciliation Strategy)

*   **Store C -> Store B (Remote Merge)**：
    *   **Auto Mode (CRDT)**: 利用 Loro 的 Op-based Merge 自动解决非冲突变更。
    *   **Manual Mode (Git-style)**: 若检测到同一文本块 (Hunk) 存在竞争性修改，标记为 **Conflict**，必须人工介入。
    *   **Atomic Persistence (原子持久化)**:
        *   **Immediate Commit**: 合并过程本质上是后端生成一系列 Ops 并顺序追加到 Local Ledger 的过程。系统 **MUST** 保证每生成一个 Op 即持久化（模拟写入），**不会** 存在“内存中合并了一半未保存”的中间状态。
        *   **Interruption Handling (中断处理)**: 若合并过程中发生异常（如浏览器关闭、网络断开），已持久化的 Ops 永久生效，未处理的 Diff 保持未合并状态。
        *   **Resumption (自然续传)**: 系统重启后，Local Ledger 的 Vector Clock 已推进。再次发起合并时，系统将基于新的 Base 重新计算剩余 Diff，自然完成续传，无需特殊的回滚或恢复逻辑。
*   **Store A -> Store B (Local Watcher)**：
    *   **Mechanism**: Watcher 监测到文件修改 -> 计算 $Diff(Content_{disk}, Content_{ledger})$ -> 生成 Ops 追加到 Ledger。
    *   **Anti-Thrashing**: 必须实现 300ms+ 防抖 (Debounce) 和 Hash 校验，防止循环触发。

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

## 本章相关命令

*   `P2P: Merge Peer`: 将当前 Spectator Mode 查看的 Peer 分支合并入本地。

## 本章相关配置

*   `diff.merge_strategy`: `manual` (Default, 推荐) | `auto` (CRDT优先)。
    *   **manual**: 总是弹出 Diff View 供用户确认，除非差异微小且确信无冲突。
    *   **auto**: 仅在检测到 Structural Conflict 时才弹出，其余自动通过。
