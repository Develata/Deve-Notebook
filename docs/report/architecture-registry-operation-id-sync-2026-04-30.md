# Architecture Registry Operation ID Sync - 2026-04-30

## Scope

While binding SET-006 Settings acceptance to the current reserved UI contract,
`scripts/check-architecture-registry.sh` exposed stale operation IDs in the
overview lisp registry.

## Fixed

- Synced `op.i18n.locale.fallback-unsupported` into doc/code lisp fragments,
  replacing the stale `fallback-missing` operation ID.
- Added Native AI Chat operation IDs from `ai_chat.md` into doc/code lisp:
  mode switch, controlled Markdown apply, and native tool rejection.
- Added the release Web WASM quality gate operation into doc/code lisp and
  its application/module nodes.
- Regenerated `docs/overview/architecture-doc.lisp` and
  `docs/overview/architecture-code.lisp`.

## Verification

- `scripts/check-architecture-registry.sh`
- `scripts/check-acceptance-bindings.sh`
- `scripts/plan-coverage.sh`
