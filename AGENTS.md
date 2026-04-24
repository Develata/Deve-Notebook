<!-- Generated: 2026-03-22 | Updated: 2026-03-22 -->

# Deve-Notebook

## Purpose

Rust workspace for a high-performance collaborative notebook application targeting low-resource environments (768 MB VPS). Three workspace members: core library (`crates/core`), Leptos WASM frontend (`apps/web`), and Axum CLI server (`apps/cli`). Features ledger-based storage (Redb), CRDT sync, git-like source control, E2E encryption, and a Rhai plugin system.

## Key Files

| File | Description |
|------|-------------|
| `Cargo.toml` | Workspace root — members, shared deps, profile config |
| `Cargo.lock` | Pinned dependency versions |
| `config.toml` | Runtime configuration |
| `config.example.toml` | Configuration template |
| `Dockerfile` | Container build (release profile) |
| `docker-compose.yml` | Multi-service orchestration |
| `.env.example` | Environment variable template |
| `.gitignore` | Ignored paths (target/, ledger/, vault/) |
| `CHANGELOG.md` | Release history |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `apps/` | Application binaries — CLI server and Web frontend (see `apps/AGENTS.md`) |
| `crates/` | Shared library crates (see `crates/AGENTS.md`) |
| `plugins/` | Built-in Rhai plugins (see `plugins/AGENTS.md`) |
| `scripts/` | Build and lint utility scripts (see `scripts/AGENTS.md`) |
| `tests/` | Integration and plugin tests (see `tests/AGENTS.md`) |
| `docs/` | Project documentation (see `docs/AGENTS.md`) |
| `docs/plan/` | Engineering blueprint chapters (see `docs/plan/AGENTS.md`) |
| `docs/features/` | Product feature specifications with Chrome MCP walkthroughs |
| `docs/acceptance-cases/` | Automation-oriented validation cases |
| `ledger/` | Runtime ledger data — host keys, local DB, remote peers (see `ledger/AGENTS.md`) |
| `.github/` | CI workflows (see `.github/AGENTS.md`) |

## For AI Agents

### Working In This Directory

- Prefer cohesive files. Split by responsibility, API boundary, or repeated infrastructure; do not split solely to satisfy a line count.
- Files over ~250 lines are soft architecture warnings and should be reviewed for cohesion, duplication, and hidden coupling.
- Hand-written source files over ~500 lines are hard fuse violations unless explicitly justified.
- Tests and test support may exceed the soft threshold when keeping scenario context together improves readability.
- Always consult `docs/plan/` before implementing features.
- Target environment is 768 MB RAM VPS — evaluate every new dependency for memory footprint.
- Path handling must use `deve_core::utils::path::to_forward_slash` for Windows compatibility.
- Edition 2024 Rust. Error handling: `anyhow` (app layer), `thiserror` (library layer).
- This repo is often used from WSL2. If Chrome MCP is unavailable because `127.0.0.1:9222` is down, run `chrome-mcp` from the shell before retrying MCP browser actions.
- `chrome-mcp [url]` launches Windows Chrome with `--remote-debugging-port=9222` using a dedicated profile and can optionally open a target URL.

### Testing Requirements

```bash
# Targeted test (preferred)
cargo test --package <pkg> --lib <test_fn> -- --nocapture

# Full suite (use sparingly)
cargo test

# Lint
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
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

<!-- MANUAL: -->
