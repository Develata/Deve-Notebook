# 18_release.md - 发布与运维 (Release & Ops)

## Metadata

- `Layer`: `Peripheral / Deferred`
- `Status`: `Reference`
- `Version`: `0.0.1`
- `Last Review`: `2026-07-09`
- `Counterpart Feature`: `docs/features/15_release.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/12_tech_release.md`
- `Primary Code Areas`: `.github/workflows/`, `Dockerfile`, `scripts/`, `tools/baseline`

本章定义发布策略、版本规范与 CI/CD。

## 1. Distribution Strategy (分发策略)

分发渠道：

### 1.1 Support Matrix (支持矩阵)
| Platform    | Artifact Format             | Architecture         | Signing                  |
| :---------- | :-------------------------- | :------------------- | :----------------------- |
| **Windows** | `.msi`, `.nsis` (Setup)     | x64, ARM64           | EV Cert (Optional)       |
| **macOS**   | `.dmg`, `.app`              | Apple Silicon, Intel | **Required** (Notarized) |
| **Linux**   | `.deb`, `.rpm`, `.AppImage` | x64                  | GPG                      |
| **Server**  | OCI Image (Docker/Podman)   | x64, ARM64           | GHCR                     |
| **iOS**     | `.ipa` (App Store)          | ARM64                | **Pending** (Not urgent) |
| **Android** | `.apk` / `.aab`             | ARM64                | **Pending** (Not urgent) |
| **Web**     | PWA (Static)                | Universal            | HTTPS                    |

First formal tag scope note: Linux native Desktop artifacts (`.deb`, `.rpm`,
`.AppImage`) are deferred until the native shell stack can move off the current
GTK3/WebKitGTK 4.x dependency line. The tracked TODO is to upgrade or replace
the Tauri/Wry Linux shell route with a maintained GTK4/WebKitGTK 6-compatible
stack, then refresh target-host package/startup evidence before re-enabling
Linux native artifacts in a public release. Until that TODO is closed, Linux
users are expected to use Web / Server / Docker delivery rather than a Linux
native Desktop package.

### 1.2 Release Channels (发布通道)
1.  **Public Preview (公开预览)**: tag `v0.y.z`，用于 pre-1.0 阶段的首批公开验证；必须通过当前 release gate，但不得声明 stable data compatibility、签名 native release、store readiness 或 physical-device readiness。
2.  **Stable (稳定版)**: tag `v1.0.0`，仅在 Milestone 完成且测试通过后发布；产物包括二进制与 Docker Image (`latest`, `v1.0.0`)；适用于生产环境。
3.  **Pre-release / Experimental (预发布 / 实验构建)**: tag `vX.Y.Z-rc.N` 或人工测试构建标识；按里程碑需要手动触发或本地构建；发布基线不要求独立 `nightly.yml` 工作流。

## 2. CI/CD Pipelines (自动化流程)

CI/CD 基于 GitHub Actions。

> [!NOTE]
> 发布基线只要求 `.github/workflows/release.yml`。`nightly.yml` 与 `speckit-sync-check.yml` 不属于权威 release / CI 要求，不构成总蓝图 drift。

