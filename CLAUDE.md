# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

Deve Notebook is a Rust workspace (Edition 2024, toolchain pinned by `rust-toolchain.toml`) for a self-hosted collaborative Markdown notebook targeting low-resource deployments (768 MB VPS — evaluate every new dependency for memory footprint). Storage is **ledger-first**: the ledger is the authority; every visible Markdown workspace is a repo-scoped projection.

This repo maintains a hierarchy of `AGENTS.md` files (root, `apps/`, `crates/`, `scripts/`, `tests/`, `docs/`, …). **Read the nearest `AGENTS.md` before editing a subdirectory** — narrower files override broader ones, and the root `AGENTS.md` defines the mandatory implementation workflow in full.

## Commands

```bash
# Targeted test (preferred over full suite)
cargo test --package <pkg> --lib <test_fn> -- --nocapture
cargo test -p deve_core          # or deve_cli / deve_desktop / deve_mobile

# Lint / format
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check

# Full CI-equivalent local checks
cargo run --quiet -p deve_baseline -- all
bash scripts/plan-coverage.sh --check-reverse-coverage
bash scripts/plan-coverage.sh --check-md-links docs/plan docs/features docs/acceptance-cases
cargo check --locked -p deve_web --target wasm32-unknown-unknown
cargo test --locked

# Run the server (dev login admin/admin)
cargo run -p deve_cli --bin deve_cli -- init --path . --repo default --projection-base notes
cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001

# Frontend iteration: backend on 3001, then Trunk on 8080
cd apps/web && NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080
```

`scripts/*.sh` need a POSIX shell (Git Bash on Windows). `scripts/` also holds the per-surface baseline checks (`check-*-baseline.sh`) and smoke scripts — run the ones nearest the surface you touched.

## Docs-first change ordering

`docs/plan/` is the **authoritative engineering blueprint**; code is a strict projection of docs, not an independent source of design authority. For behavior-affecting changes proceed `docs/plan/` → `docs/` → code:

1. Before touching `docs/plan/`, read `docs/plan/00_engineering_constitution.md` and `docs/plan/01_terminology.md`; make only local edits preserving chapter style.
2. Use `docs/coverage-matrix.md` to find the matching plan / features / acceptance docs before implementing or moving behavior.
3. Code disagreeing with a current plan invariant is implementation drift by default — align code to plan or record explicit drift/registry evidence; never weaken the plan because code already exists.
4. Every non-infra Rust module implementing plan behavior carries a `//! plan_ref:` header pointing at stable `docs/plan/` anchors (new anchors go into the plan before code references them).

`docs/features/` = user-visible behavior, `docs/acceptance-cases/` = automation-oriented proof, `docs/report/` = dated evidence (not a live contract), `docs/registry/` = live plan-concept → code-path mappings.

## Architecture

Workspace members: `crates/core` (`deve_core` — all business logic: ledger, projection, sync, source control, security, plugins, protocol), `apps/cli` (`deve_cli` — Clap commands + Axum HTTP/WebSocket server, default port 3001), `apps/web` (`deve_web` — Leptos CSR WASM frontend), `apps/desktop` / `apps/mobile` (native shell skeletons with optional Tauri `native-packaging` gates), `tools/baseline` (`deve_baseline` — developer/release checker CLI).

**Authority chain**: `Ledger → Folded State → Projection → Projection Workspace`

- `ledger/` holds authoritative repo facts (Redb-backed); `ledger/.host/projection-locators.toml` binds `RepoId → (projection_base, workspace_segment)` host-locally.
- External file changes enter `pending_fs_ops` first — nothing mutates authority until an explicit stage/commit path appends ledger facts. UI state, Git mirrors, and browser pending overlays are never authority.
- `.notegit/` is Deve-owned runtime state; `.git/` is only a Git ecosystem bridge (mirror/import/export/publish), never Git authority.
- UUID-first identity: repos and docs are identified by UUID; display names/aliases are host-local and never synchronized between peers.
- Fail-closed semantics throughout: a `doc_id` miss must not fall back to path-only; poisoned locks, missing metadata, and broken config all fail closed. Production requires `AUTH_SECRET`/`AUTH_PASS`; `--dev` enables admin/admin.
- Repo-scoped protocol: all server→client messages carry `repo_id`, `branch`, `scope_nonce`.
- `PersistGuard` shared between `RepoManager` and `SyncManager` prevents watcher storms.

**Frontend is a thin shell**: it renders UI, collects intent, and dispatches typed intents. Computation, state transitions, ledger/source-control mutations, diff/external-change decisions, and commit-anchor judgment live in backend/core. Never move that judgment into the frontend, and never bypass authority boundaries, writer gates, or Object Plane adapters for convenience — if the required path crosses those boundaries, update the plan/registry contract or stop for a USER decision first.

## Conventions

- Errors: `anyhow` in app crates, `thiserror` in `deve_core` (library). Backend-only core modules are gated `#[cfg(not(target_arch = "wasm32"))]`.
- Path handling must use `deve_core::utils::path::to_forward_slash` for Windows compatibility.
- File-size fuses: >~250 lines is a soft cohesion warning; hand-written source >~500 lines is a hard violation unless justified. Split by responsibility/API boundary, not to satisfy line counts. Tests may exceed the soft threshold for scenario readability.
- Dev/test profiles pin `debug = 1` and `incremental = false` (Windows MSVC linker OOM and WSL/DrvFS corruption) — do not "fix" these.
- Large bugs, architecture decisions, or changes touching authority, ledger, protocol, Source Control semantics, module boundaries, or data migrations MUST stop for USER analysis and approval before implementation.
- Verification of UI/browser-visible behavior uses Chrome MCP; if `127.0.0.1:9222` is down, run `chrome-mcp` from the shell first.
- Do not create or move release tags until branch CI is green and the tag-triggered workflows are explicitly accepted.
