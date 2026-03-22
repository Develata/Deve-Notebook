<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# src

## Purpose
Root source directory for the `deve_cli` binary. Contains the `main.rs` entry point which parses CLI arguments via clap, initializes logging and configuration, and dispatches to subcommand handlers. Also contains shared API types, export logic, and the dispatch router.

## Key Files
| File | Description |
|------|-------------|
| `main.rs` | Binary entry point; defines CLI args/subcommands via clap `Commands` enum, loads config, dispatches |
| `dispatch.rs` | Routes `Commands` enum variants to their handler functions in `commands/` |
| `admin_api.rs` | Shared API response types: `DumpResponse`, `ExportEntry`, `NodeCheckResponse` |
| `export_entries.rs` | Builds JSONL export data from the ledger, resolving paths via node projection |
| `dump_support.rs` | Builds debug dump of a document's ops history and reconstructed content |

## Subdirectories
| Directory | Purpose |
|-----------|---------|
| `bin/` | Additional binary targets (test utilities) |
| `commands/` | CLI subcommand implementations |
| `server/` | Axum HTTP/WebSocket server module |

## For AI Agents

### Working In This Directory
- `main.rs` defines the `Commands` enum; add new CLI subcommands here and route them in `dispatch.rs`.
- Config is loaded via `deve_core::config::Config::load_checked()` from env/config.toml.
- `admin_api.rs` types are shared between CLI commands and HTTP admin endpoints; changes affect both paths.
- `export_entries.rs` prefers node projection paths over stale doc-to-path metadata; this is intentional for data integrity.

<!-- MANUAL: -->
