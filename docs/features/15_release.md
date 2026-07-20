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
- 首个公开 tag 只由 `release.yml` 直接监听；真正的构建与 target-host 验收由 tag 前手动 `release-candidate.yml` 完成。候选输入版本必须与 workspace、Desktop Tauri、Mobile Tauri 版本逐字节一致，所有 receipts 和制品绑定同一 HEAD。聚合器验证后封存 candidate bundle；maintainer 必须创建 annotated tag，并用唯一 `Deve-Acceptance-Aggregate-Run: <run-id>` trailer 绑定该 aggregate。tag workflow 只提升该 bundle，禁止重新 build、package、rename 或选择“最新”但未绑定的 run。Windows MSI/NSIS、macOS DMG 与已签名 Android ARM64 APK、Docker archive、SBOM、checksums 和 attestations 作为一个 allowlisted set 被复核；资产先上传到 draft，远端名称与 SHA-256 完全一致后才公开。Linux Desktop 与 iOS 不在 first-tag artifact set。
- Docker release image 在 candidate workflow 只构建一次；runtime/login、双客户端、离线恢复、Source Control、External Changes 与 P2P gap/recovery 必须运行该 exact image。候选封存 Docker archive SHA-256 与 image ID；tag 后 load 同一 archive，成功后才 push Docker-safe version tag。stable 只有在 SemVer 与 Git ancestry 均前进时才更新 `latest`，prerelease 不更新；stable 的两个远端 tag 必须解析到同一 manifest digest并生成 registry digest attestation。
- Windows/macOS public-preview packages 可以 unsigned；Android 缺 signing secret 时的 unsigned APK 只能作为非候选诊断产物，绝不能进入 candidate allowlist或替代 signed ARM64 APK。任何这些 artifacts 都不等于 notarization、store 或 physical-device readiness。
- Android GitHub target-host lifecycle 运行同一 HEAD 的 x86_64 emulator package；封存的 ARM64 APK 另经严格签名与 signer 复核。前者证明功能与 WebCrypto capability，后者证明候选字节和签名身份，不能把 x86_64 receipt 写成 ARM64 逐字节安装证据。
- 后端不会把仍含 Trunk development live-reload 标记的 `index.html` 当作 release 前端服务。显式 `DEVE_STATIC_DIR` 命中该类文件时启动应 fail-closed；嵌入式前端命中该类文件时应退回非前端交付形态，并由浏览器 smoke 证明真实 release frontend 是否可用。
- 其它客户端交付形态可以存在，但成熟度应明确。
- 首个公开 tag 不发布 Linux GTK3/WebKitGTK 4.x native artifacts；Linux 用户使用 Web / Server / Docker 交付面。
- candidate 阶段任何 native build、signing、target-host、manifest 或 attestation 失败都发生在公开 tag 前，不得发布 GHCR 或 GitHub Release。candidate/aggregate artifact immutable，失败后必须 fresh dispatch，禁止覆盖同一 run ID。promotion 阶段仍不是跨服务强事务；若 GHCR 已 push 而后续步骤失败，Release 必须保持 draft并把状态明确报告为 partial delivery；只允许同一 sealed candidate 做幂等恢复。
- candidate / target-host 只要会触发 `native-packaging` compile-time context，就必须先用同一 HEAD 构建真实 `apps/web/dist`；不得依赖 runner 残留、空目录或占位前端绕过 `frontendDist` 校验。
- candidate、普通 CI、Docker builder 与 native target-host jobs 必须使用根 `rust-toolchain.toml` 规定的精确 Rust 1.97.0；浮动 `stable`、minor-only pin 或与 Cargo MSRV 不一致的 job 必须在构建前失败。
- Windows packaged UI gate 必须驱动已安装 Desktop 的真实 WebView，覆盖 native session、创建/编辑、commit/history、Settings focus trap 与关闭后 sidecar 清理；快速 marker startup probe 仍保留，但不能单独证明 UI 可用。
- Windows LocalBackend、NoteGit 与安装/卸载 smoke 分别覆盖 MSI 和 NSIS；RemoteBrowser/native recovery 在共享 Desktop runtime payload 上由 NSIS 安装面执行一次，其证据不扩张为 MSI installer-engine 的 RemoteBrowser 声明。
- Native build、manifest、draft upload 或 API 复核失败时不得把不完整状态报告为已发布版本；失败产生的 draft 只作为显式恢复对象。若公开 mutation 已成功但 runner 未能确认，同一 candidate 的重跑必须先复核完整远端状态。
- 同一 tag 的 workflow rerun 只允许复用 byte-identical draft、image-ID 相同的 immutable version tag，或 tag、完整资产名称/摘要、prerelease 分类与 registry identity 都精确匹配的已公开 Release。任何 remote probe 只有明确 HTTP 404 才表示不存在；网络、认证、限流和 5xx 均 fail-closed，且 registry version 指向其它 image identity 时禁止覆盖。

