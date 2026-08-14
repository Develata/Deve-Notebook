# 11_ui_design/03_mobile.md - 移动端设计 (Mobile UI)

## Metadata

- `Layer`: `Application / UI Shell`
- `Status`: `Current UI Contract`
- `Version`: `0.0.1`
- `Last Review`: `2026-08-14`
- `Counterpart Feature`: `docs/features/08_ui_design_03_mobile.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/05_ui.md`, `docs/acceptance-cases/12_tech_release.md`, `docs/acceptance-cases/13_ui_mobile_chat_regression.md`, `docs/acceptance-cases/17_mobile_surface_switcher.md`
- `Primary Code Areas`: `apps/web/src/components/mobile_layout/`, `apps/web/src/components/`, `apps/mobile/`

本章定义 Mobile content-first 适配策略。规范性用语继承 `01_terminology.md`。

> **Current Native Boundary**：Mobile native 是与 Web/Docker 等价的 peer 外壳，支持 `LocalBackend` 与 `RemoteBrowser` 两种互斥模式；壳层本身不拥有业务 authority。
> **Post-Gate Target**：Mobile 端目标采用 **Tauri v2 Mobile packaging** 外壳，共享 Web 前端；Android/Mobile `LocalBackend` 默认启动 embedded loopback full peer service，写入仍必须经 server/core writer gate。`RemoteBrowser` 只作为 HTTPS 远端 Web 壳层。

> **Web 映射**：当 Web 端 $W_{view} \le 768px$ 时，界面 **MUST** 遵循本章 Mobile 规范。

## 1. 原生适配器边界 {#mobile-current-native-boundary}

*   Web 端小屏视口 **MUST** 映射到 Mobile 交互规范。
*   Mobile native adapter 第一阶段只允许承担：选择 shell 模式、启动或绑定本机受控 service endpoint、注入 service endpoint/session、报告 readiness/offline 状态、转发前后台、安全区域、系统手势保留区与软键盘等有限平台 presentation 事件，或在 `RemoteBrowser` 中导航到远端 HTTPS origin。
*   默认构建 **MUST** 保持 no-Tauri Mobile skeleton；`tauri` / `tauri-build` dependency 只能作为 `apps/mobile` 的 optional dependency 挂在 `native-packaging` feature 后。
*   `native-packaging` Android/Mobile 默认模式是 `LocalBackend`；Mobile v1 full peer 不使用子进程，而是启动 in-process embedded loopback service，并在 app-private data root 初始化 zero-repo host registries，不依赖 Docker、外部 CLI 或用户手工 init。启动不得自动创建默认 repo/projection；首次 Create 才能经 server/core authority path 建立本地 workspace。
*   `RemoteBrowser` **MUST** 显式选择，且只接受远端 `https://host[:port]` origin。URL 不得包含 userinfo、query、fragment 或业务子路径；壳层不得注入本地 endpoint/session bootstrap，不得启动 embedded service。
*   recovery bootstrap 只能表达 `service_offline`、`foreground_reprobe` 与 `session_invalid` 等结构化状态；后台恢复失败 **MUST NOT** 被伪装成普通断网。
*   Mobile native adapter **MUST NOT** 自行定义 Ledger / Projection Workspace authority、schema migration、source-control 语义、同步合并语义或搜索索引语义；这些仍归 core/server。`LocalBackend` 只允许 native 壳启动/绑定本机 embedded full peer service，不授予 shell 直接写 authority。
*   UI readiness **MUST** 等待受控 service 完成 loopback/IPC endpoint 与认证会话绑定后再打开主界面；后台/离线状态不得导致本地编辑进入未声明的半可写状态。

### 1.1 Minimal Native Adapter Contract {#mobile-native-adapter-contract}

Mobile native adapter 与 Desktop 共用 `./02_desktop.md#desktop-native-adapter-contract` 的 authority 边界：native 壳层只负责进程、平台能力与本机 service 绑定，不拥有 ledger/Projection Workspace/source-control/search 的业务真相。

Packaging dependency gate 见 `17_tech_stack.md#native-packaging-dependency-gate`。

### 1.1.1 Mobile Native Shell Modes {#mobile-native-shell-modes}

`NativeShellMode` 的 Mobile 语义如下：

