# Current Runtime Runbook

This runbook describes the current implemented startup and test paths. It is not
a roadmap for future desktop/mobile native apps, server-backed Settings API, or
full Tantivy indexing.

## Local Backend

Fresh local data roots must first create a repo and host-local Projection
Locator. `serve --dev` does not infer a projection base from `ledger_dir` or a
global vault path.

```bash
export DEVE_RUNTIME_ROOT="${DEVE_RUNTIME_ROOT:-target/codex-smoke/web-runtime}"
mkdir -p "$DEVE_RUNTIME_ROOT/projection-base"
export DEVE_LEDGER_DIR="$DEVE_RUNTIME_ROOT/config-root/ledger"
cargo run -p deve_cli --bin deve_cli -- init --path "$DEVE_RUNTIME_ROOT/config-root" --repo default --projection-base "$DEVE_RUNTIME_ROOT/projection-base"
```

Use explicit development mode for local runs after the data root has a valid
Projection Locator:

```bash
cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001
```

`--dev` selects the development runtime environment for the current serve
startup without mutating process-wide `DEVE_ENV`. The default development login
is `admin` / `admin`. These defaults are only valid for `--dev` or explicit
`DEVE_ENV=development`.

To include the current lightweight search runtime gate:

```bash
cargo run -p deve_cli --features search --bin deve_cli -- serve --dev --port 3001
```

Without the `search` feature, search requests must fail closed with a structured
unavailable error.

## Local Frontend

Preferred embedded path (`CMD-007A` embedded browser runtime smoke):

```bash
# after the Local Backend prep above
scripts/smoke-web-release-build.sh
cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001
```

Open `http://127.0.0.1:3001/`. The CLI embeds `apps/web/dist` at build time, so
after Web source changes you must rebuild `apps/web/dist` before rebuilding or
running the CLI. Otherwise the embedded server can serve stale WASM.
The wrapper normalizes Trunk's `NO_COLOR` parsing and suppresses non-actionable
Browserslist database freshness noise from the Trunk Tailwind pipeline.
Backend static delivery rejects `index.html` files that still contain Trunk
development live-reload markers such as `.well-known/trunk/ws`; rebuild with
`trunk build --release` before embedding or setting `DEVE_STATIC_DIR`.
For `CMD-007A`, the browser smoke must confirm the page reaches either `Ready`
or `Login`, and network traffic includes `/api/auth/status` and `/api/node/role`.

Fallback two-process path (`CMD-007B` Trunk browser dev runtime smoke):

```bash
# after the Local Backend prep above
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
For `CMD-007B`, the browser smoke must confirm the page reaches either `Ready`
or `Login`, and network traffic includes `/api/node/role`.

To print both runtime paths without starting servers:

```bash
scripts/smoke-web-runtime-paths.sh
```

Set `DEVE_WEB_RUNTIME_SMOKE_BUILD=1` to also run the embedded Web release build
step before printing the browser smoke commands.

## Settings / Command UI Smoke

Use the Trunk fallback path while iterating on Settings or Command Palette UI:

```bash
# after the Local Backend prep above
cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001
cd apps/web
NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080
```

Open `http://127.0.0.1:8080/` in Chrome. Run the smoke once at desktop width
and once at a narrow mobile viewport such as `390x844`.

Required checks:

- `Ctrl+Shift+P` opens Command Palette; searching `settings` shows `Open
  Settings` with the Settings group and browser-local/runtime-feedback detail.
- Invoking `Open Settings` closes the command surface and opens the Settings
  modal without navigating away from the app shell.
- Focus moves into the Settings modal instead of returning to the hidden command
  surface or its opener; pressing `Escape` closes Settings.
- The modal exposes `data-deve-settings-surface="modal"`, remains within the
  viewport on mobile, and its close controls remain 44px touch targets.
- Clicking `Night`, `Off`, and `Compact` updates the browser-local markers
  `data-deve-settings-theme`, `data-deve-settings-editor-wrap`, and
  `data-deve-settings-editor-density` without writing server settings.
- Trusted CLI disabled state renders `data-deve-setting-disabled-reason`, and
  Hybrid Editing exposes `data-deve-setting-disabled` plus `aria-disabled`.

## Local Quality Gate

Run this baseline before local commits that touch command/settings, CLI runtime,
or Web shell behavior:

```bash
git diff --check
rustfmt --edition 2024 --check <touched-rust-files>
cargo test -p deve_web acc_cmd_004 -- --nocapture
cargo test -p deve_cli commands::sc::tests -- --nocapture
cargo check -p deve_cli
cargo check -p deve_web --target wasm32-unknown-unknown
scripts/check-acceptance-matrix.sh
scripts/plan-coverage.sh --check-metadata-completeness
scripts/plan-coverage.sh --check-reverse-coverage
```

Do not put `scripts/plan-coverage.sh --summary-missing-plan-ref` into the quick
gate. It is an audit command and can be handled as a separate performance debt.

## Automated Acceptance Producers

The acceptance matrix and the smoke harnesses have different jobs:

- `docs/registry/acceptance-matrix.tsv` states which evidence each case or
  first-tag journey requires.
- `docs/registry/acceptance-producers.json` binds executable test/script and
  runtime receipt evidence to typed command steps.
- `deve_baseline acceptance-run` plans or executes those producers; it owns
  timeout, clean-HEAD checks, schema 3 receipts, producer-contract and execution
  group binding, Android claims, and failure reporting.
- `deve_baseline acceptance-collect` merges receipt artifacts without following
  symlinks or accepting duplicate evidence.

Inspect a tier without running external tools:

```bash
cargo run --locked -p deve_baseline -- acceptance-run --tier ci --plan
cargo run --locked -p deve_baseline -- acceptance-run --tier full --plan
cargo run --locked -p deve_baseline -- acceptance-run --tier target-host --plan
cargo run --locked -p deve_baseline -- acceptance-run --tier tag-ready --plan
```

Execution requires a clean worktree. Runtime receipt tiers also require a
receipt directory outside the repo; the CI test/script tier deliberately writes
no receipts and is run without `--receipt-dir`.
It is intentionally sequential for low-memory hosts. On Windows the runner
selects Git for Windows Bash; set `DEVE_ACCEPTANCE_BASH` only when Git Bash is
installed elsewhere.
CI Cargo steps select one explicit `--lib`, `--bin`, or `--test` target so a
function selector does not repeatedly enumerate every integration binary.
When a registered baseline script already owns a complete selector group, the
producer runs that script once instead of duplicating its Cargo commands.
Nested baseline wrapper scripts automatically reuse the runner's current
`deve_baseline` executable. Do not clear the runner-owned `DEVE_BASELINE_BIN`:
re-entering through `cargo run -p deve_baseline` would contend with the running
Windows executable and needlessly rebuild the tool on every script step.
An `--evidence-id` filter selects the owning producer and still emits that
producer's complete atomic receipt group. Runner-owned state directories let
finally steps clean only resources created by the current execution.

`acceptance-receipt` is a low-level diagnostic wrapper and emits
`manual.unbound` receipts. It cannot substitute for a registry-owned producer
at tag-ready. A producer-owned output directory is published only after its
receipt set is complete; the collector rejects partial or mixed execution IDs.
Collector inputs are canonical-root pinned, reject symlink/reparse entries and
directory trees deeper than 32 levels, and are limited to 4096 JSON files,
1 MiB per receipt, and 16 MiB total.

```powershell
$receiptRoot = Join-Path $env:TEMP "deve-acceptance-$((git rev-parse --short HEAD).Trim())"
cargo run --locked -p deve_baseline -- acceptance-run `
  --tier full --receipt-dir $receiptRoot
