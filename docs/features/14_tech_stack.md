# 14_tech_stack.md - 支持矩阵体验篇

本章从产品视角描述当前支持的平台、运行边界与性能预算对用户的实际影响。

原子操作示例：[`operations/tech_stack_runtime_budget.md`](./operations/tech_stack_runtime_budget.md)

细粒度操作链：
[`tech_stack_dependency_policy.md`](./operations/tech_stack_dependency_policy.md),
[`tech_stack_runtime_budget_check.md`](./operations/tech_stack_runtime_budget_check.md),
[`tech_stack_platform_release_channel.md`](./operations/tech_stack_platform_release_channel.md)

## 功能目标

- 用户或部署者应明确知道当前主要支持哪些端。
- 用户应理解低配环境下哪些能力会收敛或不可用。

## 功能项

### 1. 当前主要支持端

- Web 是当前最直接可见的交付形态。
- CLI / Docker / Server 是系统控制与部署面的一部分。
- Desktop / Android / 后续 iOS 属于多端支持矩阵，但阶段成熟度可能不同。

### 2. 资源预算的可感知影响

- 低配环境下，用户可能看不到某些重能力或高级扩展。
- 核心功能必须优先保持稳定，而不是堆叠高消耗特性。
- 原子操作示例：[`operations/search_query.md`](./operations/search_query.md)

### 3. 兼容边界

- 产品应清楚区分“当前已稳定支持”和“未来计划支持”。
- 不应让未完成平台被误导成已正式可用。

## 非目标

- 当前阶段不要求所有平台具备完全一致的壳层体验。
- 当前阶段不允许以重依赖堆叠来替代核心功能稳定性。

## Chrome MCP 验收实例

### STACK-FEAT-01: 支持矩阵边界可感知

前置条件：

- 在 Web 端打开应用。

步骤：

1. 查看首页、设置或相关文档入口中是否明确当前运行形态。
2. 检查是否存在把未来平台能力误导为当前已完成的入口。

期望结果：

- 当前支持边界清楚。
- 未完成平台或能力不会伪装成正式可用。
