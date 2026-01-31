# 08_ui_design_02_desktop.md - 桌面端设计 (Desktop UI)

> **Status**: Core Specification
> **Platform**: Desktop (Windows / macOS / Linux)

本节定义了 Desktop 端的“驾驶舱”布局规范与交互逻辑。

## 规范性用语 (Normative Language)
*   **MUST**: 绝对要求。
*   **SHOULD**: 强烈建议。

## 1. 布局哲学：驾驶舱 (The Cockpit Concept)

桌面端设计遵循 **"Information Stratification" (信息分层)** 原则，将界面划分为不同关注度的区域。

*   **L1 (Focus)**: 编辑区 (Editor)。绝对中心，无干扰。
*   **L2 (Context)**: 侧边栏 (Sidebar)。提供导航上下文 (Explorer, Outline)。
*   **L3 (Meta)**: 状态栏 (Status Bar)。提供系统元数据 (Git Branch, Sync Status)。
*   **L4 (Floating)**: 悬浮层 (Overlays)。按需出现的命令入口。

## 2. 动态网格系统 (Dynamic Grid System)

### 2.1 布局定义 (Layout Definition)
系统采用 5 列动态网格布局。形式化定义如下：

$$ Grid = [Col_{sidebar}, Col_{diff\_old}, Col_{editor}, Col_{outline}, Col_{chat}] $$

*   **Constraint**: $Col_{editor}$ (Col 3) 总是占据剩余空间 (`1fr`)。
*   **CSS Implementation**:
    ```css
    display: grid;
    grid-template-columns: var(--w-sidebar) var(--w-diff) 1fr var(--w-outline) var(--w-chat);
    ```

### 2.2 布局可视化 (Visualization)

**Main Workbench Structure**:

| Layer      | Col 1 (Resizable) | Col 2 (Fixed) | Col 3 (Flex) | Col 4 (Fixed) | Col 5 (Resizable) |
| :--------- | :---------------- | :------------ | :----------- | :------------ | :---------------- |
| **Top**    | `[Explorer]`      | `Old.rs`      | `New.rs`     | `Outline`     | `AI Chat`         |
| **Body**   | File Tree         | Read-Only     | Writable     | H1..H6        | Chat Log          |
| **Resize** | `[||]` Handle     | -             | -            | -             | `[||]` Handle     |

### 2.3 组件规范 (Component Specs)

*   **Primary Sidebar (Col 1)**:
    *   **Behavior**: **MUST** 支持拖拽调整宽度 (`180px` ~ `500px`)。
    *   **State**: 包含 `ActivityBar` (Icon Strip) 与 `SideView` (Content)。
*   **Editor Area (Col 2 & 3)**:
    *   **Single Mode**: $Width(Col_2) = 0$。
    *   **Diff Mode**: $Width(Col_2) = 50\%$。
    *   **Scroll Sync**: 当滚动 Col 3 时，Col 2 必须根据文档高度比例同步滚动。
*   **Unified Search Modal (The Brain)**:
    *   **Definition**: 全局统一的输入入口 $I$。
    *   **Modes**:
        *   `Command`: Prefix `>` (e.g., `>Toggle Sidebar`).
        *   `File`: No Prefix (e.g., `src/main.rs`).
        *   `Branch`: Prefix `@` (e.g., `@feature/xyz`).

## 3. 源代码管理界面 (Source Control UI)

### 3.1 视图结构 (View Structure)
定义源代码管理视图 $V_{sc}$ 为三个集合的并集：
$$ V_{sc} = S_{staged} \cup S_{unstaged} \cup H_{commits} $$

*   **Staged ($S_{staged}$)**: 已暂存的文件集合。支持 `Unstage All`。
*   **Unstaged ($S_{unstaged}$)**: 工作区的脏文件集合。支持 `Stage All` / `Discard All`。

### 3.2 变更状态可视化
每个变更项 $Item \in V_{sc}$ **MUST** 使用语义化颜色标记状态：

*   **Modified ($M$)**: Orange (`var(--color-modified)`).
*   **Added ($A$)**: Green (`var(--color-added)`).
*   **Deleted ($D$)**: Red (`var(--color-deleted)`).

## 本章相关命令

*   `view.layout.toggle_sidebar`: 切换侧边栏可见性。
*   `view.layout.toggle_diff`: 切换 Diff/Editor 模式。
*   `git.stage_all`: 暂存所有更改。
