# Android Emulator Admission Feature Negotiation Evidence — 2026-08-01

## Scope

This is historical diagnostic evidence. It does not change the formal Android
target-host gate, create an acceptance receipt, authorize a candidate dispatch,
or alter the release freeze.

## Exact failed run

- Repository: `Develata/Deve-Notebook`
- Workflow: `android-emulator-admission.yml`
- Run: `30690812038`
- Attempt: `1`
- HEAD: `aea698daa7b8e62c2e18343390f7dda73fbe9b94`
- APK SHA-256: `66576d5ed40389eec99ec214dbe7c6e9db54df2ecf5b2be9c638da82b82027f2`
- Pinned emulator: `36.6.11.0`, build `15507667`
- System image: Android API 37.0 Google APIs x86_64, revision `6`

All nine requested cold-boot cycles published complete structured results and
reported `cleanupStatus=0`. None was stable:

| Requested renderer | Actual Vulkan/GLES pair | Stable cycles |
|---|---|---:|
| `swangle` | `swiftshader swangle` | 0/3 |
| `software` | `lavapipe swangle` | 0/3 |
| `swiftshader` | `swiftshader swiftshader` | 0/3 |

Eight captured crash buffers showed the same `surfaceflinger` SIGABRT on the
`RegionSampling` thread. The guest mapper emitted:

```text
Assertion failed: !rcEnc->featureInfo()->hasReadColorBufferDma
```

Every bounded emulator log also showed:

```text
gfxstreamFeature:HasSharedSlotsHostMemoryAllocator = 0
gfxstreamFeature:GlDirectMem = 0
```

The ninth cycle failed earlier during boot admission but carried the same host
feature state. This run rejects APK bytes, finalizer cleanup and the three
renderer paths as sufficient fixes. The earlier exact-HEAD run `30603301298`,
recorded in
[`android-emulator-admission-renderer-evidence-2026-07-31.md`](android-emulator-admission-renderer-evidence-2026-07-31.md),
separately rejected emulator source and API 36.1 versus 37.0.

## Source triangulation

The upstream AOSP guest mapper calls `readFromHost()` for a CPU-readable host
buffer and fails closed when `hasReadColorBufferDma` is unavailable before it
binds the DMA buffer and calls `rcReadColorBufferDMA`:

- [`device/generic/goldfish` mapper at commit
  `3ce87694caeb4a330280bb1284d9ad050fe28263`](https://android.googlesource.com/device/generic/goldfish/+/3ce87694caeb4a330280bb1284d9ad050fe28263/hals/gralloc/mapper.cpp#593).

The upstream gfxstream host advertises
`ANDROID_EMU_read_color_buffer_dma` only when both `GlDirectMem` and
`HasSharedSlotsHostMemoryAllocator` are enabled:

- [`platform/hardware/google/gfxstream` host at commit
  `d047a57228332d995d36600792fa9ccc26cf8ae6`](https://android.googlesource.com/platform/hardware/google/gfxstream/+/d047a57228332d995d36600792fa9ccc26cf8ae6/host/RenderControl.cpp#496).

The emulator's CLI-facing FeatureControl identifier is the case-sensitive
`GLDirectMem`, while gfxstream's internal `FeatureSet` and log key use
`GlDirectMem`. `FeatureControlImpl::fromString` compares the CLI token exactly
and ignores unknown names after a warning; its command-line parser walks every
repeated `-feature` value. The renderer bridge then maps `GLDirectMem` to
`FeatureSet::GlDirectMem` and maps `HasSharedSlotsHostMemoryAllocator` without a
case change:

- [`FeatureControlImpl.cpp` at emulator commit
  `ae9d18d2b6261179fbd57fffec720a04f7bfb053`](https://android.googlesource.com/platform/external/qemu/+/ae9d18d2b6261179fbd57fffec720a04f7bfb053/android/emu/feature/src/android/featurecontrol/FeatureControlImpl.cpp#428);
- [`opengles.cpp` at the same commit](https://android.googlesource.com/platform/external/qemu/+/ae9d18d2b6261179fbd57fffec720a04f7bfb053/android/android-emu/android/opengles.cpp#281).

This matches the target-host logs and explains why adding install sleeps,
package-service retries, changing API level, or selecting another software
renderer cannot address the crash.

## Next diagnostic cut

Keep the exact APK build, pinned emulator, API 37 image, `swangle`, AVD, RAM,
readiness, install, timeout, cleanup and three-cycle boundaries fixed. Compare
only:

1. default features — unchanged `0/0` failure control;
2. CLI `GLDirectMem` — isolates gfxstream `GlDirectMem` alone;
3. CLI `GLDirectMem + HasSharedSlotsHostMemoryAllocator` — the exact conjunction
   required for the host DMA extension.

Each cycle must parse the actual renderer pair and both actual feature states
from its bounded log. A feature policy can be proposed for the formal gate only
after all three cycles pass guest-service admission, install, post-install
admission and `system_server` PID continuity. The `1/0` result remains a
negative control even if it happens not to crash during three cycles; only a
stable exact `1/1` conjunction may be proposed when the default control is
unstable. Formal gate modification and a fresh exact-HEAD candidate remain
separate steps.

## Completed feature-policy cut

The follow-up manual diagnostic completed successfully as run `30694491880`,
attempt `1`, on exact HEAD `cd715336943efd4c42b1cc629e99cda510ca40fc`.
Every variant used APK SHA-256
`73d98035af9c2a4df21664d2230658d704a7f3c1c268c59a78f7d37575c8e35d`,
pinned emulator `36.6.11.0` build `15507667`, API 37 image revision `6`, and
the actual `swiftshader/swangle` renderer pair. All cleanup statuses were zero.

| Feature policy | Actual pair | Stable cycles | Outcome |
|---|---:|---:|---|
| default | `0/0` | 0/3 | negative control failed |
| `GLDirectMem` | `1/0` | 0/3 | isolation control failed |
| `GLDirectMem + HasSharedSlotsHostMemoryAllocator` | `1/1` | 3/3 | recommended |

All three `1/1` cycles completed boot admission, install, post-install
admission and preserved `system_server` PID continuity (`635`, `644`, and
`659`, respectively). This closes the diagnostic selection step for the exact
runtime identity above. It does not itself create acceptance evidence: the
formal target-host gate must separately configure the conjunction, verify the
actual log state fail-closed, and pass on a new exact candidate HEAD.
