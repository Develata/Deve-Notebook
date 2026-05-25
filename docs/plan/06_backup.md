# 06_backup.md - Backup and Restore

## Metadata

- `Layer`: `Authority Core / Backup Transport`
- `Status`: `Planned Contract`
- `Counterpart Feature`: `docs/features/06_repository.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/07_storage_repo.md`
- `Primary Code Areas`: `crates/core/src/backup/`, `apps/cli/src/commands/backup.rs`, `apps/web/src/components/settings/`

本章定义 repo / branch 对应 URL 的备份展开方式。它把 `06_repository.md`
中的 characteristic parameter 从普通 URL 扩展为 WebDAV、S3 与
S3-compatible backup locator，但不改变 `RepoId`、Branch、Ledger 与
Source Control authority。

## 1. Scope

本章只处理三类问题：

1. 一个 logical repo 如何绑定到 remote backup root。
2. 一个 branch / writer identity 如何绑定到 remote backup folder 或 prefix。
3. 备份包如何加密、上传、校验、下载与进入 restore/import 流程。

非目标：

- 不定义实时同步协议。
- 不定义 Git remote、GitHub repo、`.git` mirror 或 Git push。
- 不允许 WebDAV/S3 直接成为 Ledger、Source Control 或 workspace authority。
- 不允许多个 active writers 共写同一个 backup branch path。

## 2. Locator Model {#backup-locator-contract}

`06_repository.md` 中的 `URL / characteristic parameter` 在 backup 场景下
扩展为 protocol-aware locator。locator 是发现与恢复线索，不是 authority。

支持的 locator 形式至少包括：

```text
webdav+https://dav.example.com/notebooks/deve/
s3://bucket-name/deve/
s3+https://r2.example.com/bucket-name/deve/
```

locator 的有效组成：

- protocol
- endpoint
- bucket / namespace
- repo root path
- branch path

禁止组成：

- password
- access key / secret key
- session token
- encryption key material
- auth cookie

locator 解析规则：

- repo root locator 用于发现同一 logical repo 的 backup namespace。
- branch locator = repo root locator + branch writer identity path。
- locator 匹配只能作为归类 hint；最终必须以 backup manifest 中的 `RepoId`
  和本地 ledger/catalog 中的 `RepoId` 校验为准。
- locator 与 manifest `RepoId` 冲突时必须 fail-closed。

## 3. Authoritative Entities

### 3.1 Backup Root {#backup-root-contract}

Backup root 是 remote storage 上的 repo-level namespace。它可以对应一个
logical repo，但不拥有 repo authority。

最小状态：

- `repo_locator`
- `expected_repo_id`
- `format_version`
- `provider_kind`

### 3.2 Branch Backup Binding {#backup-branch-binding-contract}

Branch backup binding 是一个 branch / writer identity 到 remote folder 或
prefix 的 1:1 映射。

规则：

- 一个 Branch **MUST** 最多绑定一个可写 backup folder/prefix。
- 一个可写 backup folder/prefix **MUST NOT** 被两个 active writers 共享。
- 多端备份必须表现为同一 `RepoId` 下的多个 branch backup bindings。
- 非本 writer 的 branch backup 只能进入 `RemoteReadonly` 或 restore candidate。

### 3.3 Backup Pack {#backup-pack-contract}

Backup pack 是加密签名后的 branch ledger artifact。

pack 内容可以包含：

- ledger facts range
- snapshots
- pack manifest
- content-addressed blob references
- integrity hashes

pack 内容不得绕过 ledger append 或 source-control authority 直接写入 Projection Workspace。

### 3.4 Restore Candidate {#backup-restore-candidate-contract}

Restore candidate 是下载、验证、解密后尚未导入的备份结果。

Restore candidate 不是 local branch，也不是 current scope。只有显式 restore /
import / merge intent 通过 gate 后，才允许进入本地 repo runtime。

## 4. State Machines

### 4.1 Backup Upload {#backup-upload-state-machine-contract}

```text
Unbound
  -> BindingValidated
  -> PackPlanned
  -> PackEncrypted
  -> Uploaded
  -> RemoteVerified
  -> Complete
```

约束：

- `BindingValidated` 必须校验 `RepoId`、branch role 与 writer identity。
- `PackPlanned` 只能读取 ledger / snapshot authority，不得读取 stale UI state。
- `RemoteVerified` 必须确认 remote manifest 与 uploaded pack hash 一致。
- upload 失败不得回滚 local ledger 或 source-control state。

### 4.2 Restore / Import {#backup-restore-state-machine-contract}

