# Deve-Notebook 当前工作树全仓复审报告

**日期**: 2026-03-07  
**基线**: 当前工作树（不回退 `HEAD`）  
**范围**: `crates/core`、`apps/cli`、`apps/web`、`plugins`、`tests/`、`deve-note plan/`、`deve-note report/schedules/`  
**约束**: 本轮仅新增报告，不修改业务代码、计划文档或旧审计文件

## 执行摘要

- 旧审计报告 [report/audit-2026-03-07.md](./audit-2026-03-07.md) 的 10 个问题项中：
  - 严格 `已完成`: 4 项
  - `部分完成`: 4 项
  - `未完成/恶化`: 2 项
  - 严格完成率: `40%`
  - 含“部分完成”按 0.5 计的加权收敛率: `60%`
- 当前仍然存在 1 个 `Critical`、4 个 `High`、4 个 `Medium` 级问题。
- 最核心的未收敛点不是“单点 bug”，而是 `repo_id` 仍未成为端到端唯一真值源：前端 `current_repo` 同时承载“仓库展示名”和“repo UUID”两种语义，但实际只写入了仓库名，导致新的 repo-scoped identity 运行时处于失活状态。
- 旧报告中“登录流缺失”已经被部分修补，但当前工作树引入了新的契约错位：前端登录页解析 `{ status }`，后端返回 `{ success, error }`，正确凭据在前端依然会失败。
- 结构治理仍未回到阈值内：Rust 源文件 `>250` 已从旧报告的 2 个上升到 3 个；计划文档 `>250` 仍维持在 2 个。

## 1. 审查方法与验证边界

- 静态核对：
  - 逐项比对旧审计条目与当前代码锚点。
  - 逐份抽检 `deve-note plan/`、`验收清单`、`schedules` 与实际代码契约。
  - 统计 Rust 源文件、计划文档、JS Bridge 行数。
- 动态验证：
  - 已尝试 `cargo check --workspace`。
  - 2026-03-07 当前环境已能下载缺失 crates，但最终失败于 `openssl-sys v0.9.111`，原因是系统缺少 `pkg-config` 与 OpenSSL 开发库，因此本报告仍以静态审查为主。
- 工作区状态：
  - 当前工作树处于明显脏状态，`git status --short` 显示大量未提交改动，且计划文档本身也在持续改写。
  - 因此本报告只对“当前磁盘状态”负责，不对改动来源做归因。

## 2. 旧审计复核矩阵

| 旧条目 | 旧结论 | 当前状态 | 复核结论 |
| --- | --- | --- | --- |
| C1 | `repo_id` 未成为端到端主键 | `部分完成` | 服务端已能发 `RepoSwitched.uuid`，前端存储运行时也只接受 UUID，但前端状态层仍只保存仓库名，导致 repo-scoped identity 实际未接通。 |
| H1 | 会话未绑定 peer/repo，`SyncPush` 写入 `unknown` | `已完成` | `handle_sync_hello()` 已写入 `authenticated_peer_id` 与 `bound_repo_id`，`SyncPush` 也改为读取会话态来源 peer。 |
| H2 | 快照同步与手动合并绕过真实 repo 身份 | `部分完成` | `merge.rs` 已切到 `session.bound_repo_id`，`snapshot.rs` 也按 `request.repo_id` 取快照；但快照请求/推送入口仍不验证 `WsSession`。 |
| H3 | debug 构建静默回退开发凭据 | `已完成` | `AuthConfig::from_env()` 已改为显式 `DEVE_ENV=development` 才启用开发默认值。 |
| H4 | Web 端无登录闭环 | `部分完成` | 登录页、`/api/auth/login`、`/api/auth/me` 都已接上；但前后端登录响应契约不一致，前端仍无法成功消费登录成功响应。 |
| H5 | 断连只读未封口，写操作可离线回放 | `已完成` | `is_write_message()` 已纳入 `Commit`、`ResolveConflict`、`DeletePeer`、`SetSyncMode` 等旧漏项。 |
| M1 | 根路径会被首篇文档抢占 | `已完成` | `handle_doc_list()` 已不再自动选中首篇文档。 |
| M2 | 限流策略与文档不一致，缺 WS 消息级限流 | `部分完成` | 登录 `5/min/IP` 与 API `120/min/IP` 已分层；但 WebSocket `200 条/分钟/连接` 仍未实现。 |
| M3 | 主计划/验收/代码系统性漂移 | `未完成` | 主计划仍写 `Ready for Implementation`，验收仍保留旧端口/旧端点/失效章节名。 |
| M4 | 文件长度治理越界 | `未完成且恶化` | Rust `>250` 已增加到 3 个；计划文档 `>250` 仍维持 2 个，整体仍未回到规范阈值内。 |

