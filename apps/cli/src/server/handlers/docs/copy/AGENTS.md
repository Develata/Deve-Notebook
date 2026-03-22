<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# copy

## Purpose
Implements document and directory deep-copy operations. Handles recursive directory tree copy (registering new nodes in the ledger for each copied item) and single file copy with content duplication.

## Key Files
| File | Description |
|------|-------------|
| `dir_copy.rs` | Recursive directory copy: walks source tree, registers structure facts, copies content |
| `dir_copy_test.rs` | Tests for directory copy |
| `file_copy.rs` | Single file copy: duplicates content ops and registers new node |
| `prepare.rs` | Copy path preparation and validation (source exists, dest does not) |
| `register.rs` | `CopyRegisterCtx` context for node registration during copy |

## For AI Agents

### Working In This Directory
- Directory copy is recursive; each copied node gets a new UUID via structure facts.
- `CopyRegisterCtx` bundles state/ch/scope/scope_nonce to avoid threading many parameters.
- Copy preserves structure facts for the copied subtree but creates new identities.
- Asset-only copy (`copy_dir_assets_only` from parent) copies non-markdown files without ledger registration.

<!-- MANUAL: -->
