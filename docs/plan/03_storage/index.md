# 03_storage/index.md - Ledger、Projection 与 Workspace 存储工程蓝图（总骨架）

## Metadata

- `Layer`: `Authority Core`
- `Status`: `Current MUST`
- `Version`: `0.0.1`
- `Last Review`: `2026-06-25`
- `Counterpart Feature`: `docs/features/04_storage.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/07_storage_repo.md`
- `Primary Code Areas`: `crates/core/src/ledger/`, `crates/core/src/ledger/manager/`, `crates/core/src/sync/watcher/`, `crates/core/src/sync/materialize.rs`

> **本章已按 §12 Refactor Target 拆分为四个 runtime 子文件**：
> [authority](./authority.md) · [projection](./projection.md) · [watcher](./watcher.md) · [repair](./repair.md)。
> 本文件承载章节骨架、总览实体、物理布局与跨层边界；各 runtime 专属合同见对应子文件。

## 1. Scope

本章定义：

- Repo authority 如何持久化到 ledger。
- Workspace、Tree、Snapshot、Source Control side tables 如何作为 projection 或 workflow side tables 存在。
- Watcher、projection writeback、repair、backup 如何在存储层协作。

本章不描述按钮、列表、提示语或用户操作示例；这些属于 `docs/features/04_storage.md`。

## 2. Authoritative Entities

### 2.1 Core Stores

- `Store A: Projection Workspaces`
  - `ProjectionLocator(RepoId) -> projection_base`
  - `ProjectionWorkspaceRoot(RepoId) = projection_base/<safe_repo_name>--<repo_id>/`
  - 是 repo-scoped workspace projection 的物理容器，不是 authority。
  - 系统不再定义总 `vault` 根目录；每个本地可写 repo 必须显式绑定 projection base，再计算 repo workspace root。
- `Store B: Local Branch Ledger`
  - `ledger/local/*.redb`
  - 本地唯一可写 authority。
- `Store C: Remote Branch Ledgers`
  - `ledger/remotes/<peer>/*.redb`
  - 远端镜像 authority，仅同步路径可写。

### 2.2 Authority Model

对任意 repo `r`：

```text
L_r = OrderedLog<LedgerEntry>
S_r = Fold(L_r)
P_r = Project(S_r)
Workspace_r = P_r ⊕ D_r
```

其中：

- `L_r`：唯一权威事实日志
- `S_r`：由 ledger fold 得到的逻辑状态
- `P_r`：规范 projection
- `D_r`：已被系统跟踪的 workspace 偏差

