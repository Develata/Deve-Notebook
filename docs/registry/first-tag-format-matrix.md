<!-- Generated: 2026-07-07 -->

# First-Tag Format Matrix

This registry is a current-state release index. It does not introduce new
engineering rules; authoritative behavior remains in `docs/plan/`.

## Matrix

| Surface | Authority | Current first-tag shape | Code owner | Failure / reset / repair posture |
|---|---|---|---|---|
| Ledger entry payload | `docs/plan/03_storage/authority.md#ledger-entry-format-contract` | `LEDGER_ENTRY_FORMAT_VERSION = 2`; magic `DEVELDG2`; project-owned postcard payload | `crates/core/src/models/ledger_decode.rs` | Missing magic, missing version, old codec payload, or unsupported version fail closed. Pre-1.0 data may require explicit reset, repair, or migration; stable changes require a migration path. |
| Redb repo schema gate | `docs/plan/03_storage/authority.md#redb-schema-version-contract` | `REDB_SCHEMA_VERSION = 2`; `REPO_METADATA[1] = redb_schema_version`; project-owned postcard metadata payloads | `crates/core/src/ledger/schema.rs` | Missing or mismatched top-level schema version fails closed before repo/query entry. Pre-1.0 old schema gates may require explicit reset, repair, or migration; stable changes require a migration path. |
| Backup pack plaintext | `docs/plan/06_backup.md#backup-pack-plaintext-schema-contract` | `BACKUP_PACK_PLAINTEXT_FORMAT_VERSION = 2`; magic `DEVEBKP2`; project-owned postcard payload | `crates/core/src/backup/plaintext.rs` | Unversioned plaintext, invalid magic, metadata mismatch, corrupt ledger entry, snapshot/blob ref gap, or old codec payload fails closed before RestoreCandidate admission. Import/merge must consume verified candidate evidence only. |
| WebSocket binary protocol | `docs/plan/07_network.md#server-ws-runtime` | `WS_PROTOCOL_VERSION = 11;`; `MIN_SUPPORTED_WS_PROTOCOL_VERSION = 11;`; magic `DEVEWSF3`; project-owned postcard frame payload | `crates/core/src/protocol/frame.rs` | Missing binary magic or unsupported protocol version fails closed with structured protocol error. First-tag policy is lockstep `11..=11`; lowering minimum without adapters is not compatibility. |
| Projection Locator | `docs/plan/03_storage/projection.md#projection-locator-contract` | Host-local locator file `ledger/.host/projection-locators.toml`; derived workspace root `<projection_base>/<safe_repo_name>--<repo_id>/`; no ledger or sync payload version | `crates/core/src/ledger/manager/projection_locator.rs` | Locator missing, non-canonical, conflicting, nested, or identity-marker mismatch enters `DegradedLocator` and must use explicit locator repair/reset/rebuild flow. This host-local shape is frozen for first tag even though it is not a codec version. |
| Remote Projection locator | `docs/plan/05_diff_logic.md#remote-projection-transport` | `repo_url`-derived WebDAV/S3 transport locator; Web typed intent carries provider/direction only; `s3+https://` remains fail-closed until a credential profile route is accepted | `crates/core/src/remote_projection/`; `apps/cli/src/commands/projection_remote.rs` | Locator missing, scheme/provider mismatch, `urn:*`, unsupported transport, or unbound custom S3 endpoint reports `provider_io_ready=false` before provider I/O. Pull writes only Projection Workspace and enters External Changes. |
| Backup locator / manifest | `docs/plan/06_backup.md#backup-locator-contract`; `docs/plan/06_backup.md#backup-root-contract` | WebDAV/S3/S3-compatible locator string carries discovery/routing only; backup root and branch manifests carry typed format/version and RepoId evidence | `crates/core/src/backup/locator.rs`; `crates/core/src/backup/branch_manifest.rs` | Locator and provider metadata are never authority. RepoId mismatch, manifest mismatch, digest/authentication failure, or decrypt-before-verify attempt fails closed before restore/import/merge effects. |

## Release Gate Binding

`cargo run -p deve_baseline -- release` pins this registry, the plan constants,
and the code constants above. A format change must update `docs/plan/` first,
then this registry and the matching baseline spec before code can claim first-tag
readiness.
