# WS HTTP base derivation boundary

日期：2026-05-14

## 结论

Web node-role probe 的 WS-to-HTTP base 推导已从全局字符串替换改为显式前缀与尾部 path 处理。

## 变更

- `http_base_from_ws_url` 只重写开头的 `ws://` / `wss://` scheme。
- 只在 query/fragment 分隔符之前移除 path 末尾精确 `/ws`，不改写 query/path 中出现的 `ws://` 文本。
- Native boundary baseline 绑定该测试，防止恢复为全局 `replace`。

## 验证

- `cargo test -p deve_web http_base_from_ws_url -- --nocapture`
