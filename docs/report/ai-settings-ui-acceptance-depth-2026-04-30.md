# AI Settings UI Acceptance Depth - 2026-04-30

## Scope

Closed one P2 acceptance-depth gap from the post-P2 rescan: AI/settings UI
availability feedback is now covered by code-level policy tests instead of only
static baseline string checks.

## Implemented

- Extracted Settings AI backend button state into a pure policy helper.
- Native and Trusted CLI buttons now expose disabled reasons only when that
  backend is unavailable.
- Available backend buttons no longer carry misleading disabled fallback titles.
- Tests cover native selected, trusted-cli unavailable, native disabled, and
  trusted-cli selected states.

## Boundary

This remains UI feedback only. The server remains the authority for
`/api/ai/backend-capabilities`, and Settings still does not persist runtime
config through a server-backed Settings API.

## Verification

- `cargo test -p deve_web ai_backend -- --nocapture`
- `scripts/check-ai-baseline.sh`
- `cargo fmt --check`
