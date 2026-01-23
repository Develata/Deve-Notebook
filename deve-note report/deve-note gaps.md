# Deve-Note Gap Analysis (差距分析)

本文档基于 `deve-note plan` (v0.0.1) 与当前代码库 (`crates/core`, `apps/cli`, `apps/web`) 的对比，识别出以下实现差距。

## 1. 核心架构 (Core Architecture)

### 1.1 Ledger Schema
*   **Plan**: `04_storage.md` 定义了 `LEDGER_OPS`, `DOC_OPS`, `SNAPSHOTS`。
*   **Implementation**: `crates/core/src/ledger/schema.rs` 实现了 `LEDGER_OPS`, `DOCID_TO_PATH`, `PATH_TO_DOCID`。
*   **Gap**:
    *   `SNAPSHOTS` 表在代码中被拆分为 `SNAPSHOT_INDEX` 和 `SNAPSHOT_DATA` (这是一个优化，符合预期，但需文档同步)。
    *   `PEER_DOC_SEQ` 表已定义但未见核心逻辑使用 (Pending Integration)。
    *   **Repo Metadata**: 代码中实现了 `REPO_METADATA`，但 `RepoInfo` 结构体缺少 `URL` 字段的强制校验逻辑。

### 1.2 3-Way Merge Engine
*   **Plan**: `07_diff_logic.md` 要求实现 Myers Diff 和 3-Way Merge，并明确了 "Atomic Persistence" (原子持久化) 要求。
*   **Implementation**: `crates/core/src/ledger/merge.rs` 仅实现了框架 (`MergeEngine`)。
    *   `find_lca`: 实现了基础的 VersionVector 交集计算。
    *   `reconstruct_state_at`: 实现了基于 Ops 的状态重建。
    *   `merge_commits`: **仅有占位符逻辑**。目前的实现是 "If local == remote return success"，缺乏真正的 Diff3 算法集成。
*   **Critical Gap**: **缺少真正的 Diff3 实现**。如果不解决，无法处理任何并发修改。需集成 `diff3` crate 或移植相关算法。

## 2. P2P 网络 (P2P Network)

### 2.1 身份认证 (Authentication)
*   **Plan**: `09_auth.md` 要求 Argon2 + Ed25519 双重认证。
*   **Implementation**:
    *   `apps/cli/src/server/ws.rs` 实现了基础的 WebSocket 握手。
    *   `crates/core/src/security` 包含 `keypair.rs` (Ed25519)。
    *   **Gap**: **Argon2 密码验证逻辑缺失**。目前仅校验了 PeerId 签名，未实现 Admin Password (`AUTH_SECRET`) 的全链路校验。

### 2.2 同步协议 (Sync Protocol)
*   **Plan**: `05_network.md` 定义了 `SyncHello` -> `SyncOffer` -> `SyncRequest` -> `SyncPush` 的完整流。
*   **Implementation**: `apps/cli/src/server/handlers/sync.rs` 实现了部分逻辑。
    *   **Gap**: **Gossip 逻辑不完整**。目前代码能处理 `SyncPush` (接收 Ops)，但缺乏主动发起 `SyncOffer` (基于 Vector Clock 差异推送) 的完整调度器。

## 3. UI 实现 (UI Implementation)

### 3.1 移动端适配 (Mobile UI)
*   **Plan**: `08_ui_design.md` (Update) 明确了 "Hamburger Menu", "Single Column", "Vertical Diff" 的设计。
*   **Implementation**: `apps/web` 目前主要是 Desktop 布局 (5-Column)。
    *   **Gap**: **缺少 Responsive Design (响应式设计)**。Tailwind 类 (`md:`, `lg:`) 尚未全面覆盖侧边栏的折叠逻辑。需补充 Drawer 组件。

### 3.2 插件架构 (Plugin Architecture)
*   **Plan**: `11_plugins.md` (Update) 定义了 "Hybrid Architecture" (WASM Logic + JSON UI Protocol)。
*   **Implementation**: `crates/core/src/plugin` 仅有 `runtime.rs` 和 `rhai_v1.rs`。
    *   **Gap**: **WASM 运行时集成缺失**。代码中虽有 `loader.rs`，但未见 `wasmtime` 或 `extism` 的实际调用代码。JSON UI Protocol 尚未定义。

## 4. 总结与建议

| 模块 | 优先级 | 建议行动 |
| :--- | :--- | :--- |
| **Merge Engine** | **Critical** | 引入 `diff3` 算法，替换 `merge.rs` 中的占位逻辑。 |
| **Sync Gossip** | **High** | 完善 `SyncManager`，实现主动推送差异的定时任务。 |
| **Mobile UI** | **Medium** | 在 `apps/web` 中引入 Drawer 组件，适配小屏幕布局。 |
| **Auth** | **Medium** | 在 WebSocket 握手中加入 `AUTH_SECRET` 校验。 |
| **Plugins** | **Low** | 暂时搁置 WASM，优先完善 Rhai 脚本支持。 |
