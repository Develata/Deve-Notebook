# Current Runtime Runbook

This runbook describes the current implemented startup and test paths. It is not
a roadmap for future desktop/mobile native apps, server-backed Settings API, or
full Tantivy indexing.

## Local Backend

Use explicit development mode for local runs:

```bash
cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001
```

`--dev` sets `DEVE_ENV=development` for the current serve process when the
environment variable is unset. The default development login is `admin` /
`admin`. These defaults are only valid for `--dev` or explicit
`DEVE_ENV=development`.

To include the current lightweight search runtime gate:

```bash
cargo run -p deve_cli --features search --bin deve_cli -- serve --dev --port 3001
```

Without the `search` feature, search requests must fail closed with a structured
unavailable error.

## Local Frontend

Preferred embedded path:

```bash
scripts/smoke-web-release-build.sh
cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001
```

Open `http://127.0.0.1:3001/`. The CLI embeds `apps/web/dist` at build time, so
after Web source changes you must rebuild `apps/web/dist` before rebuilding or
running the CLI. Otherwise the embedded server can serve stale WASM.
The wrapper normalizes Trunk's `NO_COLOR` parsing and suppresses non-actionable
Browserslist database freshness noise from the Trunk Tailwind pipeline.

Fallback two-process path:

```bash
cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001
```

```bash
cd apps/web
NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080
```

Open `http://127.0.0.1:8080/`. This path avoids embedded-asset staleness during
UI work. Backend-only `serve --dev` may return 404 on `/` when neither embedded
assets nor a valid `DEVE_STATIC_DIR` are available; API and WS routes are still
the backend runtime boundary.

## Production Auth

Production is the default when `--dev` is not used and `DEVE_ENV` is not
`development`. Production must provide:

- `AUTH_SECRET`: JWT signing secret, at least 32 bytes.
- `AUTH_PASS`: Argon2 PHC password hash.
- `AUTH_USER`: optional username, defaults to `admin`.

If `AUTH_SECRET` or `AUTH_PASS` is missing, startup must exit non-zero with the
production auth error. For local testing, use `--dev` rather than weakening this
production boundary.

## Projection Health

Startup can skip a local repo when its Structure Facts authority cannot build a
safe tree projection. This is a repo-local degraded state, not a global server
startup failure.

Use diagnostics first:

```bash
cargo run -p deve_cli --bin deve_cli -- node-check --projection --repo <repo>
cargo run -p deve_cli --bin deve_cli -- repair --check --repo <repo>
```

Important fields:

- `status=healthy`: projection authority can be used.
- `status=authority_corrupt`: Structure Facts are corrupt.
- `rebuild_supported=false`: `repair --rebuild-projection` must not rewrite this
  authority automatically.
- `issue_code=missing_parent`: a node references a parent that is absent from
  Structure Facts.
- `repair_hint`: operator-facing next step for the diagnostic class.

`repair --check` is a repair-step preflight. It does not execute the repair
subcommand's mutating steps: shadow quarantine, repo-prefixed path rewrite,
backup restore, or projection table rebuild. Like other CLI diagnostics, startup
may still initialize or repair repo catalog metadata; run it against a copy if
byte-for-byte immutability is required.

For rebuild-supported projection drift, use:

```bash
cargo run -p deve_cli --bin deve_cli -- repair --repo <repo> --rebuild-projection
```

For `authority_corrupt` repos, inspect ledger/backups and restore authoritative
Structure Facts before expecting scan, watcher, export, or source-control paths
to treat that repo as healthy. The server should continue serving other healthy
repos.

## Source Control Smoke Hygiene

Source Control state lives in Deve's ledger/staging tables, not in Git's working
tree. `git status` can be clean while the app still has staged or unstaged
Source Control entries.

Before running browser commit/stage smoke tests, inspect the target repo:

```bash
cargo run -p deve_cli --bin deve_cli -- sc-status --repo <repo>
```

The command is read-only. It prints separate `staged` and `unstaged` counts so a
dirty dev fixture is visible before the UI test starts. Do not make acceptance
cases depend on the checked-in local `default` ledger being clean; use a seeded
temporary repo for exact counts, or assert only that Source Control loads and
reports its current state.

## Docker Release Smoke

After enabling Docker Desktop WSL integration or another Docker-compatible
runtime, run:

```bash
DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh
```

If the Docker CLI is available under a non-default executable name or path, set:

```bash
DEVE_DOCKER_BIN=/path/to/docker DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh
```