*   `LocalBackend` 是 native-packaging Android/Mobile 默认模式。Mobile 壳层只负责 embedded loopback lifecycle、endpoint/session bootstrap、foreground reprobe、readiness 展示与失败恢复。
*   `LocalBackend` 的本地数据根位于 app-private data root；server/CLI runtime 只初始化 host registries；零 repo 时不得自动创建隐藏或默认 repo。首次 Create 才建立 Projection Locator、workspace identity、`.notegit/` 与 repo-local `.gitignore`。
*   `LocalBackend` 必须复用 server native-session bridge 完成 session handoff，并以 HttpOnly native session cookie 与 `window.__DEVE_NATIVE_BOOTSTRAP` endpoint payload 启动 Web；bootstrap source 不得包含 token、secret 或 auth material。初次 WebView session prepare 与 resume/replacement handoff 必须共享同一个 process-local single-flight gate；同 generation 的重复 prepare 只能串行复核，不能并发竞争 Android `CookieManager` completion registry。用于阻止页面 reload 重复 prepare 的 browser marker 必须绑定当前 process/session 的非敏感安装身份；不得只绑定可能被新进程复用的 loopback endpoint，也不得让旧进程保存的 bootstrap 覆盖当前 fallback。保存的 bootstrap 必须通过 ready-shape、loopback endpoint 与稳定 capability 校验；同一 WebView process 后续 generation 的 endpoint 只能由 supervisor 的 replacement source 覆写，页面 reload 必须优先采用该同 process 保存值而不是 init script 中已过时的首 generation fallback。browser storage 不可读写或 marker 写后读不一致时必须直接投影 `session_invalid`，不得调用 native handoff 或 reload。已提交的平台写入若超时，仍须保留 fail-closed tombstone 直至迟到 callback 到达或进程重启。
*   Tauri `main` WebView **MUST** 延迟到 embedded service 完成 probe、native session handoff 与 bootstrap plugin 注册之后创建；不得先创建无 endpoint/session bootstrap 的主 WebView。Android Wry 不实现 `WebView::set_cookie`，因此 Android 必须在 WebView 已登记后通过无参数 native command 调用系统 `CookieManager` 安装 HttpOnly cookie，确认成功后才 reload 一次并进入 authenticated runtime；`Secure` cookie 的 retention 复核必须使用与 `setCookie` 相同的 HTTPS secure loopback origin，不得用 HTTP URL 的发送资格过滤误判已完成的平台写入，真实 HTTP loopback session 可用性仍必须由 reload 后的 auth/readiness 闭环证明。cookie/token/secret 不得进入 command 参数、JavaScript 或 bootstrap payload。`RemoteBrowser -> LocalBackend` 新建的 WebView 在调用该 command 或执行 resume handoff 前，还必须等待 native coordinator 确认新 `main` 已登记、recovery anchor 已退休且 `LocalWindowCreated` 已记录；该一次性 admission 只能由 backend/native lifecycle 打开，Web 不得自行推断 Activity 稳定性。
*   Android release 网络安全配置必须保持全局 cleartext fail-closed，只对精确 `127.0.0.1` / `localhost` LocalBackend loopback destination 放行 HTTP；不得为 LocalBackend 开启全局明文、通配域名或 RemoteBrowser HTTP。debug source set 的开发连接策略不构成 release authority。
*   恢复 admission timeout 必须覆盖平台 Activity acknowledgement 与 anchor manager retirement 的完整上限并保留调度余量；timeout 或 supervisor shutdown 必须进入不可迟到放行的 terminal failure、取消所有 waiter。coordinator 的迟到 grant 必须失败并按 post-retirement cold-restart 路径收敛。
*   `RemoteBrowser { https_origin }` 是显式远端模式。壳层必须在创建主 WebView 前把已校验 origin 写入 native `WindowConfig`，不得靠远端页面执行 init-script redirect；后续 `/api` 与 `/ws` 均由浏览器同源规则解析。native 壳不提供本机 session cookie、端口、repo bootstrap、native bootstrap capability 或 Tauri command handler。
*   Mobile Settings 只允许在可信 bundled `LocalBackend` origin 使用 native backend preference bridge：默认 `local`；选择 `remote` 时必须先由 Mobile native 侧短超时探测 `<origin>/api/node/role` 并确认结构化 Deve node role，成功后才写入 app-private `native-backend.json`。bridge capability 必须来自 typed native bootstrap，不得由 `__TAURI_INTERNALS__` 推断。
*   Mobile `remote` preference 只保存 HTTPS origin，不保存远端凭证、session、token、repo scope 或 writer readiness。启动参数/环境覆盖只用于诊断和脚本启动，不得回写 preference。
*   Mobile Tauri bundle 必须加载 `frontendDist` 资产，并通过 native bootstrap 或 RemoteBrowser 导航决定后端；不得把主 WebView 固定到开发服务 `devUrl = http://127.0.0.1:3001`。
*   Mobile 在 `RemoteBrowser` 失联时沿用普通浏览器锁屏/只读语义，远端 DOM 不得调用本机 IPC。Android/iOS 壳层必须提供平台原生 “Use Local Backend” 控件，且仅在 host preference 选择 `RemoteBrowser` 时显示；显式 CLI/env override 下必须隐藏并记录诊断。该控件只提交 native lifecycle intent，不得保留 Web fallback、向远端页面暴露 command handler，或把 `__TAURI_INTERNALS__` 当作 capability。
*   Mobile `RemoteBrowser -> LocalBackend` 由单一 native coordinator 串行执行：先确认 preference 与远端 surface、启动隔离的 `MobileEmbeddedBackendSupervisor` 候选并取得新 bootstrap，再移除平台恢复控件并销毁远端 WebView。候选必须先关闭初次 WebView session admission；只有 coordinator 已确认新 `main` 登记、recovery anchor 退休并记录 `LocalWindowCreated` 后才可一次性打开，且 initial/resume handoff 必须共同受该门约束。独立的 native recovery transition guard 必须在最后一个 WebView 销毁期间阻止 Tauri 提前退出，但不得持有或暴露 writer/IPC authority。Android 必须把该 guard 实现为在远端退休前创建的、未导出且 capability-free 的临时 recovery Activity/WebView：它只能维持 Activity/Tauri lifecycle，不得注册 command/bootstrap plugin、保存业务状态或成为 LocalBackend surface evidence；新 bundled-local `main` Activity/WebView 创建后必须确认其已从 manager 退休。只有确认远端 WebView 已从 manager 退休后，才可持久化 `local` preference、注册 LocalBackend command/bootstrap plugin、manage 新 supervisor 并创建 bundled-local WebView。新 local runtime 必须使用新的随机 loopback endpoint、native session、repo handshake 与 `scope_nonce`，不得迁移远端 cookie、session 或 authority。
*   coordinator 在远端 WebView 销毁前失败时必须停止候选 supervisor、保留 remote preference 与原远端 surface，并把平台控件恢复为可重试；若控件恢复或 supervisor 清理失败，必须重启进程。Activity/WebView 创建或退休一旦已派发但无法确认结果，必须视为 committed-unknown，不得恢复控件或允许同进程重试。远端 WebView 已销毁后的任一步失败都必须在 recovery anchor 仍维持 lifecycle 时先有界停止候选/已托管 supervisor，再按仍在磁盘上的 preference 重启：`local` 已提交时 cold start LocalBackend，尚未提交时 cold start RemoteBrowser。Android cold start 必须先启动未导出、无 Tauri/业务 authority 的独立进程 recovery Activity；该组件只有连续确认旧 PID 已退休后才可创建 launcher task，并必须随即退出自身进程。helper 调度确认后，旧进程必须在 failure snapshot 与有界 supervisor shutdown 已完成的边界直接退休，不能再次受普通 graceful-exit gate 阻塞。不得在旧 Tauri 进程内直接启动新的 `MainActivity`，也不得调用桌面 `current_exe` spawn 冒充 Android restart。不得依赖 best-effort preference rollback，不得留下无窗口的 active runtime，也不得重新向远端页面开放 IPC。并发点击必须 single-flight。
*   native coordinator 必须维护 typed、process-local transition snapshot，至少记录 recovery id、远端 surface retirement、preference/plugin 注册顺序、supervisor/window ownership、active runtime owner 数和失败原因。该 snapshot 只可由 bundled-local command 读取，用于 target-host receipt；RemoteBrowser 不得注册读取命令。receipt 必须把 manager 已确认的远端 surface retirement 与 fresh `http://tauri.localhost` sync CDP target、remote/local repo scope、native session generation 及 graceful process exit 关联；已销毁 WebView 的 Chromium discovery target 可能短暂残留，其消失时机不得作为产品 lifecycle authority，且所有 lifecycle 真值都必须来自实际观测而非写死。
*   从后台恢复时，`LocalBackend` 必须重新 probe session、node role、WS repo handshake 与 current `scope_nonce`；`RemoteBrowser` 的恢复语义等价于浏览器页面恢复，不得伪装本地 authority。bundled Mobile bootstrap 必须携带固定、非敏感的 `platform_lifecycle_authority = native` 标记；只有 native shell origin 上的该 typed marker 可关闭 Web 页面 lifecycle 补偿，使平台 suspend/resume 真值只来自 native `Suspended` / `Resumed` handoff。该模式下 `visibilitychange`、`window focus/blur`、输入法、搜索输入框或平台浮层造成的页面/焦点变化均不得触发 `ForegroundReprobe`；普通 Web、`RemoteBrowser` 与未携带该 marker 的 Desktop 行为不受此 Mobile 规则改写。

**Adapter inputs**:

*   `profile/config/projection-locator/ledger` 选择必须在 service boot 前完成；Web 运行后 native 层不得直接改写后端路径或 repo scope。
*   `launch_intent` 可表达分享、文件打开、deeplink、通知点击等入口，但必须转为 application command；不得绕过 writer gate。
*   `session_material` 必须绑定到当前 app install 与进程会话；不得放入 URL、Web localStorage、日志或系统剪贴板。
*   `platform_lifecycle` 只允许传递 `foreground/background/suspended/resumed/network-online/network-offline/safe-area/keyboard` 等 shell 事件；keyboard/focus 只能作为交互或布局投影，不能升级为 foreground/background authority。

**Adapter outputs**:

*   `NativeEndpointReady { http_base, ws_base, node_role, session_bound }`
*   `NativeServiceOffline { reason, retryable }`
*   `NativeServiceSuspended { reason }`
*   `NativeForegroundReprobe`
*   `NativePlatformEvent { kind }`

**Boot/lifecycle state machine**:

```text
MobileColdStart
  -> ServiceStarting
  -> EndpointBound(http_base, ws_base)
  -> SessionBound
  -> WebShellLoading
  -> RuntimeReady
  -> BackgroundSuspended
  -> ForegroundReprobe
  -> RuntimeReady | ServiceOffline | SessionInvalid
```

`RuntimeReady` 的最小条件与 Desktop 一致：endpoint 可达、`/api/auth/status` 有效、`/api/node/role` 可读、当前 repo 已完成 ws handshake、写入路径满足 `writer_ready(repo_id, scope_nonce)`。`SessionInvalid` 必须进入 `Unauthorized`；后台恢复失败不得伪装成普通断网。

**Endpoint/session injection rules**:

