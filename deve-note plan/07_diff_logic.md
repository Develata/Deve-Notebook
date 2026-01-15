# 07_diff_logic.md - "Git" Diff 篇 (Diff Logic)

本章阐述系统核心的差异比对、和解与合并逻辑。注意区分 **Internal Diff**（P2P 核心逻辑） 与 **Git CLI**（外部插件逻辑）。

## 和解策略 (Reconciliation Strategy)

* **Store C -> Store B (Remote Merge)**：
    *   **Conflict Handling**：Manual Mode 下检测冲突 MUST 报错并强制手动解决。
* **Store A -> Store B (Local Watcher)**：
    *   Debounce -> Inode Check -> Append Ops。
    *   **Patch Semantics**：外部编辑器对 Vault 的修改被视为一次 **"Patch"**（补丁）。核心 **MUST** 将其通过 Diff 算法转化为 Ops 合并入 Ledger，而 **MUST NOT** 简单地以文件内容覆盖账本。

## 合并流程 (Merging Flow)

### P2P 分支合并 (The P2P Flow)
1.  **触发方式**: 在 Source Control View 中点击 "Merge Peer-iPhone into Local"，或使用命令。
2.  **模式行为**:
    *   **Auto Mode**: 自动执行 CRDT Merge。
    *   **Manual Mode**: 显示 Diff View (类似 VS Code 合并编辑器)。
        *   有冲突 -> 用户选择保留哪个版本。
        *   无冲突 -> 直接合并。
3.  **结果**: Store B 更新 -> Store A 同步更新。

## 差异可视化
*   前端需提供 **Diff View**，用于展示 Local 与 Peer 之间的变更，支持 Side-by-Side 对比。

## 本章相关命令

*   `P2P: Merge Peer`: 将当前 Spectator Mode 查看的 Peer 分支合并入本地。

## 本章相关配置

*   无。