```

Run the source-bound executable evidence selected for ordinary CI:

```bash
cargo run --locked -p deve_baseline -- acceptance-run --tier ci
```

Windows Docker results are valid development evidence, but Docker tag-ready
receipts are intentionally Linux-host-bound. Windows is a supported Android
target host: the LocalBackend producer builds the x86_64 APK, owns an API 37.1
Google APIs AVD, records the current WebView provider and real Ed25519 probe,
drives the writable lifecycle, and cleans the emulator. RemoteBrowser uses the
same emulator owner with `DEVE_MOBILE_ANDROID_EMULATOR_JOURNEY=remote` and also
requires an exact HTTPS origin plus temporary credentials.

Aggregate downloaded workflow artifacts before the final gate:

```bash
RECEIPT_TMP=/tmp # use /private/tmp on macOS because /tmp is a symlink
cargo run --locked -p deve_baseline -- acceptance-collect \
  --output "$RECEIPT_TMP/deve-collected-receipts" artifact-linux artifact-windows artifact-android
cargo run --locked -p deve_baseline -- acceptance-matrix \
  --tag-ready "$RECEIPT_TMP/deve-collected-receipts"
```

Passing a structural matrix check, registering a producer, passing one host's
smoke, and satisfying cross-platform tag-ready are four distinct conclusions.
Current explicit `gap` rows continue to block the first tag.

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
subcommand's mutating steps: shadow quarantine, repo-prefixed path rewrite, or
projection table rebuild. Like other CLI diagnostics, startup may still initialize
or repair repo catalog metadata; run it against a copy if byte-for-byte immutability
is required.

For rebuild-supported projection drift, use:

```bash
cargo run -p deve_cli --bin deve_cli -- repair --repo <repo> --rebuild-projection
```

For `authority_corrupt` repos, inspect ledger state and restore authoritative
Structure Facts through ledger/Git recovery before expecting scan, watcher, export, or
source-control paths to treat that repo as healthy. The server should continue serving other healthy
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

## Projection Backup Diagnostics

Projection Backup is the backup-oriented product name for Remote Projection
Transport. It transfers Markdown Projection Workspace files through WebDAV/S3
and deliberately does not transfer ledger history, encrypted packs, branch
manifests, or RestoreCandidate handles.

Push examples:

```bash
cargo run -p deve_cli --bin deve_cli -- projection-remote webdav push --locator webdav+https://dav.example.com/notebooks/main
cargo run -p deve_cli --bin deve_cli -- projection-remote s3 push --locator s3://bucket-name/notebooks/main
cargo run -p deve_cli --bin deve_cli -- projection-remote s3 push --profile minio --locator s3+https://minio.example.com/bucket-name/notebooks/main
```

Immutable Remote Import examples (replace the IDs with values returned by the
preceding command):

```bash
cargo run -p deve_cli --bin deve_cli -- remote-import prepare webdav --repo <repo-uuid>
cargo run -p deve_cli --bin deve_cli -- remote-import prepare s3 --repo <repo-uuid>
cargo run -p deve_cli --bin deve_cli -- remote-import list --repo <repo-uuid>
cargo run -p deve_cli --bin deve_cli -- remote-import show --repo <repo-uuid> --session <session-uuid> --revision <revision>
cargo run -p deve_cli --bin deve_cli -- remote-import diff --repo <repo-uuid> --session <session-uuid> --revision <revision> --entry <opaque-entry-id>
cargo run -p deve_cli --bin deve_cli -- remote-import refresh --repo <repo-uuid> --session <session-uuid> --revision <revision>
cargo run -p deve_cli --bin deve_cli -- remote-import apply --repo <repo-uuid> --session <session-uuid> --revision <revision> --request-id <request-uuid>
cargo run -p deve_cli --bin deve_cli -- remote-import discard --repo <repo-uuid> --session <session-uuid> --revision <revision>
cargo run -p deve_cli --bin deve_cli -- remote-import repair --repo <repo-uuid>
cargo run -p deve_cli --bin deve_cli -- remote-import repair --repo <repo-uuid> --apply
```

S3-compatible custom endpoints require a host-local secret-free profile before
provider I/O. Example profile setup:

```bash
cargo run -p deve_cli --bin deve_cli -- projection-remote s3 profile put \
  --profile minio \
  --endpoint-origin https://minio.example.com \
  --bucket bucket-name \
  --allowed-prefix notebooks/main \
  --region us-east-1 \
  --credential-env-prefix MINIO \
  --allowed-capabilities push,source-acquisition

export MINIO_ACCESS_KEY_ID=...
export MINIO_SECRET_ACCESS_KEY=...
# Optional:
export MINIO_SESSION_TOKEN=...
```

The profile store lives under `ledger/.host/remote-projection-s3-profiles.toml`
and stores only endpoint/bucket/prefix/signing metadata plus the credential env
prefix, never raw key material.

`remote-import prepare` captures an immutable manifest/blob snapshot without
writing the Projection Workspace, Ledger, External Changes, or Source Control.
Review is backend-owned and Apply commits the whole session through the sealed
Ledger writer; Projection writeback happens only after that commit. `repair` is
dry-run by default and requires explicit `--apply` for proven cleanup debt.
Provider metadata, ETags, mtimes, object versions, and remote listing order
remain diagnostics only. The removed `projection-remote ... pull` command has no
compatibility alias or fallback.

S3-compatible custom endpoints (`s3+https://...`) run only through an explicit
profile handle. Missing profile, endpoint/bucket/prefix mismatch, unsupported
addressing style, missing credential env, or resolver failure all fail closed
before provider I/O and before ambient AWS credentials are resolved.

## Docker Release Smoke

The first-tag channel is prepared before any tag exists. Dispatch the candidate
at the intended HEAD, wait for it to pass, then aggregate that exact run:

```bash
gh workflow run release-candidate.yml --ref main -f version=0.1.0
candidate_run_id="$(gh run list --workflow release-candidate.yml --branch main --limit 1 --json databaseId --jq '.[0].databaseId')"
gh run watch "$candidate_run_id" --exit-status
gh workflow run acceptance-aggregate.yml --ref main -f receipt_run_ids="$candidate_run_id"
aggregate_run_id="$(gh run list --workflow acceptance-aggregate.yml --branch main --limit 1 --json databaseId --jq '.[0].databaseId')"
gh run watch "$aggregate_run_id" --exit-status
```

Only after version/CHANGELOG/release-set freeze and every remaining tag blocker
is closed, create an annotated tag that immutably selects that aggregate. A
lightweight tag, a missing trailer, or more than one trailer is rejected:

```bash
git tag -a v0.1.0 -m "Deve-Acceptance-Aggregate-Run: $aggregate_run_id"
git push origin v0.1.0
```

The tag workflow downloads only that sealed bundle. It never rebuilds Docker,
Desktop, macOS, or Android bytes.

Candidate and aggregate artifacts are immutable. Do not rerun a failed run after
any artifact may have been uploaded; both workflows reject `run_attempt != 1`.
Dispatch a new run ID instead. The candidate verifier uses Release basenames in
public `SHA256SUMS`, validates bounded SPDX document structure, verifies the
sealed provenance and Docker-SPDX bundles against the exact workflow/HEAD, and
re-extracts exactly one Android APK signer. macOS DMGs keep their actual
`macos-x64` or `macos-arm64` suffix; host-only output is never called universal.
The workflow entrypoint is `scripts/check-release-candidate-bundle.sh`; it binds
the Rust candidate verifier, APK signer recheck, and both typed attestation
bundle verifications into one fail-closed gate.
The aggregate and promotion paths call `scripts/check-android-apk-signer.sh`
against the sealed APK itself; they never trust only the signer value copied
into the candidate manifest.