### 2.1 C1 `repo_id` 主键化：部分完成

**已收敛部分**

- 服务端 `ListDocs` 已经发送真实仓库 UUID：`apps/cli/src/server/handlers/listing.rs:29-42`。
- 浏览器存储运行时不再接受 `"default"` 之类的伪 key，只接受可解析 UUID 的 `repo_id`：`apps/web/src/hooks/use_core/storage_runtime.rs:16-22`、`57-63`。

**未收敛部分**

- 前端唯一写入 `current_repo` 的地方仍只写仓库名，不写 UUID：`apps/web/src/hooks/use_core/effects.rs:228-229`、`apps/web/src/hooks/use_core/effects_msg.rs:92-93`。
- 但 `storage_runtime` 恰恰要求 `current_repo` 必须是 UUID 才启动：`apps/web/src/hooks/use_core/storage_runtime.rs:57-63`。
- 这意味着新的 repo-scoped identity/vector 运行时在当前工作树中实际上是失活的。
- 更糟的是，收到服务端 `SyncHello` 后，前端仍按 `current_repo` 或 `"default"` 保存向量：`apps/web/src/hooks/use_core/effects.rs:176-187`。
- 服务端仍保留 `Uuid::nil()` 兜底：`apps/cli/src/server/handlers/mod.rs:23-30`。
- 文件树仍是进程级共享 `tree_manager`，并通过全局广播发送 `TreeUpdate`：`apps/cli/src/server/mod.rs:51-63`、`127-143`，`apps/cli/src/server/setup.rs:94-105`。

**结论**

- 旧报告把问题描述为“模型没跟上”；当前工作树是“模型部分接上，但前端状态层把 UUID 丢了”，因此本项只能判为 `部分完成`。

### 2.2 H1 会话绑定：已完成

- `handle_sync_hello()` 已接收 `&mut WsSession`，并在握手成功后调用 `session.set_authenticated()` 与 `session.bind_repo()`：`apps/cli/src/server/handlers/sync.rs:17-26`、`56-59`。
- `handle_sync_request()` 已校验 `session.is_repo_bound(&repo_id)`：`apps/cli/src/server/handlers/sync.rs:115-128`。
- `handle_sync_push()` 已同时校验 repo 绑定，并强制从 `session.authenticated_peer_id` 取得来源 peer：`apps/cli/src/server/handlers/sync.rs:160-199`。
- 旧报告中的 `PeerId::new("unknown")` 已消失。

**结论**

- 旧 H1 的原始断言已经不成立，本项判为 `已完成`。
- 但快照消息未复用同样的会话校验，这个残余风险应转入 H2，而不是继续算在 H1 里。

### 2.3 H2 快照/手动合并：部分完成

**已收敛部分**

- 快照生成已按请求 `repo_id` 选取仓库：`crates/core/src/sync/engine/transfer/snapshot.rs:14-23`。
- 手动同步模式的 `GetSyncMode`、`SetSyncMode`、`GetPendingOps`、`ConfirmMerge`、`DiscardPending` 已全部改为从 `session.bound_repo_id` 取 repo：`apps/cli/src/server/handlers/merge.rs:14-17`、`25-41`、`59-84`、`103-120`、`145-161`、`184-200`。

**未收敛部分**

- `handle_sync_snapshot_request()` 没有 `WsSession` 参数，只信任消息自带的 `peer_id` 与 `repo_id`：`apps/cli/src/server/handlers/sync.rs:214-247`。
- `handle_sync_push_snapshot()` 同样没有会话绑定验证，来源 `peer_id` 仍来自客户端消息：`apps/cli/src/server/handlers/sync.rs:257-280`。

**结论**

- “快照生成/手动合并默认落本地主仓库”这部分已经明显改善。
- 但“快照路径仍可绕过会话态”没有收口，因此本项是 `部分完成`，不是 `已完成`。

