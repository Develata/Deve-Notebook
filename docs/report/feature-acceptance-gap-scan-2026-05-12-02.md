# Feature Acceptance Gap Scan - 2026-05-12 02

本报告记录 Source Control browser smoke 之后的下一轮 feature / acceptance / code 交叉扫描。`docs/plan/` 仍是唯一权威；本文件只作为执行队列输入。

## Scope

- `docs/features/operation-coverage.md`
- `docs/features/operations/`
- `docs/acceptance-cases/`
- `docs/dev-runbook.md`
- 当前 baseline / smoke scripts

## Verification Snapshot

已运行：

- `scripts/check-acceptance-bindings.sh`
- `scripts/check-architecture-registry.sh`
- `scripts/check-feature-operation-paths.sh`
- `scripts/check-rendering-baseline.sh`
- `scripts/check-search-baseline.sh`
- `scripts/check-source-control-baseline.sh`
- `scripts/check-network-baseline.sh`
- `scripts/check-auth-baseline.sh`
- `scripts/check-ai-baseline.sh`
- `scripts/check-cli-settings-baseline.sh`
- `scripts/check-mobile-baseline.sh`
- `scripts/check-ui-dashboard-refresh-baseline.sh`
- `scripts/check-ui-desktop-baseline.sh`
- `scripts/check-ui-disconnect-baseline.sh`
- `scripts/check-ui-focus-baseline.sh`
- `scripts/check-ui-spa-routing-baseline.sh`
- `scripts/check-ui-token-baseline.sh`
- `scripts/check-ui-z-index-baseline.sh`
- `scripts/check-browser-prefs-boundary.sh`
- `scripts/check-native-track-boundary.sh`
- `scripts/check-native-packaging-gate.sh`
- `scripts/check-release-baseline.sh`
- `scripts/check-dev-runbook-baseline.sh`
- `scripts/plan-coverage.sh`
- `scripts/check-graph-baseline.sh`

## Findings

### F1. Feature / Acceptance Bindings Are Closed

结果：

- automated acceptance bindings: 67
- feature walkthrough bindings: 68
- manual acceptance bindings: 49
- unbound acceptance cases: 0
- architecture registry: 72 flows, 0 active drift
- feature operation path drift: ok

当前未发现 operation registry、acceptance case 或代码路径之间的阻塞漂移。

### F2. Dev Runbook Missed The New Path Guard

问题：

- `scripts/check-feature-operation-paths.sh` 已新增并接入 `scripts/plan-coverage.sh`。
- `docs/dev-runbook.md` 的 guard list 未登记该脚本。
- `scripts/check-dev-runbook-baseline.sh` 因此失败。

处理：

- 已把 `scripts/check-feature-operation-paths.sh` 加入 `docs/dev-runbook.md` 的当前 docs/code guard scripts 列表。

### F3. Next User-Facing Gap Is Release Delivery Smoke Refresh

浏览器主链路、Rendering、Search、Source Control smoke 已在隔离数据根中刷新。下一批最有价值的用户验收不是新增抽象，而是刷新 release delivery：

- Web release build 是否仍能生成当前 embedded frontend。
- 当前运行实例 `/api/node/role` 是否仍暴露 version/profile/delivery/environment/repo health。
- Docker production-auth image smoke 是否仍能 build、start、probe 并清理。

这直接对应 `REL-002`、`REL-005`、`REL-006`，并覆盖用户实际部署入口。

### F4. Platform Packaging Still Remains Gate-Closed

Desktop / mobile 当前仍是 shell scaffold、native adapter contract 与 packaging dependency gate，不应在 release smoke 之前抢占主线。已运行 native track / packaging gate；结果为 ok。

## Next Execution Queue

1. Release delivery smoke refresh：`scripts/smoke-web-release-build.sh`、临时后端 `scripts/smoke-runtime-release-info.sh`、`DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh`。
2. 若 release smoke 暴露发布或 Docker 缺陷，先修缺陷；否则继续下一轮 feature / acceptance gap scan。
