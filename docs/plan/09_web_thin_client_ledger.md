# 09_web_thin_client_ledger.md - Web Thin Client 写入确认工程蓝图

## Metadata

- `Layer`: `Runtime Protocols`
- `Status`: `Approved Runtime Architecture`
- `Counterpart Feature`: `docs/features/16_web_thin_client_ledger.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/06_network.md`, `docs/acceptance-cases/07_storage_repo.md`
- `Primary Code Areas`: `apps/web/src/hooks/use_core/pending*.rs`, `apps/web/src/hooks/use_core/effects/message_*.rs`, `apps/cli/src/server/handlers/document/edit*.rs`, `crates/core/src/protocol/`

## 1. Scope

本章定义 Web 端作为可写 thin client 时的工程合同：

- pending overlay
- ack / reject
- repo-scoped write readiness
- browser writer identity
- document navigation guard

本章是 `03_storage`、`07_network`、`05_diff_logic` 在 Web 写路径上的收敛说明。

## 2. Authoritative Entities

### 2.1 State Model

- `L_confirmed`
  - 服务端已确认并落 ledger 的状态
- `O_session`
  - 当前浏览器会话未确认 overlay
  - 不写入 `pending_fs_ops`
  - 不由 watcher / scan 产生或清理
- `V_web`
  - 当前页面展示结果

约束：

```text
V_web = Project(L_confirmed) + O_session
State_auth = L_confirmed
```

### 2.2 Write Identity

- `client_id`
- `client_op_id`
- `repo_id`
- `scope_nonce`
- `switch_nonce`

### 2.3 Write Readiness {#write-readiness}

写入就绪状态必须是 repo-scoped：

- `SnapshotReady(repo_id)`
- `HandshakeReady(repo_id, scope_nonce)`
- `WriterRegistered(repo_id)`

## 3. Invariants

1. ledger 是唯一 authority。
2. Web 不得持有“已确认业务状态”的私有副本。
3. `Auth for Write` 与 `Handshake for Sync` 必须分离。
4. 文件切换、重连、快照刷新都必须从 `confirmed + pending overlay` 重建。
5. commit/delete/merge 的最终成立条件必须回到 ledger append / ledger anchor。

## 4. Protocol Contract

### 4.1 Edit Intent {#web-edit-intent}

`ClientMessage::Edit` MUST 携带：

- `doc_id`
- `op`
- `client_id`
- `client_op_id`
- `scope_nonce`

#### `client_op_id` Rules

- type：`u64`
- 唯一性：`(client_id, client_op_id)`
- 生成：客户端本地单调递增
- 服务端必须以其作为幂等判定键

更细约束：

- `client_id`
  - 表示当前 browser writer instance / tab instance
  - tab reload 或 session rebuild 后可以变化
- `client_op_id`
  - 在单个 `client_id` 作用域内 MUST 从 `0` 或 `1` 起单调递增
  - MUST NOT 倒退
  - MUST NOT 跨 `client_id` 共享幂等记录
- 服务端在 reconnect window 内 MUST 保留 `(client_id, client_op_id) -> ack result` 去重记录

### 4.2 Ack Contract

`ServerMessage::Ack` MUST 回显：

- `repo_id`
- `doc_id`
- `seq`
- `client_op_id`

否则前端无法精确清理 pending overlay。

补充：

- 若 ledger 已提交但后续 workspace/projection 写回失败，仍然 MUST 返回能够标识该写入已确认的 `Ack`，并通过独立 fault 通道报告 writeback error。
- 同一个 `(client_id, client_op_id)` 重发时，服务端 SHOULD 返回第一次成功写入的同一 ack 内容，而不是再次落 ledger。

### 4.3 Reject Contract

`ServerMessage::EditRejected` MUST 至少携带：

- `doc_id`
- `client_op_id`
- `scope_nonce`
- 结构化错误

前端收到 reject 后必须把对应 pending edit 从 waiting 状态移除，而不是无限挂起。

拒绝分类：

