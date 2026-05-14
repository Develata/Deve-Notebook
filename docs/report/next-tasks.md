# 当前下一步任务

> 更新日期：2026-05-14
>
> 本文件只记录 active execution queue。完成历史进入 `docs/report/*-baseline-YYYY-MM-DD.md`。

## 当前执行队列

1. Error-code catalog plan patch：只在明确允许修改 `docs/plan/` 时，将 `SC_COMMIT_DIFF_UNPROJECTABLE` 与 `GRAPH_DEGRADED_PROJECTION_REQUIRED` 补入 `11_i18n.md`；否则继续保持 plan 不动。
2. Native packaging gate opening decision：只有明确批准打开 `native-packaging` dependency gate 时，才执行 `NPG-1 Desktop Packaging Dependency Spike`；否则继续保持 no-Tauri skeleton。
3. Chrome MCP isolated browser smoke refresh：若前两项继续不开放，则下一批用隔离数据根验证当前 Web runtime 的 create/open/edit/save/reload/reconnect 与 console/network 健康；只有 smoke 暴露具体 bug 时才进入实现。
4. Mainline implementation gap scan：若 browser smoke 仍无缺陷，则下一批只做 fresh gap scan 或用户指定的非平台化 Current MUST；不得用泛化重构替代明确缺口。

## 最近完成

