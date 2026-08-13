English | [中文](README.zh.md)

# Deve Notebook

[![Check](https://github.com/Develata/Deve-Notebook/actions/workflows/check.yml/badge.svg)](https://github.com/Develata/Deve-Notebook/actions/workflows/check.yml)
[![Release](https://github.com/Develata/Deve-Notebook/actions/workflows/release.yml/badge.svg)](https://github.com/Develata/Deve-Notebook/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

Deve Notebook is a Rust workspace for a self-hosted collaborative Markdown
notebook. It targets private, low-resource deployments and uses a ledger-first
storage model: the ledger is the authority, and every visible Markdown
workspace is a repo-scoped projection.

The workspace version is `0.1.0`. This repository is suitable for engineering
validation, source review, and Docker-oriented preview usage. It is not yet a
polished end-user application, hosted SaaS product, or signed native app
release.

## What Works Today

- Rust workspace with `deve_core`, `deve_cli`, `deve_web`, `deve_desktop`,
  `deve_mobile`, and the developer checker crate `deve_baseline`.
- Clap/Tokio/Axum CLI server with HTTP, WebSocket, authentication, runtime
  status, admin diagnostics, and embedded frontend delivery.
- Leptos CSR Web frontend with login/session handling, document operations,
  command surfaces, source-control UI, merge/conflict flows, read-only graph
  views, settings surfaces, and i18n coverage.
- Ledger-backed repo state, repo-scoped projection workspaces, external file
  watcher ingestion, stage/commit/discard/merge workflows, and projection health
  diagnostics.
- Repo-scoped sync protocol with browser WebLightPeer identity, scope nonce
  gates, structured protocol errors, and recovery paths.
- Production auth fail-closed behavior using `AUTH_SECRET` and `AUTH_PASS`;
  `--dev` mode provides the local `admin` / `admin` login only for development.
- Dockerfile, production `docker-compose.yml`, embedded Web release build
  smoke, runtime smoke scripts, release/baseline guards, and architecture
  registry checks.
- Desktop and mobile native shell crates with optional Tauri v2
  `native-packaging` gates. Native shells default to LocalBackend and can
  switch to a validated RemoteBackend HTTPS origin from Settings. Current
  evidence is shell/package/startup oriented, not signed store readiness.

## Explicit Boundaries

The current release does not claim:

- hosted multi-tenant SaaS;
- browser offline-first full local ledger;
- general-purpose server-backed Settings API beyond the scoped Native AI provider surface;
- default full-text indexing;
- high-performance graph rendering;
- product runtime MCP integration;
- general plugin marketplace or arbitrary plugin authority;
- default trusted external agent execution;
- Web Git writer or Git authority;
- signed desktop installers, app-store readiness, physical-device readiness,
  native authority writes, Mobile process runtime, or Android process runtime.

Git remains a mirror/import/export/publish bridge around Deve's own
source-control authority. The ledger and `.notegit/` remain Deve-owned runtime
state.

As a design preference, cross-host machine state and host-local human
interaction are kept maximally independent. Peers preserve immutable identity,
Ledger facts and Markdown fidelity; labels and visual preferences stay local
unless they are required for correctness. Repo aliases are the concrete case:
peers share `RepoId` and never synchronize aliases. The approved first-release
target lets users explicitly move their local alias map with deterministic JSON;
the C1′ runtime is implemented, while first-tag readiness remains blocked until
the producer-bound acceptance evidence is sealed.

## Authority Model

```text
Ledger -> Folded State -> Projection -> Projection Workspace
```

- `ledger/` stores authoritative repo facts.
- `ledger/.host/projection-locators.toml` stores host-local
  `RepoId -> (projection_base, immutable workspace_segment)` bindings.
- `<projection_base>/<workspace_segment>/` stores the user-visible
  Markdown projection for one local repo.
- The repo-alias runtime keeps display state host-local. Changing or importing
  an alias never moves the workspace or changes peer identity.
- File-system changes enter `pending_fs_ops` first. They do not mutate
  authority until an explicit stage/commit path appends ledger facts.
- `.notegit/` is Deve-owned repo runtime state.
- `.git/` is only a Git ecosystem bridge.

The authoritative design source is `docs/plan/`. Feature docs and acceptance
cases refine it. Reports under `docs/report/` are dated evidence, not live
contracts.

### Remote Projection / Projection Backup

Remote Projection / Projection Backup transports only Markdown Projection
Workspace files through WebDAV/S3. It is not ledger-history backup, realtime
sync, Source Control authority, or Git mirror authority.

S3-compatible custom endpoints use a long-term credential-binding design: a
host-local, secret-free Remote Projection profile binds endpoint origin, bucket,
allowed prefix, signing settings, and a credential reference. Raw access keys,
secret keys, and session tokens must not be stored in repo metadata, locator
strings, browser state, normal logs, or README examples. Until that profile
runtime is implemented and verified, `s3+https://` custom endpoint I/O remains
fail-closed and default `AWS_*` environment credentials must not be signed to an
arbitrary custom host.

## Repository Layout

| Path | Purpose |
| --- | --- |
| `crates/core` | Ledger, projection, sync, source control, security, config, plugin boundaries |
| `apps/cli` | CLI commands and Axum/Tokio HTTP + WebSocket server |
| `apps/web` | Leptos CSR browser frontend |
| `apps/desktop` | Desktop native shell and Tauri packaging gate |
| `apps/mobile` | Mobile native shell and Android/iOS packaging gates |
| `tools/baseline` | Rust developer/release checker CLI |
| `docs/plan` | Authoritative engineering blueprint |
| `docs/features` | User-facing feature and operation specifications |
| `docs/acceptance-cases` | Acceptance and regression case registry |
| `docs/overview` | Architecture maps and drift registry |
| `docs/report` | Historical reports and smoke evidence |
| `scripts` | Build, smoke, target-host, and boundary checks |

## Prerequisites

Main development path:

- Rust 1.97.0 (pinned by `rust-toolchain.toml`) with Edition 2024 support.
- `wasm32-unknown-unknown` target for Web checks.
- Node.js 24 and npm for CI parity.
- Trunk for the WebAssembly frontend.
- Git.
- POSIX-like shell for `scripts/*.sh`; Git Bash is the usual Windows path.

Optional paths:

- Docker / Docker Compose for container smoke tests.
- Tauri CLI and platform packaging tools for Desktop/Mobile target-host checks.
- Android Studio / Android SDK for Android emulator/package checks.
- Xcode on macOS for iOS simulator/package checks.

## Quick Start

```bash
git clone https://github.com/Develata/Deve-Notebook.git
cd Deve-Notebook
bash scripts/smoke-web-release-build.sh
cargo run -p deve_cli --bin deve_cli -- init --path . --repo default --projection-base notes
cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001
```

Open:

```text
http://127.0.0.1:3001/
```

Development login:

```text
username: admin
password: admin
```

For frontend iteration, run the backend and Trunk separately:

```bash
cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001
cd apps/web
NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080
```

Then open `http://127.0.0.1:8080/`.

### Desktop Native Packaging

Desktop `native-packaging` defaults to `LocalBackend`: the Tauri shell loads the
bundled Web assets and starts a controlled sibling `deve_cli serve
--native-loopback` local service. It does not depend on an externally running
server on port `3001`.

For a local debug run, make sure the sibling CLI binary exists first:

```bash
cargo build -p deve_cli --bin deve_cli
cargo run -p deve_desktop --features native-packaging
```

LocalBackend runtime data uses the platform app-private data directory. Use
`DEVE_DESKTOP_DATA_DIR=<absolute-path>` only for diagnostic or smoke-test
isolation.

To use Desktop as a remote HTTPS Web shell instead of starting the local
backend:

```bash
cargo run -p deve_desktop --features native-packaging -- --remote-url https://example.invalid
```

Packaged or scripted launches may also use
`DEVE_NATIVE_REMOTE_URL=https://example.invalid`. RemoteBrowser URLs must be
HTTPS origins: no userinfo, query, fragment, or application subpath.

Linux native Desktop packages are a deferred first-tag TODO. The current Tauri
v2 Linux stack still resolves through GTK3/WebKitGTK 4.x dependencies; the first
formal tag should use Web / Server / Docker delivery on Linux instead of
publishing `.deb`, `.rpm`, or `.AppImage` Desktop artifacts. Re-enable Linux
native artifacts only after the shell stack is upgraded or replaced with a
maintained GTK4/WebKitGTK 6-compatible Tauri/Wry route or equivalent maintained
WebView route, followed by refreshed Linux package/startup/native-session
evidence.

In the native app, Settings exposes a Backend section:

- Local Backend starts the app-owned local service automatically.
- Remote Backend requires an HTTPS origin and must pass a native
  `<origin>/api/node/role` probe before it can be saved.
- The saved choice is host-local app-private JSON, not `config.toml`, ledger
  state, Projection Locator state, or browser localStorage.
- Remote credentials and login state remain owned by the remote Web origin.
  If the remote becomes unavailable, the native lock/read-only surface can
  switch back to Local Backend.

### Mobile Native Packaging

Mobile `native-packaging` uses the same Backend settings contract as Desktop.
Local Backend starts the embedded loopback service inside the mobile shell and
does not require an external server on port `3001`. Remote Backend loads only a
validated HTTPS origin and does not inject local session/bootstrap data.

The mobile Tauri bundle loads bundled `frontendDist` assets for production
shell runs; the backend is selected by native launch options or the host-local
Backend preference.

## Configuration

Start from `config.example.toml`.

Important local keys:

- `ledger_dir`: local ledger/runtime storage.
- `repo_creation_projection_base`: optional absolute base used only to create the first repo on a zero-repo host; startup remains valid when omitted.
- `profile`: `standard` or `low-spec`.
- `sync_mode`: `auto` or `manual`.
- `merge_strategy`: `manual` or `auto`.
- `snapshot_depth`: retained snapshot depth.
- `mem_cache_mb`: runtime cache budget.

Production mode is used when `--dev` is absent and `DEVE_ENV` is not
`development`. Production startup requires:

- `AUTH_SECRET`: JWT signing secret, at least 32 bytes.
- `AUTH_PASS`: Argon2 PHC password hash.
- `AUTH_USER`: optional username, defaults to `admin`.

## CLI

```bash
cargo run -p deve_cli --bin deve_cli -- <command>
```

Common commands:

| Command | Purpose |
| --- | --- |
| `init --path <path> --repo <name> --projection-base <path>` | Initialize ledger, repo, and Projection Locator |
| `repo projection set --repo <selector> --base <path>` | Set a repo Projection Locator |
| `repo projection list/check` | Inspect repo Projection Locators |
| `scan` | Scan repo projection workspaces |
| `watch [--dry-run]` | Watch projection workspace changes and record pending candidates |
| `serve [--dev] [--port <port>]` | Start HTTP/WebSocket backend |
| `export` | Export ledger data as JSON or Markdown |
| `graph` | Print read-only graph projection |
| `node-check` | Inspect repo/projection health |
| `repair --check` | Run repair readiness checks |
| `sc-status` | Print Deve source-control counts |
| `ngit status/mirror/export/import/push` | Inspect and operate the NoteGit Git main mirror |
| `config print/set` | Inspect or update whitelisted config keys |

## Docker

Production compose uses the published image:

```bash
docker compose up -d
```

Required environment:

```bash
AUTH_SECRET=<32-plus-byte-random-secret>
AUTH_PASS='<argon2-phc-password-hash>'
AUTH_USER=admin
```

Local Docker release smoke:

```bash
DEVE_DOCKER_SMOKE_REQUIRED=1 bash scripts/smoke-docker-release.sh
```

The image serves a single embedded frontend binary and stores runtime data under
mounted `/data` and `/notes` paths. Projection roots are still configured
through Projection Locators; `/notes` is not global authority.

Native AI provider settings can be configured after login through
`AI: Settings`; this writes only `/data/ai.env`. To manage AI settings through
the project-root `.env` instead, set `AI_PROVIDER`, `AI_BASE_URL`, `AI_API_KEY`,
`AI_MODEL`, or `AI_MAX_TOKENS` and restart Compose. Any such non-empty override
makes the in-app provider section read-only; the app never rewrites root `.env`.

## Verification

The branch workflow `.github/workflows/check.yml` is check-only: it runs
formatting, baseline contracts, plan coverage, clippy, WASM check, and tests.
It does not publish packages, push Docker images, upload artifacts, or deploy
production services.

Equivalent local checks:

```bash
cargo fmt --check
cargo run --quiet -p deve_baseline -- all
bash scripts/plan-coverage.sh --check-reverse-coverage
bash scripts/plan-coverage.sh --check-metadata-completeness
bash scripts/plan-coverage.sh --check-perf-budget
bash scripts/plan-coverage.sh --check-no-adr-plan-ref
bash scripts/plan-coverage.sh --check-md-links docs/plan docs/features docs/acceptance-cases
bash scripts/plan-coverage-selftest.sh
cargo clippy --locked --all-targets -- -D warnings
cargo check --locked -p deve_web --target wasm32-unknown-unknown
cargo test --locked
```

Release-oriented checks:

```bash
DEVE_RELEASE_AUDIT_REQUIRED=1 bash scripts/check-release-audit-gate.sh
DEVE_DOCKER_MULTI_REQUIRED=1 bash scripts/smoke-docker-multiclient.sh
DEVE_DOCKER_P2P_MESH_REQUIRED=1 bash scripts/smoke-docker-p2p-mesh.sh
```

Use full release and Docker smoke commands on machines that have the required
tooling. Missing Docker, Android, iOS, signing, or target-host tools should be
treated as a release evidence gap, not as a reason to weaken checks.

## Release Workflows

- `check.yml`: branch push / pull request checks only.
- `release-candidate.yml`: manual exact-HEAD quality, Docker/native target-host,
  SBOM, checksum and attestation candidate sealing.
- `acceptance-aggregate.yml`: exact-run receipt/candidate verification and
  immutable tag-ready bundle aggregation.
- `release.yml`: tag `v*` promotion only; it reuses the sealed bytes and does not
  rebuild or repackage them.
- `release-native.yml`: reusable pre-tag native build/smoke workflow. Native
  artifacts remain explicit platform evidence and do not imply notarization,
  store, or physical-device readiness.
- `native-target-host.yml`: manual target-host diagnostics and evidence
  collection.

Do not create or move release tags until branch CI is green and the intended
tag-triggered workflows are explicitly accepted.

## Documentation

- `docs/plan/deve-note plan.md`: blueprint index.
- `docs/plan/18_release.md`: release and CI/CD contract.
- `docs/overview/architecture.md`: architecture view.
- `docs/overview/architecture-diff.md`: current plan/code drift registry.
- `docs/features/operation-coverage.md`: operation coverage registry.
- `docs/acceptance-cases/00_index.md`: acceptance case index.
- `docs/dev-runbook.md`: startup, diagnostics, and release runbook.
- `docs/report/README.md`: report reading rules.

## License

Deve Notebook is released under the [MIT License](LICENSE).
