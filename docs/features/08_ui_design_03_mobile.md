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
- 从屏幕左边缘向右滑应打开文件树；从屏幕右边缘向左滑应打开当前 Markdown 的逐级标题大纲。该手势可从编辑器边缘开始，但短拖动、纵向滚动、多指操作和靠边按钮点击不能误开 drawer。

### 2. 移动端核心面板

- Outline、External Changes、Source Control、Remote Import、Search 等入口必须在移动端可达。
- 移动端不应因为手势或边缘滑动吞掉关键按钮点击。
- 移动端 Source Control 入口应显示同一套 Source Control 面板，不应被 `ngit: Status` 这类 Git mirror CLI-only 诊断提示替代。
- 移动端 External Changes 入口应显示外部投影文件夹修改的最小操作集：Open Diff、Stage / Unstage、Discard、确认外部修改。
- 顶部当前 surface 胶囊应显示当前文档或差异；点击后通过底部面板在已打开文档和差异之间切换。
- surface 底部面板打开后焦点进入关闭按钮，Tab 不离开面板，Escape 可关闭，关闭后焦点回到触发胶囊。
- 移动端 Source Control 必须显示与桌面相同的 Source Control read surface；只读或远端视角只禁用写动作，不得用 `ngit status 只能通过 CLI 查看` 这类 ngit CLI-only 诊断替代正常变更列表。
- 移动端 Source Control、External Changes 与 Remote Import 是同级入口：Source Control 显示
  ledger/version-anchor 状态、Commit、history/graph；External Changes 显示外部投影偏差；Remote
  Import 显示 immutable session/candidate review，后二者都不显示 history/graph。
- Remote Import candidate row 只显示 backend-generated label、Added/Modified/Unchanged 与 typed
  blocker/diff；无 checkbox 或逐文件 Apply。Refresh/Apply/Discard 是 whole-session action，390×844
  viewport 不得横向溢出且动作满足 44px touch target。
- Remote Import 移动入口已由 B5 接入独立 typed client/view，B6 已登记真实 backend/provider browser
  producer；最终 candidate current-HEAD receipt仍须重跑。B4 已删除旧 command 打开 Source Control
  的路径，缺失期间不以其它 controller 代替。

### 3. Bottom Bar 与状态折叠

- 用户应能看到 branch、ready/read-only、基础统计信息。
- bottom bar 在窄屏下应可折叠/展开，不应挤压主内容区到不可用。
- 移动辅助键盘栏必须服从完整 repo write gate；只读、握手中、快照加载中、writer 未就绪或 scope switching 时不得触发编辑、撤销或重做。
- 软键盘导致 viewport resize 时，只要宽度没有跨越 mobile breakpoint，就必须保留当前编辑器 mount、projection load session 与输入焦点，不得重新发送 OpenDoc 或用 Snapshot 打断正在进行的输入。

### 4. 搜索与 Sheet

- Search / Command 入口在移动端应以适合窄屏的方式呈现。
- 打开与关闭 sheet 时，不应和滚动、drawer、outline 产生语义冲突。

### 5. Native 双模式

