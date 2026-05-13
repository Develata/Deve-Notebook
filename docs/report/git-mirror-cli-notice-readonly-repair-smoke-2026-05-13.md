# Git Mirror CLI Notice / Readonly Repair Smoke - 2026-05-13

本报告记录 `Git mirror CLI-only notice / readonly repair review smoke`。`docs/plan/` 仍是唯一权威；本文件只记录执行结果。

## Scope

- `docs/plan/07_diff_logic.md#git-mirror-lifecycle`
- `docs/plan/12_commands.md#command-palette-shortcuts`
- `docs/features/07_diff_logic.md`
- `docs/features/12_commands.md`
- `docs/features/operations/ui_command_palette.md`
- `docs/acceptance-cases/04_diff.md` DIFF-009

## Environment

- Frontend build: rebuilt with `scripts/smoke-web-release-build.sh`.
- Backend: `DEVE_LEDGER_DIR=/tmp/deve-git-notice-20260513-JPw2nz/ledger DEVE_VAULT_PATH=/tmp/deve-git-notice-20260513-JPw2nz/vault DEVE_STATIC_DIR=apps/web/dist cargo run -p deve_cli --bin deve_cli -- serve --dev --port 32024`
- Browser URL: `http://127.0.0.1:32024/`
- Data root: `/tmp/deve-git-notice-20260513-JPw2nz`
- Chrome MCP viewport: desktop `1280x900`
- Auth: development defaults, login `admin / admin`

## Results

Passed:

- Command Palette in Chinese locale can find Git bridge commands by stable English command-id words:
  - `>git import` -> `Git: 导入外部变更`
  - `>git push` -> `Git: 推送 Mirror`
  - `>git repair` -> `Git: 修复 Mirror`
- Executing `Git: 导入外部变更` displayed Source Control CLI-only notice:
  - `Git import 只能通过 CLI 执行`
  - `deve_cli git import --apply --repo <repo>`
- Executing `Git: 推送 Mirror` displayed Source Control CLI-only notice:
  - `Git mirror 推送只能通过 CLI 执行`
  - remote/upstream, mirror mapping, out-of-sync, dirty Git worktree, dirty Deve Source Control blockers.
- Executing `Git: 修复 Mirror` displayed Source Control CLI-only notice:
  - `Git mirror 修复只能通过 CLI 执行`
  - `repair_action[...]`
  - `deve_cli git export --repo <repo> --retry-out-of-sync`
- Repair review rendered as readonly:
  - `data-deve-git-repair-review="readonly"`
  - `data-deve-git-repair-manual-only="true"`
  - retry command text is selectable/copyable only.
- Empty server-side repair-review state rendered the CLI fallback without granting write authority.
- Browser network showed `/api/sc/git-mirror/repair-review?...` as a `GET 200`.
- No browser request to a Web Git writer/import/push/repair executor was observed.
- Browser console contained no `error` or `warn` entries.

## Fixed Bugs

- `CommandProvider` now matches both localized command title and normalized stable command id, so English command-id words work under Chinese UI.
- Git Command Palette actions now capture `SourceControlContext` at command registration time instead of trying to resolve context inside the event callback.
- No `docs/plan/` files were changed.

## Verification

已运行：

- `bash scripts/check-source-control-baseline.sh`
- `cargo test -p deve_web command_sets_cli_only_notice -- --nocapture`
- `cargo test -p deve_web local_git_repair_notice -- --nocapture`
- `cargo test -p deve_web command_provider -- --nocapture`
- `cargo test -p deve_cli status_lines_include_cli_only_repair_action -- --nocapture`
- `cargo test -p deve_cli status_lines_include_guidance_for_all_repair_actions -- --nocapture`
- `cargo test -p deve_cli test_git_mirror_repair_review_is_readonly_record_source -- --nocapture`
- `cargo test -p deve_cli git_import_export_push_resolved_publish_roundtrip -- --nocapture`
- `scripts/smoke-web-release-build.sh`
- Chrome MCP browser smoke as described above

结果：

- Source Control baseline guard: pass
- Git command CLI-only notice tests: pass, 3 tests
- Local Git repair notice copy test: pass, 1 test
- Command provider id-search tests: pass, 2 tests
- Git mirror CLI repair-action tests: pass, 2 tests
- Git mirror readonly repair-review HTTP test: pass, 1 test
- Git import/export/push resolved publish roundtrip: pass, 1 test
- Web release build: pass
- Browser Git mirror CLI notice / readonly repair smoke: pass
