# 第二章：UI 架构与组件系统 (UI Architecture)

前端采用 **Leptos (Signals)** + **Tailwind CSS**，构建一套高性能组件库。

### 1. 布局系统 (The Layout Engine)

采用 **"Resizable Slot" (可缩放插槽)** 布局：

* **Left Slot**: 文件树（核心）/ 双向链接图谱 (Mini Graph, 插件可选)。
  * **文件项操作菜单 (File Item Menu)**：Rename, Copy, Paste, Open in New Window, Move to, Delete.
  * **文件夹项操作菜单 (Folder Item Menu)**：New File, New Folder, Rename, Copy, Paste, Move to, Delete.
  * **快速新建按钮 (+)**：快速创建新文档。
* **Main Slot**: 多标签页 (Tabs) 编辑器 / 分屏 (Split View)。
* **Right Slot**: 大纲 (TOC) / 属性面板 (Metadata) / 插件面板。
  * **[Implemented] Table of Contents**: 支持层级缩进与点击跳转。
* **Bottom Slot**: 日志输出（核心）/ 终端面板 (Terminal, 插件可选)。
* **Internationalization (i18n)**: 核心 UI 文本 **MUST** 使用 `leptos_i18n` 管理。
* **特性**：面板状态持久化；模态框统一样式。

### 1.x Branch Switcher & Spectator Mode (分支切换与观测者)

* **Branch Switcher (组件)**：
    * **位置**：状态栏左下角。
    * **交互**：点击显示 `Local (Master)` 和 `Peer-XXX (Shadow)`。
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
* **Time Travel Slider (时光轴，插件可选)**：交互式热力图，回放历史版本。

### 4. 命令面板 (The Commander)

* 呼出：`Cmd/Ctrl + K`。
* 功能：导航、操作（Dark Mode, Git Push）、插入、快速文件列表。
