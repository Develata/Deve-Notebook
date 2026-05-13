# Remote Spectator Readonly UI Smoke - 2026-05-13

本报告记录 `Repo / remote spectator read-only UI smoke`。`docs/plan/` 仍是唯一权威；本文件只记录执行结果。

## Scope

- `docs/plan/06_repository.md`
- `docs/plan/07_diff_logic.md`
- `docs/plan/08_ui_design.md`
- `docs/features/06_repository.md` REPO-FEAT-02 / REPO-FEAT-03
- `docs/features/07_diff_logic.md` DIFF-FEAT-03

## Environment

- Frontend build: `bash scripts/smoke-web-release-build.sh`
- Fixture: `cargo run -p deve_cli --bin deve_cli -- seed-merge-conflict-fixture --peer peer-a --path readonly-remote.md ...`
- Backend: `DEVE_LEDGER_DIR=/tmp/deve-remote-spectator-20260513-eZrjMC/ledger DEVE_VAULT_PATH=/tmp/deve-remote-spectator-20260513-eZrjMC/vault DEVE_STATIC_DIR=apps/web/dist cargo run -p deve_cli --bin deve_cli -- serve --dev --port 31994`
- Browser URL: `http://127.0.0.1:31994/`
- Data root: `/tmp/deve-remote-spectator-20260513-eZrjMC`
- Local repo: `default`
- Remote branch: `peer-a`

## Results

Passed:

- Web shell reached `Ready` on local `default`.
- Local branch opened `readonly-remote.md` with local content `# Local Writable`.
- Local branch editor was writable with `contenteditable=true`.
- Branch switcher listed `Local` and `peer-a Remote Branch`.
- Switching to `peer-a` changed header state to `Read-only` and footer state to `peer-a / 只读`.
- Remote branch opened the same `readonly-remote.md` from shadow scope with remote content `# Remote Readonly`.
- Remote branch editor was read-only with `contenteditable=false` and `aria-readonly=true`.
- Explorer create action was hidden in remote opened-document state.
- Dashboard quick action `新建文档` was disabled in remote dashboard state.
- Source Control showed the readonly branch notice: `切回本地分支后才能查看变更、暂存文件或提交。`
- Source Control did not expose commit/stage write buttons as active remote-branch actions.
- Quick Open forced create attempt `+remote-create-blocked.md` was rejected with `Cannot create document: read-only`.
- The forced remote create attempt did not write `vault/default/remote-create-blocked.md`.
- Switching `peer-a -> Local` restored header `Ready`, footer `本地 / 就绪`, local content `# Local Writable`, and editor `contenteditable=true`.

## Notes

- The only current-page console warning during smoke was the expected write-gate feedback for the forced create attempt.
- The write-gate toast currently uses the existing English copy path `Cannot create document: read-only`; this is pre-existing and separately test-covered by `write_gate_banner`.
- No `UnsupportedVersion`, stale-scope lockout, auth lockout, or uncaught application panic was observed.

## Verification

已运行：

- `bash scripts/smoke-web-release-build.sh`
- `cargo test -p deve_web repo_write_gate_blocks_remote_branches_as_read_only -- --nocapture`
- `cargo test -p deve_cli merge_manual_write_readonly_gate -- --nocapture`
- `cargo test -p deve_cli readonly_remote_source_control_writes_are_rejected_before_mutation -- --nocapture`
- `cargo test -p deve_cli rejects_browser_writer_on_remote_branch_and_clears_stale_writer -- --nocapture`
- `cargo test -p deve_cli list_docs_on_remote_branch_uses_shadow_repo_without_locked_db -- --nocapture`
- `cargo test -p deve_cli readonly_remote_doc_diff_uses_shadow_projection -- --nocapture`
- `cargo test -p deve_web source_control_scope_rejects_remote_branches -- --nocapture`
- `cargo test -p deve_web source_control_read_scope_allows_remote_branches -- --nocapture`
- `cargo test -p deve_web repo_source_control_read_gate_allows_remote_branch_reads -- --nocapture`
- `cargo test -p deve_web doc_diff_read_gate_allows_remote_branch_spectator_reads -- --nocapture`
- Chrome MCP browser smoke as described above

结果：

- Browser remote spectator readonly smoke: pass
- Remote branch source-control write gate tests: pass
- Remote branch read-scope tests: pass
- Remote branch server writer rejection tests: pass
