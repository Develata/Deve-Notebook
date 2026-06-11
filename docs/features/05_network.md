# 05_network.md - 同步与连接体验篇

本章描述连接、同步、重连与只读降级的用户可见行为。

## 功能目标

用户应当能够明确感知：

- 当前是否已连接到服务端
- 当前是否可以继续编辑
- 当前看到的是哪个 repo scope 下的同步状态
- 断连、重连、会话失效分别意味着什么

## 功能项

### 1. 初次连接

- 页面打开后应建立到当前服务端的连接。
- 连接成功后，用户能看到可用的 repo、文档树与状态栏同步状态。

### 2. 断连与重连

- 网络断开后，界面必须明确进入重连状态。
- 重连期间不能伪装成“仍然可写”。
- 重连成功后，当前 repo scope 应恢复到可用状态。

### 3. WebLightPeer 写入边界

- Web 端在线且 repo write-ready 时可以编辑。
- 断连后必须进入只读或受限状态。
- 不能把“页面还开着”误导成“仍然能安全写入”。

### 4. Repo-Scoped 同步

- 同步状态必须与当前 repo 绑定。
- 切换 repo 后，用户看到的文档、树、同步状态与错误提示都应随 repo 更新。
- 一个 repo 的异常不应污染另一个 repo 的会话状态。

### 5. Unauthorized 与 Disconnected 分离

- 未授权不是“普通断网”。
- 用户应能从提示上区分：
  - 网络断开
- 会话失效
- 权限不足

### 6. Full Peer Mesh v1

- 多个服务端可以通过静态配置组成 P2P mesh。
- Browser/WebLightPeer 仍然只连接当前服务端；server-to-server 同步使用 FullPeer `/ws` admission。
- 用户或运维者应能看到 peer 连接是 configured、connected、reconnecting、unauthorized 还是 disabled。
- `/api/node/role` 的 P2P 摘要可用于只读诊断：展示 peer label、peer/repo id、连接状态、attempt/handshake 计数与 last error code，但不暴露 token env 内容或 token material。
- 重复 peer label 只影响显示，不得导致 `/api/node/role` 中不同 peer 的连接状态、attempt 或 last error 互相覆盖。
- 普通重连尝试不应清空上一条 last error code；只有连接/同步成功或配置重新初始化后，诊断面才应清除它。
- 静态 peer 配置中的 `peer_id` 是 expected authenticated peer identity，不是显示 label；握手返回的 peer id 不一致时必须拒绝连接。
- FullPeer connector 必须验证对端 `SyncHello` 的 pubkey、peer id 与 signature；无效签名或 pubkey 无法推出声明 peer id 时必须拒绝连接。
- `peer_id` mismatch 属于确定性身份/配置错误，不应作为普通断线持续重连；诊断中应保留 last error code。
- 静态 `repo_id` / `ws_url` 无效或 outbound token env 缺失/为空时，也应停在 error/unauthorized 诊断态，而不是持续重连。
- FullPeer 收到的 request / snapshot request 只能请求本次 handshake diff 中本端向对端声明可发送的 source；请求未 offer 的 source 必须被拒绝，并在 connector 诊断中作为 `unoffered_source` 终止错误暴露。
- 静态 FullPeer v1 不 advertise 本机缓存的第三方 shadow source；这些 source 在没有 origin proof retention 前不可由当前节点重新证明。
- FullPeer 收到的 push / snapshot push 只能按 authenticated peer 或有效 source proof 写入对应 shadow；伪造另一个 source peer 的 payload 必须被拒绝，并在 connector 诊断中作为 `source_proof_rejected` 终止错误暴露。
- FullPeer connector 与普通 Server WS sync handler 必须复用 core 的 source attribution/proof 校验规则，避免两条入站路径对同一 payload 给出不同结论。
- peer A 通过 Source Control commit 确认的本地投影变更，应在下一次 FullPeer handshake/diff 后出现在 peer B 的 peer A shadow 中；peer B 本地 branch 仍保持不变，直到用户显式 merge/import。
- Mesh v1 不做自动发现、NAT 穿透或自动拓扑修复。

### 7. Shadow 与显式合并边界

- 远端 peer 写入成功同步后，必须先落到该 peer 的 shadow repo。
- 本地当前 branch 不得因为“同步成功”而被隐式污染。
- 用户必须通过显式 merge/import 流程把远端 shadow 内容合并到本地 branch。
- 断线重连后应重新握手并对齐 vector；对齐成功只说明 shadow 可读，不代表自动合并完成。

## Operation 示例

- Repo-scoped sync handshake 原子操作示例见 `docs/features/operations/net_sync_handshake.md`。
- 该示例将 runtime 握手拆为恢复当前 repo runtime、发送 `SyncHello`、接收 `SyncHello Ack`、接收 `WriteReady` 四段。
- Repo-scoped sync transfer 原子操作示例见 `docs/features/operations/net_sync_transfer.md`。
- 该示例将传输链拆为请求缺失增量、接收 `SyncPush`、请求 snapshot fallback、接收 `SyncPushSnapshot` 四段。
- Repo-scoped key exchange 原子操作示例见 `docs/features/operations/net_key_exchange.md`。
- 该示例将 key 获取拆为发送 `RequestKey`、接收 `KeyProvide`、接收 `KeyDenied` 三段。

## 非目标

- 当前阶段不要求 Web 端离线可写。
- 当前阶段不要求浏览器持有完整 ledger。
- 当前阶段不通过显示层直接操控同步核心逻辑。
- 当前阶段不要求 P2P 自动发现、NAT 穿透、自动 local merge 或移动端后台长时同步。

## Chrome MCP 验收实例

### NET-FEAT-01: 初次连接与状态可见性

前置条件：

- 服务端已启动，浏览器打开应用首页。

步骤：

1. 等待页面完成初始加载。
2. 观察状态栏、repo 内容与文档树。

期望结果：

- 页面显示已连接状态。
- 当前 repo 的树与文档可见。
- 没有误导性的断连提示。

### NET-FEAT-02: 断连与重连

前置条件：

- 页面已连上服务端，当前处于可编辑 repo。

步骤：

1. 模拟后端暂时不可达。
2. 观察页面状态变化。
3. 恢复服务端可达。

期望结果：

- 页面进入明确的重连/受限状态。
- 重连后状态恢复，不需要用户手动刷新才能继续。

### NET-FEAT-03: Repo-Scoped 同步隔离

前置条件：

- 存在至少两个 repo 或本地/远端两种 scope。

步骤：

1. 在当前 repo 观察同步状态。
2. 切换到另一个 repo 或远端分支。
3. 再切回原 repo。

期望结果：

- 同步状态始终跟随当前 repo scope。
- 不出现跨 repo 的脏状态继承。

### NET-FEAT-04: 多服务端 Mesh Shadow 可见

前置条件：

- 两个服务端使用同一 `RepoId` 与静态 peer 配置启动。

步骤：

1. 在 peer A 的本地 branch 创建或编辑文档。
2. 等待 peer B 的 mesh 状态进入 connected。
3. 在 peer B 查看来自 peer A 的 shadow 内容。
4. 在 peer B 执行显式 merge/import，再查看本地 branch。

期望结果：

- peer B 可以看到 peer A 的 shadow 更新。
- 显式 merge 前，peer B 的本地 branch 不被自动修改。
- 断线重连后状态重新进入 connected，vector 对齐不要求刷新页面。
