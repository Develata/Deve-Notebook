# 第二章：UI 架构与组件系统 (UI Architecture)

前端采用 **Leptos (Signals)** + **Tailwind CSS**，构建一套高性能组件库。

### 1. 布局系统 (The Layout Engine)

采用 **"Activity Bar + Resizable Slot"** 布局 (Ref: VS Code)：

* **Activity Bar (最左侧)**: 侧边栏导航条。
  * **(Icon) Explorer**: 资源管理器。
  * **(Icon) Search**: 全局搜索。
  * **(Icon) Source Control**: Git / P2P 版本控制。
  * **(Icon) Extensions**: 插件管理。
  * **(Icon) Settings (Bottom)**: 全局设置。

* **Left Slot (Sidebar)**: 依据 Activity Bar 切换内容。
  * **Explorer View**: 文件树（文件/文件夹菜单、+按钮）。
  * **Git / Source Control View** (Ref: VS Code Source Control Panel):
    *   **Repository Identification (仓库标识)**:
        *   **Basis**: 基于 URL 唯一标识 (e.g., `https://deve-note1.me`).
        *   **Storage**: 1 Repo = 1 Database File + 1 Local Folder.
        *   **Dev Mock**: 在 Phase 0 硬编码 `https://deve-note1.me` 和 `https://deve-note2.me` 用于测试。
    *   **P2P Connection Strategy (连接策略)**:
        *   **Match**: URL 相同 -> 同一仓库协作 (显示 Shadow Branches)。
        *   **Mismatch**: URL 不同 -> **Multi-Root Workspace** (侧边栏分列显示 Local Repos + Peer Repos)。
        *   **Access Control**: Peer-only Repos (URL 不匹配) 强制为 **Read-Only** (仅允许 Copy/Diff)。
    *   **Repositories Section**:
        *   List of Active Repositories (Local & Peer).
    *   **Strict Branching Policy (严格分支策略 - Unique Logic)**:
        *   **No Arbitrary Creation**: ❌ 禁止随意创建新分支 (No `git checkout -b`).
        *   **Establishment Only (仅确立)**: ✅ 唯一创建 Local Branch 的方式是 **"Establish from Remote"** (即激活已存在的 Remote/Shadow 分支)。
        *   **Deletion Rule**: 允许删除 Local Branch。
        *   **Last Man Standing**: ⚠️ **禁止删除仓库的最后一个分支**。若要删除，必须执行 "Delete Repository" (删除整个库)。
    *   **Changes Section**: 暂存区 (Staged) / 未暂存 (Unstaged) 变更列表。
    *   **Commit Section**: 提交信息输入框 + Commit 按钮。
    *   **History Section**: 历史版本列表 + **Time Travel Slider** (回放历史版本、热力图可视化)。
    *   **Actions**:
        *   **Sync**: 同步本地与远程。
        *   **Pull / Establish**: 从 Peer 拉取 (若无本地分支则为 Establish)。
        *   **Merge**: 将选中的 Peer 仓库合并到 Local。
        *   **Push**: 推送到远程 Git 仓库。
        *   **Stash**: 暂存当前修改。
  * **Extensions View**: Installed/Recommended 列表。

* **Main Slot**: 多标签页 (Tabs) 编辑器 / 分屏 (Split View)。
* **Right Slot**: 大纲 (TOC) / 属性面板 (Metadata) / 插件面板。
  * **[Implemented] Table of Contents**: 支持层级缩进与点击跳转。
* **Bottom Slot**: 日志输出（核心）/ 终端面板 (Terminal, 插件可选)。
* **Internationalization (i18n)**: 核心 UI 文本 **MUST** 使用 `leptos_i18n` 管理。
* **特性**：面板状态持久化；模态框统一样式。

### 1.x Branch Switcher & Spectator Mode (分支切换与观测者)

* **Branch Switcher (组件)**：
    * **位置**：状态栏左下角。
    *   **交互**：点击唤起 **Unified Search Box** (Branch Mode)。
    *   **内容**：显示 `Local (Master)` 和 `Peer-XXX (Shadow)` (Natural Sort)。
    * **行为**：选择后 VFS 挂载点切换。

* **Spectator Mode (模式)**：
    * **定义**：当用户查阅 Shadow Repo 时的全局 UI 状态。
    * **特征**：
        *   **Visual**: 编辑器背景增加**灰色/斜纹水印**，状态栏显示 "READ ONLY"。
        *   **Input**: 键盘输入被拦截；文件树操作被禁用。
        *   **Action**: 仅允许 **Copy**、**Cherry-pick** 和 **Merge**。

