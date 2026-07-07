# 06_backup.md - Backup and Restore

## Metadata

- `Layer`: `Authority Core / Backup Transport`
- `Status`: `Planned Contract`
- `Version`: `0.0.1`
- `Last Review`: `2026-07-07`
- `Counterpart Feature`: `docs/features/06_repository.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/07_storage_repo.md`
- `Primary Code Areas`: `crates/core/src/backup/`, `apps/cli/src/commands/backup.rs`, `apps/web/src/components/settings/`

本章定义 repo / branch 对应 URL 的备份展开方式。它把 `04_repository.md`
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

`04_repository.md` 中的 `URL / characteristic parameter` 在 backup 场景下
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
- Binding persistence 是 host-local backup runtime metadata，不是 ledger facts、
  Source Control state 或 Projection Workspace 内容；它只能保存 secret-free locator
  与 branch/writer/path/access 元数据，不能保存 credential ref、key ref 或任何 key
  material。

### 3.3 Backup Pack {#backup-pack-contract}

Backup pack 是加密签名后的 branch ledger artifact。

pack 内容可以包含：

- ledger facts range
- snapshots
- pack manifest
- content-addressed blob references
- integrity hashes

pack 内容不得绕过 ledger append 或 source-control authority 直接写入 Projection Workspace。

pack artifact 字节级约束：

- 上传到 provider 的 pack artifact MUST 是加密后的 artifact bytes，而不是明文
  ledger/snapshot payload。
- provider upload runtime MUST 在 PUT 前验证 artifact bytes 可被解析为 encrypted
  pack artifact，且 artifact routing metadata 与 `BackupPackManifest` 一致。
- `BackupPackManifest.payload_digest` MUST 是序列化后的 encrypted pack artifact
  bytes 的 sha256 digest；不得使用明文 payload digest 作为远端校验依据。
- encrypted pack artifact 的明文字段只允许包含 routing / verification 所需的
  `format_version / RepoId / writer_identity / branch_path / pack_sequence /
  nonce / ciphertext`；不得包含 credential、key ref 或 key material。
- restore 打开 pack 时 MUST 先校验 manifest、artifact routing metadata 与
  `payload_digest`，只有 digest 匹配后才允许执行 decrypt。

