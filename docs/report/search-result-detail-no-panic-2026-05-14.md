# Search Result Detail No-Panic - 2026-05-14

## Scope

- Runtime surface: Web unified search result item rendering.
- Plan basis: `docs/plan/14_tech_stack.md#search-baseline` and `docs/plan/08_ui_design_01_web.md#web-layout-persistence`.

## Change

- Replaced search result detail rendering that depended on `detail_text.clone().unwrap()` with explicit `Option<String>` view construction.
- Removed the duplicated `detail_cond` / `detail_text` invariant from `result_item`.
- Kept title, detail text, mobile/desktop detail classes, and search result action semantics unchanged.
- Added Search baseline guards so result detail rendering cannot regain the panic-backed invariant.

## Verification

- `cargo test -p deve_web search_box -- --nocapture`
- `bash scripts/check-search-baseline.sh`
- `bash scripts/plan-coverage.sh`
- `cargo fmt --check`

## Result

Unified search result details remain visually equivalent and no longer depend on a panic-backed UI rendering invariant.