```text
RemoteDiscovered
  -> ManifestVerified
  -> PacksDownloaded
  -> PacksDecrypted
  -> RestoreCandidate
  -> RemoteReadonly | ExplicitImport | ExplicitMerge
```

约束：

- verification、hash check、signature check 或 decrypt 失败必须 fail-closed。
- 默认下载不得 append local ledger。
- `ExplicitImport` 与 `ExplicitMerge` 必须复用 repo scope、writer gate 与
  source-control / merge authority。

## 5. Commands / Inputs / Outputs

### 5.1 Inputs

- `BackupLocator`
- `RepoId`
- `BranchSelector`
- `WriterIdentity`
- `BackupCredentialRef`
- `BackupKeyRef`

`BackupCredentialRef` 与 `BackupKeyRef` 是 secret/config 引用，不是 locator 的一部分。

### 5.2 Commands

- `BindBackupTarget`
- `InspectBackupTarget`
- `ListBackupBranches`
- `BackupBranch`
- `VerifyBackupTarget`
- `RestoreBackup`
- `UnbindBackupTarget`

### 5.3 Outputs {#backup-command-output-contract}

- `BackupBindingStatus`
- `BackupPlan`
- `BackupPackManifest`
- `BackupVerificationResult`
- `RestoreCandidate`
- `BackupError`

## 6. Remote Layout {#backup-remote-layout-contract}

推荐布局：

```text
<repo-root>/
  repo.manifest.enc
  branches/
    <writer-identity>/
      branch.manifest.enc
      packs/
        000001.pack.enc
        000002.pack.enc
```

规则：

- manifest 内路径分隔符 **MUST** 是 `/`。
- WebDAV ETag、S3 object version、mtime、object key 等 provider metadata 只能作为 transport diagnostic。
- remote layout drift 必须产生结构化诊断，不得静默 rebind。

## 7. Security Contract {#backup-secret-ref-contract} {#backup-verification-contract} {#backup-artifact-protection-contract}

- backup artifacts **MUST** be encrypted before upload。
- manifests and packs **MUST** be authenticated by signature, AEAD tag, or an
  equivalent integrity mechanism owned by `09_auth.md`。
- download 必须先 verify，再允许 decrypt/import effect 暴露给 runtime。
- credentials, tokens and key material **MUST NOT** be stored in repo catalog,
  locator string, localStorage, URL query, normal logs or crash reports。
- cloud ACL 可以限制 writer access，但 cryptographic verification 仍是必需条件。

## 8. Failure Modes

- locator unreachable
- credential rejected
- manifest missing or malformed
- `RepoId` mismatch
- duplicate writable branch binding
- pack hash mismatch
- signature / AEAD verification failure
- decrypt failure
- remote version conflict
- restore candidate incompatible with current repo health

所有 failure 必须结构化。失败的 backup 或 restore 不得留下 partial ledger writes、
staged entries、pending imports 或 workspace projection changes。

## 9. Forbidden Patterns

- 把 WebDAV/S3 当作 shared writable sync authority。
- 把两个 active writers 绑定到同一个 backup folder/prefix。
- 把 credentials 或 encryption secrets 放入 locator / repo URL。
- 未经 verification 与 decryption 就导入 downloaded packs。
- download 阶段自动 merge backup branches。
- 把 WebDAV ETag 或 S3 object version 当作 ledger、branch 或 repo authority。
- 用 backup manifests 替代 Source Control commit history。

## 10. Runtime Boundary

### 10.1 Backup Runtime {#backup-provider-dispatch-contract}

职责：

- locator parsing
- provider adapter dispatch
- pack planning
- encryption / verification orchestration
- remote upload/download

### 10.2 Repo Runtime

职责：

- repo_id validation
- branch binding validation
- restore candidate admission
- quarantine / degraded handling

### 10.3 Source Control / Merge Runtime

职责：

- explicit import
- explicit merge
- stage / commit after restore candidate admission

Backup runtime不得直接写 staging、commit anchors、Projection Workspace 或 current
repo scope。

## 11. Refactor Target

长期应形成独立 runtime：

- `backup_locator_runtime`
- `backup_pack_runtime`
- `backup_restore_runtime`

provider adapter 只能挂在 backup runtime 下，不得进入 repo、source-control 或
storage authority 层。

## 本章相关命令

- `Backup: Bind Target`
- `Backup: Inspect Target`
- `Backup: Run Backup`
- `Backup: Restore`

## 本章相关配置

- `backup.provider`
- `backup.locator`
- `backup.credential_ref`
- `backup.key_ref`
