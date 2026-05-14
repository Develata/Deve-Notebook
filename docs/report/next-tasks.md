# 当前下一步任务

> 更新日期：2026-05-14
>
> 本文件只记录 active execution queue。完成历史进入 `docs/report/*-baseline-YYYY-MM-DD.md`。

## 当前执行队列

1. Error-code catalog plan patch：只在明确允许修改 `docs/plan/` 时，将 `SC_COMMIT_DIFF_UNPROJECTABLE` 与 `GRAPH_DEGRADED_PROJECTION_REQUIRED` 补入 `11_i18n.md`；否则继续保持 plan 不动。
2. Native packaging gate opening decision：只有明确批准打开 `native-packaging` dependency gate 时，才执行 `NPG-1 Desktop Packaging Dependency Spike`；否则继续保持 no-Tauri skeleton，并回到 mainline implementation gap scan。
3. Mainline implementation gap scan：若不打开 plan patch 或 native packaging gate，则重新按 `docs/plan × features × acceptance-cases × code` 选择下一批用户可验收缺口。

## 最近完成

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
