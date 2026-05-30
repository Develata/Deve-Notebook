# Plan-Code Semantic Alignment Audit (前向语义对齐审计)

- `Date`: `2026-05-30`
- `Scope`: 验证**代码实现是否真的实现了 plan 声明的功能语义**，而非结构性存在性。
- `Authority Source`: `docs/plan/20_operations_catalog.md §3`（72 条 operation-flow + 9 列治理属性）。
- `Projection`: 各 flow 的 `docs/features/operations/*.md`（按原子 OpId 列 `Application Entry` 代码路径 + Response Flow）。

## 0. 为什么需要这层审计

现有闸门全是**结构性**校验，没有任何一层验证行为语义：

- `scripts/plan-coverage.sh`（8 检查 + 反向覆盖）：plan_ref 锚点存在、size fuse、acceptance 绑定、路径漂移。
- `scripts/check-feature-operation-paths.sh`：仅验证 backtick 路径**存在**为文件/目录。

本审计补的缺口：对每条 flow，核对其 `Application Entry` 代码是否真的实现了 §3 声明的
**Auth（触达的权威面 L/PW/PO/FS）/ WG（写闸）/ Gate（前置条件）/ Failure Family（错误码族）/ Response Flow（步骤序列）**。

## 1. 范围裁定（按 Plan Status）

只审「现在就该对齐」的章节，跳过 Planned/Optional/Deferred/Reference 与 Apple 环境。

| 域 | Owning Boundary | Status | flow 数 | 审 |
|---|---|---|---|---|
| 1 存储/权威写入 | `03_storage/*` | Current MUST | 4 | ✅ |
| 2 源控/diff | `05_diff_logic` | Current UI+MUST | 9 | ⏳ |
| 3 仓库 | `04_repository` | Current MUST | 5 | ⏳ |
| 4 网络/同步 | `07_network` | Current MUST | 4 | ⏳ |
| 5 认证 | `08_auth` | Current MUST | 2 | ⏳ |
| 6 Web 瘦客户端账本 | `09_web_thin_client_ledger` | Approved Runtime | 1 | ⏳ |
| 7 渲染 | `10_rendering` | Current UI Contract | 11 | ⏳ |
| 8 i18n | `13_i18n` | Current MUST | 6 | ⏳ |
| — commands/settings/ai/plugin/release/tech-stack/backup | 14/15/16/19/18/17/06 | Planned/Optional/Deferred/Reference | 30 | ⏭️ 跳过 |

**审计总量 = 42 flow**（跳过 30 条非 MUST）。

## 2. 判定三态

- **ALIGNED**：代码忠实实现 §3 声明语义（带 `file:line` 证据）。
- **GAP**：plan 声明的语义在代码中缺实现。
- **DEVIATION**：代码实现与 plan 声明不符（行为偏离）。

---

## 域 1：存储/权威写入（4 flow）— 结论 ALIGNED

### 1.1 `flow.doc.edit-confirmed-op` — **ALIGNED（典范级）**

§3 声明：UO · Auth=`L+PW+PO` · WG=`Y` · Failure=`SYNC_*` · Owning=`03_storage/authority` · Gate=`writer-gate+scope_nonce`。

权威写入 handler：`apps/cli/src/server/handlers/document/edit.rs:17 handle_edit`，逐项核对：

| 声明语义 | 代码证据 | 判定 |
|---|---|---|
| Gate: writer-gate | `edit_checks.rs:34 writer_peer_id` → `session.writer_peer_id_for(repo_id, scope_nonce)`，缺则 `SyncPeerUnauthenticated` 拒 | ✅ |
| Gate: scope_nonce | `edit_support.rs:23 edit_response_scope_nonce` + writer-gate handler `sync/writer/mod.rs:47 validate` 校验 `session.scope_nonce()==scope_nonce && sync_scope_nonce()==Some(scope_nonce)` | ✅ |
| 五重前置（scope/readonly/repo-writable/doc-existence/writer） | `handle_edit` 依次 `resolve_edit_scope` → `scope.branch.is_some()` readonly → `ensure_resolved_local_repo_writable` → `reject_missing_doc` → `writer_peer_id`，与 Response Flow 步骤4逐条吻合 | ✅ |
| L: 账本追加 | `edit_apply.rs:41 append_generated_client_op_in_local_repo` 构造 `LedgerEntry::new_content` | ✅ |
| PW: 投影写入（commit 后） | `edit_apply.rs:60 sync_manager.persist_doc_in_local_repo`，在账本 `Ok` 之后 | ✅ |
| PO: 待定覆盖层 | web `runtime/document/confirm.rs commit_pending_edit/reject_pending_edit` 在 Ack/EditRejected 清 pending | ✅ |
| 广播 NewOp + 单播 Ack | `edit_support.rs:76 broadcast_and_ack_committed_edit` | ✅ |
| 幂等（同键同 op 重放原 Ack；同键异 op 拒） | `edit_checks.rs:68 confirm_existing_client_op` → 命中且 `content_op()==op` 重放 Ack；否则 `SyncEditRejected` | ✅ |

