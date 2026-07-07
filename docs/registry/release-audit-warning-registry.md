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
- `Decision` must be one of:
  - `direct-migration-before-stable`: first stable/tag readiness needs a direct
    project decision or codec/dependency replacement before data format freeze.
  - `feature-gated-upstream-watch`: warning is behind an optional feature or
    platform shell dependency; keep the feature gate and track upstream.
  - `upstream-upgrade-watch`: warning is transitive through selected upstream
    framework/tooling; replacement is owned by an upstream upgrade batch.
- `Tag blocker` values:
  - `yes`: before the first formal public tag, the USER must approve the
    retained risk or the dependency must be replaced.
  - `no`: the row is acceptable for a public tag as long as the rationale,
    feature gate, and upstream route remain true.
- Rationale and replacement route must be concrete. `TODO`, `TBD`, and empty
  cells are invalid.

## Warning Registry

| Advisory | Crate | Version | Kind | Decision | Tag blocker | Rationale | Replacement route |
|---|---|---|---|---|---|---|---|
| RUSTSEC-2025-0141 | bincode | 1.3.3 | unmaintained | direct-migration-before-stable | yes | Direct codec dependency used by ledger entry envelopes, repo metadata, protocol frames, and backup plaintext tests; it cannot be silently frozen as the first stable data baseline. | Before first formal tag, choose and implement a maintained codec migration or explicitly freeze bincode v1 with documented version, migration, reset, and repair strategy. |
| RUSTSEC-2024-0413 | atk | 0.18.2 | unmaintained | feature-gated-upstream-watch | yes | Transitive GTK3 Linux shell dependency through Tauri native-packaging; first release includes formal native targets, so retaining this stack needs an explicit first-tag decision even though it does not grant authority writes. | Before first formal tag, either exclude Linux GTK3 native artifacts, upgrade to a maintained Tauri/Wry GTK4 or WebView route, or get USER approval to retain the gated GTK3 shell risk. |
| RUSTSEC-2024-0416 | atk-sys | 0.18.2 | unmaintained | feature-gated-upstream-watch | yes | Transitive GTK3 Linux shell dependency through Tauri native-packaging; first release includes formal native targets, so retaining this stack needs an explicit first-tag decision even though it does not grant authority writes. | Before first formal tag, either exclude Linux GTK3 native artifacts, upgrade to a maintained Tauri/Wry GTK4 or WebView route, or get USER approval to retain the gated GTK3 shell risk. |
| RUSTSEC-2024-0412 | gdk | 0.18.2 | unmaintained | feature-gated-upstream-watch | yes | Transitive GTK3 Linux shell dependency through Tauri native-packaging; first release includes formal native targets, so retaining this stack needs an explicit first-tag decision even though it does not grant authority writes. | Before first formal tag, either exclude Linux GTK3 native artifacts, upgrade to a maintained Tauri/Wry GTK4 or WebView route, or get USER approval to retain the gated GTK3 shell risk. |
| RUSTSEC-2024-0418 | gdk-sys | 0.18.2 | unmaintained | feature-gated-upstream-watch | yes | Transitive GTK3 Linux shell dependency through Tauri native-packaging; first release includes formal native targets, so retaining this stack needs an explicit first-tag decision even though it does not grant authority writes. | Before first formal tag, either exclude Linux GTK3 native artifacts, upgrade to a maintained Tauri/Wry GTK4 or WebView route, or get USER approval to retain the gated GTK3 shell risk. |
| RUSTSEC-2024-0411 | gdkwayland-sys | 0.18.2 | unmaintained | feature-gated-upstream-watch | yes | Transitive GTK3 Linux shell dependency through Tauri native-packaging; first release includes formal native targets, so retaining this stack needs an explicit first-tag decision even though it does not grant authority writes. | Before first formal tag, either exclude Linux GTK3 native artifacts, upgrade to a maintained Tauri/Wry GTK4 or WebView route, or get USER approval to retain the gated GTK3 shell risk. |
| RUSTSEC-2024-0415 | gtk | 0.18.2 | unmaintained | feature-gated-upstream-watch | yes | Transitive GTK3 Linux shell dependency through Tauri native-packaging; first release includes formal native targets, so retaining this stack needs an explicit first-tag decision even though it does not grant authority writes. | Before first formal tag, either exclude Linux GTK3 native artifacts, upgrade to a maintained Tauri/Wry GTK4 or WebView route, or get USER approval to retain the gated GTK3 shell risk. |
| RUSTSEC-2024-0420 | gtk-sys | 0.18.2 | unmaintained | feature-gated-upstream-watch | yes | Transitive GTK3 Linux shell dependency through Tauri native-packaging; first release includes formal native targets, so retaining this stack needs an explicit first-tag decision even though it does not grant authority writes. | Before first formal tag, either exclude Linux GTK3 native artifacts, upgrade to a maintained Tauri/Wry GTK4 or WebView route, or get USER approval to retain the gated GTK3 shell risk. |
| RUSTSEC-2024-0419 | gtk3-macros | 0.18.2 | unmaintained | feature-gated-upstream-watch | yes | Transitive GTK3 Linux shell dependency through Tauri native-packaging; first release includes formal native targets, so retaining this stack needs an explicit first-tag decision even though it does not grant authority writes. | Before first formal tag, either exclude Linux GTK3 native artifacts, upgrade to a maintained Tauri/Wry GTK4 or WebView route, or get USER approval to retain the gated GTK3 shell risk. |
| RUSTSEC-2024-0429 | glib | 0.18.5 | unsound | feature-gated-upstream-watch | yes | Transitive GTK3/Tauri shell dependency with a patched upstream line, but current Tauri Linux stack still resolves 0.18; this is runtime shell risk, not core authority risk. | Before first formal tag, either upgrade the native shell stack to a patched glib line through Tauri/Wry or get USER approval to retain the gated Linux shell risk. |
| RUSTSEC-2024-0384 | instant | 0.1.13 | unmaintained | feature-gated-upstream-watch | no | Transitive through Tantivy search, and search remains optional/feature-gated for low-spec default. | Track Tantivy upgrade or search backend replacement; keep search optional and outside authority path. |
| RUSTSEC-2026-0002 | lru | 0.12.5 | unsound | feature-gated-upstream-watch | yes | Transitive through Tantivy search; the affected mutable iterator is not part of Deve authority logic, but full-feature builds include the crate. | Before first formal tag, upgrade Tantivy to a line using patched lru, replace the search backend, or get USER approval to retain the gated search risk. |
| RUSTSEC-2024-0436 | paste | 1.0.15 | unmaintained | upstream-upgrade-watch | no | Transitive through Leptos/Tachys macro stack; build-time macro dependency, not runtime authority or persisted data. | Track Leptos/Tachys upgrade path or upstream macro replacement. |
| RUSTSEC-2026-0173 | proc-macro-error2 | 2.0.1 | unmaintained | upstream-upgrade-watch | no | Transitive through Leptos macro stack and related parser macros; build-time dependency, not runtime authority or persisted data. | Track Leptos macro stack upgrade or upstream migration away from proc-macro-error2. |
| RUSTSEC-2024-0370 | proc-macro-error | 1.0.4 | unmaintained | feature-gated-upstream-watch | no | Transitive through GTK/glib macro stack behind native-packaging and through GTK3 shell path; not default skeleton runtime. | Keep native-packaging gate closed by default and track Tauri/Wry/GTK stack upgrade. |
| RUSTSEC-2025-0081 | unic-char-property | 0.9.0 | unmaintained | upstream-upgrade-watch | no | Transitive through Tauri urlpattern tooling; shell/build configuration path, not ledger/projection authority. | Track Tauri/urlpattern upgrade or replacement. |
| RUSTSEC-2025-0075 | unic-char-range | 0.9.0 | unmaintained | upstream-upgrade-watch | no | Transitive through Tauri urlpattern tooling; shell/build configuration path, not ledger/projection authority. | Track Tauri/urlpattern upgrade or replacement. |
| RUSTSEC-2025-0080 | unic-common | 0.9.0 | unmaintained | upstream-upgrade-watch | no | Transitive through Tauri urlpattern tooling; shell/build configuration path, not ledger/projection authority. | Track Tauri/urlpattern upgrade or replacement. |
| RUSTSEC-2025-0100 | unic-ucd-ident | 0.9.0 | unmaintained | upstream-upgrade-watch | no | Transitive through Tauri urlpattern tooling; shell/build configuration path, not ledger/projection authority. | Track Tauri/urlpattern upgrade or replacement. |
| RUSTSEC-2025-0098 | unic-ucd-version | 0.9.0 | unmaintained | upstream-upgrade-watch | no | Transitive through Tauri urlpattern tooling; shell/build configuration path, not ledger/projection authority. | Track Tauri/urlpattern upgrade or replacement. |
| RUSTSEC-2024-0320 | yaml-rust | 0.4.5 | unmaintained | upstream-upgrade-watch | no | Transitive through config 0.13; config parsing is startup/settings input, not authority serialization. | Track config crate upgrade or replace config parsing with maintained TOML/env loader. |
