# 08_ui_design.md - UI Shell 与 Application Control 工程蓝图

## Metadata

- `Layer`: `Application / UI Shell`
- `Status`: `Current UI Contract`
- `Counterpart Feature`: `docs/features/08_ui_design.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/05_ui.md`, `docs/acceptance-cases/13_ui_mobile_chat_regression.md`
- `Primary Code Areas`: `apps/web/src/components/`, `apps/web/src/hooks/use_core/callbacks*.rs`, `apps/web/src/hooks/use_core/navigation.rs`, `apps/web/src/components/mobile_layout/`

> **Modules**: [Web](./08_ui_design_01_web.md) | [Desktop](./08_ui_design_02_desktop.md) | [Mobile](./08_ui_design_03_mobile.md)

## 1. Scope

本章不是视觉说明书，而是 UI shell 的工程蓝图。

本章定义：

- 显示层、application control 层、runtime 层之间的依赖方向
- 多端共享控制接口
- shell / panel / drawer / overlay / command 的状态机
- 显示层的禁区

用户点击后看到什么、按钮文案与 Chrome MCP 手工实例属于 `docs/features/08_ui_design.md`。

## 2. Design Invariants

### 2.1 Thin View Rule

- 显示层 MUST 只负责展示与收集输入。
- 显示层 MUST NOT 直接改写 repo scope、document authority、pending write、source control side table。
- 所有业务操控必须经由 application control、runtime command 或 CLI / server command。

### 2.2 Command First

- 所有核心用户能力 MUST 可映射到稳定的 `CommandId`。
- 按钮、菜单、快捷键、移动端动作只是同一 command 的不同触发器。

### 2.3 Multi-Surface Consistency

- Web / Desktop / Mobile 共享同一 feature runtime 与 command 语义。
- 各端只允许在 shell、viewport、gesture、platform adapter 层不同。

### 2.4 Runtime Ownership

- shell state 归 UI shell runtime 所有。
- document state 归 document runtime 所有。
- source control state 归 source control runtime 所有。
- repo/branch state 归 scope runtime 所有。

禁止一个 view 组件同时持有多个 runtime 的私有真相副本。

### 2.5 Responsive and Multi-Surface Mapping

- Web 端在移动端视口 MUST 匹配 Mobile shell contract。
- Web 端在大屏视口 MUST 匹配 Desktop shell contract。
- Desktop / Mobile 外壳可以不同，但 command/control/runtime 语义必须一致。

## 3. Authoritative Entities

### 3.1 View Entities

- `ViewId`: Explorer / Search / SourceControl / Extensions / Chat / Settings / Diff / Dashboard
- `PanelId`
- `DrawerId`
- `OverlayId`
- `CommandId`

### 3.2 Layout Entities

- `SidebarWidth`
- `RightPanelWidth`
- `OuterGutterWidth`
- `ActivityBarPinState`
- `FocusedSurface`

这些都是 UI prefs，不是业务 authority。

### 3.3 Application Control

application control 是连接 view 与 runtime 的稳定接口层，至少包含：

- `OpenDocument`
- `SwitchRepo`
- `SwitchBranch`
- `RequestChanges`
- `StageTarget`
- `CommitStaged`
- `OpenDiff`
- `OpenCommandPalette`
- `ToggleSidebar`
- `ToggleDrawer`

控件只能发出这些控制意图，不能绕过它们直接修改 runtime 内部信号。

### 3.4 Layout Tokens and Layer Registry

- design tokens MUST 通过 CSS variables 暴露，禁止 view 组件散布硬编码色值当作跨模块契约。
- shell 分层至少保留以下 z-index registry：
  - `Editor`
  - `Chrome`
  - `Panels`
  - `Floating`
  - `Overlay`
  - `Modal`
- `Toast`
- 不同 feature view 不得私自发明更高层级绕过统一层级表。

最小 canonical token 集：

- source control semantic colors
  - `--color-added`
  - `--color-modified`
  - `--color-deleted`
