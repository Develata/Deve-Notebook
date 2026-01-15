# 08_ui_design.md - 使用界面设计篇 (UI Design)

## 1. Desktop UI

### The "Cockpit" Concept (驾驶舱概念)
* **信息分层**：
	* **L1 (Focus)**：编辑区是绝对中心，无干扰。
	* **L2 (Context)**：左侧边栏（文件树），右侧边栏（大纲）提供导航。
	* **L3 (Meta)**：底部状态栏显示“和解状态”、Git 分支、字数统计。
	* **L4 (Floating)**：`Cmd+K` 命令面板和悬浮工具栏，按需出现。
* **键盘优先 (Keyboard First)**：所有 UI 操作必须有快捷键；模仿 Vim/VS Code/Nano 逻辑。

### Workbench Layout (工作台布局)
应用必须严格遵循 **VS Code Workbench** 标准网格布局：
*   **Activity Bar**: 最左侧固定宽度列。图标垂直排列，点击切换侧边栏内容。
*   **Primary Side Bar**: 紧邻 Activity Bar，显示 Explorer / Search / Source Control 等视图。
*   **Editor Group**: 占据中心区域，支持多标签/分屏。
*   **Status Bar**: 底部信息条。
*   **Panel**: 底部可折叠区域 (日志/终端)。

### Detailed View Specifications (视图详情)
*   **Source Control View**:
    *   **Structure**: 顶部多行 Input Box (Commit Message) -> Repositories List -> Staged/Changes Groups -> Commit Button。
    *   **History**: 包含 Time Travel Slider (热力图/历史回放)。
*   **Status Bar**:
    *   **Mandatory Items**: Remote Indicator, **Branch Name**, Sync Status (Left); Language, Cursor (Right).
    *   **Colors**: Default (Blue/Purple), **Spectator** (Orange + Watermark).
*   **Auxiliary Slots (Secondary Side Bar)**:
    *   **AI Chat**: 默认右侧，Copilot 风格。
    *   **Outline**: 目录大纲。

### Spectator Mode Visuals (旁观者视觉)
*   **Watermark**: 编辑器背景增加**灰色/斜纹水印**。
*   **Status**: 状态栏显示橙色 "**READ ONLY**"。

## 2. Web UI

### Server Dashboard (服务器面板)
* **定位**：Web 端作为 Server 节点的远程操作面板 (Remote Dashboard)。
* **限制**：
    *   **RAM-Only**: 严禁使用 IndexedDB 持久化数据。
    *   **断连锁屏**: 检测到 WebSocket 心跳丢失时，界面 **MUST** 立即进入锁定/只读状态，提示“连接断开”，严禁离线编辑。
    *   **乐观 UI**: 仅在连接存活时有效。
    *   **External Edit Flow (外部协同)**: 
        1.  VS Code 修改 -> 保存。
        2.  后端检测 -> 推送 Ops。
        3.  前端平滑更新 -> 弹出非侵入式 Toast 提示：“已合并外部修改”。

## 3. Mobile UI

### 移动端适配
*   **架构**：基于 Tauri Mobile / WebView 封装。
*   **存储**：作为完整 P2P 节点 (Full Peer)，推荐使用 Redb 或 SQLite，不依赖 IndexedDB。
*   **适配策略**：针对触摸屏优化交互；在小屏幕下侧边栏默认折叠。

## 4. UI 状态与样式指引 (Styling Guidelines)
*   **Colors**: 必须使用 CSS Variables 定义语义化颜色 (`--activity-bar-bg`, `--editor-bg`, etc.)，禁止硬编码 Hex。
*   **Typography**:
    *   UI 字体: System Sans-Serif (San Francisco, Segoe UI).
    *   Editor 字体: Monospace (JetBrains Mono, Fira Code).
*   **Focus Ring**: 所有可交互元素在键盘 Focus 时必须显示高对比度轮廓 (`outline: 2px solid var(--focus-border)`).

## 5. 体验取舍 (Inspiration & Compatibility)
*   **风格参考**：UI 视觉与版式借鉴语雀与 SilverBullet 的清爽阅读感，但保持开源可自定义主题。
*   **导航体验**：侧边栏结构参考 VitePress（分组/层级清晰），命令/快捷键呼出文件列表参考 SilverBullet 弹出式搜索。

## 6. 组件组织与扩展规范 (Component Organization)

*   **File Structure (文件结构)**:
    *   `apps/web/src/components/command_palette/`: **Command Palette** 专用目录。
    *   `apps/web/src/components/branch_switcher/`: **Branch Switcher** 专用目录。
    *   `apps/web/src/components/quick_open/`: **Open Document** 专用目录。
    *   `apps/web/src/components/search_box/`: **Core Unified Search Box** 基础组件目录。
*   **Modularization Rule (模块化原则)**:
    *   **One Component, One File/Folder**: 每个组件代码若较短则对应单文件；若代码过长或包含子组件，**MUST** 封装为独立文件夹。
    *   **Plugin Interface**: 下拉/搜索组件 **MUST** 预留插件接口 (Traits/Hooks)。

## 本章相关命令

*   `Cmd+K` / `Ctrl+K`: 呼出 Command Palette。
*   `Cmd+P` / `Ctrl+P`: 呼出 Quick Open (文件跳转)。

## 本章相关配置

*   `ui.recent_commands_count`: Command Palette 显示的最近命令数量 (Default: 3).
*   `ui.recent_docs_count`: Quick Open 显示的最近文件数量 (Default: 10).
