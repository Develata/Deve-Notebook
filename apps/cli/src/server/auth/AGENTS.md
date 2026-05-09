<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# auth

## Purpose

Authentication middleware and handlers. Implements cookie-based auth, brute force protection, header validation, and login/session management.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Auth module entry |
| `middleware.rs` | Axum middleware for authentication checks |
| `cookie.rs` | Cookie-based session management |
| `headers.rs` | Auth header extraction and validation |
| `brute_force.rs` | Brute force attack protection |
| `brute_force/tests.rs` | Brute force attack protection tests |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `handlers/` | Login and session HTTP handlers |

## For AI Agents

### Working In This Directory

- Auth is middleware-based — applied to routes in the router.
- See `09_auth.md` in deve-note plan for the auth design.

<!-- MANUAL: -->
