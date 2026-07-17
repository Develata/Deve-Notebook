# 06_backup.md - Remote Projection 与 Remote Import

## Metadata

- `Layer`: `Application / Projection Transport`
- `Status`: `Current MUST`
- `Version`: `0.1.0`
- `Last Review`: `2026-07-17`
- `Counterpart Feature`: `docs/features/06_repository.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/07_storage_repo.md`
- `Primary Code Areas`: `crates/core/src/remote_projection/`, `apps/cli/src/commands/projection_remote/`, `apps/cli/src/server/handlers/source_control/remote_projection.rs`

本章冻结两个相互分离的首发能力：

1. **Remote Projection push**：把当前 Markdown Projection Workspace 上传到 WebDAV / S3 / S3-compatible provider。
2. **Remote Import**：从 provider 捕获不可变 source snapshot，经独立 review 后，以 whole-session Ledger transaction 导入。

两者可以共享 host transport adapter，但 Remote Import 不通过 workspace overwrite、External Changes 或 Source Control staging admission。

## 1. Scope {#projection-backup-scope}

本章负责：

- Remote Projection locator/profile、provider streaming 与 push 边界；
- immutable manifest/blob capture、durable session、candidate/revision、retention 与 cleanup；
- Remote Import review/apply/discard/repair 的状态和 authority 边界；
- 资源预算、host-only layout、失败与敏感信息边界。

非目标：

- Ledger history disaster recovery、backup pack、同步协议或 Git remote；
- remote Delete、逐文件选择、逐文件 Apply 或隐式 rollback；
- WebDAV/S3 成为 Ledger、workspace identity、Source Control 或 credential authority；
- 传输 `.notegit/`、`.git/`、Ledger DB、staging、snapshot、runtime state 或 secret material。

## 2. Product Semantics {#projection-backup-contract}

```text
Push:
  Projection Workspace -> host transport adapter -> remote Markdown object set

Import:
  remote provider
    -> remote_projection_transport_runtime
    -> immutable manifest/blob capture
    -> remote_import_runtime
    -> review / typed blockers
    -> sealed source-specific writer
    -> Ledger commit
    -> Projection writeback
    -> Workspace
```

- Push 只搬运 projection files，不产生 Ledger facts、commit anchor 或 Source Control state。
- Import 在 Ledger transaction 成功前不得写 Projection Workspace、pending/staged state 或 External Changes。
- provider listing order、ETag、mtime、object version 与 locator path 只能进入 diagnostics，不能成为 facts 或 apply authority。
- Projection writeback 只发生在 Ledger commit 后。authority transaction 先保存 projection outcome=`Pending` 的 immutable commit receipt；后续 outcome 只可单调收敛为 `Written` 或 `Degraded`。writeback 失败产生“Ledger 已提交、Projection degraded”的 durable receipt，不回滚 Ledger。

## 3. Remote Projection Transport Contract {#remote-projection-transport-contract}

Transport runtime 只拥有 locator/profile admission、provider adapter、确定性 listing、ordered streaming、push 和 provider diagnostics。

- push 与 source acquisition 使用语义分离的 typed interface；共享 HTTP/signing 实现不等于共享业务 authority。
- source acquisition 只能向 project-owned bounded sink 交付 normalized path 与 payload stream；不得构造 session、写 Ledger、写 workspace 或决定 blocker/apply。
- provider path 必须先 normalize、排序并拒绝重复，再逐文件 streaming；不得依赖 provider listing order。
- Web/Command Palette 不携带 locator、endpoint URL 或 credential material。backend 从 repo-bound locator/profile 解析；CLI host operation 可以显式选择 locator/profile，但仍须 exact admission。
- transport adapter 保持轻量、可审计。引入重型 provider SDK或新的 credential authority属于架构停止条件。

### 3.1 Locator and Profile Model {#projection-backup-locator-contract}

支持的 locator 至少包括：

