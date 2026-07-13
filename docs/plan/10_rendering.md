# 10_rendering.md - Markdown Rendering 与 Editor Projection 工程蓝图

## Metadata

- `Layer`: `Application / UI Shell`
- `Status`: `Current UI Contract`
- `Version`: `0.0.1`
- `Last Review`: `2026-07-12`
- `Counterpart Feature`: `docs/features/03_rendering.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/03_rendering.md`
- `Primary Code Areas`: `apps/web/src/editor/`, `apps/web/js/extensions/`, `apps/web/src/components/outline_render/`, `apps/cli/src/server/handlers/document/`

## 1. Scope

本章定义编辑器输入、解析、渲染、outline 与 widget projection 的工程实现合同。

本章回答：

1. 文档 authority 如何进入编辑器与渲染层。
2. Source-first、Hybrid enhancement、Preview projection 如何分层。
3. 数学公式、Mermaid、Frontmatter、Task List、Code Block 等 widget 如何作为 projection 存在，而不是第二真值。

按钮入口、用户主观体验与 Chrome MCP 路径属于 `docs/features/03_rendering.md`。

## 1.1 Rendering Capability Boundary {#current-rendering-split}

本章按两层目标理解：

- **Baseline contract**：进入主线验收前必须具备稳定测试、验收脚本或可复现手工路径。
- **Extended target**：作为后续工程目标保留；未绑定验收前不得作为发布阻塞项。

当前主编辑路径必须保持三层分工：

- **Source-first 主编辑器**：以 CodeMirror 编辑源文本，并通过 decoration/widget 提供局部增强；widget 不得成为第二真值。
- **Rust/WASM editor runtime**：只负责 snapshot、history、live op、pending overlay、read-only gate 与批量调度；不得把 projection 当作 ledger authority 写回。
- **辅助 Markdown-to-HTML 渲染器**：只服务聊天、只读摘要或辅助 HTML 区域；AI chat / read-only HTML message body 可执行 KaTeX post-render math projection，但不得被视为主编辑器 hybrid engine。
- **Editor undo / redo**：只属于当前 WebLightPeer editor session 的 CodeMirror edit history；不得解释为 ledger 回滚、source-control 回滚、repo switch 回滚或远端 peer 已提交事实撤销。
- 远程 snapshot、history replay、live op 与批量 remote op **MUST NOT** 进入本地 CodeMirror undo stack；本地撤销/重做只能重放用户在当前可写 editor session 内产生的编辑事务。

以下能力属于 extended target；只有在对应实现、测试与验收同步补齐后，才可进入 baseline contract：

- 独立 Preview Projection / Live Preview / Milkdown。
- 通用富文本编辑或所见即所得 authority。
- 任意 HTML、`==highlight==`、emoji shortcode、footnote、wikilink 的完整语义支持。
- 真正跨超大文档的完整 virtual rendering；批量应用/渐进调度基础设施不得被等同于完整虚拟渲染引擎。
- rendering settings 的完整 GUI 持久化。

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

工程上必须区分三种 Markdown 显示模式：

- `Source Projection`
- `Hybrid Decoration`
- `Preview Projection`

其中：

- `Source Projection` 是真实可编辑主视图，显示完整 Markdown 源码；但 ATX 标题行仍按标题层级应用整行字号与行高。
- `Hybrid Decoration` 是混合模式：光标/选区所在行及其关联块显示源码；非活动区域可隐藏 `#`、强调、链接等语法标记并显示投影样式。它只是对 source 的 decoration / reveal 规则，不是独立 authority。
- `Preview Projection` 是只读阅读投影，参考 Obsidian reading mode：显示渲染结果，不显示 Markdown 源码，不允许编辑，不得成为写入真值源。

### 2.4 Editor Kernel Stack

- Input Layer
  - CodeMirror 6
  - CodeMirror history / history keymap for editor-scoped undo and redo
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

任一 adapter/replay 步骤失败时必须原子停止并进入 `EditorSyncError`：编辑器保持只读，
本地 version/history 不得推进到失败 batch 之后，pending overlay 不得重发。初始 snapshot
adapter 写入失败最多自动 reopen 一次；再次失败显示诊断与显式 Retry，Retry 必须使用新的
generation/request。

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
- ATX h1/h2/h3 在主编辑器中必须作为同一行级 projection 应用字号、粗细与行高；空标题与非空标题不得因为语法标记显示/隐藏而回落到正文尺寸，也不得叠加双倍缩放。
- `#`、`# s`、`## s`、`### s` 等空与非空 ATX 标题在 Source Projection、Hybrid Decoration 与 Preview Projection 三种模式下都必须保持对应标题层级的行高。
- Hybrid Decoration 的 active 行显示源码时仍必须保留标题级行高；光标/选区只控制语法标记 reveal，不得把标题行压回正文 line box。
- 标题行内包含 inline math 时，math widget/reveal 范围不得取消该行的 heading line projection；只有标题 opener 本身位于 math/code/frontmatter 范围内时才可拒绝标题行级样式。

