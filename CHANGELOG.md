# Changelog

本文件遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/) 格式，
版本号遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/)。

## [Unreleased]

## [0.1.0] - 2026-07-26

### Added
- **Projection Backup / S3-compatible**: CLI-only explicit Remote Projection profile slice for `s3+https://` custom endpoints, backed by host-local secret-free profile metadata and env-prefix credential refs; unbound or mismatched custom endpoints still fail closed before provider I/O and ambient AWS credential fallback.
- **核心引擎**: Redb 追加日志存储、ledger-first authority、repo-scoped projection workspace
- **Leptos 前端**: CodeMirror 6 编辑器、Markdown 预览、KaTeX 数学公式与基础双语 UI
- **Source Control**: 类 Git 工作流（Watcher → pending → Stage → Commit）与 fail-closed scope gate
- **Docker 部署**: 多阶段 Dockerfile（rust:1.97.0 + trunk + esbuild → debian:bookworm-slim）
- **Docker Compose**: 单服务配置，512m 内存限制，命名卷持久化
- **CI/CD**: GitHub Actions release.yml（tag 触发）与 native release workflow
- **静态文件服务**: Axum ServeDir 模块，SPA fallback 支持
- **安全审计**: cargo-audit + npm audit 集成到 release gate
- **GHCR 推送**: Docker 镜像自动推送到 ghcr.io
- **Baseline Rust CLI**: release/local/deep gate、acceptance binding、env/target/preflight 等确定性检查迁入 `deve_baseline`
- **Docker smoke preflight**: release、multiclient、P2P mesh smoke 在真实 Docker 动作前统一执行 Rust CLI 参数和环境校验

### Changed
- **Rust 1.97.0**: workspace、release workflow、Docker 与 native package gates 使用固定 Rust 1.97.0 toolchain 与 Edition 2024
- **Runtime smoke**: runtime happy/recovery smoke 复用 baseline wrapper 的 cargo 解析逻辑，提高 WSL/Git Bash/Windows 入口稳定性

### Fixed
- **First-tag release orchestration**: `release.yml` is now the sole tag trigger; native delivery runs only after quality and Docker success, validates the exact native artifact manifest in a draft, and publishes one GitHub Release only after remote assets match.
- **Native package version alignment**: Desktop and Mobile Tauri manifests plus the tracked Android direct-Gradle fallback now match the workspace `0.1.0` version used by first-tag artifacts.
- **Docker runtime metrics**: Server dashboard CPU and memory gauges now use the container-visible cgroup hierarchy, with strict cgroup v2/v1/host fallback instead of reporting Docker host or VM totals.

### Removed
- **MCP runtime**: 移除产品内 MCP manager、client executor、plugin host tool 入口；后续工具扩展走 Skills + controlled CLI / Trusted CLI path。

### Known limitations
- **Windows 文件监听事件风暴**：短时间内对 Projection Workspace 执行数千个外部文件变更时，notify 8.2.0 可能丢失该批次的变化通知；极端情况下，后续外部变化也可能在重启前不再出现。
  - 影响：目录树与 External Changes 可能暂时陈旧；若用户在 Deve 内继续修改同一文件，未被摄取的外部内容存在被后续 projection writeback 覆盖的风险。Ledger 既有事实不会因此损坏。
  - 规避：执行数千文件规模的 Git checkout、解压、批量复制、重命名、删除或同步操作后，先重启 Deve，再继续编辑或提交。
  - 退出条件：官方稳定 notify family 提供等价的 Windows overflow rescan/re-watch 修复、workspace 解析为单一 registry dependency identity，且三进程 exact-HEAD Windows receipt 通过后，整体删除本 accepted gap。
