# UI Diff Acceptance Closure - 2026-05-12

本报告记录 `UI Diff acceptance closure` 批次。`docs/plan/` 仍是唯一权威；本文件只记录本批次执行结果。

## Scope

- `docs/acceptance-cases/05_ui.md`
- `docs/acceptance-bindings.tsv`
- `scripts/check-source-control-baseline.sh`
- `apps/web/src/components/diff_view/`

## Changes

- 修正 `UI-DIFF-002..018` 在 `docs/acceptance-bindings.tsv` 中的语义备注，使其匹配 `docs/acceptance-cases/05_ui.md` 的真实 case。
- 为已有 diff behavior 增加最小自动 guard：
  - edit debounce constant
  - compute indicator visibility
  - hunk index / navigation wrap / header change stats
  - fold expand 与 context-lines visibility
  - semantic anchor delta
  - cache key repo-scope isolation
  - cache hit ratio 与 compute elapsed saturation
  - algorithm label
  - word-level replace ranges
- `UI-DIFF-004` 改为守住 hardcoded copy absence，不再用会误伤 `TextDiff::` 类型名的宽泛 `Diff:` 检索。

## Verification

已运行：

- `cargo fmt --check`
- `scripts/check-source-control-baseline.sh`
- `scripts/check-acceptance-bindings.sh`
- `scripts/check-feature-operation-paths.sh`
- `scripts/check-architecture-registry.sh`
- `scripts/plan-coverage.sh`
- `git diff --check`
- `cargo test -p deve_web diff_ -- --nocapture`
- `cargo test -p deve_web diff_edit_debounce -- --nocapture`
- `cargo test -p deve_web diff_compute_indicator -- --nocapture`
- `cargo test -p deve_web diff_hunk_navigation -- --nocapture`
- `cargo test -p deve_web diff_cache -- --nocapture`
- `cargo test -p deve_web diff_fold_rows -- --nocapture`
- `cargo test -p deve_web diff_context_lines -- --nocapture`
- `cargo test -p deve_web diff_semantic_anchor -- --nocapture`
- `cargo test -p deve_web diff_elapsed_ms -- --nocapture`
- `cargo test -p deve_web diff_algorithm_label -- --nocapture`
- `cargo test -p deve_web diff_cache_ratio -- --nocapture`
- `cargo test -p deve_web diff_cache_key -- --nocapture`
- `cargo test -p deve_web diff_replace_lines -- --nocapture`
- `cargo test -p deve_web diff_header_change_stats -- --nocapture`

结果：

- source-control baseline: pass
- acceptance binding: `89 automated / 63 feature / 32 manual / 0 unbound`
- architecture registry: `72 flows, 0 active drift`
- plan coverage blocking violations: `0`
- `cargo test -p deve_web diff_`: `73 passed`

## Residual Manual Scope

- 本批次不声称完成所有 Diff Chrome MCP 端到端验收。
- hunk button click、keyboard navigation、fold click、context select、cache badge UI 与 mobile edit debounce 仍可由后续 Chrome MCP smoke 做实机确认。
