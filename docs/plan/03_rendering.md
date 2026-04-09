# 03_rendering.md - 渲染工程蓝图

本章只定义 Markdown 渲染与编辑投影的工程实现，不描述用户功能文案。功能语义见 [../features/03_rendering.md](../features/03_rendering.md)，自动化验收见 [../acceptance-cases/03_rendering.md](../acceptance-cases/03_rendering.md)。

## 1. 目标

- 保持 `Source-First`：源码文本始终是唯一可编辑真相。
- 所有可见渲染都是 projection，不得成为第二真相。
- Web / Desktop / Mobile 共享同一文档投影规则，只允许壳层适配不同。

## 2. 权威实体

- `Document Text`
  - 权威文本内容，来自 ledger/document runtime。
- `Selection / Cursor`
  - 编辑态局部状态，用于驱动 reveal/hide。
- `Render Projection`
  - 从文本派生出的装饰、widget、outline、preview token。
- `View Adapter`
  - CodeMirror / shell 对 projection 的平台呈现层。

## 3. 分层

### 3.1 Authority

- 文本内容与写入确认由 document runtime 控制。
- 渲染层不得直接写入 ledger、repo state 或 source control state。

### 3.2 Projection

- Markdown parse、syntax decoration、outline、math/diagram widget 都属于 projection。
- projection 必须可重建，不得持久化为业务真相。

### 3.3 View

- 视图只消费 projection 和 control surface。
- 显示层不得直接修改文档真值，只能发出明确 control/intent。

## 4. 运行时管线

1. document runtime 提供当前文档文本与选择状态。
2. rendering runtime 对文本生成 syntax tree / projection fragments。
3. projection 输出：
   - inline / block decorations
   - math / mermaid widgets
   - task-list control mapping
   - outline model
   - link activation hints
4. platform adapter 把 projection 映射到具体视图。

## 5. 状态机

### 5.1 文档渲染状态

- `Empty`
- `SourceReady`
- `Projected`
- `RevealActive`
- `Degraded`

### 5.2 转换规则

- `OpenDoc -> SourceReady`
- `ProjectionBuilt -> Projected`
- `CursorTouchesRenderedRange -> RevealActive`
- `CursorLeavesRenderedRange -> Projected`
- `RendererUnavailable / ParseGuardTriggered -> Degraded`

## 6. 核心合同

### 6.1 Cursor Reveal

- 当光标进入 projection 覆盖的源码范围时，projection 必须立即让位。
- reveal 必须由统一 runtime 决策，不能由每个控件各自实现一套规则。

### 6.2 Widget 写回

- task list、link activation 等交互只能通过 document control surface 回写源码。
- widget 不得直接修改 DOM 并假装文档已变更。

### 6.3 Preview 边界

- preview 是 projection，不是独立文档模型。
- 不允许维护第二份富文本权威状态。

### 6.4 Outline 边界

- outline 来源于标题 projection。
- outline 点击只触发定位 control，不得直接篡改 editor 内部状态缓存。

## 7. 失败合同

- 公式、Mermaid 或其他 renderer 失败时，必须回退到源码可见态。
- projection 失败不得阻塞文档打开。
- 大文档预渲染失败时，允许退回纯 source view，但不得让文档不可编辑。

## 8. 禁止事项

- 禁止显示层维护独立的“富文本文档真相”。
- 禁止 link、checkbox、outline 直接跨层修改 repo / source control 状态。
- 禁止 renderer 失败时吞掉源码。
- 禁止把未支持语法静默升级为第一类渲染能力。

## 9. 代码边界

- `apps/web/src/editor/`
  - editor adapter 与文档视图整合。
- `apps/web/js/extensions/`
  - CodeMirror projection / widget 实现。
- `apps/web/src/components/outline_render/`
  - outline projection 与显示。
- `apps/web/src/hooks/use_core/`
  - 只提供 document runtime state，不直接承载渲染细节。