Full SemVer remains unchanged in the Git tag, workspace, manifest and GitHub
Release. Docker maps the unique `+` separator to `_build_`; prereleases never
update `latest`. Stable `latest` advances only when both SemVer precedence and
Git ancestry move forward.

Promotion classifies GitHub Release and GHCR state with
`scripts/probe-release-remote.sh`: only an explicit HTTP 404 is `absent`;
authentication, transport, rate-limit, and 5xx outcomes stop the workflow. If a
runner loses the result after the Release was made public, rerun is idempotent
only when the annotated tag, complete asset names and hashes, prerelease flag,
immutable version image ID, and registry digest all still match the same sealed
candidate. The original success path performs no extra network read after
`gh release edit` returns successfully. A validated already-public retry still
reapplies the stable/latest or prerelease classification idempotently so GitHub
Latest and GHCR `latest` cannot diverge.
Immediately before draft upload, registry mutation, and final publication,
`scripts/check-release-tag-binding.sh` re-fetches the remote annotated tag and
requires both its object ID and directly peeled candidate commit to remain
unchanged.

Before a tag-triggered release, require the tag (including prerelease/build
metadata) to match the workspace plus both Tauri manifests exactly. The fixture
test covers stable and prerelease/build versions as well as each mismatch:

```bash
bash scripts/check-release-version-match.sh v0.1.0
bash scripts/check-release-version-match.test.sh
bash scripts/validate-release-image-tags.test.sh
bash scripts/check-android-apk-signer.test.sh
bash scripts/probe-release-remote.test.sh
```

`scripts/validate-release-image-tags.sh` is the workflow's pre-push guard. It
accepts the metadata-action version plus the complete tag list and rejects
missing, duplicate, cross-repository, or unexpected tags before the first push.

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
`http://127.0.0.1:3102/api/node/role`, verifies production login with the
matching smoke password, then removes the smoke container and temporary Docker
data volume. Without
`DEVE_DOCKER_SMOKE_REQUIRED=1`, a machine that does not
provide Docker reports a skip instead of failing the local baseline.
Set `DEVE_DOCKER_SMOKE_PORT` to override the default host port. The script
fails fast when the selected port already serves `/api/node/role`, so a local
development server cannot be mistaken for the Docker smoke container.
When Docker is missing or unreachable, the script prints the resolved Docker
binary plus `DOCKER_HOST` / `DOCKER_CONTEXT` so WSL and remote daemon issues are
diagnosable without changing the script.

To smoke an already-built candidate without rebuilding it, set both the image
and skip-build variables. Release CI uses this mode so runtime and browser
evidence apply to the exact image ID that is later tagged and pushed:

```bash
DEVE_DOCKER_SMOKE_REQUIRED=1 DEVE_DOCKER_SMOKE_SKIP_BUILD=1 DEVE_DOCKER_SMOKE_IMAGE=<candidate-image> scripts/smoke-docker-release.sh
DEVE_DOCKER_MULTI_REQUIRED=1 DEVE_DOCKER_MULTI_SKIP_BUILD=1 DEVE_DOCKER_MULTI_IMAGE=<candidate-image> bash scripts/smoke-docker-multiclient.sh
```

Optional GitHub Actions entry for host-isolated Docker smoke:

```bash
gh workflow run docker-smoke.yml --ref main
gh run list --workflow docker-smoke.yml --limit 1
```

This workflow is manual-only. It runs the same `scripts/smoke-docker-release.sh`
on an Ubuntu runner with `DEVE_DOCKER_SMOKE_REQUIRED=1`; it does not publish a
GHCR image and does not replace the tag-triggered `release.yml` channel.
The release smoke also requires `delivery=embedded-frontend` and at least one
initialized local repo before accepting the production-auth login probe. It is
still a boot/auth preflight; REL-009 supplies the real browser path.

## Docker Multi-client Smoke

Run a real multi-browser WebLightPeer smoke against one containerized server:

```bash
DEVE_DOCKER_MULTI_REQUIRED=1 bash scripts/smoke-docker-multiclient.sh
```

The script uses `docker-compose.multiclient.yml` to build the local Dockerfile,
start a single `deve-server` on `http://127.0.0.1:3101`, wait for
`/api/node/role` and production login, then runs
`scripts/smoke-docker-multiclient.mjs` with a Playwright package installed under
`${TMPDIR:-/tmp}/deve-docker-multiclient-playwright` by default.
The script also runs `playwright install chromium` for first-time browser setup.
The Playwright harness creates isolated browser contexts, verifies same-origin
`/ws`, logs in as `admin` / `password`, creates and edits a document in one
client, opens it from a second client, and checks offline read-only plus
reconnect recovery. After reconnect, the recovered client writes again and the
other client must receive that edit; WebSocket proof is bound to the expected
container origin, and offline network errors are ignored only inside the
deliberate offline window.

Use `DEVE_DOCKER_MULTI_PORT=<port>` when 3101 is occupied. Set
`DEVE_DOCKER_MULTI_KEEP=1` to keep the compose project running for Chrome MCP
visual validation, then open `http://127.0.0.1:<port>/` and clean up with the
command printed by the script.

## Docker P2P Mesh Smoke

Run a two-server FullPeer mesh smoke with isolated Docker volumes:

```bash
DEVE_DOCKER_P2P_MESH_REQUIRED=1 bash scripts/smoke-docker-p2p-mesh.sh
```

The script uses `docker-compose.mesh.yml` to start `peer-a` and `peer-b` with
independent data/notes volumes, a shared `RepoId`, static peer configuration,
and P2P admission tokens supplied only through environment variables. It
verifies that peer A can write locally, peer B receives the update under peer
A's shadow repo, peer B's local branch remains unchanged until an explicit
merge/import step, and peer B performs a fresh authenticated handshake after
restart. The script does not execute an explicit merge or expose live vector
equality; use the NET-015 targeted merge test and NET-017 vector monotonicity
tests for those contracts.

For Windows/WSL stability, the smoke builds a single shared local image
serially and defaults `DEVE_DOCKER_P2P_MESH_BUILDKIT=0`; set it to `1` only
when the local Docker build context handles extended attributes reliably.

Use `DEVE_DOCKER_P2P_MESH_A_PORT=<port>` and
`DEVE_DOCKER_P2P_MESH_B_PORT=<port>` when the defaults are occupied. Set
`DEVE_DOCKER_P2P_MESH_KEEP=1` to keep both peers running for manual diagnostics;
the script prints the cleanup command. Tokens must not be written into
`config.toml`, compose files, logs, URLs, browser storage, or native bootstrap
payloads.

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

The script runs `cargo audit --json` when `cargo-audit` is installed, verifies
that all non-vulnerability warnings match
`docs/registry/release-audit-warning-registry.md`, and runs
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

Before the first formal public tag, also run:

```bash
cargo run -p deve_baseline -- release-audit-gate tag-ready
```

This fails while any registered audit warning still has `Tag blocker = yes`.

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
present it runs `cargo tauri build --features native-packaging` from
`apps/desktop`. Required mode first builds `deve_cli` and passes it to Tauri as
the `binaries/deve_cli` external binary sidecar, so package startup smoke can
exercise the local-service handoff path. A Linux or WSL result only validates
that host target; it does not certify macOS or Windows packages. Use
`DEVE_DESKTOP_PACKAGE_BUNDLES=deb,rpm` to verify a Linux bundle subset when the
host cannot run the AppImage `linuxdeploy` path.