- Mainline gap rescan after native port validation：新增 `mainline-gap-rescan-after-native-port-validation-2026-05-14.md`，复跑 network/native/release/dev-runbook guard、runtime happy/recovery smoke 与 plan coverage；当前未发现 unblocked 主线代码缺口，下一步收敛为 Chrome MCP 隔离浏览器 smoke refresh。
- Native endpoint port validation：新增 `native-endpoint-port-validation-2026-05-14.md`，将 Native endpoint validator 的显式端口校验从“数字即可”收敛为有效非零 `u16`，Web native bootstrap 继承同一 `InvalidEndpoint` fail-closed 语义。
- WS port query validation：新增 `ws-port-query-validation-2026-05-14.md`，将 Web runtime 的 `?ws_port=` override 收敛为有效非零 `u16` 端口输入，避免 URL 外部输入生成无效 WebSocket candidate。
- WS HTTP base derivation boundary：新增 `ws-http-base-derivation-boundary-2026-05-14.md`，将 Web node-role probe 的 WS-to-HTTP base 推导从全局字符串替换收敛为只处理 leading scheme 与末尾 `/ws`，避免边界输入中 path/query 文本被误改写。
- Web API query encoding：新增 `web-api-query-encoding-2026-05-14.md`，为 Web HTTP adapter 增加共享 query component encoder，并覆盖 Graph projection 与 Git mirror readonly repair review 的 `repo_id` 参数，避免保留字符造成 query 结构歧义。
- Auth config env no-panic：新增 `auth-config-env-no-panic-2026-05-14.md`，将 `AuthConfig::from_env` 的 `AUTH_SECRET/AUTH_PASS` 局部 `expect("checked above")` 改为显式模式匹配；生产缺失配置仍 fail-closed，`DEVE_ENV=development` 显式开发 fallback 语义不变。
- Git push target no-panic：新增 `git-push-target-no-panic-2026-05-14.md`，将 Git mirror push 执行前的 `remote/branch` 解析 `expect` 改为最终显式 target guard；异常内部状态返回 `git_remote` blocker，不触发 panic，正常 preflight/mapping/push 语义不变。
- Web entry DOM no-panic：新增 `web-entry-dom-no-panic-2026-05-14.md`，将 Web WASM 入口的 `window/document` 直接 `unwrap` 改为显式宿主能力检查；缺少浏览器 DOM 时记录错误并跳过挂载，正常浏览器 boot panel、loading overlay 与 Leptos mount 路径不变。
- HTTP surface response no-panic：新增 `http-surface-response-no-panic-2026-05-14.md`，将安全响应头 `.parse().unwrap()` 与静态/embedded SPA response builder `expect(...)` 改为不可失败的显式 `HeaderValue::from_static` / `Response::new` 构造，保持状态码、Content-Type、SPA fallback 与 API/WS no-fallback 语义不变。
- Native AI HTTP client no-panic：新增 `native-ai-http-client-no-panic-2026-05-14.md`，将 Native AI Chat SSE HTTP client 单例初始化从 `expect("Failed to create HTTP client")` 改为 `Result` 传播，保留共享 client 行为，失败进入既有 AI Chat error path 而不是 panic server。
- Remote repo selector readable no-panic：新增 `remote-repo-selector-readable-no-panic-2026-05-14.md`，将 remote repo selector 解析里的 `expect("validated readable")` 改为局部 `let Some(info) else` fail-closed guard，保持 broken shadow repo 错误语义不变，并加 storage/repo baseline 防回归。
- Scope pref serialization no-panic：新增 `scope-pref-serialization-no-panic-2026-05-14.md`，将 Web repo scope preference 持久化里的 `expect("scope pref should serialize")` 改为显式 `serialize_scope_pref` fail-soft helper；正常 repo-name-only JSON 不变，异常序列化只跳过本次写入并记录 warning。
- Protocol switch nonce no-panic：新增 `protocol-switch-nonce-no-panic-2026-05-14.md`，将 Web protocol error scope-switch cleanup 的 `switch_nonce.expect("checked above")` 改为显式 `let Some(...) else` 早退，保持缺失 nonce 不清理 pending switch 的语义不变，并加 auth/protocol baseline 防回归。
- Ctrl key DOM helper no-panic：新增 `ctrl-key-dom-helper-no-panic-2026-05-14.md`，将 Ctrl/Cmd link activation hook 的 `window/document/body` 访问改为显式 `Option` helper，正常 DOM runtime 行为不变，并加 native 单测与 rendering baseline 防回归。
- Outline inline parser no-panic：新增 `outline-inline-parser-no-panic-2026-05-14.md`，为 outline inline parser 增加 UTF-8 边界安全的 `next_char_at` helper，移除 parse/scan 里的直接 next-char unwrap，保持合法输入渲染语义不变，并加 rendering baseline 防回归。
- Dropdown viewport height no-panic：新增 `dropdown-viewport-height-no-panic-2026-05-14.md`，将 dropdown placement 的 `window.expect("window")` 改为显式 `Option` fallback，正常浏览器行为不变，并加 native 单测与 UI baseline 防回归。
- Search result detail no-panic：新增 `search-result-detail-no-panic-2026-05-14.md`，将 Unified Search result detail 渲染从 `detail_text.clone().unwrap()` 改为单一 `Option<String>` 显式 view 构造，保持 UI 输出语义不变，并加 Search baseline 防回归。
- Git status retry hint no-panic：新增 `git-status-retry-hint-no-panic-2026-05-14.md`，将 `deve_cli git status` lagging-record retry hint 从 `expect` 维护的显示层不变量改为显式 retry command，保持输出合同不变，并加 Source Control baseline 防回归。
- Source Control remote scope stale branch no-panic：新增 `source-control-remote-scope-stale-branch-no-panic-2026-05-14.md`，将 remote stale-scope detail 构造中的 `active_branch.expect("checked active branch")` 改为显式分支绑定，保持结构化错误语义不变，并加 source-control baseline 防回归。
- Chat drop file fail-closed：新增 `chat-drop-file-fail-closed-2026-05-14.md`，将 AI Chat 文件拖拽中的 `FileReader` 创建失败和读取失败改为可见 banner，保留 1 MiB 限制，并用 AI baseline guard 防止回退到 panic/静默失败路径。
- Source Control proxy client fail-closed：新增 `source-control-proxy-client-fail-closed-2026-05-14.md`，将 plugin-host proxy 的 Source Control HTTP client 初始化从 `expect` 改为 `Result` 传播，并用 source-control baseline guard 防止回退到 panic 路径。
- Watch Ctrl-C handler fail-closed：新增 `watch-ctrlc-handler-fail-closed-2026-05-14.md`，将 `deve watch` 的 Ctrl+C handler 安装提前到 scan / watcher start 之前，并把 handler 注册失败从 panic 改为 `Result` 错误。
- Node check projection vault fail-closed：新增 `node-check-projection-vault-fail-closed-2026-05-14.md`，将 `deve node-check --projection` 的 `SyncManager` 构造切到 `new_checked`，补缺失 vault 不 panic 的测试，并把 dev-data-health baseline 绑定到 checked constructor。
- Code toolbar default action boundary：新增 `code-toolbar-default-action-boundary-2026-05-14.md`，移除 Web 初始化中的 `Run Code` / `Send to AI` 默认占位动作，只保留 `deve_code_actions` 扩展点；当前默认行为回到 `RENDER-CODE-001` 的空菜单状态，并用 rendering baseline 防回归。
- Watcher lifecycle duplicate start gate：新增 `watcher-lifecycle-duplicate-start-gate-2026-05-14.md`，为同一 repo 重复启动 watcher 增加启动前 registry gate，并在竞态 rejected handle 上显式 `stop + join`，避免 orphan watcher runtime。
- Mainline gap rescan after watcher lifecycle：新增 `mainline-gap-rescan-after-watcher-lifecycle-2026-05-14.md`，复跑 runtime happy/recovery smoke、plan coverage、release/dev-runbook/native guards 与 Web dependency audit；当前未发现 unblocked 主线代码缺口，下一步收敛为 Chrome MCP 隔离浏览器 smoke refresh。
- Chrome MCP isolated browser smoke refresh：新增 `chrome-mcp-isolated-browser-smoke-refresh-2026-05-14.md`，用隔离数据根验证登录、Ready、新建、编辑写回、reload 读回、强制断线与同数据根重连恢复；最终稳定态 console/network 健康，未发现代码缺陷。
- Peer registration retry status：新增 `peer-registration-retry-status-2026-05-14.md`，将 `Connected` 但 repo peer/writer 尚未注册的状态从普通 `Handshaking repo` 中拆出，显示 `Logged in / Peer not registered`，并在 desktop/mobile 状态栏提供 retry peer 入口，驱动 handshake retry nonce 重新注册。
- Web foreground reprobe write gate：新增 `web-foreground-reprobe-write-gate-2026-05-14.md`，浏览器从后台/失焦恢复到前台时清空 stale writer-ready、handshake scope 与 node-role readiness，触发新 repo handshake 与 `/api/node/role` reprobe，避免旧写入授权跨 foreground recovery 继续生效。
- Source Control HTTP scope gate：新增 `source-control-http-scope-gate-2026-05-14.md`，为 `/api/sc/*` read/mutation surface 增加显式 `scope_nonce` gate，补齐 proxy 默认 nonce、Web Git repair readonly review scope 传递与 baseline guard。
- Acceptance stale command cleanup：新增 `acceptance-stale-command-cleanup-2026-05-14.md`，清理 POS/DIFF/AUTH/REPO/STORE acceptance 中的过期伪命令与 stale test filter，并把 Web release smoke 归属修正为 `CMD-007A`。
- Acceptance / release guard cleanup：新增 `acceptance-release-guard-cleanup-2026-05-14.md`，修复 letter-suffixed acceptance ID 解析、补齐 CLI baseline command surface，并将 `REL-001` 从过时 `dist/v1.0.0` 验收改为当前 GHCR/Docker release workflow surface。
- Mobile PendingAck scope filter：新增 `mobile-pending-ack-scope-filter-2026-05-14.md`，移动端 footer pending 状态复用 repo/scope 过滤，避免旧 scope pending 污染当前状态。
- WS text-frame debug gate：新增 `ws-text-frame-debug-gate-2026-05-14.md`，将 versioned JSON text 与 legacy JSON text 一并收敛到显式 debug flag，生产默认只接受 versioned binary。
- Auth hardening batch：新增 `auth-hardening-batch-2026-05-14.md`，修复 `AUTH_USER` token subject 硬编码与 `AUTH_PASS` PHC 启动校验，并扩展 auth baseline guard。
- Mainline gap rescan after native gate design：新增 `mainline-gap-rescan-after-native-gate-design-2026-05-14.md`，确认当前 blocking guard 仍为绿，并将下一批收敛到 auth hardening、WS text debug gate、mobile pending scope filter 与 acceptance/release guard cleanup。
- Native packaging gate design：新增 `native-packaging-gate-design-2026-05-14.md`，确认 Desktop/Mobile 仍保持 no-Tauri skeleton；真实 Tauri packaging 必须先经 Desktop dependency spike，再做 Desktop shell acceptance、Mobile dependency spike 与独立 process adapter gate。
- Error-code catalog drift review：新增 `error-code-catalog-drift-review-2026-05-14.md`，确认 `ServerErrorCode` 与 Web i18n 映射内部一致；plan catalog 仍缺 `SC_COMMIT_DIFF_UNPROJECTABLE` 与 `GRAPH_DEGRADED_PROJECTION_REQUIRED`，需明确允许修改 `docs/plan/` 后再补。
- Release dependency maintenance triage：新增 `release-dependency-maintenance-triage-2026-05-14.md`，将 Mermaid 从 `11.13.0` 升到 `11.15.0`，清除 `npm audit` 报告的 4 个 moderate advisory；Graph renderer gate 仍保持关闭。
- Docker release smoke freshness：新增 `docker-release-smoke-freshness-2026-05-14.md`，用 Docker Desktop CLI 复验当前 Dockerfile 生产容器，修复 WSL bind mount 导致非 root `appuser` 无法写 `/data/ledger` 的 smoke harness 问题；Docker smoke 现在使用临时 Docker named volume，并验证 `/api/node/role` 与生产登录。
- Server error code copy closure：新增 `server-error-code-copy-2026-05-14.md`，将 Web 搜索 banner、protocol banner、Chat PluginResponse 错误补全文案与 Source Control server notice 从 backend `detail` 展示切换为 `ServerErrorCode -> t::server_error::message`；`detail` 保留为日志/调试上下文。
- Mainline gap rescan 2026-05-14：新增 `mainline-gap-rescan-2026-05-14.md`，确认 plan coverage / native boundary / release guards 健康；将 `docker-compose.yml` 收敛为生产 release compose，新增 `docker-compose.dev.yml` 保留本地 Dockerfile build，并更新 release/dev-runbook guards。
- Native pre-gate freshness report：新增 `native-pre-gate-freshness-2026-05-13.md`，复跑 Desktop/Mobile no-packaging shell、supervisor、recovery、native packaging gate 与 core native_adapter tests；未打开 Tauri/native-packaging gate。
- Release dependency audit gate：新增 `release-dependency-audit-gate-2026-05-13.md` 与 `scripts/check-release-audit-gate.sh`，将 `REL-003` 依赖审计收敛为本地 diagnostic / CI required 双路径；release workflow 安装 Node.js 20 与 `cargo-audit` 后 required 模式执行。
- I18N visible text batch 2：新增 `i18n-visible-text-batch-2-2026-05-13.md`，将 ActivityBar more action、Sidebar item action 与 Disconnect overlay status line 收敛到 `t::*`，并扩展 `check-i18n-hardcoded-baseline.sh` 覆盖本批范围。
- Mainline gap rescan after final regression：新增 `mainline-gap-rescan-after-final-regression-2026-05-13.md`，确认 guard 输入健康，选择 `11_i18n` 可见文案作为下一批 Current MUST 缺口；已完成 Command/Search 小批次，新增 `check-i18n-hardcoded-baseline.sh` 并绑定 I18N-001。
- Final regression gate：新增 `final-regression-gate-2026-05-13.md`，跑完整 baseline scripts、runtime happy/recovery smoke、全仓库 `cargo test` 与 `cargo clippy --all-targets -- -D warnings`；修复 WS acceptance 旧错误码断言、AI Chat 插件测试 env 竞态，以及 clippy 暴露的 Web incoming / diff metrics 小问题。
- Protocol error / version alignment capture：新增 `protocol-error-version-alignment-capture-2026-05-13.md`，将 unsupported WS protocol version 统一为 `SYNC_VERSION_MISMATCH`、malformed versioned payload 统一为 `SYNC_INVALID_PAYLOAD`，补齐服务端 versioned JSON / malformed binary 与 Web malformed server frame 测试，并加强 network / structured-error guard。
- Plugin runtime security boundary refresh：新增 `plugin-runtime-security-boundary-refresh-2026-05-13.md`，确认 manifest entry、host FS、ledger-managed write、Rhai eval/env、trusted-cli policy 与 Web fallback 均 fail-closed；修复 Rhai import 使用裸 `FileModuleResolver` 的边界缺口，新增 `GuardedFileModuleResolver` 阻断 parent traversal 与 symlink escape。
- Release / production runtime verification refresh：新增 `release-production-runtime-verification-2026-05-13.md`，验证 embedded/static frontend、production auth fail-closed 与配置成功、`/api/node/role`、Chrome MCP production frontend、runtime happy/recovery 与 Docker smoke；同时加强 Docker smoke，使其验证生产登录。
- Mainline gap rescan after AI slash closure：新增 `mainline-gap-rescan-after-ai-slash-closure-2026-05-13.md`，确认上一轮 active queue 已闭合、核心 guard 无 blocking drift，并将下一批收敛到 release/production runtime、plugin runtime security、protocol error capture 与最终回归；同步修复 `docs/dev-runbook.md` 漏列 `check-storage-repo-baseline.sh`。
- Optional AI slash command smoke：新增 `ai-slash-command-browser-smoke-2026-05-13.md`，用隔离数据根验证 `/plan` 与 `/agents` 只切换 Native PLAN/BUILD session mode、不切 backend、不发起 provider/plugin call，并确认浏览器 console/network 健康。
- Plan-code mapping soft cleanup：新增 `plan-code-mapping-soft-cleanup-2026-05-13.md`，修正测试目录/`*_tests.rs` 的 `plan_ref` soft warning 豁免，并给剩余 Git bridge / protocol / commit diff 生产模块补准确 `plan_ref`；非豁免缺失注解从 56 降到 0，未触碰 `docs/plan/`，未做机械拆分。
- Source Control `CommitAndPush` browser smoke：新增 `source-control-commit-and-push-browser-smoke-2026-05-13.md`，用隔离数据根验证 watcher pending、stage、split action `提交并推送`、`CommitAck` 完成、reload clean state 与无 Web Git mirror push authority；未发现业务代码缺陷。
- Mobile residual interaction spot smoke：新增 `mobile-residual-interaction-smoke-2026-05-13.md`，用 Chrome MCP 真实移动视口验证 keyboard toolbar gate、Search Top Sheet 滚动隔离、AI Chat 可读性/结构化错误/retry 与 mobile diff 打开/关闭；修复 `ai-chat` 缺 API key 误作为 text success 返回、CLI plugin error result 结构化转换缺失、ChatPanel 错误 effect 对 pending req 过度依赖的问题。
- `.deveignore` watcher/scan user-facing smoke：新增 `deveignore-watcher-scan-browser-smoke-2026-05-13.md`，用隔离数据根验证 repo-relative 与 vault-relative ignore 规则覆盖 startup scan、watcher 增量事件、Source Control pending/status、repo docs、graph projection、export/dump 与 reload recovery；ignored Markdown 未进入 pending、tree projection、ledger 或 UI 可见变更。
- Desktop Web shell browser smoke：新增 `desktop-web-shell-browser-smoke-2026-05-13.md`，用 Chrome MCP 验证 desktop breakpoint、五列布局 marker、sidebar/right panel resize 持久化、Source Control diff 双栏滚动同步、Unified Search command/branch/file routing 与当前导航 console/network 健康；未发现代码缺陷。
- Auth security acceptance refresh：新增 `auth-security-acceptance-refresh-2026-05-13.md`，复核 `AUTH-003..010/012` 的 cookie、CORS、CSRF baseline、rate-limit、JWT、WS unauthorized 与 public status endpoint；未发现运行缺陷，并修正 `AUTH-007` 中旧 `/api/write` 示例为当前受保护写入口 `/api/sc/commit`。
- Mainline gap rescan after smoke queue closure：新增 `mainline-gap-rescan-after-smoke-closure-2026-05-13.md`，确认上一轮 G1-G7 smoke 队列已闭合，native/desktop/mobile 默认 no-packaging skeleton 不是当前阻塞项，并将下一批收敛到 Auth 安全验收复核、Desktop 宽屏 browser smoke、`.deveignore` 用户面 smoke、Mobile 残余交互 spot smoke、Source Control `CommitAndPush`、plan-code 软映射清理与 optional AI slash command smoke。
- Graph read-only projection panel browser smoke：新增 `graph-readonly-projection-panel-smoke-2026-05-13.md`，用 Chrome MCP 验证 empty / loaded / blocked / degraded Graph states、readonly projection summary、renderer gate closed、HTTP graph projection JSON 与稳定态 console/network 健康，并补齐 Graph panel 可测性 marker。
- Dashboard SystemMetrics browser smoke：新增 `dashboard-system-metrics-browser-smoke-2026-05-13.md`，用 Chrome MCP 验证 Dashboard live metrics、WS sample refresh、断线冻结、重连恢复、RAM-only 边界、稳定态 console/network 健康，并补齐 Storage/Quick Actions DOM marker。
- Git mirror CLI-only notice / readonly repair review smoke：新增 `git-mirror-cli-notice-readonly-repair-smoke-2026-05-13.md`，用 Chrome MCP 验证 Git Import/Push/Repair Command Palette notice、readonly repair review、copyable retry command、无 Web Git writer，并修复 CommandId 搜索与 Git notice callback 上下文捕获问题。
- Command Surface routing smoke：新增 `command-surface-routing-smoke-2026-05-13.md`，用 Chrome MCP 验证 `Ctrl+Shift+P` command mode、`Ctrl+P` Quick Open、`Ctrl+Shift+K` branch mode、过滤、键盘导航、执行/取消、console/network 健康，并修正非 plan 文档中的旧 `Ctrl/Cmd+K` 表述。
- Mobile Web shell narrow-viewport smoke：新增 `mobile-web-shell-narrow-viewport-smoke-2026-05-13.md`，用 Chrome MCP 真实 375x812 mobile viewport emulation 验证 mobile layout marker、左右 drawer、Search top sheet、bottom bar 折叠/展开、AI Chat 全屏页、44px touch target 与 console/network 健康。
- Browser storage / projection degraded write-gate smoke：新增 `browser-storage-projection-degraded-write-gate-smoke-2026-05-13.md`，用 Chrome init script 模拟 IndexedDB/WebCrypto 缺失，验证 degraded 横幅、只读 UI、Source Control 写阻断、projection degraded 服务端写闸与 runtime recovery smoke，并修复 Source Control `ReadOnly` hint 误报 remote branch 的文案问题。
- Repo / remote spectator read-only UI smoke：新增 `remote-spectator-readonly-ui-smoke-2026-05-13.md`，用隔离 local + `peer-a` shadow branch 验证 remote spectator 只读提示、编辑器只读、Explorer/Quick Open 创建阻断、Source Control 写入口阻断与返回本地恢复可写。
- Network / repo scope browser recovery smoke：新增 `network-repo-scope-browser-recovery-smoke-2026-05-13.md`，用隔离双 repo 数据根与 Chrome MCP 验证 WS 断线锁屏、重连恢复、repo switch 重新握手、scope 隔离与 write gate。
- Mainline gap rescan：新增 `mainline-gap-rescan-2026-05-13.md`，重新按 `docs/plan × features × acceptance-cases × code` 选择下一批 browser smoke，并修复 network/dashboard guard 对 lifecycle-aware `try_set` 的旧匹配。
- Settings / Extensions reserved UI browser smoke：新增 `settings-extensions-reserved-ui-browser-smoke-2026-05-13.md`，用隔离数据根与 Chrome MCP 验证 Trusted CLI default-off、Settings reserved marker、Calculation Runtime planned/disabled 与 `/api/settings` absent，并补齐 `aria-disabled` / reserved marker。
- Rendering interaction spot smoke：新增 `rendering-interaction-spot-smoke-2026-05-13.md`，用隔离数据根与 Chrome MCP 验证 code toolbar、Ctrl/Cmd link activation、Outline navigation、Mermaid projection/source reveal、nested rendering 与 source authority。
- Merge conflict UI browser smoke：新增 `merge-conflict-ui-browser-smoke-2026-05-13.md`，用 hidden conflict fixture 与 Chrome MCP 验证 `accept-current` / `accept-incoming` / `accept-both`，并修复 peer branch merge gate、conflict message scope、legacy `DocDiff` fallback 覆盖与 accept-both 默认内容。
- Mainline implementation gap scan：新增 `mainline-gap-scan-2026-05-12.md`，确认 plan coverage 无 blocking、architecture registry 0 drift、runtime happy/recovery smoke 通过、search feature-on/off 路径通过。
- Browser runtime/search smoke：新增 `browser-runtime-search-smoke-2026-05-12.md`，用隔离数据根验证登录、Ready、新建、编辑、刷新重连、Search `?note`，并修复 search result 选择后弹窗反向重开的 UI bug。
- Feature acceptance gap scan：新增 `feature-acceptance-gap-scan-2026-05-12.md`，确认 Commands / Settings baseline 闭合，并修正 Rendering large-doc search gate 的 feature 路径与 acceptance 自动化绑定。
- Rendering browser spot smoke：新增 `rendering-browser-spot-smoke-2026-05-12.md`，用 Chrome MCP 点验 checkbox source writeback、KaTeX projection 与 Ready 后全文搜索打开文档路径。
- Feature operation path drift scan：新增 `feature-operation-path-drift-scan-2026-05-12.md`，收敛 operation 文档中的重构前路径，并新增 `scripts/check-feature-operation-paths.sh` 防回归。
- Source Control browser spot smoke：新增 `source-control-browser-spot-smoke-2026-05-12.md`，用 Chrome MCP 点验 watcher pending、stage/unstage、commit message enablement、commit ack、refresh 与 reload recovery。
- Feature acceptance gap scan 02：新增 `feature-acceptance-gap-scan-2026-05-12-02.md`，确认 feature/acceptance 绑定闭合，并修复 dev runbook 对新增路径 guard 的遗漏。
- Release delivery smoke：新增 `release-delivery-smoke-2026-05-12.md`，确认 Web release build、embedded frontend runtime release info 与 Docker production-auth smoke 均通过。
- Feature acceptance gap scan 03：新增 `feature-acceptance-gap-scan-2026-05-12-03.md`，修复 I18N-005 chat timestamp 手写格式缺口，并新增 i18n formatting guard。
- I18N localized formatting browser smoke：新增 `i18n-localized-formatting-browser-smoke-2026-05-12.md`，用 Chrome MCP 验证 chat timestamp 与 Source Control history relative time 的 locale 切换重渲染。
- Feature acceptance gap scan 04：新增 `feature-acceptance-gap-scan-2026-05-12-04.md`，确认下一批应先收敛 UI-DIFF 验收闭环，再处理 Storage/Repo 过时 CLI 验收漂移与 WebWrite pending browser smoke。
- UI Diff acceptance closure：新增 `ui-diff-acceptance-closure-2026-05-12.md`，修正 `UI-DIFF-*` manual binding 语义漂移，并为已有 diff behavior 增加最小自动 guard。
- Storage / Repo acceptance drift：新增 `storage-repo-acceptance-drift-2026-05-12.md`，移除 `07_storage_repo.md` 中过时伪 CLI 步骤，新增 `scripts/check-storage-repo-baseline.sh`，并补齐 init/recover/export 轻量测试证据。
- WebWrite pending navigation smoke：新增 `webwrite-pending-navigation-smoke-2026-05-12.md`，用隔离后端和 Chrome MCP 验证 pending modal、取消保留、确认离开与 Reject 后 pending overlay 闭合。
- Feature acceptance gap scan 05：新增 `feature-acceptance-gap-scan-2026-05-12-05.md`，确认当前无 P0 / blocking drift，并将下一批收敛到 UI Diff、Search disabled、Auth session 与 Native AI provider preflight。
- UI Diff browser interaction smoke：新增 `ui-diff-browser-interaction-smoke-2026-05-12.md`，用隔离后端和 Chrome MCP 验证 Source Control diff 打开、hunk 导航、键盘导航、fold 展开、context 切换、cache/header 徽标与 mobile runtime edit debounce。
- Search disabled fail-closed smoke：新增 `search-disabled-fail-closed-smoke-2026-05-12.md`，用隔离后端和 Chrome MCP 验证未编译 search feature 时 `?needle` 显示用户可见 unavailable 反馈、无 stale result、状态保持就绪。
- Auth session expired browser smoke：新增 `auth-session-expired-browser-smoke-2026-05-12.md`，用隔离后端和 Chrome MCP 验证外部 session 失效与 UI logout 均进入登录页、无 Reconnecting overlay、无 console error，并修复 MainLayout 卸载后旧 WS 任务写 disposed signal 的生命周期 bug。
- Native AI positive smoke：新增 `native-ai-positive-smoke-2026-05-12.md`，用本地 OpenAI-compatible SSE mock 验证 Native AI Chat 正向 browser 链路，并修复 `ai-chat` Rhai prompt 常量在正向分支中的作用域 bug。
- Mainline gap selection after smoke closure：新增 `mainline-gap-selection-after-smoke-2026-05-12.md`，确认下一批按 AI BUILD Apply、Merge Conflict UI、Rendering 交互、Settings/Extensions reserved UI 依次执行。
- AI BUILD Apply browser smoke：新增 `ai-build-apply-browser-smoke-2026-05-12.md`，用本地 OpenAI-compatible SSE mock 验证 `/build` 不发 plugin call、assistant code block 显示 Apply、点击 Apply 后当前 Markdown 经本地 projection 与 `ClientMessage::Edit` 改变，并修复程序化 Apply 未更新 CodeMirror 本地视图的缺陷。
- Runtime happy-path smoke：新增 `scripts/smoke-runtime-happy-path.sh`，用临时 repo 覆盖 repo switch、SyncHello、RegisterWriter、CreateDoc、Edit、OpenDoc、History 与 reconnect bootstrap 单测。
- Near-fuse cohesion triage：已按职责拆分 i18n common/source-control/git copy；保留 `ClientMessage` 协议枚举与 `apps/cli/src/server/ws/route/merge/tests.rs` 场景测试上下文，不做纯行数拆分。

## 当前基线

- Git ecosystem bridge：`git-ecosystem-bridge-baseline-2026-05-01.md`
- Native shell：`native-shell-baseline-2026-05-01.md`
- Mobile UI：`mobile-ui-baseline-2026-05-01.md`
- Graph：`graph-baseline-2026-05-01.md`
- Settings / AI：`settings-ai-baseline-2026-05-01.md`
- Source Control runtime：`source-control-runtime-baseline-2026-05-01.md`
- Release verification：`release-verification-baseline-2026-05-01.md`
- Core hardening：`core-hardening-baseline-2026-05-01.md`

## MCP 方向

产品 MCP runtime 已退役。当前扩展方向是 Skills 加显式 trusted controlled CLI path。docs 中的 MCP 只允许表示退役说明，或表示 Chrome MCP 浏览器手工验收工具。

除非重新打开 plan，不要新增 MCP runtime、MCP server management、MCP tool loop 或 MCP-backed Native AI capability。