### 2.1 Workflow: `release.yml`
*   **Trigger**: Push to tag `v*` (e.g., `v1.2.3`).
*   **Steps**:
    1.  **Quality Gates**: `cargo clippy --locked --all-targets -- -D warnings`, `scripts/plan-coverage.sh --write-report`, `scripts/check-architecture-registry.sh`, native boundary checks that do not build Linux GTK3 artifacts, graph baseline, and `cargo test --locked`. The native process adapter gate is scoped with `DEVE_NATIVE_PROCESS_ADAPTER_RUN_NATIVE_PACKAGING_TESTS=0` in `release.yml`, so it verifies no-Tauri/process authority boundaries without compiling native-packaging dependencies.
        Dependency audit belongs to this gate: `scripts/check-release-audit-gate.sh`
        **MUST** fail on cargo/npm vulnerabilities and **MUST** compare every
        non-vulnerability `cargo audit` warning with
        `docs/registry/release-audit-warning-registry.md`. Any unregistered,
        stale, or field-incomplete warning is release-gate drift. The registry
        row must include the advisory identifier (or synthetic `YANKED` for
        cargo-audit yanked warnings without a RustSec advisory id), crate,
        warning kind, rationale, replacement route, and whether first-tag
        readiness requires a separate USER decision or replacement before
        public tag.
    2.  **Docker Build**: Dockerfile frontend stage 先运行 `npm run build` 产出 editor assets，再运行 `trunk build --release` 产出 Leptos/WASM。
    3.  **Embed Frontend**: Dockerfile backend stage 在 `cargo build --release --package deve_cli` 前复制 `apps/web/dist`，使 CLI build script 将前端静态资源嵌入二进制。
    4.  **Docker Push**: 使用 GitHub Actions 自动构建并发布容器镜像。
        *   **Registry**: GHCR (`ghcr.io`).
        *   **Platforms**: 发布基线为 `linux/amd64`；`linux/arm64` 需要独立验证后再加入。
        *   **Tags**: `latest`, `v1.2.3` (与 Release Tag 同步).

Native Tauri bundling、OS signing 与 GitHub Release binary upload 属于后续 delivery work；在对应 workflow 增加前，**MUST NOT** 被视为 `release.yml` 发布基线。

First-tag `release.yml` deliberately does **not** run Linux GTK3/WebKitGTK 4.x
native packaging, installer, signing, Android/iOS package-build or physical-device
smoke gates. Those remain target-host / workflow-dispatch evidence surfaces, not
tag-triggered Docker release requirements.

Linux native Desktop bundling has an additional first-tag TODO: it **MUST NOT**
ship a Linux GTK3/WebKitGTK 4.x native artifact for the first formal tag. Before
Linux native artifacts can be restored to the release set, the project must
either adopt a maintained Tauri/Wry GTK4/WebKitGTK 6 route or replace the Linux
native shell route with an equivalent maintained WebView stack, and then rerun
release audit, package build, startup, and native-session smoke evidence on the
Linux target host.

Desktop startup / native-session smoke MAY accept `DEVE_DESKTOP_PACKAGE_BUNDLES=exe`
as a target-host release-binary-only probe after `target/release/deve_desktop.exe`
and its sibling `deve_cli.exe` have been built. This selector MUST NOT be
accepted by package-build or installer smoke gates, and MUST NOT be used as
evidence for MSI/NSIS package readiness, install/uninstall readiness, signing
readiness, store readiness, or physical-device readiness.

Native 双模式属于运行时能力门禁，不属于签名/store/physical-device release readiness。发布或 target-host 证据可以声明：

- LocalBackend 本地后端 smoke 通过。
- RemoteBrowser HTTPS origin 壳层 smoke 通过。

但不得在未完成签名、store、physical-device 与长期后台同步验收前声明 Desktop/Mobile release ready。

### 2.1.1 Developer Baseline Checkers {#developer-baseline-checkers}

发布与验收基线可以提供 Rust developer CLI mirror，用于替代对 host bash/awk/rg runtime 敏感的纯文本合同检查。该入口由独立 workspace tool crate `tools/baseline`（package `deve_baseline`）承载，属于 developer/release tooling，不属于普通用户 `deve` 命令面；它 **MUST NOT** 依赖 `deve_cli` 产品 runtime，默认也 **MUST NOT** 依赖 `deve_core`，更 **MUST NOT** 获得 ledger、projection、source-control 或 native authority 写权限。