> `Content Facts` / `Structure Facts` 的事实分区与非权威 runtime state、Clean File Policy 见 [authority.md#facts-partition](./authority.md#facts-partition)（§2.3–§2.5）。

## 3. Physical Layout

### 3.1 Ledger Layout

- `ledger/local/<repo_name>.redb`
- `ledger/remotes/<peer_name>/<repo_name>.redb`
- `ledger/.host/identity.key`
- `ledger/.host/projection-locators.toml`
- `ledger/backups/<repo_name>-<timestamp>.redb`

### 3.2 Repo Runtime Layout {#repo-runtime-layout}

- `<projection_base>/<safe_repo_name>--<repo_id>/.notegit/`
  - repo keys
  - pending/staged side tables
  - commit/runtime metadata
  - migration archives

约束：

- `.notegit/` **MUST** 被 watcher 忽略。
- `.notegit/` 可以随 repo 备份，但 **MUST NOT** 被跨 repo 复用。
- `.notegit/` 是 Deve-owned repo runtime 目录，当前继续保留该命名。

> Projection Locator Layout（projection-locator-contract）见 [projection.md#projection-locator-contract](./projection.md#projection-locator-contract)（§3.2.1）。

### 3.2.2 Git Mirror Storage Boundary {#git-ecosystem-coexistence}

Git mirror 的生命周期、命令面与失败语义以 `05_diff_logic.md#git-mirror-lifecycle` 为唯一权威。本章只定义存储边界：

- `.notegit/` 是 Deve-owned runtime 目录，保存 ledger-aware workflow state 与必要 side table。
- `.git/` 是 Git ecosystem mirror 目录，只用于复用 Git 工具链、远程托管、审计、备份与发布生态。
- repo-local `.gitignore` **MUST** 忽略 `.notegit/`，避免 Git mirror 泄漏 Deve runtime state。
- watcher、scan、sync 与 projection rebuild **MUST** 按路径段语义忽略 `.notegit/` 与 `.git/` 内部路径。
- Deve core **MUST NOT** 使用 `.git/` 作为 repo authority、runtime side table 或 hidden metadata 目录。
- `.git/` 不得参与 ledger fold、repo scope 解析、stage/commit authority 或 repair。

### 3.3 Collision Rules

- 同一 branch 下，同名但不同 repo identity 的 `.redb` 文件 **MUST** 自动重命名。
- 物理文件名冲突不得改变逻辑 repo identity。

### 3.4 Browser Storage Layering {#browser-storage-layering}

浏览器端分层存储属于本章约束的一部分：

- `localStorage`
  - 纯 UI 偏好
- `IndexedDB`
  - repo-scoped metadata、vector、cache metadata
- `WebCrypto`
  - 私钥材料
- authority 业务数据
  - 仍在 server ledger，不在浏览器长期持有

### 3.4.1 Trust Registration Flow

浏览器 repo-scoped trust registration 必须按以下顺序进行：

1. 先确认 user session 已通过 `/api/auth/me` 或 ws 入口鉴权。
2. 读取 IndexedDB 中该 `repo_id` 的 browser peer metadata。
3. 若 metadata 不存在，则用 `WebCrypto` 生成 repo-scoped keypair，私钥必须 `extractable: false`。
4. 把 peer public metadata 写入 IndexedDB；私钥材料仅存于 WebCrypto。
5. 发送 repo-scoped `SyncHello` 完成 browser peer 注册。
6. 只有当 session、IndexedDB metadata、repo key 三者都齐备时，浏览器才允许进入可同步写态。

> Browser storage recovery semantics（站点数据被清理、IndexedDB/WebCrypto 缺失降级）见 [repair.md](./repair.md)（§3.4.2 Recovery Semantics）。

### 3.5 Internal Path Normalization {#internal-path-normalization}

- 所有持久化到 ledger、projection table、side table、sync payload 的路径字符串 **MUST** 统一为 forward slash。
- 规范化边界：
  - 进入系统：watcher、file dialog、CLI path 参数进入 authority/runtime 前，必须调用 `to_forward_slash`。
  - 离开系统：仅在直接调用 OS 文件系统 API 的瞬间，才允许转换回 native separator。
- 禁止：
  - 在不同表中混用 `\` 与 `/`
  - 依赖 display path 作为 authority key
  - 通过字符串替换拼接路径身份

## 5. State Machines

### 5.1 Repo Mount Lifecycle

```text
RepoDiscovered
  -> RepoOpened
  -> RuntimeTablesReady
  -> ProjectionLocated
  -> ProjectionReady
  -> WatcherReady
  -> Mounted
```

约束：

- `ProjectionLocated` 必须验证 repo-scoped Projection Locator。
- `WatcherReady` 是打开 repo 的最后一步。
- watcher 初始化失败 **MUST** fail-closed。

> Write Lifecycle（§5.2）见 [authority.md](./authority.md)；External Edit Lifecycle（§5.3）见 [watcher.md](./watcher.md)。

## 10. Forbidden Patterns（跨层）

以下为跨 runtime 的禁止项；各 runtime 专属禁止项见对应子文件的 Forbidden 段：

- 原地修改 authority 状态。
- 通过全局 `vault_path` 或 `ledger_dir` 隐式推断 repo projection base / workspace root。

各 runtime 专属禁止项：

- authority 专属（metadata/path table 直接 rename/move/delete、未经 Stage/Commit 入 ledger）见 [authority.md](./authority.md)。
- projection 专属（Projection Workspace 作为真值源）见 [projection.md](./projection.md)。
- repair 专属（side table 或 snapshot 成为删除真源）见 [repair.md](./repair.md)。

## 11. Runtime Boundary

各 runtime layer 的完整边界分布在子文件；本节给出总览与 Repo Runtime Integration。

- §11.1 Authority Layer → [authority.md](./authority.md)
- §11.2 Projection / Workspace Layer → [projection.md](./projection.md)
- §11.3 Watcher Layer → [watcher.md](./watcher.md)

### 11.4 Repo Runtime Integration

- 负责 repo open/close、runtime directory bootstrap、catalog repair 与各层生命周期编排。
- 该层只能编排 authority/projection/watcher/repair，不得把 side table 升格为 authority。

## 12. Refactor Target

长期应显式形成四个 infra 子系统（本章的文件夹化即按此四层落地）：

- `authority_storage_runtime` → [authority.md](./authority.md)
- `projection_persistence_runtime` → [projection.md](./projection.md)
- `watcher_runtime` → [watcher.md](./watcher.md)
- `repair_runtime` → [repair.md](./repair.md)

实现必须按这四层收敛；任何 manager/helper 只能作为其中一层的内部细节，不得跨层持有隐式 authority。

## 本章相关命令

- 无

## 本章相关配置

- `snapshot_depth`
- backup / retention 相关配置
- `projection.locators`
- `ledger.path`
