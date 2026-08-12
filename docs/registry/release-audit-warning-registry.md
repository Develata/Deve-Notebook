<!-- Generated: 2026-07-07 -->

# Release Audit Warning Registry

## Status

This file is the controlled registry for non-vulnerability `cargo audit`
warnings that are allowed to pass the release audit gate. It is current-state
evidence, not a product behavior contract.

Hard vulnerabilities are never allowlisted here. They must be fixed or the
release audit gate must fail.

## Rules

- Every warning emitted by `cargo audit --json` must have exactly one row below.
- `Advisory` must be a RustSec `RUSTSEC-*` id, except `yanked` warnings without
  a RustSec advisory id must use the synthetic advisory key `YANKED`.
- `Decision` must be one of:
  - `direct-migration-before-stable`: first stable/tag readiness needs a direct
    project decision or codec/dependency replacement before data format freeze.
  - `feature-gated-upstream-watch`: warning is behind an optional feature or
    platform shell dependency; keep the feature gate and track upstream.
  - `upstream-upgrade-watch`: warning is transitive through selected upstream
    framework/tooling; replacement is owned by an upstream upgrade batch.
- `Kind` must match the cargo-audit warning group, currently one of
  `unmaintained`, `unsound`, `notice`, or `yanked`.
- `Tag blocker` values:
  - `yes`: before the first formal public tag, the USER must approve the
    retained risk or the dependency must be replaced.
  - `no`: the row is acceptable for a public tag as long as the rationale,
    feature gate, and upstream route remain true.
- Rationale and replacement route must be concrete. `TODO`, `TBD`, and empty
  cells are invalid.

## Warning Registry

The native GTK3/glib rows below are tracked by
`docs/adr/0006-native-linux-gtk3-first-tag-route.md`. That ADR accepts Route 2:
the first formal tag excludes Linux GTK3/WebKitGTK 4.x native artifacts. The
warnings remain registered, but they are not first-tag blockers while those
artifacts stay out of the release set.