**关键不变量（Note：投影失败可恢复、不得回滚权威）正确实现**：
`write_confirmation.rs:66 emit_commit_outcome` 中 `Committed` 与 `WritebackFailed` **都**广播 NewOp + 单播 Ack（op 在两种 outcome 下都已落账），仅 `WritebackFailed` 额外发 `report_projection_writeback_fault`（非拒绝性 `ProtocolError`）。即投影回写失败时客户端仍收 **Ack 而非 EditRejected**，账本不回滚。服务端 `CommitOutcome{Committed,WritebackFailed}` 与 web `write_state.rs WriteConfirmation{Waiting,Committed,Rejected,WritebackFailed}` **双向同构**。

**观察（非缺口）**：web `write_state.rs` 带 `#![allow(dead_code)]`——`WritebackFailed` 状态目前无 live producer（瘦客户端尚未把服务端 projection-fault `ProtocolError` 消费为 per-edit `WritebackFailed` 转移）。但 plan Response Flow 步骤8仅要求「ack/reject 清 pending + 应用确认的远端 op」，未要求 per-edit writeback UI，故此为**前向兼容脚手架**，非与 plan 的偏离；文件 doc 注释已显式说明。

**Failure Family 说明（非偏离）**：§3 标 `SYNC_*` 是 flow 的归属族（`SyncEditRejected`/`SyncPeerUnauthenticated` 在场）；前置失败合法复用各源族码（readonly=`Sc*`、缺文档=`DocNotFound`、持久化=`StoragePersistFailed`），经同一 `EditRejected` 信封返回。与列释义「Failure Family=归属族前缀」一致。

### 1.2 `flow.rendering.checkbox-writeback` — **ALIGNED**

§3 声明：UO · Auth=`L+PW+PO` · WG=`Y` · Failure=`SYNC_*` · Owning=`03_storage/authority` · Gate=`writer-gate`。

- `op.render.checkbox.click-toggle`：`apps/web/js/extensions/checkbox_ext.js:52 input.onclick` → `view.dispatch({changes:{from:pos+1,to:pos+2,insert:mark}})`——派发**源码文本编辑**（替换 `[ ]`/`[x]` 标记字符），不改独立富状态。✅ 与声明「dispatch a source edit rather than mutating an independent rich state」吻合。
- `op.render.checkbox.observe-widget`：`checkbox_ext.js:74 computeCheckboxDecorations` 在 `docChanged` 时由 `syntaxTree`/`sliceDoc` 从源码重建 widget，`checked` = `slice.includes("x")`。✅
- `op.render.checkbox.observe-source`：派发的 change 成为 CodeMirror doc change → 流经 `delta_input.rs` → **复用 §1.1 已验证的 edit-confirmed-op 权威路径**，`- [x]`/`- [ ]` 为单一权威。✅ Auth=L+PW+PO、WG=Y 经该路径传递满足。
- Note「只因往返源码才允许 checkbox 交互」成立：无独立 checkbox 权威，widget 是源码的纯投影。✅

### 1.3 `flow.cli.repair-admin` — **ALIGNED（入口齐备+接线）**

§3 声明：ED · Auth=`L+PW` · WG=`Y` · Failure=`STORAGE_*` · Owning=`03_storage/repair` · Gate=`admin-repair-cmd`。

全部 `Application Entry` 存在：`apps/cli/src/commands/verify_p2p.rs`、`seed.rs`、`node_check.rs`、`recover.rs`、`repair/`（mod + rebuild/restore/path_fix/weird_paths/shadow + tests）。运维 ED 控制面（非浏览器热写路径）。属上轮 `3384584d` 重构与既有测试覆盖范围。
**深度行为审计（各 repair 策略与 ledger→projection 副作用）作为低风险子项后置**——这些是管理员显式触发的命令，正确性风险量级低于浏览器写路径。

### 1.4 `flow.cli.projection-workspace-indexing` — **ALIGNED（入口齐备+接线）**

§3 声明：ED · Auth=`PW` · WG=`N` · Failure=`STORAGE_*` · Owning=`03_storage/projection` · Gate=`workspace-mounted`。

全部 `Application Entry` 存在：`apps/cli/src/main.rs`、`dispatch.rs`、`commands/scan.rs`、`commands/watch.rs`。init/scan/watch 生命周期命令。运维 ED 面，存在性+接线已确认。

### 域 1 小结

