# 03_rendering.md - Markdown 体验篇

本章描述 Markdown 编辑与阅读体验，即用户在编辑区、Outline 和相关渲染区域里实际能看到和操作到的行为。

## 功能目标

用户应获得一个 **Source-first** 的 Markdown 工作流：

- 平时面对的是可控、可预测的源码编辑体验
- 阅读时可以享受公式、图表、任务列表等增强渲染
- 光标进入时永远能回到真实源码，不被“富文本假象”困住

## 验收边界

本功能篇只把下列用户可见行为纳入验收：

- 主编辑器默认 source-first；增强渲染只能作为源码之上的视觉投影存在。
- 标题、强调、引用、链接、frontmatter 等语法标记可以在非编辑焦点下隐藏或美化，但光标进入时必须显示真实源码。
- ATX 标题（`#` 到 `######`）的标题正文和空标题行都必须保持对应标题层级的整行视觉高度；隐藏 `#` 标记时不得退回普通段落行高。
- 空与非空 ATX 标题行（如 `#`、`# 标题`、`## 标题`、`### 标题`）在主编辑器与辅助 HTML Markdown 展示中应按 h1/h2/h3 层级保持可区分字号与高度；标题内的行内公式不应让整行回落到正文行高。
- 活动编辑行中的 CJK 无空格 ATX 标题候选（如 `#标题`）只作为编辑期视觉 affordance 维持标题行高，不改变保存后的 Markdown 语义。
- Math、Mermaid、task checkbox、frontmatter styling、table/image/list/blockquote/code toolbar、Ctrl/Cmd link activation 必须通过 Chrome MCP 手工走查确认具体浏览器行为。
- Outline 必须支持标题扫描、点击跳转、inline code/math/strong/em/del 的轻量显示，并把不支持语法按普通文本保留。
- 辅助 HTML 区域只承担轻量 Markdown 展示与操作入口职责；支持 tables、strikethrough、task list、code block wrapper、可选 apply button、`<br>` allowlist 与安全链接降级；AI chat 消息体额外支持 KaTeX TeX 展示；不承担主编辑器职责。
- 长文档体验只验收首屏响应、批量应用、渐进调度与重操作 gating；不宣称完整 virtual rendering。

下列能力不得作为本功能篇的已完成验收目标：

- 独立 Live Preview / Milkdown / 富文本 authority。
- 任意 HTML、`==highlight==`、完整 footnote、wikilink、emoji shortcode 语义。
- 完整 virtual render、全文 search gate、UTF-16 index cache 的端到端产品验收。
- rendering settings 的完整 GUI 持久化。

## 功能项

### 1. Source-First 编辑体验

- 文档打开后默认进入源码编辑态。
- 所有增强渲染都只是源码之上的视觉投影。
- 用户在任何时刻都可以通过移动光标看到真实 Markdown 源码。
- 原子操作示例：[`operations/doc_edit_confirmed_op.md`](./operations/doc_edit_confirmed_op.md)

### 2. Cursor Reveal

- 当光标进入公式、Frontmatter、强调、引用、列表标记等渲染区域时，对应渲染必须立即让位给源码。
- 用户不应被只读装饰遮挡，导致无法精确编辑。
- 原子操作示例：[`operations/rendering_cursor_reveal.md`](./operations/rendering_cursor_reveal.md)
- 细粒度操作示例：[`operations/rendering_inline_source_reveal.md`](./operations/rendering_inline_source_reveal.md)

### 3. 数学公式

- AI chat 消息体支持 `$...$` 行内公式与 `$$...$$` 块公式的 KaTeX 展示；其他非主编辑器 HTML 渲染路径不因此自动承诺公式渲染。
- 支持行内公式与块级公式。
- 用户输入 LaTeX 时可以看到正确渲染结果。
- 公式块在编辑与阅读之间切换时，不应破坏源码。
- 原子操作示例：[`operations/rendering_math_mermaid.md`](./operations/rendering_math_mermaid.md)
- 细粒度操作示例：[`operations/rendering_math_source_projection.md`](./operations/rendering_math_source_projection.md), [`operations/rendering_projection_refresh.md`](./operations/rendering_projection_refresh.md)

### 4. Mermaid 图表

