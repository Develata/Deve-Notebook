# Release Verification Baseline - 2026-05-01

本报告合并 release smoke、full workspace verification、Docker cargo-chef 与 Web release warning 报告。

## Current Boundary

- Full workspace verification 在 2026-04-30 达到 blocking failures 0。
- Docker Desktop WSL integration 恢复后，Docker release smoke 通过镜像 build、容器启动与宿主 `/api/node/role` probe。
- Docker-only cargo-chef warning 来自 cargo-chef generated recipe/skeleton manifest，不是 checked-in workspace manifests。
- Dockerfile 只清理生成的 recipe 噪声，不改 repo manifests。
- Web release build 通过 `scripts/smoke-web-release-build.sh` 统一 `NO_COLOR` 与 Browserslist freshness 噪音处理。

## Verified Surfaces

- `scripts/plan-coverage.sh --summary-missing-plan-ref`
- `scripts/check-source-control-baseline.sh`
- `scripts/check-source-control-smoke-hygiene.sh`
- `scripts/check-cli-settings-baseline.sh`
- `scripts/check-native-track-boundary.sh`
- `scripts/check-graph-baseline.sh`
- `scripts/smoke-web-release-build.sh`
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- Docker release smoke with Docker daemon available.

## Retired Source Reports

- `cargo-chef-skeleton-warning-cleanup-2026-04-30.md`
- `cargo-chef-warning-triage-2026-04-29.md`
- `full-workspace-verification-pass-2026-04-30.md`
- `release-smoke-status-2026-04-28.md`
- `release-smoke-status-2026-04-29.md`
- `web-release-browserslist-triage-2026-04-30.md`
- `web-release-graph-warning-cleanup-2026-04-30.md`
