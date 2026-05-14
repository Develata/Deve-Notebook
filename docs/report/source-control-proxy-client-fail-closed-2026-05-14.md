# Source Control Proxy Client Fail-Closed - 2026-05-14

## Scope

- Runtime surface: plugin-host proxy mode and remote Source Control API bridge.
- Plan basis: `docs/plan/07_diff_logic.md#source-control-runtime`.

## Change

- Converted Source Control proxy HTTP client construction from panic-based `expect` to `anyhow::Result`.
- Propagated client build failure through `RemoteSourceControlApi::new` and proxy-mode startup.
- Kept loopback `no_proxy` behavior unchanged.
- Added a source-control baseline guard so this path cannot regain the old `expect` silently.

## Verification

- `cargo test -p deve_cli source_control_proxy -- --nocapture`
- `bash scripts/check-source-control-baseline.sh`
- `bash scripts/plan-coverage.sh`
- `cargo fmt --check`

## Result

Source Control proxy startup now fails through the command error path instead of panicking if HTTP client construction fails.
