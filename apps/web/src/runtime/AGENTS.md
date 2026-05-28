<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-05-28 -->

# runtime

## Purpose

Web client runtime bands. Infra-first runtime convergence (Phase B+) per
`docs/tasks/19_repo_refactor_blueprint.md` §3.3 and
`docs/report/runtime-convergence-audit-2026-05-28.md`: scattered runtime
logic under `hooks/use_core/` (the `effects_*` / `callbacks_*` prefix
families) is migrated here into typed `runtime/{session,scope,document,...}`
bands. `document` is the first band.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Runtime band module entry |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `document/` | Document runtime band — pending overlay + write-confirmation for the thin-client write path |

<!-- MANUAL: -->