The script builds the local Dockerfile, starts the image with production
`AUTH_SECRET` / `AUTH_PASS` material, waits for
`http://127.0.0.1:3001/api/node/role`, verifies production login with the
matching smoke password, then removes the smoke container and temporary Docker
data volume. Without
`DEVE_DOCKER_SMOKE_REQUIRED=1`, a machine that does not
provide Docker reports a skip instead of failing the local baseline.
When Docker is missing or unreachable, the script prints the resolved Docker
binary plus `DOCKER_HOST` / `DOCKER_CONTEXT` so WSL and remote daemon issues are
diagnosable without changing the script.

## Docker Compose

`docker-compose.yml` is the production compose entry. It runs the published
`ghcr.io/develata/deve-notebook:latest` image, persists data under `./data`, and
fails closed unless `AUTH_SECRET` and `AUTH_PASS` are set.

```bash
docker compose up -d
```

Use `docker-compose.dev.yml` only when validating the local Dockerfile build:

```bash
docker compose -f docker-compose.dev.yml up --build
```

## Runtime Release Info Smoke

With a local backend already running, validate the public runtime/release shape
endpoint:

```bash
DEVE_RUNTIME_SMOKE_REQUIRED=1 scripts/smoke-runtime-release-info.sh
```

The script checks `role`, `version`, `profile`, `delivery`, `environment`,
`ws_port`, `main_port`, and the aggregated `repo_health` object from
`/api/node/role`. Use
`DEVE_RUNTIME_BASE_URL=http://127.0.0.1:<port>` when testing a non-default port.
Without `DEVE_RUNTIME_SMOKE_REQUIRED=1`, an unavailable local server reports a
skip instead of failing the baseline.

`repo_health.status=degraded` means at least one local repo was skipped for
projection execution while the server stayed available for other healthy repos.
The public endpoint intentionally exposes aggregate counts only. Use
`node-check --projection --repo <repo>` or protected `/api/admin/projection-check`
for repo-specific details.

## Release Dependency Audit

Local diagnostic mode:

```bash
scripts/check-release-audit-gate.sh
```

The script runs `cargo audit` when `cargo-audit` is installed and runs
`npm audit --audit-level=high` for `apps/web` when `npm` is available. Missing
tools print an explicit skip diagnostic in local mode.

Release / CI required mode:

```bash
DEVE_RELEASE_AUDIT_REQUIRED=1 scripts/check-release-audit-gate.sh
```

Required mode fails closed if `cargo-audit` or `npm` is unavailable.
Install the Rust audit tool with:

```bash
cargo install cargo-audit --locked
```

## Runtime Happy Path Smoke

Validate the current in-process runtime write/read path without depending on the
checked-in dev ledger:

```bash
scripts/smoke-runtime-happy-path.sh
```

The script uses temporary repo state and the real Axum/WebSocket harness. It
covers repo switch, `SyncHello`, `RegisterWriter`, document create, edit ack,
confirmed `NewOp`, `OpenDoc`, history readback, and the Web reconnect bootstrap
unit contract.

## Runtime Recovery Smoke

Validate the current degraded/reconnect recovery path without depending on the
checked-in dev ledger:

```bash
scripts/smoke-runtime-recovery-path.sh
```

The script covers degraded local projection write gates, stale sync-scope
cleanup, Web write/read gates for recovery states, message refresh scope guards,
status summary mapping, and auth-probe separation from ordinary reconnect.

## Desktop Package Build Gate

Validate the Desktop packaging preflight and diagnose target-host package build
readiness:

```bash
scripts/check-desktop-platform-package-build.sh
```

The script keeps ordinary local baselines diagnostic-only. It reports missing
target-host prerequisites such as the Desktop Tauri binary entrypoint, build
script, Web `dist/index.html`, or `cargo-tauri` CLI, then exits successfully
unless package build is explicitly required.

On a target platform host, require the actual package build:

```bash
DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 scripts/check-desktop-platform-package-build.sh
```

Required mode fails closed on missing prerequisites. When all prerequisites are
present it runs `cargo tauri build` from `apps/desktop`. A Linux or WSL result
only validates that host target; it does not certify macOS or Windows packages.
Use `DEVE_DESKTOP_PACKAGE_BUNDLES=deb,rpm` to verify a Linux bundle subset when
the host cannot run the AppImage `linuxdeploy` path.

For Linux AppImage verification, `linuxdeploy --plugin gtk` requires
`pkg-config librsvg-2.0` metadata; on Debian/Ubuntu hosts install
`librsvg2-dev`. On WSL hosts without `libfuse2`, run AppImage tooling through
`APPIMAGE_EXTRACT_AND_RUN=1`.

Diagnose macOS/Windows target-host prerequisites without claiming readiness on
the wrong host:

```bash
scripts/check-desktop-target-host-preflight.sh
```

Use `DEVE_DESKTOP_TARGET_HOSTS=macos` or `windows` to narrow diagnostics. On a
real target host, set `DEVE_DESKTOP_TARGET_HOST_PREFLIGHT_REQUIRED=1` to make
missing signing/build prerequisites fail closed before running the package
build script.

Target-host handoff commands:

```bash
DEVE_DESKTOP_TARGET_HOST_PREFLIGHT_REQUIRED=1 DEVE_DESKTOP_TARGET_HOSTS=macos scripts/check-desktop-target-host-preflight.sh
DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 scripts/check-desktop-platform-package-build.sh
```

```bash
DEVE_DESKTOP_TARGET_HOST_PREFLIGHT_REQUIRED=1 DEVE_DESKTOP_TARGET_HOSTS=windows scripts/check-desktop-target-host-preflight.sh
DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis scripts/check-desktop-platform-package-build.sh
```

Capture the host OS, tool versions, command output, artifact paths, install
result, startup result, and an explicit statement that no child-process runtime
or native authority write path was opened. Store the target-host result under
`docs/report/`.

Use the evidence template and validator:

```bash
cp docs/report/native-target-host-evidence-template.md docs/report/native-target-host-<target>-YYYY-MM-DD.md
scripts/check-native-target-host-evidence.sh docs/report/native-target-host-<target>-YYYY-MM-DD.md
```

Optional GitHub Actions entry:

```text
Native Target Host -> target=desktop-macos|desktop-windows
```

The workflow is manual-only and diagnostic by default. Set
`required_preflight=true` to fail closed on missing prerequisites. Set
`run_desktop_package_build=true` only when the target-host runner is intended to
produce package artifacts. Each target-host job uploads a validated
`deve-native-target-host-evidence-*` artifact.

CLI dispatch helper:

```bash
scripts/dispatch-native-target-host-workflow.sh
DEVE_NATIVE_TARGET_HOST_DISPATCH=1 DEVE_NATIVE_TARGET_HOST_TARGET=desktop-macos DEVE_NATIVE_TARGET_HOST_REQUIRED_PREFLIGHT=true DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_PACKAGE_BUILD=true scripts/dispatch-native-target-host-workflow.sh
```

The helper is dry-run by default and requires an authenticated GitHub CLI before
it can dispatch the manual workflow. If `gh` is unavailable, the helper can use
`DEVE_GITHUB_TOKEN`, `GH_TOKEN`, or `GITHUB_TOKEN` plus `curl`; set
`DEVE_NATIVE_TARGET_HOST_REPOSITORY=owner/repo` when the repository cannot be
derived from `origin`.

Collect and validate workflow evidence artifacts after a run completes:

```bash
scripts/collect-native-target-host-evidence.sh
DEVE_NATIVE_TARGET_HOST_EVIDENCE_COLLECT=1 DEVE_NATIVE_TARGET_HOST_RUN_ID=<run-id> scripts/collect-native-target-host-evidence.sh
```

The collector is dry-run by default. It validates every downloaded evidence
Markdown file with `scripts/check-native-target-host-evidence.sh`. It uses an
authenticated GitHub CLI when available, otherwise `DEVE_GITHUB_TOKEN`,
`GH_TOKEN`, or `GITHUB_TOKEN` plus `curl` and `unzip`.

## Mobile Package Build Preflight

Validate the Mobile shell manifest and diagnose Android/iOS target-host package
prerequisites:

```bash
scripts/check-mobile-platform-package-preflight.sh
```

The script keeps ordinary local baselines diagnostic-only. It verifies the
Mobile shell remains shell-only, blocks iOS generated project paths, checks the
`native-packaging` compile/test surface, and reports missing target host tools
such as `cargo tauri`, Android SDK/JDK/ADB, Rust Android target, macOS/Xcode,
or Rust iOS target.

To require prerequisites on a target host without running a package build:

```bash
DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED=1 scripts/check-mobile-platform-package-preflight.sh
```

Use `DEVE_MOBILE_PACKAGE_TARGETS=android` or
`DEVE_MOBILE_PACKAGE_TARGETS=ios` to narrow diagnostics. Linux/WSL can only
diagnose Android readiness; iOS readiness requires macOS. This gate does not
run `cargo tauri android build` or `cargo tauri ios build`.

Android shell-only package execution is a separate explicit gate:

```bash
scripts/check-mobile-android-shell-package-build.sh
```