- `AUTH_*`
  - 当前 session 或 write permission 无效
- `SC_STALE_SCOPE`
  - scope 已过期，当前 edit 不再属于有效 repo context
- `STORAGE_*`
  - append 前验证或持久化失败
- `DOC_*`
  - `doc_id` 无效或文档上下文已失效

### 4.4 Repo-Scoped Handshake

`ServerMessage::SyncHello` MUST 回显：

- `repo_id`
- `scope_nonce`

规则：

- 旧 repo 的延迟握手不得激活新 repo 的写闸门。
- `switch_nonce` 必须严格大于当前 `scope_nonce`。

#### `scope_nonce / switch_nonce` Rules

- type：`u64`
- per-connection authority
- reconnect 后重置
- stale switch 必须返回结构化错误

补充：

- `scope_nonce` 是当前已确认 repo scope 的权威版本。
- `switch_nonce` 是客户端发起 repo/branch switch 时声明的候选新版本。
- 只有 `switch_nonce > current scope_nonce` 才允许切换成功。
- 延迟到达的旧 `Ack` / `NewOp` / `Snapshot` 若 scope 不匹配，必须被丢弃或走 stale-scope recovery。

### 4.5 Structured Error Contract

- ws 与 http **MUST** 共享 `13_i18n.md#i18n-error-code-catalog` 的错误码目录。
- ws **MUST** 使用结构化 `ProtocolError`。
- source control 用户态错误 **MUST** 走 `SC_*`。
- 持久化失败才能进入 `STORAGE_*`。

## 5. Frontend State Machine

```text
Disconnected
Unauthorized
SnapshotLoading
HandshakePending(repo_id)
EditableConfirmed(repo_id)
PendingAck(doc_id, op_set)
Resyncing
```

可写条件必须满足：

```text
Editable iff SnapshotReady && HandshakeReady(current_repo)
```

而不是只要 snapshot ready。

详细转移：

- `Disconnected -> SnapshotLoading`
  - trigger: ws open + open doc request
- `SnapshotLoading -> HandshakePending(repo_id)`
  - trigger: snapshot ready, current repo context known
- `HandshakePending -> EditableConfirmed`
  - guard: `repo_id` matches current scope and handshake ack is fresh
- `EditableConfirmed -> PendingAck`
  - trigger: local edit accepted into pending overlay
- `PendingAck -> EditableConfirmed`
  - trigger: all pending ops acked or rejected
- 任意状态 -> `Unauthorized`
  - trigger: explicit auth failure, never by generic disconnect wrapper

## 6. Backend State Machine

```text
SessionVerified
  -> RepoBound
  -> WriterRegistered
  -> EditAccepted | EditRejected
  -> AckEmitted
```

必须显式区分：

- JWT-authenticated session
- repo binding
- browser writer identity
- sync peer identity

详细效果：

- `SessionVerified`
  - effect: request may enter repo-bound paths
- `RepoBound`
  - effect: `repo_id` + `scope_nonce` confirmed
- `WriterRegistered`
  - effect: browser writer is allowed to emit `Edit`
- `EditAccepted`
  - effect: append validated, authority write proceeds
- `EditRejected`
  - effect: no authority mutation for that op
- `AckEmitted`
  - effect: frontend may clear exact pending op

## 7. Pending Overlay Lifecycle

```text
LocalEdit
  -> PendingOverlayInserted
  -> AckMatched
  -> ConfirmedOverlayRemoved
```

失败旁路：

```text
LocalEdit
  -> PendingOverlayInserted
  -> EditRejected
  -> PendingOverlayRemoved
  -> NoticeShown
```

overlay state row 至少需要：

- `repo_id`
- `doc_id`
- `client_id`
- `client_op_id`
- `scope_nonce`
- `created_at`
- `op summary / pending marker`

### 7.1 Navigation Guard

- 离开文档前，必须按当前文档 pending set 判断是否可安全离开。
- `Continue` 只表示离开视图，不表示写入成功。
- 当最后一个 pending 被 ack 或 reject 清除时，navigation guard 必须同步解除。