For Linux AppImage verification, `linuxdeploy --plugin gtk` requires
`pkg-config librsvg-2.0` metadata; on Debian/Ubuntu hosts install
`librsvg2-dev`. On WSL hosts without `libfuse2`, run AppImage tooling through
`APPIMAGE_EXTRACT_AND_RUN=1`.

### Linux Apptainer / Slurm development gate

On the USTC 107 development host, the bare login and compute nodes do not own
the GTK3/WebKitGTK development libraries. Use the pinned Apptainer image and
run the heavy gate inside Slurm instead of installing a user-local recursive
APT sysroot:

```bash
mkdir -p "$HOME/.hermes-logs/deve-notebook"
sbatch \
  --account=stu \
  --partition=Students \
  --qos=qos_stu_default \
  --nodelist=anode06 \
  --cpus-per-task=2 \
  --mem=16G \
  --time=01:00:00 \
  --output="$HOME/.hermes-logs/deve-notebook/slurm-%j-desktop-apptainer.out" \
  scripts/check-desktop-linux-apptainer-slurm.sh
```

The default source mode requires a clean Git worktree and packages exactly
`HEAD` with `git archive`. When the orchestration source and 107 worktree have
diverged, upload an immutable source archive and bind it explicitly:

```bash
DEVE_APPTAINER_SOURCE_ARCHIVE="$HOME/.cache/deve-build-inputs/deve-source.tar.gz" \
DEVE_APPTAINER_SOURCE_REVISION=<git-revision> \
DEVE_APPTAINER_SOURCE_SHA256=<sha256> \
sbatch \
  --account=stu \
  --partition=Students \
  --qos=qos_stu_default \
  --nodelist=anode06 \
  --cpus-per-task=2 \
  --mem=16G \
  --time=01:00:00 \
  --output="$HOME/.hermes-logs/deve-notebook/slurm-%j-desktop-apptainer.out" \
  scripts/check-desktop-linux-apptainer-slurm.sh
```

External source archives are restricted to regular files and directories;
links, special files, duplicate paths, absolute paths, and parent traversal are
rejected before staging.

The worker verifies the source and SIF checksums; stages source, Rust toolchain,
Cargo registry, and build target under node-local `/tmp`; builds release Web
assets; the host Web/WASM and container-native builds use separate Cargo target
directories so host GLIBC build scripts cannot contaminate the older container
ABI. The worker then opens one Apptainer session for package build, startup,
and native-session checks through the existing Desktop scripts. Keeping all
native gates in one session avoids repeated SIF extraction on 107 hosts without
`squashfuse`; the temporary container extraction uses node-local `/tmp` and is
removed on exit.
The validated SIF checksum default is intentionally fail-closed; override image
path and checksum together with `DEVE_APPTAINER_IMAGE` and
`DEVE_APPTAINER_IMAGE_SHA256` when the pinned image is deliberately replaced.

This is Linux developer/target-host evidence only. It does not re-enable Linux
artifacts in the first-tag release set, prove signing or installer readiness,
or replace macOS/Windows target-host evidence.

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
DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=app,dmg scripts/check-desktop-platform-package-build.sh
DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=app,dmg DEVE_DESKTOP_PACKAGE_NO_SIGN=1 scripts/check-desktop-platform-package-build.sh
DEVE_DESKTOP_STARTUP_SMOKE_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=app,dmg scripts/check-desktop-package-startup-smoke.sh
DEVE_DESKTOP_NATIVE_SESSION_SMOKE_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=app,dmg scripts/check-desktop-native-session-package-smoke.sh
DEVE_DESKTOP_INSTALLER_SMOKE_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=app,dmg scripts/check-desktop-installer-smoke.sh
```

Use the unsigned macOS command only for CI/package-shape smoke validation. It
does not replace signed/notarized release packaging.

```bash
DEVE_DESKTOP_TARGET_HOST_PREFLIGHT_REQUIRED=1 DEVE_DESKTOP_TARGET_HOSTS=windows scripts/check-desktop-target-host-preflight.sh
DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis scripts/check-desktop-platform-package-build.sh
DEVE_DESKTOP_STARTUP_SMOKE_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis scripts/check-desktop-package-startup-smoke.sh
DEVE_DESKTOP_NATIVE_SESSION_SMOKE_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis scripts/check-desktop-native-session-package-smoke.sh
DEVE_DESKTOP_INSTALLER_SMOKE_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis scripts/check-desktop-installer-smoke.sh
```

For local Windows release-binary smoke without MSI/NSIS artifacts, build the
release binaries first and use the `exe` selector only with startup and
native-session smoke:

```bash
cargo build --release --locked -p deve_cli --bin deve_cli
cargo build --release --locked -p deve_desktop --features native-packaging
DEVE_DESKTOP_STARTUP_SMOKE_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=exe scripts/check-desktop-package-startup-smoke.sh
DEVE_DESKTOP_NATIVE_SESSION_SMOKE_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=exe scripts/check-desktop-native-session-package-smoke.sh
```

The `exe` selector is not a package-build or installer selector. It proves only
that `target/release/deve_desktop.exe` and the sibling `deve_cli.exe` can run the
native smoke probes on the current target host.

The startup smoke runs the packaged Desktop binary with
`DEVE_DESKTOP_STARTUP_SMOKE=1`. It validates that the binary can start and
report a shell-only runtime surface, then exits before opening a GUI window,
child-process runtime, ledger, Projection Locator/workspace, source-control, search, Git, or `.notegit`
authority path. This is package startup evidence, not installer
install/uninstall evidence. The probe defaults to a 20-second timeout; override
with `DEVE_DESKTOP_STARTUP_SMOKE_TIMEOUT_SECS=<seconds>` only when diagnosing a
slow target host.

The native session package smoke runs the packaged Desktop binary with
`DEVE_DESKTOP_NATIVE_SESSION_SMOKE=1` and `DEVE_DESKTOP_LOCAL_SERVICE=1` from a
temporary data root. It verifies that the bundled sibling `deve_cli` can start
`serve --native-loopback`, issue the native-only HttpOnly session cookie, pass
`/api/auth/status`, and exit without exposing token/secret material to JS-visible
bootstrap, URL, localStorage, logs, or crash reports. The smoke must stop the
local service before reporting `desktop-native-session-smoke: ok`; failure to
stop is treated as a local-backend lifecycle regression. It remains
diagnostic-only unless `DEVE_DESKTOP_NATIVE_SESSION_SMOKE_REQUIRED=1` is set.

The installer smoke is a separate target-host gate. On macOS it mounts the
`.dmg`, copies the `.app` to a temporary Applications directory, runs the same
startup probe, uninstalls by deleting the copied bundle, and verifies removal.
On Windows it runs MSI/NSIS silent install, probes the installed binary, then
runs LocalBackend lifecycle and local-bare-remote Git bridge checks. It also
invokes `scripts/check-desktop-packaged-ui-smoke.ps1`, which starts the installed
window with isolated data/WebView2 state and a WebView2-assigned ephemeral CDP
endpoint discovered through the isolated `EBWebView/DevToolsActivePort`, and drives
`scripts/smoke-desktop-packaged-ui.mjs` through native session, document edit,
NoteGit commit/history, and Settings focus-trap flows. The gate requires exactly
one installed sibling sidecar while the app is alive and zero after exit before
running the installer-specific uninstall path. It remains diagnostic-only unless
`DEVE_DESKTOP_INSTALLER_SMOKE_REQUIRED=1` is set.

The smoke sets the exact `DEVE_DESKTOP_WEBVIEW2_CDP=assigned-loopback` diagnostic
marker. The Desktop host maps only that marker to a programmatic WebView2
`--remote-debugging-port=0` option, including the Wry default disabled-feature
arguments. This avoids relying on environment browser flags that elevated
target-hosts may ignore. Arbitrary browser arguments are not accepted, and the
application's normal startup path does not enable CDP. This evidence does not
claim to override host WebView2 policy or out-of-process injection.
The installer producer gives this smoke a 720-second outer command budget and
passes fixed phase budgets whose conservative sum plus cleanup margin is smaller;
an override that reverses that nesting fails before installation starts.

The preference-driven RemoteBrowser gate is intentionally separate from CLI or
environment overrides. Given an isolated installed Windows binary and a
temporary HTTPS test origin, it pre-seeds only the host-local preference, drives
remote login/edit/commit/history through WebView2 CDP, reloads after attaching
diagnostics to prove there is no native facade, `ipc.localhost` request, or
related CSP error, then invokes the native `Use Local Backend` menu item through
Windows UI Automation. The restarted process must own exactly one fresh local
sidecar/session and the normal packaged LocalBackend UI smoke must pass before
all processes are closed:

The installer owner must write
`<InstallRoot>/.deve-desktop-install-root.json` with exactly
`{"schema":1,"marker":"deve-desktop-remote-browser-smoke"}`. The smoke resolves
the install root, marker, Desktop binary, and sibling sidecar through final-path
handles; a symlink/junction escape or a merely similar path prefix is rejected.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-desktop-remote-browser-smoke.ps1 `
  -DesktopBinary C:\isolated\DeveNotebook\deve_desktop.exe `
  -InstallRoot C:\isolated\DeveNotebook `
  -WorkRoot C:\temp\deve-remote-smoke `
  -RemoteHttpsOrigin https://temporary-test-origin.example `
  -Username random-smoke-user `
  -Password random-smoke-password
```

