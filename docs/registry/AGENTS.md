<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-05-20 -->

# docs/registry

## Purpose

Controlled live registries that connect plan concepts to current implementation
state. This directory is for status tables, ownership maps, and cross-layer
indexes that must stay current but should not turn `docs/plan/` into a progress
log.

## Key Files

| File | Description |
|------|-------------|
| `runtime-skeleton-registry.md` | Runtime name, convergence status, current module path, and tracking task registry |
| `release-audit-warning-registry.md` | Current non-vulnerability `cargo audit` warnings, allowlist rationale, and replacement route registry |

## For AI Agents

- Registry documents are current-state indexes, not new engineering contracts.
- Do not introduce product behavior, protocol rules, or acceptance criteria here.
- If a registry disagrees with code, verify code first and update the registry.
- If a registry disagrees with a plan invariant, do not weaken the plan invariant from this directory; update the plan only with explicit authorization.
- Prefer narrow tables with controlled values over narrative progress history.

<!-- MANUAL: -->