- Android/Mobile native-packaging 默认以 `LocalBackend` 模式启动，包含 in-process embedded loopback service、zero-repo host registry 初始化、loopback endpoint、session handoff、foreground reprobe、readiness 展示和失败恢复；不得为启动成功而自动创建默认 repo/projection。
- `LocalBackend` 必须在主 WebView 创建前完成 native session handoff 与 bootstrap 注入。Android 因 Wry cookie API 不受支持，必须在 WebView 登记后以无参数 native command 调用系统 CookieManager 安装 HttpOnly cookie；初次 prepare 与 resume/replacement 共用同一个 process-local WebView handoff single-flight gate，同 generation 重复 prepare 串行复核而不并发竞争 completion registry。RemoteBrowser 恢复创建的新本地 WebView 必须额外等待 native coordinator 确认 `main` 登记、recovery anchor 退休与 `LocalWindowCreated` 记录完成；只有 backend-owned 一次性 admission 能放行 initial/resume handoff，Web 不判断 Activity lifecycle。同名 `Secure` cookie 替换必须等待系统 completion callback 成功，并以与 `setCookie` 相同的 HTTPS secure loopback origin 复核当前精确 cookie；不得用 HTTP URL 的发送资格过滤误判 retention，真实 HTTP loopback session 可用性由 reload 后的 auth/readiness 闭环证明。不能把旧进程遗留值或写入尚未完成误判为成功。防止 reload 重复 prepare 的标记必须使用当前 process/session 的非敏感安装身份；即使新进程复用同一个 loopback endpoint，也必须丢弃旧 bootstrap/安装标记并安装新 cookie。保存值必须匹配 ready bootstrap 的完整非敏感形态；同一 WebView process 的后续 generation endpoint 只允许由 native supervisor replacement source 覆写，页面 reload 必须采用它而不是 init script 中过时的首 generation fallback。browser storage 不可用或 marker 不能写后读确认时直接显示 session invalid，不得反复 handoff/reload。已提交的平台写入若超时，必须在迟到 callback 到达前拒绝新的写入；callback 永不到达时要求重启进程，不能让不可取消的旧写入与新 generation 竞态。确认后只 reload 一次；cookie/secret 不得进入 JS、command 参数或失败诊断。
- 恢复 admission timeout 覆盖平台 Activity acknowledgement 与 anchor retirement 上限并保留余量；timeout 或 supervisor shutdown 会取消 waiter 并成为不可迟到放行的 terminal failure，迟到 grant 转入 post-retirement cold restart。
- Mobile v1 不使用 child process。
- Mobile 可从可信 bundled `LocalBackend` Settings 显式切换为 `RemoteBrowser` 模式，把壳层作为浏览器连接到远端 Docker/Web 的 HTTPS origin。
- Remote Backend 必须先校验远端 HTTPS origin 的 `<origin>/api/node/role`，校验成功后才能保存；失败时 Settings 显示结构化失败反馈。
- RemoteBrowser 失联时 UI 沿用浏览器锁屏/只读语义；远端页面不注册 backend facade，也不能调用 native IPC。
- preference-driven RemoteBrowser 必须显示 Android/iOS 平台原生 “Use Local Backend” 控件；显式 CLI/env override 必须隐藏。控件只调用 native coordinator，不进入远端 DOM，也不注册 RemoteBrowser IPC。
- 切回 local 时必须销毁远端 WebView，启动新的 embedded supervisor，并以新的随机 endpoint、native session、repo handshake 与 scope 创建 bundled-local WebView；远端 cookie/session/authority 不得复用。
- `RemoteBrowser` 不启动 embedded service，不注入本地 endpoint/session bootstrap，不注册 Tauri commands，也不把远端 URL 保存为本地 writer authority。
- `RemoteBrowser` 主 WebView 由 native 以已校验 HTTPS `WindowConfig` 直接创建，不通过 bundled 页面或远端页面的 init-script redirect 过渡。
- `RemoteBrowser` 只接受 HTTPS origin；包含 userinfo、query、fragment 或业务子路径的 URL 必须被拒绝。
- Mobile native bundle 不应固定依赖开发期 `http://127.0.0.1:3001` devUrl；生产 shell 加载 bundled frontendDist，并由 native backend mode 决定连接 local 或 remote。
- 后台或系统暂停期间不承诺长时同步；回到前台后必须重新探测 service 与 writer gate。
- LocalBackend 由一个 lifecycle supervisor 独占 process-scoped `EmbeddedServerRuntime`、transport task、shutdown sender、随机 endpoint 与 session generation；authority runtime 每个 app 进程只初始化一次，普通退出和切换 Remote Backend 必须先有界关闭 transport 与全部 runtime tasks，RemoteBrowser 不创建 supervisor。
- 系统暂停时编辑器立即只读但不清空未确认编辑；恢复时重新验证 auth、node role、WS 与 current scope。若 transport 已退出，则在同一 authority runtime 上创建新的随机 endpoint/session，generation token 校验通过后安装新 cookie/bootstrap，再通知 Web 恢复；旧 scope 写入必须被拒绝。
- resume probe 在 native lifecycle lock 外执行，shutdown 可取消正在进行的 reprobe；固定初始 bootstrap、旧 endpoint 与迟到的旧 generation 结果不得覆盖 current generation。
- resume 使用 single-flight gate；probe 必须验证返回 endpoint 属于当前随机 listener。transport replacement 会关闭旧 WebSocket generation，Web 通过 typed rebind control 重新读取 session-scoped bootstrap 并连接新 endpoint，不 reload 页面或清空 pending。同一 local branch、同一 repo UUID 的内部 session restore 被服务端确认后，browser document runtime 会把保留的 pending rows 重绑到新 scope；普通 repo/branch 切换不会这样做，且 fresh writer-ready 前不会 replay。
- current-generation cookie/bootstrap 安装与 resumed 事件是一次受校验的 WebView handoff；任一步失败都会进入结构化 error。Mobile LocalBackend 关闭可选 prewarm，以保证 suspend/exit 的有界 task join；其他 runtime task 仍由唯一 authority runtime 持有并关闭。
- lifecycle handoff 不持 state mutex 跨 WebView 调用，且所有 resumed/suspended/error 事件使用单调 transition guard。无法证明旧 WebSocket sessions 已 retired 时进入 `runtime_restart_required`，本进程不再自动创建 transport。
- service restart、session handoff 或 foreground reprobe 失败时显示结构化 degraded/error 状态，不得恢复可写或伪装为普通网络断开。
- 文档、ledger、source-control、search 与 repo 写入仍必须经过 embedded service 内的 server/core writer gate。
- bundled Web shell 仍需 IndexedDB 与不可导出的 WebCrypto Ed25519 repo identity；Android System WebView 缺少该能力时保持 storage-limited 只读，LocalBackend 不得以 native session 绕过 browser identity。
- Android 正式支持/可写 evidence baseline 为 Android 10/API 29+ 与当前 WebView provider 137+；版本事实只用于 support/receipt，真实 non-extractable Ed25519 probe 仍是 writer gate 的最终判据。
- WebCrypto Ed25519 不可用时，横幅会提示更新浏览器或 Android System WebView；target-host smoke 记录 API、provider、AVD/设备标识并先验证真实 key generation，再执行创建、编辑、提交与 lifecycle 流程。低于支持基线或 probe 失败只能产生只读负向证据。
- Android target-host smoke 先构建 APK 并释放 Gradle daemon，再以 low-RAM 模式、有界 RAM 和默认 `4096 MiB`（允许 `2048..8192 MiB`）的受控 writable data partition request 启动绑定本次进程的专用 emulator serial 与 AVD；它兼容目标镜像发布的 `sys.boot_completed` / `dev.bootcomplete` 完成信号，并在同一 boot deadline 内以短时有界只读 probe 等待 package manager 与 `settings` system provider 同时 ready，且两项服务必须连续稳定至少 `10s`。package manager 每次成功样本必须只含规范 `package:<name>` 行并恰有一个 `package:android`；`settings get global device_provisioned` 只有返回规范 `0` / `1` 才允许继续解析 `/data` 和安装 APK。稳定窗口中的已准入 bootstrap 暂态会清零窗口，timeout、中断、process guard failure、混合输出或未知错误立即 fail-closed，最终准入前还必须再次通过 owned-emulator process guard。已准入的 package-service bootstrap/race failure 之后也必须在同一 absolute install deadline 内重新完成相同的连续稳定准入；带 kill grace 的 timeout probe 会先从剩余 deadline 预留该 grace，精确 provider race 只允许进入该重新准入链，不能增加 install attempt 或用固定 sleep 冒充 readiness。`/data` 总容量必须达到 request 的四分之三且安装前至少有 `1024 MiB` 可用空间。首次创建文档时会立即向当前 CodeMirror host 输入文本；同 breakpoint 的真实键盘 viewport resize 必须保持同一 host 与 OpenDoc request；跨 generation pending 通过暂停并丢弃旧 transport 的 outbound edit frame 证明，只有 replacement generation 的产品 replay 能让后端首次看到该文本。editor mount 以 host owner 隔离，旧 surface 的迟到 cleanup 不会销毁新 editor 或吞掉第一笔输入；提交必须在 NoteGit history 中出现对应 message 后才算成功。
- Android RemoteBrowser target-host smoke 必须同时接受 native 启动路径输出的精确、无敏感信息的 `RemoteBrowser + embedded backend absent` 模式标记，并在原生恢复意图前拒绝精确的 `embedded backend supervisor started` ownership 标记；不得用任何包含 `LocalBackend` 单词的宽泛日志正则推断 runtime ownership，因为恢复控件与失败闭合诊断也会使用该模式名称。supervisor 启动标记本身不得过度声明 shell 已完成 LocalBackend cutover；最终切换仍由 typed transition snapshot 与 fresh endpoint/session/scope 共同证明。
- Android RemoteBrowser 切回 LocalBackend 时以未导出、无 command/bootstrap/writer capability 的临时 recovery Activity/WebView 维持同一进程 lifecycle；验收不得把该 `about:blank` 锚点冒充 bundled-local surface，且必须在 fresh local `main` 建立后确认锚点已退休。
- Android recovery 的 committed-unknown 或退休后失败必须在锚点仍存活时先关闭候选 runtime，再启动无 Tauri/业务 authority 的独立进程 recovery Activity；该组件连续确认旧 PID 已退休后才创建 launcher task并退出自身。helper 调度确认后，旧进程在 failure snapshot 与有界 shutdown 完成的边界直接退休，不会再被普通 graceful-exit gate 阻塞。不得在旧 Tauri 进程直接启动新 `MainActivity`、使用桌面 `current_exe` spawn，或把未确认的 Activity 创建/退休回滚成可重试 RemoteBrowser。
- guest-service probe 返回未准入的 exit `224` 时，target-host smoke 必须输出有界、非敏感的响应诊断：只截取最多 `160` 个原始字节，在该固定前缀内完成 CR 归一化，并记录前缀字节数、样本行数、截断标记与 alphanumeric run 已折叠的 ASCII structural preview；不得输出原始响应或其全量稳定指纹。该诊断只观察固定只读 package/settings probe 的失败输出，不记录成功 package 列表，不得改变 classifier、稳定窗口、absolute deadline、install attempt 或原始失败码。
- Android emulator pin 必须从有界 `-version` probe 的 canonical banner 识别精确 version/build，
  不能接受只碰巧包含两个 token 的任意文本；同一 build 的 checksum-pinned cache publication
  由有界 build lock 串行化并在锁内复核现有 binary；既存 invalid entry 保留并 fail-closed，不自动
  删除一个可能正在使用的 build。renderer 也必须从本次 owned emulator 的有界
  日志前缀内全部 selection 证明实际使用批准的 `swangle`/software path。`-gpu swangle` 当前投影出的完整
  `vulkan_mode_selected:swiftshader gles_mode_selected:swangle` 有序 pair 表示 Vulkan 由 SwiftShader、
  GLES 由 ANGLE-on-SwiftShader（SwANGLE）提供，是当前 pin 唯一批准的 swangle 证据；其它 software
  token pair 不能作为兼容回退。缺失、冲突、未批准 pair 或 `swiftshader_indirect` 证据会在安装 APK
  和执行业务 journey 前失败；未来 pin/rollback 的不同 pair 必须先由新的 target-host evidence 批准。
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