### 2.4 H3 认证契约：已完成

- `AuthConfig::from_env()` 已明确改为生产默认 `fail-closed`，且仅在显式 `DEVE_ENV=development` 时才回退 `dev_default()`：`crates/core/src/security/auth/config.rs:31-47`。
- `router::load_auth_config()` 也同步改为显式环境驱动：`apps/cli/src/server/router.rs:70-91`。

**结论**

- 旧 H3 的核心问题“debug 构建隐式授权”已解决，本项判为 `已完成`。

**残余风险**

- 开发默认凭据仍可与 `0.0.0.0` 绑定共存：`crates/core/src/security/auth/config.rs:78-87`、`apps/cli/src/server/mod.rs:162-166`。这不是旧 H3 的原问题，但仍值得在优化建议中继续收口。

### 2.5 H4 登录闭环：部分完成

**已收敛部分**

- 根应用已根据认证状态在 `MainLayout` 与 `LoginPage` 之间分流：`apps/web/src/app.rs:23-50`。
- 后端已经提供 `/api/auth/login`、`/api/auth/logout`、`/api/auth/me`：`apps/cli/src/server/router.rs:34-67`、`apps/cli/src/server/auth/handlers.rs:40-141`。

**新问题**

- 前端仍按旧契约解析 `LoginResponse { status }`：`apps/web/src/components/login.rs:20-31`、`166-191`。
- 后端现在返回的是 `LoginResponse { success, error }`：`apps/cli/src/server/auth/handlers.rs:28-33`、`101-118`。
- 结果是：用户名/密码正确时，浏览器收到 `200 OK` 后会在 JSON 解析阶段报错，而不是进入已认证状态。
- 此外，断连遮罩仍只区分 `Connected/Disconnected`，没有区分“未登录/401”与“网络断开”：`apps/web/src/components/disconnect_overlay.rs:7-35`。

**结论**

- 旧 H4 不能再判为“完全缺失登录流”，但也远未闭环，因此是 `部分完成`。

### 2.6 H5 断连只读封口：已完成

- 输出管理器在断连时会阻止所有写类消息入队：`apps/web/src/api/output.rs:62-69`。
- 旧漏项 `Commit`、`ResolveConflict`、`DeletePeer`、`SetSyncMode` 已全部纳入 `is_write_message()`：`apps/web/src/api/output.rs:188-206`。

**结论**

- 旧 H5 的“离线写操作回放”问题在当前工作树中已基本收口，本项判为 `已完成`。

### 2.7 M1 Dashboard 根路径：已完成

- `handle_doc_list()` 已只更新文档列表，不再自动选中文档：`apps/web/src/hooks/use_core/effects_msg.rs:13-25`。

**结论**

- 旧 M1 的直接断言已经不成立，本项判为 `已完成`。

### 2.8 M2 限流策略：部分完成

- 登录和普通 API 已拆成不同限流器：`apps/cli/src/server/router.rs:28-32`、`47-67`。
- 但限流实现仍是统一 per-IP 滑动窗口，并没有 endpoint 级或连接级消息计数：`apps/cli/src/server/rate_limit.rs:46-121`。
- WebSocket 连接建立后，消息循环中没有任何每连接计数器或桶：`apps/cli/src/server/ws/mod.rs:87-121`。

**结论**

- 本项相对旧报告已有进展，但 `09_auth.md` 要求的 `200 条消息/分钟/连接` 仍缺失，因此是 `部分完成`。

### 2.9 M3 文档漂移：未完成

- 主计划仍写着 `Ready for Implementation`：`deve-note plan/deve-note plan.md:3-5`。
- `05_network.md` 与 `deve-note report/schedules/01_core.md` 仍写“客户端与服务端使用 JSON”：`deve-note plan/05_network.md:77-80`、`deve-note report/schedules/01_core.md:24-33`。
- 但服务端对客户端的单播已经强制走 Bincode：`apps/cli/src/server/ws/send.rs:19-39`。
- 验收用例仍保留旧端口与旧端点，例如 `http://localhost:3000/api/login`、`/api/health`：`deve-note plan/acceptance-cases/08_auth.md:32-42`、`73-100`。
- 代码注释仍引用不存在的 `03_ui_architecture.md`：`apps/web/src/components/spectator_overlay.rs:5`、`apps/web/src/components/quick_open/mod.rs:5`。

