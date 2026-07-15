# 0009. First public WebSocket protocol epoch

- Status: Accepted
- Date: 2026-07-14

## Context

Before the first public tag, Deve-Notebook used development WebSocket namespaces
`DEVEWSF2` and `DEVEWSF3`, with protocol numbers reaching v13. Those builds were
never a published compatibility contract. The first release also converges
repository mutation publication, typed projection recovery, External Apply
receipts, and backend typed diff projection into one final wire shape.

Resetting only the number from 13 to 1 while keeping `DEVEWSF3` would create a
future collision if the public protocol later reached v13: the same
`(magic, version)` identity would refer to two incompatible schemas. Keeping 13
would avoid that collision but would present internal development history as the
first public protocol generation.

## Decision

The first public wire identity is:

- magic `DEVEWSF4`
- `WS_PROTOCOL_VERSION = 1`
- `MIN_SUPPORTED_WS_PROTOCOL_VERSION = 1`

The magic and version together identify the wire schema. F2, F3, F4/v0,
F4/v13, missing magic, and raw legacy payloads fail closed. There is no
compatibility adapter because no older WebSocket protocol was publicly
released. Ledger format v3, redb schema v3, and PeerFactSeq are unchanged.

After F4/v1 is publicly released, versions within the F4 namespace only advance
monotonically. A future compatibility window requires explicit per-version
decode/upgrade adapters and tests; lowering the minimum constant alone is not
compatibility.

## Consequences

- Web, CLI, Docker, Desktop, and Mobile artifacts must be rebuilt in lockstep.
- Historical development clients fail closed rather than connecting to a new
  server with an ambiguous schema.
- Protocol-facing selectors and UI capability markers use stable behavior names
  such as `backend-typed`, not development version labels.
- Historical `docs/report/**` evidence remains unchanged.
- Runtime/storage migration is unnecessary, but current-HEAD release receipts
  must be regenerated because artifacts and HEAD changed.

## References

- docs/plan/07_network.md
- docs/plan/09_web_thin_client_ledger.md
- docs/plan/18_release.md
- docs/registry/first-tag-format-matrix.md
