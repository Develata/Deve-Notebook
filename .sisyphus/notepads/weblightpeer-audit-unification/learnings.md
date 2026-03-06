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

## [2026-03-06 T5] Auth 启动与部署契约 fail-closed

**关键发现**:
- `AuthConfig::from_env()` 必须把 `DEVE_ENV` 未设置视为 `production`，否则启动层很容易继续沿用 `dev_default()` 形成静默回退
- 登录 Cookie 与登出 Cookie 必须共享同一套 `HTTPS_ENABLED` 驱动属性；只修登录而不修登出会留下浏览器端属性不一致的边界缺口
- `tower_http::cors` 在 `allow_credentials(true)` 下不能再搭配 `allow_headers(Any)`，否则开发模式启动会 panic；必须改为显式 header 白名单

**环境变量约定**:
- `DEVE_ENV=production|development`：控制 fail-closed 与显式开发模式分支，默认 `production`
- `AUTH_SECRET` + `AUTH_PASS`：生产启动必需；缺任一项即记录 `ERROR: Production mode requires AUTH_SECRET and AUTH_PASS` 并退出
- `HTTPS_ENABLED`：控制认证 Cookie 与登出清理 Cookie 是否带 `Secure`，默认 `true`
- `ALLOWED_ORIGINS`：逗号分隔的 CORS allow list；未设置时默认拒绝跨域，开发模式也不再隐式注入 localhost

**边界情况**:
- 手动 QA 若通过 `cargo run` 执行，在 Windows 上需要把 `TMP`/`TEMP` 指到 `E:\gitclone\Deve-Notebook\target\tmp`，否则可能被 `C:` 临时目录空间耗尽阻塞
- rust-analyzer 的宏展开曾因 `C:` 临时目录不足报 `serde_derive` 扩展失败；清理用户临时目录后，编译/验证恢复正常，但 LSP 进程可能仍保留旧错误缓存

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

---

## [2026-03-06 T5c] CORS 环境驱动配置

**关键发现**:
- CORS 允许来源不能硬编码为 `port..=port+4` 的 localhost 扫描列表，这会在反向代理或多前端场景下彻底失效
- 生产环境最安全的策略是"显式白名单，拒绝任何默认"——`ALLOWED_ORIGINS` 缺失时立即 panic，绝不容忍隐式回退
- 开发模式的回退必须伴随警告日志（`tracing::warn!`），确保开发者理解当前使用非生产配置
- 拒绝通配符 `*`：这是常见 CORS 配置错误，必须在启动时就校验并 panic

**实现约定**:
- `ALLOWED_ORIGINS` 接收逗号分隔的完整 origin 字符串（如 `https://app.example.com,https://admin.example.com`）
- Production 模式（`DEVE_ENV=production` 或缺省）必须提供显式 `ALLOWED_ORIGINS`，否则 panic
- Development 模式（`DEVE_ENV=development`）未提供 `ALLOWED_ORIGINS` 时，回退为 `http://localhost:8080,http://127.0.0.1:8080` 并记录警告
- 校验逻辑：拒绝空列表、拒绝通配符 `*`、拒绝无效 origin（`HeaderValue::parse` 失败）

**验收映射**:
- AC-AUTH-04 (CORS 安全配置) ✅ 完全覆盖
- 消除 3001..3005 端口扫描行为（与 T2 network 契约对齐）
- 为反向代理/HTTPS 生产部署提供可验证的确定性 CORS 策略

**性能考虑**:
- `ALLOWED_ORIGINS` 解析发生在服务器启动时，运行时零开销
- 显式 `panic!` 在启动阶段保证 fail-fast，避免运行时 CORS 拒绝的隐蔽故障

**后续协调**:
- T6 浏览器身份实现时，前端连接逻辑应使用同源相对 `/ws` 路径（与 T2 协议契约一致）
- T11 文档同步时，需将 `ALLOWED_ORIGINS` 环境变量写入部署文档

---

## [2026-03-06 T6a] 浏览器存储骨架落位

**关键发现**:
- `apps/web/src/storage/` 骨架应先只承载类型边界与中文文档，避免在未接入 `use_core` 前引入级联编译错误
- `PrefsStorage` 与 `IdentityStorage` 的职责边界必须先固定：前者只包裹 `localStorage` 偏好，后者只负责 `IndexedDB + WebCrypto` 的 repo-scoped identity 占位
- 骨架阶段用 `todo!()` 保留 API 形状，比提前写伪实现更稳妥，可避免错误的浏览器安全语义固化

