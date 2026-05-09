<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# copy

## Purpose
Implements document and directory deep-copy operations. Handles recursive directory tree copy (registering new nodes in the ledger for each copied item) and single file copy with content duplication.

## Key Files
| File | Description |
|------|-------------|
| `path_validation_tests.rs` | Copy path validation regression tests for parent `copy.rs` |
| `dir_copy.rs` | Recursive directory copy: walks source tree, registers structure facts, copies content |
| `dir_copy/tests.rs` | Tests for directory copy |
| `file_copy.rs` | Single file copy: duplicates content ops and registers new node |
| `prepare.rs` | Copy path preparation and validation (source exists, dest does not) |
| `register/mod.rs` | Copied docs registration entry point and `CopyRegisterCtx` |
| `register/dirs.rs` | Directory structure fact registration for copied trees |
| `register/files.rs` | Markdown content duplication and ledger registration for copied files |
| `register/path.rs` | Source-to-destination relative path mapping |

## For AI Agents

### Working In This Directory
- Directory copy is recursive; each copied node gets a new UUID via structure facts.
- `CopyRegisterCtx` bundles state/ch/scope/scope_nonce to avoid threading many parameters.
- Copy preserves structure facts for the copied subtree but creates new identities.
- Asset-only copy (`copy_dir_assets_only` from parent) copies non-markdown files without ledger registration.

<!-- MANUAL: -->