**结论**

- 旧 M3 没有收敛，本项判为 `未完成`。

### 2.10 M4 文件长度治理：未完成且恶化

**当前统计**

- Rust 源文件总数：`396`
- Rust `130-250`：`96`
- Rust `>250`：`3`
  - `apps/cli/src/server/handlers/sync.rs` `318`
  - `apps/cli/src/server/handlers/merge.rs` `273`
  - `apps/web/src/hooks/use_core/effects.rs` `254`
- 接近高警戒但未越线：
  - `crates/core/benches/sync_bench.rs` `240`
  - `apps/web/src/hooks/use_core/state.rs` `227`
- 计划文档总数：`36`
- 计划文档 `130-250`：`8`
- 计划文档 `>250`：`2`
  - `deve-note plan/acceptance-cases/05_ui.md` `519`
  - `deve-note plan/08_ui_design_03_mobile.md` `273`
  - 其余主计划仍大面积处于 `130-250`
- JS 源文件：
  - `apps/web/js/extensions/hybrid.js` `228`
  - `apps/web/js/editor.bundle.js` 为豁免构建产物，仍超过 `400`

**结论**

- 相比旧报告，本项在 Rust 源文件维度上继续恶化，在计划文档维度上则维持原状，因此仍判为 `未完成且恶化`。

## 3. 当前主要问题与结构性债务

## 3.1 Critical

### C1. WebLightPeer 的 repo-scoped identity 运行时当前处于失活状态

**证据链**

- 服务端发出了 `RepoSwitched { name, uuid }`：`apps/cli/src/server/handlers/listing.rs:38-42`。
- 前端却在 `handle_repo_switched()` 中只保存 `name`：`apps/web/src/hooks/use_core/effects_msg.rs:91-94`。
- `storage_runtime` 仅在 `current_repo` 可解析为 UUID 时才启动：`apps/web/src/hooks/use_core/storage_runtime.rs:16-22`、`57-63`。
- 当前工作树里没有第二条路径会把 UUID 写入 `current_repo`：`apps/web/src/hooks/use_core/effects.rs:228-229`。

**结论**

- 这不是“有 bug 的优化项”，而是当前 repo-scoped storage/runtime 根本没有进入工作态。
- 旧报告把它判为 `Critical` 是合理的；当前工作树虽然接了一半接口，但由于状态层丢失 UUID，风险级别并未下降。

## 3.2 High

### H1. 快照同步入口仍未绑定会话态，允许客户端自带 `peer_id`

- `handle_sync_request()`/`handle_sync_push()` 已做会话校验：`apps/cli/src/server/handlers/sync.rs:115-199`。
- 但 `handle_sync_snapshot_request()`/`handle_sync_push_snapshot()` 既不接收 `WsSession`，也不校验 `authenticated_peer_id`：`apps/cli/src/server/handlers/sync.rs:214-280`。

**风险**

- 快照路径仍可跨 repo 或伪造来源 peer，破坏旧 H1/H2 已经建立的会话约束。

### H2. 登录前后端契约已经分叉，`LoginPage` 在成功登录时会解析失败

- 前端期待 `status: success|invalid_credentials`：`apps/web/src/components/login.rs:20-31`、`186-191`。
- 后端返回 `success: bool, error: Option<String>`：`apps/cli/src/server/auth/handlers.rs:28-33`、`101-118`。

**风险**

- 这会让“已补齐的登录页”在真实运行中依然不可用，属于高优先级回归。

### H3. 文件树状态仍是进程级共享对象，跨 repo 广播污染未收口

- `AppState` 中只有一个全局 `tree_manager`：`apps/cli/src/server/mod.rs:51-63`。
- 文件监视器始终按本地 repo 重建整棵树，并对所有连接广播 `TreeUpdate`：`apps/cli/src/server/setup.rs:94-105`。
- `handle_list_docs()` 会根据当前 session 重写同一个 `tree_manager`：`apps/cli/src/server/handlers/listing.rs:70-98`。

**风险**

- 多 repo、多 branch、多客户端并发时，树状态与广播目标都可能互相覆盖。

### H4. WebSocket 仍缺消息级限流，`schedules` 却已宣称完成

