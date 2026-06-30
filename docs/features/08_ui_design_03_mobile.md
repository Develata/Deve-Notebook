# 08_ui_design_03_mobile.md - Mobile 壳层体验篇

本章描述移动端壳层体验，包括窄屏 Web 视口映射到 mobile shell 时，用户能看到和操作到的功能。

## 功能目标

- 用户能在窄屏中通过 top bar、drawer、sheet、bottom bar 进入同一套核心工作流。
- 关键入口必须可达、可关闭、不会互相冲突。
- 移动端交互应服务于文档与 repo 工作流，而不是制造额外状态混乱。

## 功能项

### 1. Top Bar 与 Drawer

- 用户应能通过 top bar 打开文件树或命令入口。
- 左/右 drawer 应可打开、关闭、切换，并且与主内容区配合清晰。

### 2. 移动端核心面板

- Outline、Source Control、Search 等入口必须在移动端可达。
- 移动端不应因为手势或边缘滑动吞掉关键按钮点击。
- 移动端 Source Control 入口应显示同一套 Source Control 面板，不应被 `Git: Status` 这类 Git mirror CLI-only 诊断提示替代。
- 顶部当前 surface 胶囊应显示当前文档或差异；点击后通过底部面板在已打开文档和差异之间切换。
- 移动端 Source Control 必须显示与桌面相同的 Source Control read surface；只读或远端视角只禁用写动作，不得用 `Git status 只能通过 CLI 查看` 这类 Git bridge notice 替代正常变更列表。

### 3. Bottom Bar 与状态折叠

- 用户应能看到 branch、ready/read-only、基础统计信息。
- bottom bar 在窄屏下应可折叠/展开，不应挤压主内容区到不可用。
- 移动辅助键盘栏必须服从完整 repo write gate；只读、握手中、快照加载中、writer 未就绪或 scope switching 时不得触发编辑、撤销或重做。

### 4. 搜索与 Sheet

- Search / Command 入口在移动端应以适合窄屏的方式呈现。
- 打开与关闭 sheet 时，不应和滚动、drawer、outline 产生语义冲突。

### 5. Native 双模式

- Android/Mobile native-packaging 默认以 `LocalBackend` 模式启动，包含 in-process embedded loopback service、默认 repo/projection 初始化、loopback endpoint、session handoff、foreground reprobe、readiness 展示和失败恢复。
- `LocalBackend` 必须在主 WebView 创建前完成 native session handoff、bootstrap 注入与 cookie 注册。
- Mobile v1 不使用 child process。
- Mobile 可显式切换为 `RemoteBrowser` 模式，把壳层作为浏览器连接到远端 Docker/Web 的 HTTPS origin；Settings 中的 Backend section 与 Desktop 使用同一套 Local Backend / Remote Backend 语义。
- Remote Backend 必须先校验远端 HTTPS origin 的 `<origin>/api/node/role`，校验成功后才能保存；失败时 Settings 显示结构化失败反馈。
- RemoteBrowser 失联时 UI 沿用浏览器锁屏/只读语义；native-only “Use local backend” 入口可切回 Local Backend、启动 embedded loopback service 并重载 bundled Web shell。
- `RemoteBrowser` 不启动 embedded service，不注入本地 endpoint/session bootstrap，不把远端 URL 保存为本地 writer authority。
- `RemoteBrowser` 只接受 HTTPS origin；包含 userinfo、query、fragment 或业务子路径的 URL 必须被拒绝。
- Mobile native bundle 不应固定依赖开发期 `http://127.0.0.1:3001` devUrl；生产 shell 加载 bundled frontendDist，并由 native backend mode 决定连接 local 或 remote。
- 后台或系统暂停期间不承诺长时同步；回到前台后必须重新探测 service 与 writer gate。
- 文档、ledger、source-control、search 与 repo 写入仍必须经过 embedded service 内的 server/core writer gate。
- in-process embedded loopback service 的 auth/session bootstrap material 必须经 typed runtime launch options 传递，不得通过进程级环境变量写入/读回。

## 非目标

- 当前阶段不要求移动端提供完整原生系统分享、推送、相机等功能说明。
- 当前阶段不要求 Chrome MCP 覆盖真正的 Android/iOS 原生容器能力。
- 当前阶段不要求移动端后台长时 P2P 同步，也不把 native packaging 解释为签名/商店/物理设备 release readiness。

## Chrome MCP 验收实例

### MOBILE-UI-01: Drawer 与核心入口可达

前置条件：

- 使用移动端视口打开页面。

步骤：

1. 打开左侧 drawer。
2. 切换到 Source Control 或 Search。
3. 打开并关闭 Outline。

期望结果：

- 关键入口可达、可关闭、不会卡死。
- drawer、outline、主内容区之间的切换语义清晰。

### MOBILE-UI-02: Mobile 壳层一致性

前置条件：

- 页面处于移动端视口。

步骤：

1. 观察 top bar、bottom bar、内容区。
2. 尝试打开搜索或命令入口。
3. 观察只读、连接、状态栏信息是否仍可见。

期望结果：

- 移动端壳层服务于同一套核心工作流。
- 关键状态与命令入口不会因为窄屏而丢失。

### MOBILE-UI-03: Mobile 多文件与 Diff 切换

前置条件：

- 页面处于移动端视口。
- 至少打开两个文档，并从 Source Control 打开一个 diff。

步骤：

1. 点击当前 surface 胶囊。
2. 在底部面板中切换到另一个文档。
3. 再次打开底部面板并切换回 diff。
4. 关闭 active diff。

期望结果：

- 底部面板分组显示 Documents 与 Diffs。
- 选择文档走受保护的文档导航；选择 diff 只恢复已有 diff session。
- 移动端 diff 始终使用 Unified View。
- 关闭 diff 不改变 staged、pending 或 commit state。

### MOBILE-UI-05: Mobile Source Control 正常显示

前置条件：

- 页面处于移动端视口。
- 当前 Source Control scope 可读；可处于只读或远端视角。

步骤：

1. 打开左侧 drawer。
2. 切换到 Source Control。
3. 观察 Source Control panel。

期望结果：

- Source Control 显示 `Staged Changes` / `Changes` / `Confirmed Ledger Changes` 或 clean empty state。
- 只读或远端视角下写动作被禁用，但 read list / diff 仍走 Source Control read gate。
- 未显式触发 Git bridge 命令时，不显示 `Git status 只能通过 CLI 查看` 作为 Source Control 的替代内容。

### MOBILE-UI-04: Native 双模式边界

前置条件：

- 已构建 native-packaging Mobile shell。
- 准备默认 `LocalBackend` 启动与显式 `RemoteBrowser` HTTPS origin。

步骤：

1. 默认环境启动 Mobile shell。
2. 检查 embedded loopback service、session handoff 与 Web shell 可用性。
3. 模拟 foreground reprobe，并检查 writer gate 状态。
4. 在 Settings 中切换到 RemoteBrowser HTTPS origin，检查壳层只加载远端 origin。
5. 模拟 RemoteBrowser 失联，并使用 native-only “Use local backend” 切回 local。

期望结果：

- 默认 LocalBackend 可以启动 embedded loopback service。
- RemoteBrowser 等价于浏览器连接远端 Docker/Web URL。
- Settings 保存 remote 前必须完成 node-role 校验，校验失败不能写入 preference。
- RemoteBrowser 失联时 native-only 入口可切回 LocalBackend。
- 前台恢复后 UI 重新探测 service；写入仍受 repo writer gate 控制。
- 后台长时同步不可被 UI 暗示为已支持。