```text
webdav+https://dav.example.com/notebooks/deve/main/
s3://bucket-name/deve/main/
s3+https://r2.example.com/bucket-name/deve/main/
```

locator 只含 provider kind、endpoint host、bucket/namespace 与 projection prefix；禁止 password、access key、secret、session token、cookie 或 encryption key。

ADR 0008 的 host-local、secret-free profile binding 继续生效：custom endpoint 必须 exact-match provider、normalized HTTPS origin、bucket、allowed root prefix、region/signing scope、addressing style 与 credential reference。任一不匹配都在 provider I/O 前 fail-closed；默认 AWS credential 不得被 ambient fallback 签给任意 custom host。

allowed capability 固定为 `push` 或 `source-acquisition`；不再存在批准的 `pull-to-workspace` direction。

### 3.2 Remote Object Layout {#projection-backup-remote-layout-contract}

remote 是 Markdown object set，不是 Ledger pack：

```text
<remote-prefix>/README.md
<remote-prefix>/notes/a.md
```

canonical object path 使用 `/` 和 Projection Workspace 相对路径。绝对路径、空路径、`..`、reserved/internal path、non-Markdown path 与归一化重复必须 fail-closed。

### 3.3 Push State Machine {#projection-backup-upload-state-machine-contract}

```text
PushRequested
  -> Repo/Locator/IdentityValidated
  -> MarkdownFilesEnumerated
  -> ProviderAdmitted
  -> FilesUploaded
  -> PushReported
```

Push 必须读取 `Healthy + Mounted` 的 current Projection Workspace，排除 internal/ignored path。partial remote upload 只形成 provider diagnostic/retry context，不触碰本地 Ledger 或 Source Control。

## 4. Immutable Remote Import Session {#remote-import-session-contract}

项目自有 durable 类型：

- `RemoteImportSourceSnapshot`
- `RemoteImportCandidateRevision`
- `RemoteImportSessionRecord`
- `RemoteImportApplyReceipt`
- `RemoteImportState::{Preparing, Ready, Stale, Failed, Applied, Discarded}`
- `RemoteImportChangeKind::{Added, Modified, Unchanged}`，与 typed blocker 正交

每个 repo 最多一个 active session。Redb 只保存身份、状态、exact digest、receipt 与 active/cleanup metadata；manifest、candidate 与 blobs 存在 host-only artifact tree：

```text
ledger/.host/remote-imports/<repo_id>/<session_id>/
  source-manifest.json
  candidates/<revision>.json
  blobs/<sha256>
```

`source-manifest.json` 是 deterministic JSON v1：字段顺序、path normalization、entry ordering 与 digest serialization 固定。wire/UI 不读取该 host layout。

### 4.1 State Machine {#remote-import-state-machine}

```text
Prepare:  none -> Preparing -> Ready | Failed
Refresh:  Ready | Stale -> Ready | Stale | Failed
Apply:    Ready -> Applied
Discard:  Ready | Stale | Failed -> Discarded
```

- 不存在 durable `Applying`。Apply 由 process single-flight + Redb CAS 与 idempotent stored receipt 保证。
- Prepare 固定顺序：Redb reserve `Preparing` → stream/verify temp blobs → 原子发布 blobs/manifest/candidate → CAS `Ready`。
- 启动时遗留 `Preparing` 必须转为 `Failed(Interrupted)`；不得自动重新访问 provider。
- Refresh 只能从已封存 blobs 重算 candidate。若 `RepoId`、branch、source snapshot、locator/profile binding 与 digests 仍 exact，它可以把新 candidate revision 绑定到当前 Ledger head 和当前 ignore snapshot，使 head/ignore drift 的 session 从 `Stale` 回到 `Ready`。
- locator/profile、branch、repo membership 或 source/manifest/blob digest drift 不可由 Refresh 重绑；session 必须保持 `Stale` 或进入 typed `Failed`。获取新远端内容必须 Discard 后重新 Prepare，不得猜测 source identity。
- 相同 Apply 请求在响应丢失后返回并收敛已存 `RemoteImportApplyReceipt`，不得重复 append facts。若 receipt projection outcome 仍为 `Pending`，runtime 必须从 Ledger 幂等恢复 writeback；不得把它解释为未提交。
- terminal record 只保留最近 64 条；`cleanup_pending=true` 的记录永不自动裁剪。cleanup 必须由显式 discard/repair 收敛。