*   Native 壳必须在 Web connection manager 启动前注入 `http_base/ws_base` 与 session 绑定状态；优先使用内存 bridge 或初始 HTML bootstrap。
*   Native 壳可注入只含 `service_state` 的 recovery bootstrap；payload 只能表达 `service_offline`、`foreground_reprobe` 或 `session_invalid`，不得携带 token、session secret、服务失败 reason 或 repo 写权限。
*   `?ws_port=` 只能作为开发期 fallback。mobile production 不得让 Web 端枚举、猜测或扫描本机端口。
*   session 绑定完成前不得打开可写主界面；后台恢复后必须重新 probe session、node role 与 ws repo handshake。
*   bundled Web shell 仍是 WebLightPeer，repo-scoped browser identity 必须满足 `03_storage/index#browser-storage-layering` 的 IndexedDB + non-extractable WebCrypto Ed25519 合同。Android System WebView 不支持 Ed25519 时必须保持 storage-limited 只读；LocalBackend/native session 不得伪造 browser key、跳过签名或转授 host writer authority。
*   Android 正式支持与可写 evidence baseline 是 Android 10 / API 29+ 且当前 WebView provider major 137+。该版本基线只决定 support/receipt 资格，不得替代真实 capability probe，也不得在前端解析 UA 或硬编码 OS gate。
*   Android 可写 target-host gate 必须在业务步骤前记录 API level、WebView provider package/version、AVD/system-image 或真实设备标识，并执行真实的 `crypto.subtle.generateKey({ name: "Ed25519" }, false, ["sign", "verify"])` 探测。只有版本基线与 probe 同时满足才可生成 writable receipt；不满足时必须输出结构化 blocker、证明编辑器保持只读并停止可写业务步骤。
*   Android emulator gate 必须先构建目标 APK、释放构建 daemon，再以 low-RAM 模式、有界 RAM 和 `2048..8192 MiB` 的受控 writable data partition request（默认 `4096 MiB`）启动本次进程持有的专用 serial 与匹配 AVD，并以目标镜像实际发布的 boot-complete property 为准：接受 `sys.boot_completed=1` 或 `dev.bootcomplete=1` 后，仍须在同一 boot deadline 内通过短时有界、只读 probe 确认 package manager 返回包含且仅包含规范 `package:<name>` 行（其中恰有一个 `package:android`），且 `settings get global device_provisioned` 返回规范 `0` / `1`，并要求两项 guest service 在至少 `10s` 的连续稳定窗口内保持 ready，再在安装前 fail-closed 解析 `/data` 容量，证明总容量不低于 request 的四分之三且可用空间至少 `1024 MiB`。稳定窗口内任一已准入的 package/settings bootstrap 暂态必须清零连续窗口并重新计时；未知错误、混合输出、timeout、中断或 process guard failure 必须立即传播，不得被后续成功 probe 覆盖；owned-emulator process guard 必须在最终准入前再次通过。APK install 若命中已准入的 package-service bootstrap/race signature，下一次 install attempt 前必须在同一 absolute install deadline 内重新完成同一连续稳定准入；每个带 timeout kill grace 的 probe 必须从剩余 deadline 预留该 grace，只有 bootstrap 恢复链中的精确 provider-race signature 可进入下一轮重新准入，首次出现、混合输出、未知错误或超时仍立即 fail-closed。缺失 `settings` service、system provider 尚未安装、非规范输出、单次 probe 超时或总 deadline 到期都不得准入 APK 安装；不得增加 install attempt 或用固定 sleep 代替 condition-based readiness。不得选择其他已运行 emulator、不得清理非本 gate 所有的实例、不得因某个镜像只发布其中一项而永久等待，也不得仅凭 adb online 提前执行 package smoke。
*   Android target-host 的首次文档 Create 证据必须把 quiet window 准入得到的精确 `repo_id + scope_nonce` 作为不可变期望值贯穿到 arm，并只在精确 path、唯一稳定 target 与 `ready + 非空 repo_id + 正整数 scope_nonce` 完全匹配时派发一次完整 native touch。该 touch 必须在同一触点保持固定 `50ms` 的有界非零接触时间后才发送 `touchEnd`，不得把相邻的零接触时长命令冒充真实 tap。`touchEnd` 命令完成不等价于浏览器已同步合成 `click`；harness 必须绑定本次 arm token，在不重发 Create 的前提下以固定 `2000ms` deadline 有界等待同一 click settlement，并以最终原子读取与 observer 清理的结果作结算。页面 observation 只记录固定、无敏感信息的 `touchstart / touchend / pointerdown / pointerup / click` 阶段布尔值，不得记录路径、文档内容、session、endpoint 或 credential；阶段只用于定位 transport/gesture/click 边界，不能替代精确 target、writer scope 或 click 成功条件。arm 时先安装页面侧 touch-transport lease，`touchEnd` 返回后再切换为 click-settlement deadline；二者到期都必须自行 expiry/seal，使宿主 finalize 在执行前断连时仍能阻断迟到 click。该 Create lane 在本次 document lifecycle 中是 single-use：成功后结束；错误 target、writer scope 变化、missing observation、driver error、cleanup error 或 settlement timeout 一旦进入 committed-unknown，必须封存并阻断后续迟到 click，只有新的页面 generation 才能重新准入，不得用第二次 Create 掩盖 committed-unknown。
*   Android emulator binary identity probe 必须有独立的单调时间与输出字节上限，只接受 canonical
    `Android emulator version <version> (build_id <id>)` banner 的逐字段匹配；单独出现 version/build
    token、空输出、超时或超限输出都必须 fail-closed。若 emulator wrapper 在输出完整 canonical banner
    后以 non-zero 退出，identity 可成立，但 probe 的退出状态仍须进入诊断，不能由松散 token grep
    冒充 binary identity。checksum-pinned emulator cache 的同一 build publication 必须由 build-scoped
    bounded lock 串行化，并在 lock 内重新校验现有 canonical binary；并发 producer 不得删除另一个
    producer 已发布或正在使用的有效 build。已存在但无法通过 canonical probe 的 cache entry 必须
    保留并 fail-closed，不能自动替换一个可能仍被运行中 emulator 使用的目录。
*   Android emulator renderer receipt 必须根据本次 owned emulator 的实际 bounded log 证明启动走入
    批准的 `swangle`/software renderer path；当前 emulator 对 `-gpu swangle` 的 canonical runtime
    projection 是 `vulkan_mode_selected:swiftshader gles_mode_selected:swangle`：Vulkan 由
    SwiftShader 提供，GLES 由 ANGLE-on-SwiftShader（SwANGLE）提供。当前 pin 只批准这一个有序完整
    pair 作为 swangle 实现证据；`swiftshader/swiftshader`、`swangle/swangle`、`software/software`
    或任意其它 software token 组合都不能冒充等价证据，而 legacy `swiftshader_indirect` 不可接受。
    bounded prefix 内全部 renderer selection 都必须参与一致性判断；证据缺失、冲突或落入未批准 path
    必须在 APK 安装与业务 journey 前 fail-closed。命令行参数只是 intent，不能替代 runtime
    evidence；未来 pin 或 rollback 若产生不同 pair，必须先取得新的 target-host 证据并显式更新合同。
*   Android lifecycle gate 必须在首次文档 editor host 进入当前 generation 的可写状态后立即注入第一笔真实输入并证明内容与 pending 链均保留。跨 transport generation 的 pending 证据必须在旧 generation 暂停一笔真实 outbound edit frame、确认前端 pending 非空、随后丢弃旧 frame 后取得；replacement generation 只能依靠产品 pending replay 使后端首次看到该唯一文本，不能用已写入服务端的 Snapshot 替代 pending 保留证明。CodeMirror mount/cleanup 必须服从 `10_rendering#document-authority-bridge` 的 owner-scoped 生命周期；迟到 cleanup 不得销毁新 WebView editor。

**Offline/background semantics**:

*   `NetworkOffline` 只表示公网不可用；如果 embedded service、session 与 writer gate 仍 ready，本地编辑仍可继续。
*   `BackgroundSuspended` 只允许降低资源占用或暂停远端同步，不得丢弃未 ack 的本地 pending overlay。
*   `ForegroundReprobe` 必须重新执行 `/api/auth/status`、`/api/node/role` 与 repo handshake；旧 `scope_nonce` 不得自动恢复写态。进入 reprobe 时应清空 auth、node-role、repo-handshake、writer-ready 与 scope freshness，直到 fresh readiness 完整通过。
*   `ServiceOffline` 表示本机后端不可达；UI 必须进入恢复/只读状态，不得声称 offline-first 仍可写。

**Forbidden native shortcuts**:

*   native 层不得直接写 ledger/Projection Workspace/source-control/search index。
*   native 层不得直接操作 `.notegit/` 或 `.git/` 来伪造 source-control 成功。
*   safe-area、keyboard、foreground/background、network online/offline 事件不得被解释成业务可写状态。

