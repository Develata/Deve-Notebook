# 16_web_thin_client_ledger.md - Web Thin Client 工程蓝图

本章只定义 Web 写入确认链、repo-scoped write readiness 与 pending lifecycle 的工程合同，不描述用户提示文案。功能语义见 [../features/16_web_thin_client_ledger.md](../features/16_web_thin_client_ledger.md)，自动化验收见 [../acceptance-cases/06_network.md](../acceptance-cases/06_network.md)。

## 1. 目标

- Web 是可写 thin client，但 authority 仍在 server ledger。
- 前端只允许持有 `confirmed + pending overlay`，不允许持有第二真相。
- repo-scoped handshake 与 write readiness 必须显式建模。

## 2. 权威实体

- `L_confirmed`
  - 服务端已确认并落 ledger 的状态。
- `O_session`
  - 当前浏览器会话未确认 overlay。
- `client_op_id`
  - 浏览器本地写入意图唯一标识。
- `scope_nonce`
  - 当前 scope 代次。

## 3. 核心视图公式

```text
V_web = Project(L_confirmed) + O_session
```

其中：

- `L_confirmed` 是唯一权威。
- `O_session` 只用于交互连续性。

## 4. 写入协议合同

- `Edit` 必须携带：
  - `doc_id`
  - `client_id`
  - `client_op_id`
- `Ack` 必须回显：
  - `doc_id`
  - `seq`
  - `client_op_id`
- `EditRejected` / 结构化失败必须足够精确，能让前端回收对应 pending。

## 5. Repo-Scoped Handshake

- `SyncHello` 必须回显 `repo_id`。
- `switch_nonce` 必须严格大于当前 `scope_nonce`。
- stale scope 恢复失败时，必须清掉旧 scope 并重新请求健康 repo 状态。

## 6. 前端状态机

- `Disconnected`
- `Unauthorized`
- `SnapshotLoading`
- `HandshakePending(repo_id)`
- `EditableConfirmed(repo_id)`
- `PendingAck`
- `Resyncing`

### 可写条件

```text
Editable iff SnapshotReady && HandshakeReady(current_repo)
```

## 7. Pending Lifecycle

- `LocalIntentCreated`
- `PendingOverlayVisible`
- `Acked -> Confirmed`
- `ExplicitReject -> FailedAndCleared`
- `ResyncRebuilt -> OverlayReconciled`

导航拦截、切文件、重连恢复都必须基于这条生命周期，而不是只看 DOM 当前内容。

## 8. 失败合同

- ledger append 成功但 workspace writeback 失败：
  - authority 视为已提交
  - 前端不应永久卡在假 pending
- explicit reject：
  - 必须清理对应 pending
  - 不得继续显示“等待服务端确认”
- stale repo scope：
  - 必须解绑旧 scope，重新进入健康 repo 恢复链

## 9. 禁止事项

- 禁止把快照完成误当成写入 ready。
- 禁止只靠字符串错误提示驱动状态机。
- 禁止切文件时静默丢弃未确认写入。
- 禁止让 pending overlay 升格为已确认业务真相。

## 10. 代码边界

- `apps/web/src/hooks/use_core/`
  - document runtime、pending ops、scope/session runtime。
- `apps/cli/src/server/handlers/document/`
  - edit/open/ack/reject authority path。
- `crates/core/src/protocol/`
  - client/server message contract。
