# 核心架构进度 (Core Architecture Schedule)

> 涵盖计划: 04_storage, 05_network, 06_repository, 07_diff_logic, 09_auth

## 1. 存储层 (Storage - Plan 04)
- [x] **Store A (Vault)**: 本地 Markdown 工作区投影 ($W_{user}$).
- [x] **Store B (Local Branch)**: 本地权威数据库 (`ledger/local/`).
- [x] **Store C (Remote Branches)**: 远端影子数据库 (`ledger/remotes/<PeerID>/`).
- [x] **Trinity Isolation**: 严格的三库物理隔离结构.
- [x] **Repo Filename**: 格式 `repo_name.redb`，支持冲突重命名.
- [x] **Indexing Tables**:
    - [x] `DOCID_TO_PATH`: `u128 -> &str`.
    - [x] `PATH_TO_DOCID`: `&str -> u128`.
    - [x] `INODE_TO_DOCID`: `u128 -> u128` (Rename tracking).
    - [x] `LEDGER_OPS`: 全局有序日志.
    - [x] `SNAPSHOT_INDEX` / `SNAPSHOT_DATA`: 快照双表结构.
- [x] **Atomic Persistence**: `Op -> Ledger -> Snapshot -> Disk` 写入流程.
- [x] **Path Normalization**: 强制使用 Linux 风格正斜杠 (`/`).
- [x] **Watcher**: 文件变更监听 (>500ms Debounce).

## 2. 网络与同步 (Network - Plan 05)
- [x] **Peer Identity**: 基于 Ed25519 的身份 ID 生成 (`SHA256(PubKey)[0..12]`).
- [x] **Handshake (TOFU)**: 首次连接交换公钥并验证签名.
- [x] **Protocol Format**: 节点间使用 Bincode，客户端使用 JSON.
- [x] **E2EE Encryption**: 传输层 `RepoKey` 加密 (AES-256-GCM).
- [x] **Vector Clock**: 实现逻辑时钟与差异计算.
- [x] **Gossip Sync**: 基于 VC 的增量 Ops 同步.
- [x] **Direct Write**: 接收端直接写入 Shadow Repo (无冲突).
- [x] **Snapshot Sync**: 差异 > 1000 Ops 时自动切换为快照传输.
- [x] **Indirect Sync**: 信任边界检查 (Trust Boundary)，Relay 节点不解密.

## 3. 仓库管理 (Repository - Plan 06)
- [x] **Repo Identification**: 基于 URL Hash 唯一标识仓库.
- [x] **Branch Mapping**: `local` 对应本地，`remotes/peer_id` 对应远端.
- [x] **Switching**: 支持在不同 Repo 实例间切换上下文.
- [x] **Spectator Mode**: 
    - [x] 允许浏览远端内容.
    - [x] 禁止修改 (Read-Only).
- [x] **Peer Deletion**: 物理删除无效的 Remote Branch.

## 4. Diff 与合并 (Diff Logic - Plan 07)
- [x] **Text Diff**: 使用 Myers 算法 (基于 UTF-16 code unit 索引).
- [x] **3-Way Merge**:
    - [x] **LCA**: 最近共同祖先计算.
    - [x] **Conflict Detection**: 自动检测结构性冲突.
- [x] **Merge UI**:
    - [x] **MergeModal**: 待处理操作列表与确认/丢弃按钮.
    - [x] **MergePanel**: 自动/手动模式切换.
- [x] **Cross-Branch Diff**: 支持 Local vs Remote 的差异计算.

## 5. 认证与安全 (Auth - Plan 09)
- [x] **No Init UI**: 配置通过环境变量注入 (`AUTH_SECRET`).
- [x] **JWT Auth**: 无状态 Token 验证.
- [x] **WebSocket Auth**: 握手阶段验证 Ticket.
- [ ] **Argon2**: 密码哈希存储 (目前仅明文或简单哈希，需确认).
- [ ] **Rate Limiting**: 接口限流.
- [ ] **Localhost Policy**: `AUTH_ALLOW_ANONYMOUS_LOCALHOST` 配置支持.