**Pre-Gate Acceptance Contract**:

*   cold start service bind 失败时显示恢复入口，不进入半可写 UI。
*   session invalid 时进入 `Unauthorized` 并停止普通重连。
*   network offline 但 service/session/writer ready 时，本地编辑继续可用。
*   background/resume 后必须重新握手，stale `scope_nonce` 写入被拒绝。

### 1.2 Embedded Service Supervisor Contract {#mobile-service-supervisor-contract}

Mobile 与 Desktop 共用 `./02_desktop.md#desktop-service-supervisor-contract`
的 supervisor 状态机，但 Mobile 额外保持生命周期约束：

*   `EndpointHealthy` 和 `SessionHandoffReady` 不得绕过 `ForegroundReprobe`；从后台恢复后仍必须重新 probe auth、node role、WS repo handshake 与 current `scope_nonce`。
*   `NetworkOffline` 仍只是公网提示；只有 embedded service health probe 失败才可进入 `ServiceOffline`。
*   `BackgroundSuspended` 不得清空 pending overlay，也不得把旧 supervisor session handoff 直接视为可写。
*   Health probe、retry budget、session handoff failure 分类与 Desktop 一致：bind/probe/process-exit 可预算内 retry，session handoff failure fatal。
*   supervisor 不得写 ledger/Projection Workspace/source-control/search index/`.git`/`.notegit`。
*   `MobileEmbeddedBackendSupervisor` 必须持有唯一 process-scoped `EmbeddedServerRuntime`、当前 transport task、graceful-shutdown sender、随机 loopback endpoint、session generation、`MobileShell` 与结构化 service state；这些对象必须作为一个 lifecycle owner 创建和销毁，不得再次 detach task。`EmbeddedServerRuntime` 只初始化一次 RepoManager、SyncManager、plugin host APIs、watchers 与 background task group；transport restart 不得重开 authority runtime。
*   `RemoteBrowser` 冷启动不得创建或 manage `MobileEmbeddedBackendSupervisor`。从 LocalBackend 切换到 remote 前必须先发送 graceful shutdown，并在有界等待结束后才请求 app restart；从 preference-driven RemoteBrowser 经平台原生控件切回 local 时，只能由 native coordinator 新建 supervisor，且远端 WebView 销毁前不得暴露该 supervisor 的 command/bootstrap capability。
*   CLI native runtime 必须提供 owned `EmbeddedServerRuntime` 与内部 transport graceful-shutdown 入口。正常 app exit 必须先停止 transport，再 cancel/join runtime task group 与 watcher guard，并在超时边界内等待完成。超时或 join/runtime error 必须进入结构化 error 状态，不能报告 clean shutdown。
*   Tauri mobile `WindowEvent::Suspended` 必须立即让 Web writer gate 失效并保留 pending overlay；`WindowEvent::Resumed` 必须进入 `ForegroundReprobe`。若 transport task 已退出，supervisor 必须保留唯一 authority runtime、丢弃旧 transport generation、绑定新的随机 loopback listener、生成新的 native session，并在 generation token 仍为 current 时安装新 cookie/bootstrap 后通知 bundled Web shell；不得扫描端口、复用旧 scope 或重复安装 host authority。Web 只可复用 browser document runtime 的 internal reconnect/session-restore 路径：收到同一 local branch、同一 repo UUID 的新 `RepoSwitched` 后，把该 repo 的 pending rows 从旧 nonce 重绑到新 nonce；用户 scope switch、UUID 变化或页面 reload 不得迁移 pending，且 fresh writer-ready 前不得 replay。
*   backend 存活时 resume 仍必须重新验证 native session 与 node role；Web 必须重新验证 auth、node role、WS repo handshake、writer-ready 与 current `scope_nonce`。任一失败都保持只读，并显示 `foreground_reprobe` / `service_offline` / `session_invalid` 中对应的结构化状态。
*   foreground probe 与 session handoff 必须在 supervisor lock 外执行，并使用 generation token compare-and-set 提交结果；shutdown 必须可以取消 in-flight resume，Tauri lifecycle callback 不得因同步网络请求阻塞 UI thread。
*   bundled Web bootstrap 必须由 current generation 动态提供。旧固定 `js_init_script`、旧 endpoint 或迟到的 probe result 不得覆盖新 generation；Web 只有在 native cookie/bootstrap 安装完成后才收到 resumed/reprobe 事件。
*   `Resumed` 处理必须 single-flight；并发 resume 不得同时创建 transport 或竞争 process-global port hint/node-role projection。probe 返回的 `http_base/ws_base` 必须与本次随机 listener plan 精确一致，startup retry 必须可由 suspend/shutdown 取消，单次阻塞 IO 也必须受短 timeout 约束。
*   replacement cookie、bootstrap 与 `deve-native-resumed` dispatch 必须在 current-generation 校验保护下作为一次 WebView handoff 完成；安装或 dispatch 失败必须把 supervisor 置为结构化 `Error`。Web 连接管理器收到 native resume 后必须通过 typed control 重新读取 current bootstrap、关闭旧 socket 并连接新 endpoint；不得 reload 页面、丢失 pending overlay，或继续 probe 旧 endpoint。
*   `platform_lifecycle_authority = native` 的 bundled Mobile Web 连接管理器不得监听页面 `visibilitychange` 或 `window focus/blur` 来合成 suspend/resume；Android 输入法或搜索输入框导致的 transient focus loss 不得撤销 writer-ready、清空 handshake scope 或请求 endpoint rebind。真实 `WindowEvent::Suspended` / `WindowEvent::Resumed` 仍必须沿 typed native event 路径立即 fail-closed 并重新探测，诊断必须记录固定、无敏感信息的 reprobe source category。该 marker 只选择 lifecycle 事件来源，不授予 endpoint、session、writer 或业务 authority；非 native shell origin 上的同名字段必须忽略。
*   WebView cookie/bootstrap 调用不得持有 supervisor state mutex；handoff 前后必须重新校验 current transition。`resumed`、`suspended` 与 `service-error` 事件必须携带同一 native 单调 transition guard，使迟到事件不能覆盖更新状态。
*   transport 被停止或 fault-injected 后必须立即标记为不可运行；恢复前先完成旧 listener 与全部 upgraded WebSocket session 的 cancellation/join，再创建新 generation。旧 scope 的写入仍由 server writer gate 拒绝，shell 不得自行判断或迁移 authority。
*   transient existing-probe 或 WebView handoff error 必须把当前 transport 标记为 stopping；下一次恢复使用 fresh transport 与 fresh `MobileShell`，不得复用 terminal-offline shell。若 server 明确证明旧 transport 的 upgraded sessions 已全部 retired，则 listener 异常退出仍可安全 replacement。
*   任一旧 transport retirement 无法证明 session idle（timeout、panic、join error 或显式 retirement failure）时，supervisor 必须持久进入 `runtime_restart_required`；本进程后续所有 resume 都 fail-closed，直到 app restart。

### 1.3 Process Adapter Gate {#mobile-process-adapter-decision}

Mobile process adapter gate 对默认 no-Tauri Mobile skeleton 仍关闭；真实 mobile child-process runtime **MUST NOT** 进入默认 no-Tauri Mobile skeleton。Android/Mobile `LocalBackend` 使用 embedded loopback service，不使用移动端子进程；`RemoteBrowser` 关闭 embedded service。

Gate policy 必须满足：

*   默认 no-Tauri `CURRENT_NATIVE_PROCESS_ADAPTER_POLICY.decision =
    DeferredUntilPackagingGate`
*   默认 no-Tauri `child_process_runtime_enabled = false`
*   `packaging_gate_required = true`
*   native shell 直接 `authority_writes_allowed = false`

Mobile `LocalBackend` policy 必须满足：

*   `decision = LocalBackendDefault`
*   `child_process_runtime_enabled = false`
*   `embedded_service_runtime_enabled = true`
*   `packaging_gate_required = true`
*   native shell 直接 `authority_writes_allowed = false`

