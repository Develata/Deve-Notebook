<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# docs

## Purpose

Project documentation for Deve-Notebook. Documentation is split into three distinct layers:

- `plan/`: engineering blueprint, infra boundaries, state machines, protocol contracts
- `features/`: product behavior, user-visible workflows, Chrome MCP manual verification scenarios
- `acceptance-cases/`: automation-oriented validation cases and script entry contracts

## Key Files

| File | Description |
|------|-------------|
| `ai-chat-streaming.md` | Design doc for AI chat streaming protocol and integration |

## For AI Agents

### Working In This Directory

- Documentation is in Markdown format.
- Do not mix engineering blueprint and feature description in the same document tree.
- `docs/plan/` answers: how the system is engineered.
- `docs/features/` answers: what the product does and how an agent can verify it via Chrome MCP.
- `docs/acceptance-cases/` answers: what automated tests/scripts must prove without relying on manual browser interaction.

<!-- MANUAL: -->
