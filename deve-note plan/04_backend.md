# 第三章：统一后端架构 (The Vibranium P2P Backend)

*(核心演进：从单体 Ledger 走向 P2P Repository Mesh)*

* **Repository Manager (仓库管理器)**：
    * **职责**：管理 `Local Repo` (Store B) 和 `Shadow Repos` (Store C)。
    * **Routing**：VFS 根据 UI 上下文路由到对应的 `.redb` 实例。

* **存储（三位一体隔离）**：
    * **Store B (Local)**：`local.redb`，Local Write Only。
    * **Store C (Shadows)**：`remotes/peer_X.redb`，Receive Only。
    * **Snapshot**：每个 Repo 独立维护自己的 Snapshot 链。

* **同步协议 (Gossip Protocol)**：
    * **Version Vector**：唯一真理。
    * **Transport Layer**：
        *   **Relay (Phase 1)**：WebSocket via Server。
        *   **Direct (Phase 2)**：WebRTC/QUIC 预留。
    *   **同步模式 (Sync Mode Configuration)**：
        *   **Auto (Default)**：后台自动拉取与合并（无冲突时）。
        *   **Manual (StrictMode)**：仅交换 Vector；Fetch/Merge 必须显式触发。
    *   **流控**：
        *   MUST 有背压。
        *   **Desktop MUST 支持离线**。
        *   **Web (Exception)**：**严禁离线编辑**。断网即锁屏。
    *   **失败语义**：重连后最终一致。

* **和解与合并策略 (Reconciliation & Merge Strategy)**：
    *   **Store C -> Store B (Remote Merge)**：
        *   **Conflict Handling**：Manual Mode 下检测冲突 MUST 报错并强制手动解决。
    *   **Store A -> Store B (Local Watcher)**：
        *   Debounce -> Inode Check -> Append Ops。
    *   **约束**：MUST 幂等；MUST 识别重命名 (Inode/UUID)；MUST 防抖。

* **SyncManager (同步管理器)**：
    *   负责启动扫描、网络调度和 UI 通知。

* **运行时安全（插件/脚本）**：
    *   Host Functions MUST 做 Capability 校验 (default deny)。

* **跨平台规范化 (Cross-Platform Normalization)**：
    *   **MUST Form-First**：所有输入转为 Linux 风格路径。
    *   **外壳适配**：OS API 调用前还原为原生格式。

### 认证与登录 (Auth & Login)

*   **12-Factor Auth**：配置通过环境变量注入，No Init UI。
*   **安全**：HTTPS；抗 CSRF；速率限制。
*   **体验**：2FA 可选；会话管理。
