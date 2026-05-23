# VS Code SCM Interaction Notes

These notes describe the interaction model Deve follows. They intentionally avoid VS Code DOM, CSS, and implementation details.

## Primary Flow

The primary flow is:

```text
Source Control header
Repository/provider context
Commit message input
Commit action
Resource groups
Secondary history / graph surfaces
```

The commit input is a top-level control. It is not part of the `Changes` group.

## Resource Groups

Expected groups:

- `Staged Changes`
- `Changes`
- conflict / merge group when present
- optional diagnostics or read-only group

Groups carry counts and group-level actions. Staged and unstaged entries must not be visually merged.

## Change Rows

Rows should support:

- click to open diff
- status marker for modified / added / deleted / renamed / conflict
- hover or inline actions for stage, unstage, discard, and open
- disabled or explanatory actions in read-only scopes

## Repositories and Providers

Single-repo mode should keep repository/provider context compact. Multi-repo mode may expose a repositories list, but it must not dominate the default Source Control flow.

## History and Graph

History and graph are secondary read-only surfaces. They may be collapsible or opened by command/menu. They should not displace the commit and changes workflow by default.

## Deve Differences

Deve follows the SCM mental model but keeps different authority:

- Deve stage / commit writes ledger-backed source-control state.
- Git mirror commands are separate CLI-only or read-only review surfaces unless a future gate explicitly opens them.
- Remote branches remain read-only in the editor and Source Control writer path.
- Backup / restore uses `18_backup.md`, not Git remote push/pull.