### 4.2 Resource Contract {#remote-import-resource-contract}

硬 admission budget：

| 维度 | 上限 |
|---|---:|
| 文件数 | 2048 |
| 单文件 payload | 4 MiB |
| 全部 payload | 64 MiB |
| 单路径 UTF-8 bytes | 1024 |
| 全部路径 UTF-8 bytes | 2 MiB |
| review page | 默认 100，最大 200 |

任一预算超限都在 session 可 Apply 前 fail-closed。capture 必须逐文件 streaming，不得把完整 64 MiB snapshot 聚合进内存。remote 缺失文件不产生 Delete；首版无逐文件选择。

## 5. Review and Apply Authority

- Prepare/List/Show/Page/Diff/Refresh/Discard 可以在 Ledger 可读但 repo 未 Mounted 时执行；它们不得写 Ledger facts 或 workspace。
- review 必须绑定 exact `(repo_id, branch, scope_nonce, session_id, revision)`。Diff 只接受 opaque strong `entry_id`，返回 backend display label 与 typed diff/blocker。
- 任一 blocker 禁用整个 session Apply。pending/staged overlap、head/branch/locator/ignore drift、tamper 与 membership mismatch 均由 backend 判定。
- Apply 必须满足 `RepoHealth::Healthy && RepoMountState::Mounted`，并进入 `03_storage/authority.md#sealed-ledger-change-batch` 的 whole-session transaction。
- transaction 精确复核 repo/schema/head、active session/revision、manifest/blob digests、writer identity、branch、locator/ignore snapshot、pending/staged overlap 与 RepoId membership；随后原子写全部 upsert facts/indexes、Applied receipt immutable core + projection outcome=`Pending`、clear active pointer 与 `cleanup_pending=true`。post-commit writeback 后用第二个短 Redb transaction 把 outcome CAS 为 `Written`，或与 durable projection fault evidence 一起原子 CAS 为 `Degraded`。
- Remote Import 只产生 upsert facts。远端未出现的本地文件不删除；不得拆分 whole-session transaction。

## 6. Commands / Inputs / Outputs {#projection-backup-command-output-contract}

正式 CLI surface 由 `14_commands.md#remote-import-command-contract` 定义：Remote Projection 只保留 provider push；Remote Import 提供 prepare/list/show/diff/refresh/apply/discard/repair。`repair` 默认 dry-run，真实 cleanup 需要显式 `--apply`。

输出必须区分 provider failure、capture/session failure、blocker/stale、authority apply failure、cleanup required 与 post-commit projection degraded。产品 detail 使用泛化文案；原始失败仅进入受控 tracing。

## 7. Security and Verification {#projection-backup-secret-ref-contract} {#projection-backup-verification-contract}

- credential、token、key material 不得进入 repo catalog、locator string、manifest/blob、candidate、wire/UI、localStorage、URL query、普通日志或 crash report。
- capture 发布前与 Apply transaction 内都重验 manifest/blob digest；provider metadata 永不替代 content digest。
- Applied receipt 绑定 session/revision/writer/head/exact digests，但 wire/UI 只投影安全的 typed outcome。
- CLI 直接打开 DB 执行 Apply 时必须启动临时 `RepoWatcherHandle` 并按 W6 E2 shutdown；DB 被 server 持有时只能使用 authenticated loopback `LocalCliProxyAuthority`，不得复用 browser grant 或绕锁直写。
- session artifact 使用 repo/session identity 与 path containment；symlink/reparse/path traversal 必须 fail-closed。

