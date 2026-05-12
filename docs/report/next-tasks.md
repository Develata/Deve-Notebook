# 当前下一步任务

> 更新日期：2026-05-12
>
> 本文件只记录 active execution queue。完成历史进入 `docs/report/*-baseline-YYYY-MM-DD.md`。

## 当前执行队列

| 顺序 | TODO | 优先级 | 范围 | 验收口径 |
|:--|:--|:--|:--|:--|
| 1 | Fix search operation mapping drift | P2 | docs/features/operations/search_query.md | `op.search.submit` 指向当前代码路径 |
| 2 | Browser runtime E2E smoke | P1 | isolated serve --dev + Chrome MCP | create/open/edit/save/reload/reconnect 不出现 stale scope、protocol version mismatch 或 disconnected lockout |
| 3 | Browser search smoke | P2 | search feature + Chrome MCP | `?note` 返回 repo-scoped SearchResults，结果可打开文档，状态保持 Ready |
| 4 | Next feature/acceptance gap scan | P2 | plan/features/acceptance/current code | 仅在 browser smoke 通过后选择下一批用户可感知实现项 |

## 最近完成

- Mainline implementation gap scan：新增 `mainline-gap-scan-2026-05-12.md`，确认 plan coverage 无 blocking、architecture registry 0 drift、runtime happy/recovery smoke 通过、search feature-on/off 路径通过。
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