Rust baseline checker 的默认聚合入口 `cargo run -p deve_baseline -- all` 只承载确定性的仓库文件检查：固定字符串存在/缺失、顺序检查、验收 case block 绑定、协议/文档常量钉扎，以及 `Cargo.lock` tracked / not ignored 这类轻量 git baseline。对于历史上已经作为 baseline shell 存在的确定性 `cargo test` 调度脚本，Rust checker MAY 提供 `cargo_test` TSV operation，并由 `cargo run -p deve_baseline -- full` 显式执行；该入口仍属于 developer/release tooling，不得启动产品 server、Docker/native packaging、平台 smoke、外部工具安装或 network runtime。Docker/native packaging、平台 smoke、外部工具安装与 network runtime 检查在未被显式建模前仍由 shell 脚本或 CI job 承担。Rust mirror 也 MAY 承载验收用例中已有的确定性边界检查脚本（例如结构化 WS 错误、browser prefs 边界、source-control smoke hygiene），前提是检查内容仍能表达为仓库文件合同而非运行时 smoke。Rust mirror 与 shell script 并存期间，确定性规格的唯一维护位置是 Rust checker 的 TSV spec；同名 shell 脚本只能作为兼容入口转发到 Rust checker，并输出相同风格的 fail-closed 诊断（`<name>-baseline-check: ...` 或既有脚本标签），避免 Windows/WSL bash runtime 不可用时失去本地验收入口，也避免长期双份规格漂移。

### 2.1.2 Validation Script Ownership

test / check / smoke 脚本的收敛目标是“验证逻辑尽可能由 Rust/CLI 拥有”，不是机械删除所有 shell 文件。新增或迁移脚本时必须先分类：

1.  **必须 Rust 化**：固定文本/文件合同检查、acceptance binding、registry、路径漂移、结构化错误、边界守卫、env 参数合法性、target 列表与 fail-closed 前置条件。这类检查 SHOULD 进入 `tools/baseline`；同名 shell 只能作为兼容 wrapper 调用 `run_deve_baseline`。
2.  **优先 Rust 化但允许 shell 编排**：`check-*-preflight.sh`、`check-local-quick-gate.sh`、`check-deep-audit-gate.sh`、`check-release-audit-gate.sh` 这类聚合或 preflight 入口。Rust/CLI SHOULD 拥有分类、参数校验、诊断格式与 fail-closed 判断；shell MAY 保留外部命令串联、CI glue 与宿主工具调用。
3.  **暂不强行纯 Rust**：Docker smoke、runtime server/browser smoke、adb/xcrun/installer/native package build、GitHub workflow dispatch 与 artifact collect。此类脚本 MAY 增加 Rust/CLI 前置校验或报告规范，但真实平台动作仍可由 shell/CI 编排；它们 MUST NOT 被并入 `deve_baseline -- all` 的轻量确定性聚合。

任何迁移不得形成双份长期规格：确定性规则的唯一维护位置应是 Rust checker / TSV spec；shell wrapper 不得复制同一批固定字符串、路径漂移或边界判定。

### 2.1.3 Workflow: `check.yml`

普通 branch push / pull request 可以运行一个 **check-only** CI workflow，用于在 tag release 前尽早发现格式、Rust 编译、WASM 编译、测试与文档合同漂移。该 workflow 不属于发布通道，也不得替代 `release.yml` 的 tag-triggered 发布基线。

`check.yml` **MUST** 保持以下边界：

- Trigger 仅限 branch push / pull request / 可选手动诊断；不得监听 `v*` tag。
- Permissions 只允许 `contents: read`；不得声明 `packages: write`。
- 不得登录 GHCR、不得执行 Docker build/push、不得 upload release artifact。
- 不得运行 native package build、installer smoke、store distribution、physical-device 或 production deploy。
- MAY 运行 `cargo fmt --check`、`cargo clippy --locked --all-targets -- -D warnings`、`cargo check --locked -p deve_web --target wasm32-unknown-unknown`、`cargo test --locked`、`cargo run -p deve_baseline -- all` 与 plan coverage enforcing checks。

### 2.2 Deferred Workflows (推迟的工作流)

以下 workflow 不属于权威 release / CI 基线：

- `nightly.yml`: 不再要求每日构建；如未来重新需要，应先更新本章再新增 workflow。
- `speckit-sync-check.yml`: 不再作为 release / CI 的必需校验面；规格同步检查应由后续独立治理流程重新定义。

