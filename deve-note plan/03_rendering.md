# 03_rendering.md - 渲染篇 (Rendering)

## 编辑器内核 (The Editor Kernel)

*   **Input Layer**: 采用 `ContentEditable` (Web) 或 CodeMirror 6 (Desktop) 作为输入捕获层。
*   **State Layer**: 绑定 Loro CRDT 状态 (Ledger)，作为单一真值源。
*   **Projection Layer (投影层)**: 负责将 Ledger 状态不仅呈现为 **Vault** 中的物理文件，还实时渲染为可视化的视图。支持 Block Mode, Source Mode, 和 Live Preview 三种。
*   **Technology Stack**:
	*   **Default (Light Core)**：CodeMirror 6 Source Mode (对应 **Projection** 的纯文本形态)。
	*   **Extension (Rich)**：Milkdown (Prosemirror) Live Preview (提供富文本交互)。

### Interaction Philosophy (交互哲学)
*   **Source-First (源码优先)**: 编辑器的核心是文本。任何渲染效果 (Widgets/Decorations) 均视为对源码的"增强"。
*   **Cursor Reveal (光标揭示)**:
    *   **Rule**: 当光标 **接触 (Touch)** 或 **进入 (Inside)** 渲染元素的源码范围时，渲染层 **MUST** 立即让位 (Hidden/Removed)，将原始 Markdown 源码完整呈现给用户。
    *   **Scope**: 此规则适用于所有渲染组件，包括但不限于：
        *   **Math**: Inline (`$...$`) & Block (`$$...$$`).
        *   **Diagrams**: Mermaid Code Blocks.
        *   **Inline Styles**: Bold/Italic/Strikethrough Syntax Marks.
        *   **Frontmatter**: YAML metadata block.
    *   **Goal**: 确保用户在编辑时永远面对的是"真理" (Source Code)，而在阅读时享受的是"美观" (Rendered View)。

## 数学渲染规范 (Mathematical Rendering Specification)

*   **Engine**: 默认集成 **KaTeX** (性能优先) 或 **MathJax 3** (精度优先)。
*   **Typography**: 代码体使用 JetBrains Mono/Fira Code；正文体使用 Merriweather 等衬线字体。
*   **Delimiters**:
    *   **Inline**: `$...$`。
    *   **Block**: `$$...$$`。
*   **Heuristic Logic (启发式判定)**: 仅当 `$` 紧邻非空字符时触发渲染 (e.g., $x$ is math; $ x $ is text)。普通货币符号 (e.g., $100) 无需转义。仅在歧义时支持 `\$` 强制转义。

### Interaction Flow (交互流程)
1.  **Trigger**: 输入 `$$` 自动切换为 Block Math 状态。
2.  **Editing**: 输入 LaTeX 源码，即时渲染 Live Preview。
3.  **Completion**: 按下 `Ctrl+Enter` 折叠源码，仅显示渲染后的 SVG 结果。
4.  **Copy-Paste Protection (MUST)**:
    *   **Behavior**: 当用户复制渲染后的数学公式时，系统 **MUST** 拦截复制并在剪贴板中写入原始 LaTeX 源码 (而非 Unicode 乱码)。
    *   **Implementation**: 集成 `copy-tex.js` 扩展作为核心必备组件。

## Mermaid 图表渲染规范 (Mermaid Rendering Specification)

