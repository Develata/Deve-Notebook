<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# login

## Purpose

Login page and authentication state management.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Module entry and AuthState |
| `page.rs` | LoginPage component |
| `state.rs` | Auth state |

Login/logout HTTP calls live in `apps/web/src/api/auth_login.rs`; this component layer must not create its own HTTP client.

<!-- MANUAL: -->
