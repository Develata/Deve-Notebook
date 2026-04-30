# AI Effective Config Boundary Status - 2026-04-30

## Scope

Closed the P0 drift where `ai.mode` / `ai.native_enabled` existed in `config.toml` but did not fully drive runtime behavior.

## Implemented Boundary

- `Config::load_checked()` now applies the existing `DEVE_AI_AGENT_BRIDGE_ENABLED` / `DEVE_AI_AGENT_BRIDGE_TRUSTED` env aliases before computing the effective AI mode.
- `ai.mode = "trusted-cli"` now falls back unless the bridge is enabled, trusted, and `AGENT_CLI_PATH` is absolute, present, and executable.
- `ai.native_enabled = false` disables Native AI provider registration and blocks public `ai-chat` RPC with a completed error response instead of leaving chat loading.
- `/api/ai/backend-capabilities` now exposes `native_available`, `trusted_cli_available`, and `effective_backend` so Web does not infer backend availability from defaults.
- Settings / Extensions backend buttons now disable unavailable native or trusted backends, and the Web guard switches away from unavailable backends with a visible reason.

## Verification

- `cargo fmt --check`
- `cargo test -p deve_core config -- --nocapture`
- `cargo test -p deve_cli agent_bridge -- --nocapture`
- `cargo test -p deve_web ai_backend -- --nocapture`

## Remaining Work

- Native desktop/mobile plan wording still needs the post-gate split described in `next-tasks.md`.
- Graph blocked/degraded acceptance polish remains a P1 follow-up.