判断规则：

- guard 只看当前文档 pending set，而不是全局任意 pending。
- `Continue` 只代表离开视图，不代表写入已经成功。
- `Stay` 必须保留当前 overlay，不得偷偷 discard。

### 7.2 Workspace Writeback Failure

- 如果 ledger 已提交但 workspace 写回失败，系统不得把该操作继续当成“等待 Ack”。
- 这类错误属于 projection / writeback fault，不属于 pending confirmation fault。
- 这类错误 **MUST NOT** 回滚 ledger append、重开 pending overlay 或写入 `pending_fs_ops`。

- 前端可以显示 warning / degraded notice
- 但必须把该 op 视为 confirmed，而不是无限 pending

## 8. Recovery / Reconcile Contract

### 8.1 Snapshot / History / NewOp Reconcile

- reconcile 识别 SHOULD 基于 `client_id + client_op_id` 或等价 origin metadata。
- 不得长期依赖“内容恰好相同”的弱判定。

### 8.2 Reconnect Recovery

- 重连后必须按当前 `repo_id` 重新握手。
- 旧 scope 的 ack / newop / snapshot 不得污染当前 repo scope。

补充：

- reconnect 后 SHOULD 根据 `client_id + client_op_id` 或 equivalent metadata 重建 overlay reconcile。
- 无法确认是否已提交的 op，不得默认静默丢弃；必须通过 resync 或 structured reject 收敛。

### 8.3 Stale Scope Recovery

- 当 persisted last scope 指向失效 repo 时，前端必须清理旧 scope 并重新 bootstrap。
- 不得卡在 `Repository context is invalid` 或 `scope mismatch` 的无限错误态。

## 9. Implementation Blueprint Reference

迁移顺序属于实施蓝图，不属于本章稳定权威合同。实施批次 **SHOULD** 参考 `docs/tasks/20_web_thin_client_ledger_migration.md`，但任何迁移步骤 **MUST** 继续满足本章的 protocol、pending overlay、scope gate 与 structured error 合同。

## 10. Forbidden Patterns

- 把编辑器 DOM 当前内容当成已提交事实
- 只因 snapshot ready 就解除只读
- 切文件时静默丢弃 pending local edits
- 让 pending_fs_ops / metadata / snapshot 充当删除真源
- 让 ws 和 http 走两套不同 commit 语义

## 11. Runtime Boundary

### 11.1 Protocol Layer

- 负责 write intent、ack/reject、scope-stamped server messages 与结构化错误 schema。
- 不得把 UI 展示状态编码成协议 authority。

### 11.2 Server Write Layer

- 负责 document open/snapshot/edit 的服务端权威提交与拒绝。
- 所有写入必须经过 repo scope、writer identity、readiness 与 ledger append 校验。

### 11.3 Web Runtime Layer

- 负责 pending overlay、client op id、late ack discard、navigation guard 与 reconnect recovery。
- 不得把 DOM buffer、localStorage 或 stale scope message 当成已确认事实。

## 12. Refactor Target

长期应显式形成：

- `browser_document_runtime`
- `pending_overlay_runtime`
- `write_confirmation_runtime`

实现必须把 editor sync、pending overlay、message dispatch 与 navigation guard 收敛到稳定 runtime 链路，不得依赖无边界 effects 协调写入确认。

## 13. Target Verification Criteria

1. Web 切文件不再出现“改动消失但无提示”
2. reject 后不再留下假 pending
3. reconnect 后旧 repo 握手不会污染当前 repo
4. `Unauthorized` 与 `Disconnected` 明确分离
5. 所有最终业务事实都可由 ledger 唯一追溯

补充成功标准：

6. `Ack` 与 `EditRejected` 都能精确清除对应 pending op
7. navigation guard 不再把“已拒绝”或“已确认但 writeback fault”的写入误判成 pending
8. repo/branch 切换后旧 scope 消息不会重新打开当前文档的 pending state

## 本章相关命令

- 无

## 本章相关配置

- 无
