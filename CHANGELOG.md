# Changelog

本文件遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/) 格式，
版本号遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/)。

## [Unreleased]

## [0.2.0] - 2026-06-23

### Added
- **Docker 部署**: 多阶段 Dockerfile（rust:1.85 + trunk + esbuild → debian:bookworm-slim）
- **Docker Compose**: 单服务配置，512m 内存限制，命名卷持久化
- **CI/CD**: GitHub Actions release.yml（tag 触发）+ nightly.yml（每日 + main push）
- **静态文件服务**: Axum ServeDir 模块，SPA fallback 支持
- **安全审计**: cargo-audit + npm audit 集成到 nightly CI
- **GHCR 推送**: Docker 镜像自动推送到 ghcr.io
- **Baseline Rust CLI**: release/local/deep gate、acceptance binding、env/target/preflight 等确定性检查迁入 `deve_baseline`
- **Docker smoke preflight**: release、multiclient、P2P mesh smoke 在真实 Docker 动作前统一执行 Rust CLI 参数和环境校验

### Changed
- **Rust 1.85+**: 升级最低要求以支持 Edition 2024
- **Runtime smoke**: runtime happy/recovery smoke 复用 baseline wrapper 的 cargo 解析逻辑，提高 WSL/Git Bash/Windows 入口稳定性

### Removed
- **MCP runtime**: 移除产品内 MCP manager、client executor、plugin host tool 入口；后续工具扩展走 Skills + controlled CLI / Trusted CLI path。

## [0.1.0] - 2026-03-05

### Added
- **核心引擎**: Redb 追加日志存储 + CRDT 同步 + 向量时钟冲突解决
- **Leptos 前端**: CodeMirror 6 编辑器 + Markdown 实时预览 + KaTeX 数学公式
- **AI 集成**: OpenAI 兼容 SSE 流式 + Rhai 插件系统 + Agent Bridge
- **Source Control**: 类 Git 工作流（Watcher → pending → Stage → Commit）
- **安全**: Ed25519 身份密钥 + AES-GCM 仓库加密 + JWT 认证
- **国际化**: 手工 i18n 系统（中/英双语全覆盖）
- **文件树**: 增量更新 TreeManager + 前端实时同步
- **搜索**: Tantivy 全文搜索（可选 feature）
- **AI Context Injection**: 选中代码自动注入 AI 上下文
