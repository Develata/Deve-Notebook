# Graph Renderer Gate Decision - 2026-04-29

## Decision

P3-13 Graph 当前停靠在 read-only projection data surface，不打开真实 Web renderer gate。

当前验收能力：

- `deve_core::graph`：从 repo docs 派生 authority-free graph projection。
- `deve graph`：只读 CLI JSON surface。
- `GET /api/repo/graph`：受保护 HTTP 只读 query，与 CLI 共享 adapter。
- Web Source Control Graph panel：只读 summary counts 与 loading / failed / empty / local-only fallback。

明确不进入当前批次：

- Canvas layout。
- d3-force / Pixi renderer。
- force simulation worker。
- graph interaction state。
- 任何 Graph renderer 写入 authority path。

## Rationale

真实 renderer 会引入新的 dependency、layout 性能预算、内存预算、交互状态与大图降级策略。当前 plan 的核心缺口已经通过 projection data、HTTP query 和 summary panel 关闭；继续推进 renderer 会把 P3-13 从数据面实现扩大成前端图形系统实现，不符合当前优先级。

本项目仍以 768 MB VPS 和低配可运行作为硬约束。Graph renderer 若进入后续批次，必须作为独立 dependency/performance gate 处理，而不是从已有 summary panel 顺手扩权。

## Boundary

- Web summary panel 只消费 `/api/repo/graph` projection，不写 ledger、workspace、search index、source-control state 或 `.git/.notegit`。
- `apps/web/package.json` 当前不声明 Graph renderer dependency。
- `apps/web/package-lock.json` 中由 Mermaid 等包带来的历史/间接 d3 依赖，不代表 Graph renderer gate 已打开。
- 任何未来 renderer 都必须 fail-closed 于 stale repo/scope，并在 low-spec profile 下提供禁用或轻量 fallback。

## Verification

- `docs/plan/14_tech_stack.md` 已标记 Graph renderer gate closed/deferred。
- `docs/features/07_diff_logic.md` 已同步 feature-level UI 边界。
- `docs/acceptance-cases/12_tech_release.md` 已补充 gate/dependency assertions。
- `docs/report/next-tasks.md` 已把下一 active queue 移回 P1 search/settings boundary audit。
