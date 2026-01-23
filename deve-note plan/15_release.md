# 15_release.md - 发布与运维 (Release & Ops)

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
| **Web**     | PWA (Static)                | Universal            | HTTPS                    |

### 1.2 Release Channels (发布通道)
1.  **Stable (稳定版)**:
    *   Tag: `v1.0.0`
    *   频率: 仅当 Milestone 完成且通过所有测试后发布。
    *   Artifacts: Binaries, Docker Image (`latest`, `v1.0.0`).
    *   适用: 生产环境。
2.  **Nightly (每日构建)**:
    *   Tag: `nightly` (Rolling), `nightly-yyyyMMdd`
    *   频率: 每日 `main` 分支构建。
    *   Artifacts: Docker Image (`nightly`), Binaries (Optional).
    *   适用: 尝鲜用户，QA 测试。

## 2. CI/CD Pipelines (自动化流程)

基于 GitHub Actions 实现全自动构建。

> [!NOTE]
> **Status (状态)**: 计划中 (Planned). (CI 工作流尚未在 `.github/workflows` 中实现)。

### 2.1 Workflow: `release.yml`
*   **Trigger**: Push to tag `v*` (e.g., `v1.2.3`).
*   **Steps**:
    1.  **Test**: `cargo test`, `npm run test`.
    2.  **Build Frontend**: `npm run build` (Leptos -> WASM/JS).
    3.  **Build Core**: `cargo build --release` (Native Libs).
    4.  **Bundle (Tauri)**: `npm run tauri build`.
    5.  **Sign**: 调用 macOS Notary / Windows SignTool。
    6.  **Upload**: 上传 Artifacts 至 GitHub Releases。
    7.  **Docker Push**: 使用 GitHub Actions 自动构建并发布容器镜像。
        *   **Registry**: GHCR (`ghcr.io`).
        *   **Platforms**: `linux/amd64`, `linux/arm64`.
        *   **Tags**: `latest`, `v1.2.3` (与 Release Tag 同步).

### 2.2 Workflow: `nightly.yml`
*   **Trigger**: Schedule (Daily 00:00 UTC) or Push to `main`.
*   **Steps**:
    1.  **Build & Test**: 确保代码库健康。
    2.  **Docker Push**: 构建并推送 Nightly 镜像。
        *   **Tags**: `nightly` (Updated daily).

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
> **Data Compatibility**: 任何涉及 `Ledger` 或 `Vault` 存储结构的变更，**MUST** 提供数据迁移脚本 (Migration)，并在 Major 版本中发布。

## 4. Open Source License (开源协议)

作为个人开发者项目，采用最宽松且通用的协议：

*   **MIT License**: 允许任何人免费使用、修改、分发甚至闭源商用，仅需在副本中包含原作者的版权声明。

## 5. Docker Deployment (容器化部署)

支持通过 OCI 容器在服务器或 NAS 环境中运行 `deve-server`。

### 5.1 Run with Docker CLI
```bash
docker run -d \
  --name deve-server \
  -p 3000:3000 \
  -v $(pwd)/data:/data \
  -e DEVE_VAULT_PATH=/data/vault \
  ghcr.io/develata/deve-server:latest
```

### 5.2 Run with Docker Compose
```yaml
version: '3.8'
services:
  deve-server:
    image: ghcr.io/develata/deve-note:latest
    container_name: deve-server
    restart: always
    ports:
      - "3000:3000"
    volumes:
      - ./data:/data
    environment:
      - DEVE_BIND_ADDR=0.0.0.0:3000
      - DEVE_VAULT_PATH=/data/vault
```

### 5.3 Build Strategy
*   **Base Image**: `debian:bookworm-slim` 或 `gcr.io/distroless/cc-debian12` (Runtime).
*   **Builder**: `rust:1.80-bookworm` (Multi-stage build).
*   **Optimization**: 使用 `cargo-chef` 缓存依赖构建层。

## 6. Checklist for Release (发布清单)

发布前 (Pre-flight Check) 必须确认：

- [ ] 所有 CI 测试通过 (Green).
- [ ] `CHANGELOG.md` 已更新。
- [ ] 关键依赖 (Dependencies) 无高危审计漏洞 (`cargo audit`, `npm audit`).
- [ ] 多平台 (Win/Mac/Linux) 冒烟测试通过。