**实现约定**:
- `StorageError` 统一承载 `Unavailable`、`InvalidInput`、`Backend` 三类前端存储错误边界
- `PeerIdentity` 暂以 `public_key: Vec<u8>` 与 `metadata: serde_json::Value` 作为跨层占位，等待 T6b 接入真实 JS bridge


## [2026-03-06 T6] 浏览器身份持久化基座

**关键发现**:
- 现有 `use_core` 把浏览器 peer 私钥字节直接塞进 `localStorage`，既破坏存储分层，也会在受限上下文触发 panic。
- 通过 `wasm-bindgen` 内联 JS 桥接可把 IndexedDB/WebCrypto 的异步复杂度封装在单独模块内，Rust 侧只保留公钥、repo metadata 与降级判定。
- `trunk build` 在默认 `apps/web/dist` 路径会因 Windows 文件系统移动 `.stage/js` 目录报 `os error 5`，改为输出到工作区临时目录可稳定完成构建验证。

**实现约定**:
- `apps/web/src/storage/mod.rs` 定义 `StorageCapabilities`、`DegradedSyncMode`、`RepoMetadata` 与统一 `StorageError`。
- `apps/web/src/storage/js_bridge.rs` 负责 WebCrypto Ed25519 生成/签名、IndexedDB 三类 store（`peer_identity` / `repo_meta` / `offline_cache`）与能力探测。
- `use_core` 通过 `storage_runtime` 初始化 repo-scoped identity 和向量缓存；握手 effect 改为读取 WebCrypto 签名结果，不再依赖 Rust `IdentityKeyPair`。

**降级行为**:
- 只要 WebCrypto、IndexedDB 或 Ed25519 任一不可用，就设置 `DegradedSyncMode`，顶部横幅说明“允许查看与拉取，禁止 Peer 注册、编辑提交与 SyncPush”。
- 降级态仍请求 `ListDocs` / `ListRepos`，因此 dashboard 与只读拉取不受阻塞。
- `is_spectator` 现在同时覆盖远端影子分支与浏览器持久存储降级，确保编辑入口自动进入只读保护。

---

## [2026-03-06 T6c] use_core 存储集成

**关键发现**:
- T6a 已经完成了实际的集成工作（`init_storage_runtime` 调用已就位），T6c 只需补充文档注释
- 原 `load_or_generate_identity()` 与 `IDENTITY_KEY_STORAGE` 常量已在 T6a 中被移除
- `init_storage_runtime(&signals)` 返回 `(identity, repo_vector)` 元组，直接传递给 effects

**实现约定**:
- 中文注释必须解释"为何"而非"做什么"：重点说明 localStorage 的职责边界与 WebCrypto 的安全属性
- `use_core()` 保持单一职责：组装信号、初始化存储、设置效果、构造状态对象
- 降级模式的 UI 提示推迟到 T6d（effects.rs 层处理 banner 显示）

**后续协调**:
- T6d 需在 `effects.rs` 检测 `degraded_sync_mode` 信号并触发 UI banner
- T7 需确保 `init_storage_runtime` 返回的 `repo_vector` 能正确传递给 `SyncHello` 握手

---

## [2026-03-06 T6d] 降级模式 UI 提示

**关键发现**:
- `DegradedSyncMode` 有 `reason: String` 字段和 `banner_text()` 方法，但 banner 应保持简洁
- Effect 直接使用 `format!("存储受限（{}），当前处于只读模式", mode.reason)` 而非调用 `banner_text()`
- 降级检测 Effect 必须放在 `setup_message_effect` 而非 `setup_handshake_effect`，因为 signals 在 message effect 中可用

**实现约定**:
- Effect 监听 `degraded_sync_mode.get()`，根据 `Some(mode)` / `None` 动态设置 `sync_banner`
- 中文 banner 文本格式：`"存储受限（{reason}），当前处于只读模式"`
- 注释强调"为何需要 UI 可见警告"：避免用户误以为仍可编辑

**用户体验考虑**:
- Banner 应常驻可见（顶部），而非 toast 消失通知
- 降级原因文本需清晰但简洁（详细说明可放在设置或帮助页面）
- 未来可考虑添加"了解更多"链接指向 FAQ

**后续协调**:
- T7 需确保降级模式下，`SyncHello` 仍能发送但 `SyncPush` 被禁止
- T8 需确保登录 UI 不受降级影响（JWT Cookie 独立于 peer identity）