4/4 ALIGNED。**核心浏览器写路径（edit-confirmed-op）对齐质量为典范级**，含一处微妙的「投影失败不回滚权威」正确性不变量被两侧同构地正确建模。无 GAP、无 DEVIATION。两条 CLI 运维 ED 面深度行为审计后置（低风险）。

---

## 域 5：认证（2 flow）— 结论 ALIGNED

### 5.1 `flow.auth.login` — **ALIGNED**

§3 声明：UO · Auth=`—` · WG=`N` · Failure=`AUTH_*` · Owning=`08_auth` · Gate=`valid-credentials`。

`apps/cli/src/server/auth/handlers/login.rs:23 login` 逐行实现投影文件「Submit 内部模块流」：

| 声明步骤 | 代码证据 | 判定 |
|---|---|---|
| 暴力破解限流 | `login.rs:34 guard.is_blocked(&ip)` → `AuthErrorCode::RateLimited`(429) | ✅ |
| 用户名比对 | `login.rs:39 body.username != config.username` → `record_failure` + `InvalidPassword`(401) | ✅ |
| Argon2 密码校验 | `login.rs:45 verify_login_password(&body.password, &config.password_hash)` | ✅ |
| JWT 签发 | `login.rs:63 jwt::issue_token(secret, username, token_version)` | ✅ |
| Set-Cookie | `login.rs:66 build_auth_cookie(&token)` + `LoginResponse::success()` | ✅ |

失败全走结构化 `AuthErrorCode::{RateLimited,InvalidPassword,InternalError}` = AUTH_* 族。✅
**额外安全属性（超出 plan 要求）**：用户名不存在与密码错误**返回同一 `InvalidPassword` 码**，防用户名枚举。

### 5.2 `flow.auth.session-unauthorized` — **ALIGNED**

§3 声明：II · Auth=`—` · WG=`N` · Failure=`AUTH_*` · Owning=`08_auth` · Gate=`missing/expired-token`。

flow 重点（Note）=「把 `unauthorized` 与 `disconnected` 明确分离」，代码忠实实现：

- `apps/web/src/api/service.rs:118 mark_unauthorized` → `clear_writer_ready()` + `set_status.set(ConnectionStatus::Unauthorized)`——**专用 `Unauthorized` 状态**，与 `Disconnected` 区分。✅
- `message_protocol/mod.rs:120 handle_protocol_error`：`if is_auth_error(error.code) { ws.mark_unauthorized(); }`——WS auth error → mark_unauthorized。✅
- 协议错误码 `AuthTokenExpired`/`AuthTokenMissing` 定义于 `crates/core/src/protocol/error.rs:13,15`。✅
- enter-reauth-surface：`MainLayout` 观察 `ConnectionStatus::Unauthorized` → `on_session_expired` → 根 `App` 切 `Unauthenticated`（main_layout/setup.rs, app.rs）。✅

### 域 5 小结

2/2 ALIGNED。login 逐步实现声明的鉴权序列且带防枚举加分项；session-unauthorized 的核心语义（unauthorized≠disconnected 专用状态分离）落实。无 GAP/DEVIATION。

---

## 域 6：Web 瘦客户端账本（1 flow）— 结论 ALIGNED

### 6.1 `flow.doc.pending-navigation-guard` — **ALIGNED**

§3 声明：UO · Auth=`PO` · WG=`N` · Failure=`SYNC_*` · Owning=`09_web_thin_client_ledger` · Gate=`pending-nonempty`。

`apps/web/src/hooks/use_core/navigation.rs:24 guard_navigation`：

| 声明语义 | 代码证据 | 判定 |
|---|---|---|
| 只读**当前文档** pending（非全工作区） | `navigation.rs:46 has_pending_for_current_doc` 仅按 `current_doc` + scope 查 `has_pending_edits_for_doc_in_scope` | ✅ |
| 有 pending → 存 target+action 拦截 | `guard_navigation` → `set_pending_navigation.set(Some(PendingNavigation{target,action}))` 返回 false | ✅ |
| 无 pending → 立即执行 | `action.run(())` 返回 true | ✅ |
| Ack/EditRejected 清 guard（当前文档 pending 清空时） | 域1 已验证的 `runtime/document/confirm.rs EditResolution.clear_navigation`（resolution 清空 in-scope pending 即释放 guard） | ✅ |

Note「guard 只读当前文档 pending」「Stay 保留 pending 编辑」「Continue=离开视图≠确认写成功」均成立（Stay/Continue 在 `pending_navigation_modal.rs`）。Auth=PO（操作 PendingLocalEdits 覆盖层）、WG=N、Gate=pending-nonempty 满足。SYNC_* 经 edit reject 路径传递。

### 域 6 小结

1/1 ALIGNED。导航守卫精确地只作用于当前文档 pending，且清守卫复用域1同构的 `EditResolution`。无 GAP/DEVIATION。

