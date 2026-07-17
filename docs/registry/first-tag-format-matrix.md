<!-- Generated: 2026-07-07 | Updated: 2026-07-17 -->

# First-Tag Format Matrix

This registry is the approved first-tag target index. It does not introduce new
engineering rules; authoritative behavior remains in `docs/plan/`.

## Matrix

| Surface | Authority | Current first-tag shape | Code owner | Failure / reset / repair posture |
|---|---|---|---|---|
| Ledger entry payload | `docs/plan/03_storage/authority.md#ledger-entry-format-contract` | `LEDGER_ENTRY_FORMAT_VERSION = 3`; magic `DEVELDG3`; project-owned postcard payload | `crates/core/src/models/ledger_decode.rs` | Missing magic, missing version, old codec payload, or unsupported version fail closed. Explicit offline `--allow-legacy-v2` export may read `DEVELDG2` without admitting it to normal runtime; stable changes require a migration path. |
| Redb repo schema gate | `docs/plan/03_storage/authority.md#redb-schema-version-contract` | `REDB_SCHEMA_VERSION = 3`; `REPO_METADATA[1] = redb_schema_version`; project-owned postcard metadata payloads | `crates/core/src/ledger/schema.rs` | Missing or mismatched top-level schema version fails closed before normal repo/query entry. v2 is restricted to explicit offline read-only export; stable changes require a migration path. |
| WebSocket binary protocol | `docs/plan/07_network.md#server-ws-runtime` | `WS_PROTOCOL_VERSION = 2;`; `MIN_SUPPORTED_WS_PROTOCOL_VERSION = 2;`; magic `DEVEWSF4`; project-owned postcard frame payload; capability-level workspace ingestion unavailable error; Diff uses backend typed projections and on-demand commit-file requests | `crates/core/src/protocol/frame.rs` | Missing/wrong binary magic or unsupported protocol version fails closed with structured protocol error. First-tag policy is F4 lockstep `2..=2`; historical development F2/F3 namespaces and F4/v0, F4/v1 or F4/v13 have no adapter. After first publication F4 versions only increase monotonically. |
| Projection Locator | `docs/plan/03_storage/projection.md#projection-locator-contract` | Host-local locator file `ledger/.host/projection-locators.toml`; derived workspace root `<projection_base>/<safe_repo_name>--<repo_id>/`; no ledger or sync payload version | `crates/core/src/ledger/manager/projection_locator.rs` | Locator missing, non-canonical, conflicting, nested, or identity-marker mismatch enters `DegradedLocator` and must use explicit locator repair/reset/rebuild flow. This host-local shape is frozen for first tag even though it is not a codec version. |
| Projection Backup / Remote Projection locator | `docs/plan/06_backup.md#projection-backup-locator-contract`; `docs/plan/05_diff_logic.md#remote-projection-transport` | WebDAV/S3 locator transports Markdown Projection Workspace files only; Web typed intent carries provider/direction only; accepted `s3+https://` route is host-local secret-free Remote Projection profile binding. CLI can pass an explicit profile handle; Web custom endpoint UX remains backend-profile-handle-only. | `crates/core/src/remote_projection/`; `apps/cli/src/commands/projection_remote.rs` | Locator missing, scheme/provider mismatch, `urn:*`, unsupported transport, unbound custom S3 endpoint, unsafe path, duplicate path, budget overflow, profile mismatch, or credential resolver failure reports `provider_io_ready=false` before authority effects. Pull writes only Projection Workspace and enters External Changes. |

## Release Gate Binding

`cargo run -p deve_baseline -- release` pins this registry, the plan constants,
and the code constants above. A format change must update `docs/plan/` first,
then this registry and the matching baseline spec before code can claim first-tag
readiness.

Implementation status at W0: the approved first-tag contract is frozen at F4/v2,
while `crates/core/src/protocol/frame.rs` and the release baseline still expose the
unpublished development F4/v1 shape. Therefore `deve_baseline release`, REL-003,
release candidate and tag-ready are intentionally blocking until W4 performs the
single code/baseline cutover; no W0-W3 result may be reported as format-bound or
release-ready.