- shell layer tokens
  - `--z-editor`
  - `--z-chrome`
  - `--z-panels`
  - `--z-floating`
  - `--z-overlay`
  - `--z-modal`
  - `--z-toast`

### 3.5 Iconography Contract

- iconography 必须是稳定的 shared registry，不得让各 feature 随意混用不同图标语义。
- 同一动作在多端应共享同一 icon id / semantic name，例如：
  - `repo`
  - `branch`
  - `source_control`
  - `search`
  - `settings`
  - `diff`
- 图标选择属于 shell design token contract，而不是单个 view 自由发挥。

## 4. Architecture Layers

### 4.1 Layer Order

```text
View / Widget / Shell
  -> Application Control
  -> Feature Runtime
  -> Authority / Projection / Transport
```

反向依赖是禁止的。

### 4.2 View Layer

包含：

- activity bar
- sidebar
- mobile layout
- overlays
- diff view
- chat panel

职责：

- 呈现 runtime 状态
- 转发 typed intents
- 管理纯 UI 本地态（开关、尺寸、hover、焦点）

### 4.3 Application Control Layer

包含：

- command routing
- layout command dispatch
- typed callback builders
- cross-surface intent normalization

职责：

- 把按钮/快捷键/手势统一成 feature runtime 可消费的命令
- 不直接保存业务 authority

### 4.4 Feature Runtime Layer

包含：

- `use_core` 中的 repo scope runtime
- document runtime
- source control runtime
- sync/session runtime

职责：

- 状态机
- ws/http/cli 请求
- ack/reject
- repo-scoped gating

### 4.5 Component Topology

建议目录职责：

```text
components/
  activity_bar/
  sidebar/
  mobile_layout/
  diff_view/
  chat/
  overlay/
  shared/
```

要求：

- 区域性 shell 组件按 layout 分区
- 原子组件不得偷偷携带 repo/document/source-control runtime 行为

## 5. Shell State Machines

### 5.1 Main Shell

```text
Booting
  -> Dashboard
  -> RepoScopedShell
  -> ModalOverlay
```

约束：

- `Dashboard` 与 `RepoScopedShell` 是两个不同的 shell state。
- overlay 打开时必须显式记录 `FocusedSurface` 与返回目标。

### 5.2 Focus State

```text
Editor
Sidebar
Panel
Modal
Drawer
```

规则：

- Modal MUST trap focus。
- 关闭 Modal 后 MUST restore previous focus target。
- 移动端 drawer 与 desktop sidebar 是不同 focus surface。

补充：

- `FocusedSurface` 是 shell runtime state，不是某个 view 自己猜测的 DOM 状态。
- modal / command palette / overlay 关闭后必须恢复到上一个稳定 surface。

### 5.3 Mobile Drawer State

```text
Closed
LeftDrawerOpen
RightDrawerOpen
ChatSheetOpen
```

规则：

- 边缘手势不得抢走交互控件点击。
- drawer 关闭必须清理 gesture capture。
- 移动端的更多菜单、repo switcher 与 source control menu 必须是 typed button semantics，而不是普通 div 点击。

### 5.4 Activity Bar / Menu State

```text
Collapsed
Expanded
PopupMenuOpen
PinnedSetChanged
```

规则：

- “切视图” 与 “Pin/Unpin” MUST 是两种独立控制语义。
- 菜单项整行点击只能触发 view switch，不能偷偷改 pin 状态。

### 5.5 Config Persistence Contract

- `deve_config` / layout prefs 只能存储纯 UI 偏好：
  - theme
  - sidebar visibility
  - widths
  - language
  - recent active view
- `deve_config` MUST NOT 持有：
  - session token
  - peer identity
  - repo vector
  - pending writes
  - `scope_nonce`
  - `client_op_id`

## 6. Layout Persistence Contract

- 所有 layout 持久化都属于 `deve_config` / local UI prefs。
- localStorage 中只允许保存：
  - 侧栏宽度
  - 面板宽度
  - 主题
  - 最近活动 view
  - 语言
