# 15_release.md - 发布与运维 (Release & Ops)

## Metadata

- `Layer`: `Peripheral / Deferred`
- `Status`: `Reference`
- `Counterpart Feature`: `docs/features/15_release.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/12_tech_release.md`
- `Primary Code Areas`: `.github/workflows/`, `Dockerfile`, `scripts/`

本章定义 `Deve-Note` 的软件发布策略、版本管理规范以及 CI/CD 自动化流程。

## 1. Distribution Strategy (分发策略)

我们采用多渠道分发以覆盖所有目标平台。

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
2.  **Pre-release / Experimental (预发布 / 实验构建)**: tag `vX.Y.Z-rc.N` 或人工测试构建标识；按里程碑需要手动触发或本地构建；当前基线不要求独立 `nightly.yml` 工作流。

## 2. CI/CD Pipelines (自动化流程)

基于 GitHub Actions 实现全自动构建。

> [!NOTE]
> **Status (状态)**: 当前权威基线只要求 `.github/workflows/release.yml`。`nightly.yml` 与 `speckit-sync-check.yml` 已从当前 release / CI 要求中移除，不再构成总蓝图 drift。

### 2.1 Workflow: `release.yml`
*   **Trigger**: Push to tag `v*` (e.g., `v1.2.3`).
*   **Steps**:
    1.  **Quality Gates**: `cargo clippy --all-targets -- -D warnings`, `scripts/plan-coverage.sh --write-report`, `scripts/check-architecture-registry.sh`, `cargo test`.
    2.  **Docker Build**: Dockerfile frontend stage runs `npm run build` for editor assets and `trunk build --release` for Leptos/WASM output.
    3.  **Embed Frontend**: Dockerfile backend stage copies `apps/web/dist` before `cargo build --release --package deve_cli`, so the CLI build script embeds frontend assets into the binary.
    4.  **Docker Push**: 使用 GitHub Actions 自动构建并发布容器镜像。
        *   **Registry**: GHCR (`ghcr.io`).
        *   **Platforms**: 当前 baseline 为 `linux/amd64`；`linux/arm64` 需要独立验证后再加入。
        *   **Tags**: `latest`, `v1.2.3` (与 Release Tag 同步).

Native Tauri bundling, OS signing, and GitHub Release binary uploads are deferred delivery work. They must not be treated as current `release.yml` baseline until the workflow is added.

### 2.2 Deferred Workflows (非当前基线)

以下 workflow 当前不属于权威 release / CI 基线：

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
> **Data Compatibility**: 任何涉及 `Ledger` 或 `Vault` 存储结构的变更，**MUST** 提供迁移路径。首选 "Copy & Rebuild" 策略（见 04_storage.md）；仅当无法重建时才提供增量迁移脚本，并在 Major 版本中发布。

## 4. Open Source License (开源协议)

作为个人开发者项目，采用最宽松且通用的协议：

*   **MIT License**: 允许任何人免费使用、修改、分发甚至闭源商用，仅需在副本中包含原作者的版权声明。

## 5. Docker Deployment (容器化部署)

支持通过 OCI 容器在服务器或 NAS 环境中运行 `deve-server`。

### 5.1 Run with Docker CLI
```bash
docker run -d \
  --name deve-server \
  -p 3001:3001 \
  -v $(pwd)/data:/data \
  -e DEVE_VAULT_PATH=/data/vault \
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
    environment:
      - DEVE_BIND_ADDR=0.0.0.0:3001
      - DEVE_VAULT_PATH=/data/vault
      - AUTH_SECRET=${AUTH_SECRET:?set AUTH_SECRET}
      - AUTH_USER=${AUTH_USER:-admin}
      - AUTH_PASS=${AUTH_PASS:?set AUTH_PASS}
```

### 5.3 Build Strategy
*   **Base Image**: `debian:bookworm-slim` 或 `gcr.io/distroless/cc-debian12` (Runtime).
*   **Builder**: `rust:1.85-bookworm` (Multi-stage build), with Node.js, Trunk, and `wasm32-unknown-unknown`.
*   **Optimization**: 使用 `cargo-chef` 缓存依赖构建层。
*   **Frontend Delivery**: runtime image ships a single `deve_cli` binary with embedded frontend static assets; runtime no longer requires `/app/static` or `DEVE_STATIC_DIR` for normal Docker deployment.

## 6. Checklist for Release (发布清单)

发布前 (Pre-flight Check) 必须确认：

- [ ] 所有 CI 测试通过 (Green).
- [ ] `CHANGELOG.md` 已更新。
- [ ] 关键依赖 (Dependencies) 无高危审计漏洞 (`cargo audit`, `npm audit`).
- [ ] 多平台 (Win/Mac/Linux) 冒烟测试通过。
