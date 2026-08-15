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

### 3. Change Review 同级视图

- Source Control 只显示 Confirmed Ledger Changes、commit、history/graph；External Changes 独立显示
  staged/unstaged workspace drift。
- Remote Import 是第三个同级 view，显示 immutable session、candidate rows、typed blockers 与
  whole-session Refresh/Apply/Discard。
- 三个 view 的列表、菜单、颜色语义应明确；可以共享 renderer/row/button primitive，不共享
  controller、state、notice 或 authority。
- Remote Import 不显示 checkbox、locator、host/provider/blob path、digest、credential 或 raw detail。

### 4. 命令与更多菜单

- activity bar、更多菜单、repo switcher、command palette 都应能稳定工作。
- `Pin/Unpin` 与“切换视图”的语义应严格分离。

### 5. Native 双模式

- Desktop native-packaging 默认以 `LocalBackend` 模式启动，包含本地受控后端、zero-repo host registry 初始化、loopback endpoint、session handoff、健康探测和重启协调；不得为启动成功而自动创建默认 repo/projection。
- LocalBackend 的 `deve_cli serve --native-loopback` 后端应随 Desktop 父进程生命周期停止；Windows target-host 关闭或终止 Desktop 后不应留下孤儿本地后端。其它 target-host 在具备等价平台约束前不得宣称同等级别的异常终止保护。
- Desktop 可通过启动参数 `--remote-url https://host[:port]` 显式切换为 `RemoteBrowser` 模式，把壳层作为浏览器连接到远端 Docker/Web 的 HTTPS origin；脚本化/诊断启动仍可使用 `DEVE_NATIVE_REMOTE_URL`。
- Desktop bundled LocalBackend Settings 提供 Backend section，可校验并切换到 Remote Backend。默认 Local Backend 不要求用户单独启动后端。
- Remote Backend 必须先校验远端 HTTPS origin 的 `<origin>/api/node/role`，校验成功后才能保存；失败时 Settings 显示结构化失败反馈。
- RemoteBackend 失联时 UI 沿用浏览器锁屏/只读语义；远端页面不显示可调用 IPC 的恢复按钮。Desktop 原生主菜单和托盘中的 “Use Local Backend” 通过 host coordinator 保存 local preference 并重启壳层。
- `RemoteBrowser` 不启动本地后端，不注入本地 endpoint/session bootstrap，不注册 native bridge/Tauri commands，也不把远端 URL 保存为本地 writer authority。
- `RemoteBrowser` 主 WebView 由 native 以已校验 HTTPS `WindowConfig` 直接创建，不通过 bundled 页面或远端页面的 init-script redirect 过渡。
- 单独存在 `__TAURI_INTERNALS__` 不代表拥有 LocalBackend capability；普通 Docker 浏览器和 RemoteBrowser 均不注册 backend facade。
- LocalBackend 切换到 RemoteBrowser 必须在停止 sidecar 后重启壳层，不在同一 native process 中直接导航远端 origin。
- `RemoteBrowser` 只接受 HTTPS origin；包含 userinfo、query、fragment 或业务子路径的 URL 必须被拒绝。
- 文档、ledger、source-control、search 与 repo 写入仍必须经过本地 server/core writer gate。
- service 端口、session secret 与 P2P token material 不应出现在 URL、日志、Web localStorage 或可见 bootstrap payload。
- WebView native session cookie 安装失败不得让 Desktop 崩溃，也不得产生假 writer-ready；壳层只记录固定诊断，Web auth probe 收敛到 SessionInvalid/Unauthorized。
- Desktop host 优先验证显式 `DEVE_GIT_EXECUTABLE`；未配置时可从宿主绝对 PATH entries/PATHEXT 解析并 canonicalize Git。sidecar 只接收该绝对路径或互斥的 `DEVE_GIT_EXECUTABLE_UNAVAILABLE=1`，不继承完整 PATH/PATHEXT，也不在 unavailable 时回退 executable search。
- 显式 Git 路径无效时不回退；Git 缺失只让 mirror/import/export/push unavailable/out-of-sync，不阻断 LocalBackend、native session 或 NoteGit commit。
- Windows 已安装包的真实 UI smoke 使用隔离数据根、隔离 WebView2 profile 与 WebView2-assigned ephemeral CDP port 驱动 native WebView；exact diagnostic marker 由 Desktop host 以 programmatic WebView option 注入固定 `--remote-debugging-port=0`，以覆盖 elevated target-host 忽略环境 browser flags 的情况。marker 不接受任意 browser arguments，应用自身的普通启动路径不开放 CDP；该证据不声称覆盖宿主 WebView2 policy 或进程外注入。smoke 必须先证明新鲜数据根以 `BootstrapUnbound(scope_nonce=0)` 启动、没有默认 repo/projection且不声称 writer-ready，再通过 Web typed intent 完成首次 Create 并进入 backend-projected `ready`，随后完成编辑、commit/history、Settings 键盘焦点约束，并确认关闭后无孤儿 sidecar；它仍只提交 UI intent，不能绕过后端 authority。已绑定repo移除后的正式`NoScope`仍必须使用严格递增的非零scope nonce。

## 非目标

- 当前阶段不要求 Chrome MCP 覆盖真正的原生窗口管理能力。
- 快速 startup marker 只属于 entrypoint probe，不单独构成 packaged UI 可用证据。
- 当前阶段不把 Desktop native packaging 解释为签名 release readiness；LocalBackend 与 RemoteBrowser 仍需各自独立 smoke 验收。

## Chrome MCP 验收实例

### DESKTOP-UI-01: 宽屏工作台稳定

前置条件：

- 在宽视口打开应用。

步骤：

1. 观察 sidebar、editor、right panel、status 区。
2. 打开一个文档，再进入 diff、Source Control、External Changes 或 Remote Import。
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
4. 确认远端页面没有 native bridge、`ipc.localhost` 请求或 CSP 错误。
5. 通过 Desktop 原生菜单/托盘切回 Local Backend。
6. 检查壳层只加载远端 origin，且不启动本地 service 或注入本地 bootstrap；切回 local 后进程重启、重新启动本地 service 并取得新 session/scope。

期望结果：

- 默认 LocalBackend 可以启动受控 local service，并且 UI 写入仍经过本地 server/core writer gate。
- RemoteBrowser 等价于浏览器连接远端 Docker/Web URL。
- Settings 保存 remote 前必须完成 node-role 校验，校验失败不能写入 preference。
- RemoteBrowser 失联时 Desktop 原生菜单/托盘入口可切回 LocalBackend；远端 DOM 不拥有该操作。
- URL、日志、localStorage 与 bootstrap payload 不暴露 service secret 或 P2P token material。
