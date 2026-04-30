# Rendering Current Boundary Baseline - 2026-04-30

## Scope

Closed the next small rendering follow-up by adding a script guard for the
current/future split already described in `docs/plan/03_rendering.md` and
`docs/features/03_rendering.md`.

## Guarded Boundary

- Current Web rendering remains split between CodeMirror adapter widgets,
  lightweight Markdown-to-HTML for chat/auxiliary HTML, and large-doc batching
  infrastructure.
- The lightweight Markdown renderer is not the main editor hybrid engine.
- Full preview mode, full virtual rendering, and rendering settings GUI
  persistence remain future work.
- Chat BUILD Apply remains a controlled Markdown edit affordance: apply buttons
  are BUILD-only, click handling is gated by session mode, and the write path
  sends `ClientMessage::Edit` with the current scope nonce.
- The renderer subset remains covered by tests for code block apply affordance,
  `<br>`-only HTML allowlist, secure external links, unsafe scheme rejection,
  and unsupported highlight syntax staying plain text.

## Verification

- `scripts/check-rendering-baseline.sh`
- `cargo test -p deve_web markdown -- --nocapture`
- `git diff --check`
