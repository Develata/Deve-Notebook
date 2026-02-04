# 08_ui_design.md - 使用界面设计篇 (UI Design)

> **Status**: Core Specification
> **Modules**: [Web](./08_ui_design_01_web.md) | [Desktop](./08_ui_design_02_desktop.md) | [Mobile](./08_ui_design_03_mobile.md)

本章定义了 Deve-Note 的用户界面设计规范 (Design System) 与交互原则。

> **Responsive Mapping**: Web 端界面 **MUST** 在移动端视口匹配 Mobile 规范，在大屏视口匹配 Desktop 规范。

## 规范性用语 (Normative Language)
*   **MUST**: 绝对要求，违反即视为设计错误。
*   **SHOULD**: 强烈建议，偏离需有充分理由。
*   **MAY**: 可选实现。

## 1. 体验设计哲学 (Design Philosophy)

*   **Command First (命令优先)**：
    *   **Principle**: 系统能力 $C$ 与界面 $UI$ 正交。所有功能 $f \in C$ **MUST** 拥有唯一的 `CommandId`，且可通过命令面板 (`Cmd+K`) 调用。
    *   **Goal**: 降低 UI 密度，实现“键盘流”操作。
*   **Context Aware (上下文感知)**：
    *   **Principle**: 界面布局 $L$ 是当前状态 $S$ 的函数 $L = f(S)$。例如，进入 Diff 模式时，Grid 布局自动分裂。
*   **Visual Clarity (视觉清晰性)**：
    *   借鉴 SilverBullet 的清爽阅读感，但在编辑区保持 VS Code 级的代码掌控力。

## 2. 设计系统 (Design System)

### 2.1 Color Palette (Design Tokens)
系统 **MUST** 使用 CSS 变量定义 Design Tokens，严禁硬编码 Hex 值。

```css
:root {
  /* Layout Backgrounds */
  --bg-app: #1e1e1e;          /* App container */
  --bg-sidebar: #252526;      /* Sidebars */
  --bg-editor: #1e1e1e;       /* Main editor area */
  --bg-statusbar: #007acc;    /* Bottom bar (Status: OK) */
  --bg-statusbar-ro: #e06c75; /* Bottom bar (Status: Read-Only) */

  /* Component Backgrounds */
  --bg-input: #3c3c3c;
  --bg-overlay: rgba(0, 0, 0, 0.4); /* Backdrop */
  
  /* Foreground / Text */
  --fg-primary: #cccccc;
  --fg-secondary: #969696;    /* Hints */
  
  /* Semantic Colors (Git) */
  --color-added: #81b88b;     /* Git Added */
  --color-modified: #e2c08d;  /* Git Modified */
  --color-deleted: #e06c75;   /* Git Deleted */
}
```

### 2.2 Z-Index Registry (Layer Management)
为了防止层级冲突 (Z-Fighting)，系统 **MUST** 严格遵循以下分层定义：

| Level | Z-Index | Usage Scope |
| :--- | :--- | :--- |
| **L0: Editor** | 0 | CodeMirror content, base layout. |
| **L1: Chrome** | 10 | Status Bar, Title Bar. |
| **L2: Panels** | 20 | Sidebars (Desktop). |
| **L3: Floating** | 50 | Toolbars, Floating Action Buttons. |
| **L4: Overlay** | 100 | Backdrops, Mobile Drawers. |
| **L5: Modal** | 200 | Unified Search, Dialogs. |
| **L6: Toast** | 500 | Notifications. |

### 2.3 Iconography
*   **Standard**: 系统 **MUST** 使用 `lucide-leptos` crate。
*   **Style**: Stroke width `1.5px` (Elegant), Size `16px`.

## 3. 组件架构 (Component Architecture)

组件组织 **SHOULD** 遵循原子化设计原则，目录结构映射 UI 区域：

```text
apps/web/src/components/
├── activity_bar/       # Leftmost icon strip (L2)
├── sidebar/            # Resizable sidebar container (L2)
├── editor/             # CodeMirror wrapper (L0)
├── overlay/            # Modal & Floating UIs (L5)
└── shared/             # Atomic UI (Icon, Button)
```

## 4. 交互原则 (Interaction Principles)

### 4.1 Focus Management (焦点流转)
焦点管理是键盘操作体验的核心。定义为状态机 $S_{focus} \in \{Editor, Sidebar, Panel, Modal\}$。

*   **Focus Trap**: 打开模态框 (Modal) 时，系统 **MUST** 捕获焦点，Tab 键仅在 Modal 内部循环。
*   **Focus Restore**: 模态框关闭后，焦点 **MUST** 还原至 $S_{prev}$ (通常是编辑器内的光标位置)。

### 4.2 Configuration Schema
前端配置存储在 LocalStorage 的 `deve_config` 键中：

```json
{
  "ui": { "sidebar_visible": true, "theme": "dark" },
  "editor": { "font_size": 14, "vim_mode": false }
}
```

## 本章相关命令

*   `Cmd+Shift+P`: 呼出 Command Palette。
*   `Cmd+P`: 呼出 Quick Open (文件跳转)。
*   `Cmd+Shift+K`: 呼出 Branch Switcher。
*   `Cmd+B`: Toggle Sidebar。

## 本章相关配置

*   `ui.theme`: 界面主题 (Default: "dark").
*   `ui.sidebar_visible`: 侧边栏可见性 (Default: true).