- `09_auth.md` 明写 `WebSocket: 200 条消息/分钟/连接`：`deve-note plan/09_auth.md:131-136`。
- 代码仅实现了 HTTP 层 per-IP 限流：`apps/cli/src/server/router.rs:28-32`、`62-66`，`apps/cli/src/server/rate_limit.rs:46-121`。
- `deve-note report/schedules/01_core.md` 仍把 `Rate Limiting` 全量标记为完成：`deve-note report/schedules/01_core.md:84-90`。

**风险**

- 单条已升级的 WS 连接可以在连接建立后高频灌消息，文档声明与实现能力明显不符。

## 3.3 Medium

### M1. I18n 技术栈与错误码契约没有真正落地

- 计划与技术栈都写 `leptos_i18n`：`deve-note plan/10_i18n.md:5-15`、`deve-note plan/14_tech_stack.md:5-12`。
- `apps/web/Cargo.toml` 中并无 `leptos_i18n` 依赖：`apps/web/Cargo.toml:8-37`。
- 当前实现是手写 `Locale` 枚举与静态函数：`apps/web/src/i18n/mod.rs:34-77`、`apps/web/src/i18n/common.rs:101-184`。
- 登录页仍有硬编码中文字符串：`apps/web/src/components/login.rs:65-67`。
- 后端 auth handler 直接返回自然语言错误，不是计划中的错误码：`apps/cli/src/server/auth/handlers.rs:57-60`、`71-74`、`115-117`；而 `10_i18n.md` 明确要求“后端必须返回标准错误码”：`deve-note plan/10_i18n.md:12-15`。

### M2. `schedules` 的完成声明已经明显超前于代码事实

- `deve-note report/schedules/01_core.md` 把网络同步、速率限制、Auth 都标成完成：`deve-note report/schedules/01_core.md:24-33`、`84-90`。
- `deve-note report/schedules/02_ui.md` 把 `leptos_i18n` 与 “Full Translation” 标成完成：`deve-note report/schedules/02_ui.md:41-48`。
- 这些声明与当前代码不符，因此 `schedules` 目前更像“目标态宣称”，不是可信状态板。

### M3. 行数治理恶化且高风险文件继续集中在同步/前端 effect

- Rust `>250` 文件集中在 `sync.rs`、`merge.rs`、`effects.rs` 这类高风险控制流模块。
- `apps/web/src/hooks/use_core/state.rs` 虽未越过 250，但格式与可读性已经明显下降：`apps/web/src/hooks/use_core/state.rs:124-152`。

### M4. 动态验证仍受系统依赖阻塞

- `cargo check --workspace` 已能进入实际编译阶段，但最终失败于 `openssl-sys v0.9.111`，错误原因为缺少 `pkg-config` 与 OpenSSL 开发库。
- 因此当前仓库的“可编译性结论”仍不能仅凭本地环境直接盖棺。

## 4. 计划与实现一致性矩阵

