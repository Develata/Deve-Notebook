# 0001. Leptos over Yew for the frontend

- Status: Accepted
- Date: 2026-05-27

## Context

Deve-Notebook needs a Rust/WASM frontend for a low-resource collaborative
Markdown notebook. The two mature Rust UI options were Yew (virtual-DOM,
component-diff model) and Leptos (fine-grained signal reactivity with optional
SSR/CSR). The product targets low-spec devices, so per-update DOM work and WASM
heap pressure matter.

## Decision

Use **Leptos v0.7** with `leptos_router`. Fine-grained signals update only the
affected DOM nodes (no full virtual-DOM diff per change), which suits an editor
with frequent small deltas and the `low-spec` profile's CSR-only mode.

## Consequences

- Reactivity is signal-driven; UI runtime exposes state and emits typed intents (Infra-First guardrail).
- CSR is the current build (only the Leptos `csr` feature is enabled); SSR on `standard` is a reserved/planned target, not yet wired.
- Commits the project to the Leptos signal model rather than a virtual-DOM mental model.

## References

- docs/plan/17_tech_stack.md (Frontend / Router)
- docs/plan/11_ui_design/index.md (shell/control/runtime topology)
