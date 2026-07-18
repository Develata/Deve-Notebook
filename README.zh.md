[English](README.md) | 中文

# Deve Notebook

[![Check](https://github.com/Develata/Deve-Notebook/actions/workflows/check.yml/badge.svg)](https://github.com/Develata/Deve-Notebook/actions/workflows/check.yml)
[![Release](https://github.com/Develata/Deve-Notebook/actions/workflows/release.yml/badge.svg)](https://github.com/Develata/Deve-Notebook/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

Deve Notebook 是一个 Rust workspace，用于构建自托管的协作型 Markdown
笔记系统。它面向私有、低资源部署，采用 ledger-first 存储模型：ledger 是权威源，
用户可见的 Markdown workspace 是 repo-scoped projection。

当前 workspace 版本是 `0.1.0`。本仓库适合工程验收、源码审查和 Docker 预览使用；
它还不是已经完整打磨的终端用户应用、hosted SaaS 产品或已签名 native app release。

## 当前可用能力

- Rust workspace：`deve_core`、`deve_cli`、`deve_web`、`deve_desktop`、
  `deve_mobile`，以及 developer checker crate `deve_baseline`。
- 基于 Clap/Tokio/Axum 的 CLI server，提供 HTTP、WebSocket、认证、runtime
  status、admin diagnostics 和 embedded frontend delivery。
- Leptos CSR Web 前端：登录/会话、文档操作、命令入口、Source Control UI、
  merge/conflict 流程、只读 graph 视图、settings surface 与 i18n 覆盖。
- ledger-backed repo state、repo-scoped projection workspace、外部文件 watcher
  摄入、stage/commit/discard/merge 工作流，以及 projection health 诊断。
- repo-scoped sync protocol，包含浏览器 WebLightPeer identity、scope nonce gate、
  structured protocol error 与 recovery path。
- 生产认证 fail-closed；`--dev` 模式才提供本地 `admin` / `admin` 登录。
- Dockerfile、生产 `docker-compose.yml`、embedded Web release build smoke、
  runtime smoke、release/baseline guard 与 architecture registry check。
- Desktop/Mobile native shell crate，Tauri v2 仅在可选 `native-packaging` gate
  后启用。Native shell 默认进入 LocalBackend，也可以在 Settings 中切换到
  已校验的 RemoteBackend HTTPS origin。当前证据是 shell/package/startup 方向，
  不是已签名 store readiness。

## 明确边界

当前 release 不声明：

- hosted multi-tenant SaaS；
- 浏览器 offline-first full local ledger；
- server-backed Settings API；
- 默认全文索引；
- 高性能 graph renderer；
- 产品内 MCP runtime；
- 通用 plugin marketplace 或任意插件 authority；
- 默认 trusted external agent 执行；
- Web Git writer 或 Git authority；
- 已签名桌面安装包、app-store readiness、physical-device readiness、native
  authority writes、Mobile process runtime 或 Android process runtime。

Git 只是 Deve 自身 source-control authority 外侧的 mirror/import/export/publish
bridge。ledger 和 `.notegit/` 仍是 Deve 拥有的 runtime state。

设计上倾向最大限度分离跨宿主机器数据面与 host-local 人类交互面：跨端保留不可变
identity、Ledger facts 与 Markdown 信息完整性；与正确性无关的名称和视觉偏好留在本机。
Repo alias 是这一原则的直接实例：peer 只共享 `RepoId`，永不同步 alias。首发已批准目标允许
用户显式导入/导出 deterministic JSON 映射；在 C1′ runtime 与验收证据落地前，该能力仍处于 blocked。

## 权威模型

```text
Ledger -> Folded State -> Projection -> Projection Workspace
```

- `ledger/` 保存权威 repo facts。
- `ledger/.host/projection-locators.toml` 保存 host-local
  `RepoId -> (projection_base, immutable workspace_segment)` 绑定。
- `<projection_base>/<workspace_segment>/` 保存单个本地 repo 的用户可见
  Markdown projection。
- 已批准的 repo alias 合同把 display state 保持在 host-local；C1′ runtime 落地后，修改或
  导入 alias 不移动 workspace，也不改变 peer identity。
- 文件系统变化先进入 `pending_fs_ops`；只有显式 stage/commit 才会追加 ledger facts。
- `.notegit/` 是 Deve 拥有的 repo runtime state。
- `.git/` 只是 Git ecosystem bridge。

`docs/plan/` 是权威设计来源。`docs/features/` 和 `docs/acceptance-cases/`
细化行为与验收。`docs/report/` 是带日期的历史证据，不是实时契约。

### Remote Projection / Projection Backup

Remote Projection / Projection Backup 只通过 WebDAV/S3 传输 Markdown Projection
Workspace files。它不是 ledger history backup、实时同步、Source Control authority
或 Git mirror authority。

S3-compatible custom endpoint 采用长期 credential-binding 设计：host-local、
secret-free Remote Projection profile 绑定 endpoint origin、bucket、allowed prefix、
signing settings 与 credential ref。access key、secret key、session token 不得写入
repo metadata、locator string、浏览器状态、普通日志或 README 示例。在该 profile runtime
实现并验证前，`s3+https://` custom endpoint I/O 继续 fail-closed，默认 `AWS_*`
环境凭证不得被签给任意 custom host。

## 仓库结构

| 路径 | 作用 |
| --- | --- |
| `crates/core` | ledger、projection、sync、source control、security、config、plugin boundary |
| `apps/cli` | CLI 命令与 Axum/Tokio HTTP + WebSocket server |
| `apps/web` | Leptos CSR 浏览器前端 |
| `apps/desktop` | Desktop native shell 与 Tauri packaging gate |
| `apps/mobile` | Mobile native shell 与 Android/iOS packaging gate |
| `tools/baseline` | Rust developer/release checker CLI |
| `docs/plan` | 权威工程蓝图 |
| `docs/features` | 用户可见 feature 与 operation 规格 |
| `docs/acceptance-cases` | 验收与回归用例 registry |
| `docs/overview` | 架构图与 drift registry |
| `docs/report` | 历史报告与 smoke evidence |
| `scripts` | 构建、smoke、target-host 与 boundary check |

## 前置条件

主开发路径需要：

- Rust 1.97.0（由 `rust-toolchain.toml` 精确钉住），支持 Edition 2024。
- Web 检查需要 `wasm32-unknown-unknown` target。
- Node.js 24 与 npm，用于和 CI 保持一致。
- 用于 WebAssembly 前端的 Trunk。
- Git。
- 能执行 `scripts/*.sh` 的 POSIX-like shell；Windows 通常使用 Git Bash。

可选路径：

- Docker / Docker Compose，用于 container smoke。
- Tauri CLI 和平台 packaging 工具，用于 Desktop/Mobile target-host check。
- Android Studio / Android SDK，用于 Android emulator/package check。
- macOS/Xcode，用于 iOS simulator/package check。

## 快速开始

```bash
git clone https://github.com/Develata/Deve-Notebook.git
cd Deve-Notebook
bash scripts/smoke-web-release-build.sh
cargo run -p deve_cli --bin deve_cli -- init --path . --repo default --projection-base notes
cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001
```

打开：

```text
http://127.0.0.1:3001/
```

开发登录：

```text
username: admin
password: admin
```

做前端迭代时，可以分开运行后端和 Trunk：

```bash
cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001
cd apps/web
NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080
```

然后打开 `http://127.0.0.1:8080/`。

### Desktop Native Packaging

Desktop `native-packaging` 默认进入 `LocalBackend`：Tauri shell 加载 bundled
Web assets，并启动受控 sibling `deve_cli serve --native-loopback` 本地服务。它
不依赖外部已经运行在 `3001` 端口的 server。

本地 debug 运行前，先确保 sibling CLI binary 存在：

```bash
cargo build -p deve_cli --bin deve_cli
cargo run -p deve_desktop --features native-packaging
```

LocalBackend runtime data 默认使用平台 app-private data directory。仅在诊断或
smoke-test 隔离时使用 `DEVE_DESKTOP_DATA_DIR=<absolute-path>`。

如果要把 Desktop 作为远端 HTTPS Web shell 使用，而不是启动本地后端：

```bash
cargo run -p deve_desktop --features native-packaging -- --remote-url https://example.invalid
```

packaged 或脚本化启动也可以使用
`DEVE_NATIVE_REMOTE_URL=https://example.invalid`。RemoteBrowser URL 必须是 HTTPS
origin：不得包含 userinfo、query、fragment 或业务子路径。

Linux native Desktop package 是首个正式 tag 前的 deferred TODO。当前 Tauri v2
Linux stack 仍经过 GTK3/WebKitGTK 4.x 依赖线；首个正式 tag 在 Linux 上应使用
Web / Server / Docker delivery，而不是发布 `.deb`、`.rpm` 或 `.AppImage`
Desktop artifacts。只有在 shell stack 升级/替换为 maintained GTK4/WebKitGTK 6-compatible
Tauri/Wry route 或等价 maintained WebView route，并刷新 Linux package/startup/native-session
evidence 后，才能重新启用 Linux native artifacts。

native app 的 Settings 会显示 Backend section：

- Local Backend 会自动启动 app 自有本地服务。
- Remote Backend 必须填写 HTTPS origin，并由 native 侧探测
  `<origin>/api/node/role` 成功后才能保存。
- 保存的选择是 host-local app-private JSON，不是 `config.toml`、ledger state、
  Projection Locator state 或浏览器 localStorage。
- 远端凭证与登录态仍由远端 Web origin 自己管理。remote 失联时，native 的
  lock/read-only surface 可以切回 Local Backend。

### Mobile Native Packaging

Mobile `native-packaging` 与 Desktop 使用同一套 Backend settings contract。
Local Backend 在 mobile shell 内启动 embedded loopback service，不要求外部
`3001` 端口上已经有 server。Remote Backend 只加载已校验的 HTTPS origin，
不注入本地 session/bootstrap。

Mobile Tauri bundle 在生产 shell 运行中加载 bundled `frontendDist` assets；
后端由 native 启动覆盖项或 host-local Backend preference 决定。

## 配置

从 `config.example.toml` 开始。

重要本地配置：

- `ledger_dir`：本地 ledger/runtime storage。
- `profile`：`standard` 或 `low-spec`。
- `sync_mode`：`auto` 或 `manual`。
- `merge_strategy`：`manual` 或 `auto`。
- `snapshot_depth`：保留的 snapshot 深度。
- `mem_cache_mb`：runtime cache 预算。

不使用 `--dev` 且 `DEVE_ENV` 不是 `development` 时进入生产模式。生产启动需要：

- `AUTH_SECRET`：JWT signing secret，至少 32 字节。
- `AUTH_PASS`：Argon2 PHC password hash。
- `AUTH_USER`：可选用户名，默认 `admin`。

## CLI

```bash
cargo run -p deve_cli --bin deve_cli -- <command>
```

常用命令：

| 命令 | 作用 |
| --- | --- |
| `init --path <path> --repo <name> --projection-base <path>` | 初始化 ledger、repo 与 Projection Locator |
| `repo projection set --repo <selector> --base <path>` | 设置 repo Projection Locator |
| `repo projection list/check` | 查看 repo Projection Locator |
| `scan` | 扫描 repo projection workspace |
| `watch [--dry-run]` | 监听 projection workspace 变化并记录 pending candidates |
| `serve [--dev] [--port <port>]` | 启动 HTTP/WebSocket 后端 |
| `export` | 将 ledger 数据导出为 JSON 或 Markdown |
| `graph` | 输出只读 graph projection |
| `node-check` | 检查 repo/projection 健康状态 |
| `repair --check` | 运行 repair readiness check |
| `sc-status` | 输出 Deve source-control 计数 |
| `ngit status/mirror/export/import/push` | 查看并操作 NoteGit Git main mirror |
| `config print/set` | 查看或更新白名单配置项 |

## Docker

生产 compose 使用已发布镜像：

```bash
docker compose up -d
```

必需环境变量：

```bash
AUTH_SECRET=<32-plus-byte-random-secret>
AUTH_PASS='<argon2-phc-password-hash>'
AUTH_USER=admin
```

本地 Docker release smoke：

```bash
DEVE_DOCKER_SMOKE_REQUIRED=1 bash scripts/smoke-docker-release.sh
```

镜像交付单个 embedded frontend binary，并把 runtime data 放在挂载的 `/data`
和 `/notes`。Projection root 仍通过 Projection Locator 配置；`/notes` 不是全局
authority。

## 验证

branch workflow `.github/workflows/check.yml` 是 check-only：它运行 formatting、
baseline contracts、plan coverage、clippy、WASM check 和 tests；不会发布 package、
push Docker image、upload artifact 或部署 production service。

等价本地检查：

```bash
cargo fmt --check
cargo run --quiet -p deve_baseline -- all
bash scripts/plan-coverage.sh --check-reverse-coverage
bash scripts/plan-coverage.sh --check-metadata-completeness
bash scripts/plan-coverage.sh --check-perf-budget
bash scripts/plan-coverage.sh --check-no-adr-plan-ref
bash scripts/plan-coverage.sh --check-md-links docs/plan docs/features docs/acceptance-cases
bash scripts/plan-coverage-selftest.sh
cargo clippy --locked --all-targets -- -D warnings
cargo check --locked -p deve_web --target wasm32-unknown-unknown
cargo test --locked
```

release 方向检查：

```bash
DEVE_RELEASE_AUDIT_REQUIRED=1 bash scripts/check-release-audit-gate.sh
DEVE_DOCKER_MULTI_REQUIRED=1 bash scripts/smoke-docker-multiclient.sh
DEVE_DOCKER_P2P_MESH_REQUIRED=1 bash scripts/smoke-docker-p2p-mesh.sh
```

完整 release 和 Docker smoke 应在具备对应工具的机器上运行。缺失 Docker、Android、
iOS、签名或 target-host 工具时，应记录为 release evidence gap，而不是削弱检查。

## Release Workflows

- `check.yml`：branch push / pull request check-only。
- `release-candidate.yml`：手动执行 exact-HEAD quality、Docker/native target-host、
  SBOM、checksum 与 attestation candidate sealing。
- `acceptance-aggregate.yml`：验证显式 candidate run 的 receipts/制品并聚合不可变
  tag-ready bundle。
- `release.yml`：tag `v*` 只提升 sealed bytes，不重新构建或打包。
- `release-native.yml`：tag 前 reusable native build/smoke workflow。当前 native
  artifacts 仍是明确的平台证据，不表示 notarization、store 或 physical-device readiness。
- `native-target-host.yml`：manual target-host diagnostics 与 evidence collection。

只有 branch CI 变绿且明确接受 tag-triggered workflow 范围后，才应创建或移动 release tag。

## 文档

- `docs/plan/deve-note plan.md`：蓝图索引。
- `docs/plan/18_release.md`：release 与 CI/CD 契约。
- `docs/overview/architecture.md`：架构视图。
- `docs/overview/architecture-diff.md`：当前 plan/code drift registry。
- `docs/features/operation-coverage.md`：operation coverage registry。
- `docs/acceptance-cases/00_index.md`：验收用例索引。
- `docs/dev-runbook.md`：启动、诊断与 release runbook。
- `docs/report/README.md`：历史 report 阅读规则。

## License

Deve Notebook 基于 [MIT License](LICENSE) 开源。