### 2. 编辑器内核 (The Editor Kernel)

* **Layer 1 (Input)**: `ContentEditable` 或 CodeMirror 6。
* **Layer 2 (State)**: 绑定 Loro CRDT 状态。
* **Layer 3 (Render)**：Block Mode / Source Mode / Live Preview。

* **技术选型**：
	* **默认（轻核心）**：CodeMirror 6 Source Mode。
	* **可选（重扩展）**：Milkdown (Prosemirror) Live Preview。

**Markdown 基线能力**：标题、列表、引用、代码块、表格、链接/图片、脚注、数学公式；图片拖拽自动入库。

**数学体验细节**：`$...$` 行内，`$$...$$` 块级；输入 `$$` 自动切块；KaTeX 优先。

**LaTeX 渲染约定**：禁止裸 `$`。

### 3. 可视化系统 (Visualization System)

* **Global Graph (全域图谱，插件可选)**：力导向图引擎。
* ~~Time Travel Slider~~: 已移入 Source Control View 的 History Section。

### 4. Unified Command & Search Interface (统一指令与搜索界面)

*   **核心组件 (Core Component)**: `UnifiedSearchBox`。
    *   **设计模式**: 上下文敏感的模态搜索框 (Context-Sensitive Modal Search)。
    *   **复用性**: 作为全局单例组件，供 Command Palette、Open Document、Branch Switcher 等多个入口复用。

*   **通用交互行为 (General Interactions)**:
    *   **位置 (Position)**: 统一在屏幕顶部中央或用户习惯的视线焦点处弹出。
    *   **排序算法 (Sorting Strategy)**:
        *   **Top Section**: 基于 **MRU (Most Recently Used)** 算法显示最近使用的 $N$ 项 (Configurable)。
        *   **Bottom Section**: 剩余项按 **Natural Sort Order (自然排序)** 排列 (即广义字母表顺序，支持数字敏感排序，如 `file2` 排在 `file10` 之前)。
    *   **配置项 (Configuration)**:
        *   `recent_commands_count` (Default: 3)
        *   `recent_docs_count` (Default: 10)

*   **模式详解 (Modes)**:
    1.  **Command Navigation (指令导航)**:
        *   **Trigger**: 点击右上角 `Commands` 按钮 (或快捷键 `Cmd/Ctrl + K`)。
        *   **Context**: 搜索并将输入传递给 Command Registry。
        *   **Layout**: Top 3 Recent Commands -> All Commands (Natural Sort).
    2.  **Quick Open (快速打开)**:
        *   **Trigger**: 点击右上角 `Open Document` 按钮 (或快捷键 `Cmd/Ctrl + P`)。
        *   **Context**: 搜索并将输入传递给全局文件索引 (File Indexer)。
        *   **Layout**: Top 10 Recent Documents -> All Files (Natural Sort).
    3.  **Branch Switcher (分支切换)**:
        *   **Trigger**: 点击状态栏分支指示器 (或相关快捷键)。
        *   **Context**: 搜索并将输入传递给 Git/Peer Branch Manager。
        *   **Layout**: Current/Recent Branches -> All Branches (Natural Sort).
        *   **Action**: 点击切换分支或输入名称创建/切换。

*   **组件组织与扩展规范 (Component Organization & Extensibility)** (Added):
    *   **File Structure (文件结构)**: 所有相关组件必须严格遵循以下目录规范：
        *   `apps/web/src/components/command_palette/`: **Command Palette** 专用目录。包含命令搜索逻辑、Result Item 组件等。
        *   `apps/web/src/components/branch_switcher/`: **Branch Switcher** 专用目录。包含分支列表、分支切换逻辑等。
        *   `apps/web/src/components/quick_open/` (or similar): **Open Document** 专用目录。包含文件搜索、MRU 列表逻辑等。
        *   `apps/web/src/components/search_box/`: **Core Unified Search Box** 基础组件目录 (If extracted as shared base)。
    *   **Modularization Rule (模块化原则)**:
        *   **One Component, One File/Folder**: 每个组件代码若较短则对应单文件；若代码过长或包含子组件，**MUST** 封装为独立文件夹。
        *   **No Spaghettis**: 禁止将所有逻辑堆积在 `mod.rs` 或单一文件中。
    *   **Plugin Interface (插件接口)**:
        *   所有下拉/搜索组件 (Commands, Files, Branches) **MUST** 预留/暴露插件接口 (Traits/Hooks)。
        *   允许插件注册新的 Command、提供新的 Search Source 或扩展列表项渲染 (Item Renderer)。
