# Native Process Adapter Runtime Design

Date: 2026-05-14

## Scope

This design prepares the post-gate child-process runtime. It does not open the
runtime.

## Authority Boundary

- Core may define process runtime contracts, snapshots, and failure taxonomy.
- Core must not import `std::process`, `tokio::process`, or platform spawn APIs.
- Desktop/Mobile runtime code must stay behind each app crate's
  `native-packaging` feature.
- Child-process running does not imply writable UI.
- Writable UI remains gated by endpoint health, auth status, node role, repo
  handshake, writer-ready, and current `scope_nonce`.
- Process runtime must not write ledger, vault, source-control side tables,
  search index, `.git`, or `.notegit`.

## Runtime API Shape

The first implementation batch should add typed contracts only:

- `NativeProcessSpawnSpec`
- `NativeProcessRuntimeHandle`
- `NativeProcessRuntimeEvent`
- `NativeProcessRuntimeSnapshot`
- `NativeProcessRuntimeError`

Required fields:

- executable path
- argv list
- current working directory
- environment allowlist
- selected profile/config/vault/ledger paths
- http/ws bind hints
- pid or platform handle id
- start timestamp
- exit status
- last health probe
- last failure kind

Forbidden fields:

- session secret
- auth token
- repo write permission
- raw stderr/stdout payload without redaction boundary

## State Machine

```text
Disabled
  -> SpawnRequested
  -> Spawned
  -> EndpointProbing
  -> EndpointHealthy
  -> SessionHandoffReady
  -> RuntimeReady
  -> Restarting | Offline | Stopped
```

State rules:

- `SpawnRequested` may only occur when process policy and packaging feature are
  both open.
- `Spawned` only records a child handle; it does not grant endpoint readiness.
- `EndpointHealthy` requires loopback endpoint validation and readable
  `/api/node/role`.
- `SessionHandoffReady` requires an explicit session bind result.
- `RuntimeReady` still requires the Web/runtime writer gates.
- `Stopped` must clear endpoint and session snapshots.

## Failure Contract

Retryable by budget:

- bind failure
- health probe failure
- process exited after successful spawn

Fatal by default:

- spawn executable missing
- spawn permission denied
- invalid executable path
- invalid working directory
- session handoff failure
- non-loopback endpoint
- environment policy violation

All failures must become structured native service failures. They must not be
converted to generic disconnected UI unless the existing auth/network layers do
so explicitly.

## Feature Scope

Desktop first:

- implementation home: `apps/desktop/src/process_runtime/`
- feature: `native-packaging`
- runtime entrypoint may call the process runtime only after shell/window
  creation and before Web writable UI is shown

Mobile later:

- implementation home: `apps/mobile/src/process_runtime/`
- feature: `native-packaging`
- must wait for Android/iOS package preflight and platform lifecycle review
- must preserve foreground reprobe after background/resume

Never allowed:

- workspace-root process helper
- core process spawning
- web process spawning
- CLI server self-spawn backdoor

## Test Matrix

API scaffold batch:

- spawn spec rejects empty executable
- spawn spec rejects relative executable without explicit resolver
- env allowlist rejects unknown variables
- snapshot never carries session secret or token
- process policy remains closed by default
- core contains no process imports

Desktop runtime batch:

- successful fake runtime emits `Spawned -> EndpointHealthy -> SessionHandoffReady`
- spawn failure maps to fatal offline
- health probe failure consumes retry budget
- process exit consumes retry budget
- session handoff failure is fatal offline
- process running without writer-ready does not unlock writable UI
- default build remains no-Tauri and no-process

Mobile runtime batch:

- foreground reprobe clears stale readiness after resume
- background suspend does not discard pending overlay
- process exit while backgrounded returns to recovery UI
- Android/iOS package preflight remains separate from runtime start

## Next Step

Implement only the API scaffold and tests first. Do not implement `Command::new`
or real process spawn in that batch.
