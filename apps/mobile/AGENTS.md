<!-- Parent: ../AGENTS.md -->

# mobile

## Purpose

Minimal native mobile shell skeleton. This crate fixes mobile lifecycle, endpoint/session/bootstrap, and foreground reprobe boundaries before adopting a full Tauri Mobile runtime.

## Rules

- Do not add Tauri Mobile, platform permission, push, file picker, or store packaging dependencies without updating `docs/plan/08_ui_design_03_mobile.md` and `docs/plan/14_tech_stack.md`.
- The shell may validate and inject endpoint/session bootstrap data, but must not write ledger, vault, source-control, search index, `.git/`, or `.notegit/` authority.
- Foreground/background/network/safe-area/keyboard lifecycle events are hints only; they must not grant write authority.
- Resume from background/suspension must require fresh auth/node-role/WS handshake and current `scope_nonce`.
- Session material must remain out of URLs, logs, Web localStorage, system clipboard, and bootstrap payloads.
- Use `deve_core::native_adapter` as the shared contract source.

## Testing

```bash
cargo test --package deve_mobile
```
