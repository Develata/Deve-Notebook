# 发布与运维进度 (Delivery Schedule)

> 涵盖计划: 15_release
> 最近更新: 2026-03-05

## 1. 构建 (Build)
- [ ] **Multi-Platform**: Windows, macOS, Linux 构建配置. *(延期: 需签名证书 + Tauri)*
- [x] **Docker**: `Dockerfile` 编写 (Multi-stage, rust:1.92 + pinned trunk + esbuild → debian:bookworm-slim).
- [x] **Docker Compose**: 单服务配置 (512m 内存限制, /data 卷持久化).
- [ ] **Signing**: 代码签名配置. *(延期: 需 Apple/Windows 证书)*

## 2. 质量保证 (QA)
- [x] **Unit Tests**: 核心逻辑覆盖率 (43 unit + 14 integration tests).
- [ ] **E2E Tests**: 前端自动化测试. *(延期到 Phase 7)*
- [x] **Security Audit**: 依赖审计 (npm audit: 0 漏洞; cargo-audit 集成到 nightly CI).

## 3. CI/CD
- [x] **release.yml**: Tag 触发 → clippy + test → Docker 构建推送 GHCR.
- [x] **nightly.yml**: 每日 + main push → audit + test → Docker nightly 推送.
- [x] **静态文件服务**: Axum ServeDir 模块 (SPA fallback).

## 4. 文档 (Docs)
- [ ] **User Manual**: 用户手册. *(延期到 Phase 7)*
- [ ] **Developer Guide**: 插件开发指南. *(延期到 Phase 7)*