1. 从屏幕左边缘向右滑，打开文件树 drawer。
2. 关闭 drawer，再从屏幕右边缘向左滑，打开 Markdown 标题大纲。
3. 从编辑器内容区的左右边缘重复上述手势，确认两侧 drawer 仍可达。
4. 执行短拖动、纵向滚动、多指操作，并点击靠边的真实按钮。
5. 打开左侧 drawer，切换到 External Changes、Source Control、Remote Import 或 Search。

期望结果：

- 关键入口可达、可关闭、不会卡死。
- drawer、outline、主内容区之间的切换语义清晰。
- 短拖动、纵向滚动、多指操作与真实按钮点击不会误开 drawer，也不会修改 Markdown 内容或 pending 状态。
- Remote Import 可独立打开；没有 checkbox、逐文件 Apply 或横向溢出，不会显示 raw locator/path/digest/detail。

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

- Source Control 显示 `Confirmed Ledger Changes` 或 clean empty state，并保留 commit/history/graph read surface。
- 只读或远端视角下写动作被禁用，但 read list / diff 仍走 Source Control read gate。
- 未显式触发 ngit 诊断命令时，不显示 `ngit status 只能通过 CLI 查看` 作为 Source Control 的替代内容。

### MOBILE-UI-06: Mobile External Changes 正常显示

