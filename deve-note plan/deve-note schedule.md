# Deve-Note 开发计划表 (Modular & P2P)

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

## 阶段 1: P2P 基础设施 (P2P Infrastructure) - [CURRENT]
**目标**: 实现 Trinity Isolation、Relay 网络与 Gossip 协议。

- [ ] **Repository Manager (仓库管理器)**:
    - [ ] 重构 `Ledger` 为 `RepoManager` (Store B)。
    - [ ] 实现 **Trinity Isolation** 目录结构 (Vault / Local DB / Shadow DBs)。
    - [ ] 实现 `Shadow Repo` (Store C) 的独立加载与只读控制。
- [ ] **网络层 (Networking)**:
    - [ ] 实现 `Relay Server` (Always-on Peer)。
    - [ ] 实现 `WebSocket` 客户端 (重连、心跳、鉴权)。
    - [ ] **Web Dashboard API**: 实现 Server 端 WebSocket 接口，支持 Web 端读写内存/DB。
- [ ] **同步协议 (Gossip)**:
    - [ ] 实现 `Version Vector` 结构与交换逻辑 (Manifest)。
    - [ ] 实现 `Diff & Fetch` (增量拉取 Ops)。
    - [ ] 实现 `SYNC_MODE` 配置 (Auto/Manual)。
- [ ] **CLI P2P 验证**:
    - [ ] `deve-note verify-p2p`: 模拟多端同步，验证 Shadow Repo 数据一致性。

## 阶段 2: 驾驶舱与 Web 面板 (Cockpit & Web Dashboard)
**目标**: 构建 UI，实现分支切换、Web 瘦客户端限制。

- [ ] **前端基础 (Leptos)**:
    - [ ] Leptos v0.7 + Tailwind CSS 初始化。
    - [ ] Slot Layout 布局实现。
    - [ ] `leptos_i18n` 集成。
- [ ] **Web Dashboard 特性 (Thin Client)**:
    - [ ] **RAM-Only Mode**: 确保 Web 端不使用 IndexedDB，状态刷新即失。
    - [ ] **Disconnect Lock**: 实现 WebSocket 断连即锁屏逻辑。
    - [ ] **Server RPC**: 前端通过 WS 调用 Server Host Functions。
- [ ] **编辑器内核**:
    - [ ] CodeMirror 6 组件封装。
    - [ ] Loro CRDT Wasm 绑定。
    - [ ] KaTeX 数学公式渲染。
- [ ] **P2P 交互组件**:
    - [ ] **Branch Switcher**: 切换 Local/Peer 视图。
    - [ ] **Spectator Mode**: 实现只读观测模式 (灰色背景)。
    - [ ] **Manual Merge**: 实现手动合并冲突 UI。

## 阶段 3: 双引擎运行时与扩展 (Dual-Engine & Extensions)
**目标**: 插件系统、Podman 运行时、发布。

- [ ] **插件系统 (Engine A: Wasm)**:
    - [ ] 集成 `Rhai` / `Extism`。
    - [ ] 实现 Capability Manifest 校验。
- [ ] **计算运行时 (Engine B: Podman)**:
    - [ ] **Host Integration**: Desktop/Server 端集成 Podman 命令调用。
    - [ ] **Remote Execution**: Web 端请求转发至 Server 执行代码块。
    - [ ] 实现 `python`/`r` fenced block 执行逻辑。
- [ ] **高级特性**:
    - [ ] Tantivy 全文检索 (Standard Mode)。
    - [ ] Git Sync 场景支持。
- [ ] **发布与运维**:
    - [ ] GitHub Actions 跨平台构建。
    - [ ] Docker 镜像 (GHCR, 支持数据卷挂载)。