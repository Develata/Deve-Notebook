# 0011. Immutable ledger-first Remote Import

- Status: Accepted
- Date: 2026-07-17

> The post-commit Projection Fault store and atomic receipt-settlement boundary
> are refined by ADR 0012; this ADR remains Accepted.

## Context

ADR 0007 chose a Projection Backup pull path that downloaded Markdown directly
into the Projection Workspace and then relied on watcher/scan to create External
Changes. That path makes untrusted provider content visible in the live
workspace before the user reviews it, couples remote transport to workspace
rollback/continuation logic, and routes a foreign-import concern through the
External Changes and Source Control controllers.

No formal version has been published. The project can therefore replace that
development path without an adapter, dual write, or compatibility branch while
preserving the durable Ledger envelope, Ledger payload v3, sync facts, and
Projection format.

## Decision

Replace Remote Projection pull with an independent, immutable, ledger-first
**Remote Import** route:

```text
Remote provider
  -> remote_projection_transport_runtime
  -> immutable manifest/blob capture
  -> remote_import_runtime
  -> review/blockers
  -> sealed authority writer
  -> Ledger commit
  -> Projection writeback
  -> Workspace
```

The accepted contract is:

1. WebDAV/S3 transport remains responsible only for locator/profile admission,
   ordered listing/streaming, and push/source acquisition. It does not own
   sessions, Ledger authority, or workspace mutation.
2. Prepare reserves one durable session per `RepoId`, streams provider content
   into a host-only immutable manifest/blob set, verifies deterministic digests
   and budgets, and publishes a candidate revision. It never writes the
   Projection Workspace or External Changes.
3. Review is whole-session and upsert-only for the first public preview. Remote
   absence does not mean Delete, there is no per-file selection, and any typed
   blocker disables Apply for the complete session.
4. Apply uses a source-specific constructor for a crate-private
   `PreparedLedgerChangeBatch`. One Redb transaction revalidates repo/schema/head,
   session/revision/digests, writer/branch/locator/ignore snapshots, and
   pending/staged overlap before atomically appending all facts and the durable
   apply receipt. The transaction is not split.
5. Projection writeback occurs only after the Ledger transaction commits. A
   writeback failure returns a committed, projection-degraded receipt and never
   rolls back Ledger facts.
6. Redb advances to schema v4 for the session/runtime tables. Development v3
   databases fail closed; there is no in-place adapter. Old data may be exported
   from the old HEAD and rebuilt explicitly.
7. The unpublished F4/v2 WebSocket schema is replaced by F4/v3 lockstep. Legacy
   JSON text and unversioned JSON fallbacks are removed; explicitly versioned
   debug JSON remains a development surface. ADR 0013 later supersedes only
   this still-unpublished wire target with F4/v4 Repo Control.
8. The Remote Import Web client is a sibling of Source Control and External
   Changes. It may reuse presentation primitives, but not their controller,
   state, authority, or detail-parsing behavior.

ADR 0008 remains authoritative for S3-compatible credential/profile binding.
This decision changes the destination of provider source acquisition, not the
credential authority.

## Consequences

- ADR 0007 is superseded. Projection Backup push remains as Remote Projection
  push; the pull/workspace-overwrite/rollback consequences are retired.
- ADR 0009's unpublished F4/v2 decision history is superseded before
  publication by F4/v3; no v1 or v2 adapter is created.
- `DEVELDG3`, Ledger payload v3, sync facts, Projection format, and remote-shadow
  merge semantics remain unchanged.
- Redb v4 remains the first-tag target. The later ADR 0013 makes WS v4 the
  first-tag target; F4/v3 is development history and receives no adapter.
- Failure recovery uses explicit discard/repair and durable
  `cleanup_pending`; it does not infer rollback, delete authority, or cleanup
  success from filesystem state.

## References

- docs/plan/03_storage/authority.md
- docs/plan/03_storage/index.md
- docs/plan/05_diff_logic.md
- docs/plan/06_backup.md
- docs/plan/07_network.md
- docs/plan/09_web_thin_client_ledger.md
- docs/plan/12_source_control_ui.md
- docs/plan/14_commands.md
- docs/plan/18_release.md
- docs/adr/0008-s3-compatible-remote-projection-credential-binding.md
