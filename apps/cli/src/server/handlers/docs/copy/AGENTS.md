<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# copy

## Purpose
Implements document and directory deep-copy operations. Handles recursive directory tree copy (registering new nodes in the ledger for each copied item) and single file copy with content duplication.

## Key Files
| File | Description |
|------|-------------|
| `path_validation_tests.rs` | Copy path validation regression tests for parent `copy.rs` |
| `prepare.rs` | Copy path preparation and validation (source exists, dest does not) |
| `register/mod.rs` | Two-phase copied-doc and asset preparation/registration entry point |
| `register/path.rs` | Source-to-destination relative path mapping |

## For AI Agents

### Working In This Directory
- Directory copy is prepared outside the repo lane and committed under the mounted-repo gate; each copied node gets a new UUID via structure facts.
- `CopyRegisterCtx` bundles state/ch/scope/scope_nonce to avoid threading many parameters.
- Copy preserves structure facts for the copied subtree but creates new identities.
- Asset-only copy (`copy_dir_assets_only` from parent) copies non-markdown files without ledger registration.

<!-- MANUAL: -->