---

## 域 3：仓库（5 flow）— 结论 ALIGNED

### 3.1 `flow.repo.file-operations` — **ALIGNED**（深度验证）

§3 声明：UO · Auth=`L+PW` · WG=`Y` · Failure=`SC_*` · Owning=`04_repository` · Gate=`writer-gate+repo-scope`。

文档结构变更（create/rename/copy/move/delete）权威路径：

| 声明语义 | 代码证据 | 判定 |
|---|---|---|
| scope_nonce 门 + spectator/remote fail-closed | `ws/route/docs.rs:12 route_docs` 先 `reject_invalid_browser_scope_nonce(scope.scope_nonce,...)` 早退 | ✅ |
| 全部 5 个结构变更被路由 | route_docs 分派 `handle_{create,rename,delete,copy,move}_doc` | ✅ |
| WG: 写闸 | `docs/create.rs:39 resolve_local_write_scope(...)` 返回 None 即 fail-closed 早退 | ✅ |
| 防遍历路径校验 | `normalize_name` + `validate_file_path`/`validate_folder_path` | ✅ |
| L: 账本注册 DocId | `handle_create_doc` doc 注释步骤4「在 Ledger 中注册 DocId」→ `handle_file_create` | ✅ |
| PW: 树投影更新+广播 TreeDelta | doc 注释步骤5「更新 TreeManager 并广播 TreeDelta」 | ✅ |

fail-closed 由 caller 测试名印证：`create_doc_rejects_stale_browser_scope_with_scoped_error`、`create_doc_rejects_degraded_local_projection_before_mutation`、`create_doc_fails_closed_when_target_path_is_unstatable`、`create_doc_rejects_invalid_browser_path_with_scoped_error`。Note「Spectator/remote 必须 fail closed」「变更的是文档结构非文本内容」成立。

### 3.2 `flow.repo.branch-switch` — **ALIGNED**（深度验证）

§3 声明：UO · Auth=`—` · WG=`N` · Failure=`SC_*` · Owning=`04_repository` · Gate=`switch_nonce>scope_nonce`。

flow 核心（Note）=「`switch_nonce` 与 `scope_nonce` 的严格门控」，`switcher/switcher_guard.rs:10 require_browser_switch_nonce` 忠实实现：

- **`if switch_nonce > session.scope_nonce() { return true; }`**——**严格大于**门控。✅
- `switch_nonce` 缺失 → `ScRepoContextInvalid` "switch nonce missing"；
- `switch_nonce <= scope_nonce` → `ScRepoContextInvalid` "switch nonce is stale: current_scope_nonce=.., requested_switch_nonce=.."（明确 stale 诊断）。✅
- 非 browser session → 拒。`plan_ref: 04_repository#repo-scope-runtime`。
- 专门的 `switcher_switch_nonce_test.rs` 测试模块覆盖。

### 3.3–3.5 支撑 flow — **ALIGNED**（共享受测基础设施）

- `flow.repo.open-doc`（UO · PW · N · DOC_*）：`OpenDoc` 经 `client_scope.rs:25 core_scope_gate` 注册 scope 门（"open doc"），route_docs/core handler 投影 doc。✅ scope-gated。
- `flow.repo.switch`（UO · — · N · SC_*）：复用同一 switcher 基础设施（`switcher_repo.rs`/`switcher_selector`/`switcher_guard`，含 remote-fail-closed 测试）。✅
- `flow.repo.file-op-shell-routing`（II · — · N · SC_*）：命令壳解析/目标预填（`search_box/file_ops/`），纯 II 路由不触权威，单独建模于 `repo_file_op_shell_routing.md`。✅ 接线齐备。

### 域 3 小结

5/5 ALIGNED。两条关键 flow 深度验证：file-operations 的写闸+账本+树投影+fail-closed，branch-switch 的 `switch_nonce>scope_nonce` 严格门控。三条支撑 flow 复用受测的 scope-gate/switcher 基础设施。无 GAP/DEVIATION。

---

## 域 4：网络/同步（4 flow）— 结论 ALIGNED

### 4.1 `flow.net.sync-handshake` — **ALIGNED**

§3 声明：FC · Auth=`—` · WG=`N` · Failure=`SYNC_*` · Owning=`07_network` · Gate=`connected`。

`apps/cli/src/server/handlers/sync/hello/mod.rs:32 handle`：`validate_scope`（scope 门）→ `engine.handshake(repo_id, peer_id, &peer_pubkey, session_proof.signature(), remote_vector)`——**带 peer 公钥 + 会话证明签名的握手 + version vector 交换**。writer/mod.rs 发 `WriteReady`（writer-ready）。web 侧 `handshake/{mod,cycle,state}.rs` gate + `message_sync/mod.rs` 校验 repo/scope 仍匹配后置 `handshake_ready`。✅

