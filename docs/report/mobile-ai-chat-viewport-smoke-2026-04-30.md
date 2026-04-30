# Mobile AI Chat Viewport Smoke - 2026-04-30

## 结论

Chrome MCP 375x812 mobile viewport smoke 已执行。移动端 AI Chat 展开、输入聚焦、关闭返回、drawer 层级互斥通过；Diff 真实浏览器场景未覆盖到可用 fixture，继续保留下一步 fixture smoke。

## 本批次发现并修复

- Trunk WASM build 失败：Web 端直接 import `deve_core::git_bridge`，但该模块在 `wasm32` 下被 `cfg(not(target_arch = "wasm32"))` 关闭。
- 修复方式：Web 端新增只读 Git mirror repair-review DTO，通过 `api` re-export 使用 HTTP JSON shape，不暴露后端-only Git bridge 模块到 WASM。
- Chrome MCP 测量发现移动端 Chat 发送按钮宽度为 32px，不满足 44x44 命中区。
- 修复方式：移动端关键按钮从 `min-w-11` / `min-w-12` 改为明确的 `min-w-[44px]` / `min-w-[48px]`，避免 Tailwind 未生成 spacing-based min-width。

## Chrome MCP 结果

- Viewport emulation: `375x812`, `isMobile=true`, `hasTouch=true`, `deviceScaleFactor=2`。
- Login: dev `admin/admin`。
- Chat expanded: `data-deve-mobile-chat="expanded"`，sheet rect `375x812`。
- Input focus: active element 为 `ai-chat-input`，输入框可见。
- Send button: rect `44x44`，输入文本后 enabled。
- Close: 点击右上关闭后返回主界面，collapsed chip 可见。
- Drawer: 打开文件树 drawer 后，mobile chat sheet/chip 不可见。

## 未覆盖

- 真实浏览器 Diff 场景未覆盖：当前 smoke 没有稳定打开 `.diff-view-mobile` 的 fixture。代码层已有 `diff_open` 时隐藏 chat 的测试，下一步应准备或定位可复用 diff fixture 做 Chrome MCP 验证。

## 验证

- `NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080`
- Chrome MCP viewport smoke at `http://127.0.0.1:8080/`
- `cargo test -p deve_web`
- `scripts/check-acceptance-bindings.sh`
- `scripts/check-dev-runbook-baseline.sh`
- `git diff --check`