| 领域 | 当前判断 | 证据 |
| --- | --- | --- |
| `storage` | `部分一致` | IndexedDB/WebCrypto 分层已经接入，但 `repo_id` 只到协议，不到前端状态层：`apps/web/src/hooks/use_core/storage_runtime.rs:42-135`、`apps/web/src/hooks/use_core/effects_msg.rs:91-94`。 |
| `network` | `部分一致` | 会话绑定已补，快照入口未补；协议文档仍写“Client-Server JSON”，实际 server->client 单播已是 Bincode：`apps/cli/src/server/handlers/sync.rs:56-59`、`214-280`，`apps/cli/src/server/ws/send.rs:19-39`。 |
| `repository` | `部分一致` | 分支/仓库切换与只读模式存在，但 repo 真值源仍混合为名字字符串与全局 `get_repo_id()`：`apps/cli/src/server/handlers/listing.rs:22-42`、`49-55`，`apps/cli/src/server/handlers/mod.rs:23-30`。 |
| `diff/source_control` | `大体一致` | 三阶段工作流和 SC 路由已在代码中有实体实现，旧审计重点不在此；但本轮未完成全量动态回归。 |
| `ui` | `部分一致` | Dashboard 根路径被修正，但登录态/断连态区分不完整，登录成功响应又发生新错位：`apps/web/src/app.rs:23-50`、`apps/web/src/components/login.rs:166-191`、`apps/web/src/components/disconnect_overlay.rs:7-35`。 |
| `auth` | `部分一致` | 后端 fail-closed、Cookie 策略、登录端点均在；但前端消费契约错误，WebSocket 消息级限流仍缺失：`crates/core/src/security/auth/config.rs:31-88`、`apps/cli/src/server/auth/handlers.rs:40-175`、`apps/cli/src/server/ws/mod.rs:87-121`。 |
| `i18n` | `不一致` | 计划要求 `leptos_i18n + 错误码`，实现是手写 `Locale` + 自然语言错误：`deve-note plan/10_i18n.md:5-15`，`apps/web/src/i18n/mod.rs:34-77`，`apps/cli/src/server/auth/handlers.rs:57-60`。 |
| `plugins` | `部分一致` | Rhai Runtime、capability gate、`max_operations` 已存在；WASM/Podman 仍按文档处于延期态：`crates/core/src/plugin/runtime/rhai_v1.rs:21-57`、`crates/core/src/plugin/runtime/host/fs.rs:16-68`、`crates/core/src/plugin/runtime/host/mcp.rs:15-83`。 |
| `release` | `部分一致` | `Dockerfile`、`release.yml`、`nightly.yml` 存在：`Dockerfile:1-74`、`.github/workflows/release.yml:10-72`、`.github/workflows/nightly.yml:12-99`；但本地编译仍受系统依赖阻塞，验收用例端口也未同步。 |

## 5. 具体优化建议

### 5.1 统一 repo 上下文模型，禁止 `String` 同时承载仓库名与 UUID

**当前方案**

- `current_repo: Option<String>` 同时被 UI 当作仓库名展示，又被存储运行时当作 `repo_id`。

**朴素风险**

- 一次错误赋值就让整个 repo-scoped identity/vector/runtime 失活。
- 后续任何“按 repo 分桶”的逻辑都会出现隐式回退 `"default"` 或名字字符串污染。

**建议方案**

- 引入单独的 `RepoContext { repo_id: Uuid, repo_name: String, branch: Option<PeerId> }`。
- `RepoSwitched` 在前端落地时拆成两个信号，不再把 `uuid` 丢弃。
- `storage_runtime`、握手、vector 持久化只读 `repo_id`；UI 展示只读 `repo_name`。

**不变量**

- `repo_id` 是同步、存储、缓存、权限校验的唯一主键。
- `repo_name` 仅用于展示，不参与路由或持久化分桶。

**复杂度与资源收益**

- 逻辑复杂度保持 `O(1)`。
- 减少错误 identity 桶与错误 vector 写回，避免浏览器端无效对象与无效 IndexedDB 记录增长。

### 5.2 把快照同步完全纳入 `WsSession`

**当前方案**

- 增量同步已经绑定 `WsSession`，快照同步仍信任消息体。

**朴素风险**

- 攻击者或错误客户端可伪造 `peer_id` / `repo_id`，绕过已经建立的会话信任边界。

**建议方案**

- `handle_sync_snapshot_request()`、`handle_sync_push_snapshot()` 新增 `&WsSession` 参数。
- `repo_id` 必须与 `session.bound_repo_id` 一致。
- `peer_id` 必须从 `session.authenticated_peer_id` 派生，客户端自带 `peer_id` 只允许作为调试字段，不能参与授权。

**前置条件**

- `SyncHello` 已成功，`WsSession` 已绑定 peer 和 repo。

**后置条件**

- 同一连接后续所有同步消息都只能在单一 repo 作用域内运行。

**复杂度与资源收益**

- 仅增加常数级比较，不引入新堆分配。
- 直接提升同步正确性，不增加内存占用。

### 5.3 把 `tree_manager` 改为 repo-scoped，而不是进程级全局单例

**当前方案**

- 服务器只有一个 `tree_manager`，任何 `ListDocs` 或 watcher 事件都会重写它。

**朴素风险**

- 多 repo 客户端互相覆盖树状态。
- Watcher 生成的 `TreeUpdate` 会被广播给错误 repo 的连接。

**建议方案**

- 改为 `HashMap<RepoId, TreeManager>` 的懒加载结构，或者将 `TreeUpdate` 带上 `repo_id` 并在发送层过滤。
- watcher 只更新对应 repo 的树，不再对全局广播裸 `TreeUpdate`。

