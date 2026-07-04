<!-- Generated: 2026-03-22 | Updated: 2026-07-05 -->

# Deve-Notebook

## Purpose

Rust workspace for a self-hosted collaborative Markdown notebook targeting low-resource environments (768 MB VPS). Workspace members are the core library (`crates/core`), Axum/Clap CLI server (`apps/cli`), Leptos WASM frontend (`apps/web`), and native shell skeletons (`apps/desktop`, `apps/mobile`). The system is ledger-first: repo facts live in Redb-backed authority storage, user-visible Markdown workspaces are repo-scoped projections, and sync/source-control/auth/plugin capabilities are gated by explicit runtime contracts.

## Key Files

| File | Description |
|------|-------------|
| `Cargo.toml` | Workspace root — members, shared deps, profile config |
| `Cargo.lock` | Pinned dependency versions |
| `README.md` / `README.zh.md` | Current project status, quick start, authority model |
| `config.toml` | Runtime configuration |
| `config.example.toml` | Configuration template |
| `Dockerfile` | Container build (release profile) |
| `docker-compose.yml` | Multi-service orchestration |
| `docker-compose.dev.yml` | Development compose profile |
| `.env.example` | Environment variable template |
| `.gitignore` | Ignored paths (target/, ledger/, vault/) |
| `CHANGELOG.md` | Release history |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `apps/` | Application binaries — CLI server, Web frontend, and native shell crates (see `apps/AGENTS.md`) |
| `crates/` | Shared library crates (see `crates/AGENTS.md`) |
| `plugins/` | Built-in Rhai plugins (see `plugins/AGENTS.md`) |
| `scripts/` | Build and lint utility scripts (see `scripts/AGENTS.md`) |
| `tests/` | Integration and plugin tests (see `tests/AGENTS.md`) |
| `docs/` | Project documentation (see `docs/AGENTS.md`) |
| `docs/plan/` | Engineering blueprint chapters (see `docs/plan/AGENTS.md`) |
| `docs/features/` | Product feature specifications with Chrome MCP walkthroughs |
| `docs/acceptance-cases/` | Automation-oriented validation cases |
| `docs/overview/` | Architecture views, code/doc drift maps, generated graph artifacts |
| `docs/registry/` | Controlled live registries mapping plan concepts to current code paths |
| `ledger/` | Runtime ledger data — host keys, local DB, remote peers; normally not hand-edited |
| `.github/` | CI workflows |

## For AI Agents

### Working In This Directory

- Read the nearest applicable `AGENTS.md` before editing a subdirectory; narrower files override this root file.
- For implementation/debugging/review work items where code or docs may change, follow the scoped "User-Requested Implementation Workflow" below. Pure explanation, planning, read-only investigation, or user-directed no-code discussion does not require that workflow.
- For changes covered by that workflow and affecting product or runtime behavior, proceed in this order: `docs/plan/` -> `docs/` -> code. Code is a strict projection of `docs/`, not an independent source of design authority. If a change requires modifying `docs/plan/`, first read [00_engineering_constitution.md](docs/plan/00_engineering_constitution.md) and [01_terminology.md](docs/plan/01_terminology.md), then make only local edits that preserve the original chapter style.
- `docs/plan/` is the authoritative engineering blueprint. `docs/features/` describes user-visible behavior, `docs/acceptance-cases/` describes automation-oriented proof, and `docs/report/` is dated evidence rather than a live contract.
- Use `docs/coverage-matrix.md` to find the matching plan/features/acceptance documents before implementing or moving behavior.
- Treat code that disagrees with a current plan invariant as implementation drift by default. Align code to plan, or record explicit drift/registry evidence; do not weaken the plan merely because code already exists.
- Every non-infra Rust module that implements plan behavior should carry a `//! plan_ref:` header pointing at stable `docs/plan/` anchors. New anchors belong in the plan before code refers to them.
- Prefer cohesive files. Split by responsibility, API boundary, or repeated infrastructure; do not split solely to satisfy a line count.
- Files over ~250 lines are soft architecture warnings and should be reviewed for cohesion, duplication, and hidden coupling.
- Hand-written source files over ~500 lines are hard fuse violations unless explicitly justified.
- Tests and test support may exceed the soft threshold when keeping scenario context together improves readability.
- Always preserve the ledger-first authority model: external file changes, UI state, Git mirrors, and browser pending overlays are not authority until the approved write path appends ledger facts.
- Target environment is 768 MB RAM VPS — evaluate every new dependency for memory footprint.
- Path handling must use `deve_core::utils::path::to_forward_slash` for Windows compatibility.
- Edition 2024 Rust. Error handling: `anyhow` (app layer), `thiserror` (library layer).
- This repo is often used from WSL2. If Chrome MCP is unavailable because `127.0.0.1:9222` is down, run `chrome-mcp` from the shell before retrying MCP browser actions.
- `chrome-mcp [url]` launches Windows Chrome with `--remote-debugging-port=9222` using a dedicated profile and can optionally open a target URL.

