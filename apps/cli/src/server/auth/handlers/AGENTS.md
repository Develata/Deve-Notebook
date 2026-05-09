<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# handlers

## Purpose

Authentication HTTP handlers for login and session management.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Handler module declarations |
| `login.rs` | Login endpoint — validates credentials, issues session |
| `login/tests.rs` | Login endpoint tests |
| `session.rs` | Session check/refresh endpoint |

## For AI Agents

### Working In This Directory

- Login returns a session cookie on success.
- Session handler checks token validity and refreshes if needed.

<!-- MANUAL: -->
