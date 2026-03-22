<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# tree

## Purpose

Document tree management: builds and maintains the hierarchical file tree from ledger structure facts, handles delta updates, and manages tree nodes.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Tree module entry |
| `manager.rs` | TreeManager — maintains the live tree state |
| `node.rs` | TreeNode type definition |
| `from_docs.rs` | Builds tree from document list |
| `delta.rs` | Tree delta computation (adds/removes/moves) |
| `ops.rs` | Tree mutation operations |

<!-- MANUAL: -->
