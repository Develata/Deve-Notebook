# 核心变更理念：Git-Flow P2P 架构 (Core Architecture Philosophy)

本章节定义了从传统 Client-Server 向 **Relay-First P2P Mesh** 演进的核心原则。

### 1. 拓扑定义：P2P 三角与 Web 面板 (The P2P Triangle & Web Dashboard)
* **P2P Mesh (对等网络)**：
    * **核心节点**：仅包含 **Desktop Native (PC/Mac)**、**Mobile Native (iOS/Android)** 和 **Server (Linux)**。
    * **机制**：这三方拥有独立的 `PeerID` 和本地数据库 (Redb)，通过 Gossip 协议交换数据。
    * **Server 角色**：Server 在 P2P 网络中充当 **Always-on Relay Peer** (全天候中继/备份节点)。
* **Web Client (服务器面板)**：
    * **定位**：Web 端**不是**一个独立的 P2P 节点，而是 **Server 节点的远程操作面板 (Remote Dashboard)**。
    * **数据源**：Web 端直接通过 WebSocket 读写 Server 的内存/数据库。Web 端显示的 "Local" 即为 Server 的 `local.redb`。
    * **存储**：**Stateless / RAM-Only (纯内存)**。Web 端严禁使用 IndexedDB/LocalStorage 存储业务数据。它只是 Server 状态的“易失性投影”。
    * **连接约束**：Web 端必须保持与 Server 的 WebSocket 连接才能工作。**断连即锁屏**，严禁离线编辑。

* **连接策略**：
    * **Core (默认)**：基于 Relay 的连接。所有 Peer 默认连接 Server，通过 Server 转发数据包。
    * **Extension (接口)**：预留 `Transport` 接口。允许未来通过插件实现 IPv4/IPv6 直连或 NAT 穿透。

### 2. 数据存储模型：三份数据隔离 (The Trinity Isolation)
实现“绝对数据主权”和“零污染”。本地不再是单体数据库，而是物理隔离的存储结构：
* **Store A (Vault)**: 本地 Markdown 文件（工作区）。由 Store B 实时投影生成，用户可读写。
* **Store B (Local Master DB)**: 本地核心数据库（`local.redb`）。**只有本地用户的操作能写入此库**。
* **Store C (Remote Shadow DBs)**: 远端影子数据库集合（`/data/ledger/remotes/peer_X.redb`）。
    * 来自 Peer X 的数据**只写入** `remotes/peer_X.redb`，**绝对禁止**自动合并进 Store B。
    * **Clean File Policy (干净文件策略)**:
        * **Implicit Tracking (隐式追踪)**: `DocId` (UUID) 仅存储在内部数据库 (`Ledger/RepoManager`) 中，以文件路径或 Inode 关联。
        * **No Injection (零注入)**: 系统**严禁**自动向 Markdown 文件头注入 `uuid` 或其他 Frontmatter 元数据。Markdown 文件必须保持用户原始书写的状态。
        * **Deceptive Metadata**: 如果用户手动书写 Frontmatter，仅用于渲染展示或兼容性，不是系统追踪的真理源 (Source of Truth)。

### 3. 同步协议：基于版本向量的 Gossip (Version Vector Gossip)
* **废弃**：简单的广播 (Broadcast) 和时间戳 (Timestamp)。
* **采用**：
    * **元数据**：使用 **Sequence Number (操作序列号)** 作为单一真理。
    * **握手流程**：Exchange Manifest (Version Vector) -> Diff & Fetch -> Update Shadow DB。

### 4. 交互模式：VS Code 式分支切换
* **分支切换器**：UI 提供类似 VS Code 左下角的分支切换功能。
    * 切到 `Local`：读写 Store B + Store A。
    * 切到 `Peer Mobile`：**只读模式 (Spectator Mode)**。VFS 挂载点切换，直接从 `remotes/peer_mobile.redb` 读取并在内存中生成文件树。

---

## Phase 0: 核心验证原型 (Headless Core Verification) - [必选项]

在构建任何 UI 之前，**必须**先行构建并通过验证的纯命令行原型（Headless CLI）。

*   **目标**：验证 `Reconciliation`（和解）的鲁棒性，确保 Ledger 与大量不可靠外部写入并存时数据不丢失。
*   **功能清单**：
    1.  `init`: 初始化 Ledger 和 Vault。
    2.  `watch`: 启动文件监听，能够正确处理 `vim`/`vscode`/`nano` 的保存行为（含重命名/原子写入）。
    3.  `append`: 通过 API 追加 Ops，验证 Vault 能否正确更新。
*   **验收标准**：
    *   **双向同步闭环**：`VS Code 修改 -> Watcher -> Ledger -> Vault 更新` 必须稳定，无死循环。
    *   **重命名测试**：在此阶段必须解决“文件重命名被识别为删除+新建”导致的 DocId 丢失问题（实现 Inode/FileID 追踪）。
