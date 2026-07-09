# Changelog

本文件遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/) 格式，
版本号遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/)。

## [Unreleased]

### Added
- **Projection Backup / S3-compatible**: CLI-only explicit Remote Projection profile slice for `s3+https://` custom endpoints, backed by host-local secret-free profile metadata and env-prefix credential refs; unbound or mismatched custom endpoints still fail closed before provider I/O and ambient AWS credential fallback.

### Fixed
- **Native package version alignment**: Desktop and Mobile Tauri manifests now match the workspace `0.1.0` version used by first-tag artifacts.

## [0.1.0] - 2026-06-23

### Added
- **核心引擎**: Redb 追加日志存储、ledger-first authority、repo-scoped projection workspace
- **Leptos 前端**: CodeMirror 6 编辑器、Markdown 预览、KaTeX 数学公式与基础双语 UI
- **Source Control**: 类 Git 工作流（Watcher → pending → Stage → Commit）与 fail-closed scope gate
- **Docker 部署**: 多阶段 Dockerfile（rust:1.92 + trunk + esbuild → debian:bookworm-slim）
- **Docker Compose**: 单服务配置，512m 内存限制，命名卷持久化
- **CI/CD**: GitHub Actions release.yml（tag 触发）与 native release workflow
- **静态文件服务**: Axum ServeDir 模块，SPA fallback 支持
- **安全审计**: cargo-audit + npm audit 集成到 release gate
- **GHCR 推送**: Docker 镜像自动推送到 ghcr.io
- **Baseline Rust CLI**: release/local/deep gate、acceptance binding、env/target/preflight 等确定性检查迁入 `deve_baseline`
- **Docker smoke preflight**: release、multiclient、P2P mesh smoke 在真实 Docker 动作前统一执行 Rust CLI 参数和环境校验

### Changed
- **Rust 1.92**: release workflow 使用固定 Rust 1.92 toolchain 与 Edition 2024
- **Runtime smoke**: runtime happy/recovery smoke 复用 baseline wrapper 的 cargo 解析逻辑，提高 WSL/Git Bash/Windows 入口稳定性

### Removed
- **MCP runtime**: 移除产品内 MCP manager、client executor、plugin host tool 入口；后续工具扩展走 Skills + controlled CLI / Trusted CLI path。
