# WS port query validation

日期：2026-05-14

## 结论

Web runtime 的 `?ws_port=` override 现在只接受有效非零 `u16` 端口。

## 变更

- 新增 `parse_ws_port`，集中处理 URL query 中的 `ws_port` 值。
- 拒绝 `0`、空字符串、非数字与越界端口，避免生成无效 WebSocket candidate。
- Network baseline 绑定解析器和边界测试。

## 验证

- `cargo test -p deve_web query_ws_port -- --nocapture`