真实 mobile embedded service runtime 必须位于 Mobile native adapter 的 `native-packaging` feature 后，只做 loopback endpoint、session handoff、foreground reprobe 与 runtime readiness wiring；不得绕过 writer-ready 或 repo scope gate。

### 1.4 Mobile Packaging Scaffold {#mobile-packaging-scaffold}

Mobile packaging scaffold 只描述移动壳层的 dependency spike 与 post-gate 目标能力，**MUST NOT** 被解释为 release ready、process runtime 或 native authority 已显式启用。Android shell-only package execution 由 §1.6 单独门禁；iOS shell-only package execution 由 §1.7 单独门禁：

*   dependency batch: `tauri` runtime crate + `tauri-build` build crate。
*   packaging capability 只覆盖移动壳层能力：WebView shell、permission bridge、share sheet、
    deeplink、file picker、push notification、store package。
*   packaging dependency spike 不得获得 ledger/Projection Workspace/source-control/search index/`.git`/`.notegit`
    authority；这些业务真相仍只归 core/server。
*   lifecycle correctness 仍由 no-packaging mobile skeleton tests 保证：background/resume 后必须
    fresh reprobe auth、node role、WS repo handshake 与 current `scope_nonce`，packaging 不得绕过。
*   `scripts/check-native-track-boundary.sh` 必须继续阻止 packaging dependency 或 import
    泄漏到 workspace root、core、cli、web 或 native 默认构建。

### 1.5 Mobile Packaging Dependency Gate {#mobile-packaging-dependency-gate-decision}

Mobile packaging dependency gate 已进入 Mobile dependency spike；真实 `tauri` / `tauri-build`
dependency 只允许作为 `apps/mobile` 的 optional dependency，并且必须挂在 `native-packaging`
feature 后。默认 workspace 构建仍 **MUST** 保持 no-Tauri。

Gate policy 必须满足：

*   `CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY.decision =
    DesktopAndMobileDependencySpikeOpen`
*   `desktop_tauri_dependencies_allowed = true`
*   `mobile_tauri_dependencies_allowed = true`
*   `default_build_remains_no_tauri = true`
*   `native_feature_gate_required = true`
*   `authority_writes_allowed = false`

当前 gate 分为四层：

*   Mobile dependency spike 已打开：`tauri` / `tauri-build` 只能作为 `apps/mobile`
    的 optional dependency 存在。
*   Android shell-only package execution 可按 §1.6 单独打开。
*   iOS shell-only package execution 可按 §1.7 单独打开。
*   Mobile child-process supervision 与 native shell direct authority write path 仍未打开；Android/Mobile `LocalBackend` 只打开 embedded loopback service 承载。

Foreground reprobe、writer-ready 与 repo scope gate **MUST NOT** 被 native runtime 绕过。

### 1.6 Android Shell-only Package Execution Gate {#mobile-android-shell-package-execution-gate}

Android shell-only package execution gate 只允许把 Mobile WebView 壳层推进到 Android target-host package execution；它不是 Mobile release ready，也不是 process adapter gate。

Gate policy 必须满足：

*   Android required preflight **MUST** 先通过。
*   Android project generation 与 package build **MAY** 只在 `apps/mobile` 的 `native-packaging`
    feature 与显式 target-host script 下执行。
*   Android package build **MUST** 声明 WebView shell、manifest、permission bridge、
    deeplink/share/file/store package 等壳层能力；`LocalBackend` 只通过 embedded loopback service 承载本机 full peer，不得使用子进程。
*   Android package build **MUST NOT** 启动、持有、重启后端子进程。
*   Android package build **MUST NOT** 写 ledger/Projection Workspace/source-control/search index/`.git`/`.notegit`。
*   Android package build **MUST NOT** 绕过 foreground reprobe、session handoff、node-role
    probe、repo handshake、writer-ready 或 current `scope_nonce`。
*   Android package execution 成功 **MUST NOT** 声明 iOS ready、Desktop ready、release ready
    或 process runtime ready。

### 1.7 iOS Shell-only Package Execution Gate {#mobile-ios-shell-package-execution-gate}

iOS shell-only package execution gate 只允许把 Mobile WebView 壳层推进到 iOS target-host package execution；它不是 Mobile release ready，也不是 process adapter gate。

Gate policy 必须满足：

*   iOS required preflight **MUST** 先在 macOS target host 通过。
*   iOS project generation 与 package build **MAY** 只在 `apps/mobile` 的 `native-packaging`
    feature 与显式 target-host script 下执行。
*   iOS package build **MUST** 只声明 WebView shell、manifest、permission bridge、
    deeplink/share/file/store package 等壳层能力。
*   iOS package build **MUST NOT** 启动、持有、重启后端子进程。
*   iOS package build **MUST NOT** 写 ledger/Projection Workspace/source-control/search index/`.git`/`.notegit`。
*   iOS package build **MUST NOT** 绕过 foreground reprobe、session handoff、node-role
    probe、repo handshake、writer-ready 或 current `scope_nonce`。
*   iOS package execution 成功 **MUST NOT** 声明 Android ready、Desktop ready、release ready
    或 process runtime ready。

## 2. Responsive Architecture {#mobile-responsive-layout}

### 2.1 布局状态机 (Layout State Machine)
系统布局 $L$ 根据视口宽度 $W_{view}$ 在两种状态间切换：

*   **Desktop State**: $W_{view} > 768px \implies$ Grid Layout.
*   **Mobile State**: $W_{view} \le 768px \implies$ Stack Layout.

### 2.2 视口配置 (Viewport Configuration)
HTML Header **MUST** 适配刘海屏并禁止 iOS 自动缩放：

```html
<meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover">
```

并且所有固定定位元素 **MUST** 使用 CSS `env()` 适配安全区域：
*   `padding-top: env(safe-area-inset-top)`
*   `padding-bottom: env(safe-area-inset-bottom)`

## 3. Interaction Design {#mobile-interaction-design}

### 3.1 导航策略 (Navigation)
移动端移除常驻侧边栏，改为 **Drawer (抽屉)** 模式。

*   **Left Drawer (Sidebar)**:
*   **Trigger**: 左上角汉堡菜单 (`≡`) 或 **屏幕左边缘右滑 (Edge Swipe)**。
*   **Visual**: 覆盖在编辑器之上，背景带有半透明 Backdrop (`z-index: 100`).
*   **Right Drawer (Outline)**:
*   **Trigger**: 编辑器内容区右上角的 `Toggle Outline` 浮动图标（非 Top Bar）或 **屏幕右边缘左滑**。

### 3.2 面板宽度策略 (Panel Width Policy)

*   **Resizable Handles**: 移动端 **SHOULD NOT** 显示左右拉伸手柄。
*   **Persistence**: 仍可读取已保存的桌面宽度，但移动端不提供调整入口。
*   **Outer Gutter**: 移动端 **SHOULD NOT** 提供外边距拖拽。

### 3.3 虚拟辅助键盘栏 (Mobile Toolbar)
系统 **MUST** 在软键盘上方渲染 Markdown Accessory View。

**Key Layout (Visual Representation)**:

