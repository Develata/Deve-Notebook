# Command Surface Routing Smoke - 2026-05-13

本报告记录 `Command Palette / Quick Open / Branch Switcher routing smoke`。`docs/plan/` 仍是唯一权威；本文件只记录执行结果。

## Scope

- `docs/plan/12_commands.md#command-palette-shortcuts`
- `docs/plan/13_settings.md#keyboard-shortcuts`
- `docs/features/12_commands.md`
- `docs/features/operations/ui_command_palette.md`
- `docs/features/operations/repo_open_doc.md`
- `docs/features/operations/repo_branch_switch.md`
- `docs/acceptance-cases/11_commands_settings.md` CMD-002 / CMD-003 / CMD-004
- `docs/acceptance-cases/14_operation_flow_refs.md` REPO-FEAT-02

## Environment

- Frontend build: existing `apps/web/dist` from the current release build output.
- Backend: `DEVE_LEDGER_DIR=/tmp/deve-command-surface-20260513-YsAZnB/ledger DEVE_VAULT_PATH=/tmp/deve-command-surface-20260513-YsAZnB/vault DEVE_STATIC_DIR=apps/web/dist cargo run -p deve_cli --bin deve_cli -- serve --dev --port 32021`
- Browser URL: `http://127.0.0.1:32021/`
- Data root: `/tmp/deve-command-surface-20260513-YsAZnB`
- Chrome MCP viewport: desktop `1280x900`
- Auth: development defaults, login `admin / admin`

## Results

Passed:

- `Ctrl+Shift+P` opened the unified command surface in command mode with query value `>`.
- Command mode displayed command results including `打开文档`, `打开设置 (config)`, language, P2P, Git CLI-only notice entries and AI chat toggle.
- Filtering with `>config` reduced results to `打开设置 (config)`.
- Pressing `Enter` on the filtered command opened Settings and closed the command surface.
- `Ctrl+P` opened Quick Open in file mode with no command prefix.
- Quick Open listed `routing-smoke.md`, accepted typed query `routing`, handled `ArrowDown` / `ArrowUp`, and `Enter` closed the surface while preserving the selected document.
- `Ctrl+Shift+K` opened branch switching mode with query value `@`.
- Branch switcher listed `Local` with `Current Branch`; `Escape` closed the surface without changing scope.
- Browser console contained no `error` or `warn` entries.
- Network requests observed during the selected page run returned HTTP `200`; no failed auth, node-role, static asset, WASM, editor bundle or capability request was observed.

## Fixed Drift

- Updated `docs/features/operations/ui_command_palette.md` from stale `Ctrl/Cmd+K` to `Ctrl/Cmd+Shift+P`.
- Updated component AGENTS notes from stale `Ctrl+K` wording to `Ctrl/Cmd+Shift+P`.
- No `docs/plan/` files were changed.
- No runtime source code changes were needed.

## Verification

已运行：

- `cargo test -p deve_web command_palette -- --nocapture`
- `cargo test -p deve_web search_box -- --nocapture`
- `cargo test -p deve_web branch_provider -- --nocapture`
- `cargo test -p deve_web shortcuts -- --nocapture`
- `bash scripts/check-cli-settings-baseline.sh`
- `bash scripts/check-ui-focus-baseline.sh`
- Chrome MCP browser smoke as described above

结果：

- Command palette tests: pass, 6 tests
- Search box tests: pass, 36 tests
- Branch provider tests: pass, 2 tests
- Shortcut prefs test: pass, 1 test
- CLI/settings baseline guard: pass
- UI focus baseline guard: pass
- Browser command surface routing smoke: pass
