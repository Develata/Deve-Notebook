# 03_rendering.md - 渲染篇 (Rendering)

## 编辑器内核 (The Editor Kernel)

* **Layer 1 (Input)**: `ContentEditable` 或 CodeMirror 6。
* **Layer 2 (State)**: 绑定 Loro CRDT 状态。
* **Layer 3 (Render)**：Block Mode / Source Mode / Live Preview。

* **技术选型**：
	* **默认（轻核心）**：CodeMirror 6 Source Mode。
	* **可选（重扩展）**：Milkdown (Prosemirror) Live Preview。

## 数学美学 (Mathematical Aesthetics)

* **排版**：默认集成 **KaTeX** (快速) 或 **MathJax 3** (精确)，支持复杂的数学公式渲染。
* **字体**：预设适合代码和数学公式的等宽字体 (如 JetBrains Mono, Fira Code) 和衬线字体 (如 Merriweather)。
* **体验细节**：`$...$` 行内，`$$...$$` 块级；输入 `$$` 自动切块；KaTeX 优先。
* **LaTeX 渲染约定**：采用 VS Code 风格的智能边界判定。仅当 $ 紧邻非空字符时触发渲染（例如 $x$，而$x $，$ x $与$ x$ 均不渲染），普通货币符号（例如 $100）无需转义即可显示，但在极少数产生歧义时支持 \$ 强制转义。

### 交互流程: 数学公式编写 (The Math Flow)
1.  输入 `$$` -> 切换公式块 (Block Math)。
2.  输入 LaTeX -> 实时预览 (Live Preview)。
3.  `Ctrl+Enter` -> 折叠显示 SVG。

## Markdown 解析优先级 (Parsing Priority)

### Phase 1: 块级扫描 (Block Level)
1.  **Fenced Code (```)**: 👑 最高。原子性，内部忽略所有标记 (含 $$)，仅做高亮。
2.  **Block Math ($$)**: 🥈 次高。原子性，内容交由 LaTeX 引擎。
3.  **HTML Block**: 第三。防止公式内 `< >` 破坏布局。
4.  **Structure (Header/List/Quote/Table)**: 结构确立后，内容进入行内扫描。Table 优先级较低。

### Phase 2: 行内扫描 (Inline Level)
*   *原则：先匹配先得 (First come, first served)；高优内部不渲染低优。*
1.  **Inline Code (` `)**: 👑 行内最高。**必须最先吃掉反引号**。内部不解析转义/公式/加粗 (e.g. `echo $PATH` 中的 $ 被保护)。
2.  **Escaping (\)**: 🥈 次高。转义后续单个字符 (e.g. `\$100` -> `$100`)。
3.  **Inline Math ($...$)**: 🥉 核心。原子性，内容交由 LaTeX。受 Code 和 Escaping 保护。
4.  **Auto Link (<url>)**: 防止 URL 特殊字符触发格式。
5.  **Links / Images**: 容器，内部允许加粗等样式。
6.  **Styles**: **Bold** > *Italic* > ~~Strike~~.

## Markdown 语法限制 (Syntax Whitelist)

### 块级元素 (Block Elements)
*   **Headings**: `# H1` 到 `###### H6`。
*   **Paragraphs**: 普通文本段落。
*   **Blockquotes**: `> 引用`，支持嵌套。
*   **Lists**: 无序 `-, *, +`，有序 `1.`，任务 `- [ ]` (GFM)。
*   **Code Blocks**: Fenced Code ` ```language `，支持语法高亮。
*   **Math Blocks**: `$$...$$` (LaTeX 内容)。
*   **Tables**: GFM 风格 `| col | col |`，支持对齐语法 `:---`。
*   **Horizontal Rules**: `---`, `***`。
*   **HTML Blocks**: 支持基础 HTML 标签（需做 XSS 清洗）。

### 行内元素 (Inline Elements)
*   **Code**: `` `code` ``。
*   **Math**: `$ ... $` (LaTeX 内容)。
*   **Links**: `[text](url "title")` 及自动链接 `<http://...>`。
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
