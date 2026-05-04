# 当前下一步任务

> 更新日期：2026-05-05
>
> 本文件只记录 active execution queue。完成历史进入 `docs/report/*-baseline-YYYY-MM-DD.md`。

## 当前执行队列

| 顺序 | TODO | 优先级 | 范围 | 验收口径 |
|:--|:--|:--|:--|:--|
| 1 | Native runtime batch 3: Web/native readiness loop | P1 | `apps/web/src/`, `apps/desktop/`, `apps/mobile/` | endpoint/session 注入、foreground reprobe、scope nonce、writer-ready 与 Source Control write gate 在 native loopback 下保持 fail-closed |
| 2 | Native packaging gate scaffold | P2 | `apps/desktop/`, `apps/mobile/`, `scripts/` | 只补 gated scaffold 与边界检查；真实 `tauri` / `tauri-build` 仍不得进入默认 workspace build |

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