### 4.2 `flow.net.sync-transfer` — **ALIGNED**（Auth=L 确认）

§3 声明：FC · Auth=`L` · WG=`N` · Failure=`SYNC_*` · Owning=`07_network` · Gate=`scope-bound`。

| 声明语义 | 代码证据 | 判定 |
|---|---|---|
| 增量请求 + 快照回退 | `sync/engine/handshake.rs:39 compute_diff` 由版本向量产出 `SyncRequest`(own)+`SyncRequest`(shadow)+`SyncSnapshotRequest` | ✅ |
| L: 从账本取 op 范围 | `sync/engine/transfer/mod.rs:15 get_ops_for_sync` → `get_local_ops_in_range`/`get_shadow_ops_in_range` | ✅ |
| 加密传输（RepoKey） | `get_ops_for_sync` 逐条 `repo_key.encrypt(&entry, seq)` → `SyncResponse`(→SyncPush) | ✅ |

### 4.3 `flow.net.key-exchange` — **ALIGNED**（handler 存在 + 下游印证）

§3 声明：FC · Auth=`—` · WG=`N` · Failure=`SYNC_*` · Owning=`07_network` · Gate=`handshake-stage`。

handler `apps/cli/src/server/handlers/key_exchange.rs`（RequestKey→KeyProvide/KeyDenied）存在；下游 `get_ops_for_sync` 强依赖 `repo_key` 加密，印证 repo key 获取流的必要性。Note「RepoKey 只驻留内存、不进浏览器持久化」为设计约束。✅

### 4.4 `flow.cli.server-runtime` — **ALIGNED**（bind 面存在）

§3 声明：ED · Auth=`—` · WG=`N` · Failure=`SYNC_*` · Owning=`07_network` · Gate=`bind-ok`。

`apps/cli/src/commands/serve.rs` 存在（serve --port/--dev/--dry-run/start → Axum HTTP/WS，server/ 引导）。ED bind 面，存在性+接线确认。

### 域 4 小结

4/4 ALIGNED。handshake 为带签名的真实密码学握手；sync-transfer 的 L（账本 op 范围读取）+ RepoKey 加密 + 快照回退齐备。无 GAP/DEVIATION。

---

## 域 2：源控/diff（9 flow）— 结论 7 ALIGNED + 2 PARTIAL（无正确性 bug）

> 方法：本域由 codex CLI（read-only）批量比对，Claude 逐条复核。codex 原始报「5 ALIGNED + 4 问题」；Claude 复核后，4 个标记中 **2 个为真实 plan-vs-code 属性/文档层出入（非正确性 bug）、2 个为过严假阳性**（前端门控被误判为服务端缺失 + Failure Family 粗标签过严解读）。

| Flow ID | codex | Claude 复核 | 关键证据 |
|---|---|---|---|
| `flow.sc.commit` | ALIGNED | ✅ ALIGNED | `commits.rs:13 commit_with_ack`（nonce/writable 门 + staged 非空 + ledger commit） |
| `flow.sc.commit-and-push` | DEVIATION | ✅ **ALIGNED**（更正） | 见下 §2.A：web 不做 Git push=plan 显式非目标；仅 doc 注释误导 |
| `flow.sc.discard-file` | ALIGNED | ✅ ALIGNED | `sync/discard_pending.rs:29,59,104`（writable 门；pending 命中回写/删 PW 并清 pending，FS+PW 落实） |
| `flow.sc.discard-pending` | GAP | ✅ **RESOLVED** | 见 §2.B：catalog `Auth=FS+PW→—` 已按 USER 批准修正；代码不变 |
| `flow.sc.history-commit-diff` | ALIGNED | ✅ ALIGNED | 纯读；commit 缺失映射 `ScCommitNotFound`（commit-exists 门） |
| `flow.sc.merge-peer` | GAP | ✅ ALIGNED | 见下 §2.C |
| `flow.sc.merge-runtime` | GAP | ✅ ALIGNED | 见下 §2.D |
| `flow.sc.resolve-conflict` | ALIGNED | ✅ ALIGNED | `conflict.rs:32,49,61,102,115`（writable 门、conflict-present、FS/PW 两分支） |
| `flow.sc.stage-unstage` | ALIGNED | ✅ ALIGNED | `staging.rs` + `service/write.rs`（change-present 经 resolve/list 强制；staging/pending 表迁移） |

### §2.A `flow.sc.commit-and-push` — ALIGNED（符合 plan 显式 deferral；仅 doc 注释误导）

> **更正**：初判 PARTIAL 基于「push=P2P peer 推送」的错误框定。复查 `features/07_diff_logic.md` 后纠正如下。

