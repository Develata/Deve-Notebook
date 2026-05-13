# Rendering Interaction Spot Smoke - 2026-05-13

本报告记录 `Rendering interaction spot smoke` 的真实浏览器点验。`docs/plan/` 仍是唯一权威；本文件只记录 source-first projection 的实机证据。

## Scope

- 唯一真源：`docs/plan/03_rendering.md`
- 验收目标：覆盖 code toolbar、Ctrl/Cmd link activation、Outline navigation、Mermaid projection、nested rendering。
- 非目标：不验证 Live Preview、Milkdown、富文本 authority、完整 virtual rendering 或 rendering settings GUI 持久化。

## Environment

- Web assets：`scripts/smoke-web-release-build.sh`
- Backend：`serve --dev --port 3021`
- Frontend delivery：`DEVE_STATIC_DIR=apps/web/dist`
- Data root：`/tmp/deve-rendering-interaction-20260513-h5ZPHX`
- Browser tool：Chrome MCP
- Test document：`render-interaction.md`

## Fixture

通过 Web UI 创建 `render-interaction.md`，再在当前 CodeMirror view 写入同一份 Markdown source：

- YAML frontmatter
- heading with `**bold**` / inline code / `$a^2$` / literal `==plain==`
- nested list + blockquote + nested list
- fenced `js` code block
- `[Example Link](https://example.com)`
- fenced `mermaid` block
- `## Beta Section` 与 `### Gamma Section`
- 足够 filler lines，使 Outline jump 产生实际 scroll/selection 效果

## Browser Evidence

Passed:

- Code toolbar：`.cm-code-toolbar` 出现 1 个 toolbar，包含 `Copy Code` 与 `More Actions` 两个按钮。
- Code toolbar menu：点击 `More Actions` 打开扩展菜单；当前 runtime 注册了 `Run Code` / `Send to AI` actions，未报错。
- Mermaid projection：离开源码范围后出现 `.cm-mermaid-widget svg`，SVG `width=100%`，`preserveAspectRatio=xMidYMid meet`。
- Mermaid source reveal：将 selection 移入 fenced `mermaid` 源码范围后，`.cm-mermaid-widget` 消失，源码 fence 标记与 `graph TD` 可见。
- Link activation gate：普通 click 不触发 `window.open`；Ctrl+click 触发一次 `window.open("https://example.com", "_blank", "noopener,noreferrer")`。
- Outline projection：Outline 显示 `Alpha`、`Beta Section`、`Gamma Section`；`==plain==` 按普通文本保留。
- Outline navigation：点击 `Gamma Section` 后 CodeMirror selection 落到第 170 行，`.cm-scroller.scrollTop` 进入目标区域。
- Nested rendering：nested list / blockquote lines 保持可见，blockquote line 带 `cm-blockquote-depth-1` decoration。
- Source authority：vault 文件 `vault/default/render-interaction.md` 保留原始 Markdown source，包括 `==plain==`、fenced code 与 fenced `mermaid`。
- Network：本次页面请求只命中本地 server 与本地静态 assets；未出现 Mermaid runtime 外部网络请求。
- Console：当前页面 error/warn 列表为空。

## Automated Verification

已运行：

- `scripts/smoke-web-release-build.sh`
- `scripts/check-rendering-baseline.sh`
- `scripts/check-large-doc-baseline.sh`
- `cargo test -p deve_web large_doc_search_gate -- --nocapture`
- `cargo test -p deve_web outline -- --nocapture`

结果：全部通过。

## Status

`Rendering interaction spot smoke` 可关闭。本批未修改运行时代码；后续若要扩大渲染验收，应继续保持 source-first projection 边界，不得把 widget 或 preview projection 升格为 authority。
