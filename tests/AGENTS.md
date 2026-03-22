<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# tests

## Purpose

Integration and external test suites for Deve-Notebook. Contains plugin integration tests and skill tests that exercise the full runtime stack.

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `plugins/` | Plugin system integration tests |
| `skills/` | Skill execution tests |

## For AI Agents

### Working In This Directory

- These are integration tests that may require a running server or initialized ledger.
- Use `cargo test --package deve_core --test <test_name>` to run specific test files.

<!-- MANUAL: -->
