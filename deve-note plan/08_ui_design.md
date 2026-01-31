# 08_ui_design.md - 使用界面设计篇 (UI Design)

本章主要描述 Deve-Note 的用户界面设计规范。为适应不同端的需求，我们将 UI 设计拆分为以下部分：

*   **[Web 端设计 (Web UI)](./08_ui_design_01_web.md)**
*   **[桌面端设计 (Desktop UI)](./08_ui_design_02_desktop.md)**
*   **[移动端设计 (Mobile UI)](./08_ui_design_03_mobile.md)**

以下内容为各端共通的设计原则、样式指引与组件规范。

## 1. UI 状态与样式指引 (Styling Guidelines)
*   **Colors**: 必须使用 CSS Variables 定义语义化颜色 (`--activity-bar-bg`, `--editor-bg`, etc.)，禁止硬编码 Hex。
*   **Typography**:
    *   UI 字体: System Sans-Serif (San Francisco, Segoe UI).
    *   Editor 字体: Monospace (JetBrains Mono, Fira Code).
*   **Focus Ring**: 所有可交互元素在键盘 Focus 时必须显示高对比度轮廓 (`outline: 2px solid var(--focus-border)`).

## 2. Command First (命令优先交互)

*   **原则**：功能入口以命令面板为主，UI 按钮为辅。
*   **目标**：降低界面密度，保持 VSCode/opencode 风格的极简感。
*   **状态栏**：保留远端/分支/同步与基础统计；AI 模式仅在 AI Chat 面板头部显示。

## 3. 体验取舍 (Inspiration & Compatibility)
*   **风格参考**：UI 视觉与版式借鉴语雀与 SilverBullet 的清爽阅读感，但保持开源可自定义主题。
*   **导航体验**：侧边栏结构参考 VitePress（分组/层级清晰），命令/快捷键呼出文件列表参考 SilverBullet 弹出式搜索。

## 4. 组件组织与扩展规范 (Component Organization)

*   **File Structure (文件结构)**:
    *   `apps/web/src/components/`
        *   `activity_bar/`: 活动栏组件 (Activity Bar items)。
        *   `sidebar/`: 文件资源管理器与操作 (File explorer & actions)。
        *   `editor/`: CodeMirror 编辑器封装 (CodeMirror wrapper)。
        *   `diff_view/`: 并排差异对比视图 (Side-by-side diff)。
        *   `search_box/`: 统一搜索框 (命令/文件/分支)。
        *   `settings/`: 设置模态框 (Settings modal)。
        *   `bottom_bar/`: 底部状态栏指示器 (Status indicators)。
*   **Modularization Rule (模块化原则)**:
    *   **One Component, One File/Folder**: 每个组件代码若较短则对应单文件；若代码过长或包含子组件，**MUST** 封装为独立文件夹。
    *   **Plugin Interface**: 下拉/搜索组件 **MUST** 预留插件接口 (Traits/Hooks)。

## 本章相关命令

*   `Cmd+Shift+P` / `Ctrl+Shift+P`: 呼出 Command Palette。
*   `Cmd+P` / `Ctrl+P`: 呼出 Quick Open (文件跳转)。
*   `Cmd+Shift+K` / `Ctrl+Shift+K`: 呼出 Branch Switcher (分支切换)。
*   `Cmd+Shift+O` / `Ctrl+Shift+O`: Toggle Outline (切换大纲栏)。

## 本章相关配置

*   `ui.recent_commands_count`: Command Palette 显示的最近命令数量 (Default: 3).
*   `ui.recent_docs_count`: Quick Open 显示的最近文件数量 (Default: 10).
*   `ui.sidebar_visible`: 是否显示左侧主侧边栏 (Default: true).
*   `ui.statusbar_visible`: 是否显示底部状态栏 (Default: true).
*   `ui.outline_visible`: 是否显示右侧大纲栏 (Default: true).
