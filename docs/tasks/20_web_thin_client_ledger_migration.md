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

## 1.1 Client Runtime Boundary

Web 端迁移目标必须使用 client runtime 命名与边界：

* `document_client` 只管理 pending overlay、`client_op_id`、ack/reject 显示状态与 navigation guard。
* `scope_client` 只管理当前 repo/branch/doc scope、`scope_nonce` 与 stale-scope recovery。
* `session_client` 只管理 transport/session readiness，不保存业务真相。
* `source_control_client` 只发出 source-control typed intent，不拥有 stage/commit authority。
* `rendering_client` 只封装 editor/Markdown/KaTeX/DOM object adapters。

这些 client runtime 均属于 Flow Coordination 或 Object Plane adapter；最终业务事实只能由 server/core ledger authority 追溯。

### 1.2 `use_core` Typed Runtime State Hardening

`use_core` 作为当前 Web application-control composition root，可以在不改变 WS wire
shape、不 bump `WS_PROTOCOL_VERSION` 的前提下，把内部 UI/runtime 状态收紧为命名类型：

* load phase、sync mode、AI backend mode 必须在 Web 内部以 closed typed state 表达，wire
  string 只允许出现在入站/出站转换边界。
* full-text search result 与 pending ops preview 必须使用具名结构，禁止在 runtime state 中继续传播裸三元组。
* repo / branch pending switch 必须把 target 与 `switch_nonce` 作为同一 pending value 更新，避免消息 effect 观察到 target/nonce 半更新组合。
* 本 hardening 不改变 ledger authority、pending overlay、source-control confirmed ledger dirty
  语义，也不改变 server/core 协议字段。

## 2. 验证目标

1. Web 切文件不再出现“改动消失但无提示”。
2. reject 后不再留下假 pending。
3. reconnect 后旧 repo 握手不会污染当前 repo。
4. `Unauthorized` 与 `Disconnected` 明确分离。
5. 所有最终业务事实都可由 ledger 唯一追溯。
6. `Ack` 与 `EditRejected` 都能精确清除对应 pending op。
7. navigation guard 不再把“已拒绝”或“已确认但 writeback fault”的写入误判成 pending。
8. repo/branch 切换后旧 scope 消息不会重新打开当前文档的 pending state。
