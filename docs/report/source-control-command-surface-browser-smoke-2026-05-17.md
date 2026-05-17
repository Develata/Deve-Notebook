# Source Control Command Surface Browser Smoke - 2026-05-17

本报告记录 Source Control Command Surface Refresh 的浏览器实测。`docs/plan/` 未修改。

## Environment

- Data root: `/tmp/deve-sc-command-smoke.tGIaja`
- Server: `cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3122`
- Frontend: embedded release assets from `scripts/smoke-web-release-build.sh`
- Browser: Chrome MCP attached to `http://127.0.0.1:3122/`
- Auth: development defaults

## Targeted Tests

Ran:

- `bash scripts/check-source-control-baseline.sh`
- `cargo test -p deve_web reserved_commands -- --nocapture`
- `cargo test -p deve_web command_sets_cli_only_notice -- --nocapture`
- `cargo test -p deve_web git_ -- --nocapture`
- `cargo test -p deve_web establish_branch_command -- --nocapture`
- `cargo test -p deve_web static_commands_partition_reserved_surfaces -- --nocapture`
- `cargo test -p deve_web source_control_commit -- --nocapture`

Result: all checks passed.

## Browser Steps

1. Built embedded Web assets.
2. Initialized isolated data root.
3. Started dev server with isolated `DEVE_LEDGER_DIR` and `DEVE_VAULT_PATH`.
4. Opened Command Palette in command mode.
5. Verified Source Control reserved entries:
   - `Source Control: 同步`
   - `Source Control: 提交`
   - `Source Control: 推送`
6. Verified Git entries show CLI-only boundaries:
   - `Git: 状态`
   - `Git: 执行 Mirror`
   - `Git: 导出 Mirror`
7. Executed `Git: 状态` while Source Control panel was open.
8. Created external workspace file `vault/default/sc-smoke.md`.
9. Verified Source Control detected `sc-smoke.md` as an added change.
10. Staged the file.
11. Entered commit message `source control smoke`.
12. Committed from the Source Control panel.
13. Checked current navigation console.

## Observations

- Command Palette rendered reserved Source Control entries with unavailable copy:
  - `不可用：请在 Source Control 面板中使用带作用域与提交信息的操作`
- Command Palette rendered Git mirror entries with CLI-only copy:
  - `CLI-only：Web 不执行 Git 写命令`
- Executing `Git: 状态` did not run a Web Git writer.
- Source Control notice showed:
  - `Git status 只能通过 CLI 查看`
  - `请运行 \`deve_cli git status --repo <repo>\` 查看 mirror readiness 与队列状态。`
- External file change was detected:
  - `sc-smoke.md`
  - status `A`
  - branch marker `本地*`
- Stage action moved the file into staged state and enabled the commit message surface.
- Commit message made the commit button enabled.
- Commit succeeded:
  - UI showed `Committed: cdecc89`
  - branch marker returned to `本地`
  - pending/staged lists cleared.
- Current navigation console:
  - No `error` or `warn` messages after the smoke interactions.

## Result

PASS.

No product code bug was found in this smoke. The Source Control command surface remains within the intended boundary: Command Palette can discover/route/notarize unavailable or CLI-only operations, while actual stage/commit authority stays in the Source Control panel.
