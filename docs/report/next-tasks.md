# 当前下一步任务

> 更新日期：2026-04-29
>
> 本文件只记录 active execution queue。已完成的实现历史应进入 dated reports，例如
> `code-review-2026-04-28.md` 与 `release-smoke-status-2026-04-28.md`。

## 当前执行队列

| 顺序 | TODO | 优先级 | 范围 | 验收口径 |
|:--|:--|:--|:--|:--|
| 1 | Git mirror commit mapping and OutOfSync persistence | P1/P2 | `crates/core/src/git_bridge/`, Source Control commit path, future CLI git commands | 在已完成共存/忽略/status foundation 之上，落地 `DeveCommit -> GitMirrorQueued -> GitMirrorCommitted/GitMirrorOutOfSync` 的持久化映射、retry/repair/status 可观测路径；真实 `git add/commit` 失败不得回滚 Deve ledger commit。 |
| 2 | Rendering current/future split | P1/P2 | `docs/plan/03_rendering.md`, `docs/features/03_rendering.md`, `apps/web/src/editor/`, `apps/web/src/utils/markdown.rs` | 当前可验收 rendering 行为与 future hybrid-rendering 分离；partial feature 不再被描述成 complete。 |
| 3 | Desktop / Mobile native adapter plan | P3-10 | `docs/plan/08_ui_design_02_desktop.md`, `docs/plan/08_ui_design_03_mobile.md`, future native shell | 实现前先把 minimal adapter 职责定清楚：embedded service、readiness/offline events、endpoint/session injection。 |
| 4 | Graph visualization next step | P3-13 | `crates/core/src/graph/`, future graph UI | read-only projection 不反向污染 ledger/search/source-control；visualization 只消费 projection。 |
| 5 | Docker release smoke rerun | P2 | Docker host environment, `scripts/smoke-docker-release.sh` | Docker daemon 可用后重跑；环境阻塞继续与代码失败分开记录。 |

## 最近完成基线

- P0 sync vector wire contract 与 browser storage/degraded write boundary 已关闭：`DEVEWSF3`、显式 `known_vector/server_vector`、Web degraded read-only/write gate 均已测试。
- P1 security hardening small batch 已关闭：`identity.key` owner-only、login audit `timestamp/user_agent`、CORS wildcard fail-closed、dev-only auth/CORS warnings 均已测试。
- P1 path normalization cleanup 已关闭：runtime forward-slash normalization 已集中到 `deve_core::utils::path`，剩余 `replace('\\', "\\\\")` 是测试脚本文字串转义而非路径归一化。
- P1/P2 Git mirror bridge foundation 已关闭：`.git/.notegit` internal path segment 过滤、repo-local `.gitignore` 保护 `.notegit/`、只读 `deve_cli git status` 骨架与定向测试已落地；真实 mirror commit/import/export/push 仍在 active queue。
- P0 repo health、`repair --check`、WS structured errors、writer-ready `repo_id + scope_nonce`、Source Control doc identity hardening 已记录在 `code-review-2026-04-28.md`。
- P1 search、settings current boundary、Native AI Chat minimum、graph projection、i18n cleanup、plan_ref sweeps 已记录在 `code-review-2026-04-28.md`。
- Release/runtime smoke 与 Docker daemon blocker 已记录在 `release-smoke-status-2026-04-28.md`。
- File cohesion 与 line-count policy 已记录在 `soft-size-audit-2026-04-27.md`。

## MCP 方向

产品 MCP runtime 已退役。当前扩展方向是 Skills 加显式 trusted controlled CLI path。docs 中的 MCP 只允许表示退役说明，或表示 Chrome MCP 浏览器手工验收工具。

除非重新打开 plan，不要新增 MCP runtime、MCP server management、MCP tool loop 或 MCP-backed Native AI capability。

## 旧分支概览

2026-02-28 的 Branch A-E 拆解已退役。不要恢复旧 checkbox 作为 active TODO。

历史映射：

- 旧 A UI token/component 工作并入 P2 UI/design debt。
- 旧 B dashboard 工作由 runtime observability 与 `/api/node/role` 替代。
- 旧 C E2EE/WebCrypto/IndexedDB 工作并入 browser storage boundary。
- 旧 D plugin/AI 收缩为 Rhai plugin host 与 Native AI Chat minimum；MCP 退役。
- 旧 E docs sync 由 dated baselines 与当前短队列替代。