| `↩` Undo | `↪` Redo | `⇥` Tab | `H`ead | `•` List | `☑` Task | `B`old | `I`talic | `<>` Code |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| Cmd+Z | Cmd+Shift+Z | Indent | `#` | `-` | `[ ]` | `**` | `_` | \` |

**Technical Constraint**:
必须使用 `visualViewport` API 监听键盘高度变化，动态调整 Toolbar 的 `bottom` 偏移量，防止被键盘遮挡；
该浏览器标准信号是所有 Web surface 的首选输入。bundled Android 必须按同一 viewport 宽度与同一 WebView
presentation generation 保存最近一次由 native `imeVisible = false` 准入的稳定隐藏态 viewport 高度；同宽度窗口永久
变矮或变高后必须以最近值替换历史值，不得永久保留最大高度。当前 viewport 相对该基线
已经收缩时，即使 `innerHeight - visualViewport.height = 0`，也必须把键盘归类为 resize presentation，额外 bottom
offset 保持为 `0`；只有 current-generation snapshot 可用、当前 overlap 为 `0`、隐藏态基线存在且 viewport 相对
基线未收缩时，Web 才允许使用同一 snapshot 的 native IME bottom inset 作为 overlay 显示回退。宽度或 generation
变化必须撤销旧基线并在新的隐藏态重建；两路高度不得相加或重复施加 padding。键盘是否可见与是否需要额外
offset 是两个不同 presentation 事实，禁止用伪造的 `1px` offset 驱动 Toolbar 可见性。
Android `MainActivity` **MUST** 在 manifest 明确声明 `android:windowSoftInputMode="adjustResize"`；不得依赖
`adjustUnspecified` 让系统在 resize、pan 或 overlay 之间猜测。该 platform contract 请求平台优先把 IME overlap
投影为缩小后的 WebView/`visualViewport`；但 OEM WebView 即使收到有效 root IME Insets 也可能不缩小
`visualViewport`，因此不能把 resize 当成唯一可用的 presentation 证据。它不授予 DOM focus、editor writer 或
lifecycle authority。
由于 edge-to-edge/custom WebView layout 可能让 WebView 的默认 overlap bounds check 把 IME Insets 归零，Android
presentation adapter **MUST** 是当前生成壳中 current-generation Wry WebView
`OnApplyWindowInsetsListener` 单一槽的唯一 owner，并原样返回未修改的 `WindowInsetsCompat`，不得 consume、zero 或
同时施加 native padding；当前生成的 Wry bridge 不得再占用该槽，若未来生成代码新增 owner，静态合同门必须
fail-closed，不能以覆盖未知 listener 的方式继续打包。attach 后必须请求一次 Insets dispatch，detach/replacement
只清理本 adapter 独占的 listener。adapter 必须把 decor-root 的 `imeVisible / imeBottomPx` 与
`systemGestures` 一并写入既有 `(generation, epoch)` presentation snapshot；每个新 epoch 仍先发布 pending 使旧
gesture/IME geometry 同时失效，只有 current valid snapshot 才可重新准入。该 listener 只承载 presentation 输入，
不改变 DOM/editor 或业务 authority。Insets
dispatch 必须输出固定、无敏感信息的 `android_webview_ime_insets_applied` checkpoint；只允许记录 current WebView
generation、IME visible、WebView/decor-root bottom inset 与 WebView height 等 presentation geometry，不得记录输入内容、selection、session
或 endpoint。`imeVisible = true` 但 `imeBottomPx <= 1`、数值非有限或几何越界时，native fallback 必须保持为 `0`，
输出固定 `android_webview_ime_overlay_or_unavailable`，并允许用户再次轻触编辑器重试；不得根据输入法名称、屏幕比例
或固定常量伪造键盘高度。native geometry 读取必须区分 `Ready / ImeOverlayOrUnavailable / Unavailable`；IME 可见但
bottom 为 `0/1px`、越过 decor-root 高度或 density/geometry 无效时，每个 publish epoch 至多输出一次专用诊断，
不得只落入通用 gesture unavailable。usable native fallback 只允许调整 editor 可视区域、Accessory Toolbar 与同一移动壳层的
底部 presentation；不得改变 Markdown、selection、OpenDoc、writer/session/scope、lifecycle 或 foreground authority。
软键盘引发的 viewport resize 在宽度仍处于同一 responsive breakpoint 时 **MUST NOT** 重建 MobileLayout 或 Editor；breakpoint 状态只允许在实际跨越阈值时更新，当前 editor mount、projection load session 与键盘焦点必须保持不变。
target-host 证据必须把 editor load session 身份与 selection 身份分开比较：真实轻触建立 input connection 可以按用户触点移动
caret，因此不得把该预期变化误报为 session replacement；IME Back、presentation 更新等不应移动 caret 的阶段仍必须以独立
selection predicate 精确绑定 `from / to / rangeCount`。不含用户触摸的 IME Back 隐藏态只有在 load session 与 selection
同时恢复到稳定匹配后才可准入，不得先接受第一个 hidden presentation frame 再把过渡态误报为 replacement；随后为重新建立
input connection 发出的真实轻触只绑定 load session，允许 caret 按该触点移动。
Android 正式 target-host/emulator receipt 必须继续以默认 package session 在退出时卸载精确应用包，保证跨运行隔离；
uninstall 也必须返回唯一规范 `Success`，不得吞掉命令失败或异常输出后生成成功 receipt。
本地物理设备诊断可显式设置 `DEVE_MOBILE_ANDROID_PRESERVE_PACKAGE=1` 选择覆盖更新：该模式只有在精确包已安装时才准入，
使后续 `adb install -r` 不触发 fresh-install 授权，并在每次已准入退出时先停止应用、再以 `pm clear` 清除测试数据但保留包。
保留模式只用于本地快速定位，不得生成或替代正式 receipt；`pm clear` 必须返回唯一规范 `Success`，否则固定分类并 fail-closed。
两个正式 Android producer 必须把 preserve mode 显式钉死为 `0`；若本地 preserve mode 同时收到任何 formal evidence/claims
输出路径，package session 必须在安装和设备状态变更前拒绝运行，不能让宿主继承环境污染正式证据。
当业务 journey 与任一模式的 package 清理同时失败时必须保留业务失败为主状态；只有业务成功而清理失败时，清理失败才成为最终 blocker。
Android adapter 必须在 WebView attach 与 Activity 重新取得 window focus 时恢复 current-generation Wry WebView
的 native View focus，使后续真实编辑器触摸能够建立受系统服务的 input connection；该操作不得改变 DOM selection、
主动弹出 IME 或绕过 editor readonly/writer gate。不得用 JavaScript 定时重复 `focus()` / `showSoftInput()` 掩盖
native View 未被 InputMethodManager served 的问题；恢复失败必须输出固定 `android_webview_input_focus_unavailable`。
当用户以同一 pointer 的真实 `ACTION_DOWN` / `ACTION_UP` 轻触序列再次点击同一已聚焦的可写 `.cm-content` 时，
Android adapter 还必须在 current WebView generation 内先按平台 `touchSlop`、长按阈值、pointer 数量、cancel/outside
与 WebView 可见边界排除滚动、拖动、长按和多指手势，再以抬手坐标和 `elementFromPoint` 复核触点确实属于
active editable editor 后请求 IME show；
这样关闭 IME 不必通过 blur/re-focus 改写 CodeMirror selection。非 editor 触点、synthetic Web event、只读 editor、
迟到 WebView callback、Activity 已失焦、WebView 已 detached/hidden 与无效几何均不得弹出 IME；平台 show
异常必须收敛为固定 `android_webview_ime_show_failed` 诊断，不得导致 Activity 退出。
Android 平台 Back 在 IME 可见时 **MUST** 只关闭 IME，不得同时向 `UiBackCoordinator` 发送文档/抽屉返回；
关闭后必须保留当前可写 CodeMirror host、OpenDoc request 与 DOM focus，使用户再次点击同一编辑器时可重新建立
原生 input connection 并唤起 IME。若平台暂时无法取得 root Insets，必须记录固定
`android_ui_back_ime_visibility_unavailable` 并保持 editor/Activity，不得把 unknown 当作 hidden 后继续关闭文档。
撤销与重做按钮 **MUST** 与其它写动作共用 repo writer gate；只读、握手中、快照加载中、writer 未就绪或 scope switching 时不得触发编辑器 history action。
撤销与重做属于高频恢复操作，**MUST** 保持在移动工具栏前段，390px 宽度下无需横向滚动即可看到。
Toolbar **SHOULD** 仅在软键盘可见时显示；软键盘弹出时底部状态栏可暂时让位以优先输入。
Task 按钮必须发送 `10_rendering.md` 定义的 `InsertTaskItem` 语义 intent；第一次 Enter 继续任务项，
第二次在该 continuation 生成的新空任务项上按 Enter 必须立即退出列表，不得要求第三次 Enter；
该行为只能由 intent-local 两阶段 marker 实现，不得插入零宽字符、改写全局 Enter keymap，或改变
普通键盘输入创建的空任务项行为。

### 3.4 手势系统 (Gesture System)
仅支持轻量级 Edge Swipe，参数定义如下：
*   $Zone_{app} = 20\text{ CSS px}$。普通 Web 从屏幕边缘起算；Android native 必须从
    `WindowInsets.Type.systemGestures()` 的左右系统保留区内侧起算。考虑 OEM 可能用独立系统
    Gesture Stub 拦截边缘、却把标准 Insets 错报为零，归一化安全宽度为
    $S_L = \max(\lceil I_L / density\rceil, 24\text{ CSS px})$、右侧对称；左侧激活带为
    $[S_L + 1,\ S_L + 1 + Zone_{app}]$。24 CSS px 是 Android native 的跨密度保守安全下限，
    不是设备物理像素常量；非零标准 Insets 大于该下限时必须以标准 Insets 为准。
    应用不得在系统保留区内抢占触摸，也不得把激活带扩大到全屏。
*   $Threshold_{swipe} = 50px$ (触发滑动的最小距离)。
*   **Direction**: 左边缘向右滑打开 Sidebar / File Tree；右边缘向左滑打开 Outline。Drawer 已打开时，反向滑动只关闭当前 Drawer，不得在同一手势中串联打开另一侧。
*   **Axis Lock**: 只接受单指、水平位移达到阈值且水平位移绝对值大于垂直位移绝对值的手势；短拖动、纵向滚动、斜向滚动、多指手势与取消事件不得改变 Drawer 状态。
*   **Editor Reachability**: CodeMirror 编辑内容区可以作为边缘手势起点；手势识别只产生 typed Drawer intent，不读取、修改或提交 Markdown 内容，也不得改变 pending / writer gate / repo scope。
*   **Interactive Safety**: Edge Swipe **MUST NOT** 抢占靠边可交互控件的真实点击，例如 `File tree`、`Toggle Outline` 等按钮。识别器在手势达到阈值前不得 `preventDefault` 或触发 Drawer intent；button、link、input、select 及显式 `data-no-edge-swipe` target 必须被排除。
*   **Typed Presentation Hint**: Android adapter 必须按 WebView generation 发布固定、无敏感信息的
    JavaScript wire event
    `deve-native-presentation-change { generation, epoch, widthPx, heightPx, leftPx, rightPx, density, imeVisible, imeBottomPx }`。每轮 Insets 读取必须先发送
    同 order 的 `system-gesture-insets-pending` 撤销旧激活带；Web 必须校验 generation/epoch、有限数值与几何边界后
    换算为 CSS px；bundled native 在首个有效 hint 前及任何 current pending/invalid hint 后 fail-closed，不得退回
    与系统 Back 重叠的绝对边缘。该 hint 只选择 presentation geometry，不授予 endpoint、session、writer 或
    业务 authority；WebView replacement 的迟到 hint 必须丢弃。
*   **Document Lifecycle**: Android adapter 必须以 `DOCUMENT_START_SCRIPT` + main-frame message bridge 为每个
    新 document（包含同一 WebView 的 reload 与 RemoteBrowser navigation）设置独立的
    `__DEVE_ANDROID_PRESENTATION_PENDING__` presentation marker，并请求 current-generation snapshot。该 bridge
    只能请求 presentation，不能携带 endpoint/session/cookie；旧 document、subframe 与 replacement WebView
    的消息必须丢弃。普通 Web 未看到 marker 时立即使用普通边缘语义；任何 native marker 或 event 都只允许
    valid hint 解锁激活带，invalid/missing hint 必须保持 fail-closed。delivery 重试耗尽必须输出固定、无敏感信息的
    `android_system_gesture_insets_unavailable`，等待下一个 document/focus/layout/Insets 生命周期入口重试。
    系统 Insets 变化的独立 lifecycle trigger 必须由 adapter 自有、零尺寸且不接收触摸的 sibling observer 承载；
    该 observer 与前述 adapter 独占的 WebView IME passthrough listener 是两个不同职责，不得覆盖或替代 source slot，
    也不得用永久轮询代替 document 生命周期。

## 4. Visual Adaptations

### 4.1 布局约束
*   **Diff View**: 移动端 **MUST NOT** 使用左右并排 (Side-by-Side) 对比，而应强制回退到 **Unified View** (单列混合)。
*   **Font Size**: 默认字号 **SHOULD** 设为 `16px` 以避免 iOS Safari 输入时强制放大页面。

### 4.2 只读模式指示器 (Spectator Indicator)
在 Spectator Mode 下，顶部导航栏下方 **MUST** 插入一条醒目的橙色横幅 (`Height: 24px`)，提示 "Read-Only Mode"。

## 5. Mobile UI Layout

### 5.1 结构层级 (Hierarchy)
*   **Top App Bar**: 固定顶部，包含导航与核心操作。
*   **Content Stack**: 单列内容区，默认全屏编辑器。
*   **Bottom Bar**: 与 Desktop 功能对齐（Branch/连接状态/加载状态/历史条/统计），移动端允许多行折叠布局。
    *   默认折叠态 **MUST** 仅显示一行：`Branch / Ready / Words / Lines / Col`。
    *   通过右侧箭头按钮展开详情；再次点击或点击状态栏外区域自动收起。
    *   折叠态信息 **SHOULD** 无需横向滚动。
    *   分支名在折叠态 **SHOULD** 自动截断，避免挤压状态与统计信息。

### 5.2 顶部导航栏 (Top App Bar)
*   **Left**: Hamburger Menu (`≡`) 打开 Sidebar Drawer。
    *   该入口按钮文案/可访问性语义建议统一为 `File tree`。
*   **Center**: 文档标题/仓库名（省略溢出）。
*   **Right**: Home / Open / Command（与 Desktop 顶栏语义一致）。

### 5.3 Drawer 规范 (Side Drawers)
*   **Sidebar Drawer**:
    *   内容：文件树、快速操作、新建。
    *   关闭按钮建议使用 `X` 图标而非文本 `Close`，以符合移动端通用习惯。
    *   行为：点击文件后自动收起。
    *   `More(...)` 菜单 **MUST** 复用桌面端语义：整行点击切换视图，`Pin/Unpin` 仅修改固定状态，不得伪装成“点击无反应”。
    *   Source Control tab **MUST** 复用共享 Source Control read surface 与 read gate，只显示
        `Confirmed Ledger Changes`、commit/history/graph；External Changes 的 staged/unstaged groups
        必须留在独立 sibling view，普通入口不得退化成 `ngit status` CLI-only 诊断。
    *   Remote Import tab **MUST** 是 Source Control 与 External Changes 的第三个同级入口，复用共享
        typed diff/render primitive，但拥有独立 `remote_import_client`。候选行、blocker 与
        whole-session Apply/Refresh/Discard 在 390px 宽度下不得横向溢出，所有动作满足 44px touch target。
    *   Remote Import 首版不得显示 checkbox 或逐文件 Apply；UI 只消费 backend label、typed state 与
        typed blocker，不解析 raw detail、locator、provider/host/blob path 或 digest。
*   **Outline Drawer**:
    *   内容：标题结构、大纲条目。
    *   行为：点击条目后自动收起并滚动定位。

### 5.4 编辑器区 (Editor)
*   **Mode**: 单列布局，支持全屏编辑。
*   **Diff**: 强制 Unified View。
*   **Selection**: 单指选区，长按弹出操作菜单。

### 5.4.1 Mobile Surface Switcher {#mobile-surface-switcher}

移动端必须复用 `index.md#editor-group-tabstrip` 定义的 document/diff tab identity 与
view-local lifecycle，但不得直接复制桌面横向 tabstrip。

