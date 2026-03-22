<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# repo

## Purpose

Repository management HTTP endpoints. Handles repo listing, creation, and metadata operations exposed as REST API.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Module entry and route registration |
| `http.rs` | HTTP handlers for repo CRUD |
| `http_test.rs` | Tests for repo HTTP endpoints |

## For AI Agents

### Working In This Directory

- Repos are identified by UUID, not by name/path.
- These are HTTP endpoints (not WebSocket messages).

<!-- MANUAL: -->
