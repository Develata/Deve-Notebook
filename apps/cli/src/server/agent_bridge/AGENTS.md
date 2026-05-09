<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# agent_bridge

## Purpose

Policy-gated bridge for a user-trusted external AI CLI. It is default-off, requires explicit `enabled + trusted` configuration and an absolute `AGENT_CLI_PATH`, and only streams controlled CLI output back to chat.

This is not a plugin marketplace surface, not MCP, and not a generic notebook-authority bridge. It must not silently inherit host environment, search PATH, write source control, or bypass Native AI Chat fallback.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Agent bridge facade, global policy state, HTTP capability endpoint, and chat dispatch |
| `policy.rs` | Default-off trusted CLI policy and backend capability resolution |
| `prompt.rs` | Prompt construction for agent interactions |
| `stream.rs` | Response streaming from agent bridge |

<!-- MANUAL: -->
