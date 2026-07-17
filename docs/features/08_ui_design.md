# 08_ui_design.md - 界面与控件体验篇

本章描述通用 UI 与多端壳层体验。

## 功能目标

无论在 Web、Desktop 还是 Mobile，用户都应获得一致的核心控制入口与清晰的交互反馈。

本章关注的是：

- 用户能看到什么界面结构
- 用户可以通过哪些控件完成哪些动作
- 控件如何触发应用控制，而不是直接改写底层真相

## 功能项

### 1. 主工作台

- 主界面应提供稳定的工作台布局：
  - Activity Bar / 导航入口
  - Sidebar / 功能面板
  - Editor / 主内容区
  - Bottom / Status 区

### 2. 控件与命令的一致性

- 关键能力应同时能通过控件和命令入口触发。
- 控件点击结果必须稳定，不应出现“点了没反应”或“误触发别的动作”。

### 3. Web / Desktop / Mobile 壳层

- Web、Desktop、Mobile 可以有不同的壳层布局，但核心能力应一致。
- Mobile 的 drawer、top bar、bottom bar 必须服务于核心工作流，而不是引入额外状态混乱。

### 4. Source Control / External Changes / Remote Import / Explorer / Search 等核心控件

- 主侧栏功能切换必须稳定。
- `More(...)`、drawer、sheet、sidebar 切换必须语义清晰。
- 只读、错误、加载、重连等状态必须有明确可见提示。
- Source Control、External Changes 与 Remote Import 必须是三个同级入口；Remote Import review 不得
  伪装成 Source Control notice，也不得先覆盖 workspace 再进入 External Changes。
- 原子操作示例：[`operations/search_query.md`](./operations/search_query.md)

### 5. 薄显示层原则

- 控件只负责发出用户意图，不应直接操纵业务真相。
- 同一个用户动作，在不同端上应尽量收敛到同一条 application/control 路径。
- Remote Import UI 只渲染 backend-generated label、typed state/blocker/diff，并发送 whole-session
  intent；不解析 detail，不计算 diff/blocker，不持有 manifest/blob/Ledger authority。
- Web runtime 共享 request/state 类型应归属于 runtime/domain contract；`use_core` 只能作为 application-control composition root 重新导出这些类型，不得让 `runtime/*_client` 反向依赖 `hooks/use_core` 内部模块。Editor/sync 等 runtime 消费端也应直接使用 `runtime/domain` 的共享类型，而不是经由 `use_core` re-export 取回 domain state；AI backend fallback hook 应接收明确的 runtime/domain 信号而不是 import `use_core` context；repo/scope 稳定性 helper 归属于 `runtime/scope_client`，Editor/sync/chat 以及 `hooks/use_core` 之外的 hook 消费端不应直接 import `hooks/use_core::callbacks_scope`。

## 非目标

- 当前阶段不追求显示层堆叠复杂特效来替代核心控制逻辑。
- 当前阶段不允许页面组件各自维护一套独立业务真相。

## Chrome MCP 验收实例

### UI-FEAT-01: 主工作台可达性

前置条件：

- 打开应用首页。

步骤：

1. 观察 Activity Bar、Sidebar、Editor、Bottom 区。
2. 打开一个文档。
3. 切换 Source Control、External Changes、Remote Import 与 Explorer。

期望结果：

- 主工作台结构稳定。
- 用户能明确理解当前所在区域与控制入口。

### UI-FEAT-02: 控件触发语义正确

前置条件：

- 页面已进入主工作流。

步骤：

1. 点击 `More(...)`、切换主要面板、打开 drawer 或菜单。
2. 观察菜单项点击是否触发预期行为。
3. 观察 `Pin/Unpin`、关闭按钮、切换按钮的语义是否独立。

期望结果：

- 每个控件只做自己应做的事情。
- 不出现“点击项实际上改了别的状态”的情况。

### UI-FEAT-03: Mobile 壳层一致性

前置条件：

- 用移动端视口打开页面。

步骤：

1. 打开左/右 drawer。
2. 切换核心面板。
3. 打开并关闭 Outline、Source Control、External Changes、Remote Import、Search 等入口。

期望结果：

- Mobile 壳层服务于同一套核心工作流。
- 关键控件可达、可关闭、不会互相冲突。
