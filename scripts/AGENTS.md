<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 | Updated: 2026-04-24 -->

# scripts

## Purpose

Build and lint utility scripts for Deve-Notebook. Provides low-memory lint configurations for resource-constrained environments.

## Key Files

| File | Description |
|------|-------------|
| `plan-coverage.sh` | Plan-code coverage, file-size fuse, i18n leak, and acceptance binding checks |
| `check-local-quick-gate.sh` | Fast local implementation gate: diff hygiene, core/CLI checks, focused governance, and focused tests |
| `check-deep-audit-gate.sh` | Explicit deep audit gate: governance suite, baseline scripts, runtime smokes, and optional full/Docker tests |
| `lint-low-mem.cmd` | Windows CMD script — runs clippy with reduced memory |
| `lint-low-mem.ps1` | PowerShell script — runs clippy with reduced memory |
| `check-architecture-registry.sh` | Verifies operation registry, acceptance refs, drift map, Lisp IDs, and graph spines stay aligned |
| `check-ws-structured-errors.sh` | Verifies WS protocol errors remain structured as `ServerError`/`ServerErrorCode` |
| `check-auth-unauthorized-state.sh` | Verifies auth failures map to Unauthorized instead of reconnect/disconnected UI |
| `check-auth-baseline.sh` | Verifies Auth startup, cookie/JWT/status, WS 401, rate-limit, headers, and frontend session-expired contracts |
| `check-network-baseline.sh` | Verifies NET-001..NET-004 reconnect, `/ws`, node role, and WS frame baseline contracts |
| `check-acceptance-matrix.sh` | Verifies acceptance cases, operation relations, first-tag journeys, and generated matrix drift through the Rust checker |
| `check-foundation-baseline.sh` | Verifies foundation terminology, source-of-truth wording, positioning boundaries, init, watcher, rename, and `.deveignore` acceptance bindings |
| `check-search-baseline.sh` | Verifies current Search scope, feature-gate, stale-result, and future-index boundaries |
| `check-rendering-baseline.sh` | Verifies Markdown rendering current/future split, lightweight renderer subset, and controlled apply boundaries |
| `check-ui-token-baseline.sh` | Verifies style color literals stay confined to design-token files |
| `check-ui-z-index-baseline.sh` | Verifies shell z-index registry tokens and prevents private numeric z-levels |
| `check-ui-focus-baseline.sh` | Verifies modal focus trap and restore bindings for shared dialog surfaces |
| `check-ui-spa-routing-baseline.sh` | Verifies SPA route fallback stays 200 while API/WS paths do not fall back to index |
| `check-ui-disconnect-baseline.sh` | Verifies disconnect lockdown overlay copy and edit-disabled bindings |
| `check-ui-dashboard-refresh-baseline.sh` | Verifies Dashboard SystemMetrics refresh and WS-backed sync status bindings |
| `check-ui-desktop-baseline.sh` | Verifies Desktop canonical column markers, split diff scroll sync, layout resize persistence, and Unified Search mode routing bindings |
| `check-cli-settings-baseline.sh` | Verifies CLI command surface, `config.toml` settings mutation, and shortcut entry contracts |
| `check-settings-local-feedback-baseline.sh` | Verifies SET-003..007 local config persistence, effective feedback, reserved UI feedback, and future Settings API boundary |
| `check-browser-prefs-boundary.sh` | Verifies harmless Web UI prefs are the only functional localStorage users and go through the fallback layer |
| `check-diff-color-baseline.sh` | Verifies diff gutter colors use canonical source-control semantic tokens |
| `check-large-doc-baseline.sh` | Verifies large-document snapshot-first, batch replay, and search gate contracts |
| `check-ai-baseline.sh` | Verifies Native AI slash modes, planned palette command boundaries, and trusted-cli default-off gates |
| `check-source-control-baseline.sh` | Verifies Source Control panel commit/publish boundaries, diff first-viewport rendering, and planned Git palette commands |
| `check-source-control-smoke-hygiene.sh` | Verifies Source Control smoke tests use read-only `sc-status` and do not assume Git-clean app state |
| `check-dev-data-health-baseline.sh` | Verifies projection health diagnostics expose repair hints and fail-closed authority corruption boundaries |
| `check-native-track-boundary.sh` | Verifies Desktop/Mobile native adapter boundaries remain future-safe and do not redefine core authority |
| `check-native-packaging-gate.sh` | Verifies Desktop Tauri dependencies stay isolated behind `apps/desktop/native-packaging` while default and Mobile builds remain no-Tauri |
| `check-native-process-adapter-gate.sh` | Verifies child-process runtime remains gate-closed and process observations stay state-machine-only |
| `check-native-target-host-evidence.sh` | Verifies Desktop/Mobile target-host evidence reports include host, artifact, install/startup, and no-process/no-authority fields |
| `collect-native-target-host-evidence.sh` | Downloads Native Target Host workflow evidence artifacts through GitHub CLI or token-backed API fallback and validates each report |
| `dispatch-native-target-host-workflow.sh` | Builds or explicitly dispatches the manual Native Target Host GitHub Actions workflow through GitHub CLI or token-backed API fallback |
| `install-native-target-host-tools.sh` | Installs pinned Trunk and Tauri CLI release binaries for manual target-host workflows without compiling those tools from source |
| `build-web-dist-ci.sh` | Builds Web assets in native target-host CI with explicit npm/trunk command diagnostics |
| `write-native-target-host-evidence.sh` | Writes validated Desktop/Mobile target-host evidence reports for manual workflow artifacts or local target-host runs |
| `check-desktop-package-preflight.sh` | Verifies Desktop default/no-packaging and native-packaging compile surfaces before target-host package builds |
| `check-desktop-linux-apptainer-slurm.sh` | Runs checksum-bound, node-local Linux Tauri package/startup/native-session gates in one pinned-SIF Apptainer session under Slurm |
| `check-desktop-platform-package-build.sh` | Diagnoses target-host Desktop package build prerequisites and only runs `cargo tauri build` when explicitly required |
| `check-desktop-package-startup-smoke.sh` | Verifies target-host Desktop package artifacts expose a startup-probeable shell binary without opening process runtime or authority writes |
| `check-desktop-native-session-package-smoke.sh` | Starts the bundled `deve_cli` with smoke state bound to a temporary `DEVE_DESKTOP_DATA_DIR`, verifies native-session cookie handoff, and requires cleanup before success |
| `check-desktop-installer-smoke.sh` | Verifies target-host Desktop install/startup/real-WebView/NoteGit-Git/uninstall flow while keeping all writes behind the installed sidecar authority path |
| `check-desktop-packaged-ui-smoke.ps1` | Starts an installed Windows Desktop package with isolated data/WebView2 roots and verifies real UI plus sidecar cleanup through a random CDP endpoint |
| `smoke-desktop-packaged-ui.mjs` | Drives create/edit/commit/history and Settings focus trapping inside the installed native WebView without direct authority calls |
| `check-desktop-remote-browser-smoke.ps1` | Runs an installed preference-driven RemoteBrowser, proves zero Web IPC/CSP leakage, invokes native local recovery, and verifies fresh LocalBackend restart plus sidecar cleanup |
| `smoke-desktop-remote-browser.mjs` | Drives remote login/edit/commit/history through WebView2 CDP and records the no-facade/no-`ipc.localhost` browser contract |
| `lib/desktop-webview-business-flow.mjs` | Shared UI-only create/edit/commit/history flow used by packaged LocalBackend and RemoteBrowser WebView smokes |
| `check-desktop-target-host-preflight.sh` | Diagnoses macOS/Windows Desktop target-host prerequisites without claiming package readiness on the wrong host |
| `check-mobile-platform-package-preflight.sh` | Diagnoses Android/iOS target-host prerequisites while keeping Mobile package build/project generation closed |
| `check-mobile-android-shell-package-build.sh` | Runs the Android WebView shell package gate only when explicitly required on an Android-capable target host |
| `check-mobile-android-emulator-install-startup-smoke.sh` | Owns an Android emulator target lifecycle, builds a debug WebView shell APK, and selects an explicit LocalBackend or RemoteBrowser writable journey before bounded cleanup |
| `android-emulator-cleanup.test.sh` | Exercises emulator cleanup ownership rejection and the verified `emu kill` shutdown transition without touching ambient ADB targets |
| `cleanup-mobile-android-emulator.sh` | Uses the runner-scoped serial plus exact AVD identity to request Android emulator shutdown; recorded PID is observation-only outside the owning shell, and cleanup waits for disappearance before evidence publication |
| `lib/android-emulator-owner.sh` | Canonicalizes the runner-owned emulator state path and rejects ambient owner-file overrides that escape the current execution directory |
| `mobile-android-emulator-journey.test.mjs` | Verifies the shared emulator owner keeps local/remote journey selection and claims outputs explicit |
| `smoke-mobile-android-lifecycle.sh` | Drives the debug Android WebView through CDP, verifies non-zero pending preservation, transport-generation recovery, foreground reprobe, resumed commit, and bounded graceful runtime cleanup |
| `smoke-mobile-android-lifecycle.mjs` | Raw page-target CDP harness for the Android LocalBackend lifecycle smoke; requires WebCrypto Ed25519, uses debug-only lifecycle fault/exit commands, and otherwise submits UI intents only |
| `smoke-mobile-android-remote-browser.sh` / `.mjs` | Runs a preference-driven Android RemoteBrowser against an HTTPS Deve origin, proves zero native facade/IPC, real login/edit/commit/history/background recovery, invokes the platform-owned local recovery control, verifies a fresh LocalBackend endpoint/session/scope, and emits typed writable claims |
| `lib/android-business-flow.mjs` | Shared Android WebView UI-intent helpers for login, document edit, and Source Control commit/history; it owns no authority and is reused by LocalBackend and RemoteBrowser smokes |
| `inspect-android-target-capability.mjs` | Records Android SDK, current WebView provider/version, AVD/device identity, and enforces the API 29+/WebView 137+ writable-evidence floor without replacing the real Ed25519 probe |
| `android-target-capability.test.mjs` | Regression tests for Android target-fact parsing, support qualification, and writable-versus-read-only evidence modes |
| `lib/mobile-webview-interaction.mjs` | Focus, text-input failure diagnostics, and mobile drawer navigation helpers for the Android WebView lifecycle harness |
| `lib/mobile-source-control-interaction.mjs` | Source Control open, confirmed-row commit acknowledgement, history proof, and failure diagnostics for the Android WebView lifecycle harness |
| `lib/android-webview-cdp.mjs` | Raw Android WebView page-target discovery, CDP request routing, evaluation, and bounded reconnect helpers for native lifecycle smoke |
| `lib/websocket-delivery-gate.mjs` | CDP-installed, smoke-only outbound WebSocket gate; discards an old-generation edit frame so only product pending replay can deliver it after Android transport replacement |
| `lib/webcrypto-capability.mjs` | Shared target-host probe for non-extractable WebCrypto Ed25519 capability; returns stable fail-closed blocker facts only |
| `webcrypto-capability.test.mjs` | Verifies the Android/WebView probe requests non-extractable Ed25519 signing keys and rejects unsupported or extractable results |
| `check-mobile-ios-shell-package-build.sh` | Runs the iOS WebView shell package gate only when explicitly required on a macOS target host |
| `check-graph-baseline.sh` | Verifies Graph remains a read-only derived projection and does not become a ledger/workspace authority path |
| `check-mobile-baseline.sh` | Verifies Mobile Web shell viewport mapping, drawer gestures, resize-handle exclusion, keyboard toolbar, search top sheet/results scrolling, bottom bar, and editor font-size baseline contracts |
| `check-dev-runbook-baseline.sh` | Verifies current startup, auth, frontend, Chrome MCP, search, and verification runbook boundaries |
| `check-feature-operation-paths.sh` | Verifies feature operation and acceptance docs do not point at removed source/script/doc paths |
| `check-i18n-formatting-baseline.sh` | Verifies visible frontend time formatting goes through the locale-aware formatting utility |
| `check-reliability-observability-baseline.sh` | Verifies reliability/observability governance covers SLO/SLI, telemetry schema, metrics taxonomy, tracing, health mapping, alert tier, and DR index |
| `check-release-baseline.sh` | Verifies Docker, compose, and release workflow surfaces match the embedded-frontend release baseline |
| `check-release-version-match.sh` | Exact-matches a release tag with workspace, Desktop Tauri, and Mobile Tauri versions, including prerelease/build metadata |
| `validate-release-image-tags.sh` | Rejects incomplete, duplicate, cross-repository, or wrong-version Docker tag sets before any release push |
| `smoke-web-release-build.sh` | Builds the Web release assets with normalized Trunk/Browserslist environment |
| `smoke-web-runtime-paths.sh` | Prints the repeatable CMD-007A/CMD-007B browser runtime smoke command sequence |
| `smoke-runtime-happy-path.sh` | Runs temporary-repo Axum/WebSocket happy-path tests for switch, handshake, writer, edit, open, history, and reconnect bootstrap |
| `smoke-runtime-recovery-path.sh` | Runs degraded-local, stale-scope, reconnect gate, status, and auth-probe recovery smoke tests |
| `smoke-docker-release.sh` | Builds and runs the Docker release image smoke, or verifies an explicitly supplied existing candidate image without rebuilding |
| `smoke-docker-multiclient.sh` | Builds or reuses one Docker candidate image, then drives isolated Playwright clients; the required product tier also proves repo lifecycle, typed diff, Source Control, and External Changes |
| `lib/docker-multiclient-product-journeys.mjs` | UI-only Docker browser journey helper plus a narrow `docker exec` projection mutation used to prove External Changes does not bypass ledger authority |
| `smoke-runtime-release-info.sh` | Checks a running server's `/api/node/role` runtime release info fields |
| `lib/android-tools.sh` | Shared Android SDK / Android Studio JBR discovery helpers for local and target-host Android gates |

## For AI Agents

### Working In This Directory

- Scripts target Windows (CMD/PowerShell) since primary dev environment is WSL on Windows.
- Keep scripts minimal and focused on a single task.

<!-- MANUAL: -->
