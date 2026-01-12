# 第四章：数据完整性与灾备 (Integrity & Recovery)

*   **Append-only**：所有写操作追加账本。
*   **投影策略**：Markdown 投影非真源，从 Ledger 渲染。
*   **恢复场景**：Markdown 误删 -> 重建；Ledger 损坏 -> 反向导入；客户端错乱 -> Hard Reset。
*   **Portable Ledger Export (灾难恢复)**: 导出为 JSON Lines 格式。

# 第五章：数据流与交互 (Interaction Flows)

### 场景一：外部编辑器协同 (The "Alt-Tab" Flow)

1.  VS Code 修改 -> 保存。
2.  **后端**: `Notify` -> Debounce -> Inode Check -> Diff -> Ops -> Redb。
3.  **推送**: WebSocket Push Ops。
4.  **前端**: 平滑更新，Toast 提示。

### 场景二：数学公式编写 (The Math Flow)

1.  输入 `$$` -> 切换公式块。
2.  输入 LaTeX -> 实时预览。
3.  `Ctrl+Enter` -> 折叠显示 SVG。

### 场景三：Git 同步 (The Git Flow)

1.  **触发方式**:
    *   **Activity Bar**: 点击 Source Control 图标 -> Commit/Sync。
    *   **Command Palette**: `Cmd+K` -> 输入 `Git: Sync` / `Git: Commit` / `Git: Push`。
2.  **前端**: 显示差异 (Diff View)。
3.  **权限检查**: 校验 Capability。
4.  **调用**: `git_sync.rhai` (Host Functions)。
5.  **后端**: `git add` -> `commit` -> `push`。
6.  **反馈**: 状态栏变绿。

### 场景四：P2P 分支切换与合并 (The P2P Flow)

1.  **触发方式**:
    *   **Activity Bar**: 点击 Source Control 图标 -> 展开 Repositories Section。
    *   **Command Palette**: `Cmd+K` -> 输入 `P2P: Switch to Peer` / `P2P: Merge Peer`。
2.  **选择 Peer**: 在 Repositories 列表中选择 `Peer-iPhone (Shadow)`。
3.  **UI**: 进入 **Spectator Mode**；VFS 切换至 Shadow Repo。
    *   编辑器背景变灰 + "READ ONLY" 状态栏提示。
4.  **浏览**: 查看 Peer 文件内容，可右键 "Copy to Local"。
5.  **合并**: 在 Source Control View 中点击 "Merge Peer-iPhone into Local"。
    *   **Auto Mode**: 自动执行 CRDT Merge。
    *   **Manual Mode**: 显示 Diff View (类似 VS Code 合并编辑器)。
        *   有冲突 -> 用户选择保留哪个版本。
        *   无冲突 -> 直接合并。
    *   **结果**: Store B 更新 -> Store A 同步更新。
6.  **UI**: 合并完成后自动切回 `Local (Master)` 分支。
