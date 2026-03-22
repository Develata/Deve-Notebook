<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# source_control

## Purpose

Git-like source control handlers: changes detection, staging/unstaging, commit, diff, conflict resolution, and history. Supports both local and remote repos through the proxy layer.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Module declarations and shared types |
| `http.rs` | HTTP endpoint registration for source control |
| `http_commits.rs` | Commit listing HTTP endpoint |
| `http_mutations.rs` | Mutation endpoints (stage, unstage, discard) |
| `http_mutations_commit.rs` | Commit endpoint |
| `changes.rs` | Detect uncommitted changes |
| `staging.rs` | Stage/unstage operations |
| `commits.rs` | Commit history listing |
| `diff.rs` | Local diff computation |
| `diff_remote.rs` | Remote diff via proxy |
| `discard.rs` | Discard uncommitted changes |
| `local_discard.rs` | Local-only discard path |
| `conflict.rs` | Merge conflict handling |
| `present.rs` | Source control presence/status |
| `repo_scope.rs` | Source control scope bootstrap |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `errors/` | Op-aware error mapping (local and remote) |
| `service/` | Source control service layer (read/write/target) |

## For AI Agents

### Working In This Directory

- `repo_scope.rs` bootstraps scope for source control — fail-closed on missing scope.
- Error mapping in `errors/` is op-aware — different operations produce different error codes.
- Remote operations delegate through `source_control_proxy_http.rs` in parent directory.

<!-- MANUAL: -->
