<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# errors

## Purpose

Op-aware error mapping for source control operations. Maps internal errors to client-facing error codes, differentiating by operation type (changes, diff, commit, etc.).

## Key Files

| File | Description |
|------|-------------|
| `map/` | Error mapping entry points, common classifiers, operation context, and tests |

## For AI Agents

### Working In This Directory

- Error mapping must be fail-closed: unknown errors should not silently succeed.
- Each operation type may map the same underlying error to different client error codes.

<!-- MANUAL: -->
