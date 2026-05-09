<!-- Parent: ../../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# switcher_prepare_test

## Purpose

Tests for the switcher preparation phase, validating both remote switching and fail-closed behavior when targets are invalid or missing.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Test module declarations |
| `remote.rs` | Tests for remote repo/branch switching preparation |
| `remote_fail_closed.rs` | Tests verifying fail-closed behavior on invalid remote targets |

## For AI Agents

### Working In This Directory

- These tests validate that the switcher does NOT silently accept invalid targets.
- `remote_fail_closed.rs` is critical for ensuring fail-closed semantics.

<!-- MANUAL: -->
