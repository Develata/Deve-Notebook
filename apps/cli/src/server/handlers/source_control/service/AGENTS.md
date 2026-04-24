<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# service

## Purpose

Source control service layer that abstracts read/write/target operations. Provides a clean API boundary between HTTP handlers and the core source control logic.

## Key Files

| File | Description |
|------|-------------|
| `read.rs` | Read operations (changes, diff, history queries) |
| `read_test.rs` | Read service tests |
| `write.rs` | Write operations (stage, unstage, discard, commit) |
| `target.rs` | Target resolution — resolves doc_id for source control operations |
| `target_*_test.rs` | Target resolution guard tests |

## For AI Agents

### Working In This Directory

- `target.rs` must use UUID-first resolution — never fall back to path-only.

<!-- MANUAL: -->
