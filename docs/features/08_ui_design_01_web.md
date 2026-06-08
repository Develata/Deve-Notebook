# 08_ui_design_01_web.md - Web 壳层体验篇

本章描述 Web 端作为浏览器工作台与 WebLightPeer 薄客户端时，用户能看到和操作到的功能。

## 功能目标

- 用户能在浏览器中进入稳定的 Web 工作台。
- 用户能看到 Dashboard、Web sync 状态、repo 切换与只读/重连反馈。
- Web 壳层在大屏和窄屏下都能维持清晰的控制入口。

## 功能项

### 1. Dashboard 首页

- 打开根路径且未进入具体文档时，应看到 server dashboard。
- dashboard 应显示 system health、sync status、storage stats、quick actions。

### 2. WebLightPeer 状态反馈

- 用户应能看见当前是 `Connected`、`Reconnecting` 还是 `Read-only`。
- repo 切换时应能看到 handshake / repo rebinding 的过渡状态。
- 浏览器存储缺失时，应出现只读提示而不是静默失败。

### 3. Web 壳层与主功能区

- 大屏下应提供 desktop-style workbench。
- 大屏下文件树、显示/编辑区、AI Chat 三个区域之间的分界线应可拖到边缘；普通拖拽折叠区域时分界线仍保留，可从边缘拉回。
- 窄屏下应映射到 mobile-style shell。
- URL、刷新、打开新窗口后，页面应保持可解释的工作台状态。

### 4. Web 特有入口

- `Home`、`Open`、`Command` 等入口应可用。
- `Open in New Window` 应保留现有 query context，并正确追加文档参数。
- 文件树右键菜单应由 Context Action registry 投影；触发时提交 action intent，并在 handler 中重新 resolve。

## 非目标

- 当前阶段不要求 Web 具备 native 端完整离线 authority。
- 当前阶段不要求浏览器端承担完整本地 ledger。

## Chrome MCP 验收实例

### WEB-UI-01: Dashboard 可达且指标可见

前置条件：

- 打开应用根路径。

步骤：

1. 观察 dashboard 卡片区域。
2. 检查 system health、sync status、storage、quick actions。
3. 点击 `New Doc` 或 `Sync Now` 观察是否有响应。

期望结果：

- dashboard 结构完整。
- 指标区与操作区都可见且可交互。

### WEB-UI-02: Web sync 状态可见

前置条件：

- 页面已连接到服务端。

步骤：

1. 观察当前连接/同步状态。
2. 切换 repo 或刷新页面。
3. 观察 handshake、reconnect、readonly 提示是否变化。

期望结果：

- 用户能分辨 connected、reconnecting、readonly 等不同状态。
- repo 切换期间不会出现无语义的空白状态。

### WEB-UI-03: 文件树右键菜单来自 Context Action registry

前置条件：

- Web 端已运行并存在文件树节点。

步骤：

1. 打开文件树节点右键菜单。
2. 检查菜单动作来自 Context Action registry projection。
3. 在 readonly 状态或不匹配 surface/target 下触发动作。

期望结果：

- 菜单只展示 resolver 允许的动作。
- handler 会重新 resolve intent；resolver miss fail-closed 且无副作用。
- external action 默认不投影、不执行。
