# Indirect Sync Source Attribution Envelope

Date: 2026-05-16

## Scope

- Plan basis: `05_network.md` §10.2 / §10.5.
- Code paths: `SyncPush`, `SyncPushSnapshot`, server inbound sync transfer handlers, versioned frame schema.
- Non-goal: implement gossip offer/fetch relay, multi-hop re-export storage, or direct ledger authority writes from a relay.

## Changes

- Added `SyncSourceProof` to the sync payload envelope.
- Proof signs `repo_id`, source peer, version vector, payload kind, and encrypted payload digest.
- Inbound indirect `SyncPush` / `SyncPushSnapshot` now requires a valid source proof when transport peer differs from source peer.
- Direct source transport remains compatible: missing proof is accepted when the authenticated transport peer is the source.
- Local outbound sync payloads are signed only when the local identity key is the source.
- Source-proof failure returns structured `SyncInvalidPayload` instead of depending on generic apply-error string classification.
- WebSocket protocol version was bumped to `9` with compatibility window `9..=9` because the bincode message schema changed.

## Verification

Ran:

- `cargo test -p deve_core source_proof -- --nocapture`
- `cargo test -p deve_core sync_transfer_field -- --nocapture`
- `cargo test -p deve_core frame -- --nocapture`
- `cargo test -p deve_cli sync_transfer_push -- --nocapture`
- `cargo test -p deve_cli sync_transfer_snapshot -- --nocapture`
- `cargo test -p deve_cli sync_transfer_scope -- --nocapture`
- `cargo test -p deve_cli receive -- --nocapture`
- `cargo test -p deve_web incoming -- --nocapture`
- `cargo check -p deve_web --target wasm32-unknown-unknown`
- `bash scripts/check-network-baseline.sh`
- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/plan-coverage.sh`

Results:

- Protocol proof signs and verifies valid payloads.
- Payload tamper is rejected.
- Proof signed by relay key while claiming another source is rejected.
- Relay transport cannot write forged diff or snapshot payloads under a claimed source.
- Existing legacy/current sync transfer field compatibility remains intact through serde defaults.
- WS protocol version `9` is accepted by core, CLI receive path, Web incoming path, and network baseline guard.

## Residual Boundary

Full multi-hop re-export still requires storing or forwarding the original source proof with the relay payload. Current code enforces inbound indirect attribution and local-origin proof generation, but does not claim a complete gossip relay protocol.

## Decision

This batch closes the current forged relay payload acceptance gap for existing `SyncPush` / `SyncPushSnapshot` surfaces. Future relay work must preserve the source proof across re-export rather than re-signing with the relay identity.