| Advisory | Crate | Version | Kind | Decision | Tag blocker | Rationale | Replacement route |
|---|---|---|---|---|---|---|---|
| RUSTSEC-2024-0413 | atk | 0.18.2 | unmaintained | feature-gated-upstream-watch | no | Transitive GTK3 Linux shell dependency through Tauri native-packaging; first formal tag excludes Linux GTK3/WebKitGTK 4.x native artifacts, so this warning is registered but outside the shipped release set and does not grant authority writes. | Keep Linux GTK3 native artifacts excluded until the native shell stack moves to a maintained GTK4/WebKitGTK 6 or equivalent WebView route with refreshed target-host evidence. |
| RUSTSEC-2024-0416 | atk-sys | 0.18.2 | unmaintained | feature-gated-upstream-watch | no | Transitive GTK3 Linux shell dependency through Tauri native-packaging; first formal tag excludes Linux GTK3/WebKitGTK 4.x native artifacts, so this warning is registered but outside the shipped release set and does not grant authority writes. | Keep Linux GTK3 native artifacts excluded until the native shell stack moves to a maintained GTK4/WebKitGTK 6 or equivalent WebView route with refreshed target-host evidence. |
| RUSTSEC-2024-0412 | gdk | 0.18.2 | unmaintained | feature-gated-upstream-watch | no | Transitive GTK3 Linux shell dependency through Tauri native-packaging; first formal tag excludes Linux GTK3/WebKitGTK 4.x native artifacts, so this warning is registered but outside the shipped release set and does not grant authority writes. | Keep Linux GTK3 native artifacts excluded until the native shell stack moves to a maintained GTK4/WebKitGTK 6 or equivalent WebView route with refreshed target-host evidence. |
| RUSTSEC-2024-0418 | gdk-sys | 0.18.2 | unmaintained | feature-gated-upstream-watch | no | Transitive GTK3 Linux shell dependency through Tauri native-packaging; first formal tag excludes Linux GTK3/WebKitGTK 4.x native artifacts, so this warning is registered but outside the shipped release set and does not grant authority writes. | Keep Linux GTK3 native artifacts excluded until the native shell stack moves to a maintained GTK4/WebKitGTK 6 or equivalent WebView route with refreshed target-host evidence. |
| RUSTSEC-2024-0411 | gdkwayland-sys | 0.18.2 | unmaintained | feature-gated-upstream-watch | no | Transitive GTK3 Linux shell dependency through Tauri native-packaging; first formal tag excludes Linux GTK3/WebKitGTK 4.x native artifacts, so this warning is registered but outside the shipped release set and does not grant authority writes. | Keep Linux GTK3 native artifacts excluded until the native shell stack moves to a maintained GTK4/WebKitGTK 6 or equivalent WebView route with refreshed target-host evidence. |
| RUSTSEC-2024-0415 | gtk | 0.18.2 | unmaintained | feature-gated-upstream-watch | no | Transitive GTK3 Linux shell dependency through Tauri native-packaging; first formal tag excludes Linux GTK3/WebKitGTK 4.x native artifacts, so this warning is registered but outside the shipped release set and does not grant authority writes. | Keep Linux GTK3 native artifacts excluded until the native shell stack moves to a maintained GTK4/WebKitGTK 6 or equivalent WebView route with refreshed target-host evidence. |
| RUSTSEC-2024-0420 | gtk-sys | 0.18.2 | unmaintained | feature-gated-upstream-watch | no | Transitive GTK3 Linux shell dependency through Tauri native-packaging; first formal tag excludes Linux GTK3/WebKitGTK 4.x native artifacts, so this warning is registered but outside the shipped release set and does not grant authority writes. | Keep Linux GTK3 native artifacts excluded until the native shell stack moves to a maintained GTK4/WebKitGTK 6 or equivalent WebView route with refreshed target-host evidence. |
| RUSTSEC-2024-0419 | gtk3-macros | 0.18.2 | unmaintained | feature-gated-upstream-watch | no | Transitive GTK3 Linux shell dependency through Tauri native-packaging; first formal tag excludes Linux GTK3/WebKitGTK 4.x native artifacts, so this warning is registered but outside the shipped release set and does not grant authority writes. | Keep Linux GTK3 native artifacts excluded until the native shell stack moves to a maintained GTK4/WebKitGTK 6 or equivalent WebView route with refreshed target-host evidence. |
| RUSTSEC-2024-0429 | glib | 0.18.5 | unsound | feature-gated-upstream-watch | no | Transitive GTK3/Tauri shell dependency with a patched upstream line, but first formal tag excludes Linux GTK3/WebKitGTK 4.x native artifacts; this is runtime shell risk outside the shipped release set, not core authority risk. | Keep Linux GTK3/glib native artifacts excluded until the native shell stack resolves to a patched maintained glib/WebView route with refreshed target-host evidence. |
| RUSTSEC-2024-0436 | paste | 1.0.15 | unmaintained | upstream-upgrade-watch | no | Transitive through Leptos/Tachys macro stack; build-time macro dependency, not runtime authority or persisted data. | Track Leptos/Tachys upgrade path or upstream macro replacement. |
| RUSTSEC-2026-0173 | proc-macro-error2 | 2.0.1 | unmaintained | upstream-upgrade-watch | no | Transitive through Leptos macro stack and related parser macros; build-time dependency, not runtime authority or persisted data. | Track Leptos macro stack upgrade or upstream migration away from proc-macro-error2. |
| RUSTSEC-2026-0249 | smartstring | 1.0.1 | unmaintained | upstream-upgrade-watch | no | Transitive through the selected Rhai compatibility host; this is an informational maintenance warning, not a reported vulnerability, and the current upstream Rhai release line still selects smartstring. The plugin host remains capability-gated and does not make smartstring an authority store. | Track Rhai's maintained-string migration or replace the compatibility host in a separately reviewed plugin-runtime batch; do not add a direct duplicate string identity. |
| RUSTSEC-2026-0253 | lru | 0.16.4 | unsound | feature-gated-upstream-watch | no | Transitive only through optional Tantivy search; the first-tag default low-spec/runtime artifact set does not enable the `search` feature, so the affected `LruCache::pop()` path is outside shipped runtime capabilities. | Keep Tantivy search feature-gated and disabled in first-tag artifacts; upgrade Tantivy when its stable dependency line resolves to lru >=0.18.2, then rerun search and release audit gates. |
| RUSTSEC-2024-0370 | proc-macro-error | 1.0.4 | unmaintained | feature-gated-upstream-watch | no | Transitive through GTK/glib macro stack behind native-packaging and through GTK3 shell path; not default skeleton runtime. | Keep native-packaging gate closed by default and track Tauri/Wry/GTK stack upgrade. |
| RUSTSEC-2025-0081 | unic-char-property | 0.9.0 | unmaintained | upstream-upgrade-watch | no | Transitive through Tauri urlpattern tooling; shell/build configuration path, not ledger/projection authority. | Track Tauri/urlpattern upgrade or replacement. |
| RUSTSEC-2025-0075 | unic-char-range | 0.9.0 | unmaintained | upstream-upgrade-watch | no | Transitive through Tauri urlpattern tooling; shell/build configuration path, not ledger/projection authority. | Track Tauri/urlpattern upgrade or replacement. |
| RUSTSEC-2025-0080 | unic-common | 0.9.0 | unmaintained | upstream-upgrade-watch | no | Transitive through Tauri urlpattern tooling; shell/build configuration path, not ledger/projection authority. | Track Tauri/urlpattern upgrade or replacement. |
| RUSTSEC-2025-0100 | unic-ucd-ident | 0.9.0 | unmaintained | upstream-upgrade-watch | no | Transitive through Tauri urlpattern tooling; shell/build configuration path, not ledger/projection authority. | Track Tauri/urlpattern upgrade or replacement. |
| RUSTSEC-2025-0098 | unic-ucd-version | 0.9.0 | unmaintained | upstream-upgrade-watch | no | Transitive through Tauri urlpattern tooling; shell/build configuration path, not ledger/projection authority. | Track Tauri/urlpattern upgrade or replacement. |