## 8. Failure Modes {#projection-backup-failure-modes}

- locator/profile/provider/credential unavailable or mismatch
- unsafe、duplicate 或 budget-exceeded remote path/payload
- temp capture、atomic publication、digest verification 或 CAS failure
- active session、not found、stale、blocked、invalid state、cleanup required
- exact writer/head/branch/membership/snapshot revalidation failure
- Ledger transaction failure
- post-commit Projection writeback failure
- committed receipt 停留在 projection outcome `Pending`，等待启动/重试幂等恢复

pre-commit failure 不得留下事实前缀、workspace prewrite、External Changes、Source Control staging、commit anchor 或 Git mirror queue。cleanup failure 保留 `cleanup_pending`；不得自动裁剪或伪装成功。

## 9. Forbidden Patterns {#projection-backup-forbidden-patterns}

- provider → workspace overwrite → watcher/External Changes admission；
- Remote Import session 直接操作 Ledger authority tables，或暴露 generic callback/batch constructor；
- remote Delete、逐文件 Apply、checkbox/select-all、隐式 rollback 或自动 cleanup authority；
- 前端解析 raw detail、digest/path 或自行推导 blocker/stale/readiness；
- 把 session `Failed/Stale/cleanup_pending` 映射为 `RepoHealth` 或 projection fault；
- 在 Ledger commit 前写 Projection Workspace；在 writeback failure 后回滚 Ledger；
- 将 locator/provider metadata 当作 repo/fact authority。

## 10. Runtime Boundary {#projection-backup-provider-dispatch-contract} {#remote-import-runtime-boundary}

### 10.1 `remote_projection_transport_runtime`

唯一拥有 provider/profile/HTTP/signing、push、ordered source streaming 与 diagnostics；不拥有 session、Ledger、workspace 或 apply。

### 10.2 `remote_import_runtime`

唯一拥有 session store、host artifact、candidate/revision、retention、blocker aggregation、refresh/discard/repair 与 source-specific prepared batch construction；只能通过 sealed authority API Apply。

### 10.3 Authority / Projection Runtime

authority storage runtime 唯一提交 whole-session facts/receipt；Projection runtime 只消费已提交 Ledger outcome完成 writeback。Source Control、External Changes 与 watcher 均不承载 Remote Import controller 或 authority。

### 10.4 `remote_import_client`

只发送 typed intent、绑定 exact scope/session/revision、丢弃 stale response 并渲染 backend label/diff/blocker。可复用无状态 diff/render primitive，不复用 Source Control/External Changes controller、state 或 notice。

## 11. Current Pull Transition {#projection-backup-pull-state-machine-contract}

当前未发布代码仍实现 pull → Projection Workspace overwrite → watcher/scan → External Changes，以及 workspace rollback continuation。该 anchor 仅在 B4 前保持现有 Rust `plan_ref` 可追踪；旧路径是 release-blocking drift，不是批准能力、兼容层或 fallback。B4 必须删除 pull direction、workspace apply/rollback、scan bridge、旧 handler/message/CommandId 与双轨测试。

## 12. Deferred / Removed From First Tag {#projection-backup-deferred-ledger-backup}

独立 Ledger backup pack、history disaster recovery、remote Delete、逐文件 Apply、自动 cleanup、实时 sync、Git remote 与 provider-specific重型 SDK均不在首发范围。未来若引入，必须经独立 authority/runtime 决策；不得扩张本章现有边界。

## 本章相关命令

- `remote_projection.webdav.push`
- `remote_projection.s3.push`
- `remote_import.webdav.prepare`
- `remote_import.s3.prepare`
- `remote_import.open`
- `remote_import.refresh`
- `remote_import.apply`
- `remote_import.discard`

## 本章相关配置

- `remote_projection.profile`
- `remote_projection.locator`
- `remote_projection.credential_ref`

Remote Import session/state 不是配置域。
