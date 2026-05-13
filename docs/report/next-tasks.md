# 当前下一步任务

> 更新日期：2026-05-13
>
> 本文件只记录 active execution queue。完成历史进入 `docs/report/*-baseline-YYYY-MM-DD.md`。

## 当前执行队列

| 顺序 | TODO | 优先级 | 范围 | 验收口径 |
|:--|:--|:--|:--|:--|
| 1 | Network / repo scope browser recovery smoke | P1 | WS reconnect, repo switch, stale scope isolation, write gate | 隔离数据根 + Chrome MCP；验证后端不可达/恢复、多 repo 切换、旧 scope 消息不驱动新 scope、断连/重连写闸 |
| 2 | Repo / remote spectator read-only UI smoke | P1 | remote/spectator scope, readonly controls, Source Control write block | 验证 remote/spectator 可见只读、create/edit/stage/commit 不可写、返回 local scope 后恢复 |
| 3 | Browser storage / projection degraded write-gate smoke | P1 | IndexedDB/WebCrypto fallback, degraded projection, mutation block | 验证 degraded 状态用户可见，`RegisterWriter`、edit、Source Control mutation 被阻断 |
| 4 | Mobile Web shell narrow-viewport smoke | P2 | 375x812 viewport, drawers, search sheet, bottom bar, chat | 验证移动壳层核心入口与手势，console/network 无异常 |
| 5 | Command Palette / Quick Open / Branch Switcher routing smoke | P2 | Ctrl+P, Ctrl+Shift+P, Ctrl+Shift+K, action routing | 验证快捷键、搜索、键盘导航、执行/取消和 branch switcher UI |
| 6 | Git mirror CLI-only notice / readonly repair review smoke | P2 | Git Import/Push/Repair notices, repair review readonly states | 验证 Web 不执行 Git writer，只显示 CLI-only notice / readonly review |
| 7 | Dashboard SystemMetrics browser smoke | P2 | live metrics, stale epoch, disconnected freeze | 验证 Dashboard 指标刷新、断线冻结/恢复、RAM-only 状态 |
| 8 | Graph read-only projection panel browser smoke | P2 | graph summary, empty/degraded/blocked states, renderer gate | 验证 readonly projection summary 与 renderer future-only 边界 |

## 最近完成

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
