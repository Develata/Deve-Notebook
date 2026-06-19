# 08_ui_design_02_desktop.md - Desktop 壳层体验篇

本章描述 Desktop 端在宽屏工作台语义下应呈现的用户体验。当前 Chrome MCP 可通过宽视口的共享 Web shell 验证大部分交互行为。

## 功能目标

- 用户能获得稳定的 desktop-style workbench。
- 用户能在宽屏布局下高效访问 editor、diff、outline、chat、source control。
- 桌面壳层不应因为菜单、pin、panel 切换而产生语义混乱。

## 功能项

### 1. 宽屏工作台布局

- 用户应看到清晰的多列工作区。
- sidebar、editor、outline、chat、diff 等区域应在宽屏中合理分布。

### 2. 可调整的面板

- sidebar、display/editor 区域和右侧 AI Chat 面板应通过两条分界线调整宽度。
- 普通拖拽可把任一区域折叠到边缘；折叠后分界线仍可见，并可从边缘把区域拉回。
- Settings 隐藏 AI Chat 时，AI Chat 区域和它左侧的分界线应同时消失；这不同于拖拽折叠。
- 用户设置后的布局应保持稳定，不因刷新或切换 view 而混乱。

### 3. Source Control 视图

- staged / unstaged / history / graph 等区域应在桌面壳层内清晰可见。
- source control 列表、菜单、颜色语义应明确。

### 4. 命令与更多菜单

- activity bar、更多菜单、repo switcher、command palette 都应能稳定工作。
- `Pin/Unpin` 与“切换视图”的语义应严格分离。

### 5. Native 双模式

- Desktop native-packaging 默认以 `LocalBackend` 模式启动，包含本地受控后端、默认 repo/projection 初始化、loopback endpoint、session handoff、健康探测和重启协调。
- Desktop 可显式切换为 `RemoteBrowser` 模式，把壳层作为浏览器连接到远端 Docker/Web 的 HTTPS origin。
- `RemoteBrowser` 不启动本地后端，不注入本地 endpoint/session bootstrap，不把远端 URL 保存为本地 writer authority。
- `RemoteBrowser` 只接受 HTTPS origin；包含 userinfo、query、fragment 或业务子路径的 URL 必须被拒绝。
- 文档、ledger、source-control、search 与 repo 写入仍必须经过本地 server/core writer gate。
- service 端口、session secret 与 P2P token material 不应出现在 URL、日志、Web localStorage 或可见 bootstrap payload。

## 非目标

- 当前阶段不在本章定义 Tauri 原生托盘、系统菜单等平台整合细节。
- 当前阶段不要求 Chrome MCP 覆盖真正的原生窗口管理能力。
- 当前阶段不把 Desktop native packaging 解释为签名 release readiness；LocalBackend 与 RemoteBrowser 仍需各自独立 smoke 验收。

## Chrome MCP 验收实例

### DESKTOP-UI-01: 宽屏工作台稳定

前置条件：

- 在宽视口打开应用。

步骤：

1. 观察 sidebar、editor、right panel、status 区。
2. 打开一个文档，再进入 diff 或 source control。
3. 观察布局是否仍然清晰稳定。

期望结果：

- 宽屏工作台结构稳定。
- 用户不会失去当前区域与控制入口的上下文。

### DESKTOP-UI-02: 更多菜单与固定语义分离

前置条件：

- 宽视口下进入 activity bar 或侧栏。

步骤：

1. 打开 `More(...)` 菜单。
2. 点击某个 view 切换项。
3. 再点击 `Pin/Unpin`。

期望结果：

- view 切换只切换 view。
- `Pin/Unpin` 只修改固定状态，不会误触发其他动作。

### DESKTOP-UI-03: Native 双模式边界

前置条件：

- 已构建 native-packaging Desktop 包。
- 准备默认 `LocalBackend` 启动与显式 `RemoteBrowser` HTTPS origin。

步骤：

1. 默认环境启动 Desktop shell。
2. 检查本地 loopback service health、native session handoff 与 Web shell 可用性。
3. 切换到 RemoteBrowser HTTPS origin。
4. 检查壳层只加载远端 origin，且不启动本地 service 或注入本地 bootstrap。

期望结果：

- 默认 LocalBackend 可以启动受控 local service，并且 UI 写入仍经过本地 server/core writer gate。
- RemoteBrowser 等价于浏览器连接远端 Docker/Web URL。
- URL、日志、localStorage 与 bootstrap payload 不暴露 service secret 或 P2P token material。
