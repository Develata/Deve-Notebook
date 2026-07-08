# 06_backup.md - Projection Backup

## Metadata

- `Layer`: `Application / Projection Transport`
- `Status`: `Planned Contract`
- `Version`: `0.0.2`
- `Last Review`: `2026-07-08`
- `Counterpart Feature`: `docs/features/06_repository.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/07_storage_repo.md`
- `Primary Code Areas`: `crates/core/src/remote_projection/`, `apps/cli/src/commands/projection_remote.rs`, `apps/web/src/components/command_palette/registry/remote_projection.rs`

本章定义 **Projection Backup**：把 Markdown Projection Workspace 文件集合传输到
WebDAV / S3 / S3-compatible remote，并可从 remote 拉回 Projection Workspace。

Projection Backup 的备份对象是：

```text
Projection Workspace 中的 Markdown 文件集合
```

它不是 Ledger history disaster recovery，不承诺恢复 ledger global sequence、writer
causality、commit anchors、Source Control history 或 Git mirror queue。需要 history 的场景
属于 NoteGit/ngit + Git remote workflow；Projection Backup 只负责搬运 Markdown projection
files。

## 1. Scope {#projection-backup-scope}

本章只处理四类问题：

1. 当前 local repo 的 Projection Workspace Markdown files 如何上传到 WebDAV/S3 remote。
2. WebDAV/S3 remote 上的 Markdown files 如何下载并覆盖 Projection Workspace。
3. 下载后的文件变化如何进入 External Changes，并由用户确认后写入 Ledger。
4. Remote Projection transport、credential binding 与 provider metadata 的 authority 边界。

非目标：

- 不定义任何 ledger-history disaster recovery path；Ledger history 由 NoteGit / Git remote 负责。
- 不定义独立的 backup pack、manifest、restore-candidate 或 ledger import/merge runtime。
- 不定义实时多端同步协议。
- 不定义 Git remote、GitHub repo、`.git` mirror 或 Git push；Git history 由
  Source Control / NoteGit / Git remote 语义负责。
- 不允许 WebDAV/S3 直接成为 Ledger、Source Control、Git mirror 或 workspace identity
  authority。
- 不传输 `.notegit/`、`.git/`、ledger、staging、snapshot、runtime state 或 secret material。

## 2. Product Semantics {#projection-backup-contract}

Projection Backup 是 Remote Projection Transport 的 backup-oriented 产品语义。

```text
Upload:
  Projection Workspace Markdown files
    -> Projection Locator / identity gate
    -> WebDAV / S3 / S3-compatible provider
    -> remote Markdown object set

Pull / Download:
  remote Markdown object set
    -> Projection Locator / identity gate
    -> Projection Workspace overwrite
    -> Watcher / scan
    -> External Changes
    -> user confirmation
    -> Ledger facts
```

核心规则：

- Upload 只上传 Markdown projection files，不上传 ledger facts、commit anchors、runtime
  metadata 或 provider-local credentials。
- Pull / Download 只写 Projection Workspace；它不得直接 append ledger、写 Source Control
  staging、创建 commit anchor、写 Git mirror queue 或自动确认 External Changes。
- External Changes 是 remote files 进入 Ledger 的唯一首版 admission surface。用户确认前，remote
  files 只是外部输入。
- Provider object metadata 只能作为 diagnostic。ETag、mtime、object version、object key、locator
  path 或 remote listing order 都不得成为 ledger/source-control authority。

## 3. Locator and Profile Model {#projection-backup-locator-contract}

Projection Backup 复用 Remote Projection locator / profile 模型。支持的 locator 形式至少包括：

```text
webdav+https://dav.example.com/notebooks/deve/main/
s3://bucket-name/deve/main/
s3+https://r2.example.com/bucket-name/deve/main/
```

locator 的有效组成：

- protocol / provider kind
- endpoint host
- bucket / namespace
- projection prefix

locator 禁止组成：

- password
- access key / secret key
- session token
- encryption key material
- auth cookie

规则：

- Web / Command Palette 不接收 locator 或 credential material；backend 必须从当前 local repo
  的 characteristic `repo_url` 或显式 Remote Projection profile 解析 locator。
