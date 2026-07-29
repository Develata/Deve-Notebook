# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Deve Notebook — a self-hosted collaborative Markdown notebook. Rust workspace (Edition 2024, toolchain pinned to 1.97.0 via `rust-toolchain.toml`) targeting low-resource deployment (768 MB VPS); every new dependency must be weighed for memory footprint.

This repo also carries a hierarchy of `AGENTS.md` files (root plus per-directory). They are authoritative agent instructions: read the nearest applicable `AGENTS.md` before editing a subdirectory; narrower files override broader ones. The root `AGENTS.md` defines the full "User-Requested Implementation Workflow" (docs-first ordering, review-subagent step, Chrome MCP verification, commit gate) that applies to any work item that may change code or docs.

## Commands

```bash
# Targeted test (preferred over full suite)
cargo test --package <pkg> --lib <test_fn> -- --nocapture

# Full suite / lint / format
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --check

# WASM frontend type-check
cargo check --locked -p deve_web --target wasm32-unknown-unknown

# Baseline contract checker (developer/release gates)
cargo run --quiet -p deve_baseline -- all

# Plan/docs/code contract checks
bash scripts/plan-coverage.sh --check-reverse-coverage
bash scripts/plan-coverage.sh --check-md-links docs/plan docs/features docs/acceptance-cases
```

Run the server locally:

```bash
cargo run -p deve_cli --bin deve_cli -- init --path . --repo default --projection-base notes
cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001   # dev login: admin / admin
```

Frontend iteration (backend + Trunk separately, then open `http://127.0.0.1:8080/`):

```bash
cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001
cd apps/web && NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080
```

Scripts under `scripts/` assume a POSIX shell (Git Bash on Windows). Production mode (no `--dev`) fail-closes without `AUTH_SECRET` (≥32 bytes) and `AUTH_PASS` (Argon2 PHC hash).

Do not "fix" the dev/test profile in the root `Cargo.toml` (`debug = 1`, `incremental = false`): it works around Windows MSVC linker OOM (LNK1102) and WSL2/DrvFS incremental-artifact corruption.

## Architecture

### Ledger-first authority model (the core invariant)

```text
Ledger -> Folded State -> Projection -> Projection Workspace
```

- The Redb-backed ledger (`ledger/`) is the **only** authority for repo facts. User-visible Markdown workspaces are repo-scoped projections of it.
- External file changes, UI state, Git mirrors, and browser pending overlays are **not** authority. Filesystem changes enter `pending_fs_ops` first and mutate nothing until an explicit stage/commit path appends ledger facts.
- `.notegit/` is Deve-owned runtime state. `.git/` is only a Git ecosystem bridge — Git is mirror/import/export/publish around Deve's own source control, never authority.
- `ledger/.host/projection-locators.toml` binds `RepoId -> (projection_base, workspace_segment)` host-locally.
- Fail-closed semantics throughout: a `doc_id` miss must not fall back to path-only resolution.

### Workspace members

| Crate | Path | Role |
|-------|------|------|
| `deve_core` | `crates/core` | All business logic: ledger, projection, sync, source control, security, config, plugins, search, protocol |
| `deve_cli` | `apps/cli` | Clap commands + Axum/Tokio HTTP + WebSocket server, embedded frontend delivery |
| `deve_web` | `apps/web` | Leptos CSR WASM frontend (thin shell) |
| `deve_desktop` / `deve_mobile` | `apps/desktop`, `apps/mobile` | Native shells with optional Tauri v2 `native-packaging` feature gates |
| `deve_baseline` | `tools/baseline` | Developer/release contract checker CLI |

- **The frontend is a thin shell**: render UI, collect user intent, dispatch typed intents. Computation, state transitions, ledger/source-control mutations, diff/external-change decisions, and commit-anchor judgment belong in `deve_core`/`deve_cli`. Never move that judgment into the frontend for UI convenience.
- Native shells must not write ledger/projection/source-control/search authority directly.
- Do not bypass authority, runtime boundaries, writer gates, or Object Plane adapters to ship faster — if the required path crosses those boundaries, update the plan/registry contract first or stop for a USER decision.

### Cross-cutting invariants

- UUID-first identity: repos and docs are identified by UUID; display names/aliases are host-local and never synchronized between peers.
- All server→client messages are repo-scoped: they carry `repo_id`, `branch`, `scope_nonce`.
- `PersistGuard` is shared between `RepoManager` and `SyncManager` to prevent watcher storms.
- Path handling must use `deve_core::utils::path::to_forward_slash` (Windows compatibility).
- Error handling: `anyhow` in the app layer, `thiserror` in library code.

## Docs are the source of design authority

`docs/plan/` is the authoritative engineering blueprint; **code is a projection of docs**. For behavior-affecting changes, work in this order: `docs/plan/` → `docs/` (features/acceptance/registry) → code.

- Before implementation work, read `docs/plan/00_engineering_constitution.md` and `docs/plan/01_terminology.md`; use `docs/coverage-matrix.md` to locate the matching plan/features/acceptance chapters.
- Code that disagrees with a current plan invariant is implementation drift by default: align code to plan or record explicit drift/registry evidence — do not weaken the plan because the code already exists.
- Every non-infra Rust module implementing plan behavior carries a `//! plan_ref:` header pointing at stable `docs/plan/` anchors; new anchors go into the plan before code references them.
- Docs layers: `docs/plan/` = how it's engineered (authoritative); `docs/features/` = user-visible behavior + Chrome MCP walkthroughs; `docs/acceptance-cases/` = automated proof; `docs/registry/` = where plan concepts currently live in code; `docs/report/` = dated evidence, not live contracts; `docs/adr/` = decision history, never a `plan_ref` target.
- `docs/overview/` artifacts (`architecture.svg`, `.dot`, `.lisp` files) are generated — do not hand-edit or locally re-render them.

## File-size discipline

Prefer cohesive files; split by responsibility or API boundary, not to satisfy a line count. ~250 lines is a soft architecture warning; hand-written source files over ~500 lines are hard fuse violations unless explicitly justified. Tests may exceed the soft threshold to keep scenario context together.
