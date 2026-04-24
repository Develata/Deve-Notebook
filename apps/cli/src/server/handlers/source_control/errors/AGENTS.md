<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# errors

## Purpose

Op-aware error mapping for source control operations. Maps internal errors to client-facing error codes, differentiating by operation type (changes, diff, commit, etc.).

## Key Files

| File | Description |
|------|-------------|
| `map.rs` | Error mapping entry points — maps core errors to SC-specific errors per operation |
| `map_common.rs` | Common repo-scope/storage error classification |
| `map_op.rs` | Source-control operation context enum |
| `map_op_specific.rs` | Operation-specific error classification |
| `map_op_test.rs` | Operation-specific mapping regression tests |
| `map_scope_test.rs` | Repo-scope mapping regression tests |

## For AI Agents

### Working In This Directory

- Error mapping must be fail-closed: unknown errors should not silently succeed.
- Each operation type may map the same underlying error to different client error codes.

<!-- MANUAL: -->
