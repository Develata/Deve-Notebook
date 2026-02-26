<!--
Sync Impact Report
- Version change: 0.0.0 -> 1.0.0
- Modified principles: template placeholders -> concrete project principles
- Added sections: 资源与工程约束, 工作流与质量门禁, Governance
- Removed sections: none
- Templates requiring updates: ⚠ pending `.specify/templates/plan-template.md`, `.specify/templates/spec-template.md`, `.specify/templates/tasks-template.md`
- Deferred TODOs: none
-->

# Deve Notebook Constitution

## Core Principles

### I. 低资源优先（Low-Resource First, MUST）
本项目默认部署目标为 768MB-1GB RAM VPS。所有设计与实现必须优先保证低内存可运行性。
新增依赖前必须评估内存与启动开销；若存在更轻量方案，应优先采用。涉及数据处理时应优先考虑
Zero-copy 与减少分配（allocations）。

### II. 模块化铁律（Modularity Limits, MUST）
单文件目标长度 <130 行；250 行为硬熔断上限。若接近或超过上限，必须立即拆分模块，不得继续堆叠。
拆分应以职责边界为准，保持可读性、可测试性与低认知负担。

### III. 数学与形式化可迁移（Formal-Friendly, MUST）
涉及同步、存储、图结构、冲突解决等复杂逻辑时，必须在文档注释或设计说明中明确：
Invariants、Pre-conditions、Post-conditions。规则表述应可被测试验证，并为未来迁移 Lean4 留出空间。

### IV. 规格分层治理（Spec Layering, MUST）
`deve-note plan/` 是上游知识库（白皮书/架构基线/长期原则）；
`spec-kit` 是下游 feature 执行流水线（spec/plan/tasks/implement）；
`deve-note report/` 是复盘与偏差回灌。
任何 feature 任务必须可追溯到上游约束；执行偏差必须回写 report 并决定是否反哺 plan/模板。

### V. 质量门禁先于交付（Quality Gates, MUST）
Rust 工程必须满足 `cargo fmt` 与 `cargo clippy --all-targets --all-features -- -D warnings`。
测试应优先按 feature 相关最小范围执行，避免无谓全量回归。未通过质量门禁不得标记为完成。

## 资源与工程约束

- Language baseline: Rust Edition 2024（新代码默认遵循）。
- Error handling: 应用层使用 `anyhow::Result`；库层使用 `thiserror` 定义可恢复错误。
- Path handling: 必须保证 Windows 兼容；路径规范化优先使用 `deve_core::utils::path::to_forward_slash`。
- Security baseline: 禁止提交密钥、凭据、`.env` 实际值等敏感信息。
- Dependency policy: 引入新依赖需说明必要性、替代方案与资源影响。

## 工作流与质量门禁

1. Docs First（MUST）
   - 编码前先查 `deve-note plan/`、`deve-note report/`、`README` 与当前 feature 文档。
2. Clarify Then Plan（SHOULD）
   - 在 `/speckit.plan` 前尽量完成关键澄清，减少返工。
3. Task Execution（MUST）
   - 任务必须具备明确文件路径、依赖关系与可验证完成标准。
4. Validation（MUST）
   - 至少执行与改动直接相关的测试；Rust 改动需执行 clippy 门禁。
5. Review Traceability（MUST）
   - PR/Review 应能回答：改动对应哪个上游约束、哪个 feature 需求、是否有偏差与回灌记录。

## Governance

- 本宪章高于日常工作习惯与临时约定；冲突时以宪章为准。
- 宪章修订采用语义化版本：
  - MAJOR：原则删除/重定义或不兼容治理变更；
  - MINOR：新增原则/章节或实质扩展；
  - PATCH：措辞澄清、错别字、非语义调整。
- 每次修订必须记录：变更原因、影响范围、需要同步的模板/流程文件。
- 合规检查点至少覆盖：规格分层、资源约束、质量门禁、可追溯性。

**Version**: 1.0.0 | **Ratified**: 2026-02-26 | **Last Amended**: 2026-02-26