- CLI 可以为 host operation 显式传入 `--locator` 或显式 Remote Projection profile。
- `s3+https://` / S3-compatible endpoint 必须绑定显式 Remote Projection profile；未绑定时
  fail-closed，避免默认 AWS 环境凭证被签给任意 host。
- locator/profile 只能作为 transport target 选择依据，不拥有 repo identity。执行前必须复用
  Projection Locator、`.notegit` identity marker 与 current repo scope gate。

## 4. Remote Layout {#projection-backup-remote-layout-contract}

Projection Backup remote layout 是 Markdown object set，不是 ledger pack layout。

推荐布局：

```text
<remote-prefix>/
  README.md
  notes/
    a.md
    b.md
  journals/
    2026-07-08.md
```

规则：

- Remote object path 使用 `/` 分隔，并以 Projection Workspace 相对路径为 canonical path。
- 只枚举和传输 `.md` Markdown projection files。
- 归一化后为空、绝对路径、包含 `..`、指向 reserved/internal path 或重复的 remote path 必须在下载
  payload 或写 workspace 前 fail-closed。
- Provider metadata、remote listing order 与 object version 只进入 diagnostics，不进入 Ledger 或
  Source Control authority。

## 5. Upload State Machine {#projection-backup-upload-state-machine-contract}

```text
UploadRequested
  -> ProjectionWorkspaceValidated
  -> MarkdownFilesEnumerated
  -> ProviderResolved
  -> FilesUploaded
  -> UploadReported
```

约束：

- `ProjectionWorkspaceValidated` 必须确认当前 local repo、Projection Locator 与 `.notegit`
  identity marker 一致。
- `MarkdownFilesEnumerated` 只能从 Projection Workspace 读取 Markdown files；必须排除
  `.notegit/`、`.git/`、ledger、staging、snapshot、runtime state 与 ignored/internal paths。
- `FilesUploaded` 只表示 provider adapter 完成 file object PUT；它不生成 ledger facts，不修改
  Source Control state，不创建 commit anchor。
- 上传失败不得回滚 local ledger 或 Source Control state；已上传的 remote objects 只能作为 provider
  diagnostic / retry context。

## 6. Pull / Download State Machine {#projection-backup-pull-state-machine-contract}

```text
PullRequested
  -> ProviderResolved
  -> RemoteMarkdownListed
  -> RemotePathsValidated
  -> RemoteMarkdownDownloaded
  -> ProjectionWorkspaceOverwritten
  -> WatcherOrScanDetected
  -> ExternalChanges
```

约束：

- `RemoteMarkdownListed` / `RemoteMarkdownDownloaded` 必须受 hard budget 约束：文件数、单文件
  bytes、总下载 bytes 超限时必须在写 Projection Workspace 前 fail-closed。
- `RemotePathsValidated` 必须在下载 payload 或写 workspace 前拒绝归一化后重复、越界、reserved、
  non-Markdown 或 unsafe target path。
- `ProjectionWorkspaceOverwritten` 必须只覆盖 Projection Workspace 中的 Markdown projection files；
  workspace apply 应使用 staging + rollback 或等价机制，避免半写入可见。
- `WatcherOrScanDetected` 后才能进入 External Changes。该步骤不得直接 stage、commit、Apply to
  Ledger 或创建 Git mirror queue。
- External Changes 由用户明确确认后，才通过 existing Ledger authority path 追加 facts。

## 7. Commands / Inputs / Outputs {#projection-backup-command-output-contract}

### 7.1 Inputs

- `ProjectionBackupLocator` / `RemoteProjectionLocator`
- `RemoteProjectionProfile`
- `RepoSelector`
- `ProviderKind`
- `Direction` = `push | pull`
- provider credential refs owned by Remote Projection profile/runtime

credential refs 是 host-local runtime config 引用，不是 locator 的一部分。

### 7.2 Commands

Projection Backup 不引入独立 provider runtime；首版命令应收敛到 Remote Projection transport。

- `deve projection-remote webdav push/pull`
- `deve projection-remote s3 push/pull`

旧 `deve backup ...` CLI surface 不属于首版命令面；Projection Backup 的唯一 CLI surface 是
`deve projection-remote ... push/pull`。

### 7.3 Outputs

- `ProjectionBackupPlan`
- `ProjectionBackupReport`
- `RemoteProjectionProviderReport`
- `ExternalChangesRequired`
- `ProjectionBackupError`

