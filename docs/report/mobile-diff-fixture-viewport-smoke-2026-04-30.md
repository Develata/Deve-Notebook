# Mobile Diff Fixture Viewport Smoke - 2026-04-30

## 结论

Chrome MCP 375x812 mobile viewport smoke 已覆盖真实 `.diff-view-mobile` 场景。移动端 Diff 打开时 AI Chat 入口与移动辅助键盘栏均隐藏，关闭按钮保持在视口内并满足 44x44 命中区，点击关闭后返回 editor。

## 本批次发现并修复

- 移动端 Diff header 原来把文件名、增删统计、缓存、算法、耗时、hunk 导航、编辑和关闭按钮全部塞进单行。
- Chrome MCP 测量显示 `.diff-close-button` 被挤到视口右缘，位置约为 `x=371.97,w=36`，375px 视口下不可稳定点击。
- 修复方式：`DiffHeader` 移动端改为两行布局；第一行固定标题与 `.diff-close-button`，第二行横向承载统计、编辑、缓存、算法与 hunk 导航。
- `.diff-edit-toggle` 前置到二级工具栏靠左位置，避免默认被横向内容挤出首屏。

## Chrome MCP 结果

- Viewport emulation: `375x812`, `isMobile=true`, `hasTouch=true`, `deviceScaleFactor=2`。
- Fixture: 隔离目录 `/tmp/deve-mobile-diff-smoke-registered`，先由 UI 创建 `Untitled.md`，再修改已登记的临时 vault 文件以产生 Source Control diff。
- Diff visible: `.diff-view-mobile` rect `x=0,y=49,w=375,h=712`。
- Close button: `.diff-close-button` rect `x=319,y=49,w=44,h=44`，`closeWithinViewport=true`。
- Edit button: `.diff-edit-toggle` rect `x=76.21875,y=92,w=50,h=36`，`editWithinViewport=true`。
- Hidden chrome: `[data-deve-mobile-chat] == null`，`.mobile-accessory-toolbar == null`。
- Close return: 点击 `.diff-close-button` 后 `.diff-view-mobile == false`，editor visible，editor rect `x=16,y=221.5,w=259.5,h=36`。

## Fixture 注意事项

不要把未登记到 ledger 的 markdown 直接预置到临时 vault 作为 fixture。实测该路径会触发 `Handler: New file detected` 重复日志并最终撞到 WS rate limit；后续应单独做 watcher external new-file pending debounce/registration hardening。

## 验证

- `DEVE_LEDGER_DIR=/tmp/deve-mobile-diff-smoke-registered/ledger DEVE_VAULT_PATH=/tmp/deve-mobile-diff-smoke-registered/vault cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001`
- `NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080`
- Chrome MCP viewport smoke at `http://127.0.0.1:8080/`
- `cargo test -p deve_web`
