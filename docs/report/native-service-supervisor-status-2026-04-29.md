# Native Service Supervisor Status - 2026-04-29

## Result

P3-10 native embedded service supervisor contract has landed.

Implemented scope:

- Added `deve_core::native_adapter::NativeServiceSupervisor`.
- Supervisor state is explicit: `Idle`, `Starting`, `EndpointHealthy`,
  `SessionHandoffReady`, `Restarting`, and `Offline`.
- Health probe is intentionally narrow: endpoint reachable plus node role
  readable. It does not imply repo writer readiness.
- Session handoff is a separate transition and requires a healthy endpoint plus
  bound session.
- Failure classification is explicit: bind/probe/process-exit failures are
  retryable within the restart budget; spawn and session-handoff failures are
  fatal by default.
- `apps/desktop` and `apps/mobile` shell skeletons now carry supervisor
  snapshots and route supervisor failures to service-offline recovery.
- `ServerLaunchOptions::native_loopback` has test coverage against a supervisor
  snapshot that distinguishes endpoint health from session handoff readiness.

Boundary:

- This is still not a real process supervisor.
- No Tauri dependency or native packaging runtime was introduced.
- Supervisor state does not grant ledger/vault/source-control/search/`.git`/
  `.notegit` authority.
- Recovery bootstrap still omits internal failure reason, token, secret, and
  repo write permission.

## Verification

Commands run:

```bash
cargo test -p deve_core native_adapter
cargo test -p deve_desktop
cargo test -p deve_mobile
cargo test -p deve_cli native_launch
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features
cargo fmt --all --check
scripts/plan-coverage.sh
git diff --check
```

Observed result:

- Core native adapter supervisor tests passed.
- Desktop and mobile shell tests passed.
- CLI native launch supervisor tests passed.
- Workspace all-targets all-features check passed.
- Workspace all-targets all-features tests passed.
- Formatting, plan coverage, and whitespace checks passed.

## Next Work

The next native-track batch should stay no-Tauri by default and decide whether
to implement a real process adapter behind the supervisor contract or defer it
until the Tauri dependency gate opens.