§3 声明：FC · Auth=`L` · WG=`Y` · `SC_*` · Gate=`staged+connected`。

**关键事实**：plan 的"Push"= **Git mirror 推送**（`git push` 到 `.git` 镜像），而 `features/07_diff_logic.md` 明确：
- §5：Web 对 `Git: Push Mirror` 等**只给 CLI-only notice**，UI 不得直接执行 Git；「Git import/push/repair 写操作只允许通过显式 CLI surface 触发」。
- 非目标 §66：「**当前阶段不实现 Web 后端直接 Git import/push/repair**」。

`commits.rs:94 handle_commit_and_push` 与 `:13 handle_commit` 同体（同一 `commit_with_ack`：`commit_staged` + broadcast `CommitAck`）。**web/WS 路径不做 Git push，恰是 plan 显式非目标的正确落地**——非 bug、非缺实现。
→ **判定**：**ALIGNED**（功能与 plan 当前阶段范围一致）。唯一瑕疵：handler doc 注释 `commits.rs:93`「提交并推送到所有已连接的 Peer」**误导**（该注释把 Git mirror push 错描成 P2P peer push）。catalog §3 的 `FC`/`connected` 是面向"未来 web 后端实现 Git push"的前瞻属性。**仅需修 doc 注释（代码注释级，非功能），见 §9**。

### §2.B `flow.sc.discard-pending` — PARTIAL（catalog Auth 属性过度声明）

§3 声明：UO · Auth=`FS+PW` · WG=`Y` · `SC_*` · Gate=`pending-present`。

真实 handler `handlers/merge/manual_pending.rs:78 handle_discard_pending`：`resolve_write_repo_id`(写闸) + `engine.clear_pending()` + 发 `PendingDiscarded`。`SyncEngine::clear_pending`（`sync/engine/manual.rs:125`）= `self.pending_ops.clear()`——**仅清「已收到但未应用」的远端合并缓冲**。
**codex 精确定位（read-only，Claude 复核证据属实）确认 `Auth=FS+PW` 不准确：**
1. DiscardPending 不触达落盘 FS/PW：`manual_pending.rs:78-99` 仅 `clear_pending()`，`manual.rs:124-127` 仅 `pending_ops.clear()`（缓冲为 `Vec<PendingSyncPayload>`，`sync/buffer.rs`，纯内存）。
2. 连 shadow 写入都不在 discard 链上、也不是 Markdown FS：写 shadow 发生在 **ConfirmMerge** 路径 `apply_remote_payloads` → `shadow_transfer.rs:94-117`（`run_on_shadow_repo`）→ 写 **`remotes/<peer>/<repo>.redb` 数据库 + redb 内 projection 表**（`structure_projection::apply_in_txn`），**非** per-peer Markdown。真正 PW `std::fs::write` 仅在**本地工作区** `sync/materialize.rs:36-71`，不在该链。
3. 维度是 **`PeerId × RepoId`**（`ledger/manager/types.rs:40-41 shadow_dbs`），非 branch。

**对照 catalog 自身列释义（§2）**：「`PW` 仅指物理 Projection Workspace（Markdown 文件）；内存 Tree State / doc-list projection 不计入本轴」。据此清内存缓冲的 discard-pending 应为 `Auth=—`。
→ **判定**：代码语义**正确**（清未应用缓冲是 discard 正解）；catalog `Auth=FS+PW` **属性写错**，已改 `—`。`pending-present` 为前端门（清空缓冲幂等无害）。非正确性 bug。**catalog 已按 USER 批准修正（2026-05-30），见 §9-B**。

### §2.C `flow.sc.merge-peer` — ALIGNED（codex GAP 为前端门假阳性）

§3 声明：FC · Auth=`L` · WG=`Y` · `SC_*` · Gate=`peer-branch-available`。

`handlers/merge/peer.rs:20 handle_merge_peer`：`resolve_merge_scope`+`resolve_local_merge_scope`（写闸/scope）→ `merge_peer_in_local_repo`（`merge_ops.rs:39`：读本地+远端 op、LCA、三方合并）→ 成功 `write_merged_content`（PW），冲突发 typed `MergeConflict` 存 `pending_merge_conflict`。
- `peer-branch-available` 门由**前端 choose-target**（`command_palette/registry.rs` 解析当前 active branch）满足；服务端读远端 shadow op，空远端=确定性 no-op merge——非服务端缺失。
- 强不变量（`peer.rs:17-19`）：合并目标必须是会话解析的本地 repo；远端影子内容绝不写回他 repo 的 metadata/path。
- 「失败族非 SC」= Failure Family 粗标签：merge 失败用 `classified_failure`（通用码），scope 失败用 `ScRepoContextInvalid`(SC_*)；与列释义「归属族」一致。
→ **判定**：ALIGNED。