### 2.3 Security & Signing (安全签名)
*   **macOS**: 必须配置 `APPLE_SIGNING_IDENTITY` 和 `APPLE_PROVIDER_SHORT_NAME` 以通过 Gatekeeper。
*   **Update**: 使用 Tauri Updater 机制，公钥 (`pubkey.pem`) 硬编码在客户端，私钥仅在 CI Secret 中。
*   **Container**: 镜像使用 GitHub Actor 签名 (Keyless signing with Sigstore/Cosign optional).

## 3. Versioning (版本规范)

遵循 **Semantic Versioning 2.0.0** (`MAJOR.MINOR.PATCH`).

*   **MAJOR**: 做了不兼容的 API 修改 (e.g., Ledger 数据结构变更).
*   **MINOR**: 做了向下兼容的功能性新增 (e.g., 新增 UI 插件槽).
*   **PATCH**: 做了向下兼容的问题修正 (e.g., 修复渲染 Bug).

> [!IMPORTANT]
> **Data Compatibility**: 首个 stable 发布后，任何涉及 `Ledger` 或 Projection Workspace / Locator 存储结构的变更，**MUST** 提供迁移路径。首选 "Copy & Rebuild" 策略（见 03_storage/）；仅当无法重建时才提供增量迁移脚本，并在 Major 版本中发布。pre-1.0 阶段允许一次性不兼容重置，但必须更新 plan 与 release notes。
>
> 首个 stable 的持久化基线包含 `LEDGER_ENTRY_FORMAT_VERSION = 2` 与 `REDB_SCHEMA_VERSION = 2`，二者均使用 project-owned postcard codec payload。Projection Backup 不引入 ledger pack plaintext 格式；其 locator/transport 形态由 first-tag format matrix 钉住。pre-1.0 未发布开发期产生的无版本 ledger entry、旧 codec ledger entry 或旧 schema gate `.redb` 可以 fail-closed 并要求显式 reset / repair / migration，不进入 stable 兼容承诺。

## 4. Open Source License (开源协议)

采用 MIT License；再分发副本必须保留版权声明。

## 5. Docker Deployment (容器化部署)

支持通过 OCI 容器在服务器或 NAS 环境中运行 `deve-server`。

### 5.1 Run with Docker CLI
```bash
docker run -d \
  --name deve-server \
  -p 3001:3001 \
  -v $(pwd)/data:/data \
  -v $(pwd)/notes:/notes \
  -e DEVE_LEDGER_DIR=/data/ledger \
  -e AUTH_SECRET=<32-plus-byte-random-secret> \
  -e AUTH_USER=admin \
  -e AUTH_PASS='<argon2-phc-password-hash>' \
  ghcr.io/develata/deve-notebook:latest
```

### 5.2 Run with Docker Compose
```yaml
version: '3.8'
services:
  deve-server:
    image: ghcr.io/develata/deve-notebook:latest
    container_name: deve-server
    restart: always
    ports:
      - "3001:3001"
    volumes:
      - ./data:/data
      - ./notes:/notes
    environment:
      - DEVE_BIND_ADDR=0.0.0.0:3001
      - DEVE_LEDGER_DIR=/data/ledger
      - AUTH_SECRET=${AUTH_SECRET:?set AUTH_SECRET}
      - AUTH_USER=${AUTH_USER:-admin}
      - AUTH_PASS=${AUTH_PASS:?set AUTH_PASS}
```

容器部署 **MUST NOT** 假设 `/data/vault` 是全局投影根。每个本地 repo 的 projection base 必须先通过
`deve init --path <data-root> --repo <name> --projection-base <projection-base>` 或 `deve repo projection set --repo <selector> --base <projection-base>`
写入 host-local Projection Locator；实际 workspace root 为 `<projection-base>/<safe_repo_name>--<repo_id>/`。
例如 `--projection-base /notes --repo default` 对应 `/notes/default--<repo_id>/`。

