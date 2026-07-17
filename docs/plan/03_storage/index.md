# 03_storage/index.md - Ledger、Projection 与 Workspace 存储工程蓝图（总骨架）

## Metadata

- `Layer`: `Authority Core`
- `Status`: `Current MUST`
- `Version`: `0.0.1`
- `Last Review`: `2026-07-17`
- `Counterpart Feature`: `docs/features/04_storage.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/07_storage_repo.md`
- `Primary Code Areas`: `crates/core/src/ledger/`, `crates/core/src/ledger/manager/`, `crates/core/src/sync/watcher/`, `crates/core/src/sync/materialize.rs`

> **本章已按 §12 Refactor Target 拆分为四个 runtime 子文件**：
> [authority](./authority.md) · [projection](./projection.md) · [watcher](./watcher.md) · [repair](./repair.md)。
> 本文件承载章节骨架、总览实体、物理布局与跨层边界；各 runtime 专属合同见对应子文件。
> `projection_locator_runtime` 是 `projection_persistence_runtime` 内独立命名的 host-local
> 子 runtime，不增加第五个 storage authority / infra 层；其所有权与禁止跨界见
> [projection.md#projection-locator-contract](./projection.md#projection-locator-contract)。

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

- `ledger/local/<repo_id>.redb`
- `ledger/remotes/<peer_id>/<repo_id>.redb`
- `ledger/.host/identity.key`
- `ledger/.host/projection-locators.toml`
- `ledger/backups/<repo_id>-<timestamp>.redb`

Physical filenames use UUID identity. `RepoNameBinding.repo_name` is a display
alias and selector hint; it never determines a database filename.

### 3.1.1 Remote Import Runtime Layout {#remote-import-runtime-layout}

Remote Import capture is host-only, immutable runtime state:

```text
ledger/.host/remote-imports/<repo_id>/<session_id>/
  source-manifest.json
  candidates/<revision>.json
  blobs/<sha256>
```

- `source-manifest.json` is deterministic JSON schema v1; `blobs/<sha256>` are
  content-addressed immutable bytes and candidates are deterministic projections
  of that sealed snapshot.
- This directory is not a fourth authority store. It must not be synced,
  materialized as Projection content, watched as workspace input, exposed to the
  browser, or interpreted as Source Control/External Changes state.
- Redb owns session identity/state and exact digests; the filesystem owns only
  the sealed payload bytes. Neither side may infer the missing half from names
  or timestamps.

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

- 同一 branch 下 `.redb` 文件名必须是 canonical `RepoId`；同名 display alias
  不产生物理冲突。
- 非 UUID filename、同一 `RepoId` 的重复文件或 identity mismatch 必须
  fail-closed 并进入显式 catalog/repair；不得自动重命名后猜测归属。

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

浏览器能力探测必须保持三层边界：JS bridge 只调用平台 API 并返回能力事实；
`browser_identity_client` 把能力事实归一化为强类型 write blocker；UI 只把 blocker
投影为本地化只读原因与恢复指引。`localStorage` 不属于 writer identity 前置条件。
WebCrypto、IndexedDB 或 WebCrypto Ed25519 任一必需能力缺失时必须 fail-closed；不得由
JS/WASM polyfill 生成可导出私钥、伪造平台能力或绕过 browser peer 签名。
Android native WebView 的正式可写 evidence baseline 是 API 29+ 与当前 provider major 137+；
版本事实只属于 support/receipt 诊断，最终 writer identity 仍必须由上述真实 non-extractable
Ed25519 probe 证明。技术上通过 probe 但低于支持基线的设备不得生成正式 target-host receipt，
而缺失 probe 的任何设备都不得因版本号、native session 或 host capability 获得写权限。

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
  -> WatcherTransitioning(generation)
  -> WatcherReady
  -> Mounted(generation)
```

约束：

- `ProjectionLocated` 必须验证 repo-scoped Projection Locator。
- `WatcherReady` 是打开 repo 的最后一步；只有 owned watcher handle 已完成 capture-first clean scan cut，才允许发布 `Mounted(generation)`。
- durable `RepoHealth` 与 process-local `RepoMountState` 正交。依赖 Projection Workspace 当前性的在线本地写路径只有在下式成立时才可准入：

  ```text
  RepoHealth::Healthy && RepoMountState::Mounted
  ```

- watcher 初始化或运行期失败必须对该 repo fail-closed 为 `RepoMountState::Failed`，不得写入 projection fault journal，也不得伪装为 `DegradedProjection`。
- server bootstrap 对 repo-local watcher start failure 按 repo 隔离；健康且成功 mounted 的 repo 继续运行。启动完成时若零个 repo 处于 `Mounted`，host 必须回滚已启动 watcher 并终止 server。
- server 已运行后即使全部 watcher 后续失败，仍保留纯读、ledger inspect/export 与离线 repair/diagnostic 能力；所有依赖 workspace 当前性的在线本地 mutation 保持关闭。
- `Transitioning / Mounted / Failed` 的 generation、失败原子切点与 owned lifecycle 唯一由 [watcher contract](./watcher.md#watcher-contract) 定义；repo create/rename/remove 的 durable 与 mount 协调唯一由 `04_repository#repo-health-and-repair` 定义。

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
`projection_locator_runtime` 作为 `projection_persistence_runtime` 的独立子 runtime，唯一拥有
host-local locator 文件；该父子关系不得被解释为新的顶层 storage authority 层。

浏览器侧另设 `browser_identity_client` 作为本章 storage layering 的 client adapter；
它只归一化平台 capability、管理 non-extractable browser identity readiness，并向 UI
投影强类型 blocker，不是第五个 storage authority runtime。

## 本章相关命令

- 无

## 本章相关配置

- `snapshot_depth`
- backup / retention 相关配置
- `projection.locators`
- `ledger.path`