### User-Requested Implementation Workflow

For implementation, debugging, review, architecture-convergence, module/function audit, planned follow-up completion, or commit-producing work items where code or docs may be changed, follow this scoped workflow:

1. Read `docs/plan/00_engineering_constitution.md`.
2. Read `docs/plan/01_terminology.md`.
3. Read the matching `docs/plan/` contract, confirm the authority, runtime boundary, source of truth, failure path, and verification entrypoint, then decide whether the plan contract itself must be updated before code.
4. Read matching `docs/` feature / acceptance / registry / overview / task documents, then decide whether those projection documents must be updated before code.
5. Implement `docs/plan/` -> `docs/` -> code/docs, keeping implementation as a projection of contracts.
6. Run a quick gate sized to the touched surface, such as formatting/diff checks, targeted tests, and the nearest baseline scripts.
7. Run review with at most three review subagents when available. Scope them to high cohesion, low coupling, boundary drift, file size, failure paths, verification coverage, and frontend-thin-shell violations. If subagents are unavailable, perform an explicit self-review and report that they were unavailable.
8. The main agent fixes accepted review findings and closes every review subagent promptly after it returns a final result or is no longer needed.
9. Run final baseline and contract checks after fixes, including targeted tests, relevant baseline scripts, and required plan/docs/code checks.
10. Run Chrome MCP for UI/browser/mobile/editor/Source Control-visible behavior. For pure docs or non-UI backend work, explicitly report why Chrome MCP is not applicable.
11. If verification finds problems, return to the relevant `docs/plan/` -> `docs/` -> code step and repeat.
12. If verification passes, stage only relevant files and create a git commit.

This workflow is not required for pure explanation, planning, read-only investigation, or user-directed no-code discussion.

When the USER puts this repo into continuous main-branch audit/fix mode, work on `main` module by module and function by function: review, investigate, fix small bounded issues, and complete planned-but-unfinished items that are already covered by the current contracts. Small, bounded bugs may be fixed through this workflow. Large bugs, architecture decisions, major behavior changes, or changes touching authority, ledger, protocol, Source Control semantics, module boundaries, data migrations, or architecture models MUST stop for USER analysis and approval before implementation.

No formal version has been released yet, so this repo does not require preserving backward compatibility with old released versions during this workflow. That does not relax data-safety, authority, migration/fallback analysis, or verification requirements.

Architecture constraints for this workflow:

- The frontend is a thin shell: render UI, collect user intent, and dispatch typed intents.
- Computation, state transitions, ledger/source-control authority mutations, diff/external-change decisions, and commit-anchor business decisions belong in backend/core infra.
- Do not move ledger, External Changes, Source Control, diff, or commit-anchor business judgment into the frontend for UI convenience.
- Do not bypass authority, runtime boundaries, writer gates, or Object Plane adapters to ship faster, fix small bugs, simplify tests, or make UI wiring convenient. If the required path crosses those boundaries, update the relevant plan/registry contract or stop for a USER decision before implementation.
- Review subagents must focus on high cohesion, low coupling, boundary drift, file size, failure paths, and verification coverage, and completed review subagents must be closed promptly.

### Testing Requirements

```bash
# Targeted test (preferred)
cargo test --package <pkg> --lib <test_fn> -- --nocapture

# Full suite (use sparingly)
cargo test

# Lint
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check

# Plan/docs/code contract checks
scripts/plan-coverage.sh
scripts/plan-coverage.sh --check-md-links docs/plan docs/features docs/acceptance-cases
```

### Common Patterns

- Ledger-first storage: content facts + structure facts → projection → workspace.
- UUID-first identity: repos, docs identified by UUID; display names are aliases.
- Fail-closed semantics: `doc_id` miss must not fall back to path-only.
- Repo-scoped messages: all server→client messages carry `repo_id`, `branch`, `scope_nonce`.
- `PersistGuard` shared between `RepoManager` and `SyncManager` prevents watcher storms.

## Dependencies

### Internal

| Crate | Role |
|-------|------|
| `deve_core` | Core business logic — ledger, sync, source control, security, plugins |
| `deve_cli` | Axum server, commands, WebSocket handlers |
| `deve_web` | Leptos WASM frontend |
| `deve_desktop` | Desktop native shell skeleton and packaging gate |
| `deve_mobile` | Mobile native shell skeleton and packaging gate |

### External

| Crate | Role |
|-------|------|
| `redb` | Embedded key-value storage |
| `tokio` | Async runtime |
| `axum` | HTTP/WS server framework |
| `leptos` | Reactive WASM UI framework |
| `serde` / `serde_json` | Serialization |
| `uuid` | Entity identifiers |
| `chrono` | Timestamps |
| `clap` | CLI command surface |
| `tracing` | Structured runtime diagnostics |

<!-- MANUAL: -->
