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
- 后端不会把仍含 Trunk development live-reload 标记的 `index.html` 当作 release 前端服务。显式 `DEVE_STATIC_DIR` 命中该类文件时启动应 fail-closed；嵌入式前端命中该类文件时应退回非前端交付形态，并由浏览器 smoke 证明真实 release frontend 是否可用。
- 其它客户端交付形态可以存在，但成熟度应明确。

### 2. 版本与升级预期

- 用户应能知道当前运行的大致版本或构建来源。
- 当前 `/api/node/role` 与 Web dashboard 应暴露只读运行摘要，包括版本、profile、环境、交付形态和 repo health 聚合状态。
- `/api/node/role` 中的 `api-only` 只能说明当前没有可服务的前端资产，不能单独证明嵌入式前端健康；发布前必须配套浏览器入口 smoke。
- 升级后核心数据与核心工作流不应无提示地断裂。
- 首个 stable 前产生的无版本或旧 codec 开发期 ledger / `.redb` 不属于兼容承诺；正式运行时应 fail-closed 并提示显式 reset / repair / migration。

### 3. 运行环境提示

- 部署者应能分辨当前是本地开发、服务器部署还是容器化运行。
- 不同运行环境的差异不应混淆成产品功能差异。
- 生产服务器/容器运行必须显式提供 `AUTH_SECRET` 和 `AUTH_PASS`；本地开发应使用 `deve serve --dev` 或 `DEVE_ENV=development`。
- degraded repo 必须被显示为运行状态，而不是伪装为全局启动失败或静默健康。

### 4. Mesh 与 Native 双模式成熟度提示

- Docker multi-client smoke 验证“单服务端 + 多 WebLightPeer”。
- Docker P2P mesh smoke 验证“两服务端 + 静态 FullPeer mesh + shadow-only apply”。
- Desktop/Android/Mobile native-packaging 默认 LocalBackend 可作为本机 FullPeer；RemoteBrowser 显式连接远端 Docker/Web HTTPS origin。
- Native 双模式 smoke 可以作为功能证据，但不能替代签名、store、physical-device 或后台同步 release readiness。
- 对纯文本 baseline 与确定性边界检查合同，开发者可以使用独立 Rust CLI mirror（例如 `cargo run -p deve_baseline -- all`）做本地验收；需要覆盖历史 baseline shell 中的确定性 `cargo test` 调度时，可以显式运行 `cargo run -p deve_baseline -- full`。validation script ownership policy 要求确定性规则归 Rust/CLI，shell 仅保留兼容入口、CI glue 或真实平台编排；Docker、runtime、native install/package 与 GitHub artifact smoke 不被强行塞进 `deve_baseline -- all`。这些入口减少 Windows/WSL 环境对 bash/awk/rg runtime 的依赖，不改变普通用户可见命令面。
- 发布依赖审计必须区分 hard vulnerabilities 与 non-vulnerability warnings。Hard vulnerabilities
  不允许进入公开 tag；warnings 必须在 release audit warning registry 中有明确
  allowlist 理由或替换路线，新增 warning 未登记时 fail-closed。
- `cargo audit` 的 `yanked` warning 若没有 RustSec advisory id，registry 使用
  synthetic `YANKED` advisory key 并仍按 crate / version / kind 精确匹配。
- 首个公开 tag 的 release audit 作业必须设置 `DEVE_RELEASE_TAG_READY_REQUIRED=1`
  或显式运行 `deve_baseline release-audit-gate tag-ready`；仍登记为
  `tag_blocker=yes` 的 warning 会 fail-closed，直到完成替换、重新归类或 USER
  决策。
- `REL-013` reliability/observability governance baseline 固定 SLO/SLI、telemetry schema、metrics taxonomy、tracing、health mapping、alert tier 与 DR index 的发布前检查；它是合同漂移闸门，不声明 runtime telemetry 已完整实现。

## 非目标

- 当前阶段不要求在 Web UI 内完整实现发布渠道管理。
- 当前阶段不要求把运维流程全部暴露给普通终端用户。
- 当前阶段不把 P2P 自动发现、NAT 穿透、自动 merge、store 分发或 physical-device release readiness 作为 release 承诺。

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

### RELEASE-FEAT-02: Mesh 与 Native 双模式证据边界可理解

前置条件：

- 已运行 Docker multi-client、Docker P2P mesh 或 native 双模式 smoke 之一。

步骤：

1. 查看 runbook、验收用例和 smoke 输出说明。
2. 对照当前运行形态、LocalBackend 默认模式和 RemoteBrowser 远端 URL。

期望结果：

- 文档能区分单服务端 WebLightPeer smoke 与多服务端 FullPeer mesh smoke。
- 文档明确 LocalBackend 与 RemoteBrowser 的不同 authority 边界。
- 文档不把 native 双模式 smoke 误写为签名、store、physical-device 或后台同步 ready。
