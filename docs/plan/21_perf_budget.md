# 21_perf_budget.md - Performance Budget (性能预算)

## Metadata

- `Layer`: `Governance Contracts (non-layer ownership-axis slice)`
- `Status`: `Current MUST`
- `Version`: `0.0.1`
- `Last Review`: `2026-05-28`
- `Authority Owns`: `op 维度 latency / RSS budget；CI fuse 阈值`
- `Authority Defers To`: `17_tech_stack#performance-profiles-and-feature-matrix (profile 枚举与 feature matrix)`
- `Counterpart Feature`: `docs/features/operation-coverage.md (perf-sensitive flows)`
- `Counterpart Acceptance`: `docs/acceptance-cases/00_index.md (PERF-001)`
- `Primary Code Areas`: `scripts/plan-coverage.sh`（--check-perf-budget enforcing）；CI 性能基准入口见 18_release

## 1. Scope & Authority {#perf-budget-scope}

本章是 **op 维度性能预算唯一权威**：登记关键路径的 latency / RSS 目标预算与 CI fuse 阈值。

- **Owns**：关键路径 P50/P99 latency budget（§2）、RSS Δ budget（§2）、CI fuse 阈值（§3）。
- **Defers To**：profile 枚举（`standard` / `low-spec`）、feature matrix 与 profile fallback 归 `17_tech_stack#performance-profiles-and-feature-matrix`。本章 **MUST NOT** 新增 profile、改默认 feature matrix 或定义 profile fallback（吸收 00 §7 单一可信来源）。
- **Profile 列约束**：§2 的 `Profile` 列只能引用 `17_tech_stack` 已定义的 profile 枚举值；新增 profile 必须先按 00 §8 修改 `17_tech_stack`，再在此引用。
- **预算性质**：本表是**目标契约**（target budget），不是实测快照；实测回归由 CI 基准（`18_release`）对照本表执行。

## 2. Critical Path Budget {#critical-path-budget}

关键路径目标预算。latency 为端到端用户感知耗时（ms）；`RSS Budget`：op 行为相对空闲基线的常驻内存增量（如 `+8MB`），cold mount 行为常驻基线上限（如 `≤128MB`）。`—` 表示该 profile 下功能关闭（见 feature matrix）。

| Critical Path (op-flow) | Profile | P50 | P99 | RSS Budget | Test Entry |
|---|---|---|---|---|---|
| `flow.repo.open-doc` | `standard` | 60ms | 200ms | +8MB | `PERF-001` open-doc bench |
| `flow.repo.open-doc` | `low-spec` | 120ms | 400ms | +4MB | `PERF-001` open-doc bench |
| `flow.doc.edit-confirmed-op` (edit→ack) | `standard` | 25ms | 90ms | +2MB | `PERF-001` edit-ack bench |
| `flow.doc.edit-confirmed-op` (edit→ack) | `low-spec` | 40ms | 150ms | +2MB | `PERF-001` edit-ack bench |
| `flow.search.query` | `standard` | 30ms | 120ms | +12MB | `SEARCH-001` query bench |
| `flow.search.query` | `low-spec` | — | — | — | feature off (no index) |
| `flow.sc.commit` (stage→commit) | `standard` | 50ms | 200ms | +4MB | `DIFF-FEAT-01` commit bench |
| `flow.sc.commit` (stage→commit) | `low-spec` | 80ms | 300ms | +4MB | `DIFF-FEAT-01` commit bench |
| `flow.repo.branch-switch` | `standard` | 40ms | 150ms | +6MB | `REPO-FEAT-02` switch bench |
| `flow.repo.branch-switch` | `low-spec` | 70ms | 250ms | +4MB | `REPO-FEAT-02` switch bench |
| `flow.rendering.large-doc-prefetch` | `standard` | 80ms | 300ms | +16MB | `RENDER-LARGE-001` prefetch bench |
| `flow.rendering.large-doc-prefetch` | `low-spec` | 150ms | 500ms | +8MB | `RENDER-LARGE-001` prefetch bench |
| `flow.net.sync-transfer` (apply) | `standard` | 35ms | 140ms | +4MB | `NET-FEAT-02` sync bench |
| `flow.net.sync-transfer` (apply) | `low-spec` | 60ms | 220ms | +4MB | `NET-FEAT-02` sync bench |
| cold mount (repo open → ready) | `standard` | 300ms | 800ms | ≤128MB | `REL-002` startup bench |
| cold mount (repo open → ready) | `low-spec` | 500ms | 1200ms | ≤64MB | `REL-002` startup bench |

**RSS baseline（常驻基线，非 op 增量）**：`standard` ≤ 128MB，`low-spec` ≤ 64MB（各 profile 的 `MEM_CACHE_MB` 默认值定义见 `17_tech_stack`，本章不复制其数值）。前端 WASM 堆目标见 `17_tech_stack` §4（Mobile < 64MB / Desktop < 128MB）；本章不重定义 WASM 堆约束。

## 3. CI Fuse Thresholds {#perf-budget-fuse}

CI fuse 把 §2 中的 P99 列作为硬阈值；基准回归超出 P99 即 fail（不接受"略超"）。

| Fuse | 阈值来源 | 失败动作 |
|---|---|---|
| edit-ack P99 | §2 `flow.doc.edit-confirmed-op` P99 | CI 性能 job 失败 |
| open-doc P99 | §2 `flow.repo.open-doc` P99 | CI 性能 job 失败 |
| cold-mount P99 | §2 cold mount P99 | CI 性能 job 失败 |
| RSS baseline | §2 RSS baseline 上限 | CI 内存 job 失败 |

**文档侧 fuse（`scripts/plan-coverage.sh --check-perf-budget`）**：校验本章 §2 预算表已写入数值（非 TBD/TODO），保证预算契约不回退为空壳；**运行时回归**由 CI 基准（`18_release`）对照本表执行，不在 plan-coverage 内跑基准。

## 4. Profile Reference (Defers To 17_tech_stack)

profile 枚举（`standard` / `low-spec`）、定义与 feature matrix 唯一权威为 `17_tech_stack#performance-profiles-and-feature-matrix`。本章 §2 `Profile` 列只引用枚举值，不复制其 feature 组合；`flow.search.query` 在 `low-spec` 标 `—` 即依据 17 feature matrix 中 Full-Text Search 关闭。

## 5. Related Configuration (本章相关配置)

- `DEVE_PROFILE`、`MEM_CACHE_MB`：定义归 `17_tech_stack` §6 / `15_settings`；本章只引用其对 budget 的影响。
- CI 性能基准入口与发布门禁：归 `18_release`。
