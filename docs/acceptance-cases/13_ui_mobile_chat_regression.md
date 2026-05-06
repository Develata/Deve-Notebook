## UI 移动端 AI Chat 最小回归脚本

```markdown
- case_id: UI-MOB-CHAT-REG-001
  goal: AI Chat 移动端展开为全屏页面，并可通过右上角关闭返回原页面。
  preconditions:
    - trunk serve 已启动
    - 浏览器视口 375x812
  steps:
    - run: scripts/check-mobile-baseline.sh
    - run: cargo test -p deve_web mobile_chat_page -- --nocapture
    - ui_click: "mobile_chat_chip"
    - ui_assert: chat_page_fullscreen true
    - ui_click: "chat_close_button"
  assertions:
    - cli_assert: mobile_chat_chip_marker_bound true
    - cli_assert: mobile_chat_close_marker_bound true
    - cli_assert: mobile_chat_fullscreen_marker_bound true
    - cli_assert: mobile_chat_page_state_transition_bound true
    - ui_assert: chat_page_fullscreen false
    - ui_assert: editor_visible true

- case_id: UI-MOB-CHAT-REG-002
  goal: 键盘弹起时输入区和发送按钮可见，Bottom Bar 不冲突。
  preconditions:
    - AI Chat 已展开
  steps:
    - run: scripts/check-mobile-baseline.sh
    - run: cargo test -p deve_web mobile_chat_keyboard -- --nocapture
    - ui_focus: "chat_input"
    - ui_wait_keyboard: true
    - ui_measure: "chat_send_button"
  assertions:
    - cli_assert: mobile_chat_input_marker_bound true
    - cli_assert: mobile_chat_send_button_marker_bound true
    - cli_assert: mobile_chat_keyboard_offset_bound true
    - cli_assert: mobile_chat_bottom_bar_hidden_bound true
    - ui_assert: chat_input_not_overlapped_by_keyboard true
    - ui_assert: min_target_size "44x44"
    - ui_assert: bottom_bar_hidden true

- case_id: UI-MOB-CHAT-REG-003
  goal: 错误态与重试态闭环。
  preconditions:
    - 当前 AI 后端返回错误
  steps:
    - ui_send_chat_text: "trigger_error"
    - ui_assert: chat_error_banner_visible true
    - ui_click: "chat_retry_button"
  assertions:
    - ui_assert: retry_action_triggered true
```
