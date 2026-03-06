# Learnings - WebLightPeer Audit Unification

记录执行过程中发现的模式、约定和最佳实践。

---

## [2026-03-06 T1] WebLightPeer 术语定义

**关键发现**:
- 原文档将 Web 描述为 "不是 P2P 节点"，但实际代码中浏览器持久化 IdentityKeyPair 并发送 SyncHello（peer 行为）
- 新定义明确 WebLightPeer 为"受限同步端点"，避免"纯 dashboard"与"完全 peer"的模糊地带
- 核心约束：repo-scoped isolation, online dependency, storage separation, auth layering

**术语表**:
- WebLightPeer, DashboardSession, PeerIdentity, RepoScopedVector, OfflineCache, DegradedSyncMode

**不变量**:
- INV-1: Repo Scope Isolation
- INV-2: Online Dependency
- INV-3: Storage Separation
- INV-4: Auth Layering

---

## [2026-03-06 T2] 网络协议 Repo-Scoped 重写

**关键发现**:
- 原网络契约把 `3001..3005` 端口探测写成默认前端行为，这适合本地调试，但不适合作为生产路由规范
- 旧协议对 `SyncHello` / `SyncRequest` / `Snapshot` 的 repo 上下文约束不够强，无法保证多仓库下的确定性路由
- Proxy 模式的真正稳定面向浏览器契约应是同源 `relative /ws`，而不是暴露后端实际端口选择逻辑

**协议更新**:
- WebLightPeer 握手改为显式 `repo_id` 驱动，Server 在握手阶段即绑定单个 repo 路由上下文
- 重连流程要求重新发送当前 repo 的 `SyncHello`，repo 切换时必须重建 identity/vector/connection context
- Snapshot fallback 保留，但 `Snapshot { repo_id, server_vector, payload }` 必须与具体 repo 绑定，禁止空 repo 占位符

**验收更新**:
- 生产验收改为校验 `relative /ws` 或单一配置端点，不再把端口扫描当作规范行为
- 新增多仓库切换重新握手与状态隔离验收
- SyncRequest / SyncPush / GossipOffer 示例全部补齐 repo-scoped 上下文

## [2026-03-06 T3] 浏览器存储与信任模型定义

**关键发现**:
- 原存储模型虽已提出分层方向，但 `04_storage.md` 尚未给出四类浏览器存储的允许/禁止边界与恢复语义
- 必须把 `session token` 与 `peer identity` 写成两条独立流程，否则会误导实现把登录 Cookie 当成同步节点身份
- 浏览器私钥的正确约束不是“把字节放进 IndexedDB”，而是“私钥材料保持在 WebCrypto 非导出对象内，IndexedDB 仅保存可恢复句柄与元数据”

**存储分层**:
- `localStorage`: `UI prefs`，仅限主题、布局、语言等无害前端偏好
- `JWT Cookie`: 用户会话，负责 Dashboard/API/WebSocket 的访问鉴权
- `IndexedDB`: `peer identity` metadata 与 `offline cache`
- `WebCrypto`: 不可导出的 browser peer 私钥

**降级策略**:
- IndexedDB 或 WebCrypto 不可用时进入 `DegradedSyncMode`
- 允许保留登录态与只读拉取，但禁止编辑、写入同步与持久化 peer 注册

## [2026-03-06 T4] 验收套件重写

**关键发现**:
- 原验收用例假设生产回退默认凭据（与 C1 冲突）
- 原验收用例编码 3001..3005 扫描为规范行为（与 H1 冲突）
- 原验收用例未覆盖 multi-repo isolation（与 H3 冲突）
- 原验收用例未覆盖 plugin capability enforcement（与 H4 冲突）
- 原验收用例未考虑 dashboard 根状态稳定性（与 H5 冲突）

**验收更新**:
- Auth: 新增 fail-closed 启动、explicit dev mode、cookie secure 策略、CORS 环境驱动、精确 cookie 名称匹配、localStorage panic 防护
- Network: 新增 WebLightPeer repo-scoped handshake、multi-repo isolation、relative WS 连接、dashboard root state stability
- Plugin: 新增 capability gates enforcement、Rhai runtime limits

**审计覆盖**:
- C1: AC-AUTH-01, AC-AUTH-02
- H1: AC-AUTH-03, AC-AUTH-04, AC-NET-02
- H2: AC-NET-03
- H3: AC-NET-03, AC-NET-04
- H4: AC-PLUG-02, AC-PLUG-03
- H5: AC-NET-05
- M1: AC-AUTH-05
- M2: AC-AUTH-06

## [2026-03-06 T5a] Auth 启动 fail-closed 配置

**关键发现**:
- 认证配置层必须先区分 `DEVE_ENV`，否则缺失生产密钥时会意外滑落到开发默认凭据，破坏 fail-closed 原则
- 更稳妥的判定顺序是先检查 `AUTH_SECRET` / `AUTH_PASS` 是否齐备，再决定是报错还是仅在显式 `development` 下回退
- 一旦显式提供密钥，应无条件优先使用环境值，这样开发/生产都能共享同一条正常加载路径

**实现约定**:
- `DEVE_ENV` 缺省值固定为 `production`
- 非 `development` 模式下，缺少任一认证密钥都返回统一错误 `Production mode requires AUTH_SECRET and AUTH_PASS`
- `development` 模式下，仅在缺失密钥时记录警告并回退到 `dev_default()`

## [2026-03-06 T5b] Cookie 安全属性环境驱动

**关键发现**:
- 登录 Cookie 的 `Secure` 不能硬编码为 `false`，否则在 HTTPS 代理与直连 HTTPS 下都会偏离验收要求
- 这里最稳的缺省值应是 fail-closed：`HTTPS_ENABLED` 未配置时按 `true` 处理，避免生产遗漏配置导致降级
- `HttpOnly` 与 `SameSite::Strict` 应保持显式设置，避免未来重构时被链式调用顺序或默认值误伤

**实现约定**:
- `HTTPS_ENABLED` 的真值仅接受 `true` 与 `1`
- `HTTPS_ENABLED` 缺省时，认证 Cookie 写入 `Secure`
- 登录 Cookie 固定包含 `HttpOnly; SameSite=Strict`，删除 Cookie 继续保持 `HttpOnly; SameSite=Strict; Max-Age=0`

## [2026-03-06 T5b] Cookie 安全策略环境驱动

**关键发现**:
- Cookie `Secure` 属性必须可配置，因为本地开发HTTP场景与生产HTTPS场景需求不同
- 默认值应保守（`true`），这样即使忘记配置，生产环境也不会意外暴露明文cookie
- `HttpOnly` 与 `SameSite::Strict` 无需配置——它们应始终启用以防XSS与CSRF

**实现约定**:
- `HTTPS_ENABLED` 默认 `true`
- 仅显式设置 `HTTPS_ENABLED=false` 时才关闭 `Secure` 属性
- `HttpOnly` 与 `SameSite::Strict` 硬编码为 `true` / `Strict`
