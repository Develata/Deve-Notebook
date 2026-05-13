# 当前下一步任务

> 更新日期：2026-05-13
>
> 本文件只记录 active execution queue。完成历史进入 `docs/report/*-baseline-YYYY-MM-DD.md`。

## 当前执行队列

| 顺序 | TODO | 优先级 | 范围 | 验收口径 |
|:--|:--|:--|:--|:--|
| 1 | Graph read-only projection panel browser smoke | P2 | graph summary, empty/degraded/blocked states, renderer gate | 验证 readonly projection summary 与 renderer future-only 边界 |

## 最近完成

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