*   顶部当前 surface 胶囊 **MUST** 位于 sync banner 与 content stack 之间，显示当前
    document/diff 名称、surface 类型与已打开数量。
*   点击胶囊 **MUST** 打开底部 sheet；sheet 分组显示 Documents 与 Diffs，并提供选择、
    关闭与 active 标记。
*   sheet 行与关闭按钮 **MUST** 满足移动端 44px touch target。
*   选择 document 必须复用 guarded document navigation；选择 diff 只能恢复已有
    view-local `DiffSession`，不得触发新的 diff 计算。
*   关闭 active diff 只关闭 diff surface，不得修改 staged、pending 或 commit state。
*   repo / branch / scope 切换、drawer 打开、选择条目或关闭当前 surface 时，sheet
    **MUST** 自动收起。
*   sheet 打开时 **MUST** 避免 AI Chat、辅助键盘栏与 Bottom Bar 遮挡。
*   sheet 必须使用 `role="dialog"` 与 `aria-modal="true"`；打开后初始焦点进入关闭按钮，
    `Tab` / `Shift+Tab` 必须困在 sheet 内，`Escape` 关闭，关闭后恢复到触发胶囊（若仍可聚焦）。

### 5.5 快捷入口 (Quick Actions)
*   **Search**: 打开 Quick Open / Command Palette（移动端应为底部抽屉）。
*   **Sync**: 可在 More 菜单中触发。

