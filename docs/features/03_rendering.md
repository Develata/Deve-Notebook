# 03_rendering.md - Markdown 体验篇

本章描述 Markdown 编辑与阅读体验，即用户在编辑区、Outline 和相关渲染区域里实际能看到和操作到的行为。

## 功能目标

用户应获得一个 **Source-first** 的 Markdown 工作流：

- 平时面对的是可控、可预测的源码编辑体验
- 阅读时可以享受公式、图表、任务列表等增强渲染
- 光标进入时永远能回到真实源码，不被“富文本假象”困住
- 编辑块级公式、Mermaid 或表格时，可以在源码下方同时看到最新的伴随预览

## 验收边界

本功能篇只把下列用户可见行为纳入验收：

- 主编辑器默认 source-first；增强渲染只能作为源码之上的视觉投影存在。
- Markdown 正文采用论文/公文风格的内容字体栈：英文优先 Times New Roman，中文优先仿宋系列；平台缺少专有字体时使用可用的 CJK serif fallback。应用按钮、菜单等壳层字体不随之改变。
- Markdown 显示必须区分三种模式：纯源码模式、混合模式、禁止编辑的纯 preview 模式。
- 纯源码模式显示完整 Markdown 源码，但标题行仍保留标题字号和行高。
- 混合模式中，鼠标指针、光标或选区所在行及其关联块显示源码；非活动区域参与渲染，可隐藏 `#` 等 Markdown 标记。
- 混合模式中，折叠的主光标进入块级公式、Mermaid 或表格时，源码下方显示一个伴随预览；框选源码时只保留源码，多选区也只允许主光标产生一个伴随预览。
- 纯 preview 模式是只读阅读模式，参考 Obsidian reading mode，显示渲染结果，不显示 Markdown 源码。
- 标题、强调、引用、链接、frontmatter 等语法标记可以在非编辑焦点下隐藏或美化，但光标进入时必须显示真实源码。
- `**strong**` 内容必须在 Android WebView 与其它端都形成可辨识的粗体；不能只依赖 OEM 字体可能不提供的匿名 `700` 字重。光标 reveal 仍只恢复标记，不得改变源码。
- ATX 标题（`#` 到 `######`）的标题正文和空标题行都必须保持对应标题层级的整行视觉高度；隐藏 `#` 标记时不得退回普通段落行高。
- 空与非空 ATX 标题行（如 `#`、`# 标题`、`## 标题`、`### 标题`）在主编辑器与辅助 HTML Markdown 展示中应按 h1/h2/h3 层级保持可区分字号与高度；标题内的行内公式不应让整行回落到正文行高。
- `# s`、`## s`、`### s` 这类标准 Markdown 标题行，在纯源码、混合与 preview 三种模式下都不得退回正文行高；混合模式 active 行显示源码时也必须保持标题层级行高。
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
- 纯源码模式不隐藏 Markdown 标记；但块级标题、引用、代码块等行级 projection 仍可承担可读性样式。
- 原子操作示例：[`operations/doc_edit_confirmed_op.md`](./operations/doc_edit_confirmed_op.md)

### 2. Cursor Reveal

- 当光标进入公式、Frontmatter、强调、引用、列表标记等渲染区域时，对应渲染必须立即让位给源码。
- 用户不应被只读装饰遮挡，导致无法精确编辑。
- Cursor reveal 不应改变标题行的行高；它只控制源码标记是否显示。
- 块级公式、Mermaid 和表格在主光标编辑时，源码仍是主层，紧邻下方的预览只负责反馈当前源码结果，不应抢走光标或点击行为。
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
- 编辑 Mermaid 时，伴随预览在停止输入约 200 ms 后刷新，并且不得用迟到结果覆盖更新内容。
- 原子操作示例：[`operations/rendering_math_mermaid.md`](./operations/rendering_math_mermaid.md)
- 细粒度操作示例：[`operations/rendering_mermaid_source_projection.md`](./operations/rendering_mermaid_source_projection.md), [`operations/rendering_projection_refresh.md`](./operations/rendering_projection_refresh.md)

### 4.1 活动块伴随预览

- 块级公式与表格在源码编辑后立即刷新伴随预览；Mermaid 使用短 debounce 保持输入流畅。
- 预览使用编辑器现有主题和内容宽度；表格在窄屏时只在预览内部横向滚动。
- 公式、图表或表格过大时，伴随预览显示已暂停状态，源码编辑不受影响。
- 单个预览失败不得隐藏、替换或损坏用户正在编辑的源码。

### 5. 任务列表与 Frontmatter

- 任务列表复选框可点击，点击结果会回写到源码。
- 任务项只显示 checkbox，不得同时叠加普通列表圆点；移动工具栏创建任务后，第一次 Enter 继续下一任务，第二次 Enter 立即退出并留下普通空行。
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

### 7.1 移动任务项输入

- 点击移动工具栏的任务项按钮后，第一次按 Enter 应继续生成下一条空任务项，而不是删除当前行。
- 再次在未填写的新空任务项上按 Enter 时，应立即退出列表，不得残留移动到下一行的任务标记，
  也不得要求第三次 Enter。
- 该行为不得向文档写入不可见字符，也不得改变键盘输入创建的普通空任务项行为。

### 8. 长文档体验

- 本功能篇只验收批量应用与渐进调度基础设施，不宣称完整 virtual rendering。
- 打开长文档时，用户应先看到首屏内容。
- 剩余内容可以渐进加载，但编辑区不应卡死。
- 在预加载完成前，全文搜索等重操作可以被限制或延后。
- snapshot 或 replay 无法原子应用时，编辑器保持只读并显示可诊断错误；初始 snapshot
  自动重试最多一次，之后必须由用户点击 Retry 生成新的打开请求，不能永久 Loading 或无限重试。
- backend 生成 UTF-16 patch 时，超出 wire 位置范围会明确失败，不会把内容差异静默投影为空 patch。
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
- 若光标进入 `# s` 标题行，源码标记显示，但该行仍保持 h1 行高。

### RENDER-FEAT-01B: Markdown 三模式标题行高

前置条件：

- 文档包含 `#`、`# s`、`## s`、`### s`。

步骤：

1. 切到纯源码模式。
2. 观察四个标题行。
3. 切到混合模式，并把光标放入 `# s`。
4. 移出光标，让 `# s` 回到非活动渲染状态。
5. 切到禁止编辑的纯 preview 模式。

期望结果：

- 三种模式下 `#`、`# s`、`## s`、`### s` 都保持对应标题层级行高。
- 混合模式 active 行显示源码，但不退回正文行高。
- preview 模式不显示 Markdown 源码，不允许编辑。

### RENDER-FEAT-02: 数学公式、Mermaid 与表格伴随预览

前置条件：

- 文档中包含行内公式、块级公式、Mermaid 代码块和 Markdown 表格。

步骤：

1. 打开文档并观察公式和图表渲染。
2. 点击或移动光标进入对应源码区域。
3. 修改少量 LaTeX、Mermaid 和表格文本。
4. 框选整段源码，再测试多光标与 night 主题。

期望结果：

- 阅读时显示渲染结果。
- 编辑块级对象时显示真实源码及其下方唯一的伴随预览；框选时只显示源码。
- 数学与表格立即刷新，Mermaid 在停止输入约 200 ms 后只显示最新结果。
- night 主题与窄屏表格保持可读，光标移动和主题切换不产生文档写入。

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
