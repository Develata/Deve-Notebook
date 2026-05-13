# Source Control CommitAndPush Browser Smoke - 2026-05-13

本报告记录 `Commit & Push` 浏览器点验。`docs/plan/` 仍是唯一权威；本文件只记录当前实现的验证结果。

## Scope

- `docs/plan/07_diff_logic.md#source-control-runtime`
- `docs/features/07_diff_logic.md`
- `docs/features/operations/sc_commit_and_push.md`
- `docs/acceptance-cases/04_diff.md` DIFF-009

## Environment

- Backend: `DEVE_LEDGER_DIR=/tmp/deve-commit-push-smoke-20260513-VJ8CFo/ledger DEVE_VAULT_PATH=/tmp/deve-commit-push-smoke-20260513-VJ8CFo/vault DEVE_STATIC_DIR=apps/web/dist cargo run -p deve_cli --bin deve_cli -- serve --dev --port 32125`
- Browser URL: `http://127.0.0.1:32125/`
- Data root: `/tmp/deve-commit-push-smoke-20260513-VJ8CFo`
- Chrome MCP viewport: mobile emulation `375x812`
- Auth: development defaults

## Results

Passed:

- Source Control panel reached local writable repo scope: `default / 本地`.
- External workspace file `commit-push-smoke.md` appeared as watcher-driven pending `A`.
- `暂存全部更改` moved the file into staged state and enabled the commit input.
- Entering `commit push smoke` enabled the split commit action.
- Opening the split action menu exposed `提交并推送`.
- Clicking `提交并推送` completed with user-visible `Committed: 6dd2e0f`.
- Server log confirmed the `CommitAndPush` handler path:
  - `Commit & Push: 6dd2e0f4-310e-406a-aef8-4b4bb7401239 - commit push smoke`
- Completion used the current `CommitAck` path; Source Control refreshed to clean local branch state.
- Reload recovery preserved the committed document and clean Source Control state.
- No browser console `error` or `warn` entries were present after the stable reload check.
- Browser network for the current page only showed auth/status and node role HTTP requests; no Web Git import/push/repair writer endpoint was called.
- The isolated worktree contained `.notegit/` and `.gitignore` protection only; no `.git/` mirror was created by the Web action.

## Boundary

- `CommitAndPush` remains a Source Control publish entry point and currently completes as `CommitAck`.
- Web did not gain Git mirror push authority.
- Git mirror publishing remains the explicit CLI surface defined by the plan.

## Verification

已运行：

- `bash scripts/check-source-control-baseline.sh`
- `cargo test -p deve_cli source_control_scope_nonce_gate -- --nocapture`
- `cargo test -p deve_cli readonly_remote_source_control_writes -- --nocapture`
- `cargo test -p deve_cli ws_source_control_stage_commit_history_roundtrip -- --nocapture`
- `cargo test -p deve_web commit_ack_dispatch -- --nocapture`
- `cargo test -p deve_web commit_refresh -- --nocapture`
- `cargo test -p deve_web commit_write_block -- --nocapture`
- Chrome MCP browser smoke as described above

结果：

- Source Control baseline guard: pass.
- Source Control scope nonce tests: pass, 2 tests.
- Remote readonly write gate test: pass, 1 test.
- WS stage / commit / history roundtrip: pass, 1 test.
- Web CommitAck dispatch test: pass, 1 test.
- Web commit refresh tests: pass, 4 tests.
- Web commit write-block test: pass, 1 test.
- Browser `Commit & Push` smoke: pass.
