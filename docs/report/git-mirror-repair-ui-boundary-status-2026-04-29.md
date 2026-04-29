# Git Mirror Repair UI Boundary Status - 2026-04-29

## 已完成

- `docs/features/07_diff_logic.md` 新增 Git Mirror Repair UI Boundary：当前 Web 仍是 CLI-only notice；future clickable repair UI 必须先进入只读 review flow。
- `docs/features/12_commands.md` 明确 `Git: Repair Mirror` 当前只打开 CLI-only notice；下一阶段 Command Palette 只能进入 review flow，不能绕过 Source Control gate 直接写 Git。
- `docs/features/14_tech_stack.md` 明确 Git ecosystem bridge 当前稳定边界是 CLI status/export/import/push 与 Web CLI-only notices；可点击 repair UI 是 future UI layer。
- `docs/acceptance-cases/04_diff.md` 的 DIFF-009 扩展为同时检查 `repair_guidance[...]`、manual-only 边界、future manual confirmation 与后台 Git writer 缺席。
- `docs/plan/12_commands.md` 与 `docs/plan/14_tech_stack.md` 同步 future UI 约束：只读 review、manual confirmation、fail-closed gates、`.notegit` authority、禁止后台自动 Git repair。

## 允许的下一阶段

- 可以实现 Web read-only repair review scaffold：展示 `repair_action[...]` / `repair_guidance[...]` 的解释、subject、next step 与 copyable retry command。
- 可以在 UI 上预留 disabled / future confirmation affordance，但不得执行 Git。
- 可以把 CLI guidance 文案映射到 Web i18n，但不得新增 Web 后端 Git writer。

## 明确禁止

- 不得从 Command Palette 直接执行 Git repair。
- 不得在后台任务中自动运行 `git mirror`、`git export --retry-out-of-sync` 或任何 Git write。
- 不得把 `.git` mirror 状态提升为 authority，也不得让 repair UI 改写 `.notegit` / ledger source-control tables。

## 验证

- `scripts/plan-coverage.sh`
- `scripts/check-acceptance-bindings.sh`
- `git diff --check`
