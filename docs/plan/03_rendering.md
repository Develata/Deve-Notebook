# 03_rendering.md - Markdown Rendering 与 Editor Projection 工程蓝图

## 1. Scope

本章定义编辑器输入、解析、渲染、outline 与 widget projection 的工程实现合同。

本章回答：

1. 文档 authority 如何进入编辑器与渲染层。
2. Source-first、Hybrid enhancement、Preview projection 如何分层。
3. 数学公式、Mermaid、Frontmatter、Task List、Code Block 等 widget 如何作为 projection 存在，而不是第二真值。

按钮入口、用户主观体验与 Chrome MCP 路径属于 `docs/features/03_rendering.md`。

## 2. Authoritative Entities

### 2.1 Document Authority

- `L_confirmed`：服务端已确认 ledger 状态
- `O_pending`：当前会话未确认 overlay
- `V_editor`：编辑器当前可见状态

必须满足：

```text
V_editor = Project(L_confirmed) + O_pending
```

### 2.2 Rendering Entities

- `EditorBuffer`
- `ParserArtifact`
- `RenderProjection`
- `OutlineProjection`
- `DecorationRange`
- `WidgetInstance`
- `OpenDocSnapshot`

### 2.3 Mode Semantics

工程上必须区分：

- `Source Projection`
- `Hybrid Decoration`
- `Preview Projection`

其中：

- `Source Projection` 是真实可编辑主视图。
- `Hybrid` 只是对 source 的 decoration / reveal 规则，不是独立 authority。
- `Preview Projection` 是派生阅读投影，不得成为写入真值源。

### 2.4 Editor Kernel Stack

- Input Layer
  - CodeMirror 6
- State Layer
  - confirmed + pending document projection
- Projection Layer
  - source / hybrid / preview / outline / widgets
- Rich Extension Layer
  - math / mermaid / code toolbar / frontmatter / task list

## 3. Runtime Pipeline

### 3.1 Open Document

```text
OpenDoc
  -> SnapshotLoading
  -> SnapshotApplied
  -> HistoryReplay
  -> EditorBufferReady
  -> RenderProjectionReady
```

### 3.2 Live Editing

```text
InputDelta
  -> PendingOverlay
  -> Acked | Rejected
  -> RebuildEditorProjection
  -> RebuildOutlineProjection
```

### 3.3 Render Refresh

```text
EditorBufferChanged
  -> ParseBlocks
  -> ParseInlines
  -> BuildDecorations
  -> BuildWidgets
  -> PublishProjection
```

约束：

- render refresh 只能消费 editor projection，不得直接改 ledger authority。
- outline、widget、preview 必须基于同一份 confirmed+pending buffer 构建。

## 4. Parsing Contract

### 4.1 Block Parsing Order

块级解析优先级 MUST 为：

1. fenced code
2. block math
3. html block allowlist
4. headings / list / quote / table / hr / paragraph

### 4.2 Inline Parsing Order

行内解析优先级 MUST 为：

1. inline code
2. escaping
3. inline math
4. autolink
5. link / image containers
6. emphasis / italic / strike

### 4.3 Whitelist Rule

rendering 层只能实现 plan 明确允许的 Markdown 子集。

明确禁止把非标准语法 silently 视为已支持真值能力，例如：

- `==highlight==` 不进入受支持语义集合
- 任意 HTML 不得当成通用渲染通道

### 4.4 Syntax Whitelist

块级支持至少包含：

- headings
- paragraphs
- blockquotes
- callouts / admonitions（`> [!NOTE]` 等）
- lists
- fenced code
- indented code
- tables
- horizontal rules
- math blocks
- mermaid fenced blocks
- footnote definitions
- limited HTML block allowlist（如 `<br>`）

行内支持至少包含：

- code span
- math
- links / autolinks / wikilinks
- images
- emphasis / italic / strikethrough
- emoji shortcode
- footnote refs
- escaping

## 5. Source-First Contract

### 5.1 Cursor Reveal

- 光标进入 widget 的源码范围时，render projection MUST 让位给源码。
- 适用范围：
  - math
  - mermaid
  - emphasis syntax
  - frontmatter
  - link source