### 2. 版本与升级预期

- 用户应能知道当前运行的大致版本或构建来源。
- 当前 `/api/node/role` 与 Web dashboard 应暴露只读运行摘要，包括版本、profile、环境、交付形态和 repo health 聚合状态。
- `/api/node/role.watcher_health` 只暴露 workspace ingestion 的 status/expected/running/unavailable aggregate；不得泄漏 repo identity、workspace path、generation 或 failure detail。
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
- RemoteBrowser evidence 必须证明远端页面零 native IPC、零 `ipc.localhost` 请求且服务器 CSP 未放宽；Desktop 只能由 native-owned 菜单/托盘切回 LocalBackend。
- Android writable evidence 只接受 API 29+、当前 WebView provider 137+ 且真实 non-extractable Ed25519 probe 通过的 target；其它 target 只作为只读负向证据。
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
  Remote Projection / Remote Import 当前格式必须能在 `docs/registry/first-tag-format-matrix.md`
  中查到，并由 release baseline 钉住对应 plan 与代码常量；未登记的格式变更不能声明 tag-ready。
- first-tag 批准目标为 ledger entry format v3 / `DEVELDG3`、Redb schema v4、WS binary namespace
  `DEVEWSF4` 且 lockstep `4..=4`、immutable Remote Import。Redb v4、B4 Remote Import 与 C1′ F4/v4 cutover 已对齐；
  B5 typed review UI、B6 fresh receipts 及后续 watcher/release gates 仍阻塞 tag，不存在旧格式兼容 epoch。
- Redb v2 只保留 `--allow-legacy-v2` 离线只读导出；v3 开发 DB 必须用旧 HEAD 导出后重建。WS v1/v2/v3、无版本 JSON、旧 CommandId 与旧 pull 不提供 adapter。
- first-tag 验收使用 `docs/registry/acceptance-matrix.tsv`：普通 CI 验证 case/flow/journey/evidence locator 结构，tag-ready 再验证 clean current-HEAD 与 30 天内 target-host receipts。生成的 `docs/acceptance-matrix.md` 只用于阅读。
- 矩阵允许诚实显示 PVR、候选交付面 receipts、版本/CHANGELOG/release-set freeze 与 Android signing target-host evidence 等 blocker；SBOM/checksum/provenance 只能由 exact candidate producer receipt 关闭，不能改成 source-ref。这些 gap 不阻止普通开发提交，但必须阻止正式 tag。
- Receipt 同时绑定 evidence locator、surface/mode、target OS 与命令前后 clean HEAD；平台 producer/聚合尚未闭环时，tag workflow 必须在任何公开发布前明确失败。producer 超时清理必须只终止经隔离校验的 child process group，不能误伤 CI runner 或宿主父进程组；主动脱离该 group 的宿主资源必须预先登记 ownership 并由显式 finally step 回收，Windows tree cleanup 未验证成功时也必须 fail-closed。
- 普通 CI 必须实际执行 producer registry 中全部适用的 required test/script evidence，不能只打印 plan；候选聚合 workflow 必须验证显式 source run 与当前 HEAD 相同，Rust collector/tag-ready 通过后，tag workflow 才能进入任何公开发布步骤。
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
