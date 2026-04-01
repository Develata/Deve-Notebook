<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# mcp

## Purpose

Model Context Protocol (MCP) server endpoints. Exposes notebook capabilities to external AI tools via HTTP, SSE, and stdio transports.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | MCP module entry |
| `http.rs` | HTTP transport for MCP |
| `sse.rs` | SSE transport for MCP |
| `stdio.rs` | Stdio transport for MCP |
| `protocol.rs` | MCP protocol message definitions |

## For AI Agents

### Working In This Directory

- MCP allows AI tools to read/write notes, search, and interact with the ledger.
- See `10_ai_agent.md` and `17_plugins.md` in deve-note plan.

<!-- MANUAL: -->
