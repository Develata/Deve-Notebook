<!-- Generated: 2026-07-07 -->

# First-Tag Format Matrix

This registry is a current-state release index. It does not introduce new
engineering rules; authoritative behavior remains in `docs/plan/`.

## Matrix

| Surface | Authority | Current first-tag shape | Code owner | Failure / reset / repair posture |
|---|---|---|---|---|
| Ledger entry payload | `docs/plan/03_storage/authority.md#ledger-entry-format-contract` | `LEDGER_ENTRY_FORMAT_VERSION = 2`; magic `DEVELDG2`; project-owned postcard payload | `crates/core/src/models/ledger_decode.rs` | Missing magic, missing version, old codec payload, or unsupported version fail closed. Pre-1.0 data may require explicit reset, repair, or migration; stable changes require a migration path. |
| Redb repo schema gate | `docs/plan/03_storage/authority.md#redb-schema-version-contract` | `REDB_SCHEMA_VERSION = 2`; `REPO_METADATA[1] = redb_schema_version`; project-owned postcard metadata payloads | `crates/core/src/ledger/schema.rs` | Missing or mismatched top-level schema version fails closed before repo/query entry. Pre-1.0 old schema gates may require explicit reset, repair, or migration; stable changes require a migration path. |
| WebSocket binary protocol | `docs/plan/07_network.md#server-ws-runtime` | `WS_PROTOCOL_VERSION = 11;`; `MIN_SUPPORTED_WS_PROTOCOL_VERSION = 11;`; magic `DEVEWSF3`; project-owned postcard frame payload | `crates/core/src/protocol/frame.rs` | Missing binary magic or unsupported protocol version fails closed with structured protocol error. First-tag policy is lockstep `11..=11`; lowering minimum without adapters is not compatibility. |
| Projection Locator | `docs/plan/03_storage/projection.md#projection-locator-contract` | Host-local locator file `ledger/.host/projection-locators.toml`; derived workspace root `<projection_base>/<safe_repo_name>--<repo_id>/`; no ledger or sync payload version | `crates/core/src/ledger/manager/projection_locator.rs` | Locator missing, non-canonical, conflicting, nested, or identity-marker mismatch enters `DegradedLocator` and must use explicit locator repair/reset/rebuild flow. This host-local shape is frozen for first tag even though it is not a codec version. |
| Projection Backup / Remote Projection locator | `docs/plan/06_backup.md#projection-backup-locator-contract`; `docs/plan/05_diff_logic.md#remote-projection-transport` | WebDAV/S3 locator transports Markdown Projection Workspace files only; Web typed intent carries provider/direction only; accepted `s3+https://` route is host-local secret-free Remote Projection profile binding. CLI can pass an explicit profile handle; Web custom endpoint UX remains backend-profile-handle-only. | `crates/core/src/remote_projection/`; `apps/cli/src/commands/projection_remote.rs` | Locator missing, scheme/provider mismatch, `urn:*`, unsupported transport, unbound custom S3 endpoint, unsafe path, duplicate path, budget overflow, profile mismatch, or credential resolver failure reports `provider_io_ready=false` before authority effects. Pull writes only Projection Workspace and enters External Changes. |

## Release Gate Binding

`cargo run -p deve_baseline -- release` pins this registry, the plan constants,
and the code constants above. A format change must update `docs/plan/` first,
then this registry and the matching baseline spec before code can claim first-tag
readiness.
