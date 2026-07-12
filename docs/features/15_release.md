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
- 首个公开 tag 只由 `release.yml` 直接监听；`v*` glob 触发后必须先验证完整 SemVer，checkout 后去掉前导 `v` 的 tag 还必须与 workspace、Desktop Tauri、Mobile Tauri 版本逐字节一致，包含 prerelease/build metadata。非法或不一致 tag 在 build/publish 前失败。它在 quality gates 与 Docker publish 成功后调用 reusable native delivery track。Windows MSI/NSIS、macOS DMG 与 Android ARM64 APK 全部构建成功后，publish job 还要验证 downloaded containers 总文件数与 exact artifact manifest；资产先上传到 draft，GitHub API 复核完整后才公开一次 GitHub Release。Linux Desktop 与 iOS 不在 first-tag artifact set。
- Docker release image 只构建一次；runtime/login 与双客户端浏览器 smoke 必须运行该 candidate image，成功后才赋予 version/latest tag 并 push。两个远端 tag 必须解析到同一 manifest digest，smoke 不得通过隐式 rebuild 测到另一份内容。
- Windows/macOS public-preview packages 可以 unsigned；Android 只有在 signing secrets 齐全时才是可安装 signed APK，否则只能作为明确标记的 unsigned diagnostic artifact。任何这些 artifacts 都不等于 signing、notarization、store 或 physical-device readiness。
- 后端不会把仍含 Trunk development live-reload 标记的 `index.html` 当作 release 前端服务。显式 `DEVE_STATIC_DIR` 命中该类文件时启动应 fail-closed；嵌入式前端命中该类文件时应退回非前端交付形态，并由浏览器 smoke 证明真实 release frontend 是否可用。
- 其它客户端交付形态可以存在，但成熟度应明确。
- 首个公开 tag 不发布 Linux GTK3/WebKitGTK 4.x native artifacts；Linux 用户使用 Web / Server / Docker 交付面。
- Docker image 可能先于 native track 完成而发布；native build 失败时不得留下公开 GitHub Release，也不得把已有 GHCR image 表述为完整 first-tag delivery。
- Windows packaged UI gate 必须驱动已安装 Desktop 的真实 WebView，覆盖 native session、创建/编辑、commit/history、Settings focus trap 与关闭后 sidecar 清理；快速 marker startup probe 仍保留，但不能单独证明 UI 可用。
- Native build、manifest、draft upload 或 API 复核失败时，公开 GitHub Release 必须保持不存在；失败产生的 draft 只作为显式恢复对象，不得被报告为已发布版本。
- 同一 tag 的 workflow rerun 只允许复用 draft；若 Release 已公开，必须在上传资产前拒绝自动覆盖并转入 maintainer 显式恢复。

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
- Docker/Web dashboard 的 CPU 与内存占用应优先反映当前可见容器 cgroup hierarchy，
  而不是 Docker host / VM 的聚合使用量；只有同一 cgroup 数据源的 usage/capacity
  不完整时才整条回退宿主 Linux 指标。

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
  `tag_blocker=yes` 的 warning 会 fail-closed。当前 GTK3/glib warning 依据 ADR
  0006 Route 2 重新归类为非 blocker，因为 Linux GTK3 native artifacts 不进入
  first-tag release set。
- 首个公开 tag 的 Ledger / Redb / WS protocol / Projection Locator /
  Projection Backup locator 当前格式必须能在 `docs/registry/first-tag-format-matrix.md`
  中查到，并由 release baseline 钉住对应 plan 与代码常量；未登记的格式变更不能声明 tag-ready。
- 当前 first-tag 精确基线为 ledger entry format v3 / `DEVELDG3`、redb schema v3、WS protocol lockstep `12..=12`。schema v2 只允许显式离线只读导出后重建，不属于正常 runtime 兼容窗口。
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
