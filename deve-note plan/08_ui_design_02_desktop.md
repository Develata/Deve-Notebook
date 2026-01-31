# 08_ui_design_02_desktop - Desktop UI

## The "Cockpit" Concept (驾驶舱概念)
* **信息分层**：
	* **L1 (Focus)**：编辑区是绝对中心，无干扰。
	* **L2 (Context)**：左侧边栏（文件树），右侧边栏（大纲）提供导航。
	* **L3 (Meta)**：底部状态栏显示“和解状态”、Git 分支、字数统计。
	* **L4 (Floating)**：`Cmd+K` 命令面板和悬浮工具栏，按需出现。
* **键盘优先 (Keyboard First)**：所有 UI 操作必须有快捷键；模仿 Vim/VS Code/Nano 逻辑。

## Workbench Layout (Cursor-Style)
应用采用类似 Cursor 的现代化布局，最大支持 **5列并排 (5-Column Grid)**，顶部为全局标题栏，底部为独立状态栏。

### Layout Visualization (布局模拟)

**1. Title Bar (Global Header)**
| Section    | Content (Flex Layout)                                   |
| :--------- | :------------------------------------------------------ |
| **Left**   | **Logo**: `Deve-Note` `[Badge: Connected]`              |
| **Center** | *(Empty / Drag Region)*                                 |
| **Right**  | `[Home 🏠]` `[Toggle Sidebar 🗖]` `[Terminal/Command >_]` |

**2. Floating Overlays (Unified Search Modal)**
*   **Component**: 一个统一的模态搜索组件 (Unified Search Box)，复用于三种核心场景。
*   **Modes**:
    *   **Command Palette** (`>_`): `Cmd+Shift+P` / `Ctrl+Shift+P` (Prefix: `>`)
    *   **Quick Open** (`🔍`): `Cmd+P` / `Ctrl+P` (Prefix: None)
    *   **Branch Switcher** (`🌿`): `Cmd+Shift+K` / `Ctrl+Shift+K` (Prefix: `@` or custom UI)

*   **Smart Toggle Logic (智能切换逻辑)**:
    系统 **MUST** 根据当前状态判断快捷键行为：
    1.  **If Hidden**: 唤出搜索框，进入对应模式，指针锁定输入栏。
    2.  **If Visible & Mode Matches**: **关闭** 搜索框 (Toggle Off)。
    3.  **If Visible & Mode Differs**: **立即切换** 到新快捷键对应的模式 (Context Switch)，保持输入焦点。

*   **Focus Restoration Rule (焦点还原)**:
    当搜索框关闭时：
    1.  **If Action Taken** (e.g., 打开了新文件/切换了分支/执行了命令): 焦点移至 Editor 的默认位置或新内容的起始位置。
    2.  **If Cancelled** (无变化): 焦点 **MUST** 还原到唤出前的 **精确位置** (Line & Column)，确保用户心流不被打断。

**3. Main Grid (5 Columns)**
| Layer      | Col 1 (Resizable)       | Col 2 (Fixed/Ratio) | Col 3 (Fixed/Ratio) | Col 4 (Fixed)              | Col 5 (Resizable) |
| :--------- | :---------------------- | :------------------ | :------------------ | :------------------------- | :---------------- |
| **Header** | **Primary Sidebar**     | **Diff Old (RO)**   | **Diff New (RW)**   | **Outline**                | **AI Chat**       |
| **Top**    | `[Explorer][Search]`    | `Filename (Left)`   | `Filename (Right)`  | `Filter...`                | `Model: GPT-4`    |
| **Body**   | `> src`<br>`  > sub.rs` | `2 -  old()`        | `2 +  new()`        | `H1 Title`<br>`  $E=mc^2$` | `User: Hi`        |

### Status Bar Layout (独立的底部通栏)
状态栏 **MUST NOT** 遵循上方的分列网格，而是 **MUST** 采用 Flex 布局（左/右对齐）：

| Section         | Content (Left to Right)                             |
| :-------------- | :-------------------------------------------------- |
| **Left Group**  | `[Remote: iPad]` `[Branch: main*]` `[Sync: 🔄]`      |
| **Right Group** | `[Spectator: READ-ONLY]` `[UTF-8]` `[Ln 12, Col 5]` |

*   **Column 1: Primary Side Bar (主要侧边栏)**
    *   **Position**: 最左侧。
    *   **Structure**: 顶部 **MUST** 包含 **Activity Tabs** (图标水平排列)，下方为具体视图内容。
    *   **Behavior**: **MUST** 支持拖拽调整宽度，**MUST** 支持折叠。
*   **Column 2 & 3: Main Editor Area (主编辑区)**
    *   **Single Mode**: 只有一列编辑器。
    *   **Diff Mode**: 分裂为两列 (`Diff Old` | `Diff New`)。
        *   **Left (Old)**: 只读 (Read-Only)。
        *   **Right (New)**: 可读写 (Writable)。
        *   **Behavior**: 两列 **MUST** 保持行对齐 (Line Alignment) 并同步滚动 (Sync Scrolling)。
