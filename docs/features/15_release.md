# 15_release.md - 交付与分发体验篇

本章描述用户或部署者如何获得、运行、更新和理解当前交付形态。

原子操作示例：[`operations/release_ci.md`](./operations/release_ci.md)

细粒度操作链：
[`release_tag_dispatch.md`](./operations/release_tag_dispatch.md),
[`release_quality_gates.md`](./operations/release_quality_gates.md),
[`release_artifact_publish.md`](./operations/release_artifact_publish.md),
[`release_delivery_verification.md`](./operations/release_delivery_verification.md)

相关技术栈操作链：
[`tech_stack_platform_release_channel.md`](./operations/tech_stack_platform_release_channel.md)

## 功能目标

- 部署者应知道有哪些主要分发形态。
- 用户应能理解当前运行实例来自哪种交付方式，以及升级后的预期行为。

## 功能项

### 1. 分发形态

- Web / Server / Docker 是当前主要交付面。
- Docker/Server 当前主通道是单个 `deve_cli` 二进制；当 CLI 在 `trunk build --release` 之后构建时，前端静态资源会被编译进二进制。
- 其它客户端交付形态可以存在，但成熟度应明确。

### 2. 版本与升级预期

- 用户应能知道当前运行的大致版本或构建来源。
- 当前 `/api/node/role` 与 Web dashboard 应暴露只读运行摘要，包括版本、profile、环境和交付形态。
- 升级后核心数据与核心工作流不应无提示地断裂。

### 3. 运行环境提示

- 部署者应能分辨当前是本地开发、服务器部署还是容器化运行。
- 不同运行环境的差异不应混淆成产品功能差异。
- 生产服务器/容器运行必须显式提供 `AUTH_SECRET` 和 `AUTH_PASS`；本地开发应使用 `deve serve --dev` 或 `DEVE_ENV=development`。

## 非目标

- 当前阶段不要求在 Web UI 内完整实现发布渠道管理。
- 当前阶段不要求把运维流程全部暴露给普通终端用户。

## Chrome MCP 验收实例

### RELEASE-FEAT-01: 当前运行形态与版本边界可理解

前置条件：

- 打开当前部署实例。

步骤：

1. 查看设置、关于页或其它公开入口中的版本/运行信息。
2. 观察是否能区分当前实例的大致交付形态。

期望结果：

- 版本或运行信息可被用户/部署者理解。
- 不会把实验构建、未来渠道或未完成交付方式误导成稳定主通道。