### 5.3 Build Strategy
*   **Base Image**: `debian:bookworm-slim` 或 `gcr.io/distroless/cc-debian12` (Runtime).
*   **Builder**: `rust:1.92-bookworm` (Multi-stage build)，包含 Node.js、固定版本的 Cargo-installed tools（当前为 `trunk`）与 `wasm32-unknown-unknown` target。
*   **Optimization**: Docker 发布基线 **MUST** 使用 locked direct release build；依赖缓存层属于可选构建优化，只有在 locked CI 与 Docker smoke 通过后才可进入发布基线。
*   **Frontend Delivery**: runtime image 只交付单个嵌入前端静态资源的 `deve_cli` 二进制；正常 Docker 部署 **MUST NOT** 依赖 `/app/static` 或 `DEVE_STATIC_DIR`。嵌入或显式静态根的 `index.html` **MUST NOT** 包含 Trunk development live-reload 标记；显式 `DEVE_STATIC_DIR` 命中该类 index 时 fail-closed，嵌入式前端命中该类 index 时不得被报告或服务为 `embedded-frontend`，发布 smoke 不能只依赖 `/api/node/role` 的 `api-only`，还必须用浏览器入口证明 release frontend 可用。
*   **Local Smoke Diagnostics**: `scripts/smoke-docker-release.sh` **MUST** 支持 `DEVE_DOCKER_BIN` 以覆盖非默认 Docker CLI 路径，并在 Docker 缺失或不可达时输出 Docker binary/context 诊断。

### 5.4 Runtime Observability {#runtime-observability}

公开 `/api/node/role` endpoint 是面向 smoke test 与运维的轻量 release/runtime shape
观测面。它 **MUST** 暴露 version、profile、delivery shape、environment、ports 与聚合 repo
health counts。degraded repo 的细节仍只属于 CLI/admin diagnostics；公开 endpoint 只能返回
聚合计数，以便运维发现 degraded startup，同时避免泄漏 repo name 或 corruption detail。

### 5.5 Docker P2P Mesh Smoke

发布前的本地 Docker mesh smoke **MAY** 使用独立 compose override 启动两个 `deve-server`
实例。该 smoke 必须使用隔离 volume、显式共享 `RepoId`、静态 peer 配置与 env token，
并验证：

- 两个服务端各自拥有独立 local ledger。
- A 的本地写入只进入 B 的 A-shadow。
- B 的 local branch 在显式 merge 前不被污染。
- 断线重连后重新 `SyncHello` 并按当前 vector 对齐。

该 smoke 只证明 server-to-server mesh runtime；不等价于 native release、store readiness、
公网 discovery 或 NAT traversal readiness。

## 6. Checklist for Release (发布清单)

发布前 (Pre-flight Check) 必须确认：

- [ ] 所有 CI 测试通过 (Green).
- [ ] `CHANGELOG.md` 已更新。
- [ ] 关键依赖 (Dependencies) 无高危审计漏洞 (`cargo audit`, `npm audit`).
- [ ] 非漏洞依赖 warning 均有 registry allowlist 理由或替换路线；首个正式 tag 前
      `tag_blocker=yes` 项已被 USER 决策、替换或重新归类。
- [ ] Remote Projection S3-compatible credential binding 遵循 ADR 0008 的长期 profile
      contract；CLI 显式 profile slice 可执行，未绑定 / locator-profile 不匹配 / Web
      profile UX 尚未接入的 custom endpoint 必须继续 fail-closed，且不得把默认
      `AWS_*` 环境凭证签给任意 custom host。
- [ ] Linux native Desktop first-tag TODO 已关闭，或首个正式 tag 的 release set
      已明确排除 Linux native Desktop artifacts，并将 GTK4/WebKitGTK 6-compatible
      Tauri/Wry route 或等价 maintained WebView route 记录为后续工作。
- [ ] 多平台 (Win/Mac/Linux) 冒烟测试通过；若 Linux native Desktop artifact 被
      首个正式 tag 排除，本项的 Linux native package/startup 部分不作为该 tag
      的 release blocker，但必须保留为后续 evidence gap。