*   **Type**: Core Rendering Feature (Not Plugin). 视为核心渲染能力，随主包同步加载。
*   **Syntax**: ` ```mermaid ` 代码块。
*   **Rendering Logic**:
    *   **Synchronous**: 静态打包，无网络请求，确保离线可用性。
    *   **DOM Awareness**: 自动检测 DOM 挂载状态，防止渲染竞争。
*   **Sizing Strategy (尺寸策略)**:
    *   **Constraint**: 图表容器高度 **Strictly Equals** 源代码行数占据的高度 (`LineCount * LineHeight`)。
    *   **Scaling**: 图表内容 (SVG) 强制设置为 `100% Width/Height` 并保持比例 (`preserveAspectRatio="meet"`).
    *   **Zoom via Newlines**: 用户可以通过在代码块末尾添加换行符 (Enter) 来增加容器高度，从而等比例放大图表 (Zoom In)。

## Markdown 解析规则 (Parsing Rules)

### Phase 1: Block Level Parsing (块级解析)
1.  **Fenced Code (```)**: 优先级最高 (Highest Priority)。解析器 **MUST** 将其视为原子块，内部忽略所有 Markdown 标记（包括 `$$`），仅执行语法高亮。
2.  **Block Math ($$)**: 优先级次高。解析器 **MUST** 将其视为原子块，内容直接传递给 LaTeX 引擎。
3.  **HTML Block**: 第三优先级。防止公式内的 `< >` 符号破坏 HTML 结构。
4.  **Structure Elements**: Header, List, Quote, Table 确立结构后，其内容进入行内扫描阶段。

### Phase 2: Inline Level Parsing (行内解析)
*   **Principle**: First come, first served (先匹配者优先)。高优先级元素内部 **MUST NOT** 渲染低优先级元素。
1.  **Inline Code (` `)**: 优先级最高。解析器 **MUST** 优先消耗反引号。内部不解析转义字符、公式或样式标记 (e.g., `echo $PATH` 中的 `$` 被保护为普通字符)。
2.  **Escaping (\)**: 次高。转义紧随其后的单个字符 (e.g., `\$100` 渲染为 `$100`)。
3.  **Inline Math ($...$)**: 核心优先级。视为原子节点，内容传递给 LaTeX 引擎。受 Inline Code 和 Escaping 保护。
4.  **Auto Link (<url>)**: 防止 URL 中的特殊字符触发格式解析。
5.  **Containers (Links / Images)**: 允许内部嵌套样式 (e.g., Bold)。
6.  **Styles**: **Bold** > *Italic* > ~~Strike~~.

## Markdown 语法限制 (Syntax Whitelist)

### 块级元素 (Block Elements)
*   **Headings**: `# H1` 到 `###### H6`。
*   **Paragraphs**: 普通文本段落。
*   **Blockquotes**: `> 引用`，支持嵌套。
    *   **Callouts (Admonitions)**: `> [!NOTE]` 语法，支持 INFO, CAUTION, TIP 等类型。
*   **Lists**: 无序 `-, *, +`，有序 `1.`，任务 `- [ ]` (GFM)。
*   **Code Blocks**: Fenced Code ` ```language `，支持语法高亮。
    *   **Mermaid**: ` ```mermaid ` 块自动渲染为图表。
*   **Math Blocks**: `$$...$$` (LaTeX 内容)。
*   **Tables**: GFM 风格 `| col | col |`，支持对齐语法 `:---`。
*   **Horizontal Rules**: `---`, `***`。
*   **HTML Blocks**: 支持基础 HTML 标签（需做 XSS 清洗）。
*   **Footnotes Definitions**: `[^1]: ...`。

### 行内元素 (Inline Elements)
*   **Code**: `` `code` ``。
*   **Math**: `$ ... $` (LaTeX 内容)。
*   **Links**: `[text](url "title")` 及自动链接 `<http://...>`。
    *   **WikiLinks**: `[[Link]]` 或 `[[Link|Alias]]`。支持内部文档跳转。
*   **Footnote Refs**: `[^1]`。
*   **Images**: `![alt](src)`。
*   **Emphasis**: **Bold** (`**` / `__`)，*Italic* (`*` / `_`)。
*   **Strikethrough**: ~~Strike~~ (`~~`) (GFM)。
*   **Escaping**: `\` (反斜杠转义)。

## 本章相关命令

* 无。

## 本章相关配置

*   `rendering.engine`: `KaTeX` (Default) | `MathJax`.
*   `rendering.font_family_mono`: Code block font setting.
*   `rendering.font_family_serif`: Document body font setting.
