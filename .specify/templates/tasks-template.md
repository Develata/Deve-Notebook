---
description: "Feature 实施任务模板（按用户故事与依赖顺序）"
---

# Tasks: [FEATURE NAME]

**Input**: Design documents from `/specs/[###-feature-name]/`
**Prerequisites**: plan.md, spec.md（必需）; research.md, data-model.md, contracts/（按需）

**测试策略**: 默认至少包含“与改动直接相关”的验证任务；Rust 改动必须包含 `cargo clippy --all-targets --all-features -- -D warnings`。

## Format: `[ID] [P?] [Story] Description with file path`

- `- [ ]` 开头为强制格式
- `[P]` 表示可并行（不同文件、无未完成依赖）
- `[Story]` 仅用于用户故事阶段（如 `[US1]`）
- 每条任务必须包含明确文件路径

## 路径约定（按本仓库）

- Core: `crates/core/src/...`
- CLI: `apps/cli/src/...`
- Web: `apps/web/src/...`
- Tests: 对应 crate/app 的测试目录

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: 初始化本 feature 的最小骨架与校验入口

- [ ] T001 确认涉及模块与目标路径（crates/apps/plugins）
- [ ] T002 建立 feature 所需文件骨架（遵守 <130 行目标）
- [ ] T003 [P] 补充必要配置/开关并记录默认值

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 在任何用户故事前必须完成的公共前置

- [ ] T004 建立共享数据结构与错误类型于 [file path]
- [ ] T005 [P] 定义接口边界与契约于 [file path]
- [ ] T006 [P] 记录复杂逻辑约束（Invariants/Pre/Post）于 [file path]
- [ ] T007 建立基础日志/可观测埋点于 [file path]

**Checkpoint**: Foundation 完成后，用户故事可并行推进

---

## Phase 3: User Story 1 - [Title] (Priority: P1) 🎯 MVP

**Goal**: [该故事交付价值]
**Independent Test**: [可独立验证方式]

### Validation for User Story 1

- [ ] T008 [P] [US1] 增加故事级验证用例于 [test file path]
- [ ] T009 [US1] 运行定向测试并记录结果于 [artifact/log path]

### Implementation for User Story 1

- [ ] T010 [P] [US1] 实现数据模型/结构于 [file path]
- [ ] T011 [US1] 实现核心服务逻辑于 [file path]
- [ ] T012 [US1] 实现接口/命令/UI 入口于 [file path]
- [ ] T013 [US1] 补充错误处理与边界行为于 [file path]

**Checkpoint**: US1 可独立运行与验证

---

## Phase 4: User Story 2 - [Title] (Priority: P2)

**Goal**: [该故事交付价值]
**Independent Test**: [可独立验证方式]

- [ ] T014 [P] [US2] 增加故事级验证用例于 [test file path]
- [ ] T015 [P] [US2] 实现模型/服务于 [file path]
- [ ] T016 [US2] 实现接口集成于 [file path]
- [ ] T017 [US2] 完成定向验证并记录于 [artifact/log path]

---

## Phase 5: User Story 3 - [Title] (Priority: P3)

**Goal**: [该故事交付价值]
**Independent Test**: [可独立验证方式]

- [ ] T018 [P] [US3] 增加故事级验证用例于 [test file path]
- [ ] T019 [P] [US3] 实现核心改动于 [file path]
- [ ] T020 [US3] 集成并完成验收于 [file path]

---

## Final Phase: Polish & Cross-Cutting Concerns

- [ ] T021 [P] 文档与注释校对（仅保留必要注释）于 [file path]
- [ ] T022 运行 `cargo fmt --check`
- [ ] T023 运行定向测试（列出具体命令）
- [ ] T024 运行 `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] T025 记录与上游 plan 的偏差到 `deve-note report/...`

---

## Dependencies & Execution Order

- Phase 1 -> Phase 2 -> User Stories -> Final Phase
- 用户故事默认可并行，但同文件任务必须串行
- 每个故事完成后先做独立验证，再进入下一优先级

## Parallel Example

```bash
# 并行示例（不同文件）
Task: "[US1] implement model in crates/core/src/..."
Task: "[US1] implement endpoint in apps/cli/src/..."
```

## Notes

- 避免模糊任务（如“优化一下”）；每条必须可执行、可验证
- 任务应能追溯到 FR/NFR/SC 与上游 `deve-note plan/` 约束
