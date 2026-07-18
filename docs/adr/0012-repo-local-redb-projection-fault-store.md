# 0012. Repo-local Redb Projection Fault store

- Status: Accepted
- Date: 2026-07-18

## Context

ADR 0011 requires Remote Import Apply to store an immutable receipt with
projection outcome `Pending`, then converge that outcome in a second short Redb
transaction. On writeback failure, the same transaction must both persist the
normal durable Projection Fault evidence and compare-and-set the receipt to
`Degraded`.

The existing implementation stored Projection Faults in the host-wide
`ledger/.host/projection-faults.toml` file. A Redb transaction cannot atomically
commit a TOML file and a Remote Import session row. Keeping both would create
two durable authorities and an unrecoverable crash window between them.

No formal version has been published, so the incomplete TOML implementation can
be removed without a dual write, adapter, or migration branch.

## Decision

Replace the TOML journal with the repo-local Redb v4 `PROJECTION_FAULTS` side
table. A crate-private `crates/core/src/projection_fault/` support module owns
its model and narrow Redb primitives; this is not a new runtime or public API.

1. `projection_persistence_runtime` exclusively owns the typed Projection Fault
   store API. The table is recovery evidence, not a Ledger Fact authority, and
   is never synchronized.
2. A domain-separated deterministic SHA-256 key binds the exact `RepoId`, fault
   kind, typed origin, and normalized semantic target. Each versioned record binds the same
   identity plus affected
   projection identity, Ledger head/range evidence, timestamps, bounded
   diagnostic detail, retry count, and pending status. Remote Import origins
   additionally bind `session_id`, candidate revision, and `request_id`.
3. Ordinary projection writeback/rebuild failures use a short repo-local Redb
   transaction. Repair deletes a pending record only after exact RepoId and
   projection revalidation succeeds.
4. The Remote Import post-commit coordinator owns the dedicated short Redb
   transaction. It calls the Projection Fault store's narrow `upsert_in_txn`
   primitive and the Remote Import store's receipt-specific CAS in that same
   transaction. Projection code never owns or mutates Remote Import receipt
   state. Writeback success CASes the receipt to `Written` without a fault; a
   failed transaction leaves it `Pending` for idempotent recovery.
5. The store codec must recompute and exact-compare each key while reading;
   unsupported value versions, RepoId mismatch, malformed payload, or key/value
   mismatch fail closed.
6. The TOML journal, global journal mutex, dual-write behavior, and unpublished
   compatibility path are deleted. A v4 local database missing the required
   table fails closed as an incomplete development schema.
7. Startup validates the complete local v4 table profile, loads fault rows and
   Applied/Pending receipts per repo, recovers Pending receipts idempotently,
   and only then materializes/scans and publishes `RepoHealth`.

The Projection Fault store remains orthogonal to `RepoMountState`. Watcher
startup, worker, overflow, or shutdown failures never create Projection Fault
records unless independent projection evidence also exists.

## Consequences

- Remote Import can satisfy the second-transaction atomicity required by ADR
  0011 without splitting its whole-session Ledger transaction.
- Projection recovery evidence is isolated per `RepoId`; one corrupt host-wide
  file can no longer hide or conflate unrelated repos.
- Opening a pre-decision development v4 database without `PROJECTION_FAULTS`
  fails closed. Rebuild from a supported export is the only recovery route.
- `DEVELDG3`, Ledger payload v3, sync facts, Projection format, Remote Import
  manifests/blobs, and WebSocket framing are unchanged.
- A historical receipt that reached `Degraded` remains immutable history after
  a later successful repair; repair clears the active fault record rather than
  rewriting the receipt to `Written`.

## References

- docs/plan/03_storage/authority.md
- docs/plan/03_storage/projection.md
- docs/plan/04_repository.md
- docs/plan/06_backup.md
- docs/plan/22_reliability_observability.md
- docs/adr/0011-immutable-ledger-first-remote-import.md
