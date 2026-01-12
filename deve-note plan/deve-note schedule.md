# Deve-Note 开发计划表 (P2P Refactored)

**预计总时长**: 8-12 周
**当前阶段**: Phase 1 (P2P Infrastructure)

## 阶段 0: 钢铁核心 (Headless Core) - [COMPLETED]
**目标**: 验证双向同步闭环 (Ledger <-> Vault) 与核心数据结构。

- [x] **基础设施**:
    - [x] 搭建 Rust Workspace。
    - [x] 实现 `Ledger` 结构体 (Redb + Loro CRDT)。
    - [x] 实现 `VFS` 层 (DocId 分配与路径映射)。
- [x] **和解引擎 (Reconciliation)**:
    - [x] 实现 `notify` 文件监听。
    - [x] 实现 Inode 追踪与防抖 (Debounce)。
    - [x] 实现 Diff-to-Ops 转换 (Dissimilar)。
- [x] **CLI 工具**:
    - [x] `deve-note init`: 初始化仓库。
    - [x] `deve-note watch`: 启动监听与同步。
    - [x] `deve-note append`: 模拟 API 写入。
- [x] **验证通过**:
    - [x] VS Code 修改 -> Ledger 记录。
    - [x] Ledger 回放 -> Vault 更新。
    - [x] OS 重命名 -> DocId 保持。

## 阶段 1: P2P 基础设施 (P2P Infrastructure) - [CURRENT]
**目标**: 实现 Repository Manager、Relay 网络与 Gossip 协议。

- [ ] **Repository Manager (仓库管理器)**:
    - [ ] 重构 `Ledger` 为 `RepoManager`。
    - [ ] 实现 **Trinity Isolation** (Store A/B/C) 目录结构。
    - [ ] 实现 `Local Repo` (读写) 与 `Shadow Repo` (只读) 的差异化加载。
- [ ] **网络层 (Networking)**:
    - [ ] 实现 `Relay Server` (无状态转发 + 离线邮箱)。
    - [ ] 实现 `WebSocket` 客户端 (重连、心跳、鉴权)。
    - [ ] 实现 `Transport` Trait (预留 Direct 扩展)。
- [ ] **同步协议 (Gossip)**:
    - [ ] 实现 `Version Vector` 结构与交换逻辑 (Manifest)。
    - [ ] 实现 `Diff & Fetch` (增量拉取 Ops)。
    - [ ] 实现 `SYNC_MODE` 配置 (Auto/Manual)。
- [ ] **CLI P2P 验证**:
    - [ ] `deve-note verify-p2p`: 模拟多端同步，验证 Shadow Repo 数据一致性。

## 阶段 2: 驾驶舱与 P2P 交互 (Cockpit & Merge)
**目标**: 构建 UI，实现分支切换与 Spectator Mode。

- [ ] **前端基础**:
    - [ ] Leptos v0.7 + Tailwind CSS 初始化。
    - [ ] 集成 `leptos_i18n`。
    - [ ] 实现 Slot Layout 布局。
- [ ] **编辑器**:
    - [ ] CodeMirror 6 组件封装。
    - [ ] 绑定 Loro CRDT (Wasm)。
    - [ ] Markdown/Latex 渲染支持。
- [ ] **P2P 交互组件**:
    - [ ] **Branch Switcher**: 状态栏组件，切换 Local/Peer 视图。
    - [ ] **Spectator Mode**: 实现只读锁定与灰色背景 UI。
    - [ ] **Diff View**: 实现简单的双栏文本对比。
- [ ] **合并逻辑**:
    - [ ] 实现 Manual Merge 指令 (CRDT Merge)。
    - [ ] 实现冲突检测与报错 (Manual Mode)。

## 阶段 3: 扩展与鲁棒性 (Extensions & Polish)
**目标**: 插件系统、性能优化与发布。

- [ ] **性能优化**:
    - [ ] 实现 `Low-Spec` Profile (CSR Only, No SSR)。
    - [ ] Ledger 快照裁剪 (Pruning)。
- [ ] **插件系统**:
    - [ ] 集成 `Rhai` 脚本引擎。
    - [ ] 实现 Host Functions 与 Capability 检查。
- [ ] **高级特性**:
    - [ ] Tantivy 全文检索 (Standard Mode)。
    - [ ] Git Sync 场景支持 (Scenario 3)。
- [ ] **发布**:
    - [ ] GitHub Actions (跨平台构建)。
    - [ ] Docker 镜像。