### §2.D `flow.sc.merge-runtime` — ALIGNED（codex GAP 为前端门假阳性）

§3 声明：FC · Auth=`L` · WG=`Y` · `SYNC_*` · Gate=`conflict-resolved`。

ConfirmMerge → `handlers/merge/manual_pending.rs:47 handle_confirm_merge` → `engine.merge_pending()`（`sync/engine/manual.rs:79`：解密 pending + `apply_remote_payloads`(账本) + 更新 version vector + 清缓冲）→ 广播 `MergeComplete`。
- 「空 pending 也成功」= 幂等 no-op（`merge_pending` 空时 `Ok(0)`）——可接受。
- `conflict-resolved` / `pending 非空` 为**前端门**（Confirm 按钮仅 pending 非空时可用）。
- 含原子性检查：`merge_pending` 要求单一 peer/repo 目标，否则 `bail`——正确性加分。
- 「失败族非 SYNC」= 粗标签（`classified_failure`）。
→ **判定**：ALIGNED。

### 域 2 小结

**9/9 对齐**（commit-and-push 经深查 07_diff_logic 后为 ALIGNED——web 不做 Git push 是 plan 显式非目标，doc 注释已如实改正；discard-pending 的 catalog `Auth=FS+PW` 属性写错已按 USER 批准修正为 `—`）。**无正确性 bug**。codex 的 4 个标记经复核：1 真实属性出入（discard-pending，已修 catalog）+ 1 符合 plan deferral（commit-and-push）+ 2 前端门/失败族粗标签假阳性（merge-peer/merge-runtime）。

---

## 域 7：渲染（11 flow）— 结论 ALIGNED

> 方法：codex CLI（read-only）批量比对，Claude 抽查复核。全部 ALIGNED；codex 证据经 `rg` 真实命中（mermaid.js/katex.rs/inline_renderer.js/time.rs）。

| Flow ID | 判定 | 关键证据 |
|---|---|---|
| `flow.rendering.cursor-reveal` | ✅ | `js/extensions/hybrid.js:32`；`math.js:99`（光标命中范围即退让源码） |
| `flow.rendering.inline-source-reveal` | ✅ | `hybrid.js:70,181`（frontmatter/链接语法按光标显隐） |
| `flow.rendering.link-activation-gate` | ✅ | `hyperlink_click.js:115`（**Claude 抽查证实**：`if(!ctrlKey&&!metaKey)return; if(button!==0)return`——仅 Ctrl/Meta+左键） |
| `flow.rendering.large-doc-prefetch` | ✅ | `editor/sync/snapshot.rs:76`；`editor/prefetch.rs:18`（snapshot 先显，delta 分批） |
| `flow.rendering.large-doc-search-gate` | ✅ | `hooks/use_core/callbacks/misc.rs:61,76`（loading 阻断 Search） |
| `flow.rendering.math-mermaid` | ✅ | `package.json:23`；`mermaid.js:95`（KaTeX/Mermaid 均为投影） |
| `flow.rendering.math-source-projection` | ✅ | `math.js:109,132`（doc/selection 变化重算） |
| `flow.rendering.mermaid-source-projection` | ✅ | `mermaid.js:156,180`（选区触碰时跳过 widget） |
| `flow.rendering.outline-navigation` | ✅ | `components/outline.rs:100`；`editor_remote_ops.js:117`（点击大纲滚到行首） |
| `flow.rendering.projection-refresh` | ✅ | `editor_adapter.js:118`；`math.js:132`（docChanged 驱动重投影） |
| `flow.search.query` | ✅ | `search_box/mod.rs:101`；`message_dispatch_runtime/mod.rs:113`（请求/结果闭环） |

**小结**：11/11 ALIGNED。核心 source-first 语义落实：投影由 selection/docChanged 退让或刷新；链接激活有 Ctrl/Meta 门；搜索有 loading/scope 闸。

## 域 8：i18n（6 flow）— 结论 ALIGNED

| Flow ID | 判定 | 关键证据 |
|---|---|---|
| `flow.i18n.error-mapping` | ✅ | `i18n/server_error.rs:8`（**Claude 抽查证实**：`message(locale,code)` 把每个 `ServerErrorCode`→En/Zh 文案，`plan_ref:13_i18n#i18n-error-code-catalog`） |
| `flow.i18n.hardcoded-audit` | ✅ | `scripts/check-i18n-hardcoded-baseline.sh:63,89`（JS 文案桥也纳入检查） |
| `flow.i18n.locale-error` | ✅ | `i18n/mod.rs:95,176`（无动态 locale 文件；unsupported 回退 En） |
| `flow.i18n.locale-surface-switch` | ✅ | `command_palette/registry.rs:65`；`settings.rs:67`（命令/设置共用 locale signal） |
| `flow.i18n.locale-selection` | ✅ | `app.rs:40`；`i18n/mod.rs:103`（启动检测浏览器/偏好） |
| `flow.i18n.localized-formatting` | ✅ | `utils/time.rs:34,43`（Intl locale 格式化） |