- 禁止存储：
  - repo vector
  - pending writes
  - peer identity
  - auth token
  - scope nonce

## 7. Interaction Contracts

### 7.1 Repo / Branch Switching

- view 层只能请求 `SwitchRepo` / `SwitchBranch`。
- 成功与失败反馈来自 scope runtime，不允许 view 层自行假设切换完成。

### 7.2 Document Navigation

- 导航前是否可离开当前文档，必须由 document runtime 根据 pending write state 决定。
- view 层只能展示 pending navigation modal，不得自行判断“是否有未确认写入”。

### 7.3 Source Control Actions

- stage / unstage / discard / commit 都必须经过 source control runtime gate。
- remote readonly branch MUST 在 control 层就进入只读，不得等 view 层“按钮按了没反应”。

### 7.4 Markdown Shell Actions

- outline、preview、open doc、diff navigation 都必须读取 runtime 投影，不得自己重建 authority state。

### 7.5 Command Routing Contract

- 所有核心能力 SHOULD 同时具备：
  - toolbar/button trigger
  - keyboard shortcut trigger（若适用）
  - command palette trigger
- 这三个入口最终必须映射到同一 `CommandId` / application control。

## 8. Failure / Recovery

### 8.1 UI Recovery

- 页面 reload 后可恢复 layout prefs，但 MUST NOT 恢复过期的业务 authority。
- stale repo scope、stale diff session、stale pending navigation 必须通过 runtime 明确清理。

### 8.2 Runtime Failure Surfacing

- 未认证、断网、只读、repo scope mismatch、pending write reject 都必须来自结构化 runtime 状态。
- view 层禁止把任意 `String` 文本 error 当成稳定协议。

### 8.3 Multi-Surface Degrade

- Desktop / Mobile / Web 外壳差异只允许影响 shell 行为，不得改写 authority contract。
- Web 断连可降级只读；Desktop/Mobile 离线可继续使用本地 authority，但 UI 仍通过同一 command/control 体系驱动。

### 8.4 Offline / Thin-Client Split

- Web 是 thin client：
  - disconnect 后可降级只读
  - 不得继续伪装本地 authority 可写
- Desktop / Mobile native 可在本地 authority 上继续运行：
  - 但 view 层仍不能绕过 application control 直接写业务状态

## 9. Forbidden Patterns

- 组件直接写 `use_core` 内部信号以跳过 control 层。
- 菜单项点击既改 view 又改 pin。
- view 组件直接调用底层 repo/source control/storage API。
- 把业务真相塞进 localStorage。
- 让 shell 组件自行猜测 repo writable / readonly 状态。
- 用普通 `div` 冒充需要 button semantics 的交互控件。

## 10. Runtime Boundary

### 10.1 View / Shell Layer

- 负责可见 shell、panel、drawer、overlay、diff/chat surface 与可访问性交互。
- 只能消费 runtime state 并发出 typed intent。

### 10.2 Application Control Layer

- 负责 command dispatch、navigation guard、write gate、repo/document/source-control intent 编排。
- 不得持有 authority state 的私有副本。

### 10.3 Feature Runtime Layer

- 负责 feature-specific state machine、server message dispatch、pending overlay、diff session lifecycle。
- runtime 间交互必须通过稳定 command/control surface，不得互相篡改内部状态。

### 10.4 Shared Layout Infrastructure

- 负责 layout prefs、panel dimensions、overlay host 与跨 surface focus/stacking 管理。
- 不得保存 repo authority、session secret、peer private key 或业务事实。

## 11. Refactor Target

长期应显式形成三层前端结构：

- `ui_shell`
- `application_control`
- `feature_runtime`

目标不是继续增加无边界 effect，而是固定 command/control/runtime 的依赖方向：view 发 intent，control 做编排，runtime 持有本功能状态机。

## 本章相关命令

- `Cmd+Shift+P`
- `Cmd+P`
- `Cmd+Shift+K`
- `Cmd+B`

## 本章相关配置

- `ui.theme`
- `ui.sidebar_visible`
