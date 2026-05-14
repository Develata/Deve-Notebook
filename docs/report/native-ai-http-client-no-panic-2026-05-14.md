# Native AI HTTP Client No-Panic

Date: 2026-05-14

## Scope

- `apps/cli/src/server/ai_chat/stream.rs`
- `apps/cli/src/server/ai_chat/stream/tests.rs`
- `scripts/check-ai-baseline.sh`

## Contract

- `docs/plan/10_ai_agent.md#native-ai-chat-runtime`

## Change

- Replaced `expect("Failed to create HTTP client")` in Native AI Chat SSE setup with a result-returning `get_http_client`.
- Preserved the shared HTTP client singleton and existing SSE execution path.
- HTTP client construction failure now propagates through the existing AI Chat error path instead of panicking the server process.
- Added an AI baseline guard to prevent reintroducing panic-backed client initialization.

## Verification

- `cargo test -p deve_cli native_ai_http_client -- --nocapture`
- `bash scripts/check-ai-baseline.sh`
- `bash scripts/plan-coverage.sh`
- `cargo fmt --check`
- `git diff --check`