The HTTPS origin and credentials must be temporary test material. This manual
external-origin route is diagnostic-only and cannot satisfy a tag-ready receipt:
a same-origin HEAD proof is still a deployment assertion, not proof of the
backend executable bytes. Formal Desktop/Android RemoteBrowser receipts therefore
use the workflow-owned internal fixture from the candidate HEAD.

`scripts/remote-browser-fixture.ps1` and
`scripts/remote-browser-fixture.sh` expose `start|run|stop`. Internal mode starts
the current-HEAD CLI as an ordinary `main` backend with hidden `--loopback-only`
(never native session semantics), generates random credentials without secret
argv, and downloads the pinned cloudflared binary only after its fixed SHA-256
matches. `run` installs signal/finally cleanup around the child command. `stop`
checks PID start tokens and container owner labels, removes every owned resource,
and deletes state only after absence is proven. If Docker/process inspection or
secret deletion is uncertain, it fails and preserves `.fixture-owner` plus
`fixture-state.json` so the same `stop --state-dir ...` can be retried. Do not
replace this gate with `--remote-url`, because explicit shell overrides
deliberately hide the native recovery menu.

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
produce package artifacts. Set `run_desktop_startup_smoke=true` with package
builds to run both the target-host packaged-binary startup probe and the native
session package smoke. Set
`run_desktop_installer_smoke=true` with package builds to run install/uninstall
smoke. A startup, native-session, or installer smoke request without
`run_desktop_package_build=true` is an invalid Desktop target-host request and
must fail closed. Each target-host job uploads a validated
`deve-native-target-host-evidence-*` artifact.
The workflow installs pinned Trunk and Tauri CLI release binaries through
`scripts/install-native-target-host-tools.sh`; it must not compile those tools
from source on every target-host run.

CLI dispatch helper:

```bash
scripts/dispatch-native-target-host-workflow.sh
DEVE_NATIVE_TARGET_HOST_DISPATCH=1 DEVE_NATIVE_TARGET_HOST_TARGET=desktop-macos DEVE_NATIVE_TARGET_HOST_REQUIRED_PREFLIGHT=true DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_PACKAGE_BUILD=true scripts/dispatch-native-target-host-workflow.sh
DEVE_NATIVE_TARGET_HOST_DISPATCH=1 DEVE_NATIVE_TARGET_HOST_TARGET=desktop-macos DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_PACKAGE_BUILD=true DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_INSTALLER_SMOKE=true scripts/dispatch-native-target-host-workflow.sh
DEVE_NATIVE_TARGET_HOST_DISPATCH=1 DEVE_NATIVE_TARGET_HOST_TARGET=desktop-macos DEVE_NATIVE_TARGET_HOST_REQUIRED_PREFLIGHT=true DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_PACKAGE_BUILD=true DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_STARTUP_SMOKE=true scripts/dispatch-native-target-host-workflow.sh
DEVE_NATIVE_TARGET_HOST_DISPATCH=1 DEVE_NATIVE_TARGET_HOST_TARGET=mobile-android DEVE_NATIVE_TARGET_HOST_REQUIRED_PREFLIGHT=true DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_PACKAGE_BUILD=true DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE=true scripts/dispatch-native-target-host-workflow.sh
DEVE_NATIVE_TARGET_HOST_DISPATCH=1 DEVE_NATIVE_TARGET_HOST_TARGET=desktop-windows DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_PACKAGE_BUILD=true DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_INSTALLER_SMOKE=true DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_REMOTE_BROWSER_SMOKE=true scripts/dispatch-native-target-host-workflow.sh
DEVE_NATIVE_TARGET_HOST_DISPATCH=1 DEVE_NATIVE_TARGET_HOST_TARGET=mobile-android DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_PACKAGE_BUILD=true DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE=true DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_REMOTE_BROWSER_SMOKE=true scripts/dispatch-native-target-host-workflow.sh
DEVE_NATIVE_TARGET_HOST_DISPATCH=1 DEVE_NATIVE_TARGET_HOST_TARGET=mobile-ios DEVE_NATIVE_TARGET_HOST_REQUIRED_PREFLIGHT=true DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_IOS_PACKAGE_BUILD=true DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_IOS_INSTALL_STARTUP_SMOKE=true scripts/dispatch-native-target-host-workflow.sh
```

Omitting the RemoteBrowser origin/username/HEAD-proof triplet selects the
internal exact-HEAD fixture and is the only tag-ready path. Supplying all three
`DEVE_NATIVE_TARGET_HOST_{DESKTOP|MOBILE_ANDROID}_REMOTE_*` values selects the
external diagnostic path; passwords are never dispatch inputs and remain GitHub
repository secrets.

The helper is dry-run by default and requires an authenticated GitHub CLI before
it can dispatch the manual workflow. If `gh` is unavailable, the helper can use
`DEVE_GITHUB_TOKEN`, `GH_TOKEN`, or `GITHUB_TOKEN` plus `curl`; set
`DEVE_NATIVE_TARGET_HOST_REPOSITORY=owner/repo` when the repository cannot be
derived from `origin`.

Collect and validate workflow evidence artifacts after a run completes:

```bash
scripts/collect-native-target-host-evidence.sh
DEVE_NATIVE_TARGET_HOST_STATUS=1 scripts/collect-native-target-host-evidence.sh
DEVE_NATIVE_TARGET_HOST_EVIDENCE_COLLECT=1 DEVE_NATIVE_TARGET_HOST_RUN_ID=<run-id> scripts/collect-native-target-host-evidence.sh
DEVE_NATIVE_TARGET_HOST_EVIDENCE_COLLECT=1 DEVE_NATIVE_TARGET_HOST_RUN_ID=latest scripts/collect-native-target-host-evidence.sh
```

