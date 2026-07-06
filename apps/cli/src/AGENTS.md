<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# src

## Purpose
Root source directory for the `deve_cli` crate. Contains the shared CLI runner that parses arguments via clap, initializes logging and configuration, and dispatches to subcommand handlers. The `deve` and `deve_cli` binaries are thin wrappers over the same runner. Also contains shared API types, export logic, and the dispatch router.

## Key Files
| File | Description |
|------|-------------|
| `cli.rs` | Shared CLI runner; defines CLI args/subcommands via clap `Commands` enum, loads config, dispatches |
| `main.rs` | Thin `deve_cli` binary wrapper that delegates to the shared CLI runner |
| `dispatch.rs` | Routes `Commands` enum variants to their handler functions in `commands/` |
| `admin_api.rs` | Shared API response types: `DumpResponse`, `ExportEntry`, `NodeCheckResponse` |
| `export_entries.rs` | Builds JSONL export data from the ledger, resolving paths via node projection |
| `dump_support.rs` | Builds debug dump of a document's ops history and reconstructed content |

## Subdirectories
| Directory | Purpose |
|-----------|---------|
| `bin/` | Additional binary targets, including the user-facing `deve` alias and test utilities |
| `commands/` | CLI subcommand implementations |
| `server/` | Axum HTTP/WebSocket server module |

## For AI Agents

### Working In This Directory
- `cli.rs` defines the `Commands` enum; add new CLI subcommands there and route them in `dispatch.rs`.
- Config is loaded via `deve_core::config::Config::load_checked()` from env/config.toml.
- `admin_api.rs` types are shared between CLI commands and HTTP admin endpoints; changes affect both paths.
- `export_entries.rs` prefers node projection paths over stale doc-to-path metadata; this is intentional for data integrity.

<!-- MANUAL: -->