## 4. Parsing Contract

### 4.1 Block Parsing Order

块级解析优先级 **MUST** 为：

1. fenced code
2. block math
3. html block allowlist
4. headings / list / quote / table / hr / paragraph

### 4.2 Inline Parsing Order

行内解析优先级 **MUST** 为：

1. inline code
2. escaping
3. inline math
4. autolink
5. link / image containers
6. emphasis / italic / strike

### 4.3 Whitelist Rule {#markdown-render-whitelist}

rendering 层只能实现 plan 明确允许的 Markdown 子集。

明确禁止把非标准语法 silently 视为已支持真值能力，例如：

- `==highlight==` 不进入 baseline contract；只能按 extended target 单独规划
- 任意 HTML 不得当成通用渲染通道

辅助 Markdown-to-HTML 渲染器的 math projection 只允许识别 `$...$` 与 `$$...$$`。该 projection 必须跳过 `pre` / `code` 等源码区域；KaTeX 不可用或单个公式解析失败时必须保留可见文本，不得中断消息渲染。

### 4.4 Baseline Syntax Whitelist

Baseline contract 的块级支持集合：

- headings
- paragraphs
- blockquotes
- lists
- fenced code
- tables
- horizontal rules
- math blocks
- mermaid fenced blocks
- limited HTML block allowlist（如 `<br>`）

Baseline contract 的行内支持集合：

- code span
- math
- links / autolinks
- images
- emphasis / italic / strikethrough
- escaping

### 4.5 Extended Syntax Target

以下语法属于 extended target，未绑定测试与验收前 **MAY** 保留在蓝图中，但 **MUST NOT** 作为 baseline contract 或发布阻塞项：

- callouts / admonitions（`> [!NOTE]` 等）
- indented code
- footnote definitions / refs
- wikilinks
- emoji shortcode
- `==highlight==`
- 完整 preview projection / Live Preview / Milkdown

验收只能覆盖 `1.1 Rendering Capability Boundary` 中定义的可证明范围。extended target 进入 baseline 前，必须同步补齐实现、测试、feature 行为说明与 acceptance case。

## 5. Source-First Contract

### 5.1 Cursor Reveal

- 光标进入 widget 的源码范围时，render projection **MUST** 让位给源码。
- 在 Hybrid Decoration 中，光标/选区所在行及关联块让位给源码；非活动行继续按渲染投影显示。
- Cursor reveal 只改变标记可见性与 widget/source 切换，不得取消标题、引用、代码块等块级行高 projection。
- 适用范围：
  - math
  - mermaid
  - emphasis syntax
  - frontmatter
  - link source

### 5.2 Link Activation {#link-activation-gate}

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
- math widget 应优先使用 KaTeX；KaTeX 不可用时必须降级显示源码。
- AI chat / read-only HTML message body 的 TeX 展示是 post-render projection，不提供 cursor reveal，不得扩展为 Mermaid、完整 preview 或富文本 authority。

补充：

- `$...$` 仅在紧邻非空字符时进入公式识别。
- math strict warning 只影响单个 widget，不得中断整篇文档编辑。

### 6.2 Mermaid

- mermaid 只通过 fenced code block 进入渲染。
- 图形容器高度与源码块边界必须一致，不允许脱离源码占位。
- Mermaid 必须静态打包；光标进入源码范围时必须恢复源码显示。

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
- 若未来启用 callout/admonition，其视觉类型（`NOTE`/`TIP`/`CAUTION`）只能来自源码第一行解析，不得在 UI 层凭颜色或 label 猜测。

### 6.4 Code Block Toolbar Contract {#code-block-toolbar-contract}

- 代码块右上角必须有：
  - `Copy`
  - `Ellipsis (...)`
- `Ellipsis` 打开的是可扩展菜单，不是写死逻辑。
- 若未注册 action，允许显示空状态，但不得报错中断渲染。
- 轻量 HTML 渲染器只承担 wrapper 与可选 apply button 语义；完整 toolbar 属于 CodeMirror adapter 路径，不应混为同一实现。

### 6.5 Outline Projection {#outline-projection}

- outline 必须从解析后的 heading projection 导出。
- outline 渲染不得发明新语义。
- 不受支持的 inline syntax 在 outline 中必须按普通文本保留。
- outline heading scan 只按轻量解析器处理：跳过 fenced code，支持 heading 层级与 inline code/math/strong/em/del projection；不支持的 `==highlight==` 保留为普通文本。
- outline heading scan 的 ATX 标题识别必须与主编辑器 baseline 保持一致：支持 `#` 空标题、space/tab 分隔、可选 closing `#` 序列剥离；fenced code 内的 `#` 与四空格缩进代码样式行不得进入 outline。

### 6.6 Hybrid / Frontmatter / Preview Status

