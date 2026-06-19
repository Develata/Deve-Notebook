# 18_release.md - 发布与运维 (Release & Ops)

## Metadata

- `Layer`: `Peripheral / Deferred`
- `Status`: `Reference`
- `Version`: `0.0.1`
- `Last Review`: `2026-06-20`
- `Counterpart Feature`: `docs/features/15_release.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/12_tech_release.md`
- `Primary Code Areas`: `.github/workflows/`, `Dockerfile`, `scripts/`

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

### 1.2 Release Channels (发布通道)
1.  **Stable (稳定版)**: tag `v1.0.0`，仅在 Milestone 完成且测试通过后发布；产物包括二进制与 Docker Image (`latest`, `v1.0.0`)；适用于生产环境。
2.  **Pre-release / Experimental (预发布 / 实验构建)**: tag `vX.Y.Z-rc.N` 或人工测试构建标识；按里程碑需要手动触发或本地构建；发布基线不要求独立 `nightly.yml` 工作流。

## 2. CI/CD Pipelines (自动化流程)

CI/CD 基于 GitHub Actions。

> [!NOTE]
> 发布基线只要求 `.github/workflows/release.yml`。`nightly.yml` 与 `speckit-sync-check.yml` 不属于权威 release / CI 要求，不构成总蓝图 drift。

### 2.1 Workflow: `release.yml`
*   **Trigger**: Push to tag `v*` (e.g., `v1.2.3`).
*   **Steps**:
    1.  **Quality Gates**: `cargo clippy --locked --all-targets -- -D warnings`, `scripts/plan-coverage.sh --write-report`, `scripts/check-architecture-registry.sh`, native/graph boundary scripts, `cargo test --locked`.
    2.  **Docker Build**: Dockerfile frontend stage 先运行 `npm run build` 产出 editor assets，再运行 `trunk build --release` 产出 Leptos/WASM。
    3.  **Embed Frontend**: Dockerfile backend stage 在 `cargo build --release --package deve_cli` 前复制 `apps/web/dist`，使 CLI build script 将前端静态资源嵌入二进制。
    4.  **Docker Push**: 使用 GitHub Actions 自动构建并发布容器镜像。
        *   **Registry**: GHCR (`ghcr.io`).
        *   **Platforms**: 发布基线为 `linux/amd64`；`linux/arm64` 需要独立验证后再加入。
        *   **Tags**: `latest`, `v1.2.3` (与 Release Tag 同步).

Native Tauri bundling、OS signing 与 GitHub Release binary upload 属于后续 delivery work；在对应 workflow 增加前，**MUST NOT** 被视为 `release.yml` 发布基线。

Native 双模式属于运行时能力门禁，不属于签名/store/physical-device release readiness。发布或 target-host 证据可以声明：

- LocalBackend 本地后端 smoke 通过。
- RemoteBrowser HTTPS origin 壳层 smoke 通过。

但不得在未完成签名、store、physical-device 与长期后台同步验收前声明 Desktop/Mobile release ready。

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
> 首个 stable 的持久化基线包含 `LEDGER_ENTRY_FORMAT_VERSION = 1` 与 `REDB_SCHEMA_VERSION = 1`。pre-1.0 未发布开发期产生的无版本 ledger entry 或无 schema gate `.redb` 可以 fail-closed 并要求显式 reset / repair / migration，不进入 stable 兼容承诺。

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
`deve init --repo <name> --projection-base <path>` 或 `deve repo projection set --repo <selector> --base <path>`
写入 host-local Projection Locator；实际 workspace root 为 `<path>/<repo_name>/`。例如 `--projection-base /notes --repo default` 对应 `/notes/default/`。

### 5.3 Build Strategy
*   **Base Image**: `debian:bookworm-slim` 或 `gcr.io/distroless/cc-debian12` (Runtime).
*   **Builder**: `rust:1.92-bookworm` (Multi-stage build)，包含 Node.js、固定版本的 Cargo-installed tools（当前为 `trunk`）与 `wasm32-unknown-unknown` target。
*   **Optimization**: Docker 发布基线 **MUST** 使用 locked direct release build；依赖缓存层属于可选构建优化，只有在 locked CI 与 Docker smoke 通过后才可进入发布基线。
*   **Frontend Delivery**: runtime image 只交付单个嵌入前端静态资源的 `deve_cli` 二进制；正常 Docker 部署 **MUST NOT** 依赖 `/app/static` 或 `DEVE_STATIC_DIR`。
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
- [ ] 多平台 (Win/Mac/Linux) 冒烟测试通过。
