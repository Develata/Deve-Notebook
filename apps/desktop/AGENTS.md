<!-- Parent: ../AGENTS.md -->

# desktop

## Purpose

Minimal native desktop shell skeleton. This crate fixes the adapter/session/bootstrap boundary before the project adopts a full Tauri runtime.

## Rules

- Do not add Tauri or platform packaging dependencies without updating `docs/plan/08_ui_design_02_desktop.md`, `docs/plan/14_tech_stack.md`, and `scripts/check-native-track-boundary.sh`.
- Any future packaging runtime must stay behind the `native-packaging` feature in this crate; the default build remains the no-Tauri shell skeleton.
- The shell may validate and inject endpoint/session bootstrap data, but must not write ledger, vault, source-control, search index, `.git/`, or `.notegit/` authority.
- Session material must remain out of URLs, logs, localStorage, and bootstrap payloads.
- Use `deve_core::native_adapter` as the shared contract source.

## Testing

```bash
cargo test --package deve_desktop
```