## 6. Key Flows

### 6.1 打开文档
1. 点击 Hamburger -> 打开 Sidebar Drawer。
2. 选择文档 -> Drawer 自动收起 -> Editor 渲染。

### 6.2 查看大纲
1. 点击编辑器内容区右上角 `Toggle Outline` 图标 -> 打开右侧 Drawer。
2. 点击条目 -> Drawer 自动收起 -> Editor 滚动定位。

### 6.3 搜索/命令
1. 点击 Search -> Top Sheet 自上而下展开。
2. 选择结果 -> 自动关闭并跳转。
3. 关闭手势以顶部拖拽上滑为主（避免与结果列表滚动冲突）。

补充约束：

*   Sheet 必须提供黑色文本字符 `×` 的显式关闭按钮，触控区至少 44×44 CSS px，并带可访问名称；
    不得使用彩色 emoji 代替。
*   移动 Sheet 打开时初始焦点进入 dialog/关闭按钮，不得自动聚焦搜索输入框或主动弹出 IME；
    只有用户明确点击输入框后才请求输入焦点。Desktop overlay 仍可按桌面合同 autofocus。
*   `×`、外部点击、上滑、`Escape` 与选中结果必须汇入同一个 overlay close transition，关闭后恢复
    到仍可聚焦的触发控件。
*   Android system back 必须先关闭可见 IME；IME 已隐藏时才委派给
    `index.md#overlay-back-coordination` 的 `UiBackCoordinator`：先关闭最上层 presentation surface，再经过
    document pending-edit guard。typed `Unhandled` ack 只把 root task 移到后台，Activity/PID 不得由该路径
    主动结束；重新进入后仍必须完成 native lifecycle rebind。不得直接调用 `WebView.goBack()`，也不得在 ack
    超时后猜测退出或后台化。
*   Android target-host 的 root Back 证据必须先显式关闭并等待本场景打开的 repo switcher、菜单、dialog 与左右
    Drawer 全部收敛，再发送唯一一次用于证明 `Unhandled -> background` 的 Back；不得用连续 Back 掩盖测试编排
    泄漏的 presentation surface，也不得把“关闭 Drawer”的 handled transition 冒充 root background。
*   target-host 对 Activity 前后台的读取必须识别 AOSP `mResumedActivity` 以及 Android 15/OEM 可能投影的
    `topResumedActivity` / `ResumedActivity` 规范记录，并按精确 package component 分类；不存在任何已批准 key、
    已批准 key 无法解析唯一规范 component、多个 key 的 component 冲突、package 前缀碰撞或输出异常时必须返回
    unavailable 并阻断证据，不能把未知格式当作已退后台，也不能从冲突样本中任选一个状态。

## 7. Performance & Size
*   **Target**: 首屏渲染 < 1s，输入延迟 < 16ms。
*   **Memory**: 低端设备 **MUST** 平稳运行。
*   **Dependency**: 移动端 **MUST** 避免重型 UI 框架。

### 7.1 Visual Reference (Yuque-Inspired)
*   **Tone**: 轻盈、克制、阅读优先。
*   **Layout**: 卡片化信息层级，内容区留白适中。
*   **Typography**: 标题略微加重，正文中性字重，行高舒适。
*   **Surface**: 浅色背景 + 轻阴影，强调内容层次而非装饰。
*   **Interaction**: 抽屉与底部面板动效柔和，避免夸张动画。

## 8. Related Configuration

*   `ui.mobile.font_size`: 移动端专用字号 (Default: 16).
*   `ui.mobile.toolbar_visible`: 是否显示辅助键盘栏 (Default: true).

## 9. Post-Gate Implementation Target

### 9.1 移动端 UI 方案

本节是 post-gate normative target：只有 `native-packaging` 与 process adapter gate 显式打开后，以下规则才进入验收；默认 no-Tauri Mobile skeleton 仍以 §1 为准。

*   **Rule**: Mobile post-gate 采用 **Tauri v2 Mobile** 作为外壳（WKWebView / Android WebView），前端代码与 Web 端共享，配合原生层访问摄像头/文件系统/推送等系统 API。
*   **Consistency**: 交互与布局规则 **MUST** 与本章一致，行为不以 Web 端为准。

### 9.2 Common Post-Gate Contract

Mobile post-gate **MUST** 服从 `./index.md#native-post-gate-common-contract`。

### 9.3 Mobile Deltas

*   App 进入后台时 **SHOULD** 降低服务资源占用；恢复前台时自动唤醒。
*   post-gate 无公网时 **MUST** 提供本地读写能力；完整后台/索引/同步能力仍受 platform lifecycle、profile、feature 与资源预算约束。
*   网络恢复后 **SHOULD** 执行增量同步；失败不得影响本地编辑。
*   体积 **MUST** 可控，避免引入重型依赖。
*   输入延迟与滚动流畅性 **MUST** 优先保障。

## 10. Web Mobile Alignment Contract

在 `W_view <= 768px` 的 Web 视口中，Web shell **MUST** 按本章 Mobile 规范降级，而不是继续套用 Desktop 交互。

必须保持的对齐点：

*   移动端 **MUST NOT** 显示桌面左右拉伸手柄或外边距拖拽入口。
*   主编辑区默认字号 **SHOULD** 不低于 `16px`，避免 iOS 输入焦点自动缩放。
*   左右 Drawer、Top Sheet、Bottom Sheet、Outline、Search Result、Source Control、External Changes
    与 Remote Import 面板 **MUST** 遵守同一套 touch target、focus 与 selected/active 语义。
*   Bottom Sheet 手势关闭 **MUST** 具备阈值、防抖与滚动冲突判定；轻微位移不得误关闭。
*   边缘滑动 **MUST NOT** 抢占靠边真实控件点击。
*   左边缘右滑与右边缘左滑 **MUST** 分别只产生 Sidebar / Outline typed Drawer intent；纵向滚动、多指、短拖动与取消事件不得打开 Drawer，编辑器边缘仍必须可触发。
*   `More(...)` 菜单 **MUST** 复用桌面端语义：整行点击切换视图，`Pin/Unpin` 只改变固定状态。
*   移动端 Diff **MUST** 使用 Unified View，并避免 AI Chat、辅助键盘栏或抽屉层级遮挡 diff 操作。
*   移动端 AI Chat **SHOULD** 以页面级或全屏交互呈现，避免半屏手势与编辑器/键盘层级冲突。

## 11. Mobile SHOULD Boundary

以下 SHOULD 只定义责任边界；Web 映射不得据此扩展为原生能力声明：

*   Resizable Handles：移动端 **SHOULD NOT** 显示左右拉伸手柄。
*   Outer Gutter：移动端 **SHOULD NOT** 提供外边距拖拽。
*   Font Size：移动端编辑器默认字号 **SHOULD** 设为 `16px` 或更高。
*   Background Resource：原生 Mobile App 后台时服务 **SHOULD** 降低资源占用；Web 映射只能表达前台视口行为。
*   Firewall：移动端内嵌服务 **SHOULD** 显式阻断非回环访问。
*   Export：单文档/全量导出属于原生端或后端服务能力，**SHOULD** 由对应章节定义实现路径。
*   Audit：关键操作日志属于 core/service 审计链路，**SHOULD** 避免塞入移动 UI 层。
*   Recovery Drill：恢复演练属于 release/ops 能力，**SHOULD** 由发布与运维流程承接。
