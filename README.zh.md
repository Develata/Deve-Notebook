[English](README.md) | 中文

# Deve Notebook

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

Deve Notebook 是一个 Rust workspace，用于构建自托管的个人 Markdown
笔记系统。它采用 ledger-first 存储模型：ledger 是权威源，用户可见的
Markdown workspace 是 repo-scoped projection。

本仓库仍处于活跃开发阶段。当前更准确的定位是：已有大量 runtime 路径和回归证据的
工程原型，而不是已经打磨完成的终端用户发行版。

## 当前状态

已经实现并有测试或 smoke 证据覆盖：

- Rust workspace：`deve_core`、`deve_cli`、`deve_web`、`deve_desktop`、
  `deve_mobile`。
- 基于 Clap、Tokio、Axum、HTTP、WebSocket 的 CLI/server runtime。
- Leptos CSR Web 前端：登录/会话、文档操作、命令入口、Source Control UI、
  merge/conflict 流程、只读 graph 视图、settings surface、i18n。
- ledger-backed local repo state、repo-scoped projection workspace、watcher-to-pending 外部编辑摄入、
  stage/commit/discard/merge 工作流，以及 projection health 诊断。
- repo-scoped sync protocol，包含浏览器 WebLightPeer identity、scope nonce gate、
  structured protocol error 与 recovery path。
- 生产认证 fail-closed；`--dev` 模式才提供本地 `admin` / `admin` 登录。
- Dockerfile、生产 `docker-compose.yml`、Web release build smoke、runtime smoke、
  acceptance/baseline guard 与 architecture registry check。
- Desktop/Mobile native shell crate，Tauri v2 仅在可选 `native-packaging` feature 后启用。
- 最新 target-host evidence 覆盖 Windows Desktop no-sign MSI/NSIS package
  build/startup/installer smoke、Android shell APK emulator smoke、iOS shell simulator smoke。

未实现或不声明为当前能力：

- 不是 hosted multi-tenant SaaS。
- 浏览器不是 offline-first full local ledger；浏览器是 WebLightPeer，authority 依赖 server。
- 没有 server-backed Settings API；当前是文件/config/runtime surface。
- 没有默认全文索引；Tantivy 是 optional feature-gated 路线。
- 没有高性能 graph renderer；当前 graph 是只读 projection 和 summary/review UI。
- 没有产品内 MCP runtime。MCP 只保留为历史说明或开发验收工具语境。
- 没有通用 plugin marketplace，也不声明任意插件 authority；当前是 Rhai/plugin 兼容边界和显式 capability gate。
- 没有默认 trusted external agent 执行。Native AI chat 已存在，trusted CLI bridge 必须显式启用且默认关闭。
- 没有 Web Git writer，也不把 Git 当 authority；Git 只是 Deve Source Control 外侧的 mirror/import/export/publish bridge。
- 不声明 signing、store readiness、physical-device readiness、native authority writes、
  Mobile process runtime 或 Android process runtime。

## 权威模型

```text
Ledger -> Folded State -> Projection -> Projection Workspace
```

- `ledger/` 保存权威 repo facts。
- `ledger/.host/projection-locators.toml` 保存 host-local
  `RepoId -> projection_base` 绑定。
- `<projection_base>/<safe_repo_name>--<repo_id>/` 保存单个本地 repo 的用户可见
  Markdown projection；repo 名称只是显示别名，`RepoId` 才是权威身份。
- 文件系统变化先进入 `pending_fs_ops`；只有显式 stage/commit 才会追加 ledger facts。
- `.notegit/` 是 Deve 拥有的 repo runtime state。
- `.git/` 只是 Git ecosystem mirror bridge。

`docs/plan/` 是权威设计来源。`docs/features/` 和 `docs/acceptance-cases/`
细化行为与验收。`docs/report/` 是带日期的历史证据，不是实时契约。

## 仓库结构

| 路径 | 作用 |
| --- | --- |
| `crates/core` | ledger、projection、sync、source control、security、config、plugin boundary |
| `apps/cli` | CLI 命令与 Axum/Tokio HTTP + WebSocket server |
| `apps/web` | Leptos CSR 浏览器前端 |
| `apps/desktop` | Desktop native shell 与 Tauri packaging gate |
| `apps/mobile` | Mobile native shell 与 Android/iOS packaging gate |
| `docs/plan` | 权威工程蓝图 |
| `docs/features` | 用户可见 feature 与 operation 规格 |
| `docs/acceptance-cases` | 验收与回归用例 registry |
| `docs/overview` | 架构图与 drift registry |
| `docs/report` | 历史报告与 smoke evidence |
| `scripts` | 构建、smoke、target-host 与 boundary check |

## 前置条件

主开发路径需要：

- 兼容 Edition 2024 的 Rust toolchain。
- Node.js 与 npm。
- 用于 WebAssembly 前端的 Trunk。
- Git。
- 能执行 `scripts/*.sh` 的 POSIX-like shell；Windows 通常使用 Git Bash。

可选路径：

- Docker / Docker Compose，用于 container smoke。
- Tauri CLI 和平台 packaging 工具，用于 Desktop/Mobile target-host check。
- Android Studio / Android SDK，用于 Android emulator/package check。

## 快速开始

```bash
git clone https://github.com/develeta/deve-note.git
cd deve-note
scripts/smoke-web-release-build.sh
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

做 UI 迭代时，可以分开运行后端和 Trunk：

```bash
cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001
cd apps/web
NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080
```

然后打开 `http://127.0.0.1:8080/`。

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
| `git status/export/import/push` | 操作 Git mirror bridge |
| `config print/set` | 查看或更新白名单配置项 |

## 验证

Targeted Rust test：

```bash
cargo test --package <pkg> --lib <test_fn> -- --nocapture
```

通用检查：

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

常用脚本 gate：

```bash
bash scripts/check-foundation-baseline.sh
bash scripts/check-network-baseline.sh
bash scripts/check-source-control-baseline.sh
bash scripts/check-native-track-boundary.sh
bash scripts/check-release-baseline.sh
```

Docker smoke 只在具备 Docker 的主机上启用：

```bash
DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh
```

## 文档

- `docs/plan/deve-note plan.md`：蓝图索引。
- `docs/overview/architecture.md`：架构视图。
- `docs/overview/architecture-diff.md`：当前 plan/code drift registry。
- `docs/features/operation-coverage.md`：operation coverage registry。
- `docs/acceptance-cases/00_index.md`：验收用例索引。
- `docs/dev-runbook.md`：当前启动、诊断与 release runbook。
- `docs/report/README.md`：历史 report 阅读规则。

## License

Deve Notebook 基于 [MIT License](LICENSE) 开源。
