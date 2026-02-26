# Implementation Plan: [FEATURE]

**Branch**: `[###-feature-name]` | **Date**: [DATE] | **Spec**: [link]
**Input**: Feature specification from `/specs/[###-feature-name]/spec.md`

## Summary

[从 spec 提炼：核心用户价值、边界、本次实现策略]

## Technical Context

**Language/Version**: [Rust 2024 or NEEDS CLARIFICATION]
**Primary Dependencies**: [Leptos/Axum/Redb/Tantivy/Rhai 等 or NEEDS CLARIFICATION]
**Storage**: [Redb/files/N/A]
**Testing**: [cargo test + targeted tests + clippy]
**Target Platform**: [Linux VPS 768MB-1GB RAM / WASM / Windows]
**Project Type**: [library/cli/web-service/web-frontend/workspace]
**Performance Goals**: [例如 p95、吞吐、内存峰值]
**Constraints**: [必须量化：内存/延迟/并发/离线能力]
**Scale/Scope**: [用户规模、仓库规模、数据规模]

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [ ] 低资源约束：默认目标 768MB-1GB RAM，可说明预算与降级策略
- [ ] 模块化约束：文件目标 <130 行，>=250 行必须拆分
- [ ] 形式化约束：复杂逻辑已声明 Invariants/Pre-conditions/Post-conditions
- [ ] 质量门禁：计划中包含 `cargo fmt`、定向测试、`cargo clippy --all-targets --all-features -- -D warnings`
- [ ] 分层治理：引用了上游 `deve-note plan/` 条目；偏差将记录到 `deve-note report/`
- [ ] 安全约束：无敏感信息入库；路径与平台兼容性（含 Windows）已考虑

## Upstream Traceability

- 上游知识条目：[`deve-note plan/...`](填写具体文件)
- 关键约束引用：
  - [约束1]
  - [约束2]
- 偏差预案（若有）：[触发条件 -> 记录到 `deve-note report/...`]

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
└── tasks.md
```

### Source Code (repository root)

```text
crates/core/
apps/cli/
apps/web/
plugins/
```

**Structure Decision**: [本次 feature 具体涉及路径，删掉无关项]

## Phase 0 - Research

- 待澄清项清单（NEEDS CLARIFICATION -> 决策）
- 依赖与替代方案（尤其是低内存替代）
- 风险前置验证（复杂度/性能/兼容性）

## Phase 1 - Design

- `data-model.md`：实体、约束、关系、状态迁移
- `contracts/`：对外接口契约（若适用）
- `quickstart.md`：最小验证路径
- 设计后再次执行 Constitution Check

## Complexity Tracking

> 仅在违反宪章且必须豁免时填写

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [示例] | [当前必要性] | [为何更简单方案不满足] |

## Definition of Ready (for /speckit.tasks)

- [ ] 所有 NEEDS CLARIFICATION 已解决或明确延期
- [ ] 约束可测量（尤其内存/性能）
- [ ] 用户故事可独立交付
- [ ] 验证命令明确可执行
