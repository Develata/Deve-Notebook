# Chat Drop File Fail-Closed - 2026-05-14

## Scope

- Runtime surface: Web AI Chat drag-and-drop file attachment helper.
- Plan basis: `docs/plan/10_ai_agent.md#native-ai-chat-runtime`.

## Change

- Replaced `FileReader::new().unwrap()` with a visible banner failure path.
- Converted `read_as_text` failure from ignored result to visible banner failure.
- Kept the existing 1 MiB attachment cap and successful file-to-code-block behavior unchanged.
- Added an AI baseline guard to prevent returning to panic or ignored-read behavior.

## Verification

- `cargo test -p deve_web attach_file_errors -- --nocapture`
- `bash scripts/check-ai-baseline.sh`
- `bash scripts/plan-coverage.sh`
- `cargo fmt --check`

## Result

AI Chat file drop no longer panics or fails silently when browser file reading is unavailable or rejected.
