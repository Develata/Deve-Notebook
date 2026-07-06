<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# bin

## Purpose
Contains additional binary targets. `deve.rs` is the user-facing CLI alias and delegates to the shared runner. `mock_divergence.rs` is a development utility that injects conflicting operations into shadow repos to simulate P2P divergence scenarios for testing merge and conflict resolution logic.

## Key Files
| File | Description |
|------|-------------|
| `deve.rs` | User-facing `deve` binary alias; delegates to the same runner as `deve_cli` |
| `mock_divergence.rs` | Injects conflict ops into peer shadow repos via `append_remote_op` for testing P2P divergence |

## For AI Agents

### Working In This Directory
- `deve.rs` must stay a thin alias over `deve_cli::run_cli()` so the `deve` and `deve_cli` command surfaces cannot diverge.
- `mock_divergence` directly manipulates shadow repos which would be dangerous in production; test-only.
- It uses `append_remote_op` to inject ops as if they came from remote peers.

<!-- MANUAL: -->