By default it runs boundary/preflight checks and does not build. On an Android
target host, set `DEVE_MOBILE_ANDROID_PACKAGE_BUILD_REQUIRED=1` to allow
`cargo tauri android init` and `cargo tauri android build` under
`apps/mobile/native-packaging`. This still does not open iOS packaging,
child-process runtime, or native authority writes.
The required Android build also needs Gradle wrapper distribution and Gradle
Plugin Portal dependencies to resolve or already exist in the host cache.

Target-host handoff commands:

```bash
DEVE_MOBILE_ANDROID_PACKAGE_BUILD_REQUIRED=1 scripts/check-mobile-android-shell-package-build.sh
```

```bash
DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED=1 DEVE_MOBILE_PACKAGE_TARGETS=ios scripts/check-mobile-platform-package-preflight.sh
```

The current iOS gate is preflight-only. Do not run `cargo tauri ios init` or
`cargo tauri ios build` until an explicit iOS shell-only package execution gate
is added. Capture Android artifacts or iOS missing prerequisites under
`docs/report/`, together with the same no-process/no-authority boundary.

Optional GitHub Actions entry:

```text
Native Target Host -> target=mobile-ios
```

The workflow is manual-only and runs iOS preflight only. It does not run
`cargo tauri ios init` or `cargo tauri ios build`. It uploads a validated
`deve-native-target-host-evidence-ios` artifact.

## Native Process Adapter Gate

Validate that the native process adapter remains gate-closed and state-machine
only:

```bash
scripts/check-native-process-adapter-gate.sh
```

The script checks the process adapter policy, rejects `std::process`,
`Command::new`, `tokio::process`, or direct spawn usage in Desktop/Mobile/native
adapter code, and runs targeted process-observation tests. It does not open the
child-process runtime.

## Chrome MCP Smoke

In WSL2, if Chrome MCP cannot connect because `127.0.0.1:9222` is down, run:

```bash
chrome-mcp http://127.0.0.1:8080/
```

Use `http://127.0.0.1:3001/` for the embedded path or
`http://127.0.0.1:8080/` for the Trunk fallback path. For search smoke testing,
start the backend with `--features search`, log in with the development account,
and verify the UI reaches `Ready` before submitting a search query.

## Verification

Targeted tests are preferred while implementing:

```bash
cargo test -p deve_cli <filter> -- --nocapture
cargo test -p deve_core <filter> -- --nocapture
cargo test -p deve_web <filter> -- --nocapture
```

Current docs/code guard scripts:

```bash
scripts/check-acceptance-bindings.sh
scripts/check-auth-baseline.sh
scripts/check-auth-unauthorized-state.sh
scripts/check-network-baseline.sh
scripts/check-cli-settings-baseline.sh
scripts/check-browser-prefs-boundary.sh
scripts/check-storage-repo-baseline.sh
scripts/check-search-baseline.sh
scripts/check-rendering-baseline.sh
scripts/check-ai-baseline.sh
scripts/check-feature-operation-paths.sh
scripts/check-i18n-hardcoded-baseline.sh
scripts/check-i18n-formatting-baseline.sh
scripts/check-source-control-baseline.sh
scripts/check-source-control-smoke-hygiene.sh
scripts/check-dev-data-health-baseline.sh
scripts/check-native-track-boundary.sh
scripts/check-native-packaging-gate.sh
scripts/check-native-process-adapter-gate.sh
scripts/check-native-target-host-evidence.sh
scripts/dispatch-native-target-host-workflow.sh
scripts/write-native-target-host-evidence.sh
scripts/check-desktop-package-preflight.sh
scripts/check-desktop-platform-package-build.sh
scripts/check-mobile-platform-package-preflight.sh
scripts/check-graph-baseline.sh
scripts/check-diff-color-baseline.sh
scripts/check-large-doc-baseline.sh
scripts/check-mobile-baseline.sh
scripts/check-ui-dashboard-refresh-baseline.sh
scripts/check-ui-desktop-baseline.sh
scripts/check-ui-disconnect-baseline.sh
scripts/check-ui-focus-baseline.sh
scripts/check-ui-spa-routing-baseline.sh
scripts/check-ui-token-baseline.sh
scripts/check-ui-z-index-baseline.sh
scripts/check-dev-runbook-baseline.sh
scripts/check-ws-structured-errors.sh
scripts/check-release-audit-gate.sh
scripts/check-release-baseline.sh
scripts/check-architecture-registry.sh
scripts/plan-coverage.sh
scripts/smoke-web-release-build.sh
scripts/smoke-runtime-release-info.sh
scripts/smoke-docker-release.sh
```

Use full-suite checks as release/final verification, not as the default inner
loop on a low-memory machine:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
