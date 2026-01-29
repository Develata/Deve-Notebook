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

## Markdown 解析规则 (Parsing Rules)

### Phase 1: Block Level Parsing (块级解析)
1.  **Fenced Code (```)**: 优先级最高 (Highest Priority)。解析器 **MUST** 将其视为原子块，内部忽略所有 Markdown 标记（包括 `$$`），仅执行语法高亮。
2.  **Block Math ($$)**: 优先级次高。解析器 **MUST** 将其视为原子块，内容直接传递给 LaTeX 引擎。
3.  **HTML Block**: 第三优先级。防止公式内的 `< >` 符号破坏 HTML 结构。
4.  **Structure Elements**: Header, List, Quote, Table 确立结构后，其内容进入行内扫描阶段。

### Phase 2: Inline Level Parsing (行内解析)
*   **Principle**: First come, first served (先匹配者优先)。高优先级元素内部 **MUST NOT** 渲染低优先级元素。
1.  **Inline Code (` `)**: 优先级最高。解析器 **MUST** 优先消耗反引号。内部不解析转义字符、公式或样式标记 (e.g., `echo $PATH` 中的 `$` 被保护为普通字符)。
2.  **Escaping (\)**: 次高。转义紧随其后的单个字符。
    *   **MUST** 正确处理上下文相关的特殊转义：`\$` (Prevent Math), `\|` (Prevent Table Split), `\` (Literal Backslash)。
    *   e.g., `\|` 在表格中应渲染为竖线而不切分单元格；`\$` 应渲染为美元符号不触发公式。
3.  **Inline Math ($...$)**: 核心优先级。视为原子节点，内容传递给 LaTeX 引擎。受 Inline Code 和 Escaping 保护。
4.  **Auto Link (<url>)**: 防止 URL 中的特殊字符触发格式解析。
5.  **Containers (Links / Images)**: 允许内部嵌套样式 (e.g., Bold)。
6.  **Styles**: **Bold** > *Italic* > ~~Strike~~.

## 核心渲染能力 (Core Rendering Capabilities)

本节定义的渲染组件均为系统内置的第一类公民 (First-Class Citizens)，随主包同步加载，具备一致的交互哲学。

### 1. 数学公式 (Mathematics)
*   **Engine**: 默认集成 **KaTeX** (性能优先) 或 **MathJax 3** (精度优先)。
*   **Typography**: 代码体使用 JetBrains Mono/Fira Code；正文体使用 Merriweather 等衬线字体。
*   **Delimiters**: Inline `$...$`, Block `$$...$$`.
*   **Heuristic Logic**: 仅当 `$` 紧邻非空字符时触发渲染。
*   **Interaction Flow**:
    1.  **Trigger**: 输入 `$$` 自动切换为 Block Math 状态。
    2.  **Editing**: 输入 LaTeX 源码，即时渲染 Live Preview。
    3.  **Completion**: 按下 `Ctrl+Enter` 折叠源码，仅显示渲染后的 SVG 结果。
    4.  **Protection**: 复制公式时拦截并写入 LaTeX 源码。

### 2. Mermaid 图表 (Diagrams)
*   **Syntax**: ` ```mermaid ` 代码块。
*   **Rendering Logic**: 静态打包，无网络请求，DOM 感知。
*   **Sizing Strategy**:
    *   **Constraint**: 容器高度 **Strictly Equals** 源码行数高度。
    *   **Scaling**: 内容 (SVG) 强制 `100%` 填充并保比 (`preserveAspectRatio="meet"`).
    *   **Zoom**: 通过添加换行符增加高度来放大图表。

### 3. 标准富文本扩展 (Rich Text Widgets)

以下扩展增强了标准 Markdown 的表现力：

*   **Smart Tables (智能表格)**:
    *   **Syntax**: GFM Table Syntax.
    *   **Behavior**: 渲染为样式化的 HTML `<table>`。
*   **Interactive Task Lists (交互式任务列表)**:
    *   **Syntax**: `- [ ]` / `- [x]`.
    *   **Behavior**: 渲染为可点击的 Checkbox，点击即修改源码。
*   **List Markers (列表标记)**:
    *   **Target**: Bullet lists (`-`, `*`) and Ordered lists (`1.`).
    *   **Behavior**: 将 Markdown 标记 (`-`) 替换为视觉符号 (e.g., `○` or `•`)，有序列表保持数字。
    *   **Implementation**: `list_marker.js` (Decoration Widget).
*   **Inline Images (行内图片)**:
    *   **Syntax**: `![alt](url)`.
    *   **Behavior**: 渲染为受限宽高的行内图片 (`max-height: 400px`)。
*   **Block Styling (块级样式)**:
    *   **Target**: Fenced Code / Blockquotes.
    *   **Behavior**: 为整行添加背景色装饰 (`cm-code-block-line`, `cm-blockquote-line`)。
    *   **Note**: 唯一不受光标揭示逻辑影响的持久化装饰。
*   **Hybrid View (混合视图)**:
    *   **Scope**: Headings (`#`), Emphasis (`*`, `_`), Strikethrough (`~~`), Quotes (`>`).
    *   **Behavior**: 当光标离开元素范围时，自动隐藏 Markdown 语法标记；光标进入时显示。
*   **Frontmatter Support (元数据支持)**:
    *   **Syntax**: YAML Frontmatter (`---` ... `---`).
    *   **Behavior**: 自动识别并提供特殊的背景样式 (`cm-frontmatter-block`)。
    *   **Cursor Reveal**: 光标移出区域时隐藏首尾 `---` 分隔符，仅保留内容区域的视觉提示。

### 4. 代码块 (Code Blocks)
*   **Syntax**: Fenced Code (` ``` `).
*   **Toolbar**: 渲染的代码块右上角 **MUST** 显示两个按钮（从左到右）：
    *   **Copy**: 点击复制块内所有内容。
    *   **Ellipsis (...)**: 点击唤出菜单。
*   **Menu Extensibility**:
    *   **Plugin API**: **MUST** 预留接口允许插件向菜单添加选项。
    *   **Empty State**: 如果没有选项，**MUST** 唤出一个空白菜单。

### 5. 深度嵌套与混合列表 (The Nested Hell)

*   **Definition**: 测试列表、引用、代码块与数学公式的混合递归嵌套能力。
*   **Rendering Logic**: 渲染引擎 **MUST** 支持任意层级的递归嵌套 (Recursive Nesting)，不得出现渲染崩坏或样式错位。
*   **Test Case Criteria (验收标准)**:
    *   **Indentation (缩进)**: 每一层嵌套 **MUST** 具有清晰的视觉缩进 (Visual Indentation)。
        *   **Implementation**: 使用 CSS Variable `--depth` 结合 `linear-gradient` 动态计算背景。
        *   **Formula**: `calc(var(--bq-indent-step) * (var(--depth) - 1))` 用于计算边框线偏移量。
    *   **Context Preservation (上下文保留)**:
        *   引用块内的代码块 **MUST** 使用多层背景 (`background-image`) 叠加：底层为引用块边框线，顶层为代码块背景色。
        *   具体逻辑见 `apps/web/style/_code-block.css`.
    *   **Complexity Support**: 支持 List -> Blockquote -> List -> Code/Math 的混合结构。

## Markdown 语法限制 (Syntax Whitelist)

### 块级元素 (Block Elements)
*   **Headings**: `# H1` 到 `###### H6`。
*   **Paragraphs**: 普通文本段落。
*   **Blockquotes**: `> 引用`，支持嵌套。
    *   **Callouts (Admonitions)**: `> [!NOTE]` 语法，支持 INFO, CAUTION, TIP 等类型。
*   **Lists**: 无序 `-, *, +`，有序 `1.`，任务 `- [ ]` (GFM)。
*   **Code Blocks**: Fenced Code ` ```language `，支持语法高亮。
    *   **Indented Code**: 4个空格缩进的代码块。
    *   **Mermaid**: ` ```mermaid ` 块自动渲染为图表。
*   **Math Blocks**: `$$...$$` (LaTeX 内容)。
*   **Tables**: GFM 风格 `| col | col |`，支持对齐语法 `:---`。
*   **Horizontal Rules**: `---`, `***`。
*   **HTML Blocks**: 仅支持 `<br>` 换行标签。其他 HTML 标签将被过滤。
*   **Footnotes Definitions**: `[^1]: ...`。

### 行内元素 (Inline Elements)
*   **Code**: `` `code` ``。
*   **Math**: `$ ... $` (LaTeX 内容)。
*   **Links**: `[text](url "title")` 及自动链接 `<http://...>`。
    *   **WikiLinks**: `[[Link]]` 或 `[[Link|Alias]]`。支持内部文档跳转。
*   **Line Breaks**:
    *   **GFM Hard Breaks**: 每一个换行符（回车）均视为硬换行。
    *   **HTML**: 支持 `<br>` 标签强制换行。
*   **Emoji**: 支持短代码语法 `:smile:` (😃)。
*   **Footnote Refs**: `[^1]`。
*   **Images**: `![alt](src)`。
    *   **Note**: 不支持非标尺寸语法 (e.g. `![|100]`) 以保证通用性。
*   **Emphasis**: **Bold** (`**` / `__`)，*Italic* (`*` / `_`)。
*   **Strikethrough**: ~~Strike~~ (`~~`) (GFM)。
*   **Highlight**: 不支持非标高亮语法 (`==`) 以保证通用性。
*   **Escaping**: `\` (反斜杠转义)。
    *   **Support**: `!`, `"`, `#`, `$`, `%`, `&`, `'`, `(`, `)`, `*`, `+`, `,`, `-`, `.`, `/`, `:`, `;`, `<`, `=`, `>`, `?`, `@`, `[`, `\`, `]`, `^`, `_`, `` ` ``, `{`, `|`, `}`, `~`.

## 本章相关命令

* 无。

## 本章相关配置

*   `rendering.engine`: `KaTeX` (Default) | `MathJax`.
*   `rendering.font_family_mono`: Code block font setting.
*   `rendering.font_family_serif`: Document body font setting.
