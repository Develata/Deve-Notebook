# 20_web_thin_client_ledger_migration.md - Web Thin Client Ledger 迁移顺序

本文是 `docs/plan/09_web_thin_client_ledger.md` 的实施蓝图，不是独立权威 plan。若与 plan 冲突，以 plan 为准。

## 1. 迁移顺序

1. 所有编辑操作引入稳定 `client_op_id`。
2. `Ack` 回显 `client_op_id`。
3. `EditRejected` 回显 `doc_id + client_op_id`。
4. repo-scoped handshake 显式化。
5. pending set 与 navigation guard 精确绑定。
6. `History / NewOp / Snapshot delta` 补齐 origin metadata。
7. WS / HTTP / Source Control 写入口统一回到同一 ledger authority。

## 2. 验证目标

1. Web 切文件不再出现“改动消失但无提示”。
2. reject 后不再留下假 pending。
3. reconnect 后旧 repo 握手不会污染当前 repo。
4. `Unauthorized` 与 `Disconnected` 明确分离。
5. 所有最终业务事实都可由 ledger 唯一追溯。
6. `Ack` 与 `EditRejected` 都能精确清除对应 pending op。
7. navigation guard 不再把“已拒绝”或“已确认但 writeback fault”的写入误判成 pending。
8. repo/branch 切换后旧 scope 消息不会重新打开当前文档的 pending state。