输出必须区分：

- provider IO 是否成功；
- Projection Workspace 是否被写入；
- External Changes 是否已被检测；
- Ledger 是否已被用户确认写入。

Provider success 不等于 pull/admission success；Projection Workspace overwrite 不等于 Ledger recovery。

## 8. Security and Authority Contract {#projection-backup-secret-ref-contract} {#projection-backup-verification-contract}

- Credentials, tokens and key material **MUST NOT** be stored in repo catalog, locator string,
  localStorage, URL query, normal logs or crash reports。
- Remote Projection profile 可以保存 secret-free endpoint/bucket/prefix/credential-ref binding；credential
  value 只能由 runtime resolver 在 provider IO 时解析。
- Provider metadata **MUST** remain diagnostic-only。
- Remote files are external input. They become Ledger facts only through External Changes user
  confirmation and existing authority storage runtime。
- Projection Backup does not encrypt or authenticate ledger history because it does not transport
  ledger history. If durable history is required, use NoteGit/ngit + Git remote。
- S3-compatible custom endpoint signing must be profile-bound and fail-closed on missing endpoint,
  missing region/signing scope, missing credential ref, or provider/profile mismatch。

## 9. Failure Modes {#projection-backup-failure-modes}

- locator/profile missing or provider mismatch
- credential rejected or credential resolver unavailable
- custom S3 endpoint missing explicit profile binding
- Projection Locator or `.notegit` identity marker broken
- workspace cannot be canonicalized
- remote listing unavailable or malformed
- remote path normalization failure
- duplicate normalized remote Markdown path
- file count / single file bytes / total bytes budget exceeded
- provider PUT/GET failure
- workspace apply rollback required or failed
- watcher/scan failed to surface External Changes

所有 failure 必须结构化。失败的 upload / pull 不得留下 partial ledger writes、Source Control staging、
commit anchors、Git mirror queue entries 或 silently confirmed External Changes。

## 10. Forbidden Patterns {#projection-backup-forbidden-patterns}

- 把 WebDAV/S3 当作 shared writable sync authority。
- 把 Projection Backup 描述为 Ledger history disaster recovery。
- 上传或下载 ledger-history artifacts、snapshot/runtime state 作为首版 Backup 合同的一部分。
- 把 ledger import/merge runtime 当作 Projection Backup pull/admission path。
- 把 provider metadata、locator path、ETag、mtime 或 object version 当作 ledger/source-control
  authority。
- 远端 pull 后自动 Apply to Ledger、自动 stage、自动 commit 或自动 Git push。
- 在 Web / Command Palette 收集 WebDAV/S3 credentials 或直接访问 provider。
- 复用独立 backup credential/key model；WebDAV/S3 credential binding 应归 Remote Projection profile
  runtime 所有。

## 11. Runtime Boundary {#projection-backup-provider-dispatch-contract}

### 11.1 Remote Projection Transport Runtime

职责：

- locator/profile resolution
- provider adapter dispatch
- Markdown file enumeration
- WebDAV/S3/S3-compatible upload/download
- provider diagnostics

### 11.2 Projection Workspace Runtime

职责：

- Projection Locator validation
- `.notegit` identity marker validation
- safe workspace overwrite
- staging / rollback for pull apply
- watcher/scan trigger

### 11.3 External Changes / Ledger Runtime

职责：

- external file diff detection
- user confirmation / Apply to Ledger
- ledger facts append through existing authority storage path
- Source Control dirty derivation after ledger confirmation

Remote Projection Transport 不得直接写 Ledger、Source Control staging、commit anchors、Git mirror queue
或确认 External Changes。

## 12. Deferred / Removed From First Tag {#projection-backup-deferred-ledger-backup}

已从首版范围删除：独立 ledger backup pack/manifest、restore-candidate admission、ledger import/merge
runtime，以及 WebDAV/S3 上的 ledger-history disaster recovery。

若未来重新引入 ledger backup，必须作为独立 ADR 与独立 runtime proposal 重开；不得从 Projection
Backup 语义中回填。

## 本章相关命令

- `Remote Projection: WebDAV Push/Pull`
- `Remote Projection: S3 Push/Pull`

## 本章相关配置

- `remote_projection.profile`
- `remote_projection.locator`
- `remote_projection.credential_ref`
