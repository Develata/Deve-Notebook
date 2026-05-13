# Release Dependency Audit Gate

日期：2026-05-13

## 范围

- 计划锚点：`docs/plan/15_release.md`
- 验收锚点：`docs/acceptance-cases/12_tech_release.md#REL-003`
- 代码范围：`.github/workflows/release.yml`、`scripts/check-release-audit-gate.sh`、release/runbook guards。

## 结果

- 新增 `scripts/check-release-audit-gate.sh`。
- 本地默认模式：`cargo-audit` 或 `npm` 缺失时输出显式 skip 诊断。
- Release / CI required 模式：`DEVE_RELEASE_AUDIT_REQUIRED=1` 时缺少审计工具 fail-closed。
- GitHub release workflow 先安装 Node.js 20 与 `cargo-audit`，再以 required 模式运行审计脚本。
- `REL-003` 从裸 `cargo audit` 改为统一脚本入口。
- `docs/dev-runbook.md` 记录本地 diagnostic 与 CI required 两条路径。

## 本地验证结果

- `cargo-audit` 当前本机未安装；脚本输出 skip 诊断。
- `npm audit --audit-level=high` 通过。
- 现有 Mermaid advisory 为 `moderate`，不阻断 high/critical release gate。

## 验证

- `bash scripts/check-release-audit-gate.sh`
- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-dev-runbook-baseline.sh`
