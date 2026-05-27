# 0004. Tauri v2 as the native packaging target

- Status: Accepted
- Date: 2026-05-27

## Context

Desktop and Mobile shells need a path to native packaging without letting
packaging dependencies leak into the default build or grant the shell authority
over ledger/projection/source-control. The native ecosystem choice was between
committing broadly to a framework now versus reserving an interface.

## Decision

Adopt **Tauri v2** as the native packaging target, but gated: `tauri` /
`tauri-build` may appear only inside the `native-packaging` feature scope of the
`apps/desktop` and `apps/mobile` crates. The default build stays no-Tauri
(shell skeleton), and the native shell core holds no authority.

## Consequences

- Desktop/Mobile native-adapter contracts are written against a gated Tauri target.
- A `native-packaging` dependency gate governs when real Tauri deps may enter.
- Default surface is no-runtime (no real child process, no Tauri runtime capability). A desktop local service (e.g. `DEVE_DESKTOP_LOCAL_SERVICE`) may start a child process only under `native-packaging` + explicit opt-in, and still holds no ledger/projection/source-control authority.

## References

- docs/plan/17_tech_stack.md (Build; native-packaging-dependency-gate)
- docs/plan/11_ui_design/02_desktop.md, docs/plan/11_ui_design/03_mobile.md (native adapter contracts)
