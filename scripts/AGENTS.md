<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 | Updated: 2026-04-24 -->

# scripts

## Purpose

Build and lint utility scripts for Deve-Notebook. Provides low-memory lint configurations for resource-constrained environments.

## Key Files

| File | Description |
|------|-------------|
| `plan-coverage.sh` | Plan-code coverage, file-size fuse, i18n leak, and acceptance binding checks |
| `lint-low-mem.cmd` | Windows CMD script — runs clippy with reduced memory |
| `lint-low-mem.ps1` | PowerShell script — runs clippy with reduced memory |
| `check-architecture-registry.sh` | Verifies operation registry, acceptance refs, drift map, Lisp IDs, and graph spines stay aligned |
| `check-ws-structured-errors.sh` | Verifies WS protocol errors remain structured as `ServerError`/`ServerErrorCode` |
| `check-auth-unauthorized-state.sh` | Verifies auth failures map to Unauthorized instead of reconnect/disconnected UI |
| `check-auth-baseline.sh` | Verifies Auth startup, cookie/JWT/status, WS 401, rate-limit, headers, and frontend session-expired contracts |
| `check-network-baseline.sh` | Verifies NET-001..NET-004 reconnect, `/ws`, node role, and WS frame baseline contracts |
| `check-search-baseline.sh` | Verifies current Search scope, feature-gate, stale-result, and future-index boundaries |
| `check-rendering-baseline.sh` | Verifies Markdown rendering current/future split, lightweight renderer subset, and controlled apply boundaries |
| `check-ui-token-baseline.sh` | Verifies style color literals stay confined to design-token files |
| `check-ui-z-index-baseline.sh` | Verifies shell z-index registry tokens and prevents private numeric z-levels |
| `check-ui-focus-baseline.sh` | Verifies modal focus trap and restore bindings for Command Palette surfaces |
| `check-ui-spa-routing-baseline.sh` | Verifies SPA route fallback stays 200 while API/WS paths do not fall back to index |
| `check-ui-disconnect-baseline.sh` | Verifies disconnect lockdown overlay copy and edit-disabled bindings |
| `check-ui-dashboard-refresh-baseline.sh` | Verifies Dashboard SystemMetrics refresh and WS-backed sync status bindings |
| `check-ui-desktop-baseline.sh` | Verifies Desktop canonical column markers, split diff scroll sync, layout resize persistence, and Unified Search mode routing bindings |
| `check-cli-settings-baseline.sh` | Verifies CLI command surface, `config.toml` settings mutation, and shortcut entry contracts |
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
| `check-desktop-platform-package-build.sh` | Diagnoses target-host Desktop package build prerequisites and only runs `cargo tauri build` when explicitly required |
| `check-desktop-package-startup-smoke.sh` | Verifies target-host Desktop package artifacts expose a startup-probeable shell binary without opening process runtime or authority writes |
| `check-desktop-installer-smoke.sh` | Verifies target-host Desktop installer install/startup/uninstall flow without opening process runtime or authority writes |
| `check-desktop-target-host-preflight.sh` | Diagnoses macOS/Windows Desktop target-host prerequisites without claiming package readiness on the wrong host |
| `check-mobile-platform-package-preflight.sh` | Diagnoses Android/iOS target-host prerequisites while keeping Mobile package build/project generation closed |
| `check-mobile-android-shell-package-build.sh` | Runs the Android WebView shell package gate only when explicitly required on an Android-capable target host |
| `check-mobile-android-emulator-install-startup-smoke.sh` | Boots an Android emulator target host, builds a debug WebView shell APK, and delegates install/startup smoke without opening process runtime |
| `check-mobile-ios-shell-package-build.sh` | Runs the iOS WebView shell package gate only when explicitly required on a macOS target host |
| `check-graph-baseline.sh` | Verifies Graph remains a read-only derived projection and does not become a ledger/workspace authority path |
| `check-mobile-baseline.sh` | Verifies Mobile Web shell viewport mapping, drawer gestures, resize-handle exclusion, keyboard toolbar, search top sheet/results scrolling, bottom bar, and editor font-size baseline contracts |
| `check-dev-runbook-baseline.sh` | Verifies current startup, auth, frontend, Chrome MCP, search, and verification runbook boundaries |
| `check-feature-operation-paths.sh` | Verifies feature operation and acceptance docs do not point at removed source/script/doc paths |
| `check-i18n-formatting-baseline.sh` | Verifies visible frontend time formatting goes through the locale-aware formatting utility |
| `check-release-baseline.sh` | Verifies Docker, compose, and release workflow surfaces match the embedded-frontend release baseline |
| `smoke-web-release-build.sh` | Builds the Web release assets with normalized Trunk/Browserslist environment |
| `smoke-runtime-happy-path.sh` | Runs temporary-repo Axum/WebSocket happy-path tests for switch, handshake, writer, edit, open, history, and reconnect bootstrap |
| `smoke-runtime-recovery-path.sh` | Runs degraded-local, stale-scope, reconnect gate, status, and auth-probe recovery smoke tests |
| `smoke-docker-release.sh` | Builds and runs the Docker release image smoke test when Docker is available |
| `smoke-runtime-release-info.sh` | Checks a running server's `/api/node/role` runtime release info fields |

## For AI Agents

### Working In This Directory

- Scripts target Windows (CMD/PowerShell) since primary dev environment is WSL on Windows.
- Keep scripts minimal and focused on a single task.

<!-- MANUAL: -->
