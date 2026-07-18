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
- 大屏主编辑区顶部应显示已打开 surface 的 tab；Markdown 文档 tab 超过用户设置的上限时，应自动关闭最早访问且不是当前 active 的文档 tab。
- 大屏 tab strip 中的可见 tab 应可拖拽到任意显式可见位置，包括拖到最后一个 tab 之后的可见空白区域；拖拽只改变可见顺序，不改变后台文档访问 LRU。
- Diff tab 不参与 Markdown 文档 tab 上限；打开 diff 不应挤掉文档 tab，也不应被文档上限自动关闭。
- 大屏 tab 的选择与关闭控件应在可访问标签中同时包含 surface 类型与当前 tab 标题，避免多个 tab 在辅助技术中重名。
- 窄屏下应映射到 mobile-style shell。
- URL、刷新、打开新窗口后，页面应保持可解释的工作台状态。
- Source Control、External Changes 与 Remote Import 在大屏/窄屏下都是同级 sidebar view；切换
  surface 不得复制或迁移彼此的业务 state。

### 4. Web 特有入口

- `Home`、`Open`、`Command` 等入口应可用。
- `Open in New Window` 应保留现有 query context，并正确追加文档参数。
- 文件树右键菜单应由 Context Action registry 投影；触发时提交 action intent，并在 handler 中重新 resolve。
- 当当前后端声明 host-file capability 时，文件树右键菜单应显示 `复制绝对路径` 与 `在系统资源管理器中显示`；前者复制后端返回的 canonical absolute path，后者请求后端/native adapter 在宿主系统文件管理器中 reveal 该 projection target。
- 普通远端 Web / VPS / 不支持的 native adapter 不应显示 host-file 菜单项；即使旧客户端直接请求，后端也应 fail-closed。
- Remote Import 入口只打开独立 typed review surface。候选行只显示 backend-generated label 与
  Added/Modified/Unchanged；无 checkbox、逐文件 Apply 或 raw locator/path/digest/detail。
- B4 已删除旧 Remote Projection command 打开 Source Control 的路径；B5 继续实现独立
  Remote Import client/view，缺失期间不以其它 controller 代替。

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
3. 检查本机 host-file capability 可用时菜单包含 `复制绝对路径` 与 `在系统资源管理器中显示`。
4. 在 readonly 状态、不匹配 surface/target 或不支持 host-file capability 下触发动作。

期望结果：

- 菜单只展示 resolver 允许的动作。
- handler 会重新 resolve intent；resolver miss fail-closed 且无副作用。
- external action 默认不投影、不执行。
- host-file action 不由前端拼绝对路径或执行系统命令；复制路径使用后端 canonical path，系统资源管理器 reveal 由后端/native adapter 受控执行。