pack plaintext schema gate 约束 {#backup-pack-plaintext-schema-contract}：

- decrypt 后的 pack plaintext MUST 使用 backup runtime 拥有的版本化 schema；
  任意未带 backup plaintext magic / format version 的 bytes 不得进入
  RestoreCandidate 或 import/merge planning。
- 当前首版 plaintext schema 为 `BACKUP_PACK_PLAINTEXT_FORMAT_VERSION = 2`，
  使用 project-owned postcard codec payload；pre-1.0 旧 plaintext magic /
  codec payload 不进入 stable 兼容承诺。
- plaintext schema MUST 显式携带 `format_version / RepoId / writer_identity /
  branch_path / pack_sequence / ledger_seq_range / ledger_entries /
  snapshot_refs / blob_refs`。
- plaintext 中的 repo、writer、branch、pack sequence、ledger range、ledger entry
  count、snapshot count 与 blob refs MUST 与 `BackupPackManifest` 完全一致；
  任一不一致必须 fail-closed。
- `ledger_entries` MUST 逐条携带 `global_seq` 与 `serialize_ledger_entry` 产生的
  versioned ledger entry bytes；打开 plaintext 时必须逐条执行
  `deserialize_ledger_entry` 校验。未版本化、损坏或 sequence 不连续的 ledger entry
  不得进入 RestoreCandidate。
- plaintext schema validation 只是解密后的 typed evidence gate。它不得 append
  ledger，不得写 staging，不得写 Projection Workspace，不得创建 commit anchor 或
  Git mirror queue。
- branch manifest pack refs MUST carry the pack manifest metadata required to
  validate decrypted plaintext (`ledger_seq_range / ledger_event_count /
  snapshot_count / blob_refs`). Restore 只能从已验证的 branch manifest pack ref
  还原 `BackupPackManifest` 并打开 plaintext；不得信任 plaintext 自带 metadata
  来完成自证。

### 3.4 Restore Candidate {#backup-restore-candidate-contract}

Restore candidate 是下载、验证、解密后尚未导入的备份结果。

Restore candidate 不是 local branch，也不是 current scope。只有显式 restore /
import / merge intent 通过 gate 后，才允许进入本地 repo runtime。

RestoreCandidate admission MUST consume manifest verification 与
`PacksPlaintextVerified` typed evidence。只有 `PacksDecrypted` 而没有版本化
plaintext schema evidence 的 pack，不得进入 RestoreCandidate。

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
- `Uploaded` 只能在 encrypted pack artifact bytes 已通过 manifest / routing /
  digest 校验且 provider PUT 成功后进入。
- `RemoteVerified` 必须确认 remote object 读回的 encrypted pack artifact bytes
  与 uploaded pack hash 一致；provider metadata、ETag、version 或 mtime 不能作为
  该确认依据。
- upload 失败不得回滚 local ledger 或 source-control state。

Provider adapter 的 PUT / GET 只是 Backup Runtime 的传输原语。上传后进入
`RemoteVerified` 前，Backup Runtime 必须对同一 object 执行 readback，并用
readback bytes 重新计算 sha256，与 manifest `payload_digest` / uploaded digest
比对；不一致必须 fail-closed。GET 成功只表示 encrypted artifact bytes 已从 remote
object 读入内存，并附带 diagnostic-only provider metadata；它不能代替
manifest/hash/signature verification，不能触发 decrypt，也不能创建 restore candidate。

### 4.2 Restore / Import {#backup-restore-state-machine-contract}

```text
RemoteDiscovered
  -> ManifestVerified
  -> PacksDownloaded
  -> PacksDecrypted
  -> PacksPlaintextVerified
  -> RestoreCandidate
  -> RemoteReadonly | ExplicitImport | ExplicitMerge
```

约束：

- verification、hash check、signature check 或 decrypt 失败必须 fail-closed。
- `PacksDownloaded` 必须由 branch manifest 中的 pack object refs 驱动，逐个下载
  encrypted pack artifact bytes；下载结果在通过 manifest digest、artifact digest、
  authentication 与 decrypt gate 前，不得暴露为 plaintext 或 restore candidate。
- `PacksPlaintextVerified` 必须由 core 通过已验证 branch manifest pack refs
  还原 pack manifest 后打开 plaintext schema；任意 raw plaintext、metadata
  mismatch、损坏 ledger entry 或 plaintext 自证失败都不得进入 RestoreCandidate。
- restore candidate admission MUST 由 core-owned resource budget 约束 pack 数、
  encrypted aggregate bytes 与 plaintext aggregate bytes；超出预算必须
  fail-closed，且不得继续导入、合并或写本地 authority。
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
- `BackupEncryptedPackArtifactFile`

`BackupCredentialRef` 与 `BackupKeyRef` 是 secret/config 引用，不是 locator 的一部分。

### 5.2 Commands

- `BindBackupTarget`
- `InspectBackupTarget`
- `ListBackupBranches`
- `BackupBranch`
- `VerifyBackupTarget`
- `RestoreBackup`
- `UnbindBackupTarget`

`BackupBranch` 可以先以 dry-run 输出 pack / upload plan；真实 provider upload
只能上传显式传入且已通过 manifest/digest 校验的 encrypted pack artifact file。
该阶段不读取 stale UI state，不把 provider metadata 当 authority，也不写 ledger、
staging、commit anchor 或 Projection Workspace。

`RestoreBackup` 的 provider download 只能由 Backup Runtime 自有 adapter 执行，
并且只返回 encrypted artifact bytes 给 verification / decrypt pipeline。未完成
manifest-backed verification、artifact authentication 与 decrypt gate 前，CLI/UI 不得把
download success 呈现为 restore success，也不得写 ledger、staging、commit anchor、
Projection Workspace 或 Git mirror queue。

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
  equivalent integrity mechanism owned by `08_auth.md`。
- provider upload credential resolution 初始只允许 `env:` ref；`keyring:` 与
  `config:` 必须 fail-closed，直到对应 resolver 被纳入本章合同。
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