- `Hybrid View`
  - 是 source projection 上的 reveal/hide decoration 规则。
  - 光标进入标题标记、强调标记、frontmatter 边界、link source、math source 时，必须恢复源码可见。
- `Frontmatter Support`
  - 仅支持 YAML frontmatter。
  - frontmatter 内容保持普通文本 authority；render 层只允许样式化和折叠边界，不得赋予系统配置语义。
- `Live Preview / Milkdown`
  - 属于推迟 / 可选的 preview engine。
  - 当前工程蓝图允许其作为派生 projection 存在，但不得替代 source-first 主编辑链。
  - 任何 rich-text preview engine 接入都必须继续服从 confirmed+pending buffer 单一真值。

## 7. Large Document Strategy {#large-document-runtime}

- 首屏优先
- virtual render
- progressive prefetch
- search gate

工程约束：

- “未完全预加载前禁用全文搜索”是 runtime gate，不是 view 层提示文字而已。
- snapshot-first / replay 策略必须与 `09_web_thin_client_ledger.md` 保持一致。

补充：

- first paint 优先首屏 + 预缓冲区
- 文本可完整加载，但渲染层只虚拟可视区
- UTF-16 index cache 可作为定位优化，但不是第二真值

阶段边界：

- 批量应用与动态 batch size 调度只属于大文档 runtime 基础设施。
- 在完整 virtual render、search gate 与索引缓存验收补齐前，不得宣称大文档策略已完整落地。

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
- CodeMirror remote batch 必须先按逐步变化后的虚拟 UTF-16 文档长度验证并构造全部
  transaction specs；任意非法 op 时不得 dispatch。成功时只允许一次 sequential dispatch，
  且 remote transaction 不得进入本地 undo history。
- pending/history/live replay 或 editor content readback 失败时必须保持只读并进入结构化
  `EditorSyncError`；不得进入 Ready、不得重发 pending、不得留下已应用前缀。
- CodeMirror adapter 的 mount 与 cleanup 必须绑定到精确的 editor host owner。新 mount
  发布为 active view 前必须退休旧 view；迟到的旧组件 cleanup 只能销毁自己拥有的 view，
  owner 不匹配时必须 no-op，不得清空新 mount 的 bridge/queue。`editorBridgeReady`、active
  view 与可写状态必须属于同一 host。每个新 mount 都是新的 projection load session，
  必须先撤销旧 `Ready` 并保持只读，直到该 mount 自己的 OpenDoc/Snapshot/history 完成；
  不得把空白 editor 暴露为短暂可写。当前 host 首次进入可写后的第一笔真实输入不得因旧
  cleanup 迟到而丢失。mount 前置构造失败或 view teardown 抛错时也必须清空 active
  view、callback 与 bootstrap owner，保持 fail-closed，不得保留半活跃 bridge。已脱离
  DOM 的迟到 host 必须在退休当前 view 之前被拒绝，不得接管 active owner，也不得通过
  全局 readonly adapter 修改当前 owner 的可写状态。
- 单个 widget 渲染失败时，必须回退为源码显示，不得破坏整篇文档编辑。
- outline projection 失败时，只允许降级为 plain heading text，不得阻断文档编辑主链。

## 11. Forbidden Patterns

- 把 Preview 当成新的 authority buffer。
- 让 widget 直接改业务真值，绕过源码 delta。
- 在显示层自己推断 pending/confirmed。
- 让 outline 渲染不受支持语法并假装是规范能力。
- 通过任意 HTML 绕开 whitelist。

## 12. Runtime Boundary

### 12.1 Editor Runtime

职责：

- open doc
- delta input
- snapshot/history/newop apply
- pending overlay bridge

### 12.2 JS Adapter / Widget Projection

职责：

- CodeMirror adapter
- hybrid decoration
- math / mermaid / frontmatter parser
- code toolbar/menu

Browser globals exposed for editor/widget interoperability **MUST** be registered through
`web_bridge_registry.js`. The registry is admission control for projection-only bridge
facades: every entry MUST declare `authority: "none"` and a runtime in
`render_projection_runtime`, `widget_bridge_runtime`, or `native_shell_mode_runtime`.
Entry names, sources, and roles claiming ledger, pending, ack/reject, write-success,
Source Control, staging, commit anchor, Git mirror, backup, or remote-projection
authority MUST fail closed before they are written onto `window`.

### 12.3 Outline Runtime

职责：

- outline parsing
- supported syntax projection
- plain-text fallback

### 12.4 Authority Bridge {#document-authority-bridge}

职责：

- snapshot / history / ack / reject contract

## 13. Refactor Target

长期应显式拆成四个子系统：

- `document_runtime`
- `render_projection_runtime`
- `widget_bridge_runtime`
- `outline_projection_runtime`

上述四层必须分离；editor hook / widget 文件不得继续聚合 runtime、adapter 与 outline projection 行为。

## 本章相关命令

- 无

## 本章相关配置

- `rendering.engine`
- `rendering.font_family_mono`
- `rendering.font_family_serif`
