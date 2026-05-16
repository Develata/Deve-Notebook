# Mainline Gap Scan After Release Boundary Alignment

Date: 2026-05-16

## Scope

- Source of truth: `docs/plan/`.
- Checked layers: features, acceptance cases, overview architecture registry, guard scripts, and current code-linked operation paths.
- This batch does not reopen native process runtime, native authority writes, signed release, app store, or physical-device readiness.

## Result

- No P0 gap was found.
- No `docs/plan/` change was required.
- One P1 documentation/registry drift was found and fixed:
  - `docs/features/operations/rendering_large_doc_prefetch.md` contains `RENDER-LARGE-002` and `op.render.large-doc.delta-fallback`.
  - `docs/features/operation-coverage.md` still bound the flow only to `RENDER-LARGE-001`.
  - `docs/overview/` architecture Lisp/DOT registries did not include the delta fallback operation/application edge.

## Fix

- Bound `flow.rendering.large-doc-prefetch` to both `RENDER-LARGE-001` and `RENDER-LARGE-002`.
- Added `op.render.large-doc.delta-fallback` to user-operation architecture fragments.
- Added `app.render.large-doc.delta-fallback` to application architecture fragments.
- Regenerated `docs/overview/architecture-doc.lisp`, `docs/overview/architecture-code.lisp`, `docs/overview/architecture.dot`, and `docs/overview/architecture.svg`.

## Verification

- `bash scripts/check-architecture-registry.sh`
- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/check-network-baseline.sh`
- `bash scripts/check-storage-repo-baseline.sh`
- `bash scripts/check-source-control-baseline.sh`
- `bash scripts/check-rendering-baseline.sh`
- `bash scripts/check-mobile-baseline.sh`
- `bash scripts/check-graph-baseline.sh`
- `bash scripts/plan-coverage.sh`
- `git diff --check`

## Next Batch

Run a full regression gate refresh after this alignment:

- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo fmt --check`
- release/native/mobile/domain guards
- runtime happy/recovery smoke
