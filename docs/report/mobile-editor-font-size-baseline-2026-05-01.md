# Mobile Editor Font Size Baseline 2026-05-01

This report closes the static docs/code guard portion of `MOB-SHOULD-003`.

## Result

- Blocking failures: 0.
- `docs/plan/08_ui_design_03_mobile.md` now marks `MOB-SHOULD-003` as
  implemented.
- The implementation path is `apps/web/style/_base.css`, where `.cm-content`
  fixes `font-size: 16px`.
- `docs/acceptance-cases/05_ui.md` now includes a CLI assertion for
  `mobile_editor_font_size_16px`.
- `scripts/check-mobile-baseline.sh` guards the plan/code/acceptance binding.

## Verified

```bash
scripts/check-mobile-baseline.sh
rg "MOB-SHOULD-003|mobile_editor_font_size_16px" docs/plan/08_ui_design_03_mobile.md docs/acceptance-cases/05_ui.md
rg "\.cm-content|font-size: 16px" apps/web/style/_base.css
```

## Next Narrow Batch

Run a Chrome MCP mobile viewport smoke against the Web shell and assert the
computed `.cm-content` font size is `16px`.