The collector is dry-run by default. It validates every downloaded evidence
Markdown file with `scripts/check-native-target-host-evidence.sh`. It uses an
authenticated GitHub CLI when available, otherwise `DEVE_GITHUB_TOKEN`,
`GH_TOKEN`, or `GITHUB_TOKEN` plus `curl` and `unzip`. Use
`DEVE_NATIVE_TARGET_HOST_RUN_ID=latest` to resolve the most recent
`native-target-host.yml` workflow_dispatch run for the selected ref. Use
`DEVE_NATIVE_TARGET_HOST_STATUS=1` to inspect the current run status without
downloading artifacts.

## Platform Artifact Consumption

Use this section when the goal is to obtain and smoke-test current shell-only
platform artifacts. It records consumption workflow, not release readiness.

Dispatch a full shell-only target-host run:

```bash
DEVE_NATIVE_TARGET_HOST_DISPATCH=1 \
DEVE_NATIVE_TARGET_HOST_TARGET=all \
DEVE_NATIVE_TARGET_HOST_REQUIRED_PREFLIGHT=true \
DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_PACKAGE_BUILD=true \
DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_STARTUP_SMOKE=true \
DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_INSTALLER_SMOKE=true \
DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_PACKAGE_BUILD=true \
DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE=true \
DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_IOS_PACKAGE_BUILD=true \
DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_IOS_INSTALL_STARTUP_SMOKE=true \
scripts/dispatch-native-target-host-workflow.sh
```

After the run completes, validate evidence artifacts:

```bash
DEVE_NATIVE_TARGET_HOST_EVIDENCE_COLLECT=1 \
DEVE_NATIVE_TARGET_HOST_RUN_ID=<run-id> \
scripts/collect-native-target-host-evidence.sh
```

Download package artifacts when manual inspection is needed:

```bash
mkdir -p target/platform-artifacts
gh run download <run-id> --name deve-desktop-macos-packages --dir target/platform-artifacts/desktop-macos
gh run download <run-id> --name deve-desktop-windows-packages --dir target/platform-artifacts/desktop-windows
gh run download <run-id> --name deve-mobile-android-packages --dir target/platform-artifacts/mobile-android
gh run download <run-id> --name deve-mobile-ios-packages --dir target/platform-artifacts/mobile-ios
```

Artifact interpretation:

- Docker: validate with `DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh` or production compose with explicit `AUTH_SECRET` / `AUTH_PASS`.
- Desktop macOS: `.app/.dmg` evidence is shell-only; unsigned CI artifacts do not claim signing, notarization, or Gatekeeper release readiness.
- Desktop Windows: MSI/NSIS evidence covers package build, LocalBackend/NoteGit startup, and installer install/uninstall smoke. RemoteBrowser/native recovery runs against the NSIS installation as the shared Desktop runtime payload representative; it does not claim separate MSI installer-engine RemoteBrowser coverage. It does not claim signed installer readiness.
- Android: emulator evidence covers shell APK install/startup; it does not claim Play Store, signed release, or physical-device readiness.
- iOS: simulator evidence covers shell `.app` install/startup; it does not claim device signing, TestFlight, App Store, or physical-device readiness.

All platform artifacts remain shell-only. They must not open native
child-process runtime, backend supervision ownership, ledger/Projection Locator/source-control
authority, search authority, Git authority, or `.notegit` authority.

## Platform Signing / Physical-device Preflight

Use these gates before attempting signed Desktop artifacts, Android signed
release artifacts, or Android physical-device smoke. They validate prerequisite
shape only; they do not sign, install on physical devices, upload store
artifacts, open child-process runtime, or create native authority writes.

Desktop signing diagnostics:

```bash
scripts/check-desktop-signing-preflight.sh
DEVE_DESKTOP_SIGNING_TARGETS=macos scripts/check-desktop-signing-preflight.sh
DEVE_DESKTOP_SIGNING_TARGETS=windows scripts/check-desktop-signing-preflight.sh
```

Required mode fails closed on missing target-host tools or signing material:

```bash
DEVE_DESKTOP_SIGNING_PREFLIGHT_REQUIRED=1 DEVE_DESKTOP_SIGNING_TARGETS=macos scripts/check-desktop-signing-preflight.sh
DEVE_DESKTOP_SIGNING_PREFLIGHT_REQUIRED=1 DEVE_DESKTOP_SIGNING_TARGETS=windows scripts/check-desktop-signing-preflight.sh
```

macOS signing/notarization preflight checks `APPLE_SIGNING_IDENTITY`,
`APPLE_PROVIDER_SHORT_NAME`, and either
`APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID` or
`APPLE_API_KEY` / `APPLE_API_KEY_ID` / `APPLE_API_ISSUER`. Windows signing
preflight checks `WINDOWS_SIGNING_CERT_PATH` or
`WINDOWS_SIGNING_CERT_BASE64`, `WINDOWS_SIGNING_CERT_PASSWORD`, and `signtool`
on a Windows target host.

Android signed release and physical-device diagnostics:

```bash
scripts/check-mobile-android-release-preflight.sh
```

Required signing mode checks keystore material and key metadata without
printing secret values:

```bash
DEVE_MOBILE_ANDROID_RELEASE_PREFLIGHT_REQUIRED=1 scripts/check-mobile-android-release-preflight.sh
```

Required physical-device mode checks for a non-emulator `adb` target:

```bash
DEVE_MOBILE_ANDROID_PHYSICAL_DEVICE_PREFLIGHT_REQUIRED=1 scripts/check-mobile-android-release-preflight.sh
DEVE_MOBILE_ANDROID_SERIAL=<adb-serial> DEVE_MOBILE_ANDROID_PHYSICAL_DEVICE_PREFLIGHT_REQUIRED=1 scripts/check-mobile-android-release-preflight.sh
```

Android signing preflight accepts `DEVE_ANDROID_KEYSTORE_PATH` or
`DEVE_ANDROID_KEYSTORE_BASE64`, plus `DEVE_ANDROID_KEY_ALIAS`,
`DEVE_ANDROID_KEYSTORE_PASSWORD`, and `DEVE_ANDROID_KEY_PASSWORD`.
`DEVE_MOBILE_ANDROID_RELEASE_ARTIFACT_KIND=apk|aab` records the intended
artifact kind; it does not build or upload the artifact.

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

Android gates resolve SDK tools from `PATH`, `ANDROID_HOME`, or
`ANDROID_SDK_ROOT`. On Windows Android Studio installs where `adb` is available
but `emulator` is not on `PATH`, the scripts also check
`$ANDROID_HOME/emulator/emulator.exe`. If the shell `java` resolves to an older
JRE, the gates use Android Studio's bundled JBR when present, for example the
scoop path `~/scoop/apps/android-studio/current/jbr`.

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
For emulator install/startup smoke, build an installable debug APK:

```bash
DEVE_MOBILE_ANDROID_PACKAGE_BUILD_REQUIRED=1 DEVE_MOBILE_ANDROID_PACKAGE_DEBUG=1 scripts/check-mobile-android-shell-package-build.sh
```

The required Android package gate rebuilds `apps/web/dist` before any
`native-packaging` Cargo check/test or Tauri/Gradle invocation. Tauri validates
`frontendDist` while compiling its context, so the preflight compile surface
must consume the newly built current-HEAD dist. A pre-existing dist directory
is never accepted as evidence for the packaged WebView shell, because it may
not contain the current capability gates or DOM smoke markers.

Cold Android WebView startup can recycle its first renderer while the system
settles memory pressure. The CDP helper bounds each discovery command and
retries the page within the lifecycle deadline; it does not extend that
deadline or weaken WebCrypto and business-flow assertions.

