# 03_storage/projection.md - Projection & Persistence Runtime

## Metadata

- `Layer`: `Authority Core`
- `Status`: `Current MUST`
- `Version`: `0.0.1`
- `Last Review`: `2026-05-30`
- `Parent`: `03_storage/index`
- `Primary Code Areas`: `crates/core/src/sync/projection_persistence_runtime.rs`, `crates/core/src/ledger/manager/projection_locator.rs`, `crates/core/src/sync/projection_io.rs`, `crates/core/src/writeback/`

> 本文件是 `03_storage` 章的 `projection_persistence_runtime` 子合同：projection locator layout 与 projection / persistence contract。章节骨架与总览见 [index.md](./index.md)。

## 3. Physical Layout（projection 部分）

> §3.2 Repo Runtime Layout 见 [index.md#repo-runtime-layout](./index.md#repo-runtime-layout)。

### 3.2.1 Projection Locator Layout {#projection-locator-contract}

Projection Locator 是 host-local runtime state，负责把本地 repo instance 绑定到宿主文件系统中的 projection base。最终 repo workspace root 必须由 locator base、当前 repo name 派生的安全显示段与完整 `RepoId` 计算得到：

```text
WorkspaceSegment(repo_id) = safe_repo_name(current RepoNameBinding) + "--" + full_repo_id
ProjectionWorkspaceRoot(repo_id) = projection_base_abs / WorkspaceSegment(repo_id)
```

示例：

```text
<projection_base>/<safe_repo_name>--<repo_id>/
  .notegit/
  .deveignore
  a.md
  notes/a.md
```

`projection_base` 可以包含其它文件或目录；系统只能 scan/watch/import 计算出的 `<projection_base>/<safe_repo_name>--<repo_id>/`，不得把 base 根目录本身当作 repo workspace。

路径归属判定示例：

- 若 `projection_base = E:/`、`repo_name = my-notebooks` 且 `repo_id = 550e8400-e29b-41d4-a716-446655440000`，则 workspace root 是 `E:/my-notebooks--550e8400-e29b-41d4-a716-446655440000/`；该目录内 `.notegit/`、`notes/a.md` 与 `a.md` 都属于该 repo。
- 若 `projection_base = E:/my-notebooks`、`repo_name = math` 且 `repo_id = 550e8400-e29b-41d4-a716-446655440000`，则 workspace root 是 `E:/my-notebooks/math--550e8400-e29b-41d4-a716-446655440000/`；`E:/my-notebooks/a.md` 可以存在，但不属于该 repo，系统不得 scan/watch/import 它。

最小模型：

```text
ProjectionLocatorKey = RepoId
ProjectionLocatorValue = {
  repo_id,
  repo_name_hint,
  projection_base_abs,
  canonicalized_at,
}
```

约束：

- `projection_base_abs` **MUST** 是 canonicalize 后的绝对路径；若 base 不存在，`init` / locator repair 可以先创建 base，再 canonicalize。
- `repo_name` 参与 workspace root 路径段时 **MUST** 先规范化为单一 `safe_repo_name`：不得包含路径分隔符、drive prefix、NUL、Windows 非法字符（`< > : " | ? *`），不得等于 `.` / `..`，不得使用 Windows reserved device name（大小写不敏感），不得以空格或点结尾，并且必须经过大小写/Unicode normalization 后做同目录冲突检查。
- workspace segment **MUST** 使用完整 `RepoId` 后缀：`<safe_repo_name>--<repo_id>`。实现不得使用短 id 作为唯一判定依据。
- 本地可写 repo 进入 `ProjectionReady` 前 **MUST** 存在 locator。
- locator **MUST NOT** 写入 `LEDGER_OPS`、Structure Facts、Content Facts 或 sync payload。
- locator **MUST NOT** 作为 repo identity；`repo_name_hint` 只能用于诊断，不得替代 `RepoId` 或当前 repo metadata。
- workspace root 是派生值。实现可以缓存 `workspace_root_abs`，但缓存 **MUST** 可由 `projection_base_abs + safe_repo_name(current RepoNameBinding) + RepoId` 重建，且不得成为 authority。
- workspace admission **MUST** 校验 `.notegit` identity marker 中的 `repo_id` 等于当前 `RepoId`；路径名匹配但 marker 缺失或不一致时必须 fail-closed。
- repo rename / display name repair 时，locator base 保持不变；系统 **MUST** 将 workspace root 从 `<base>/<old_safe_repo_name>--<repo_id>/` realign / move 到 `<base>/<new_safe_repo_name>--<repo_id>/`，若目标已存在、identity marker 不一致或不可安全移动则 fail-closed 并进入 `DegradedLocator`。
- repo rename realign 前若存在 `pending_fs_ops`、staging、未解释 dirty workspace、projection writeback fault 或 active watcher write，系统 **MUST** 先要求用户 commit / discard / repair；不得隐式移动带脏状态的 workspace。
- 两个本地 repo **MUST NOT** 解析到同一 workspace root。
- 任意两个 workspace root **MUST NOT** 互为父子目录。
- workspace root **MUST NOT** 位于 `ledger/`、`ledger/.host/`、`.notegit/` 或 `.git/` 内部。
- locator 缺失、路径不可读、路径不可 canonicalize 或路径冲突时，repo **MUST** 进入 `DegradedLocator`，不得进入 mounted write path。

## 7. Projection and Persistence Contract {#projection-contract}

- 所有系统写盘都必须满足：

```text
Intent -> Ledger Facts -> Projection -> Projection Workspace
```

- `metadata`、`path mapping`、`tree cache`、`NodeMeta` 只能由 projection builder 写入。
- handler、component、source control action 不得把这些表当成主写路径。
- ledger append 成功而 projection 失败时，系统 **MUST** 标记 recoverable fault，并支持从 ledger 重建；对 writeback / realign / rebuild interrupted 这类重启后仍需精确重放的故障，durable fault journal 语义见 `22_reliability_observability.md#observation-to-health-mapping`。

具体要求：

- projection builder 至少输出：
  - `NodeMeta`
  - path mapping
  - doc projection
  - tree projection
  - snapshot checkpoints
- projection writeback **MUST** 具备 repo-scoped 原子感知：
  - 同一 repo 的 projection rebuild 与 workspace persist 不能并发乱序覆盖
  - 不同 repo 之间必须隔离
- `PersistGuard` / `WriteSuppressor` 必须在 projection writeback 前后成对生效，防止 watcher storm
- projection writeback **MUST** 通过 Projection Locator 解析目标 base，并计算 `<projection_base>/<safe_repo_name>--<repo_id>/` 作为目标 workspace root；禁止从全局 vault root 隐式推断。

## 10. Forbidden Patterns（projection）

> 跨层禁止项见 [index.md](./index.md)。

- 让 Projection Workspace 作为真值源。

## 11. Runtime Boundary（projection 部分）

### 11.2 Projection / Workspace Layer

- 负责由 ledger fold 派生 projection、workspace writeback、projection cleanup 与 drift 解释。
- projection 失败不得伪装成 authority 成功。
