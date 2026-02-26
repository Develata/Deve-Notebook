# [PROJECT_NAME] 宪章（Constitution）
<!-- 示例：Spec Constitution、TaskFlow Constitution 等 -->

## 核心原则（Core Principles）

### [PRINCIPLE_1_NAME]
<!-- 示例：I. Library-First -->
[PRINCIPLE_1_DESCRIPTION]
<!-- 示例：每个 feature 从可独立库开始；库必须自包含、可独立测试、可文档化；目标必须清晰，禁止仅为组织而拆库 -->

### [PRINCIPLE_2_NAME]
<!-- 示例：II. CLI Interface -->
[PRINCIPLE_2_DESCRIPTION]
<!-- 示例：每个库都通过 CLI 暴露能力；文本 I/O 协议：stdin/args -> stdout，错误输出到 stderr；支持 JSON 与人类可读格式 -->

### [PRINCIPLE_3_NAME]
<!-- 示例：III. Test-First (NON-NEGOTIABLE) -->
[PRINCIPLE_3_DESCRIPTION]
<!-- 示例：强制 TDD：先写测试 -> 用户确认 -> 测试先失败 -> 再实现；严格执行 Red-Green-Refactor -->

### [PRINCIPLE_4_NAME]
<!-- 示例：IV. Integration Testing -->
[PRINCIPLE_4_DESCRIPTION]
<!-- 示例：必须做集成测试的场景：新增库契约、契约变更、服务间通信、共享 schema -->

### [PRINCIPLE_5_NAME]
<!-- 示例：V. Observability、VI. Versioning & Breaking Changes、VII. Simplicity -->
[PRINCIPLE_5_DESCRIPTION]
<!-- 示例：文本 I/O 提升可调试性；要求结构化日志；或采用 MAJOR.MINOR.BUILD；或坚持 YAGNI 的简洁策略 -->

## [SECTION_2_NAME]
<!-- 示例：附加约束、安全要求、性能标准等 -->

[SECTION_2_CONTENT]
<!-- 示例：技术栈要求、合规标准、部署策略等 -->

## [SECTION_3_NAME]
<!-- 示例：开发流程、评审流程、质量门禁等 -->

[SECTION_3_CONTENT]
<!-- 示例：代码评审要求、测试门禁、部署审批流程等 -->

## 治理（Governance）
<!-- 示例：宪章高于其他实践；修订需文档、审批与迁移计划 -->

[GOVERNANCE_RULES]
<!-- 示例：所有 PR/Review 必须校验合规；复杂度必须可论证；运行期开发指引见 [GUIDANCE_FILE] -->

**Version**: [CONSTITUTION_VERSION] | **Ratified**: [RATIFICATION_DATE] | **Last Amended**: [LAST_AMENDED_DATE]
<!-- Example: Version: 2.1.1 | Ratified: 2025-06-13 | Last Amended: 2025-07-16 -->
