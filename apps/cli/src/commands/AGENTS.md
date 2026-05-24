<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# commands

## Purpose
Implements all CLI subcommands. Each subcommand is a separate module that initializes the required `RepoManager` and domain services, then performs its work. Commands that detect a database lock (another server process running) automatically fall back to HTTP proxy mode via `live_proxy`.

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | Module declarations for all subcommands |
| `serve.rs` | Starts the Axum server; detects port conflicts and switches to proxy/plugin-host mode |
| `serve/support.rs` | Serve command runtime initialization and port helpers |
| `serve/tests.rs` | Serve command tests |
| `init.rs` | Initializes vault directory structure, ledger, config.toml, and .env |
| `scan.rs` | Indexes all Markdown files in the vault into the ledger |
| `watch.rs` | Starts filesystem watcher for real-time vault change tracking (blocks until Ctrl+C) |
| `dump.rs` | Debug tool: prints all ops for a file path and reconstructs content |
| `export.rs` | Exports entire ledger to JSONL format for backup/migration |
| `export/doc.rs` | Markdown single-document export helpers |
| `export/tests.rs` | Export command tests |
| `backup.rs` | Read-only backup locator and provider adapter inspection command |
| `verify_p2p.rs` | Simulates multi-node P2P sync to verify shadow repo isolation and CRDT merging |
| `seed.rs` | Copies local repo ops into a shadow repo for testing (test/migration only) |
| `node_check.rs` | Checks and optionally repairs node consistency (missing/orphan nodes) |
| `node_check/tests.rs` | Node-check command tests |
| `repo_arg.rs` | Shared helper for resolving `--repo` CLI arguments (supports both UUID and name) |
| `live_proxy.rs` | HTTP proxy client for CLI commands when the database is locked by a running server |
| `live_proxy/tests.rs` | Tests for the live proxy port hint mechanism |
| `config/schema.rs` | `deve_cli config set` schema whitelist |
| `config/tests.rs` | Config command tests |
| `graph/tests.rs` | Graph command tests |
| `git_output.rs` | Git mirror CLI output facade |
| `git_output/` | Git mirror status/import/export/push output modules and tests |

## Subdirectories
| Directory | Purpose |
|-----------|---------|
| `repair/` | Data repair and corruption recovery subcommand |

## For AI Agents

### Working In This Directory
- Commands that open `RepoManager` must handle the database-lock fallback pattern: try init, check `is_db_lock_error`, fall back to `live_proxy`.
- `repo_arg::resolve_local_repo_arg` handles both UUID and name-based repo selectors; `resolve_local_repo_args` expands to all repos when no selector is given.
- `serve.rs` has a proxy mode that detects an existing main process and starts as a plugin-host satellite on a free port.
- `live_proxy.rs` reads a port hint file from the ledger `.host/main_port` before scanning candidate ports.
- All commands use fail-closed error handling; invalid inputs must produce errors, not silent fallbacks.

<!-- MANUAL: -->
