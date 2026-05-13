# Plan-Code Mapping Soft Cleanup - 2026-05-13

本报告记录 `plan_ref` soft warning 清理。`docs/plan/` 仍是唯一权威；本文件只记录执行结果。

## Scope

- `docs/plan/AGENTS.md` Layer 1 / Layer 2
- `scripts/plan-coverage.sh`
- Git mirror bridge submodules
- WS JSON payload adapter
- Source Control commit diff typed error module

## Changes

- 修正 `plan-coverage.sh` 的测试豁免规则：
  - `*_test/` 目录
  - `*_tests/` 目录
  - `test_*/` 目录
  - `*_tests.rs` 文件
- 给剩余生产模块补充准确 `plan_ref`：
  - `crates/core/src/git_bridge/error/*.rs`
  - `crates/core/src/git_bridge/import_apply/*.rs`
  - `crates/core/src/git_bridge/push/target.rs`
  - `crates/core/src/protocol/json_wire.rs`
  - `crates/core/src/source_control/commit_diff_error.rs`

## Results

Passed:

- 非豁免缺失 `plan_ref` 从 `56` 降到 `0`.
- `plan_ref` dangling blocking 仍为 `0`.
- 测试目录与测试文件不再被错误计入 runtime mapping debt.
- 未触碰 `docs/plan/`.
- 未进行机械拆分；剩余 >250 行 soft warnings 只保留为 cohesive file review 信号。

## Verification

已运行：

- `bash scripts/plan-coverage.sh --summary-missing-plan-ref`
- `bash scripts/plan-coverage.sh --list-missing-plan-ref`
- `bash scripts/plan-coverage.sh`
- `bash -n scripts/plan-coverage.sh`
- `cargo check -p deve_core`
- `git diff --check`

结果：

- Missing `plan_ref`: pass, `0`.
- Full plan coverage: pass, blocking violations `0`.
- Shell syntax: pass.
- Core compile check: pass.
- Diff whitespace check: pass.