- 只支持 fenced `mermaid` code block。
- ` ```mermaid ` 代码块应渲染成图表。
- 图表大小与源码块高度保持可预测关系。
- 用户进入源码区域时，必须能继续编辑原 Mermaid 文本。
- 原子操作示例：[`operations/rendering_math_mermaid.md`](./operations/rendering_math_mermaid.md)
- 细粒度操作示例：[`operations/rendering_mermaid_source_projection.md`](./operations/rendering_mermaid_source_projection.md), [`operations/rendering_projection_refresh.md`](./operations/rendering_projection_refresh.md)

### 5. 任务列表与 Frontmatter

- 任务列表复选框可点击，点击结果会回写到源码。
- Frontmatter 具有明显的视觉边界，但光标进入后应还原为标准 YAML 源码。
- 细粒度操作示例：[`operations/rendering_checkbox_writeback.md`](./operations/rendering_checkbox_writeback.md), [`operations/rendering_inline_source_reveal.md`](./operations/rendering_inline_source_reveal.md)

### 6. Outline

- Outline 语法支持是轻量子集。
- Outline 反映标题层级。
- Outline 应识别空与非空 ATX 标题、tab 分隔标题，并剥离可选 closing `#` 序列。
- Outline 不应错误解释非支持语法。
- 点击 Outline 项后，编辑区应跳转到对应标题位置；Outline 项应使用可键盘触发的按钮语义。
- 细粒度操作示例：[`operations/rendering_outline_navigation.md`](./operations/rendering_outline_navigation.md)

### 7. 链接激活

- 默认状态下链接不应误触跳转。
- 仅在按住 `Ctrl/Cmd` 时，链接才转为可点击状态。
- 细粒度操作示例：[`operations/rendering_link_activation_gate.md`](./operations/rendering_link_activation_gate.md)

### 8. 长文档体验

- 本功能篇只验收批量应用与渐进调度基础设施，不宣称完整 virtual rendering。
- 打开长文档时，用户应先看到首屏内容。
- 剩余内容可以渐进加载，但编辑区不应卡死。
- 在预加载完成前，全文搜索等重操作可以被限制或延后。
- 细粒度操作示例：[`operations/rendering_large_doc_prefetch.md`](./operations/rendering_large_doc_prefetch.md), [`operations/rendering_large_doc_search_gate.md`](./operations/rendering_large_doc_search_gate.md)

## 细粒度操作链

- 行内源码揭示：[`operations/rendering_inline_source_reveal.md`](./operations/rendering_inline_source_reveal.md)
- 投影刷新：[`operations/rendering_projection_refresh.md`](./operations/rendering_projection_refresh.md)
- 数学源码投影：[`operations/rendering_math_source_projection.md`](./operations/rendering_math_source_projection.md)
- Mermaid 源码投影：[`operations/rendering_mermaid_source_projection.md`](./operations/rendering_mermaid_source_projection.md)
- 任务列表源码回写：[`operations/rendering_checkbox_writeback.md`](./operations/rendering_checkbox_writeback.md)
- Outline 跳转：[`operations/rendering_outline_navigation.md`](./operations/rendering_outline_navigation.md)
- 链接激活闸门：[`operations/rendering_link_activation_gate.md`](./operations/rendering_link_activation_gate.md)
- 大文档渐进预加载：[`operations/rendering_large_doc_prefetch.md`](./operations/rendering_large_doc_prefetch.md)
- 大文档搜索闸门：[`operations/rendering_large_doc_search_gate.md`](./operations/rendering_large_doc_search_gate.md)

## 非目标

- 当前阶段不把富文本编辑作为主编辑体验。
- 当前阶段不要求用户通过复杂模式切换才能完成常规 Markdown 编辑。
- 不支持把非标准高亮语法 `==...==` 当作正式渲染能力。

## Chrome MCP 验收实例

### RENDER-FEAT-01: 基础编辑与 Cursor Reveal

前置条件：

- 打开一篇包含标题、强调、引用、任务列表和 Frontmatter 的 Markdown 文档。

步骤：

1. 将光标移动到标题与强调文本区域。
2. 将光标移入 Frontmatter。
3. 观察渲染是否让位给源码。

期望结果：

- 光标进入对应范围时，源码立即可见。
- 用户可以直接编辑，而不是被装饰层阻挡。

### RENDER-FEAT-02: 数学公式与 Mermaid

前置条件：

- 文档中包含行内公式、块级公式和 Mermaid 代码块。

步骤：

1. 打开文档并观察公式和图表渲染。
2. 点击或移动光标进入对应源码区域。
3. 修改少量 LaTeX 或 Mermaid 文本。

期望结果：

- 阅读时显示渲染结果。
- 编辑时显示真实源码。
- 修改后结果与源码保持一致。

### RENDER-FEAT-03: Outline 与链接激活

前置条件：

- 文档包含多个标题和至少一个链接。

步骤：

1. 打开 Outline。
2. 点击某个标题项。
3. 观察编辑区是否定位。
4. 普通点击链接一次。
5. 按住 `Ctrl/Cmd` 再点击链接。

期望结果：

- Outline 能正确跳转。
- 普通点击不误跳转。
- `Ctrl/Cmd` 激活后链接可跳转。
