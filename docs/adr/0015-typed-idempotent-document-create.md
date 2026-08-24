# 0015. Typed idempotent Document Create

- Status: Accepted
- Date: 2026-08-24

## Context

Before the first public tag, document Create only queued an uncorrelated WebSocket
mutation and relied on a later broadcast projection refresh. A DOM click, search
surface closure, `ProjectionRecoveryRequired`, or final DocList timeout could not
distinguish transport loss, authority rejection, committed-but-degraded writeback,
or a delayed projection. Retrying with a newly generated identity could create a
duplicate document after an unknown commit.

The F4/v5 target was never published, so the protocol may still make one lockstep
pre-publication cut without a compatibility adapter.

This ADR supersedes only the unpublished F4/v5 wire-epoch portion recorded by
ADR 0014. ADR 0014's ownership-aware repository-removal decision remains accepted.

## Decision

- Advance the unpublished first-public target to F4/v6 with a `6..=6` lockstep
  compatibility window and unchanged `DEVEWSF4` magic.
- Add nested `DocumentCreateRequest / DocumentCreateResponse` messages.
- The client proposes one stable NodeId UUID. For a Markdown file, the backend
  derives the DocId from that same UUID; for a directory, the NodeId is exact and
  no DocId exists.
- The backend remains the sole authority for scope/path validation, normalized
  target, Ledger Structure Facts, projection writeback, and result classification.
- Same UUID plus same normalized target is idempotent. UUID/target/kind mismatch
  fails closed. Request confirmation is unicast; observer convergence remains a
  separate `ProjectionRecoveryRequired` broadcast.
- Typed success carries the backend-normalized target. Web opens a created document only after both typed success and an exact
  authority-derived DocList identity are observed. A same-page, same-repo,
  local-branch internal reconnect may replay the original UUID once after fresh
  WriteReady.

## Consequences

- Web, CLI, Docker, Desktop, and Mobile protocol consumers must rebuild in
  lockstep; unpublished F4/v5 frames fail closed and receive no adapter.
- Create can distinguish rejected, committed, and recovery-required outcomes,
  and a lost acknowledgement can be recovered without duplicate Structure Facts.
- Create gains a stronger contract than the remaining legacy document structure
  mutations. Rename/copy/move/delete are not auto-replayed until they migrate to
  an equivalent typed idempotent family.
- Exact-HEAD release and Android receipts must be regenerated.

## References

- docs/plan/07_network.md
- docs/plan/09_web_thin_client_ledger.md
- docs/features/16_web_thin_client_ledger.md
- docs/acceptance-cases/06_network.md
- docs/registry/first-tag-format-matrix.md
