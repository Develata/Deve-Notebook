# Post Git Priority Reselection 2026-05-01

This report closes the `Post-Git-mirror priority reselection` active queue
item.

## Result

- Blocking failures: 0.
- Git mirror import/export/push is at a clean handoff point for the current
  plan: CLI surface, resolved import chain, export, push blockers, docs-code
  drift, and smoke cohesion are covered.
- No P0/P1 blocker was reopened by the rescan.
- Native packaging/process gates remain intentionally closed; reopening them
  would require a larger dependency/runtime batch.
- Graph data surfaces are already at the current read-only boundary, and the
  renderer gate remains intentionally closed.

## Selected Next Domain

The next narrow implementation domain is mobile Web polish, starting with
`MOB-SHOULD-003` in `08_ui_design_03_mobile.md`.

Reason:

- It is concrete and low-risk.
- It is still marked partial in plan, but current code already sets
  `.cm-content { font-size: 16px; }`.
- It directly improves mobile editing behavior by preventing iOS Safari focus
  zoom.
- It can be guarded without pulling in new dependencies or reopening native
  packaging.

## Verified

```bash
rg "MOB-SHOULD-003|Font Size|16px" docs -n
rg "\.cm-content|font-size: 16px" apps/web/style apps/web/js apps/web/src -n
```

## Next Narrow Batch

Close the `MOB-SHOULD-003` docs/code guard, then run a small mobile viewport
smoke to confirm the computed editor font size in the actual Web shell.
