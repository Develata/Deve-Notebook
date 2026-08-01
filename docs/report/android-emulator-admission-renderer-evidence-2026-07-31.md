# Android Emulator Admission Renderer Evidence — 2026-07-31

## Scope

This report records historical diagnostic evidence only. It does not change the
release gate, create an acceptance receipt, or authorize a candidate dispatch.

## Exact run

- Repository: `Develata/Deve-Notebook`
- Workflow: `android-emulator-admission.yml`
- Run: `30603301298`
- Attempt: `1`
- HEAD: `61988af1e29440d44ec1a90d5fa88d7a369a92bd`
- APK SHA-256: `f10b987142d4dd16ee56c2bc32f63107005afce20c2bc7e2a97e9e1b5977e60b`

The run compared pinned emulator + API 37, SDK emulator + API 37, and pinned
emulator + API 36.1. Neither emulator source nor API level admitted a stable
configuration:

| Variant | Observed result |
|---|---|
| pinned emulator 36.6.11 + API 37 | 0/3; all `binder_epipe` |
| SDK emulator 37.1.11 + API 37 | 0/3; all `binder_epipe` |
| pinned emulator 36.6.11 + API 36.1 | two `binder_epipe`; one invalid false pass |

The false pass came from invoking a Bash function through `if run_cycle ...`,
which suppressed the function's `errexit` behavior. The matrix summary rejected
that result because the claimed pass had no continuous `system_server` PID.
Local commit `b830a2da70ba1d46ad5fb45c657000325fffc9dc` fixes the invocation and adds a
regression contract; it was not part of run `30603301298`.

## Root-cause evidence

All eight explicit failed cycles captured repeated `surfaceflinger` aborts in
the crash buffer:

```text
Fatal signal 6 (SIGABRT) ... (RegionSampling) ... (surfaceflinger)
Abort message: 'Assertion failed: !rcEnc->featureInfo()->hasReadColorBufferDma'
```

The three variants all launched with `-gpu swangle`, and their emulator logs
reported `vulkan_mode_selected:swiftshader gles_mode_selected:swangle`.
Therefore emulator source and API level are rejected as sufficient fixes. The
next supported hypothesis is renderer-path instability; this does not yet prove
that another renderer is stable.

## Next diagnostic cut

Keep the exact APK build, pinned emulator identity, API 37 `google_apis` x86_64
system image, memory, AVD, readiness, install, timeout and cleanup boundaries
fixed. Parse the unique actual Vulkan/GLES renderer pair from each bounded
emulator log; a requested mode alone is not renderer proof. Compare only:

1. `swangle` — unchanged failure control;
2. `software` — generic software fallback;
3. `swiftshader` — explicit software renderer.

A renderer may be proposed for the formal gate only after all three of its cold
boots pass guest-service admission, APK install, post-install admission and
`system_server` PID continuity. A formal gate change and candidate dispatch
remain separate exact-HEAD work.

## Outcome

Exact-HEAD run `30690812038` completed this renderer cut and found no stable
variant. All three renderer paths retained the same gfxstream feature-negotiation
failure class. The later evidence and next diagnostic cut are recorded in
`docs/report/android-emulator-admission-feature-negotiation-evidence-2026-08-01.md`.
