# apps/web/src/runtime

## Boundary

This directory contains Web client runtime adapters. They are Flow Coordination
or Object Plane adapter code, not Execution Domain authority.

## Rules

- Do not append ledger facts or decide source-control authority here.
- Do not import from `apps/web/src/hooks/use_core` internals.
- Expose typed client handles and pure client-side helpers only.
- DOM, CodeMirror, KaTeX, and browser globals belong under rendering client
  adapters and must not clear pending writes or mark writes successful.