The debug graceful-exit command may retire its CDP target before its invoke
response returns. The harness accepts only that narrow transport race and then
requires bounded process disappearance plus the clean-shutdown logcat marker;
all command errors and failed-shutdown markers remain failures.

iOS shell-only package execution is a separate explicit gate:

```bash
scripts/check-mobile-ios-shell-package-build.sh
```

By default it runs boundary/preflight checks and does not build. On a macOS
target host, set `DEVE_MOBILE_IOS_PACKAGE_BUILD_REQUIRED=1` to allow
`cargo tauri ios init` and `cargo tauri ios build --target aarch64-sim` under
`apps/mobile/native-packaging`. The CI/default target is `aarch64-sim` to avoid
Apple signing requirements. Signed device IPA builds require a later signing
gate and must use `DEVE_MOBILE_IOS_PACKAGE_TARGET=aarch64` with signing material.
This still does not open child-process runtime or native authority writes. iOS
device/simulator install and startup smoke remain a later independent gate.

Target-host handoff commands:

```bash
DEVE_MOBILE_ANDROID_PACKAGE_BUILD_REQUIRED=1 scripts/check-mobile-android-shell-package-build.sh
```

```bash
DEVE_MOBILE_IOS_PACKAGE_BUILD_REQUIRED=1 scripts/check-mobile-ios-shell-package-build.sh
```

Capture Android/iOS artifacts or missing prerequisites under `docs/report/`,
together with the same Mobile child-process-closed / native-authority-closed
boundary.

Mobile install/startup smoke is separate from package build evidence:

```bash
scripts/check-mobile-android-install-startup-smoke.sh
scripts/check-mobile-android-emulator-install-startup-smoke.sh
scripts/smoke-mobile-android-lifecycle.sh
scripts/check-mobile-ios-install-startup-smoke.sh
```

Both scripts are diagnostic-only by default. Required Android mode needs `adb`,
an attached emulator/device, and an installable APK. The default install-smoke
APK path points to the debug APK output. Use
`DEVE_MOBILE_ANDROID_APK_PATH=/path/to/signed.apk` when using another signed APK.
Use `DEVE_MOBILE_ANDROID_SERIAL=<adb-serial>` when more than one Android
emulator/device is attached. Android `adb` calls are bounded by
`DEVE_MOBILE_ANDROID_ADB_TIMEOUT_SECS`:

```bash
DEVE_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE_REQUIRED=1 scripts/check-mobile-android-install-startup-smoke.sh
```

GitHub-hosted emulator orchestration uses the wrapper script below; it installs
SDK packages, creates/boots a lean `default/x86_64` AVD, builds an `x86_64`
debug APK, then delegates to the Android install/startup smoke. The wrapper uses
an isolated `ANDROID_AVD_HOME` under `target/` and uploads emulator diagnostics
with the target-host evidence artifact.

```bash
DEVE_MOBILE_ANDROID_EMULATOR_INSTALL_STARTUP_SMOKE_REQUIRED=1 scripts/check-mobile-android-emulator-install-startup-smoke.sh
```

The emulator gate now keeps the debug APK installed after its marker startup
probe and runs `scripts/smoke-mobile-android-lifecycle.sh` against the same
artifact. The lifecycle harness forwards the app's debug WebView socket to a
random host CDP port, then uses raw page-target CDP from
`scripts/smoke-mobile-android-lifecycle.mjs` to
verify native-session startup, create/edit/commit, a real non-zero pending
overlay, HOME background read-only, debug-only transport fault injection,
replacement random endpoint/session generation, foreground
auth/node-role/WS/scope reprobe, pending replay, a second edit/commit, and
bounded graceful app/runtime exit. It does not call ledger or Source Control
authority APIs directly; the fault-injection and exit commands are unavailable
in release builds.

The writable lifecycle path records Android SDK level, the current WebView
provider package/full version, AVD name or device fingerprint, and model before
installation. It requires Android 10/API 29+ and provider major 137+ as the
support qualification, then separately requires the actual non-extractable
WebCrypto Ed25519 key-generation probe. A frozen AOSP image without that
capability remains storage-limited/read-only by contract; the default writable
smoke fails with the capability reason instead of waiting indefinitely or
bypassing browser peer identity. LocalBackend installs its HttpOnly native-session cookie
through a no-argument Tauri command backed by Android CookieManager because the
current Wry Android `set_cookie` surface is unsupported; cookie material never
enters JavaScript or command arguments.

The emulator gate defaults to the Android 17 / API 37.1 Google APIs 16 KB
x86_64 image as the current target-host candidate. Chromium introduced WebCrypto
Ed25519 in 137; the currently exercised image carries a newer System WebView.
That default is only a reproducible input:
the lifecycle harness still proves non-extractable Ed25519 key generation at
runtime before any create/edit/commit step. Override the image when needed;
never treat the API level or image tag as capability evidence.

With an already booted emulator and built debug APK, run the narrower gate:

```bash
DEVE_MOBILE_ANDROID_SERIAL=emulator-5554 DEVE_MOBILE_ANDROID_LIFECYCLE_SMOKE_REQUIRED=1 scripts/smoke-mobile-android-lifecycle.sh
```

Capture an AOSP 124/133-style read-only negative receipt by selecting the
explicit negative mode. This mode succeeds only when the real Ed25519 probe is
unavailable and the product visibly remains read-only; it cannot satisfy the
tag-ready writable receipt:

```bash
DEVE_MOBILE_ANDROID_SERIAL=emulator-5554 \
DEVE_MOBILE_ANDROID_LIFECYCLE_SMOKE_REQUIRED=1 \
DEVE_MOBILE_ANDROID_EXPECT_WRITABLE=0 \
DEVE_MOBILE_ANDROID_TARGET_FACTS_PATH=target/android-smoke/readonly-target.json \
scripts/smoke-mobile-android-lifecycle.sh
```

For writable evidence, leave `DEVE_MOBILE_ANDROID_EXPECT_WRITABLE=1` (the
default), set `DEVE_MOBILE_ANDROID_TARGET_FACTS_PATH`, and set
`DEVE_MOBILE_ANDROID_EVIDENCE_PATH` to a worktree-external claims JSON. Pass the
same evidence path to `deve_baseline acceptance-receipt --claims`; tag-ready
validates the bound smoke producer, current provider, real probe, and completed
writable lifecycle instead of accepting an arbitrary exit-zero command.

For Android `RemoteBrowser`, prebuild the same debug APK and expose a temporary
HTTPS Deve origin with random test credentials. The harness writes the remote
preference into app-private storage before launch, verifies that no embedded
LocalBackend/native facade/`ipc.localhost` request appears, then runs login,
edit, commit/history and HOME/resume before emitting the separate remote claims:

```bash
DEVE_MOBILE_ANDROID_REMOTE_SMOKE_REQUIRED=1 \
DEVE_MOBILE_ANDROID_SERIAL=emulator-5554 \
DEVE_MOBILE_ANDROID_REMOTE_HTTPS_ORIGIN=https://deve.example \
DEVE_MOBILE_ANDROID_REMOTE_USERNAME=test-user \
DEVE_MOBILE_ANDROID_REMOTE_PASSWORD=test-password \
DEVE_MOBILE_ANDROID_TARGET_FACTS_PATH=/tmp/android-remote-target.json \
DEVE_MOBILE_ANDROID_EVIDENCE_PATH=/tmp/android-remote-claims.json \
scripts/smoke-mobile-android-remote-browser.sh
```

