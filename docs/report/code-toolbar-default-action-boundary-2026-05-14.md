# Code Toolbar 默认动作边界

日期：2026-05-14

## 范围

浏览器 smoke refresh 后的 fresh mainline gap scan。

Plan 来源：

- `docs/plan/03_rendering.md#code-block-toolbar-contract`
- `docs/plan/03_rendering.md#failure-modes`

Acceptance 来源：

- `docs/acceptance-cases/03_rendering.md` / `RENDER-CODE-001`

## 发现

Rendering plan 将 CodeMirror code toolbar 定义为 `Copy` 加可扩展 `Ellipsis` 菜单。未注册 action 时，菜单允许显示空状态，且不得中断渲染。

`apps/web/js/init.js` 注册了两个默认动作：

- `Run Code`
- `Send to AI`

两个 handler 只写 browser console，并保留 future runtime TODO。这会把 future Calculation Runtime / AI code-context 行为暴露成当前可点击能力。

## 修改

- 从 `apps/web/js/init.js` 移除默认 `Run Code` 与 `Send to AI` 注册。
- 保留 `window.deve_code_actions` 初始化，作为 browser-side extension point。
- 增加 rendering baseline guard，使默认 runtime 持续对齐 `RENDER-CODE-001`。
- 刷新 ignored local `apps/web/dist/js/init.js`，仅用于本机 embedded-frontend smoke 一致性；该文件不是 tracked artifact。

## 验证

- `bash scripts/check-rendering-baseline.sh`
- `bash scripts/plan-coverage.sh`
- `cargo test -p deve_cli static_files -- --nocapture`
- `git diff --check`

## 结果

Code toolbar 现在默认回到 plan 要求的空 action registry。未来 code execution 或 AI code-context action 必须显式注册真实 handler；console-only placeholder 不再作为当前能力暴露。
