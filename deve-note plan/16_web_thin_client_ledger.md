# 16. Web Thin Client & Ledger Confirmation

**Status**: Approved target architecture
**Scope**: WebLightPeer write path, editor confirmation, repo-scoped handshake, Source Control convergence

## 1. Goal

本设计将 Web 端明确收敛为 **可写的薄客户端 (Thin Client)**，同时保持 **Server Ledger 为唯一真值源**。

目标不是把 Web 降级成“显示屏”，而是建立如下权威关系：

- Web 可以发起写入意图。
- Server 决定写入是否成立。
- 只有成功追加到 Ledger 的结果才是业务真相。
- 前端任何未确认状态都只是短暂 Overlay，不得升格为权威数据。

## 2. Authoritative State Model

记：

- `L_confirmed` = 服务端已确认并落 Ledger 的状态
- `O_session` = 当前浏览器会话的未确认本地操作 Overlay
- `V_web` = Web 端展示给用户的内容

则前端视图必须满足：

```text
V_web = Project(L_confirmed) + O_session
```

并且：

```text
State_auth = L_confirmed
```

其中 `O_session` 仅用于交互连续性，绝不是第二真源。

## 3. Non-Negotiable Invariants

1. Ledger 是唯一权威状态。
2. Web MUST NOT 持有“已确认业务状态”的私有副本。
3. `Auth for Write` 与 `Handshake for Sync` MUST 分离建模。
4. 写入就绪状态 MUST 是 repo-scoped，而不是 connection-scoped 猜测值。
5. 文件切换、重连、快照刷新都 MUST 从 `confirmed + pending overlay` 重建。
6. Commit、Delete、Merge 的最终成立条件 MUST 是 ledger append / ledger anchor，而不是 metadata 或 snapshot 副作用。

## 4. Mandatory Protocol Rules

### 4.1 Edit Intent

`ClientMessage::Edit` MUST 携带：

- `doc_id`
- `op`
- `client_id`：客户端实例标识
- `client_op_id`：该次写入意图在客户端内的唯一标识

### 4.2 Commit Acknowledgement

`ServerMessage::Ack` MUST 回显：

- `doc_id`
- `seq`
- `client_op_id`

这样前端才能把某个 Pending Overlay 精确降格为 Confirmed State。

### 4.3 Repo-Scoped Handshake

`ServerMessage::SyncHello` MUST 回显 `repo_id`。

原因：

- 写入闸门必须严格绑定当前仓库。
- 旧仓库的延迟握手消息不得把新仓库误标成“可写”。

### 4.4 Structured Error Contract

错误协议必须满足：

- `WebSocket` 与 `HTTP` MUST 共享同一错误码目录与同一 `code + optional detail` 结构。
- `WebSocket` MUST 使用结构化 `ProtocolError { error }`；不得新增 `Error(String)` 作为实现目标。
- `Source Control` 用户态错误 MUST 使用独立 `SC_*` 目录。
- 仅 DB/Vault/FS 持久化失败可继续落到 `STORAGE_*`。

## 5. Frontend State Machine

Web 编辑器必须至少区分以下状态：

1. `Disconnected`
2. `SnapshotLoading`
3. `HandshakePending(repo_id)`
4. `EditableConfirmed(repo_id)`
5. `PendingAck(doc_id, client_op_id set)`
6. `Resyncing`

其中可写条件必须是：

```text
Editable iff SnapshotReady && HandshakeReady(current_repo)
```

而不是仅凭 Snapshot Ready。

## 6. Backend State Machine

服务端对浏览器写入必须区分：

- `JWT-authenticated session`
- `repo binding`
- `browser writer identity`
- `P2P sync peer identity`

长期目标是：

- 本地浏览器写入路径不再复用纯 P2P 的 `authenticated_peer_id` 作为唯一准入条件。
- `SyncHello` 负责同步协商。
- 浏览器写入身份负责本地 op 归属与确认。

## 7. Required Migration Order

1. 为每个编辑操作引入 `client_op_id`，并让 `Ack` 回显它。
2. 让握手完成状态显式化、repo-scoped 化，并作为编辑器写闸门的一部分。
3. 前端建立 `pending local op -> acked op` 的精确集合，而不是只改 DOM。
4. 文件切换前检查未确认写入；不得静默丢弃。
5. 统一 WS / HTTP / Watcher / SC 的最终 ledger 写入口。
6. 删除语义改为显式 tombstone / delete op。

## 8. Forbidden Patterns

以下做法在本设计下视为错误：

- 把编辑器 DOM 当前内容当成已提交事实。
- 只因快照加载完成就解除只读。
- 用通用 `Error(String)` 或 HTTP 裸文本错误替代结构化确认/错误协议。
- 切文件时丢弃未确认本地改动而不提示。
- 让 `pending_fs_ops`、metadata、snapshot 充当删除真源。
- 让 WS 与 HTTP 走两套不同的 Commit 语义。

## 9. Compatibility Notes

- 现有 `ledger-first` 设计原则保持不变。
- 本文档是对 `01_terminology.md`、`04_storage.md`、`05_network.md`、`07_diff_logic.md` 在 Web 写入模型上的收敛说明。
- 若未来协议需要新增 `OpRejected`、`WriteReady` 等更细粒度消息，应视为对本文档的自然延伸，而不是另起架构。

## 10. Success Criteria

当以下条件全部成立时，认为迁移完成：

1. Web 切文件不再出现“改动消失但无提示”。
2. Web 重连后不会把旧仓库握手错误地当成当前仓库可写。
3. 前端能精确区分 `pending` 与 `confirmed` 本地改动。
4. 所有最终业务事实都可由 Ledger 唯一追溯。
5. Source Control 与编辑器写入都回到同一权威模型。
