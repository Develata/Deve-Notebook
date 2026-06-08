English | [中文](README.zh.md)

# Deve Notebook

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

Deve Notebook is a Rust workspace for a self-hosted personal Markdown notebook.
It is built around a ledger-first storage model: the ledger is the authority,
and each visible Markdown workspace is a repo-scoped projection.

This repository is in active development. It is useful as an engineering
prototype with substantial implemented runtime paths and regression evidence,
but it is not yet a polished end-user release.

## Current Status

Implemented and exercised today:

- Rust workspace with `deve_core`, `deve_cli`, `deve_web`, `deve_desktop`, and
  `deve_mobile`.
- CLI/server runtime based on Clap, Tokio, Axum, HTTP, and WebSocket.
- Leptos CSR Web frontend with login/session handling, document operations,
  command surfaces, source-control UI, merge/conflict flows, graph/read-only
  views, settings surfaces, and i18n coverage.
- Ledger-backed local repo state, repo-scoped projection workspaces, watcher-to-pending external
  edit ingestion, stage/commit/discard/merge workflows, and projection health
  diagnostics.
- Repo-scoped sync protocol with browser WebLightPeer identity, scope nonce
  gates, structured protocol errors, and recovery paths.
- Production auth fail-closed behavior using `AUTH_SECRET` and `AUTH_PASS`;
  `--dev` mode provides the local `admin` / `admin` login only for development.
- Dockerfile, production `docker-compose.yml`, Web release build smoke, runtime
  smoke scripts, acceptance/baseline guards, and architecture registry checks.
- Desktop and mobile native shell crates using optional Tauri v2
  `native-packaging` features.
- Recent target-host evidence covers Windows Desktop no-sign MSI/NSIS package
  build/startup/installer smoke, Android shell APK emulator smoke, and iOS shell
  simulator smoke.

Not implemented or not claimed:

- No hosted multi-tenant SaaS mode.
- No browser offline-first full local ledger; browser is a WebLightPeer and
  depends on the server for authority.
- No server-backed Settings API; current settings are file/config/runtime
  surfaces.
- No default full-text index. Tantivy is optional and feature-gated.
- No high-performance graph renderer; the current graph path is read-only
  projection and summary/review UI.
- No MCP product runtime. MCP references are historical or developer-tooling
  related, not a runtime direction.
- No general plugin marketplace or arbitrary plugin authority. The current
  boundary is Rhai/plugin compatibility plus explicit capability gates.
- No default trusted external agent execution. Native AI chat exists, while the
  trusted CLI bridge is explicit and default-off.
- No Web Git writer and no Git authority. Git is a mirror/import/export/publish
  bridge around Deve's own source-control authority.
- No signing, app-store readiness, physical-device readiness, native authority
  writes, Mobile process runtime, or Android process runtime claim.

## Authority Model

```text
Ledger -> Folded State -> Projection -> Projection Workspace
```

- `ledger/` stores authoritative repo facts.
- `ledger/.host/projection-locators.toml` stores host-local
  `RepoId -> projection_base` bindings.
- `<projection_base>/<safe_repo_name>--<repo_id>/` stores the user-visible
  Markdown projection for one local repo; repo names are display aliases, while
  `RepoId` is the authority identity.
- File-system changes enter `pending_fs_ops` first; they do not mutate authority
  until an explicit stage/commit path appends ledger facts.
- `.notegit/` is Deve-owned repo runtime state.
- `.git/` is only a Git ecosystem mirror bridge.

The authoritative design source is `docs/plan/`. Feature docs and acceptance
cases refine it. Reports under `docs/report/` are dated evidence, not live
contracts.

## Repository Layout

| Path | Purpose |
| --- | --- |
| `crates/core` | Ledger, projection, sync, source control, security, config, plugin boundaries |
| `apps/cli` | CLI commands and Axum/Tokio HTTP + WebSocket server |
| `apps/web` | Leptos CSR browser frontend |
| `apps/desktop` | Desktop native shell and Tauri packaging gate |
| `apps/mobile` | Mobile native shell and Android/iOS packaging gates |
| `docs/plan` | Authoritative engineering blueprint |
| `docs/features` | User-facing feature and operation specifications |
| `docs/acceptance-cases` | Acceptance and regression case registry |
| `docs/overview` | Architecture maps and drift registry |
| `docs/report` | Historical reports and smoke evidence |
| `scripts` | Build, smoke, target-host, and boundary checks |

## Prerequisites

Main development path:

- Rust toolchain compatible with Edition 2024.
- Node.js and npm.
- Trunk for the WebAssembly frontend.
- Git.
- POSIX-like shell for `scripts/*.sh`; Git Bash is the usual Windows path.

Optional paths:

- Docker / Docker Compose for container smoke tests.
- Tauri CLI and platform packaging tools for Desktop/Mobile target-host checks.
- Android Studio / Android SDK for Android emulator/package checks.

## Quick Start

```bash
git clone https://github.com/develeta/deve-note.git
cd deve-note
scripts/smoke-web-release-build.sh
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

For UI iteration, run the backend and Trunk separately:

```bash
cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001
cd apps/web
NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080
```

Then open `http://127.0.0.1:8080/`.

## Configuration

Start from `config.example.toml`.

Important local keys:

- `ledger_dir`: local ledger/runtime storage.
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
| `git status/export/import/push` | Operate the Git mirror bridge |
| `config print/set` | Inspect or update whitelisted config keys |

## Verification

Targeted Rust test:

```bash
cargo test --package <pkg> --lib <test_fn> -- --nocapture
```

General checks:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Representative script gates:

```bash
bash scripts/check-foundation-baseline.sh
bash scripts/check-network-baseline.sh
bash scripts/check-source-control-baseline.sh
bash scripts/check-native-track-boundary.sh
bash scripts/check-release-baseline.sh
```

Docker smoke, only on Docker-capable hosts:

```bash
DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh
```

## Documentation

- `docs/plan/deve-note plan.md`: blueprint index.
- `docs/overview/architecture.md`: architecture view.
- `docs/overview/architecture-diff.md`: current plan/code drift registry.
- `docs/features/operation-coverage.md`: operation coverage registry.
- `docs/acceptance-cases/00_index.md`: acceptance case index.
- `docs/dev-runbook.md`: current startup, diagnostics, and release runbook.
- `docs/report/README.md`: report reading rules.

## License

Deve Notebook is released under the [MIT License](LICENSE).
