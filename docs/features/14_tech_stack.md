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
- Mobile 平台 bridge 依赖仅在对应 Android/iOS target 且启用 `native-packaging` 时进入构建；默认 Mobile skeleton 不携带 JNI、Objective-C 或 Tauri packaging runtime。

### 2. 资源预算的可感知影响

- 低配环境下，用户可能看不到某些重能力或高级扩展。
- 核心功能必须优先保持稳定，而不是堆叠高消耗特性。
- 原子操作示例：[`operations/search_query.md`](./operations/search_query.md)

### 3. 兼容边界

- 产品应清楚区分“当前已稳定支持”和“未来计划支持”。
- 不应让未完成平台被误导成已正式可用。
- Git ecosystem bridge 稳定边界是 CLI status/export/import/push 与 Web CLI-only notices；可点击 Git mirror repair UI 不得被描述为已完成。
- Git repair UI 后续边界必须保持 `.notegit` / ledger authority，`.git` 只作为 projection mirror；任何写 Git 的路径都必须显式、可审计、可失败关闭。

### 4. 自动验收与 Smoke 的关系

- 验收矩阵描述每个功能或发布旅程需要什么证据；它不是 smoke 脚本，也不会仅凭路径存在就声称功能已通过。
- Smoke 是 producer 的一种，负责在 Docker、真实浏览器、Desktop WebView 或 Android emulator/设备上执行一条具体业务旅程。
- Rust `deve_baseline acceptance-run` 根据 producer registry 选择并执行适用 evidence：`ci` 层运行明确绑定的 test/script 且不伪造 receipts，runtime 层生成绑定 HEAD、平台、surface/mode、producer contract 与完整 execution evidence 集合的 receipts；`acceptance-collect` 只聚合完整且同源的 receipt group，不修改产品数据或业务 authority。
- Windows 可以完整驱动 Android emulator 的 LocalBackend/RemoteBrowser lifecycle；最终可写结论仍由 emulator 内的 WebView provider 与 non-extractable Ed25519 WebCrypto probe 决定。
- “矩阵结构完整”“producer 已登记”“当前宿主 smoke 通过”“跨平台 tag-ready”是四个不同结论，界面和发布说明不得混用。

## 非目标

- 当前阶段不要求所有平台具备完全一致的壳层体验。
- 当前阶段不允许以重依赖堆叠来替代核心功能稳定性。
- 当前阶段不允许用后台自动 Git writer 替代显式 CLI/手动确认式 repair。
- 当前阶段不允许把结构 checker、脚本 exit 0 或不合格 Android WebView 的只读负向结果描述为完整业务验收通过。

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