*   **Column 4: Outline Panel (大纲栏)**
    *   **Position**: 紧邻编辑器右侧。
    *   **Content**: **MUST** 仅渲染纯文本与 Inline Math。**MUST NOT** 渲染 Block Math 或其他富文本格式。
    *   **Trigger (Toggle)**: **MUST** 使用 **Editor Overlay Button** (悬浮按钮)。
        *   位置：主编辑器右上角 (Top-Right Corner)，滚动条内侧。
        *   图标：Book Icon (📖)。
        *   行为：点击切换大纲栏的展开/折叠。
    *   **Behavior (Fixed)**: 宽度 **MUST** 固定 (Fixed Width, e.g., 260px)，**MUST NOT** 允许拖拽调整。
*   **Column 5: AI Agent Chat (AI 助手)**
    *   **Position**: 最右侧 (Far Right)。
    *   **Behavior**: **MUST** 支持拖拽调整宽度，默认隐藏 (Collapsed)。
*   **Resizability Note**: 除 Diff 视图内部比例可能锁定外，Sidebar 和 AI Chat **MUST** 支持用户拖拽边缘调整宽度。Outline **MUST** 固定宽度。

### Detailed View Specifications (视图详情)
*   **Title Bar (顶部标题栏)**:
    *   **Style**: 极简风格 (Minimalist).
    *   **Content**: 左侧 **MUST** 仅显示 App Name + Connection Status；右侧 **MUST** 显示核心导航图标。
    *   **Interaction**: 顶部 **MUST NOT** 包含输入框。点击 `>_` 图标或快捷键 **MUST** 唤起 **悬浮搜索框 (Floating Modal)**。
*   **Unified Search Box (统一搜索框)**:
    *   **Visual**: 屏幕中上方弹出的模态框 (Centered Modal).
    *   **Structure**: `[Icon + Input Field]` -> `[Scrollable List]` -> `[Footer Hints]`.
    *   **Shadow**: **MUST** 有明显的 Drop Shadow 以区分层级。
    *   **Modes**: 支持 `Command`, `File`, `Branch` 三种模式的UI复用。
    *   **FileOps (命令式文件操作)**:
        *   **入口统一**: 侧边栏文件树的 UI 按钮 (Copy/Rename/Move/Create) **MUST** 唤起同一个搜索框。
        *   **预填策略**:
            *   Move: `>mv "<selected>" ""`，光标定位到目标参数。
            *   Copy: `>cp "<selected>" ""`，光标定位到目标参数。
            *   Rename: `>mv "<selected>" ""` (等价 Move)。
            *   Create: `+` 作为创建模式入口。
        *   **命令语法**:
            *   `>mv <src> <dst>` / `>cp <src> <dst>` / `>rm <path>` / `+<path>`.
        *   **参数解析规则**:
            *   无空格路径可不加引号；含空格必须加引号。
            *   允许混合引号 (src 无引号 / dst 有引号)。
            *   自动补 `.md`：参数**不以 `/` 结尾**且**无 `.xxx` 后缀**时自动补齐。
        *   **候选列表**:
            *   **只显示目录**。
            *   分组显示：`recent` (最近 4 个 mv 目标目录) 与 `all` (所有目录按顺序排列)。
        *   **错误提示**:
            *   路径含空格但未加引号时，**MUST** 给出明确提示。

## Source Control UI (源代码管理界面)

*   **SourceControlView (源代码管理视图)**：侧边栏的版本控制主容器。
    *   **Description (描述)**：采用 VS Code 风格的紧凑布局，提供可折叠的子区块。
    *   **Structure**: $V_{sc} = \{ \text{Repositories}, \text{Changes}, \text{History} \}$。
*   **Changes (变更列表)**：文件变更的可视化容器。
    *   **Description (描述)**：将变更按状态分为 Staged（已暂存）与 Unstaged（未暂存）两个区块。
    *   **Composition**: $Changes = StagedSection \cup UnstagedSection$。
*   **ChangeItem (变更条目)**：单个文件变更的可视化单元。
    *   **Description (描述)**：渲染文件图标、名称、路径，以及状态标记 (M/A/D)。
    *   **Status Colors**: $M \to \text{Orange}$ (`#d7ba7d`), $A \to \text{Green}$ (`#73c991`), $D \to \text{Red}$ (`#f14c4c`)。
*   **StagedSection (暂存区组件)**：已暂存变更的列表容器。
    *   **Actions**: `Unstage All` (取消全部暂存)。
*   **UnstagedSection (工作区组件)**：未暂存变更的列表容器。
    *   **Actions**: `Stage All` (暂存全部), `Discard All` (放弃全部)。
*   **Commit (提交组件)**：提交信息输入框与提交按钮。
    *   **Constraint**: 提交信息 MUST 非空。
*   **History (历史记录)**：提交历史的时间轴视图。
*   **Repositories (仓库列表)**：当前 Branch 下的 Repo 下拉切换列表。

## Spectator Mode Visuals (旁观者视觉)
*   **Watermark**: 编辑器背景增加**灰色/斜纹水印**。
*   **Status**: 状态栏显示橙色 "**READ ONLY**"。