前置条件：

- 页面处于移动端视口。
- 当前 repo scope 可读，Projection Workspace 中存在外部修改。

步骤：

1. 打开左侧 drawer。
2. 切换到 External Changes / 外部修改。
3. 观察外部修改列表与行级动作。

期望结果：

- External Changes 显示 external unstaged / staged external groups 或 clean empty state。
- 行级 Open Diff、Stage / Unstage、Discard 与 `确认外部修改` 触控尺寸可用。
- External Changes 不显示 Source Control history / graph，也不把 `确认外部修改` 写成普通 Commit。

### MOBILE-UI-04: Native 双模式边界

前置条件：

- 已构建 native-packaging Mobile shell。
- 准备默认 `LocalBackend` 启动与显式 `RemoteBrowser` HTTPS origin。

步骤：

1. 默认环境启动 Mobile shell。
2. 检查 embedded loopback service、session handoff 与 Web shell 可用性。
3. 模拟 foreground reprobe，并检查 writer gate 状态。
4. 在后台期间终止当前 transport generation，再恢复前台并确认唯一 authority runtime 未重建、新 endpoint/session generation 被安装、旧 scope 写入被拒绝且非零 pending 未丢失。
5. 在 Settings 中切换到 RemoteBrowser HTTPS origin，检查壳层只加载远端 origin且旧 embedded service 已有界退出。
6. 模拟 RemoteBrowser 失联，确认远端页面没有 backend facade/native IPC；通过平台原生 “Use Local Backend” 控件切换。
7. 确认远端 WebView 被销毁，bundled-local WebView 使用新的 loopback endpoint/session/scope，且切换前后没有孤儿 embedded runtime。

期望结果：

- 默认 LocalBackend 可以启动 embedded loopback service。
- RemoteBrowser 等价于浏览器连接远端 Docker/Web URL。
- Settings 保存 remote 前必须完成 node-role 校验，校验失败不能写入 preference。
- RemoteBrowser 失联时保持只读且不暴露 native IPC；native 恢复入口可在不复用远端 authority 的前提下建立全新 LocalBackend runtime。
- 前台恢复后 UI 重新探测 service；写入仍受 repo writer gate 控制。
- 后端退出后恢复使用新的随机 endpoint/session，不扫描端口；stale scope 被拒绝且 pending overlay 保留。
- app exit 后 transport、metrics、prewarm、watcher 与 P2P task 不残留；RemoteBrowser 全程不创建 embedded runtime。
- 后台长时同步不可被 UI 暗示为已支持。
