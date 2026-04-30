# Mobile AI Chat Keyboard Regression Status - 2026-04-30

## 结论

Post-queue plan/code drift rescan 没有发现 P0/P1/P3 gate 重新打开项；实际发现的下一批具体缺口是移动端 AI Chat 键盘态回归：`MobileChatSheet` 在 `keyboard_offset > 0` 时会隐藏整个 chat sheet，导致已展开聊天页在软键盘弹起时不可见。

## 修复

- 展开态 chat sheet 现在在键盘弹起时保持可见。
- 折叠态 `AI` chip 仍会在键盘弹起时隐藏，避免遮挡输入。
- Drawer 或 Diff 打开时继续隐藏 mobile chat，维持层级冲突治理。
- 展开态根据 `keyboard_offset` 设置 bottom offset，使输入区和发送按钮避开软键盘。

## 覆盖的验收口径

- `UI-MOB-011`: AI Chat 移动端展开为同页全屏页面，并可关闭返回原页面。
- `UI-MOB-012`: AI Chat 输入区在键盘弹起时可见且发送按钮可达。
- `REG-MOB-017`: Drawer 打开时 AI Chat 不叠层显示。
- `REG-MOB-023`: 移动端 Diff 打开时不显示 AI Chat 入口。

## 验证

- `cargo test -p deve_web chat_sheet::tests -- --nocapture`
- `scripts/check-acceptance-bindings.sh`
- `scripts/check-dev-runbook-baseline.sh`
- `git diff --check`