The HTTPS instance and credentials are test-only and must be retired after the
receipt run. The smoke uses a preference-driven RemoteBrowser launch, invokes
the platform-native recovery control, and binds the resulting receipt to the
typed recovery transition, CDP surface retirement, fresh local scope/session,
and graceful process cleanup. CLI/environment remote overrides intentionally
hide the recovery control and cannot satisfy this journey.

Required iOS mode needs macOS, a built simulator `.app`, and a booted simulator:

```bash
DEVE_MOBILE_IOS_INSTALL_STARTUP_SMOKE_REQUIRED=1 scripts/check-mobile-ios-install-startup-smoke.sh
```

The narrower install/startup gates open only the WebView shell. The lifecycle
smoke additionally starts the embedded LocalBackend supervisor and exercises
normal UI intent paths, but it does not call ledger/Projection Locator,
Source Control, Git, or `.notegit` authority APIs directly.

Optional GitHub Actions entry:

```text
Native Target Host -> target=mobile-android
Native Target Host -> target=mobile-ios
```

The workflow is manual-only. It runs Android/iOS preflight by default. Android
package execution runs only when `run_mobile_android_package_build=true`.
Set `run_mobile_android_install_startup_smoke=true` together with package build
to start a GitHub-hosted Android emulator, build an `x86_64` debug APK, install
it, and launch the shell. It uploads `deve-native-target-host-evidence-android`
with emulator logs and, when package build is requested,
`deve-mobile-android-packages`. The Android job skips host Linux
native-packaging cargo checks because the target evidence is
`cargo tauri android build` plus emulator install/startup, not a desktop
Wry/GTK host build.

It runs iOS preflight by default and only runs
`cargo tauri ios init` / `cargo tauri ios build` when
`run_mobile_ios_package_build=true`. It uploads a validated
`deve-native-target-host-evidence-ios` artifact and, when package build is
requested, a `deve-mobile-ios-packages` artifact. Set
`run_mobile_ios_install_startup_smoke=true` together with package build to boot a
simulator, install the generated `.app`, and launch the shell.

## Native Process Adapter Gate

Validate that the native process adapter remains default-closed and that opt-in
runtime paths stay behind explicit gates:

```bash
scripts/check-native-process-adapter-gate.sh
```

The script checks the process adapter policy, rejects unauthorized
`std::process`, `Command::new`, `tokio::process`, or direct spawn usage in
Desktop/Mobile/native adapter code, and runs targeted process-observation tests.
It validates both native shell modes without granting native shell direct
authority writes.

## Native Shell Modes

Desktop/Android/Mobile native-packaging shells support two mutually exclusive
modes:

- `LocalBackend`: default. Desktop starts a controlled child-process local
  service. Android/Mobile starts an in-process embedded loopback service. Both
  initialize app-private ledger/repo/projection state through the server/CLI
  runtime and keep all writes behind the server/core writer gate.
- `RemoteBrowser`: explicit. The shell acts like a browser and loads a remote
  Docker/Web HTTPS origin. It does not start the local backend and does not
  inject native endpoint/session bootstrap.

Use the default environment for LocalBackend. For RemoteBrowser, pass a
validated HTTPS origin at startup:

```bash
deve_desktop --remote-url https://example.invalid
```

Packaged/scripted launches may also use:

```bash
DEVE_NATIVE_REMOTE_URL=https://example.invalid
```

The URL must be exactly an HTTPS origin: no userinfo, query, fragment, or
application subpath.

Desktop LocalBackend does not depend on an externally running server on port
3001. The Tauri shell loads bundled Web assets and gets the child-process
endpoint only through native bootstrap.

Its default data root is the platform app-private data directory, not the
current working directory. Use `DEVE_DESKTOP_DATA_DIR=<absolute path>` only for
diagnostic or smoke-test isolation.

LocalBackend runtime enablement and native shell direct-authority experiments
are separate knobs. `DEVE_DESKTOP_LOCAL_SERVICE=0` disables the Desktop
LocalBackend runtime; `DEVE_NATIVE_AUTHORITY=0` does not. Direct native
authority remains off by default and only participates in explicit test gates.

Desktop LocalBackend starts a controlled child-process local service.
Mobile LocalBackend starts an in-process embedded loopback service; Mobile v1
must not use a child process. In both cases the shell handles
endpoint/session/readiness only.
Native LocalBackend CORS must allow both Tauri origin forms:
`http://tauri.localhost` for Windows/Android and `tauri://localhost` for
macOS/iOS/Linux.
Ledger, source-control, search, repo writes, `.git`, and `.notegit` writes still
go through the local server/core writer gate.

Diagnostics:

```bash
scripts/check-native-process-adapter-gate.sh
scripts/check-native-packaging-gate.sh
cargo test -p deve_desktop --features native-packaging -- --nocapture
cargo test -p deve_mobile --features native-packaging -- --nocapture
```

Do not record service ports, session secrets, P2P token material, or bootstrap
secrets in URLs, logs, Web localStorage, persistent config, or crash reports.

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

Use the local quick gate before handing off a normal implementation batch:

```bash
scripts/check-local-quick-gate.sh
DEVE_QUICK_GATE_TESTS=0 scripts/check-local-quick-gate.sh
DEVE_LOCAL_QUICK_GATE_TARGET_DIR=target/local-quick-gate-alt scripts/check-local-quick-gate.sh
```

The quick gate runs diff hygiene, `deve_core`/`deve_cli` checks, focused
governance checks, focused projection-locator tests, and Source Control
HTTP/WS scope-gate tests. The second form keeps only the compile/governance
subset for very small doc-only changes. The quick gate uses an isolated Cargo
target directory by default so it can run while a development `deve_cli` server
is holding `target/debug/deve_cli.exe`.

Use the deep audit gate for broad architecture changes, release-prep, or after
several related batches have landed:

```bash
scripts/check-deep-audit-gate.sh
DEVE_DEEP_AUDIT_WRITE_REPORT=1 scripts/check-deep-audit-gate.sh
DEVE_DEEP_AUDIT_FULL_TESTS=1 scripts/check-deep-audit-gate.sh
DEVE_DEEP_AUDIT_DOCKER_SMOKE=1 scripts/check-deep-audit-gate.sh
```

The deep gate runs the plan/architecture governance suite, baseline scripts,
runtime happy/recovery smokes, and optional plan report, full Cargo, or Docker
verification.

Current docs/code guard scripts:

```bash
scripts/check-local-quick-gate.sh
scripts/check-deep-audit-gate.sh
scripts/check-acceptance-matrix.sh
scripts/check-auth-baseline.sh
scripts/check-auth-unauthorized-state.sh
scripts/check-network-baseline.sh
scripts/check-foundation-baseline.sh
scripts/check-cli-settings-baseline.sh
scripts/check-settings-local-feedback-baseline.sh
scripts/check-browser-prefs-boundary.sh
scripts/check-storage-repo-baseline.sh
scripts/check-search-baseline.sh
scripts/check-rendering-baseline.sh
scripts/check-ai-baseline.sh
scripts/check-feature-operation-paths.sh
scripts/check-i18n-hardcoded-baseline.sh
scripts/check-i18n-formatting-baseline.sh
scripts/check-source-control-baseline.sh
scripts/check-repo-file-ops-baseline.sh
scripts/check-source-control-smoke-hygiene.sh
scripts/check-dev-data-health-baseline.sh
scripts/check-perf-budget-baseline.sh
scripts/check-reliability-observability-baseline.sh
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
scripts/smoke-docker-multiclient.sh
scripts/smoke-docker-p2p-mesh.sh
```

Use full-suite checks as release/final verification, not as the default inner
loop on a low-memory machine:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