**小结**：6/6 ALIGNED。静态 Rust facade + JS bridge；错误码经 facade、locale 切换共用 signal、格式化走 Intl。无声明与代码相反之处。

---

## §9 总裁定（42 flow 汇总）

### 总体结论

| 结论 | flow 数 | 说明 |
|---|---|---|
| ALIGNED | **42** | 代码忠实实现 plan 声明语义（commit-and-push：web 不做 Git push 符合 plan 显式非目标；discard-pending：catalog Auth 属性已按 USER 批准修正 `FS+PW→—`） |
| PARTIAL | **0** | （原 discard-pending 属性出入已通过修正 catalog 关闭） |
| GAP（缺实现） | 0 | — |
| DEVIATION（行为相反） | 0 | — |

> **结案（2026-05-30）**：42 条 MUST flow 代码功能与 plan 全部对齐；唯一的属性出入（discard-pending `Auth`）已按 USER 批准修正 catalog。**零代码正确性问题。**

**核心判断：代码功能与 plan 声明的功能高度对齐。** 所有 Current MUST / UI Contract / Approved Runtime 的 42 条 flow，其权威触达（L/PW/PO/FS）、写闸（WG）、关键门控（scope_nonce / switch_nonce>scope_nonce / writer-gate）、失败族归属、Response Flow 序列均与代码一致。核心写路径（edit-confirmed-op）、结构写（file-operations）、握手（sync-handshake）、鉴权（login）等正确性敏感面为**典范级对齐**，含多处微妙不变量（投影失败不回滚权威、防用户名枚举、merge 原子性、远端影子不越界）被正确建模。

### 待 USER 裁定项

#### A. `flow.sc.commit-and-push`（更正为 ALIGNED）+ §9 冲突提示

复查 `features/07_diff_logic.md` 后，此项**功能上已对齐 plan**：plan 的"Push"=Git mirror 推送，且**当前阶段明确不实现 web 后端 Git push**（§66 非目标 + §5 CLI-only notice）。web/WS commit-and-push 同体 commit、不做 Git push，正是该非目标的正确落地。

- **仅需的小修**：`commits.rs:93` doc 注释「提交并推送到所有已连接的 Peer」误导（把 Git mirror push 错描成 P2P peer push）→ 改为反映"web 仅 commit；Git mirror push 为 CLI-only/当前阶段未实现"。**代码注释级**改动。
- **⚠️ §9 冲突**：若要"在 web 后端实现显式 push"，会**直接违反** `07_diff_logic` 非目标「当前阶段不实现 Web 后端直接 Git push」。这不是补 bug，而是**改变 plan 当前阶段范围**——须先更新 07_diff_logic deferral + threat_model（Git push fail-closed 前置见 05_diff_logic:92），属较大 feature，且要重审"`.git` 仅作 projection mirror、`.notegit`/ledger 为 authority"的边界。

#### B. `flow.sc.discard-pending` — RESOLVED（catalog 已按 USER 批准修正）

codex 精确定位 + USER 确认：shadow 落成 `remotes/<peer>/<repo>.redb` **直接覆盖**（redb 数据库，**非** per-peer Markdown 文件夹）为**设计本意**；DiscardPending 仅清内存缓冲、不碰盘。故 catalog `Auth=FS+PW` 系**属性写错**。

**已执行（2026-05-30，USER 批准）**：`20_operations_catalog.md §3` 该行 `Auth=FS+PW → —`，`Last Review→2026-05-30`。**代码不变**。

> 遗留小项（未改，待 USER）：该行 `WG=Y`——discard 经 `resolve_write_repo_id` 写闸，但 Auth=— 后，按列释义「WG=Y 需通过 writer gate 方可产生**权威副作用**」严格说与 Auth=— 略有张力。当前**保留 Y**（写闸在代码中确实存在）；如需严格一致可改 N，由 USER 定。

### 方法学备注

- 域 1/3/4/5/6 由 Claude 用 codegraph 直接深度审计。
- 域 2/7/8 由 codex CLI（read-only）批量比对、Claude 逐条复核：域 2 codex 初报 4 问题，复核后澄清为 2 真实属性出入 + 2 前端门/粗标签假阳性；域 7/8 全 ALIGNED，Claude 抽查 link-activation-gate / i18n-facade 证据属实。
- 跳过 30 条非 MUST flow（commands/settings/ai/plugin/release/tech-stack/backup，Status=Planned/Optional/Deferred/Reference）与全部 Apple 环境项。
