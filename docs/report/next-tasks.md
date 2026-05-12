# 当前下一步任务

> 更新日期：2026-05-12
>
> 本文件只记录 active execution queue。完成历史进入 `docs/report/*-baseline-YYYY-MM-DD.md`。

## 当前执行队列

| 顺序 | TODO | 优先级 | 范围 | 验收口径 |
|:--|:--|:--|:--|:--|
| 1 | WebWrite pending navigation browser smoke | P1 | Web write pending overlay + Chrome MCP | pending modal、Stay、确认离开、Reject 后不永久 pending 在隔离后端中可见闭合 |

## 最近完成

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
