# Android Emulator Admission Finalizer Evidence — 2026-08-01

## Scope

This report records historical diagnostic evidence only. It does not change the
formal Android release gate, recommend a renderer, create an acceptance receipt,
or authorize another workflow dispatch.

## Exact run

- Repository: `Develata/Deve-Notebook`
- Workflow: `android-emulator-admission.yml`
- Run: `30619419910`
- Attempt: `1`
- HEAD: `06ab2b1f09008da704d47b43cf863921c1bfb763`
- APK SHA-256: `d47329fe1732c3d757f3d78d9f73cd48c1235a3ff8daa510d83d91cf7ef98ac3`
- Pinned emulator: `36.6.11.0` (`build_id 15507667`)
- SDK emulator revision: `37.1.11`
- API 37 system-image revision: `6`

All three variants reached a real renderer observation during the first cold
boot:

| Variant | First-cycle renderer observation |
|---|---|
| `pinned-api37-swangle` | `vulkan_mode_selected:swiftshader gles_mode_selected:swangle` |
| `pinned-api37-software` | `vulkan_mode_selected:lavapipe gles_mode_selected:swangle` |
| `pinned-api37-swiftshader` | `vulkan_mode_selected:swiftshader gles_mode_selected:swiftshader` |

The run is not valid renderer-comparison evidence. The first cycle of every
variant failed while the cycle finalizer was running, before cleanup and atomic
cycle-result publication completed. The remaining cycles then rejected the
still-owned serial as already in use and observed the orphaned first emulator,
so they were not independent cold boots.

## Root cause

The shared failure was:

```text
scripts/diagnose-android-emulator-admission.sh: line 315: cycle_dir: unbound variable
```

`run_cycle` is an isolated Bash subshell function with `set -euo pipefail`. Its
`EXIT` trap needs the cycle path, owner marker, child PID, timestamps and result
state. Those values were declared as function-local variables. When an ordinary
cycle command triggered implicit `errexit`, Bash unwound the function-local
scope before invoking the subshell `EXIT` trap. The trap therefore could not
read `cycle_dir`, skipped bounded emulator cleanup, and did not publish the
first cycle JSON.

The per-variant writer also trusted the caller's `complete` flag without
checking the collected cycle set. Each remote variant therefore advertised
`complete: true` while containing only cycles 2 and 3. The matrix summarizer
rejected those artifacts, so no renderer recommendation was produced, but the
variant JSON itself was inaccurate. The writer now requires the exact cycle
sequence `1..requestedCycles`; any missing, duplicate or non-canonical set is
published as `complete: false`, `stable: false` with a harness error.

The correction keeps the existing isolated subshell and EXIT cleanup boundary,
but stores finalizer context at subshell scope instead of function-local scope.
Regression coverage must exercise an implicit-errexit exit path and prove that
the trap can still read its context, perform cleanup publication, and return the
original non-zero status. The Rust release baseline must reject a worker that
reintroduces function-local finalizer context.

## Boundary decision

- Do not select a renderer from run `30619419910`.
- Do not change the pinned API 37 formal candidate gate from this run.
- Do not change release-freeze, candidate/aggregate/tag state, or STORE-016.
- After the local harness correction passes review and baseline checks, a new
  exact-HEAD attempt-1 admission run is required before making a renderer
  recommendation.