**不变量**

- 每个 `RepoId` 仅对应自己的树状态与广播流。

**复杂度与资源收益**

- 单次查找 `O(1)`。
- 内存增长与活跃 repo 数线性相关，但在 768 MB VPS 下可通过惰性初始化与空闲回收保持可控。

### 5.4 为 WebSocket 增加每连接消息限流

**当前方案**

- 只有 HTTP 层 per-IP 限流，没有已建立连接的消息桶。

**朴素风险**

- 连接建立后可以绕过登录/API 限流，对同步/搜索/插件路由高频施压。

**建议方案**

- 在 `WsSession` 内挂载轻量级令牌桶或滑动窗口计数器。
- 对每条收到的 `ClientMessage` 在进入路由前计数；超限后返回错误并断开。

**不变量**

- 单连接在 60 秒窗口内最多处理 200 条消息。

**复杂度与资源收益**

- 每条消息常数级计数。
- 每连接只需一个小型计数结构，内存开销远低于一次错误广播或大批量快照处理。

### 5.5 把登录/错误响应契约收敛到共享类型，并切回错误码

**当前方案**

- 前后端分别维护独立 `LoginResponse` 结构，且已经分叉。
- Auth API 直接返回自然语言字符串。

**朴素风险**

- 登录页这种最基础路径也会因字段名漂移而回归。
- 错误文本耦合语言，阻断 `i18n` 规范。

**建议方案**

- 将登录响应与错误码定义提升到共享协议层，例如 `deve_core::protocol::auth`。
- 前端只解析共享类型；后端只返回错误码，如 `AUTH_INVALID_CREDENTIALS`、`AUTH_RATE_LIMITED`。

**复杂度与资源收益**

- 结构体共享后，编译期即可发现字段漂移。
- 错误码比自然语言更短，网络负载更小，缓存更稳定。

### 5.6 在文档层停止“完成态先行”，改为区分 `implemented / partial / planned`

**当前方案**

- `schedules` 对很多仍未闭环的能力直接打 `[x]`。

**朴素风险**

- 新协作者会以文档为真值，错误判断回归范围与实现边界。

**建议方案**

- `schedules` 改三态：`implemented`、`partial`、`planned`。
- 主计划去掉 `Ready for Implementation` 这类过时总状态，改为“最后校准日期 + 当前实现覆盖率”。

**复杂度与资源收益**

- 无运行时成本。
- 直接降低审查与协作认知成本。

### 5.7 优先拆分高风险超长文件

**建议顺序**

1. `apps/cli/src/server/handlers/sync.rs`
2. `apps/cli/src/server/handlers/merge.rs`
3. `apps/web/src/hooks/use_core/effects.rs`
4. `apps/web/src/hooks/use_core/state.rs`

**理由**

- 这些文件都在同步、认证、运行时副作用主路径上。
- 继续在这些文件上叠补丁，会放大状态错位和回归概率。

## 6. 附录

### 6.1 本轮核对命令

```bash
git status --short
rg -n "RepoSwitched|get_repo_id|authenticated_peer_id|set_authenticated|Uuid::nil|allow_anonymous_localhost|/api/auth/login|leptos_i18n" crates apps "deve-note plan" report
find crates apps plugins tests -path '*/node_modules' -prune -o -path '*/dist' -prune -o -path '*/target' -prune -o -name '*.rs' -print0 | xargs -0 wc -l
find "deve-note plan" -name '*.md' -print0 | xargs -0 wc -l
find apps/web/js -type f -name '*.js' ! -name '*.bundle.js' | xargs wc -l
cargo check --workspace
```

### 6.2 动态验证结论

- `cargo check --workspace` 已完成依赖下载阶段，但最终失败于 `openssl-sys v0.9.111`。
- 直接阻塞原因：
  - 缺少 `pkg-config`
  - 缺少 OpenSSL 开发库路径
- 因此当前环境下无法把“可编译性”作为本轮报告的最终裁决依据。

### 6.3 本轮未修改项

- 未修改任何业务代码。
- 未修改任何 `deve-note plan/` 文档。
- 未覆盖旧报告 [report/audit-2026-03-07.md](./audit-2026-03-07.md)。