### 5.2 Link Activation

- 默认链接不可直接点击。
- 只有 Ctrl/Cmd 激活态下才可点击。
- 该行为属于 render interaction contract，不得由每个 link 节点单独绑业务事件。

实现约束：

- Rust/WASM 负责监听全局 keydown/keyup，并更新一个全局激活态。
- CSS / decoration 负责视觉反馈。
- 禁止为每个 link node 逐个绑定昂贵的业务监听器。

### 5.3 Widget Authority

- Checkbox 点击可以生成源码变更。
- 但 widget 本身不是 authority；它只能经 editor delta 路径回到源码。

## 6. Rendering Capabilities

### 6.1 Math

- engine 可为 KaTeX / MathJax，但接口语义统一。
- block / inline math 必须保持 source-first reveal。
- 复制公式时应优先保留源码语义。

补充：

- `$...$` 仅在紧邻非空字符时进入公式识别。
- math strict warning 只影响单个 widget，不得中断整篇文档编辑。

### 6.2 Mermaid

- mermaid 只通过 fenced code block 进入渲染。
- 图形容器高度与源码块边界必须一致，不允许脱离源码占位。

补充：

- mermaid 渲染必须静态打包，无运行时网络依赖。
- zoom/scale 应通过源码块高度或容器约束完成，不得脱离源码边界悬浮。

### 6.3 Rich Markdown Widgets

- task list
- table
- frontmatter styling
- list marker decoration
- inline image
- code block toolbar

这些都属于 projection widgets，不得直接写 authority state。

补充：

- checkbox 点击是例外的“交互式 widget”，但它也只能经源码 delta 回写。
- blockquote / code block 背景样式是持久化 decoration，不因 cursor reveal 整块移除。
- frontmatter 在光标离开时可隐藏 `---` 边界，但内容区仍由源码投影而来。
- callout/admonition 的视觉类型（`NOTE`/`TIP`/`CAUTION`）只能来自源码第一行解析，不得在 UI 层凭颜色或 label 猜测。

### 6.4 Code Block Toolbar Contract

- 代码块右上角必须有：
  - `Copy`
  - `Ellipsis (...)`
- `Ellipsis` 打开的是可扩展菜单，不是写死逻辑。
- 若未注册 action，允许显示空状态，但不得报错中断渲染。

### 6.5 Outline Projection

- outline 必须从解析后的 heading projection 导出。
- outline 渲染不得发明新语义。
- 不受支持的 inline syntax 在 outline 中必须按普通文本保留。

### 6.6 Hybrid / Frontmatter / Preview Status

- `Hybrid View`
  - 是 source projection 上的 reveal/hide decoration 规则。
  - 光标进入标题标记、强调标记、frontmatter 边界、link source、math source 时，必须恢复源码可见。
- `Frontmatter Support`
  - 仅支持 YAML frontmatter。
  - frontmatter 内容保持普通文本 authority；render 层只允许样式化和折叠边界，不得赋予系统配置语义。
- `Live Preview / Milkdown`
  - 属于 deferred / optional preview engine。
  - 当前工程蓝图允许其作为派生 projection 存在，但不得替代 source-first 主编辑链。
  - 任何 rich-text preview engine 接入都必须继续服从 confirmed+pending buffer 单一真值。

## 7. Large Document Strategy

- 首屏优先
- virtual render
- progressive prefetch
- search gate

工程约束：

- “未完全预加载前禁用全文搜索”是 runtime gate，不是 view 层提示文字而已。
- snapshot-first / replay 策略必须与 `16_web_thin_client_ledger.md` 保持一致。

补充：

- first paint 优先首屏 + 预缓冲区
- 文本可完整加载，但渲染层只虚拟可视区
- UTF-16 index cache 可作为定位优化，但不是第二真值

## 8. Commands / Inputs / Outputs

### 8.1 Inputs

- `OpenDoc`
- `ApplySnapshot`
- `ApplyHistory`
- `ApplyNewOp`
- `UpdateEditorSelection`
- `TogglePreviewProjection`
- `RequestOutlineRefresh`

