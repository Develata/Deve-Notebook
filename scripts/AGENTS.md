<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 | Updated: 2026-08-02 -->

# scripts

## Purpose

Build and lint utility scripts for Deve-Notebook. Provides low-memory lint configurations for resource-constrained environments.

## Key Files

| File | Description |
|------|-------------|
| `plan-coverage.sh` | Plan-code coverage, file-size fuse, i18n leak, and acceptance binding checks |
| `plan-ref-exemptions.tsv` | Exact-path, typed, reasoned exemptions for test/generated/repo-local infra Rust modules |
| `plan-coverage-selftest.sh` | Positive/negative fixtures for plan reference, exemption, and governance scanner rules |
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
| `dispatch-native-target-host-workflow.test.sh` | Verifies RemoteBrowser dispatch fields, platform dependencies, API fallback parity, and absence of password dispatch inputs |
| `remote-browser-fixture.{sh,ps1}` | Owns exact-HEAD loopback backend, random credentials, pinned HTTPS tunnel, and fail-closed cleanup for RemoteBrowser target-host evidence; Unix final cleanup shares the zombie-aware active-process classifier used by bounded termination |
| `remote-browser-fixture-bounded-start.ps1` | Runs the Windows fixture start worker under one total startup deadline and a worker output cap, relays allowlisted stage progress, and on timeout terminates the worker tree and recovers owned resources/secrets from the atomic startup state |
| `lib/remote-browser-fixture-cloudflared.ps1` | Owns the checksum-pinned Windows cloudflared download/install path used by the RemoteBrowser fixture |
| `lib/remote-browser-fixture-state.ps1` | Atomically publishes ready/recovery state and preflights source/resource/live-owner identity before any Windows fixture secret cleanup |
| `lib/remote-browser-fixture-http.sh` | Owns bounded Linux/Android loopback readiness, quick-tunnel origin discovery, exact public role probes, propagation timing, and allowlisted HTTP diagnostics |
| `lib/remote-browser-fixture-start-supervisor.sh` | Owns signal-ready Unix startup supervision and cancellation cleanup |
| `lib/desktop-remote-browser-native-recovery.ps1` | Owns Win32 native-menu discovery/dispatch and replacement-process proof for the Desktop RemoteBrowser-to-LocalBackend recovery journey |
| `lib/remote-browser-fixture-progress.ps1` | Atomic startup-state serialization plus the fixed nonsecret stage-name allowlist shared by the Windows fixture and its bounded-start watchdog |
| `remote-browser-fixture.test.{sh,ps1}` | Exercises fixture input, atomic final state, dual-failure preservation, secret cleanup, PID/owner identity, zombie-aware final cleanup, and platform lifecycle invariants |
| `remote-browser-fixture-http.test.sh` | Exercises bounded quick-tunnel readiness, exact role endpoint, process identity, deadline, and redacted failure diagnostics |
| `remote-browser-fixture-start-supervisor.test.sh` | Exercises pending-signal handoff and successful-publication rollback at cancellation boundaries |
| `remote-browser-fixture-bounded-start.test.ps1` | Proves single env-path success output, a pipe-captured success return while fixture survivors keep running (std-handle inheritance hygiene), deadline tree termination, stage-named errors (including zero-secret worker failures), PID-token/container-owner refusal, partial/corrupted state fail-closed, fast-exit combined output limits, bounded redacted failure output, and a real-worker integration failure path |
| `lib/remote-browser-fixture-json.sh` | Serializes the fixed private fixture state/environment JSON schemas without owning lifecycle decisions |
| `desktop-install-root.test.ps1` | Executes real Windows install-root, prefix-escape, and junction containment regressions |
| `lib/desktop-install-root.ps1` | Canonical Win32 install-root validator shared by the packaged Desktop RemoteBrowser smoke and tests |
| `check-release-candidate-bundle.sh` | Recomputes the Rust-owned sealed candidate contract, re-extracts the APK signer, and verifies fixed provenance/SPDX bundles against exact workflow/HEAD |
| `check-android-apk-signer{,.test}.sh` | Verifies a sealed APK has exactly one expected SHA-256 signer without trusting a companion manifest value |
| `check-release-tag-binding.sh` | Revalidates the remote annotated tag object and its directly peeled candidate commit before public mutations |
| `probe-release-remote.sh` | Distinguishes GitHub Release and GHCR present/explicit-404-absent/error states before promotion |
| `install-native-target-host-tools.sh` | Installs pinned Trunk and Tauri CLI release binaries for manual target-host workflows without compiling those tools from source |
| `build-web-dist-ci.sh` | Builds Web assets in native target-host CI with explicit npm/trunk command diagnostics |
| `write-native-target-host-evidence.sh` | Writes validated Desktop/Mobile target-host evidence reports for manual workflow artifacts or local target-host runs |
| `check-desktop-package-preflight.sh` | Verifies Desktop default/no-packaging and native-packaging compile surfaces before target-host package builds |
| `check-desktop-linux-apptainer-slurm.sh` | Runs checksum-bound, node-local Linux Tauri package/startup/native-session gates in one pinned-SIF Apptainer session under Slurm |
| `check-desktop-platform-package-build.sh` | Diagnoses target-host Desktop package build prerequisites and only runs `cargo tauri build` when explicitly required |
| `check-desktop-package-startup-smoke.sh` | Verifies target-host Desktop package artifacts expose a startup-probeable shell binary without opening process runtime or authority writes |
| `check-desktop-native-session-package-smoke.sh` | Starts the bundled `deve_cli` with smoke state bound to a temporary `DEVE_DESKTOP_DATA_DIR`, verifies native-session cookie handoff, and requires cleanup before success |
| `check-desktop-installer-smoke.sh` | Verifies target-host Desktop install/startup/real-WebView/NoteGit-Git/uninstall flow while keeping all writes behind the installed sidecar authority path |
| `desktop-installer-windows.test.sh` | Injects Windows registry restore failure/success paths and verifies failure preservation plus cleanup-state transitions |
| `lib/desktop-installer-windows.sh` | Owns Windows MSI/NSIS install, packaged-app journey, registry isolation, and uninstall helpers for the Desktop installer producer |
| `check-desktop-packaged-ui-smoke.ps1` | Starts an installed Windows Desktop package with isolated data/WebView2 roots and verifies real UI plus sidecar cleanup through the exact programmatic WebView2-assigned CDP marker |
| `smoke-desktop-packaged-ui.mjs` | Drives create/edit/commit/history and Settings focus trapping inside the installed native WebView without direct authority calls |
| `check-desktop-remote-browser-smoke.ps1` | Runs an installed preference-driven RemoteBrowser, proves zero Web IPC/CSP leakage, invokes native local recovery, and verifies fresh LocalBackend restart plus sidecar cleanup |
| `lib/webview2-cdp.ps1` | Discovers programmatically enabled WebView2-assigned CDP endpoints from isolated `DevToolsActivePort` state, owns bounded Node/process-tree and exact-profile cleanup, detects host exit, and writes sanitized diagnostics shared by Desktop smokes |
| `lib/windows-process-cleanup.ps1` | Owns bounded Node journey execution, creation-time-bound process-tree snapshots, taskkill deadlines, direct-child fallback, and residual identity checks |
| `smoke-desktop-remote-browser.mjs` | Drives remote login/edit/commit/history through WebView2 CDP and records the no-facade/no-`ipc.localhost` browser contract |
| `lib/desktop-webview-business-flow.mjs` | Shared UI-only create/edit/commit/history flow used by packaged LocalBackend and RemoteBrowser WebView smokes |
| `check-desktop-target-host-preflight.sh` | Diagnoses macOS/Windows Desktop target-host prerequisites without claiming package readiness on the wrong host |
| `check-mobile-platform-package-preflight.sh` | Diagnoses Android/iOS target-host prerequisites while keeping Mobile package build/project generation closed |
| `check-mobile-android-shell-package-build.sh` | Runs the Android WebView shell package gate only when explicitly required on an Android-capable target host |
| `check-mobile-android-emulator-install-startup-smoke.sh` | Owns an Android emulator target lifecycle, starts a minified release-variant APK, then delegates the debug APK's only install/start to one explicit LocalBackend or RemoteBrowser writable journey before bounded cleanup |
| `lib/android-emulator-targeted-preflight.sh` | Owns targeted-emulator host input validation and the read-only local SDK package reuse contract, leaving emulator lifecycle and product journeys in the orchestrator |
| `sign-mobile-android-target-host-release-apk.sh` | Applies an ephemeral diagnostic signer to the exact minified release-variant APK for emulator startup proof without consuming or impersonating release signing identity |
| `prepare-android-emulator-host.sh` | Installs the explicit Ubuntu emulator runtime prerequisite and configures KVM access shared by Android target-host workflows |
| `lib/android-emulator-pin.sh` / `android-emulator-pin.test.sh` | Resolves the checksum-pinned emulator binary, serializes same-build cache publication, and proves its canonical version/build banner under independent time/output bounds |
| `lib/android-emulator-renderer.sh` / `android-emulator-renderer.test.sh` | Parses every renderer selection in a bounded owned-emulator log prefix and fail-closes missing, conflicting, unapproved, or legacy evidence |
| `lib/android-guest-service-readiness.sh` / `android-guest-service-readiness.test.sh` | Owns canonical package/settings response admission, the shared continuous stable window, exact transient classifiers, final process-guard check, absolute deadline, and fail-closed regression matrix |
| `lib/android-emulator-boot-readiness.sh` / `android-emulator-boot-readiness.test.sh` | Owns bounded ADB device-state, AVD identity, and boot-property readiness before delegating guest-service stability to the shared admission boundary |
| `lib/android-emulator-diagnostics.sh` | Bounded target-emulator diagnostics extracted from the lifecycle orchestrator so failure reporting remains cohesive |
| `lib/android-install-retry.sh` | Bounded APK install recovery shared by every Android install host: retries only exact package/settings bootstrap-race signatures under one absolute deadline with timeout kill-grace reservation, re-enters shared stable guest-service admission before another install attempt, waits for launcher readiness, and stays fail-closed for timeouts and mixed failures |
| `diagnose-android-emulator-admission.sh`; `lib/android-admission-emulator-lifecycle.sh` | Manual exact-HEAD Android admission matrix worker and its reserved/launched direct-child cleanup boundary; reuses shared readiness/install/cleanup infrastructure and emits bounded per-variant diagnostic JSON rather than receipts |
| `android-emulator-admission-result.test.sh`; `android-emulator-admission-summary.mjs` / `.test.mjs` | Verifies atomic diagnostic result/classification, process cleanup and log budgets, validates the complete identity-bound matrix, writes the Actions summary, and recommends the least-divergent fully stable variant without changing the release gate |
| `lib/android-startup-diagnostics.sh` | Bounded, app-specific startup-process diagnostics (exit-info, crash/runtime logcat, process state) collected only after a missing/replaced identity or failed readiness probe; never replaces the primary startup failure |
| `lib/android-app-process-readiness.sh`; `android-app-process-readiness.test.sh` | Shared LocalBackend/RemoteBrowser PID probe and identity state machine plus mocked regression coverage; anchors the first canonical PID, tolerates one bookkeeping gap before and after admission, and fails closed on replacement, continued absence, transport/probe failure, or deadline expiry |
| `lib/android-app-process-observation.mjs`; `android-app-process-observation.test.mjs` | CDP-journey-side continuation of the admitted Android app PID contract plus isolated mocked coverage; tolerates one `pidof` bookkeeping gap, rejects probe failure/replacement/continued absence, and requires two consecutive missing samples before graceful-exit proof |
| `lib/android-logcat-observation.mjs`; `android-logcat-observation.test.mjs` | Streams bounded Android logcat snapshots for RemoteBrowser pre/post-recovery marker proof without buffering the full device log; timeout, process error, oversized lines, and total output overflow fail closed |
| `android-startup-diagnostics.test.sh` | Mocked tests proving diagnostics stay time/output bounded, keep unsupported exit-info nonfatal, and never mask the primary process failure or print secret-like values |
| `android-emulator-cleanup.test.sh` | Exercises emulator cleanup ownership rejection and the verified `emu kill` shutdown transition without touching ambient ADB targets |
| `cleanup-mobile-android-emulator.sh` | Uses the runner-scoped serial plus exact AVD identity to request Android emulator shutdown; recorded PID is observation-only outside the owning shell, and cleanup waits for disappearance before evidence publication |
| `lib/android-emulator-owner.sh` | Canonicalizes the runner-owned emulator state path and rejects ambient owner-file overrides that escape the current execution directory |
| `mobile-android-emulator-journey.test.mjs` | Verifies the shared emulator owner keeps local/remote journey selection and claims outputs explicit |
| `smoke-mobile-android-lifecycle.sh` | Drives the debug Android WebView through CDP, verifies non-zero pending preservation, transport-generation recovery, foreground reprobe, resumed commit, and bounded graceful runtime cleanup; the WebView-socket wait fails fast on app exit/restart and reports bounded socket-inventory/process/logcat diagnostics |
| `lib/android-ime-test-session.sh`; `android-ime-test-session.test.sh` | Owns reversible physical-test IME selection, enters restore-required before the ambiguous device mutation, exact-verifies bounded prior-IME recovery, and tests lost-response plus restore-failure paths with mocked ADB |
| `lib/android-package-session.sh`; `android-package-session.test.sh` | Keeps formal Android cleanup on uninstall while providing an explicit physical-device overlay-update mode that requires an existing exact package, clears test data without uninstalling, and preserves primary failure precedence |
| `lib/android-lifecycle-harness.mjs`; `android-lifecycle-harness.test.mjs` | Owns bounded Android lifecycle/Back target-host observation, including fail-closed AOSP and Android 15/OEM resumed-Activity classification, same-PID background/reentry, per-sample read budgets, and fresh native rebind proof |
| `lib/android-business-flow-removal-fixture.mjs` | Owns the isolated desktop/mobile DOM state used to execute Android last-repo removal, repo-switcher, and drawer-settlement regressions without inflating the business-flow test module |
| `smoke-mobile-android-lifecycle.mjs` | Raw page-target CDP harness for the Android LocalBackend lifecycle smoke; requires WebCrypto Ed25519, uses debug-only lifecycle fault/exit commands, and otherwise submits UI intents only |
| `smoke-mobile-android-remote-browser.sh` / `.mjs` / `.test.mjs` | Runs a preference-driven Android RemoteBrowser against an HTTPS Deve origin, binds reload/login/ready to the exact origin and a new document loader, proves zero native facade/IPC, real login/edit/commit/history/background recovery, invokes the platform-owned local recovery control, verifies a fresh LocalBackend endpoint/session followed by zero-repo BootstrapUnbound and an ordinary UI first-repo Create into a non-zero scope, and emits typed writable claims |
| `lib/android-business-flow.mjs`; `android-business-flow.test.mjs` | Shared Android WebView UI-intent helpers and regressions for stable repo writer admission, document edit, and Source Control commit/history; they own no authority and are reused by LocalBackend and RemoteBrowser smokes |
| `lib/android-remote-auth-flow.mjs` | Isolates exact-origin RemoteBrowser login projection and ready admission from local repo/document business-flow orchestration |
| `lib/android-document-create-flow.mjs`; `lib/android-document-create-touch.mjs`; `lib/android-document-create-observation.mjs`; `lib/android-document-create-pointer-fixture.mjs`; `lib/android-document-search-admission.mjs`; `android-document-create-flow.test.mjs`; `android-document-create-pointer.test.mjs`; `android-document-create-settlement.test.mjs`; `android-document-create-observation.test.mjs` | Separates path-bound native-touch identity, single-use click observation/settlement with bounded late-click/scroll diagnostic evidence on the timeout path, reusable pointer test fixture, and document-flow orchestration; reopens mobile Explorer and waits for a stable writer action before exact-path lookup, then carries the unique backend-projected OpenDoc `doc_id` into exact writable-editor admission without retrying Create |
| `lib/android-webview-pointer.mjs`; `android-webview-pointer.test.mjs` | Owns and tests complete CDP mouse and native-touch gestures plus their pre-contact canonical point replacement hooks; document Create uses native touch while established editor/Source Control journeys retain their proven pointer path |
| `lib/android-drawer-touch-proof.mjs`; `android-drawer-touch-proof.test.mjs` | Owns and tests stable current-WebView input-focus admission, action-safe Drawer swipe hit testing, bounded touch-delivery classification, and probe cleanup without owning Drawer transition orchestration |
| `inspect-android-target-capability.mjs` | Records Android SDK, current WebView provider/version, AVD/device identity, and enforces the API 29+/WebView 137+ writable-evidence floor without replacing the real Ed25519 probe |
| `android-target-capability.test.mjs` | Regression tests for Android target-fact parsing, support qualification, and writable-versus-read-only evidence modes |
| `lib/mobile-webview-interaction.mjs` | Focus, text-input failure diagnostics, and mobile drawer navigation helpers for the Android WebView lifecycle harness |
| `lib/mobile-editor-session-observation.mjs` | Exact visible editor/session observation bound to doc, repo/scope, native presentation generation, separately comparable selection positions, and visual viewport geometry without reading Markdown content |
| `mobile-editor-session-observation.test.mjs` | Executes exact session identity and non-zero visual viewport offset clipping regressions independently of editor input sequencing |
| `lib/mobile-keyboard-presentation.mjs` | Proves same-breakpoint Android keyboard presentation through either the primary visual viewport path or the current-generation native IME inset fallback without replacing the editor load session |
| `mobile-keyboard-presentation.test.mjs` | Executes primary/fallback/timeout and IME Back regressions for the Android keyboard presentation proof without mixing editor input fixtures into that responsibility |
| `lib/mobile-source-control-interaction.mjs` | Source Control open, confirmed-row commit acknowledgement, history proof, and failure diagnostics for the Android WebView lifecycle harness |
| `lib/android-webview-cdp.mjs`; `lib/android-webview-cdp-client.mjs`; `android-webview-cdp.test.mjs` | Raw Android WebView page-target discovery, bounded CDP transport client, sanitized diagnostics, and generation-aware reconnect/reload regressions including target-list/snapshot origin races and exact-origin RemoteBrowser entry admission without lease renewal |
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
| `smoke-docker-remote-import.sh`; `lib/docker-remote-import-{edge,stable-edge,fixture}.sh` | Runs the atomic B6 provider/browser producer, pins public-CA quick-tunnel hostnames to verified edge IPs, and permits one distinct fixture-only edge replacement while preserving exact candidate identity and the 60-sample candidate-network gate |
| `lib/docker-p2p-mesh-diagnostics.sh` / `docker-p2p-mesh-diagnostics.test.sh` | Bounded, token-redacted node-role/container-health/resource diagnostics for FullPeer mesh failures |
| `lib/docker-multiclient-product-journeys.mjs` | Stable re-export boundary for Docker product journeys |
| `lib/docker-multiclient-repo-lifecycle.mjs` | Repo create/switch/removal, last-repo NoScope, restart, and cross-client recreation journey |
| `lib/docker-multiclient-workspace.mjs` | Candidate-container projection locator, identity, mutation, and removal-preservation fixture boundary |
| `lib/docker-multiclient-source-control.mjs` | Typed diff, Source Control commit/history, and ledger-gated External Changes journey |
| `lib/docker-multiclient-runtime.mjs` | Browser diagnostics, shell probes, exact runtime-incarnation restart proof, and narrow restart transport-error classification for Docker multiclient evidence |
| `smoke-runtime-release-info.sh` | Checks a running server's `/api/node/role` runtime release info fields |
| `lib/android-tools.sh` | Shared Android SDK / Android Studio JBR discovery helpers for local and target-host Android gates |

## For AI Agents

### Working In This Directory

- Scripts target Windows (CMD/PowerShell) since primary dev environment is WSL on Windows.
- Keep scripts minimal and focused on a single task.

<!-- MANUAL: -->
