<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# bin

## Purpose
Contains additional binary targets for development and testing. Currently holds `mock_divergence`, a utility that injects conflicting operations into shadow repos to simulate P2P divergence scenarios for testing merge and conflict resolution logic.

## Key Files
| File | Description |
|------|-------------|
| `mock_divergence.rs` | Injects conflict ops into peer shadow repos via `append_remote_op` for testing P2P divergence |

## For AI Agents

### Working In This Directory
- These are standalone binaries, not part of the main server.
- `mock_divergence` directly manipulates shadow repos which would be dangerous in production; test-only.
- It uses `append_remote_op` to inject ops as if they came from remote peers.

<!-- MANUAL: -->
