<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-04-09 -->

# features

## Purpose

Product feature specification for Deve-Notebook. `docs/features/` defines what the product does, what the user can observe, and how an agent should verify each feature through Chrome MCP manual walkthroughs.

## Key Rules

- `docs/features/` must stay user-visible and behavior-oriented.
- Do not move protocol contracts or low-level runtime implementation details here; those belong in `docs/plan/`.
- Every feature chapter should describe at least one concrete Chrome MCP validation instance.
- Feature validation should drive the UI through application controls, commands, or stable workflows rather than relying on fragile DOM accidents.

## For AI Agents

### Working In This Directory

- Read the matching chapter in `docs/plan/` first, then describe the observable product behavior here.
- Keep each feature item concrete enough for Chrome MCP to run end-to-end.
- Use this directory to define final manual acceptance, not automation scripts.

<!-- MANUAL: -->
