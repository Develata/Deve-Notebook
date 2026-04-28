# 当前下一步任务

> 更新日期：2026-04-28
>
> 本文件只记录 active execution queue。已完成的实现历史应进入 dated reports，例如
> `code-review-2026-04-28.md` 与 `release-smoke-status-2026-04-28.md`。

## 当前执行队列

| 顺序 | TODO | 优先级 | 范围 | 验收口径 |
|:--|:--|:--|:--|:--|
| 1 | Sync vector wire contract | P0 | `docs/plan/05_network.md`, `crates/core/src/sync/protocol.rs`, `crates/core/src/protocol/{client,server}.rs`, WS sync handlers | 实现显式 `known_vector/server_vector`，或修订 plan 接受 `SyncHello.vector + range requests` 作为当前合同。 |
| 2 | Browser storage / degraded write boundary | P0 | `apps/web/src/storage/`, `apps/web/src/hooks/use_core/`, `docs/plan/04_storage.md`, `docs/plan/16_web_thin_client_ledger.md` | WebCrypto/IndexedDB capability、私钥语义、degraded read-only、SyncPush/write blocking 可测试且有文档边界。 |
| 3 | Security hardening small batch | P1 | `apps/cli/src/server/auth/`, `apps/cli/src/server/security.rs`, `apps/cli/src/server/setup.rs`, `docs/plan/09_auth.md` | key-file permissions、login audit fields、production CORS origin、dev CORS warning text 与 plan 对齐。 |
| 4 | Path normalization cleanup | P1 | `crates/core/src/plugin/manifest.rs`, server/web path wrappers, `deve_core::utils::path` | 手写 slash replacement 被移除或在边界 wrapper 中明确豁免；不改变已存储路径语义。 |
| 5 | Git ecosystem mirror bridge plan-to-code | P1/P2 | `docs/plan/04_storage.md`, `docs/plan/07_diff_logic.md`, `docs/plan/12_commands.md`, future `crates/core/src/git_bridge/`, future CLI git commands | `.notegit/` 与 `.git/` 共存；`.gitignore` 忽略 `.notegit/`；watcher 忽略 `.git/`；Deve commit 可镜像为 Git commit；失败进入 `GitMirrorOutOfSync`；外部 Git 变化只能 import。 |
| 6 | Rendering current/future split | P1/P2 | `docs/plan/03_rendering.md`, `docs/features/03_rendering.md`, `apps/web/src/editor/`, `apps/web/src/utils/markdown.rs` | 当前可验收 rendering 行为与 future hybrid-rendering 分离；partial feature 不再被描述成 complete。 |
| 7 | Desktop / Mobile native adapter plan | P3-10 | `docs/plan/08_ui_design_02_desktop.md`, `docs/plan/08_ui_design_03_mobile.md`, future native shell | 实现前先把 minimal adapter 职责定清楚：embedded service、readiness/offline events、endpoint/session injection。 |
| 8 | Graph visualization next step | P3-13 | `crates/core/src/graph/`, future graph UI | read-only projection 不反向污染 ledger/search/source-control；visualization 只消费 projection。 |
| 9 | Docker release smoke rerun | P2 | Docker host environment, `scripts/smoke-docker-release.sh` | Docker daemon 可用后重跑；环境阻塞继续与代码失败分开记录。 |

## 最近完成基线

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
