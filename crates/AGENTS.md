<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# crates

## Purpose

Shared library crates for the Deve-Notebook workspace. Currently contains a single crate `core` which holds all core business logic — ledger storage, sync engine, source control, security, plugins, search, and protocol definitions.

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `core/` | Core library — ledger, sync, source control, security, plugins, protocol (see `core/AGENTS.md`) |

## For AI Agents

### Working In This Directory

- All shared logic belongs in `crates/core`. Application-specific code goes in `apps/`.
- The crate is referenced as `deve_core` in workspace dependencies.

<!-- MANUAL: -->
