<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# state

## Purpose

Document state management: content reconstruction from OT operations, UTF-16 rope for WASM compatibility, and state diff computation.

## Key Files

| File | Description |
|------|-------------|
| `rope_utf16.rs` | UTF-16 compatible rope data structure |
| `utf16.rs` | UTF-16 offset conversion utilities |
| `tests.rs` | State reconstruction tests |

## For AI Agents

### Working In This Directory

- UTF-16 is required because JavaScript/WASM operates on UTF-16 strings.
- The rope supports efficient insert/delete at arbitrary positions.

<!-- MANUAL: -->
