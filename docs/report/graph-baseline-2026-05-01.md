# Graph Baseline - 2026-05-01

本报告合并 graph projection、Web summary panel、degraded error 与 renderer gate 的短状态报告。

## Current Boundary

- Graph 当前停靠在只读 projection data surface。
- `deve_core::graph` 保持 authority-free projection helper。
- CLI/HTTP 共享只读 adapter；默认 fail-closed 于损坏 Structure Facts authority。
- Web 只显示 Source Control Graph summary panel，不引入 Canvas/d3-force/Pixi renderer dependency。
- Degraded projection 必须显式 opt-in；HTTP degraded error 使用结构化 `GRAPH_DEGRADED_PROJECTION_REQUIRED`。

## Verified Surfaces

- `deve graph` repo-scoped `GraphProjection` JSON。
- 受保护 `GET /api/repo/graph` query。
- Web graph summary loading/failed/empty/local-only/blocked/degraded 状态。
- `scripts/check-graph-baseline.sh`。
- `ServerErrorCode::GraphDegradedProjectionRequired` 序列化为 `GRAPH_DEGRADED_PROJECTION_REQUIRED`，来源是 typed `GraphProjectionError::DegradedProjectionRequired`。
- Web 必须按 server error code 识别 degraded projection，不得匹配人类可读 detail 文本；Web 不自动以 `allow_degraded_projection=true` 重试。
- CLI 可以保留 `--allow-degraded-projection` operator hint；该提示不属于 Web 分类依据。
- 验证包括 `cargo test -p deve_core protocol -- --nocapture`、`cargo test -p deve_cli graph -- --nocapture`、`cargo test -p deve_web graph -- --nocapture`。

## Retired Source Reports

- `graph-blocked-degraded-acceptance-polish-2026-04-30.md`
- `graph-http-projection-status-2026-04-29.md`
- `graph-renderer-gate-decision-2026-04-29.md`
- `graph-structured-degraded-error-2026-04-30.md`
- `graph-web-projection-panel-status-2026-04-29.md`
