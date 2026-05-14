# Outline Inline Parser No-Panic - 2026-05-14

## Scope

- Runtime surface: Web outline inline Markdown parser.
- Plan basis: `docs/plan/03_rendering.md#markdown-render-whitelist` and `docs/plan/03_rendering.md#document-authority-bridge`.

## Change

- Added a shared `next_char_at` scanner helper that uses `str::get` before reading the next UTF-8 character.
- Replaced direct `text[i..].chars().next().unwrap()` in outline scan and parse paths.
- Made scan helpers fail soft on non-character-boundary starts.
- Kept existing inline code, math, strong, emphasis, and delete parsing behavior unchanged for valid input.
- Added tests for non-character-boundary scan starts and multibyte escape handling.
- Added rendering baseline guards so outline parsing cannot regain direct next-char unwraps.

## Verification

- `cargo test -p deve_web outline -- --nocapture`
- `bash scripts/check-rendering-baseline.sh`
- `bash scripts/plan-coverage.sh`
- `cargo fmt --check`

## Result

Outline inline parsing remains a lightweight rendering helper and no longer depends on panic-backed UTF-8 index invariants.
