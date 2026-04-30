# Release Verification Baseline - 2026-05-01

本报告合并 release smoke、full workspace verification、Docker cargo-chef 与 Web release warning 报告。

## Current Boundary

- Full workspace verification 在 2026-04-30 达到 blocking failures 0。
- Docker Desktop WSL integration 恢复后，Docker release smoke 通过镜像 build、容器启动与宿主 `/api/node/role` probe。
- Docker-only cargo-chef warning 来自 cargo-chef generated recipe/skeleton manifest，不是 checked-in workspace manifests。
- Dockerfile 只清理生成的 recipe 噪声，不改 repo manifests。
- Web release build 通过 `scripts/smoke-web-release-build.sh` 统一 `NO_COLOR` 与 Browserslist freshness 噪音处理。

## Verified Surfaces

- Full verification run 在修复 WS structured-error guard false positive 与 `serve.rs` 的 `clippy` needless-borrow 后记录 blocking failures 0。
- `scripts/plan-coverage.sh --summary-missing-plan-ref`
- `scripts/check-source-control-baseline.sh`
- `scripts/check-source-control-smoke-hygiene.sh`
- `scripts/check-auth-baseline.sh`
- `scripts/check-network-baseline.sh`
- `scripts/check-cli-settings-baseline.sh`
- `scripts/check-browser-prefs-boundary.sh`
- `scripts/check-search-baseline.sh`
- `scripts/check-rendering-baseline.sh`
- `scripts/check-ai-baseline.sh`
- `scripts/check-dev-data-health-baseline.sh`
- `scripts/check-native-track-boundary.sh`
- `scripts/check-graph-baseline.sh`
- `scripts/check-dev-runbook-baseline.sh`
- `scripts/check-ws-structured-errors.sh`
- `scripts/check-release-baseline.sh`
- `scripts/check-architecture-registry.sh`
- `scripts/smoke-web-release-build.sh`
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `DEVE_RUNTIME_BASE_URL=http://127.0.0.1:3101 DEVE_RUNTIME_SMOKE_REQUIRED=1 scripts/smoke-runtime-release-info.sh`
- `DEVE_DOCKER_SMOKE_REQUIRED=1 DEVE_DOCKER_SMOKE_PORT=3102 scripts/smoke-docker-release.sh`
- Runtime smoke 使用隔离 `/tmp/deve-runtime-smoke-*` 数据根。
- Docker smoke 构建 `deve-notebook:local-smoke`，以 production auth material 启动 release image，并从宿主 probe `/api/node/role`。
- 如果 Docker host endpoint smoke 回归，先检查 proxy 变量与 WSL port forwarding；脚本会绕过 local proxy，并在失败前输出 container health 与内部 endpoint diagnostics。

## Retired Source Reports

- `cargo-chef-skeleton-warning-cleanup-2026-04-30.md`
- `cargo-chef-warning-triage-2026-04-29.md`
- `full-workspace-verification-pass-2026-04-30.md`
- `release-smoke-status-2026-04-28.md`
- `release-smoke-status-2026-04-29.md`
- `web-release-browserslist-triage-2026-04-30.md`
- `web-release-graph-warning-cleanup-2026-04-30.md`
