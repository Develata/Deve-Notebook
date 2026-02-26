# Feature Specification: [FEATURE NAME]

**Feature Branch**: `[###-feature-name]`
**Created**: [DATE]
**Status**: Draft
**Input**: User description: "$ARGUMENTS"

## Upstream Alignment *(mandatory)*

- 上游依据：[`deve-note plan/...`](必填)
- 长期原则引用：[`AGENTS.md`](必填)
- 本 feature 的非目标（Out of Scope）：[明确列出]

## User Scenarios & Testing *(mandatory)*

### User Story 1 - [Brief Title] (Priority: P1)

[以用户语言描述价值]

**Why this priority**: [为什么是 P1]
**Independent Test**: [只实现本故事也可验证价值]

**Acceptance Scenarios**:

1. **Given** [初始态], **When** [动作], **Then** [结果]
2. **Given** [初始态], **When** [动作], **Then** [结果]

---

### User Story 2 - [Brief Title] (Priority: P2)

[描述]

**Why this priority**: [说明]
**Independent Test**: [说明]

**Acceptance Scenarios**:

1. **Given** [初始态], **When** [动作], **Then** [结果]

---

### User Story 3 - [Brief Title] (Priority: P3)

[描述]

**Why this priority**: [说明]
**Independent Test**: [说明]

**Acceptance Scenarios**:

1. **Given** [初始态], **When** [动作], **Then** [结果]

## Edge Cases *(mandatory)*

- 边界条件：[例如输入为空、超长、并发冲突]
- 失败场景：[例如网络失败、依赖不可用]
- 恢复路径：[例如重试、回滚、降级]

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST [能力]
- **FR-002**: System MUST [能力]
- **FR-003**: Users MUST be able to [交互]
- **FR-004**: System MUST [数据行为]
- **FR-005**: System MUST [可观测/审计行为]

### Non-Functional Requirements *(mandatory)*

- **NFR-001 (Memory)**: 在目标场景下内存预算 [填写具体数值]
- **NFR-002 (Performance)**: [填写可度量指标，如 p95]
- **NFR-003 (Reliability)**: [恢复/可用性目标]
- **NFR-004 (Security)**: [鉴权、敏感数据处理约束]
- **NFR-005 (Portability)**: [Windows/Linux 路径与行为一致性]

### Key Entities *(if data involved)*

- **[Entity 1]**: [含关键字段与约束]
- **[Entity 2]**: [与其他实体关系]

## Constraints from Constitution *(mandatory)*

- 文件长度目标 <130，250 为硬上限
- 复杂逻辑必须声明 Invariants/Pre-conditions/Post-conditions
- Rust 质量门禁：`cargo fmt` + 定向测试 + `cargo clippy --all-targets --all-features -- -D warnings`

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: [用户任务完成时间/成功率]
- **SC-002**: [性能指标]
- **SC-003**: [可靠性/错误率指标]
- **SC-004**: [业务指标或体验指标]

## Assumptions

- [默认前提 1]
- [默认前提 2]

## Clarifications

### Session [YYYY-MM-DD]

- Q: [问题] -> A: [答案]
