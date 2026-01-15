# 03_rendering.md - 渲染篇 (Rendering)

## 编辑器内核 (The Editor Kernel)

*   **Input Layer**: 采用 `ContentEditable` (Web) 或 CodeMirror 6 (Desktop) 作为输入捕获层。
*   **State Layer**: 绑定 Loro CRDT 状态，作为单一真值源。
*   **Render Layer**: 支持 Block Mode, Source Mode, 和 Live Preview 三种渲染策略。
*   **Technology Stack (选型)**：
	*   **Default (Light Core)**：CodeMirror 6 Source Mode (纯文本源码模式)。
	*   **Extension (Rich)**：Milkdown (Prosemirror) Live Preview (实时预览模式)。

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