### 8.2 Outputs

- `EditorProjectionReady`
- `OutlineProjectionReady`
- `WidgetDecorationSet`
- `OpenDocRenderReady`
- `RenderRejected`（仅结构化错误，不得返回裸字符串）

### 8.3 Rendering Settings Contract

- `rendering.engine`
  - `KaTeX | MathJax`
- `rendering.font_family_mono`
- `rendering.font_family_serif`

设置只能改变渲染实现或视觉，不得改变 authority 语义。

## 9. Failure Modes

- snapshot delta 越界
- widget parse failure
- math render warning / strict mode warning
- mermaid parse failure
- frontmatter parse ambiguity
- large document prefetch lag

补充：

- unsupported syntax encountered
- code toolbar action registry empty
- outline parser fallback

## 10. Recovery / Repair

- snapshot delta 不可应用时，必须回退到 full snapshot，而不是继续向前端推坏 batch。
- 单个 widget 渲染失败时，必须回退为源码显示，不得破坏整篇文档编辑。
- outline projection 失败时，只允许降级为 plain heading text，不得阻断文档编辑主链。

## 11. Forbidden Patterns

- 把 Preview 当成新的 authority buffer。
- 让 widget 直接改业务真值，绕过源码 delta。
- 在显示层自己推断 pending/confirmed。
- 让 outline 渲染不受支持语法并假装是规范能力。
- 通过任意 HTML 绕开 whitelist。

## 12. Module Boundary

### 12.1 Editor Runtime

- `apps/web/src/editor/`
- `apps/web/src/editor/sync/`

职责：

- open doc
- delta input
- snapshot/history/newop apply
- pending overlay bridge

### 12.2 JS Adapter / Widget Projection

- `apps/web/js/editor_adapter.js`
- `apps/web/js/extensions/`

职责：

- CodeMirror adapter
- hybrid decoration
- math / mermaid / frontmatter parser
- code toolbar/menu

### 12.3 Outline Runtime

- `apps/web/src/components/outline_render/`

职责：

- outline parsing
- supported syntax projection
- plain-text fallback

### 12.4 Authority Bridge

- `apps/cli/src/server/handlers/document/open.rs`
- `apps/cli/src/server/handlers/document/snapshot.rs`
- `apps/cli/src/server/handlers/document/edit.rs`
- `crates/core/src/protocol/server.rs`

职责：

- snapshot / history / ack / reject contract

## 13. Code Mapping

- editor runtime:
  - `apps/web/src/editor/mod.rs`
  - `apps/web/src/editor/delta_input.rs`
  - `apps/web/src/editor/sync/snapshot.rs`
  - `apps/web/src/editor/sync/history.rs`
  - `apps/web/src/editor/sync/live.rs`
- JS adapter:
  - `apps/web/js/editor_adapter.js`
  - `apps/web/js/extensions/hybrid.js`
  - `apps/web/js/extensions/math.js`
  - `apps/web/js/extensions/mermaid.js`
  - `apps/web/js/extensions/frontmatter_parser.js`
  - `apps/web/js/extensions/code_toolbar.js`
- outline:
  - `apps/web/src/components/outline_render/mod.rs`
  - `apps/web/src/components/outline_render/parse.rs`
  - `apps/web/src/components/outline_render/scan.rs`
- transport:
  - `apps/cli/src/server/handlers/document/open.rs`
  - `apps/cli/src/server/handlers/document/snapshot.rs`
  - `apps/cli/src/server/handlers/document/edit.rs`

## 14. Refactor Target

长期应显式拆成四个子系统：

- `document_runtime`
- `render_projection_runtime`
- `widget_bridge_runtime`
- `outline_projection_runtime`

当前代码已经有雏形，但 editor Rust runtime、JS adapter 与 outline projection 之间的边界还没有完全显式化。未来重构必须围绕这四层收敛，而不是继续把行为塞进单个 editor hook 或任意 widget 文件。

## 本章相关命令

- 无

## 本章相关配置

- `rendering.engine`
- `rendering.font_family_mono`
- `rendering.font_family_serif`
