# Mobile UI Baseline - 2026-05-01

本报告合并移动端 AI Chat、Diff、touch feedback 与 editor font-size 的 viewport/baseline 报告。

## Current Boundary

- Mobile AI Chat 展开态在软键盘出现时保持可见，折叠入口在键盘态隐藏。
- Drawer / Diff 层级优先于 AI Chat，避免移动端遮挡核心编辑路径。
- Diff mobile view 使用两行 header，关闭和编辑入口必须留在 375px 视口内并满足可点击命中区。
- `.cm-content` 移动端 font-size 固定为 16px，避免 iOS/移动浏览器 focus zoom。
- Sidebar、Outline、Search Result 复用统一 interactive item state，保持 selected/hover/active/disabled 语义一致。

## Verified Surfaces

- Chrome MCP 375x812 mobile viewport smoke：AI Chat 展开、输入 focus、发送按钮、关闭返回、drawer 互斥。
- Chrome MCP 375x812 Diff fixture smoke：AI Chat/辅助键盘栏隐藏、关闭按钮视口内可用、关闭返回 editor。
- Chrome MCP 375x812 editor font-size smoke：focus 前后 `.cm-content` computed font-size 为 16px，`visualViewport.scale=1`。
- `scripts/check-mobile-baseline.sh` 守住 mobile editor 16px baseline。

## Retired Source Reports

- `mobile-ai-chat-keyboard-regression-status-2026-04-30.md`
- `mobile-ai-chat-viewport-smoke-2026-04-30.md`
- `mobile-diff-fixture-viewport-smoke-2026-04-30.md`
- `mobile-editor-font-size-baseline-2026-05-01.md`
- `mobile-editor-font-size-viewport-smoke-2026-05-01.md`
- `mobile-touch-feedback-status-2026-04-29.md`
